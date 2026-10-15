// SPDX-License-Identifier: GPL-3.0-or-later
//! CRL history.

use rusqlite::{OptionalExtension as _, Row, params};
use time::OffsetDateTime;

use super::models::CrlRecord;
use super::{Database, format_ts, parse_ts};
use crate::Result;

const COLS: &str = "id, ca_id, crl_number, this_update, next_update, crl_der, exported_path";

fn map_crl(r: &Row<'_>) -> rusqlite::Result<CrlRecord> {
    let tu: String = r.get(3)?;
    let nu: String = r.get(4)?;
    Ok(CrlRecord {
        id: r.get(0)?,
        ca_id: r.get(1)?,
        crl_number: r.get(2)?,
        this_update: parse_ts(&tu).map_err(|_| rusqlite::Error::InvalidQuery)?,
        next_update: parse_ts(&nu).map_err(|_| rusqlite::Error::InvalidQuery)?,
        crl_der: r.get(5)?,
        exported_path: r.get(6)?,
    })
}

impl Database {
    pub fn insert_crl(
        &self,
        ca_id: i64,
        crl_number: i64,
        this_update: OffsetDateTime,
        next_update: OffsetDateTime,
        crl_der: &[u8],
        exported_path: Option<&str>,
    ) -> Result<CrlRecord> {
        self.ensure_writable()?;
        self.conn().execute(
            &format!("INSERT INTO crls({COLS}) VALUES (NULL, ?1, ?2, ?3, ?4, ?5, ?6)"),
            params![
                ca_id,
                crl_number,
                format_ts(this_update),
                format_ts(next_update),
                crl_der,
                exported_path
            ],
        )?;
        let id = self.conn().last_insert_rowid();
        // Remember in which CRL each revocation was first published.
        self.conn().execute(
            "UPDATE revocations SET first_crl_id = ?1 WHERE first_crl_id IS NULL AND cert_id IN (SELECT id FROM certificates WHERE ca_id = ?2)",
            params![id, ca_id],
        )?;
        self.audit("crl.publish", "ca", ca_id, serde_json::json!({ "crl_number": crl_number, "next_update": format_ts(next_update), "exported_path": exported_path }))?;
        Ok(self.conn().query_row(
            &format!("SELECT {COLS} FROM crls WHERE id = ?1"),
            params![id],
            map_crl,
        )?)
    }

    pub fn latest_crl(&self, ca_id: i64) -> Result<Option<CrlRecord>> {
        Ok(self
            .conn()
            .query_row(
                &format!(
                    "SELECT {COLS} FROM crls WHERE ca_id = ?1 ORDER BY crl_number DESC LIMIT 1"
                ),
                params![ca_id],
                map_crl,
            )
            .optional()?)
    }

    pub fn list_crls(&self, ca_id: i64, limit: u32) -> Result<Vec<CrlRecord>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLS} FROM crls WHERE ca_id = ?1 ORDER BY crl_number DESC LIMIT ?2"
        ))?;
        let rows = stmt.query_map(params![ca_id, limit], map_crl)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
