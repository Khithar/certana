// SPDX-License-Identifier: GPL-3.0-or-later
//! Key/value settings (export directory, subject defaults, auto-lock minutes, language …).

use rusqlite::{OptionalExtension as _, params};

use super::Database;
use crate::Result;

/// Well-known setting keys.
pub mod keys {
    pub const ORGANIZATION: &str = "subject.organization";
    pub const ORGANIZATIONAL_UNIT: &str = "subject.organizational_unit";
    pub const COUNTRY: &str = "subject.country";
    pub const STATE: &str = "subject.state";
    pub const LOCALITY: &str = "subject.locality";
    pub const EXPORT_DIR: &str = "export.dir";
    pub const CRL_EXPORT_DIR: &str = "crl.export_dir";
    pub const AUTO_LOCK_MINUTES: &str = "security.auto_lock_minutes";
    pub const EXPIRY_WARN_DAYS: &str = "notify.expiry_warn_days";
}

impl Database {
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.ensure_writable()?;
        self.conn().execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn conn_delete_setting(&self, key: &str) -> Result<()> {
        self.ensure_writable()?;
        self.conn()
            .execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn all_settings(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn()
            .prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
