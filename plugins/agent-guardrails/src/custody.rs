//! Self-contained filesystem custody, strict JSON parsing, canonical hashing, and flock locks.
use crate::error::{ErrorKind, GuardError, Result};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Lowercase SHA-256 hex digest of raw bytes.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Detect symbolic links (and Windows reparse authority points).
pub fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

/// Return true when `path` is not a symlink and belongs to the current effective UID on Unix.
pub fn owned_by_current_user(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if linked(&metadata) {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.uid() == unsafe { libc::geteuid() }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Lexical path normalization without resolving symlinks beyond `current_dir`.
pub fn normalize_lexical(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(result.components().next_back(), Some(Component::Normal(_))) {
                    result.pop();
                }
            }
            _ => result.push(part.as_os_str()),
        }
    }
    result
}

/// Make a path lexically absolute relative to the current working directory.
pub fn absolute_lexical(path: &Path) -> Result<PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize_lexical(&joined))
}

/// Return true when `path` is a packaged `agent-guardrails` plugin directory.
pub fn is_plugin_dir(path: &Path) -> bool {
    (path.join(".codex-plugin/plugin.json").is_file()
        || path.join(".claude-plugin/plugin.json").is_file())
        && path.join("bin/guardrails").is_file()
}

/// Return true when `start` lies inside a packaged plugin directory (before reaching `.git`).
pub fn inside_plugin_dir(start: &Path) -> bool {
    let abs = absolute_lexical(start).unwrap_or_else(|_| start.to_path_buf());
    for ancestor in abs.ancestors() {
        if is_plugin_dir(ancestor) {
            return true;
        }
        if ancestor.join(".git").exists() {
            break;
        }
    }
    false
}

/// Walk ancestor directories to locate the governing project root (`.agent-guardrails`, `.git`,
/// or `project-brain`), falling back to `start`.
pub fn discover_repo_root(start: &Path) -> PathBuf {
    let abs = absolute_lexical(start).unwrap_or_else(|_| start.to_path_buf());
    for ancestor in abs.ancestors() {
        if ancestor.join(".agent-guardrails").is_dir()
            || ancestor.join(".git").exists()
            || ancestor.join("project-brain").is_dir()
        {
            return fs::canonicalize(ancestor).unwrap_or_else(|_| ancestor.to_path_buf());
        }
    }
    fs::canonicalize(&abs).unwrap_or(abs)
}

/// Confirm `path` is either absent (`Ok(false)`) or a real non-symlink directory (`Ok(true)`).
pub fn admit_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !linked(&metadata) => Ok(true),
        Ok(_) => Err(GuardError::policy(
            "Custody directory is linked or not a directory",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Create a directory hierarchy safely, rejecting symlinked components.
pub fn ensure_directory(path: &Path, private: bool) -> Result<()> {
    if admit_directory(path)? {
        return Ok(());
    }
    if let Some(parent) = path.parent()
        && parent != path
    {
        ensure_directory(parent, private)?;
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(if private { 0o700 } else { 0o755 });
    }
    match builder.create(path) {
        Ok(()) => {
            #[cfg(unix)]
            if private {
                use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
                let dir = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(path)?;
                dir.set_permissions(fs::Permissions::from_mode(0o700))?;
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => admit_directory(path)?
            .then_some(())
            .ok_or_else(|| GuardError::unavailable("Directory disappeared during creation")),
        Err(error) => Err(error.into()),
    }
}

/// Read an ordinary non-symlink file up to `max_bytes`.
pub fn read_regular(path: &Path, max_bytes: usize) -> Result<Vec<u8>> {
    let observed = fs::symlink_metadata(path)?;
    if linked(&observed) || !observed.is_file() {
        return Err(GuardError::policy("Target is linked or not a regular file"));
    }
    if observed.len() > max_bytes as u64 {
        return Err(GuardError::invalid("File exceeds configured byte limit"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(GuardError::policy("Opened handle is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(GuardError::invalid("File exceeds configured byte limit"));
    }
    Ok(bytes)
}

/// Read UTF-8 regular text file with LF normalization.
pub fn read_text(path: &Path, max_bytes: usize) -> Result<String> {
    let text = String::from_utf8(read_regular(path, max_bytes)?)
        .map_err(|_| GuardError::invalid("Source file is not valid UTF-8"))?;
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Atomically write bytes to `path` using a private sibling temporary file, fsync, and rename.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| GuardError::invalid("Atomic destination has no parent directory"))?;
    ensure_directory(parent, false)?;
    if let Ok(meta) = fs::symlink_metadata(path)
        && (linked(&meta) || !meta.is_file())
    {
        return Err(GuardError::policy(
            "Atomic destination is linked or not a regular file",
        ));
    }
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temporary = parent.join(format!(
        ".{}.{}.{}.{}",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("guardrails"),
        std::process::id(),
        seq,
        nanos
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let result = (|| -> Result<()> {
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Advisory non-blocking exclusive `flock` guard for coordinating writer locks in `/tmp`.
pub struct DrainGuard(File);

impl DrainGuard {
    pub fn acquire(folder: &Path, name: &str) -> Result<Option<Self>> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
        }
        let file = match options.open(folder.join(format!(".{name}.drain-lock"))) {
            Ok(file) => file,
            Err(_) => return Ok(None),
        };
        if !file.metadata()?.is_file() {
            return Ok(None);
        }
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Ok(None);
            }
        }
        Ok(Some(Self(file)))
    }
}

impl Drop for DrainGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

/// Deterministic compact canonical JSON with recursively sorted object keys.
pub fn canonical_json(value: &Value) -> Result<String> {
    fn sort_value(value: &Value) -> Value {
        match value {
            Value::Array(items) => Value::Array(items.iter().map(sort_value).collect()),
            Value::Object(map) => {
                let mut ordered = BTreeMap::new();
                for (k, v) in map {
                    ordered.insert(k.clone(), sort_value(v));
                }
                let mut out = Map::new();
                for (k, v) in ordered {
                    out.insert(k, v);
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }
    Ok(serde_json::to_string(&sort_value(value))?)
}

fn strict_refused(code: &str, message: &str) -> GuardError {
    GuardError::coded(
        ErrorKind::InvalidInput,
        format!("strict-json:{code}"),
        message,
    )
}

/// Bounded strict JSON parser rejecting oversized payloads, excess nesting, duplicate keys,
/// control characters, or trailing non-whitespace tokens.
pub fn strict_parse_json(
    bytes: &[u8],
    max_bytes: usize,
    max_depth: usize,
    max_nodes: usize,
) -> Result<Value> {
    if bytes.len() > max_bytes {
        return Err(strict_refused("byte-limit", "JSON body exceeds byte limit."));
    }
    std::str::from_utf8(bytes)
        .map_err(|_| strict_refused("encoding", "JSON input is not valid UTF-8."))?;
    let mut parser = StrictParser {
        bytes,
        at: 0,
        max_depth: max_depth.min(512),
        max_nodes,
        nodes: 0,
    };
    let value = parser.value(0)?;
    parser.space();
    if parser.at != bytes.len() {
        return Err(strict_refused("syntax", "Trailing JSON data."));
    }
    Ok(value)
}

struct StrictParser<'a> {
    bytes: &'a [u8],
    at: usize,
    max_depth: usize,
    max_nodes: usize,
    nodes: usize,
}

impl StrictParser<'_> {
    fn space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b" \t\r\n".contains(b))
        {
            self.at += 1;
        }
    }

    fn take(&mut self, byte: u8) -> bool {
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Result<String> {
        let begin = self.at;
        if !self.take(b'"') {
            return Err(strict_refused("syntax", "JSON object keys must be strings."));
        }
        while let Some(byte) = self.bytes.get(self.at).copied() {
            self.at += 1;
            if byte == b'\\' {
                if self.at == self.bytes.len() {
                    return Err(strict_refused("encoding", "Incomplete JSON escape."));
                }
                self.at += 1;
            } else if byte == b'"' {
                return serde_json::from_slice::<String>(&self.bytes[begin..self.at])
                    .map_err(|_| strict_refused("encoding", "Invalid JSON string or escape."));
            } else if byte < 0x20 {
                return Err(strict_refused(
                    "encoding",
                    "Unescaped control character in JSON string.",
                ));
            }
        }
        Err(strict_refused("syntax", "Unterminated JSON string."))
    }

    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > self.max_depth {
            return Err(strict_refused("depth-limit", "JSON nesting limit exceeded."));
        }
        if self.nodes >= self.max_nodes {
            return Err(strict_refused("node-limit", "JSON value count exceeded."));
        }
        self.nodes += 1;
        self.space();
        match self.bytes.get(self.at).copied() {
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'{') => {
                self.at += 1;
                self.space();
                let mut members = Map::new();
                if self.take(b'}') {
                    return Ok(Value::Object(members));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.space();
                    if !self.take(b':') {
                        return Err(strict_refused("syntax", "JSON object member has no colon."));
                    }
                    if members.contains_key(&key) {
                        return Err(strict_refused("duplicate-key", "Duplicate JSON field."));
                    }
                    let val = self.value(depth + 1)?;
                    members.insert(key, val);
                    self.space();
                    if self.take(b'}') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err(strict_refused("syntax", "Invalid JSON object separator."));
                    }
                }
                Ok(Value::Object(members))
            }
            Some(b'[') => {
                self.at += 1;
                self.space();
                let mut values = Vec::new();
                if self.take(b']') {
                    return Ok(Value::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.space();
                    if self.take(b']') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err(strict_refused("syntax", "Invalid JSON array separator."));
                    }
                }
                Ok(Value::Array(values))
            }
            Some(_) => {
                let begin = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| !b" \t\r\n,]}".contains(b))
                {
                    self.at += 1;
                }
                if begin == self.at {
                    return Err(strict_refused("syntax", "Expected a JSON value."));
                }
                let raw = std::str::from_utf8(&self.bytes[begin..self.at])
                    .map_err(|_| strict_refused("encoding", "Invalid UTF-8 token."))?;
                serde_json::from_str::<Value>(raw)
                    .map_err(|_| strict_refused("syntax", "Invalid JSON scalar."))
            }
            None => Err(strict_refused("syntax", "Unexpected end of JSON input.")),
        }
    }
}
