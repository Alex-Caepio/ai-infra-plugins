//! Customizable `.agent-guardrails/policy.json` schema, custody validation, and preset loading.
use crate::{
    custody,
    error::{GuardError, Result},
    presets::{self, Preset},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub const STANDALONE_POLICY_PATH: &str = ".agent-guardrails/policy.json";
pub const LEGACY_HOOK_POLICY_PATH: &str = "project-brain/config/hook-policy.json";
pub const MAX_BYTES: usize = 128 * 1024;

/// Optional provenance binding used when strict manifest custody is enabled.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub profile_path: String,
    pub profile_sha256: String,
    pub selection_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandPrefix {
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default)]
    pub options_with_values: Vec<String>,
    #[serde(default)]
    pub short_options_with_values: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Matching {
    Exact,
    Console,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    DestructiveCommand,
    SecretExposure,
    VerificationBypass,
}

impl Reason {
    pub fn label(self) -> &'static str {
        match self {
            Self::DestructiveCommand => "destructive command",
            Self::SecretExposure => "secret exposure",
            Self::VerificationBypass => "verification bypass",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    Always,
    AnyOption { patterns: Vec<String> },
    NoOption { patterns: Vec<String> },
    FirstArgument { patterns: Vec<String> },
    ArgumentContains { patterns: Vec<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenyRule {
    pub id: String,
    pub tool: String,
    pub command: Vec<String>,
    pub matching: Matching,
    pub condition: Condition,
    pub reason: Reason,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoopThresholds {
    pub cmd_warn: u32,
    pub cmd_block: u32,
    pub edit_warn: u32,
    pub edit_block: u32,
}

impl Default for LoopThresholds {
    fn default() -> Self {
        Self {
            cmd_warn: 6,
            cmd_block: 12,
            edit_warn: 7,
            edit_block: 10,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NamingRules {
    pub enforce_accelerator_conventions: bool,
    pub blocked_globs: Vec<String>,
}

impl Default for NamingRules {
    fn default() -> Self {
        Self {
            enforce_accelerator_conventions: true,
            blocked_globs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SubagentGateConfig {
    pub enabled: bool,
    pub block_builtin_codex: bool,
    pub allowed_agents: Vec<String>,
    pub writer_agents: Vec<String>,
}

impl Default for SubagentGateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            block_builtin_codex: false,
            allowed_agents: Vec::new(),
            writer_agents: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutsideRepoRmPolicy {
    #[default]
    Block,
    Allow,
}

/// Complete declarative policy configuration parsed from `.agent-guardrails/policy.json`
/// or legacy `project-brain/config/hook-policy.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookPolicy {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default, alias = "framework")]
    pub preset: String,
    #[serde(default)]
    pub evidence: Option<Evidence>,
    #[serde(default)]
    pub approved_tools: Vec<String>,
    #[serde(default)]
    pub project_command_prefixes: Vec<CommandPrefix>,
    #[serde(default)]
    pub deny_rules: Vec<DenyRule>,
    #[serde(default)]
    pub loop_thresholds: LoopThresholds,
    #[serde(default)]
    pub naming_rules: NamingRules,
    #[serde(default)]
    pub subagent_gate: SubagentGateConfig,
    #[serde(default)]
    pub outside_repo_rm: OutsideRepoRmPolicy,
    #[serde(default)]
    pub additional_secret_globs: Vec<String>,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for HookPolicy {
    fn default() -> Self {
        Self::template(Preset::Generic)
    }
}

/// Resolved policy paired with source path provenance.
#[derive(Clone, Debug)]
pub struct LoadedPolicy {
    pub preset: Preset,
    pub policy: HookPolicy,
    pub source_path: Option<PathBuf>,
    pub strict_custody: bool,
}

fn invalid(detail: &str) -> GuardError {
    GuardError::policy(format!("Invalid guardrails policy data or custody: {detail}"))
}

fn literal(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn slug(value: &str) -> bool {
    value.len() <= 64
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-')
}

fn hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn relative(value: &str) -> bool {
    literal(value, 1024)
        && !value.contains(['\\', ':'])
        && value
            .split('/')
            .all(|part| !part.is_empty() && ![".", ".."].contains(&part))
}

fn option(value: &str) -> bool {
    let stripped = value.strip_prefix("--").or_else(|| value.strip_prefix('-'));
    stripped.is_some_and(|v| {
        v.len() <= 63
            && v.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
            && v.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    })
}

fn entry(value: &str) -> String {
    let base = value.rsplit(['/', '\\']).next().unwrap_or(value);
    let trimmed = base.strip_suffix(".phar").unwrap_or(base);
    if trimmed == "wp-cli" {
        "wp".into()
    } else {
        trimmed.into()
    }
}

pub fn compile_glob(value: &str) -> Result<regex::Regex> {
    if !literal(value, 128) {
        return Err(invalid("glob pattern length or characters"));
    }
    let mut expression = String::from("^");
    let chars: Vec<char> = value.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        match chars[at] {
            '*' => expression.push_str(".*"),
            '?' => expression.push('.'),
            '[' => {
                expression.push('[');
                at += 1;
                if chars.get(at) == Some(&'!') {
                    expression.push('^');
                    at += 1;
                }
                let begin = at;
                while at < chars.len() && chars[at] != ']' {
                    if matches!(chars[at], '\\' | '[' | '^') {
                        expression.push('\\');
                    }
                    expression.push(chars[at]);
                    at += 1;
                }
                if at == chars.len() || at == begin {
                    return Err(invalid("unclosed character class in glob"));
                }
                expression.push(']');
            }
            ']' => return Err(invalid("unexpected ']' in glob")),
            c => expression.push_str(&regex::escape(&c.to_string())),
        }
        at += 1;
    }
    expression.push('$');
    regex::Regex::new(&expression).map_err(|_| invalid("invalid glob regex"))
}

fn patterns(condition: &Condition) -> Option<&[String]> {
    match condition {
        Condition::Always => None,
        Condition::AnyOption { patterns }
        | Condition::NoOption { patterns }
        | Condition::FirstArgument { patterns }
        | Condition::ArgumentContains { patterns } => Some(patterns),
    }
}

fn short(value: &str, key: char, stops: &str) -> bool {
    if !value.starts_with('-') || value.starts_with("--") || value.len() > 64 {
        return false;
    }
    for c in value.chars().skip(1) {
        if c == key {
            return true;
        }
        if stops.contains(c) {
            return false;
        }
    }
    false
}

fn short_value(value: &str, stops: &str) -> bool {
    if !value.starts_with('-') || value.starts_with("--") || value.len() > 64 {
        return false;
    }
    let chars: Vec<char> = value.chars().skip(1).collect();
    for (i, c) in chars.iter().enumerate() {
        if stops.contains(*c) {
            return i + 1 == chars.len();
        }
    }
    false
}

fn php_script(words: &[String], mut i: usize) -> Option<usize> {
    while i < words.len() {
        match words[i].as_str() {
            "-r" | "-R" | "-B" | "-E" | "-a" | "-i" | "-l" | "-m" | "-s" | "-v" | "-w" | "-h"
            | "--run" | "--info" => return None,
            "-f" | "-F" => return (i + 1 < words.len()).then_some(i + 1),
            "-c" | "-d" | "-S" | "-t" | "-z" | "--rf" | "--rc" | "--re" | "--rz" | "--ri" => {
                i += 2
            }
            "--" => return (i + 1 < words.len()).then_some(i + 1),
            value if value.starts_with('-') => i += 1,
            _ => return Some(i),
        }
    }
    None
}

impl HookPolicy {
    /// Clean template configuration written by `policy-init`.
    pub fn template(preset: Preset) -> Self {
        Self {
            schema_version: 1,
            preset: preset.as_str().into(),
            evidence: None,
            approved_tools: Vec::new(),
            project_command_prefixes: Vec::new(),
            deny_rules: Vec::new(),
            loop_thresholds: LoopThresholds::default(),
            naming_rules: NamingRules::default(),
            subagent_gate: SubagentGateConfig::default(),
            outside_repo_rm: OutsideRepoRmPolicy::Block,
            additional_secret_globs: Vec::new(),
        }
    }

    pub fn selection_value(&self) -> Value {
        json!({
            "schema_version": self.schema_version,
            "framework": self.preset,
            "approved_tools": self.approved_tools,
            "project_command_prefixes": self.project_command_prefixes,
            "deny_rules": self.deny_rules,
        })
    }

    pub fn selection_digest(&self) -> Result<String> {
        Ok(custody::sha256(
            custody::canonical_json(&self.selection_value())?.as_bytes(),
        ))
    }

    pub fn parse(raw: &[u8]) -> Result<Self> {
        let value = custody::strict_parse_json(raw, MAX_BYTES, 24, 20000)?;
        let policy: Self = serde_json::from_value(value).map_err(|_| invalid("schema mismatch"))?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(invalid("schema_version must be 1"));
        }
        let preset_name = if self.preset.is_empty() {
            "generic"
        } else {
            self.preset.as_str()
        };
        Preset::parse(preset_name)?;

        if self.loop_thresholds.cmd_warn == 0
            || self.loop_thresholds.cmd_block < self.loop_thresholds.cmd_warn
            || self.loop_thresholds.edit_warn == 0
            || self.loop_thresholds.edit_block < self.loop_thresholds.edit_warn
        {
            return Err(invalid("invalid loop_thresholds ordering"));
        }

        for glob in &self.naming_rules.blocked_globs {
            compile_glob(glob)?;
        }
        for glob in &self.additional_secret_globs {
            compile_glob(glob)?;
        }

        if let Some(evidence) = &self.evidence
            && (!relative(&evidence.profile_path)
                || !hex64(&evidence.profile_sha256)
                || !hex64(&evidence.selection_sha256)
                || self.selection_digest()? != evidence.selection_sha256)
        {
            return Err(invalid("evidence digest verification failed"));
        }

        if self.approved_tools.len() > 64
            || self.project_command_prefixes.len() > 128
            || self.deny_rules.len() > 256
        {
            return Err(invalid("rule counts exceed maximum bounds"));
        }

        let tools: BTreeSet<String> = self.approved_tools.iter().cloned().collect();
        if tools.len() != self.approved_tools.len() || tools.iter().any(|t| !slug(t)) {
            return Err(invalid("approved_tools must be unique slugs"));
        }

        let mut covered = BTreeSet::new();
        for prefix in &self.project_command_prefixes {
            if !tools.contains(&prefix.tool)
                || prefix.argv.is_empty()
                || prefix.argv.len() > 8
                || prefix.argv.iter().any(|w| !literal(w, 256))
                || prefix.options_with_values.len() > 32
                || prefix.options_with_values.iter().any(|w| !option(w))
                || prefix.short_options_with_values.len() > 26
                || !prefix
                    .short_options_with_values
                    .bytes()
                    .all(|b| b.is_ascii_alphabetic())
                || prefix
                    .short_options_with_values
                    .bytes()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != prefix.short_options_with_values.len()
            {
                return Err(invalid("invalid project_command_prefixes entry"));
            }
            covered.insert(prefix.tool.clone());
        }
        if !self.approved_tools.is_empty() && covered != tools {
            return Err(invalid("every approved_tool needs a project_command_prefixes entry"));
        }

        let mut ids = BTreeSet::new();
        for rule in &self.deny_rules {
            if !slug(&rule.id)
                || !ids.insert(&rule.id)
                || !tools.contains(&rule.tool)
                || rule.command.len() > 8
                || rule.command.iter().any(|w| !literal(w, 128))
            {
                return Err(invalid("invalid deny_rules entry"));
            }
            if let Some(pats) = patterns(&rule.condition) {
                if pats.is_empty() || pats.len() > 16 {
                    return Err(invalid("invalid deny rule condition pattern count"));
                }
                for pat in pats {
                    compile_glob(pat)?;
                }
            }
        }
        Ok(())
    }

    fn effective(prefix: &CommandPrefix) -> Option<&[String]> {
        let words = &prefix.argv;
        let first = entry(words.first()?);
        if first == "php"
            || first
                .strip_prefix("php")
                .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
        {
            if let Some(at) = php_script(words, 1) {
                Some(&words[at..])
            } else {
                Some(words)
            }
        } else {
            Some(words)
        }
    }

    pub fn knows_program(&self, program: &str) -> bool {
        self.project_command_prefixes
            .iter()
            .filter_map(Self::effective)
            .any(|words| words.first().is_some_and(|word| entry(word) == program))
    }

    fn next_option(prefix: &CommandPrefix, words: &[String], at: usize) -> usize {
        let word = &words[at];
        if !word.contains('=')
            && words.get(at + 1).is_some_and(|w| !w.starts_with('-'))
            && (prefix.options_with_values.iter().any(|opt| opt == word)
                || short_value(word, &prefix.short_options_with_values))
        {
            at + 2
        } else {
            at + 1
        }
    }

    fn condition(
        prefix: &CommandPrefix,
        rule: &DenyRule,
        words: &[String],
        first: usize,
        position: usize,
        console: Option<usize>,
    ) -> bool {
        let Some(pats) = patterns(&rule.condition) else {
            return true;
        };
        let matches = |word: &str| {
            pats.iter()
                .any(|value| compile_glob(value).is_ok_and(|p| p.is_match(word)))
        };
        match &rule.condition {
            Condition::Always => true,
            Condition::AnyOption { .. } | Condition::NoOption { .. } => {
                let found = words
                    .iter()
                    .enumerate()
                    .skip(first)
                    .take_while(|(_, word)| word.as_str() != "--")
                    .filter(|(at, _)| Some(*at) != console)
                    .any(|(_, word)| {
                        matches(word)
                            || console.is_some()
                                && pats.iter().any(|value| {
                                    value.len() == 2
                                        && value.starts_with('-')
                                        && short(
                                            word,
                                            value.chars().nth(1).unwrap_or(' '),
                                            &prefix.short_options_with_values,
                                        )
                                })
                    });
                if matches!(&rule.condition, Condition::NoOption { .. }) {
                    !found
                } else {
                    found
                }
            }
            Condition::FirstArgument { .. } => {
                let mut at = position;
                while at < words.len()
                    && words[at].starts_with('-')
                    && !(words[at].len() > 1 && words[at][1..].bytes().all(|b| b.is_ascii_digit()))
                {
                    at = Self::next_option(prefix, words, at);
                }
                words.get(at).is_some_and(|word| matches(word))
            }
            Condition::ArgumentContains { .. } => {
                let operands: Vec<&String> = words
                    .iter()
                    .skip(position)
                    .filter(|word| !word.starts_with('-'))
                    .collect();
                operands.is_empty()
                    || operands
                        .iter()
                        .any(|word| pats.iter().any(|pat| pat.contains(word.as_str())))
            }
        }
    }

    pub fn classify(&self, words: &[String], program: &str, argument: usize) -> Option<String> {
        for prefix in &self.project_command_prefixes {
            let Some(expected) = Self::effective(prefix) else {
                continue;
            };
            if expected.first().is_none_or(|w| entry(w) != program) {
                continue;
            }
            let mut at = argument;
            let mut prefix_matches = true;
            for fixed in &expected[1..] {
                if !fixed.starts_with('-') {
                    while at < words.len()
                        && (words[at].starts_with('-') || words[at].starts_with('@'))
                    {
                        at = Self::next_option(prefix, words, at);
                    }
                }
                if words.get(at) != Some(fixed) {
                    prefix_matches = false;
                    break;
                }
                at += 1;
            }
            if !prefix_matches {
                continue;
            }
            let first = at;
            while at < words.len() && words[at].starts_with('-') {
                at = Self::next_option(prefix, words, at);
            }
            for rule in self.deny_rules.iter().filter(|r| r.tool == prefix.tool) {
                let mut cursor = at;
                let mut matched = true;
                for expected_word in &rule.command {
                    if !expected_word.starts_with('-') {
                        while cursor < words.len()
                            && (words[cursor].starts_with('-') || words[cursor].starts_with('@'))
                        {
                            cursor = Self::next_option(prefix, words, cursor);
                        }
                    }
                    let Some(actual) = words.get(cursor) else {
                        matched = false;
                        break;
                    };
                    let accepted = match rule.matching {
                        Matching::Exact => actual == expected_word,
                        Matching::Console => {
                            let actual_l = actual.to_ascii_lowercase();
                            let expected_l = expected_word.to_ascii_lowercase();
                            let a_parts: Vec<&str> = actual_l.split(':').collect();
                            let e_parts: Vec<&str> = expected_l.split(':').collect();
                            a_parts.len() == e_parts.len()
                                && a_parts
                                    .iter()
                                    .zip(e_parts.iter())
                                    .all(|(a, e)| !a.is_empty() && e.starts_with(a))
                        }
                    };
                    if !accepted {
                        matched = false;
                        break;
                    }
                    cursor += 1;
                }
                if matched
                    && Self::condition(
                        prefix,
                        rule,
                        words,
                        first,
                        cursor,
                        if matches!(rule.matching, Matching::Console) {
                            Some(at)
                        } else {
                            None
                        },
                    )
                {
                    return Some(format!(
                        "{} - custom policy rule {}",
                        rule.reason.label(),
                        rule.id
                    ));
                }
            }
        }
        None
    }
}

/// Write `.agent-guardrails/policy.json` atomically for a selected preset.
pub fn init_policy(root: &Path, preset: Preset, force: bool) -> Result<PathBuf> {
    custody::admit_directory(root)?;
    let target = root.join(STANDALONE_POLICY_PATH);
    if target.exists() && !force {
        return Err(GuardError::conflict(format!(
            "Policy file already exists at {} (pass --force to overwrite)",
            target.display()
        )));
    }
    let policy = HookPolicy::template(preset);
    let mut rendered = serde_json::to_string_pretty(&policy)?;
    rendered.push('\n');
    custody::atomic_write(&target, rendered.as_bytes())?;
    Ok(target)
}

/// Load policy from `.agent-guardrails/policy.json`, falling back to legacy
/// `project-brain/config/hook-policy.json` (with `.infra-manifest.json` custody check)
/// or built-in defaults.
pub fn load_policy(root: &Path, preset_override: Option<Preset>) -> Result<LoadedPolicy> {
    let standalone = root.join(STANDALONE_POLICY_PATH);
    if standalone.exists() {
        let raw = custody::read_regular(&standalone, MAX_BYTES)?;
        let policy = HookPolicy::parse(&raw)?;
        let preset = preset_override.unwrap_or_else(|| {
            if policy.preset.is_empty() {
                Preset::Generic
            } else {
                Preset::parse(&policy.preset).unwrap_or(Preset::Generic)
            }
        });
        return Ok(LoadedPolicy {
            preset,
            policy,
            source_path: Some(standalone),
            strict_custody: false,
        });
    }

    let legacy = root.join(LEGACY_HOOK_POLICY_PATH);
    let manifest_path = root.join(".infra-manifest.json");
    if legacy.exists() {
        let raw = custody::read_regular(&legacy, MAX_BYTES)?;
        let policy = HookPolicy::parse(&raw)?;
        if manifest_path.is_file() {
            let mraw = custody::read_regular(&manifest_path, 16 * 1024 * 1024)?;
            let manifest = custody::strict_parse_json(&mraw, 8 * 1024 * 1024, 32, 300000)?;
            let expected_hash = custody::sha256(&raw);
            let actual_hash = manifest["files"][LEGACY_HOOK_POLICY_PATH].as_str();
            let profile_match = policy
                .evidence
                .as_ref()
                .is_some_and(|e| manifest["profile"].as_str() == Some(e.profile_path.as_str()));
            if actual_hash != Some(expected_hash.as_str()) || !profile_match {
                return Err(invalid("manifest hash mismatch"));
            }
        }
        let preset = preset_override.unwrap_or_else(|| {
            if policy.preset.is_empty() {
                Preset::Generic
            } else {
                Preset::parse(&policy.preset).unwrap_or(Preset::Generic)
            }
        });
        return Ok(LoadedPolicy {
            preset,
            policy,
            source_path: Some(legacy),
            strict_custody: true,
        });
    } else if manifest_path.is_file() {
        return Err(invalid("manifest exists without hook-policy.json"));
    }

    let preset = preset_override.unwrap_or(Preset::Generic);
    Ok(LoadedPolicy {
        preset,
        policy: HookPolicy::template(preset),
        source_path: None,
        strict_custody: false,
    })
}

/// Produce structured inspection report of the effective policy and available presets.
pub fn inspect_policy(root: &Path, preset_override: Option<Preset>) -> Result<Value> {
    let loaded = load_policy(root, preset_override)?;
    let spec = loaded.preset.spec();
    let presets_info: Vec<Value> = Preset::PUBLIC_NAMES
        .iter()
        .filter_map(|name| Preset::parse(name).ok())
        .map(|p| {
            json!({
                "name": p.as_str(),
                "description": p.description(),
            })
        })
        .collect();
    let mut rules_count = spec.rules.len() + loaded.policy.deny_rules.len();
    if !loaded.policy.additional_secret_globs.is_empty() {
        rules_count += 1;
    }
    Ok(json!({
        "root": root.display().to_string(),
        "preset": loaded.preset.as_str(),
        "preset_description": loaded.preset.description(),
        "policy_file": loaded.source_path.as_ref().map(|p| p.display().to_string()),
        "policy_configured": loaded.source_path.is_some(),
        "strict_custody": loaded.strict_custody,
        "builtin_rule_count": spec.rules.len(),
        "custom_rule_count": loaded.policy.deny_rules.len(),
        "effective_rule_count": rules_count,
        "outside_repo_rm": loaded.policy.outside_repo_rm,
        "additional_secret_globs": loaded.policy.additional_secret_globs,
        "loop_thresholds": loaded.policy.loop_thresholds,
        "naming_rules": loaded.policy.naming_rules,
        "subagent_gate": loaded.policy.subagent_gate,
        "available_presets": presets_info,
        "note": "Guardrails are deterministic advisory hooks, not an operating-system sandbox.",
    }))
}

pub fn extra_secret_rule(globs: &[String]) -> Option<String> {
    if globs.is_empty() {
        None
    } else {
        Some(format!(
            "read|{}|{}|secret exposure - reading custom restricted secret file",
            presets::reader_commands_csv(),
            globs.join(",")
        ))
    }
}
