//! Session command/edit loop counters in `$TMPDIR` and advisory `/tmp` writer leases.
use crate::{
    custody,
    error::{GuardError, Result},
};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

/// Extract sanitized alphanumeric session identifier from a hook payload.
pub fn session_id(payload: &Value) -> String {
    let source = payload["session_id"]
        .as_str()
        .or_else(|| payload["conversation_id"].as_str())
        .unwrap_or("");
    let value: String = source
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(64)
        .collect();
    if value.is_empty() {
        "shared".into()
    } else {
        value
    }
}

pub fn sanitize_session(raw: Option<&str>) -> String {
    let value: String = raw
        .unwrap_or("")
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(64)
        .collect();
    if value.is_empty() {
        "shared".into()
    } else {
        value
    }
}

/// Current effective user ID string.
pub fn uid() -> String {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() }.to_string()
    }
    #[cfg(not(unix))]
    {
        "0".into()
    }
}

/// POSIX CRC-32 `cksum` compatibility digest for repository-scoped `$TMPDIR` directories.
pub fn checksum(raw: &[u8]) -> u32 {
    let mut crc = 0u32;
    let mut feed = |byte: u8| {
        crc ^= (byte as u32) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04c1_1db7
            } else {
                crc << 1
            };
        }
    };
    for byte in raw {
        feed(*byte);
    }
    let mut length = raw.len();
    while length != 0 {
        feed((length & 255) as u8);
        length >>= 8;
    }
    !crc
}

pub fn repo_key(root: &Path) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        checksum(root.as_os_str().as_bytes())
    }
    #[cfg(not(unix))]
    {
        checksum(root.to_string_lossy().as_bytes())
    }
}

pub fn directory(root: &Path) -> Result<PathBuf> {
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = base.join(format!("claude-loop-detection-{}-{}", uid(), repo_key(root)));
    if !path.exists() {
        custody::ensure_directory(&path, true)?;
    }
    if !custody::owned_by_current_user(&path) || !path.is_dir() {
        return Err(GuardError::policy("Counter directory is not owned by user"));
    }
    Ok(path)
}

/// Standard MD5 hex digest used as collision-free counter filename suffix.
pub fn md5(bytes: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let bits = (bytes.len() as u64).wrapping_mul(8);
    let mut input = bytes.to_vec();
    input.push(128);
    while input.len() % 64 != 56 {
        input.push(0);
    }
    input.extend_from_slice(&bits.to_le_bytes());
    let mut state = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    for block in input.as_chunks::<64>().0 {
        let mut words = [0u32; 16];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            words[i] = u32::from_le_bytes(*word);
        }
        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        for i in 0..64 {
            let (f, g) = if i < 16 {
                ((b & c) | (!b & d), i)
            } else if i < 32 {
                ((d & b) | (!d & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | !d), (7 * i) % 16)
            };
            let next = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(words[g])
                    .rotate_left(S[i]),
            );
            a = d;
            d = c;
            c = b;
            b = next;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
    state
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_counter(path: &Path) -> u32 {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| text.len() <= 9 && text.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|text| text.parse::<u32>().ok())
        .unwrap_or(0)
}

/// Increment per-session command or edit counter, resetting command streak when a newer edit occurred.
pub fn advance(
    root: &Path,
    payload: &Value,
    kind: &str,
    identity: &str,
    reset_after_edit: bool,
) -> Result<u32> {
    let folder = directory(root)?;
    let mut image = identity.as_bytes().to_vec();
    if kind == "edit" {
        image.push(b'\n');
    }
    let target = folder.join(format!("{kind}-{}-{}", session_id(payload), md5(&image)));
    if target.exists() && !custody::owned_by_current_user(&target) {
        return Err(GuardError::policy("Counter file is not owned by user"));
    }
    let mut count = read_counter(&target);
    if reset_after_edit && count > 0 {
        let modified = fs::metadata(&target)?.modified()?;
        if fs::read_dir(&folder)?
            .filter_map(std::result::Result::ok)
            .any(|entry| {
                entry.file_name().to_string_lossy().starts_with("edit-")
                    && custody::owned_by_current_user(&entry.path())
                    && entry
                        .metadata()
                        .and_then(|meta| meta.modified())
                        .is_ok_and(|at| at > modified)
            })
        {
            count = 0;
        }
    }
    count = count.saturating_add(1);
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut output = options.open(target)?;
    if !output.metadata()?.is_file() {
        return Err(GuardError::policy("Counter target is not a regular file"));
    }
    writeln!(output, "{count}")?;
    Ok(count)
}

/// Clear all loop counters for `session` and prune counters older than 24h.
pub fn reset(root: &Path, session: &str) -> Result<u32> {
    let folder = directory(root)?;
    let now = SystemTime::now();
    let mut removed = 0u32;
    for entry in fs::read_dir(folder)?.filter_map(std::result::Result::ok) {
        let path = entry.path();
        if !custody::owned_by_current_user(&path) || !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let expired = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| now.duration_since(at).ok())
            .is_some_and(|age| age > Duration::from_secs(86400));
        if (name.starts_with(&format!("cmd-{session}-"))
            || name.starts_with(&format!("edit-{session}-"))
            || expired)
            && fs::remove_file(path).is_ok()
        {
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn writer_folder() -> Result<PathBuf> {
    let path = std::env::var_os("SUBAGENT_WRITE_LOCK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if !path.is_absolute() || !path.is_dir() {
        return Err(GuardError::invalid(
            "Write-agent lock directory configuration is invalid.",
        ));
    }
    Ok(path)
}

fn lock_ttl() -> Duration {
    let minutes = std::env::var("SUBAGENT_WRITE_LOCK_TTL_MINUTES")
        .ok()
        .filter(|text| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    Duration::from_secs(minutes.saturating_mul(60))
}

fn host_lock_prefix(host: &str) -> &'static str {
    if host == "cursor" { "cursor" } else { "claude" }
}

/// Attempt to acquire the exclusive advisory write-capable subagent lock for `root`.
/// Returns `Ok(Some(holder))` if another write-capable agent currently holds a fresh lease.
pub fn take_writer(root: &Path, host: &str, agent: &str) -> Result<Option<String>> {
    let folder = writer_folder()?;
    let key = repo_key(root);
    let name = format!("{}-write-agent-lock-{key}", host_lock_prefix(host));
    let _guard = custody::DrainGuard::acquire(&folder, &format!("{name}.guard"))?
        .ok_or_else(|| GuardError::conflict("Write-agent spawning is already in progress"))?;
    let path = folder.join(name);
    if path.exists() && !custody::owned_by_current_user(&path) {
        return Err(GuardError::policy("Write-agent lock is not owned by user"));
    }
    let ttl = lock_ttl();
    if path.is_file() {
        let fresh = fs::metadata(&path)?
            .modified()
            .ok()
            .and_then(|at| SystemTime::now().duration_since(at).ok())
            .is_some_and(|age| age < ttl);
        if fresh {
            let holder = custody::read_regular(&path, 4096)?;
            if !holder.is_empty() {
                return Ok(Some(
                    String::from_utf8_lossy(&holder).chars().take(64).collect(),
                ));
            }
        }
    }
    custody::atomic_write(&path, agent.as_bytes())?;
    Ok(None)
}

/// Release the write-agent lock held by `agent` (or any holder if `agent` is empty).
pub fn release_writer(root: &Path, agent: &str) -> Result<bool> {
    let folder = writer_folder()?;
    let mut released = false;
    for prefix in ["claude", "cursor"] {
        let name = format!("{prefix}-write-agent-lock-{}", repo_key(root));
        let path = folder.join(&name);
        let Some(_guard) = custody::DrainGuard::acquire(&folder, &format!("{name}.guard"))? else {
            continue;
        };
        if custody::owned_by_current_user(&path) {
            let matches_holder = agent.is_empty()
                || custody::read_regular(&path, 4096).ok().as_deref() == Some(agent.as_bytes());
            if matches_holder && fs::remove_file(&path).is_ok() {
                released = true;
            }
        }
    }
    Ok(released)
}

#[derive(Clone, Debug, Serialize)]
pub struct LoopStatus {
    pub repo_root: String,
    pub repo_key: u32,
    pub counter_dir: String,
    pub session_id: String,
    pub command_counters: usize,
    pub max_command_count: u32,
    pub edit_counters: usize,
    pub max_edit_count: u32,
    pub active_writer_lock: Option<String>,
}

/// Inspect active session command/edit counters and any active write-agent lock.
pub fn inspect_status(root: &Path, session: Option<&str>) -> Result<LoopStatus> {
    let session = sanitize_session(session);
    let key = repo_key(root);
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let folder = base.join(format!("claude-loop-detection-{}-{key}", uid()));

    let (mut cmd_n, mut max_cmd, mut edit_n, mut max_edit) = (0usize, 0u32, 0usize, 0u32);
    if folder.is_dir() && custody::owned_by_current_user(&folder) {
        for entry in fs::read_dir(&folder)?.filter_map(std::result::Result::ok) {
            let p = entry.path();
            if !p.is_file() || !custody::owned_by_current_user(&p) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("cmd-{session}-")) {
                cmd_n += 1;
                max_cmd = max_cmd.max(read_counter(&p));
            } else if name.starts_with(&format!("edit-{session}-")) {
                edit_n += 1;
                max_edit = max_edit.max(read_counter(&p));
            }
        }
    }

    let mut active_writer = None;
    if let Ok(lock_dir) = writer_folder() {
        let ttl = lock_ttl();
        for prefix in ["claude", "cursor"] {
            let p = lock_dir.join(format!("{prefix}-write-agent-lock-{key}"));
            if p.is_file()
                && custody::owned_by_current_user(&p)
                && fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|at| SystemTime::now().duration_since(at).ok())
                    .is_some_and(|age| age < ttl)
                && let Ok(holder) = custody::read_regular(&p, 4096)
                && !holder.is_empty()
            {
                active_writer = Some(String::from_utf8_lossy(&holder).chars().take(64).collect());
                break;
            }
        }
    }

    Ok(LoopStatus {
        repo_root: root.display().to_string(),
        repo_key: key,
        counter_dir: folder.display().to_string(),
        session_id: session,
        command_counters: cmd_n,
        max_command_count: max_cmd,
        edit_counters: edit_n,
        max_edit_count: max_edit,
        active_writer_lock: active_writer,
    })
}
