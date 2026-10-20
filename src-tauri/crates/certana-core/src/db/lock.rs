// SPDX-License-Identifier: GPL-3.0-or-later
//! Application-level writer lock next to the database file (`<db>.lock`). SQLite's own locks are
//! unreliable on SMB/NFS, so this file tells other Certana instances who is writing.
//!
//! The holder refreshes the lock every [`REFRESH_EVERY`] from a background thread. A lock whose
//! last refresh is older than [`STALE_AFTER`] counts as abandoned (crash, unplugged laptop) and may
//! be taken over. If the holder later finds a foreign identity in the file (its machine was
//! suspended for longer than `STALE_AFTER`), the guard flags the lock as lost and the database
//! refuses further writes until it is reopened.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::Result;

/// A lock whose last refresh is older than this is considered abandoned.
pub const STALE_AFTER: Duration = Duration::minutes(10);
/// How often the holder rewrites its lock file.
pub const REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub host: String,
    pub user: String,
    pub pid: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub acquired_at: OffsetDateTime,
    /// Last heartbeat of the holder; absent in files written by Certana ≤ 1.0.1.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub refreshed_at: Option<OffsetDateTime>,
}

impl LockInfo {
    /// When the holder was last seen alive.
    pub fn last_seen(&self) -> OffsetDateTime {
        self.refreshed_at.unwrap_or(self.acquired_at)
    }

    /// Same process on the same machine.
    pub fn same_holder(&self, other: &LockInfo) -> bool {
        self.pid == other.pid && self.host == other.host
    }
}

pub enum LockState {
    Acquired(LockGuard),
    HeldBy(LockInfo),
}

/// Keeps the lock fresh while alive; removes the lock file on drop, but only if it still carries
/// our own identity.
pub struct LockGuard {
    path: PathBuf,
    info: LockInfo,
    lost: Arc<AtomicBool>,
    stop: Option<Sender<()>>,
    heartbeat: Option<JoinHandle<()>>,
}

impl LockGuard {
    /// True once another instance has overwritten our lock (we must not write any more).
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(h) = self.heartbeat.take() {
            let _ = h.join();
        }
        if !self.is_lost()
            && let Ok(Some(current)) = read_lock(&self.path)
            && current.same_holder(&self.info)
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
        refreshed_at: None,
    }
}

/// Acquire the lock for `db_path`, refreshing it every [`REFRESH_EVERY`].
pub fn acquire(db_path: &Path) -> Result<LockState> {
    acquire_with(db_path, REFRESH_EVERY)
}

/// Like [`acquire`] with a custom heartbeat interval (tests).
pub fn acquire_with(db_path: &Path, refresh_every: std::time::Duration) -> Result<LockState> {
    let path = lock_path(db_path);
    if let Some(existing) = read_lock(&path)? {
        let age = OffsetDateTime::now_utc() - existing.last_seen();
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
    let lost = Arc::new(AtomicBool::new(false));
    let (stop, rx) = mpsc::channel::<()>();
    let heartbeat = {
        let path = path.clone();
        let info = info.clone();
        let lost = Arc::clone(&lost);
        std::thread::Builder::new()
            .name("certana-lock-heartbeat".into())
            .spawn(move || {
                loop {
                    match rx.recv_timeout(refresh_every) {
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    if !refresh(&path, &info) {
                        lost.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            })?
    };
    Ok(LockState::Acquired(LockGuard {
        path,
        info,
        lost,
        stop: Some(stop),
        heartbeat: Some(heartbeat),
    }))
}

/// Rewrite our lock with a fresh timestamp. Returns `false` when the file now belongs to someone
/// else. A missing file (cleaned up by hand) is recreated; a transient I/O error is retried next time.
fn refresh(path: &Path, info: &LockInfo) -> bool {
    match read_lock(path) {
        Ok(Some(current)) if !current.same_holder(info) => {
            tracing::warn!(
                holder = %format!("{}@{}", current.user, current.host),
                "database write lock was taken over by another Certana instance"
            );
            return false;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "could not read database lock file");
            return true;
        }
    }
    let fresh = LockInfo {
        refreshed_at: Some(OffsetDateTime::now_utc()),
        ..info.clone()
    };
    if let Err(e) = write_lock(path, &fresh) {
        tracing::warn!(error = %e, "could not refresh database lock file");
    }
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: std::time::Duration = std::time::Duration::from_millis(30);

    fn wait_for(cond: impl Fn() -> bool) {
        for _ in 0..200 {
            if cond() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("condition not met in time");
    }

    #[test]
    fn heartbeat_refreshes_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("x.db");
        let LockState::Acquired(guard) = acquire_with(&db, FAST).unwrap() else {
            panic!("lock should be free");
        };
        wait_for(|| {
            read_lock(&lock_path(&db))
                .unwrap()
                .is_some_and(|l| l.refreshed_at.is_some())
        });
        assert!(!guard.is_lost());
        drop(guard);
        assert!(!lock_path(&db).exists());
    }

    #[test]
    fn takeover_marks_lock_lost_and_keeps_foreign_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("x.db");
        let LockState::Acquired(guard) = acquire_with(&db, FAST).unwrap() else {
            panic!("lock should be free");
        };
        let foreign = LockInfo {
            host: "other-host".into(),
            user: "someone".into(),
            pid: 4242,
            acquired_at: OffsetDateTime::now_utc(),
            refreshed_at: None,
        };
        write_lock(&lock_path(&db), &foreign).unwrap();
        wait_for(|| guard.is_lost());
        drop(guard);
        let current = read_lock(&lock_path(&db)).unwrap().unwrap();
        assert_eq!(current.host, "other-host");
    }

    #[test]
    fn stale_is_judged_by_last_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("x.db");
        let old = LockInfo {
            host: "other-host".into(),
            user: "someone".into(),
            pid: 4242,
            acquired_at: OffsetDateTime::now_utc() - Duration::hours(5),
            refreshed_at: Some(OffsetDateTime::now_utc() - Duration::minutes(1)),
        };
        write_lock(&lock_path(&db), &old).unwrap();
        assert!(matches!(
            acquire_with(&db, FAST).unwrap(),
            LockState::HeldBy(_)
        ));
        let abandoned = LockInfo {
            refreshed_at: Some(OffsetDateTime::now_utc() - Duration::minutes(11)),
            ..old
        };
        write_lock(&lock_path(&db), &abandoned).unwrap();
        assert!(matches!(
            acquire_with(&db, FAST).unwrap(),
            LockState::Acquired(_)
        ));
    }

    /// Lock files written by 1.0.x have no `refreshed_at`.
    #[test]
    fn legacy_lock_file_parses() {
        let json = r#"{"host":"h","user":"u","pid":1,"acquired_at":"2026-10-20T06:54:40Z"}"#;
        let info: LockInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.last_seen(), info.acquired_at);
    }
}
