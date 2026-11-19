//! Runs the frozen 331-case shell safety corpus plus Node, Python, Rust, and generic guard checks.
use agent_guardrails::{
    presets::Preset,
    shell::{Policy, validate, validate_in_repo},
};
use serde::Deserialize;
use std::path::Path;

const CORPUS: &str = include_str!("fixtures/bash-validator-corpus.json");
const FRAMEWORKS: [&str; 5] = [
    "laravel",
    "symfony",
    "php-core",
    "wordpress",
    "infrastructure-creator",
];

#[derive(Deserialize)]
struct Corpus {
    editions: Vec<String>,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    command: String,
    expect: Expectation,
    editions: Editions,
    note: String,
    category: Option<String>,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Expectation {
    Pass,
    Block,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Editions {
    All(String),
    Named(Vec<String>),
}

impl Case {
    fn blocks(&self, framework: &str) -> bool {
        let selected = match &self.editions {
            Editions::All(name) => {
                assert_eq!(name, "all");
                true
            }
            Editions::Named(names) => names.iter().any(|name| name == framework),
        };
        selected == (self.expect == Expectation::Block)
    }
}

struct RetiredMemoryLauncher {
    case_index: usize,
    legacy: &'static str,
    native: &'static str,
}

const RETIRED_MEMORY_LAUNCHERS: [RetiredMemoryLauncher; 4] = [
    RetiredMemoryLauncher {
        case_index: 107,
        legacy: r###"cd /srv/shop && echo -E '{"progress":"The down() migration runs DROP TABLE invoices.","verified":true,"learnings":[]}' | python3 memory-bank/scripts/context.py --root /srv/shop native-result --request 0f4c2a6b8d0e1f3a5c7e9b1d3f5a7c9e0f2b4d6a8c0e1f3a5b7d9f1a3c5e7b9d --token 6f1d3b5a7c9e0f2a4c6e8b0d2f4a6c8e --input - --json"###,
        native: r###"cd /srv/shop && echo -E '{"progress":"The down() migration runs DROP TABLE invoices.","verified":true,"learnings":[]}' | infra-engine memory --root /srv/shop native-result --request 0f4c2a6b8d0e1f3a5c7e9b1d3f5a7c9e0f2b4d6a8c0e1f3a5b7d9f1a3c5e7b9d --token 6f1d3b5a7c9e0f2a4c6e8b0d2f4a6c8e --input - --json"###,
    },
    RetiredMemoryLauncher {
        case_index: 108,
        legacy: r###"printf '%s' '{"progress":"TRUNCATE TABLE invoices resets the counters."}' | (cd /srv/shop && python3 memory-bank/scripts/context.py --root /srv/shop native-result --input - --json)"###,
        native: r###"printf '%s' '{"progress":"TRUNCATE TABLE invoices resets the counters."}' | (cd /srv/shop && infra-engine memory --root /srv/shop native-result --input - --json)"###,
    },
    RetiredMemoryLauncher {
        case_index: 109,
        legacy: r###"(cd /srv/shop && python3 memory-bank/scripts/context.py --root /srv/shop native-result --input - --json) <<'JSON'
{"progress":"The rollback runs DROP SCHEMA reports."}
JSON"###,
        native: r###"(cd /srv/shop && infra-engine memory --root /srv/shop native-result --input - --json) <<'JSON'
{"progress":"The rollback runs DROP SCHEMA reports."}
JSON"###,
    },
    RetiredMemoryLauncher {
        case_index: 110,
        legacy: r###"python3 memory-bank/scripts/context.py retrieve --query 'why does the rollback DROP TABLE invoices'"###,
        native: r###"infra-engine memory retrieve --query 'why does the rollback DROP TABLE invoices'"###,
    },
];

fn category(command: &str, policy: &Policy) -> Option<String> {
    match validate(command, policy) {
        Ok(category) => category,
        Err(error) => Some(error.message),
    }
}

#[test]
fn frozen_shell_corpus_matches_all_five_framework_scopes() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("frozen corpus JSON");
    assert_eq!(corpus.cases.len(), 331);
    assert_eq!(corpus.editions, FRAMEWORKS);
    let mut failures = Vec::new();
    for framework in FRAMEWORKS {
        let policy = Policy::for_framework(framework);
        for (index, case) in corpus.cases.iter().enumerate() {
            let retired = RETIRED_MEMORY_LAUNCHERS
                .iter()
                .find(|entry| entry.case_index == index);
            let command = if let Some(retired) = retired {
                assert_eq!(case.command, retired.legacy);
                assert!(case.expect == Expectation::Pass);
                retired.native
            } else {
                &case.command
            };
            let actual = category(command, &policy);
            if actual.is_some() != case.blocks(framework) {
                failures.push(format!(
                    "case {index}, {framework}: decision differs ({}) -> got {actual:?} for {}",
                    case.note, command
                ));
            } else if let (Some(expected), Some(actual)) = (&case.category, actual)
                && !actual.starts_with(expected)
            {
                failures.push(format!(
                    "case {index}, {framework}: category differs ({}): expected {expected}, got {actual}",
                    case.note
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn encoded_shell_children_and_launchers_stay_protected() {
    for framework in FRAMEWORKS {
        let policy = Policy::for_framework(framework);
        for cmd in [
            r#"bash -c "printf '%s' 'DELETE FROM users;' | php artisan db""#,
            "bash -c \"php artisan db\" <<'SQL'\nDELETE FROM users;\nSQL",
            "bash --noprofile --norc -eo pipefail -c 'git reset --hard'",
        ] {
            assert!(category(cmd, &policy).is_some(), "{framework}: {cmd}");
        }
        assert!(category("bash --noprofile -c 'git status'", &policy).is_none());
    }
}

#[test]
fn ecosystem_presets_block_destructive_and_secret_patterns() {
    // Node preset
    let node = Policy::for_preset(Preset::Node);
    for cmd in [
        "npx prisma migrate reset --force",
        "pnpm dlx prisma migrate reset",
        "drizzle-kit drop",
        "npm publish",
        "cat .npmrc",
        "ssh prod-box 'npm publish'",
    ] {
        assert!(category(cmd, &node).is_some(), "node should block: {cmd}");
    }
    for cmd in [
        "npm publish --dry-run",
        "npm test",
        "cat .env.example",
        "npx prisma migrate status",
    ] {
        assert!(category(cmd, &node).is_none(), "node should allow: {cmd}");
    }

    // Python preset
    let python = Policy::for_preset(Preset::Python);
    for cmd in [
        "alembic downgrade base",
        "uv run alembic downgrade -1",
        "python3 manage.py flush --noinput",
        "twine upload dist/*",
        "uv publish",
        "cat ~/.pypirc",
    ] {
        assert!(
            category(cmd, &python).is_some(),
            "python should block: {cmd}"
        );
    }
    for cmd in [
        "uv publish --dry-run",
        "pytest -q",
        "python3 -m unittest",
        "alembic upgrade head",
    ] {
        assert!(
            category(cmd, &python).is_none(),
            "python should allow: {cmd}"
        );
    }

    // Rust preset
    let rust = Policy::for_preset(Preset::Rust);
    for cmd in [
        "cargo publish",
        "cargo yank --version 0.1.0",
        "sqlx database drop -y",
        "diesel database reset",
        "cat ~/.cargo/credentials.toml",
    ] {
        assert!(category(cmd, &rust).is_some(), "rust should block: {cmd}");
    }
    for cmd in [
        "cargo publish --dry-run",
        "cargo test --workspace",
        "sqlx migrate run",
    ] {
        assert!(category(cmd, &rust).is_none(), "rust should allow: {cmd}");
    }

    // Generic outside-repo rm
    let generic = Policy::for_preset(Preset::Generic);
    let repo = Path::new("/home/dev/my-project");
    assert!(
        validate_in_repo("rm -rf /etc/systemd/system", &generic, Some(repo))
            .unwrap()
            .is_some()
    );
    assert!(
        validate_in_repo("rm -rf /home/dev/other-project", &generic, Some(repo))
            .unwrap()
            .is_some()
    );
    assert!(
        validate_in_repo("rm -rf /home/dev/my-project/target", &generic, Some(repo))
            .unwrap()
            .is_none()
    );
}
