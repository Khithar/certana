// SPDX-License-Identifier: GPL-3.0-or-later
//! Certificate rows, revocation marks and expiry maintenance.

use rusqlite::{OptionalExtension as _, Row, params};
use time::{Duration, OffsetDateTime};

use super::models::{
    CertStatus, CertificateFilter, CertificateRecord, NewCertificate, RevocationRecord, enum_parse,
    enum_str,
};
use super::{Database, format_ts, parse_ts};
use crate::pki::crl::Reason;
use crate::pki::parse::parse_certificate_pem;
use crate::pki::profiles::San;
use crate::{CertanaError, Result};

const COLS: &str = "id, ca_id, serial_hex, profile, subject, sans_json, not_before, not_after, cert_pem, encrypted_key_pem, csr_pem, status, notes, created_at, created_by";

fn map_cert(r: &Row<'_>) -> rusqlite::Result<CertificateRecord> {
    let profile: String = r.get(3)?;
    let sans: String = r.get(5)?;
    let nb: String = r.get(6)?;
    let na: String = r.get(7)?;
    let key: Option<String> = r.get(9)?;
    let status: String = r.get(11)?;
    let created: String = r.get(13)?;
    Ok(CertificateRecord {
        id: r.get(0)?,
        ca_id: r.get(1)?,
        serial_hex: r.get(2)?,
        profile: enum_parse(&profile)?,
        subject: r.get(4)?,
        sans: serde_json::from_str::<Vec<San>>(&sans).unwrap_or_default(),
        not_before: parse_ts(&nb).map_err(|_| rusqlite::Error::InvalidQuery)?,
        not_after: parse_ts(&na).map_err(|_| rusqlite::Error::InvalidQuery)?,
        cert_pem: r.get(8)?,
        has_key: key.is_some(),
        encrypted_key_pem: key,
        csr_pem: r.get(10)?,
        status: CertStatus::parse(&status).ok_or(rusqlite::Error::InvalidQuery)?,
        notes: r.get(12)?,
        created_at: parse_ts(&created).map_err(|_| rusqlite::Error::InvalidQuery)?,
        created_by: r.get(14)?,
    })
}

impl Database {
    pub fn insert_certificate(&self, new: &NewCertificate) -> Result<CertificateRecord> {
        self.ensure_writable()?;
        let info = parse_certificate_pem(&new.cert_pem)?;
        if info.is_ca {
            return Err(CertanaError::Invalid(
                "leaf table does not accept CA certificates".into(),
            ));
        }
        if info.serial_hex != new.serial_hex {
            return Err(CertanaError::Invalid(
                "serial does not match certificate".into(),
            ));
        }
        self.conn().execute(
            &format!("INSERT INTO certificates({COLS}) VALUES (NULL, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'valid', ?11, ?12, ?13)"),
            params![
                new.ca_id,
                new.serial_hex,
                enum_str(&new.profile),
                info.subject,
                serde_json::to_string(&new.sans).expect("sans serialise"),
                format_ts(info.not_before),
                format_ts(info.not_after),
                new.cert_pem,
                new.encrypted_key_pem,
                new.csr_pem,
                new.notes,
                format_ts(OffsetDateTime::now_utc()),
                super::lock::username(),
            ],
        )?;
        let id = self.conn().last_insert_rowid();
        self.audit("cert.issue", "certificate", id, serde_json::json!({ "serial": new.serial_hex, "subject": info.subject, "profile": enum_str(&new.profile), "not_after": format_ts(info.not_after), "from_csr": new.csr_pem.is_some() }))?;
        self.get_certificate(id)?
            .ok_or_else(|| CertanaError::Db(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn get_certificate(&self, id: i64) -> Result<Option<CertificateRecord>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLS} FROM certificates WHERE id = ?1"),
                params![id],
                map_cert,
            )
            .optional()?)
    }

    pub fn get_certificate_by_serial(&self, serial_hex: &str) -> Result<Option<CertificateRecord>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLS} FROM certificates WHERE serial_hex = ?1"),
                params![serial_hex.to_lowercase()],
                map_cert,
            )
            .optional()?)
    }

    pub fn list_certificates(&self, filter: &CertificateFilter) -> Result<Vec<CertificateRecord>> {
        let mut sql = format!("SELECT {COLS} FROM certificates WHERE 1=1");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(ca) = filter.ca_id {
            sql.push_str(" AND ca_id = ?");
            args.push(Box::new(ca));
        }
        if let Some(p) = filter.profile {
            sql.push_str(" AND profile = ?");
            args.push(Box::new(enum_str(&p)));
        }
        if let Some(s) = filter.status {
            sql.push_str(" AND status = ?");
            args.push(Box::new(s.as_str()));
        }
        if let Some(q) = filter
            .search
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            sql.push_str(" AND (lower(subject) LIKE ? OR lower(sans_json) LIKE ? OR lower(serial_hex) LIKE ?)");
            let like = format!("%{}%", q.to_lowercase());
            args.push(Box::new(like.clone()));
            args.push(Box::new(like.clone()));
            args.push(Box::new(like));
        }
        if let Some(days) = filter.expiring_within_days {
            sql.push_str(" AND status = 'valid' AND not_after <= ?");
            args.push(Box::new(format_ts(
                OffsetDateTime::now_utc() + Duration::days(days),
            )));
        }
        sql.push_str(" ORDER BY not_after ASC, id DESC");
        let mut stmt = self.conn().prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(args.iter().map(|a| a.as_ref())),
            map_cert,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_certificate_notes(&self, id: i64, notes: &str) -> Result<()> {
        self.ensure_writable()?;
        self.conn().execute(
            "UPDATE certificates SET notes = ?2 WHERE id = ?1",
            params![id, notes],
        )?;
        Ok(())
    }

    /// Mark a certificate revoked. Irreversible; the next CRL build picks it up.
    pub fn revoke_certificate(
        &self,
        id: i64,
        reason: Reason,
        invalidity_date: Option<OffsetDateTime>,
        comment: &str,
    ) -> Result<RevocationRecord> {
        self.ensure_writable()?;
        let cert = self
            .get_certificate(id)?
            .ok_or_else(|| CertanaError::Invalid(format!("certificate {id} not found")))?;
        if cert.status == CertStatus::Revoked {
            return Err(CertanaError::Invalid(
                "certificate is already revoked".into(),
            ));
        }
        let now = OffsetDateTime::now_utc();
        let tx_conn = self.conn();
        tx_conn.execute("BEGIN IMMEDIATE", [])?;
        let res = (|| -> Result<()> {
            tx_conn.execute(
                "INSERT INTO revocations(cert_id, revoked_at, reason, invalidity_date, comment, first_crl_id) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                params![id, format_ts(now), enum_str(&reason), invalidity_date.map(format_ts), comment],
            )?;
            tx_conn.execute(
                "UPDATE certificates SET status = 'revoked' WHERE id = ?1",
                params![id],
            )?;
            Ok(())
        })();
        match res {
            Ok(()) => tx_conn.execute("COMMIT", [])?,
            Err(e) => {
                let _ = tx_conn.execute("ROLLBACK", []);
                return Err(e);
            }
        };
        self.audit("cert.revoke", "certificate", id, serde_json::json!({ "serial": cert.serial_hex, "reason": enum_str(&reason), "comment": comment }))?;
        Ok(RevocationRecord {
            cert_id: id,
            revoked_at: now,
            reason,
            invalidity_date,
            comment: comment.to_string(),
            first_crl_id: None,
        })
    }

    pub fn get_revocation(&self, cert_id: i64) -> Result<Option<RevocationRecord>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT cert_id, revoked_at, reason, invalidity_date, comment, first_crl_id FROM revocations WHERE cert_id = ?1",
                params![cert_id],
                map_revocation,
            )
            .optional()?)
    }

    /// Revocations of a CA's certificates that still matter for the CRL (not yet expired longer
    /// than one CRL period ago).
    pub fn revocations_for_crl(
        &self,
        ca_id: i64,
        keep_expired_days: i64,
    ) -> Result<Vec<(CertificateRecord, RevocationRecord)>> {
        let cutoff = format_ts(OffsetDateTime::now_utc() - Duration::days(keep_expired_days));
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLS}, r.cert_id, r.revoked_at, r.reason, r.invalidity_date, r.comment, r.first_crl_id \
             FROM certificates JOIN revocations r ON r.cert_id = certificates.id \
             WHERE certificates.ca_id = ?1 AND certificates.not_after >= ?2 ORDER BY r.revoked_at"
        ))?;
        let rows = stmt.query_map(params![ca_id, cutoff], |r| {
            let cert = map_cert(r)?;
            let revoked_at: String = r.get(16)?;
            let reason: String = r.get(17)?;
            let inv: Option<String> = r.get(18)?;
            Ok((
                cert,
                RevocationRecord {
                    cert_id: r.get(15)?,
                    revoked_at: parse_ts(&revoked_at).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    reason: enum_parse(&reason)?,
                    invalidity_date: inv
                        .as_deref()
                        .map(parse_ts)
                        .transpose()
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    comment: r.get(19)?,
                    first_crl_id: r.get(20)?,
                },
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Flip `valid` → `expired` for everything past `notAfter`. Returns how many rows changed.
    pub fn refresh_expired(&self) -> Result<usize> {
        self.ensure_writable()?;
        Ok(self.conn().execute(
            "UPDATE certificates SET status = 'expired' WHERE status = 'valid' AND not_after < ?1",
            params![format_ts(OffsetDateTime::now_utc())],
        )?)
    }
}

fn map_revocation(r: &Row<'_>) -> rusqlite::Result<RevocationRecord> {
    let revoked_at: String = r.get(1)?;
    let reason: String = r.get(2)?;
    let inv: Option<String> = r.get(3)?;
    Ok(RevocationRecord {
        cert_id: r.get(0)?,
        revoked_at: parse_ts(&revoked_at).map_err(|_| rusqlite::Error::InvalidQuery)?,
        reason: enum_parse(&reason)?,
        invalidity_date: inv
            .as_deref()
            .map(parse_ts)
            .transpose()
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        comment: r.get(4)?,
        first_crl_id: r.get(5)?,
    })
}
