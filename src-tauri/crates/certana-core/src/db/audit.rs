// SPDX-License-Identifier: GPL-3.0-or-later
//! Append-only audit trail. Details must never contain key material or passphrases.

use rusqlite::params;
use time::OffsetDateTime;

use super::models::AuditRecord;
use super::{Database, format_ts, parse_ts};
use crate::Result;

impl Database {
    pub fn audit(
        &self,
        action: &str,
        target_type: &str,
        target_id: impl ToString,
        details: serde_json::Value,
    ) -> Result<()> {
        self.ensure_writable()?;
        self.conn().execute(
            "INSERT INTO audit_log(ts, actor, host, action, target_type, target_id, details_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format_ts(OffsetDateTime::now_utc()),
                super::lock::username(),
                super::lock::hostname(),
                action,
                target_type,
                target_id.to_string(),
                details.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn recent_audit(&self, limit: u32) -> Result<Vec<AuditRecord>> {
        let mut stmt = self.conn().prepare(
            "SELECT id, ts, actor, host, action, target_type, target_id, details_json FROM audit_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            let ts: String = r.get(1)?;
            let details: String = r.get(7)?;
            Ok(AuditRecord {
                id: r.get(0)?,
                ts: parse_ts(&ts).map_err(|_| rusqlite::Error::InvalidQuery)?,
                actor: r.get(2)?,
                host: r.get(3)?,
                action: r.get(4)?,
                target_type: r.get(5)?,
                target_id: r.get(6)?,
                details: serde_json::from_str(&details).unwrap_or(serde_json::Value::Null),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// CSV export of the whole audit log (newest first).
    pub fn audit_csv(&self) -> Result<String> {
        let rows = self.recent_audit(u32::MAX)?;
        let mut out = String::from("id,ts,actor,host,action,target_type,target_id,details\n");
        for r in rows {
            let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                r.id,
                format_ts(r.ts),
                esc(&r.actor),
                esc(&r.host),
                esc(&r.action),
                esc(&r.target_type),
                esc(&r.target_id),
                esc(&r.details.to_string())
            ));
        }
        Ok(out)
    }
}
