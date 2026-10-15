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

    /// Replace the CA key and all given leaf keys in one transaction (passphrase change).
    /// `leaf_keys` are `(certificate_id, encrypted_key_pem)` pairs already wrapped with the new
    /// passphrase. Audited as `ca.change_passphrase`.
    pub fn rewrap_ca_keys(
        &self,
        ca_id: i64,
        encrypted_ca_key_pem: &str,
        leaf_keys: &[(i64, String)],
    ) -> Result<()> {
        self.ensure_writable()?;
        let conn = self.conn();
        conn.execute("BEGIN IMMEDIATE", [])?;
        let res = (|| -> Result<()> {
            let n = conn.execute(
                "UPDATE cas SET encrypted_key_pem = ?2 WHERE id = ?1",
                params![ca_id, encrypted_ca_key_pem],
            )?;
            if n == 0 {
                return Err(CertanaError::Invalid(format!("CA {ca_id} not found")));
            }
            for (cert_id, pem) in leaf_keys {
                let n = conn.execute(
                    "UPDATE certificates SET encrypted_key_pem = ?3 WHERE id = ?1 AND ca_id = ?2",
                    params![cert_id, ca_id, pem],
                )?;
                if n == 0 {
                    return Err(CertanaError::Invalid(format!(
                        "certificate {cert_id} does not belong to CA {ca_id}"
                    )));
                }
            }
            Ok(())
        })();
        match res {
            Ok(()) => conn.execute("COMMIT", [])?,
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(e);
            }
        };
        self.audit(
            "ca.change_passphrase",
            "ca",
            ca_id,
            serde_json::json!({ "leaf_keys": leaf_keys.len() }),
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

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;
    use crate::db::models::{CertificateFilter, NewCertificate};
    use crate::pki::ca::{CaSpec, DistributionUrls, create_root};
    use crate::pki::issue::{IssueSpec, issue_with_new_key};
    use crate::pki::keys::KeyAlgorithm;
    use crate::pki::profiles::{Profile, San, Subject};
    use crate::vault::keywrap::{decrypt_key_pem, encrypt_key_pem};

    /// Passphrase change: CA key and every leaf key are re-wrapped atomically, and the old
    /// passphrase no longer opens any of them.
    #[test]
    fn rewrap_ca_keys_rewraps_ca_and_leaf_keys() {
        let db = Database::open_in_memory().unwrap();
        let old = SecretString::from("old-passphrase-0123456789".to_string());
        let new = SecretString::from("new-passphrase-9876543210".to_string());
        let spec = CaSpec {
            subject: Subject::with_common_name("Rewrap Test CA"),
            algorithm: KeyAlgorithm::EcdsaP256,
            validity_days: 365,
            path_len: 0,
            urls: DistributionUrls::default(),
        };
        let ca_material = create_root(&spec).unwrap();
        let ca = db
            .insert_ca(&NewCa {
                kind: CaKind::Root,
                name: "Rewrap".into(),
                cert_pem: ca_material.certificate_pem.clone(),
                key_algo: KeyAlgorithm::EcdsaP256,
                encrypted_key_pem: Some(encrypt_key_pem(&ca_material.key, &old).unwrap()),
                parent_ca_id: None,
                crl_days: 7,
                crl_url: None,
                aia_url: None,
            })
            .unwrap();
        let leaf = issue_with_new_key(
            &IssueSpec {
                profile: Profile::Server,
                subject: Subject::default(),
                sans: vec![San::Dns("rewrap.internal".into())],
                validity_days: 30,
                urls: DistributionUrls::default(),
            },
            KeyAlgorithm::EcdsaP256,
            &ca_material.certificate_pem,
            &ca_material.key,
        )
        .unwrap();
        let leaf_key = leaf.key.as_ref().unwrap();
        let cert = db
            .insert_certificate(&NewCertificate {
                ca_id: ca.id,
                serial_hex: leaf.serial_hex.clone(),
                profile: Profile::Server,
                sans: vec![San::Dns("rewrap.internal".into())],
                cert_pem: leaf.certificate_pem.clone(),
                encrypted_key_pem: Some(encrypt_key_pem(leaf_key, &old).unwrap()),
                csr_pem: None,
                notes: String::new(),
            })
            .unwrap();

        let new_ca_pem = encrypt_key_pem(&ca_material.key, &new).unwrap();
        let new_leaf_pem = encrypt_key_pem(leaf_key, &new).unwrap();
        db.rewrap_ca_keys(ca.id, &new_ca_pem, &[(cert.id, new_leaf_pem)])
            .unwrap();

        let ca_after = db.get_ca(ca.id).unwrap().unwrap();
        let enc = ca_after.encrypted_key_pem.as_deref().unwrap();
        assert!(decrypt_key_pem(enc, KeyAlgorithm::EcdsaP256, &old).is_err());
        let k = decrypt_key_pem(enc, KeyAlgorithm::EcdsaP256, &new).unwrap();
        assert_eq!(k.pkcs8_der(), ca_material.key.pkcs8_der());

        let certs = db
            .list_certificates(&CertificateFilter { ca_id: Some(ca.id), ..Default::default() })
            .unwrap();
        let enc = certs[0].encrypted_key_pem.as_deref().unwrap();
        assert!(decrypt_key_pem(enc, KeyAlgorithm::EcdsaP256, &old).is_err());
        let k = decrypt_key_pem(enc, KeyAlgorithm::EcdsaP256, &new).unwrap();
        assert_eq!(k.pkcs8_der(), leaf_key.pkcs8_der());

        // A foreign certificate id rolls everything back.
        let before = db.get_ca(ca.id).unwrap().unwrap().encrypted_key_pem;
        let bogus = encrypt_key_pem(&ca_material.key, &old).unwrap();
        assert!(db.rewrap_ca_keys(ca.id, &bogus, &[(9999, String::new())]).is_err());
        assert_eq!(db.get_ca(ca.id).unwrap().unwrap().encrypted_key_pem, before);
        assert!(db.audit_csv().unwrap().contains("ca.change_passphrase"));
    }
}
