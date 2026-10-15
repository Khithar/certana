// SPDX-License-Identifier: GPL-3.0-or-later
//! Leaf certificates: wizard issuance, listing, export packages, revocation, renewal.

use std::path::PathBuf;

use certana_core::db::models::{
    CertStatus, CertificateFilter, CertificateRecord, NewCertificate, RevocationRecord,
};
use certana_core::pki::ca::DistributionUrls;
use certana_core::pki::crl::Reason;
use certana_core::pki::csr::{CsrInfo, parse_csr_pem};
use certana_core::pki::export::{
    Pkcs12Flavor, ServerSnippets, build_chain, certificate_pem_to_der, server_snippets, to_pkcs12,
};
use certana_core::pki::issue::{IssueSpec, issue_from_csr, issue_with_new_key};
use certana_core::pki::keys::KeyAlgorithm;
use certana_core::pki::parse::{CertificateInfo, parse_certificate_pem};
use certana_core::pki::profiles::{Profile, San, Subject};
use certana_core::vault::keywrap::{decrypt_key_pem, encrypt_key_pem};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::error::{CmdResult, CommandError};
use super::files::{safe_stem, write_private, write_public};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ProfileInfo {
    pub profile: Profile,
    pub default_validity_days: u32,
    pub max_validity_days: u32,
    pub default_key_algorithm: KeyAlgorithm,
    pub key_algorithms: Vec<KeyAlgorithm>,
}

#[tauri::command]
pub fn cert_profiles() -> Vec<ProfileInfo> {
    Profile::all()
        .iter()
        .map(|&p| ProfileInfo {
            profile: p,
            default_validity_days: p.default_validity_days(),
            max_validity_days: p.max_validity_days(),
            default_key_algorithm: p.default_key_algorithm(),
            key_algorithms: KeyAlgorithm::all().to_vec(),
        })
        .collect()
}

#[tauri::command]
pub fn cert_list(
    state: State<'_, AppState>,
    filter: Option<CertificateFilter>,
) -> CmdResult<Vec<CertificateRecord>> {
    state
        .0
        .with_db(|db| Ok(db.list_certificates(&filter.unwrap_or_default())?))
}

#[derive(Debug, Serialize)]
pub struct CertDetail {
    pub record: CertificateRecord,
    pub info: CertificateInfo,
    pub revocation: Option<RevocationRecord>,
    pub ca_name: String,
}

#[tauri::command]
pub fn cert_get(state: State<'_, AppState>, id: i64) -> CmdResult<CertDetail> {
    state.0.with_db(|db| {
        let record = db
            .get_certificate(id)?
            .ok_or_else(|| CommandError::invalid("certificate not found"))?;
        let info = parse_certificate_pem(&record.cert_pem)?;
        let revocation = db.get_revocation(id)?;
        let ca_name = db.get_ca(record.ca_id)?.map(|c| c.name).unwrap_or_default();
        Ok(CertDetail {
            record,
            info,
            revocation,
            ca_name,
        })
    })
}

#[tauri::command]
pub fn cert_parse_csr(csr_pem: String) -> CmdResult<CsrInfo> {
    Ok(parse_csr_pem(&csr_pem)?)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeySource {
    Generate { algorithm: KeyAlgorithm },
    Csr { csr_pem: String },
}

#[derive(Debug, Deserialize)]
pub struct IssueRequest {
    pub ca_id: i64,
    pub profile: Profile,
    pub subject: Subject,
    pub sans: Vec<San>,
    pub validity_days: u32,
    pub key: KeySource,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn cert_issue(
    state: State<'_, AppState>,
    req: IssueRequest,
) -> CmdResult<CertificateRecord> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ca = inner.with_db(|db| {
            db.get_ca(req.ca_id)?
                .ok_or_else(|| CommandError::invalid("CA not found"))
        })?;
        let spec = IssueSpec {
            profile: req.profile,
            subject: req.subject,
            sans: req.sans.clone(),
            validity_days: req.validity_days,
            urls: DistributionUrls {
                crl_url: ca.crl_url.clone(),
                ca_issuers_url: ca.aia_url.clone(),
                ocsp_url: None,
            },
        };
        inner.with_session(req.ca_id, |session| {
            let (issued, csr_pem) = match &req.key {
                KeySource::Generate { algorithm } => (
                    issue_with_new_key(&spec, *algorithm, &ca.cert_pem, &session.key)?,
                    None,
                ),
                KeySource::Csr { csr_pem } => (
                    issue_from_csr(&spec, csr_pem, &ca.cert_pem, &session.key)?,
                    Some(csr_pem.clone()),
                ),
            };
            let encrypted = issued
                .key
                .as_ref()
                .map(|k| encrypt_key_pem(k, &session.passphrase))
                .transpose()?;
            inner.with_db(|db| {
                Ok(db.insert_certificate(&NewCertificate {
                    ca_id: req.ca_id,
                    serial_hex: issued.serial_hex.clone(),
                    profile: req.profile,
                    sans: req.sans.clone(),
                    cert_pem: issued.certificate_pem.clone(),
                    encrypted_key_pem: encrypted,
                    csr_pem,
                    notes: req.notes.clone().unwrap_or_default(),
                })?)
            })
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub cert_id: i64,
    pub dest_dir: String,
    pub base_name: Option<String>,
    pub include_key: bool,
    pub include_der: bool,
    pub include_p12: bool,
    pub p12_password: Option<String>,
    pub p12_legacy: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub files: Vec<String>,
    pub snippets: ServerSnippets,
    pub key_written: bool,
}

/// Write the export package. Keys require the issuing CA to be unlocked (they are wrapped with its passphrase).
#[tauri::command]
pub async fn cert_export(
    state: State<'_, AppState>,
    req: ExportRequest,
) -> CmdResult<ExportResult> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (record, chain): (CertificateRecord, Vec<String>) = inner.with_db(|db| {
            let r = db.get_certificate(req.cert_id)?.ok_or_else(|| CommandError::invalid("certificate not found"))?;
            // chain.pem = issuing CA + parents, excluding the self-signed root.
            let mut pems: Vec<String> = Vec::new();
            if let Some(issuing) = db.get_ca(r.ca_id)? {
                pems.push(issuing.cert_pem);
            }
            for c in db.ca_ancestors(r.ca_id)? {
                if c.parent_ca_id.is_some() {
                    pems.push(c.cert_pem);
                }
            }
            Ok((r, pems))
        })?;
        let info = parse_certificate_pem(&record.cert_pem)?;
        let stem = req.base_name.clone().filter(|s| !s.trim().is_empty()).map(|s| safe_stem(&s)).unwrap_or_else(|| safe_stem(info.subject_common_name.as_deref().unwrap_or("certificate")));
        let dir = PathBuf::from(&req.dest_dir);
        let files_out = build_chain(&record.cert_pem, &chain);
        let mut files = Vec::new();
        let mut put = |name: String, data: &[u8], private: bool| -> CmdResult<()> {
            let p = dir.join(name);
            if private { write_private(&p, data)? } else { write_public(&p, data)? }
            files.push(p.display().to_string());
            Ok(())
        };
        put(format!("{stem}.crt.pem"), files_out.cert.as_bytes(), false)?;
        put(format!("{stem}.chain.pem"), files_out.chain.as_bytes(), false)?;
        put(format!("{stem}.fullchain.pem"), files_out.fullchain.as_bytes(), false)?;
        if req.include_der {
            put(format!("{stem}.crt.der"), &certificate_pem_to_der(&record.cert_pem)?, false)?;
        }
        let mut key_written = false;
        if (req.include_key || req.include_p12) && record.encrypted_key_pem.is_some() {
            let encrypted = record.encrypted_key_pem.clone().unwrap_or_default();
            let key = inner.with_session(record.ca_id, |session| Ok(decrypt_key_pem(&encrypted, key_algo_of(&info), &session.passphrase).or_else(|_| decrypt_any(&encrypted, &session.passphrase))?))?;
            if req.include_key {
                put(format!("{stem}.key.pem"), key.to_pkcs8_pem().as_bytes(), true)?;
                key_written = true;
            }
            if req.include_p12 {
                let pw = req.p12_password.clone().unwrap_or_default();
                let p12 = to_pkcs12(&stem, &key, &record.cert_pem, &chain, &pw, if req.p12_legacy { Pkcs12Flavor::Legacy } else { Pkcs12Flavor::Modern })?;
                put(format!("{stem}.p12"), &p12, true)?;
            }
        }
        inner.with_db(|db| Ok(db.audit("cert.export", "certificate", req.cert_id, serde_json::json!({ "dir": req.dest_dir, "key": key_written, "p12": req.include_p12 }))?))?;
        Ok(ExportResult { files, snippets: server_snippets(&req.dest_dir, &stem), key_written })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

fn key_algo_of(info: &CertificateInfo) -> KeyAlgorithm {
    match info.public_key_algorithm.as_str() {
        "rsaEncryption" => KeyAlgorithm::Rsa2048,
        "Ed25519" | "ED25519" => KeyAlgorithm::Ed25519,
        _ => KeyAlgorithm::EcdsaP256,
    }
}

fn decrypt_any(
    encrypted: &str,
    passphrase: &secrecy::SecretString,
) -> certana_core::Result<certana_core::pki::keys::PrivateKey> {
    certana_core::vault::keywrap::decrypt_key_pem_detect(encrypted, passphrase)
}

#[tauri::command]
pub fn cert_revoke(
    state: State<'_, AppState>,
    id: i64,
    reason: Reason,
    comment: Option<String>,
) -> CmdResult<RevocationRecord> {
    state.0.with_db(|db| {
        Ok(db.revoke_certificate(id, reason, None, comment.as_deref().unwrap_or(""))?)
    })
}

#[tauri::command]
pub fn cert_update_notes(state: State<'_, AppState>, id: i64, notes: String) -> CmdResult<()> {
    state
        .0
        .with_db(|db| Ok(db.update_certificate_notes(id, &notes)?))
}

#[derive(Debug, Deserialize)]
pub struct RenewRequest {
    pub cert_id: i64,
    pub validity_days: u32,
    /// `None` keeps the existing key (only possible when Certana holds it); `Some` generates a new one.
    pub new_key_algorithm: Option<KeyAlgorithm>,
    pub revoke_old: bool,
}

/// Re-issue with the same names; optionally revoke the old certificate as `superseded`.
#[tauri::command]
pub async fn cert_renew(
    state: State<'_, AppState>,
    req: RenewRequest,
) -> CmdResult<CertificateRecord> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let old = inner.with_db(|db| {
            db.get_certificate(req.cert_id)?
                .ok_or_else(|| CommandError::invalid("certificate not found"))
        })?;
        let ca = inner.with_db(|db| {
            db.get_ca(old.ca_id)?
                .ok_or_else(|| CommandError::invalid("CA not found"))
        })?;
        let old_info = parse_certificate_pem(&old.cert_pem)?;
        let spec = IssueSpec {
            profile: old.profile,
            subject: Subject::with_common_name(
                old_info.subject_common_name.clone().unwrap_or_default(),
            ),
            sans: old.sans.clone(),
            validity_days: req.validity_days,
            urls: DistributionUrls {
                crl_url: ca.crl_url.clone(),
                ca_issuers_url: ca.aia_url.clone(),
                ocsp_url: None,
            },
        };
        inner.with_session(old.ca_id, |session| {
            let (issued, csr_pem, encrypted) =
                match (&req.new_key_algorithm, &old.encrypted_key_pem, &old.csr_pem) {
                    (Some(alg), _, _) => {
                        let issued = issue_with_new_key(&spec, *alg, &ca.cert_pem, &session.key)?;
                        let enc = issued
                            .key
                            .as_ref()
                            .map(|k| encrypt_key_pem(k, &session.passphrase))
                            .transpose()?;
                        (issued, None, enc)
                    }
                    (None, Some(enc), _) => {
                        let key = decrypt_any(enc, &session.passphrase)?;
                        let csr =
                            certana_core::pki::csr::create_csr(&spec.subject, &spec.sans, &key)?;
                        let issued = issue_from_csr(&spec, &csr, &ca.cert_pem, &session.key)?;
                        (issued, None, Some(enc.clone()))
                    }
                    (None, None, Some(csr)) => (
                        issue_from_csr(&spec, csr, &ca.cert_pem, &session.key)?,
                        Some(csr.clone()),
                        None,
                    ),
                    (None, None, None) => {
                        return Err(CommandError::invalid(
                            "no key available – choose a new key algorithm",
                        ));
                    }
                };
            inner.with_db(|db| {
                let new = db.insert_certificate(&NewCertificate {
                    ca_id: old.ca_id,
                    serial_hex: issued.serial_hex.clone(),
                    profile: old.profile,
                    sans: old.sans.clone(),
                    cert_pem: issued.certificate_pem.clone(),
                    encrypted_key_pem: encrypted,
                    csr_pem,
                    notes: format!("renewed from {}", old.serial_hex),
                })?;
                if req.revoke_old && old.status == CertStatus::Valid {
                    db.revoke_certificate(
                        old.id,
                        Reason::Superseded,
                        None,
                        &format!("superseded by {}", new.serial_hex),
                    )?;
                }
                Ok(new)
            })
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
