// SPDX-License-Identifier: GPL-3.0-or-later
//! CA rows.

use rusqlite::{OptionalExtension as _, Row, params};
use time::OffsetDateTime;

use super::models::{CaKind, CaRecord, NewCa, enum_parse, enum_str};
use super::{Database, format_ts, parse_ts};
use crate::pki::parse::parse_certificate_pem;
use crate::{CertanaError, Result};

const COLS: &str = "id, kind, name, cert_pem, cert_sha256, subject, not_before, not_after, key_algo, encrypted_key_pem, parent_ca_id, crl_url, aia_url, crl_number, crl_days, created_at";

fn map_ca(r: &Row<'_>) -> rusqlite::Result<CaRecord> {
    let kind: String = r.get(1)?;
    let nb: String = r.get(6)?;
    let na: String = r.get(7)?;
    let algo: String = r.get(8)?;
    let created: String = r.get(15)?;
    let key: Option<String> = r.get(9)?;
    Ok(CaRecord {
        id: r.get(0)?,
        kind: CaKind::parse(&kind).ok_or(rusqlite::Error::InvalidQuery)?,
        name: r.get(2)?,
        cert_pem: r.get(3)?,
        cert_sha256: r.get(4)?,
        subject: r.get(5)?,
        not_before: parse_ts(&nb).map_err(|_| rusqlite::Error::InvalidQuery)?,
        not_after: parse_ts(&na).map_err(|_| rusqlite::Error::InvalidQuery)?,
        key_algo: enum_parse(&algo)?,
        has_key: key.is_some(),
        encrypted_key_pem: key,
        parent_ca_id: r.get(10)?,
        crl_url: r.get(11)?,
        aia_url: r.get(12)?,
        crl_number: r.get(13)?,
        crl_days: r.get(14)?,
        created_at: parse_ts(&created).map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

impl Database {
    pub fn insert_ca(&self, new: &NewCa) -> Result<CaRecord> {
        self.ensure_writable()?;
        let info = parse_certificate_pem(&new.cert_pem)?;
        if !info.is_ca {
            return Err(CertanaError::Invalid(
                "certificate is not a CA certificate".into(),
            ));
        }
        if new.kind == CaKind::Intermediate && new.parent_ca_id.is_none() && info.self_signed {
            return Err(CertanaError::Invalid(
                "an intermediate cannot be self-signed".into(),
            ));
        }
        self.conn().execute(
            &format!("INSERT INTO cas({}) VALUES (NULL, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13, ?14)", COLS),
            params![
                new.kind.as_str(),
                new.name,
                new.cert_pem,
                info.sha256_fingerprint,
                info.subject,
                format_ts(info.not_before),
                format_ts(info.not_after),
                enum_str(&new.key_algo),
                new.encrypted_key_pem,
                new.parent_ca_id,
                new.crl_url,
                new.aia_url,
                new.crl_days,
                format_ts(OffsetDateTime::now_utc()),
            ],
        )?;
        let id = self.conn().last_insert_rowid();
        self.audit("ca.create", "ca", id, serde_json::json!({ "name": new.name, "kind": new.kind.as_str(), "fingerprint": info.sha256_fingerprint }))?;
        self.get_ca(id)?
            .ok_or_else(|| CertanaError::Db(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn get_ca(&self, id: i64) -> Result<Option<CaRecord>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLS} FROM cas WHERE id = ?1"),
                params![id],
                map_ca,
            )
            .optional()?)
    }

    pub fn list_cas(&self) -> Result<Vec<CaRecord>> {
        let mut stmt = self
            .conn()
            .prepare(&format!("SELECT {COLS} FROM cas ORDER BY kind DESC, name"))?;
        let rows = stmt.query_map([], map_ca)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_ca_urls(
        &self,
        id: i64,
        crl_url: Option<&str>,
        aia_url: Option<&str>,
        crl_days: i64,
    ) -> Result<()> {
        self.ensure_writable()?;
        let n = self.conn().execute(
            "UPDATE cas SET crl_url = ?2, aia_url = ?3, crl_days = ?4 WHERE id = ?1",
            params![id, crl_url, aia_url, crl_days],
        )?;
        if n == 0 {
            return Err(CertanaError::Invalid(format!("CA {id} not found")));
        }
        self.audit(
            "ca.update_urls",
            "ca",
            id,
            serde_json::json!({ "crl_url": crl_url, "aia_url": aia_url, "crl_days": crl_days }),
        )
    }

    /// Replace the stored (encrypted) key – e.g. after a passphrase change. Audited.
    pub fn update_ca_key(&self, id: i64, encrypted_key_pem: Option<&str>) -> Result<()> {
        self.ensure_writable()?;
        self.conn().execute(
            "UPDATE cas SET encrypted_key_pem = ?2 WHERE id = ?1",
            params![id, encrypted_key_pem],
        )?;
        self.audit(
            "ca.update_key",
            "ca",
            id,
            serde_json::json!({ "has_key": encrypted_key_pem.is_some() }),
        )
    }

    /// Atomically reserve the next CRL number for a CA.
    pub fn next_crl_number(&self, ca_id: i64) -> Result<i64> {
        self.ensure_writable()?;
        self.conn().execute(
            "UPDATE cas SET crl_number = crl_number + 1 WHERE id = ?1",
            params![ca_id],
        )?;
        Ok(self.conn().query_row(
            "SELECT crl_number FROM cas WHERE id = ?1",
            params![ca_id],
            |r| r.get(0),
        )?)
    }

    /// Issuing chain (parent, grandparent …) for building `chain.pem`. Excludes `ca_id` itself.
    pub fn ca_ancestors(&self, ca_id: i64) -> Result<Vec<CaRecord>> {
        let mut out = Vec::new();
        let mut current = self.get_ca(ca_id)?;
        while let Some(ca) = current {
            match ca.parent_ca_id {
                Some(pid) => {
                    let parent = self.get_ca(pid)?;
                    if let Some(p) = &parent {
                        out.push(p.clone());
                    }
                    current = parent;
                }
                None => current = None,
            }
            if out.len() > 8 {
                break; // defensive against cycles
            }
        }
        Ok(out)
    }
}
