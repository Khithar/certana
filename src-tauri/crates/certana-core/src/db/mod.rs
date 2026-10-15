// SPDX-License-Identifier: GPL-3.0-or-later
//! SQLite persistence. One `Database` per opened file; all key columns hold PBES2-encrypted PEM.
//!
//! Sharing rules (see vault «Shared-SQLite-Betrieb»): on a local file system WAL is used; on a
//! network mount Certana switches to rollback journal + exclusive locking and guards the file
//! with its own lock file so only one writer exists at a time.

pub mod audit;
pub mod fs;
pub mod lock;
pub mod models;
pub mod repo_ca;
pub mod repo_cert;
pub mod repo_crl;
pub mod settings;
mod time_fmt;

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::{M, Migrations};
use serde::{Deserialize, Serialize};

use crate::{CertanaError, Result};
use lock::{LockGuard, LockInfo, LockState};

pub use time_fmt::{format_ts, parse_ts};

/// Schema migrations, applied in order via `PRAGMA user_version`.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("sql/0001_init.sql"))])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageMode {
    /// Local file system: WAL journal, normal locking.
    Local,
    /// Network share (cifs/nfs/sshfs …): DELETE journal, exclusive locking, lock file.
    Shared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInfo {
    pub path: PathBuf,
    pub mode: StorageMode,
    pub read_only: bool,
    /// Who holds the write lock when we had to fall back to read-only.
    pub held_by: Option<LockInfo>,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Default)]
pub struct OpenOptions {
    /// Treat the file as shared even on a local file system (tests, user override).
    pub force_shared: bool,
    /// Open read-only regardless of locks.
    pub read_only: bool,
}

pub struct Database {
    conn: Connection,
    info: OpenInfo,
    _lock: Option<LockGuard>,
}

impl Database {
    /// Open (or create) a database file, apply migrations, set pragmas and acquire the lock.
    pub fn open(path: impl AsRef<Path>, options: OpenOptions) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mode = if options.force_shared || fs::is_network_path(&path) {
            StorageMode::Shared
        } else {
            StorageMode::Local
        };

        let (lock, read_only, held_by) = if options.read_only {
            (None, true, None)
        } else {
            match lock::acquire(&path)? {
                LockState::Acquired(guard) => (Some(guard), false, None),
                LockState::HeldBy(info) => (None, true, Some(info)),
            }
        };

        let flags = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
        };
        if read_only && !path.exists() {
            return Err(CertanaError::Invalid(format!(
                "database {} does not exist",
                path.display()
            )));
        }
        let mut conn = Connection::open_with_flags(&path, flags)?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        if !read_only {
            match mode {
                StorageMode::Local => {
                    conn.pragma_update(None, "journal_mode", "WAL")?;
                    conn.pragma_update(None, "synchronous", "NORMAL")?;
                }
                StorageMode::Shared => {
                    conn.pragma_update(None, "locking_mode", "EXCLUSIVE")?;
                    conn.pragma_update(None, "journal_mode", "DELETE")?;
                    conn.pragma_update(None, "synchronous", "FULL")?;
                }
            }
            migrations().to_latest(&mut conn)?;
        }
        let schema_version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

        Ok(Self {
            conn,
            info: OpenInfo {
                path,
                mode,
                read_only,
                held_by,
                schema_version,
            },
            _lock: lock,
        })
    }

    /// In-memory database for tests and dry runs.
    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations().to_latest(&mut conn)?;
        Ok(Self {
            conn,
            info: OpenInfo {
                path: PathBuf::from(":memory:"),
                mode: StorageMode::Local,
                read_only: false,
                held_by: None,
                schema_version: 1,
            },
            _lock: None,
        })
    }

    pub fn info(&self) -> &OpenInfo {
        &self.info
    }

    pub fn is_read_only(&self) -> bool {
        self.info.read_only
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    pub(crate) fn ensure_writable(&self) -> Result<()> {
        if self.info.read_only {
            return Err(CertanaError::Invalid("database is open read-only".into()));
        }
        Ok(())
    }

    /// Consistent online backup to `dest` (works while the DB is in use).
    pub fn backup_to(&self, dest: impl AsRef<Path>) -> Result<()> {
        let dest = dest.as_ref();
        if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut target = Connection::open(dest)?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut target)?;
        backup.run_to_completion(256, std::time::Duration::from_millis(10), None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        migrations().validate().unwrap();
    }

    #[test]
    fn open_local_creates_file_and_wal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("certana.db");
        let db = Database::open(&path, OpenOptions::default()).unwrap();
        assert_eq!(db.info().mode, StorageMode::Local);
        assert!(!db.is_read_only());
        assert_eq!(db.info().schema_version, 1);
        let jm: String = db
            .conn()
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(jm, "wal");
        assert!(path.exists());
    }

    #[test]
    fn shared_mode_uses_lock_file_and_second_opener_is_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.db");
        let first = Database::open(
            &path,
            OpenOptions {
                force_shared: true,
                read_only: false,
            },
        )
        .unwrap();
        assert_eq!(first.info().mode, StorageMode::Shared);
        let jm: String = first
            .conn()
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(jm, "delete");
        assert!(lock::lock_path(&path).exists());
        // Simulate a different process holding the lock: rewrite the lock file with another pid.
        let mut other = lock::read_lock(&lock::lock_path(&path)).unwrap().unwrap();
        other.pid += 1;
        other.host = "other-host".into();
        lock::write_lock(&lock::lock_path(&path), &other).unwrap();
        drop(first); // our guard must not delete a lock that is no longer ours
        assert!(lock::lock_path(&path).exists());
        let second = Database::open(
            &path,
            OpenOptions {
                force_shared: true,
                read_only: false,
            },
        )
        .unwrap();
        assert!(second.is_read_only());
        assert_eq!(second.info().held_by.as_ref().unwrap().host, "other-host");
        assert!(second.ensure_writable().is_err());
    }

    #[test]
    fn stale_lock_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stale.db");
        Database::open(&path, OpenOptions::default()).unwrap();
        let stale = LockInfo {
            host: "ghost".into(),
            user: "nobody".into(),
            pid: 999_999,
            acquired_at: ::time::OffsetDateTime::now_utc() - ::time::Duration::minutes(30),
        };
        lock::write_lock(&lock::lock_path(&path), &stale).unwrap();
        let db = Database::open(&path, OpenOptions::default()).unwrap();
        assert!(!db.is_read_only());
        assert_eq!(
            lock::read_lock(&lock::lock_path(&path))
                .unwrap()
                .unwrap()
                .host,
            lock::current_info().host
        );
    }

    #[test]
    fn backup_copies_data() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("a.db"), OpenOptions::default()).unwrap();
        db.set_setting("org", "Example").unwrap();
        db.backup_to(dir.path().join("backups").join("a-copy.db"))
            .unwrap();
        let copy = Database::open(
            dir.path().join("backups").join("a-copy.db"),
            OpenOptions::default(),
        )
        .unwrap();
        assert_eq!(copy.get_setting("org").unwrap().as_deref(), Some("Example"));
    }
}
