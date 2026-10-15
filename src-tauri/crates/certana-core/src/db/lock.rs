// SPDX-License-Identifier: GPL-3.0-or-later
//! Application-level writer lock next to the database file (`<db>.lock`). SQLite's own locks are
//! unreliable on SMB/NFS, so this file tells other Certana instances who is writing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::Result;

/// A lock older than this is considered abandoned (crashed process, unplugged laptop).
pub const STALE_AFTER: Duration = Duration::minutes(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub host: String,
    pub user: String,
    pub pid: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub acquired_at: OffsetDateTime,
}

pub enum LockState {
    Acquired(LockGuard),
    HeldBy(LockInfo),
}

/// Removes the lock file on drop – but only if it still carries our own identity.
pub struct LockGuard {
    path: PathBuf,
    info: LockInfo,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Ok(Some(current)) = read_lock(&self.path)
            && current.pid == self.info.pid
            && current.host == self.info.host
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub fn lock_path(db_path: &Path) -> PathBuf {
    let mut p = db_path.as_os_str().to_owned();
    p.push(".lock");
    PathBuf::from(p)
}

pub fn current_info() -> LockInfo {
    LockInfo {
        host: hostname(),
        user: username(),
        pid: std::process::id(),
        acquired_at: OffsetDateTime::now_utc(),
    }
}

pub fn acquire(db_path: &Path) -> Result<LockState> {
    let path = lock_path(db_path);
    if let Some(existing) = read_lock(&path)? {
        let age = OffsetDateTime::now_utc() - existing.acquired_at;
        let same_host = existing.host == hostname();
        let is_me = same_host && existing.pid == std::process::id();
        // A lock from this host whose process is gone (crash, kill) is abandoned regardless of age.
        let abandoned = same_host && !process_alive(existing.pid);
        if age < STALE_AFTER && !is_me && !abandoned {
            return Ok(LockState::HeldBy(existing));
        }
    }
    let info = current_info();
    write_lock(&path, &info)?;
    Ok(LockState::Acquired(LockGuard { path, info }))
}

/// Whether a process with this id exists on the local machine. Only Linux can answer this cheaply
/// (procfs); elsewhere we conservatively assume it is alive.
pub fn process_alive(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        Path::new(&format!("/proc/{pid}")).exists()
    } else {
        true
    }
}

pub fn read_lock(path: &Path) -> Result<Option<LockInfo>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn write_lock(path: &Path, info: &LockInfo) -> Result<()> {
    let json = serde_json::to_vec_pretty(info).expect("lock info serialises");
    std::fs::write(path, json)?;
    Ok(())
}

pub fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "unknown-host".into())
}

pub fn username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".into())
}
