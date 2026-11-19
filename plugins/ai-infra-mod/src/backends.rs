use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BackendKind {
    Graph,
    Orch,
    Context,
    Skill,
    Guard,
}

impl BackendKind {
    pub const ALL: [BackendKind; 5] = [
        BackendKind::Graph,
        BackendKind::Orch,
        BackendKind::Context,
        BackendKind::Skill,
        BackendKind::Guard,
    ];

    pub fn id(self) -> &'static str {
        match self {
            BackendKind::Graph => "graph-map",
            BackendKind::Orch => "system-orchestration",
            BackendKind::Context => "context-graph",
            BackendKind::Skill => "skill-foundry",
            BackendKind::Guard => "agent-guardrails",
        }
    }

    pub fn cli_namespace(self) -> &'static str {
        match self {
            BackendKind::Graph => "graph",
            BackendKind::Orch => "orch",
            BackendKind::Context => "context",
            BackendKind::Skill => "skill",
            BackendKind::Guard => "guard",
        }
    }

    pub fn binary_name(self) -> &'static str {
        let base = match self {
            BackendKind::Graph => "graph-map",
            BackendKind::Orch => "orch-engine",
            BackendKind::Context => "context-graph-engine",
            BackendKind::Skill => "skill-foundry",
            BackendKind::Guard => "guardrails",
        };
        if cfg!(windows) {
            match self {
                BackendKind::Graph => "graph-map.exe",
                BackendKind::Orch => "orch-engine.exe",
                BackendKind::Context => "context-graph-engine.exe",
                BackendKind::Skill => "skill-foundry.exe",
                BackendKind::Guard => "guardrails.exe",
            }
        } else {
            base
        }
    }
}

#[derive(Clone, Debug)]
pub struct BackendInfo {
    pub kind: BackendKind,
    pub path: PathBuf,
    pub available: bool,
    pub version: Option<String>,
}

pub fn bin_dir() -> Result<PathBuf, String> {
    if let Some(dir) = env::var_os("AI_INFRA_BIN_DIR").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let exe = env::current_exe().map_err(|e| format!("Failed to locate ai-infra-mod binary: {e}"))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "ai-infra-mod executable has no parent directory".to_string())
}

pub fn locate_binary(kind: BackendKind) -> Result<PathBuf, String> {
    let dir = bin_dir()?;
    let candidate = dir.join(kind.binary_name());
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err(format!(
        "Missing sibling backend binary `{}` in {}",
        kind.binary_name(),
        dir.display()
    ))
}

pub fn probe_version(path: &Path) -> Option<String> {
    let out = Command::new(path).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

pub fn discover_all() -> Vec<BackendInfo> {
    BackendKind::ALL
        .into_iter()
        .map(|kind| match locate_binary(kind) {
            Ok(path) => {
                let version = probe_version(&path);
                BackendInfo {
                    kind,
                    path,
                    available: true,
                    version,
                }
            }
            Err(_) => {
                let path = bin_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(kind.binary_name());
                BackendInfo {
                    kind,
                    path,
                    available: false,
                    version: None,
                }
            }
        })
        .collect()
}
