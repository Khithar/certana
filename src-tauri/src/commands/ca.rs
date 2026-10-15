// SPDX-License-Identifier: GPL-3.0-or-later
//! CA lifecycle: root ceremony, root import, intermediates, unlock/lock, export.

use std::path::{Path, PathBuf};
use std::time::Instant;

use certana_core::db::models::{CaKind, CaRecord, NewCa};
use certana_core::pki::ca::{
    CaSpec, DistributionUrls, create_intermediate, create_root, intermediate_csr,
};
use certana_core::pki::export::{certificate_pem_to_der, from_pkcs12};
use certana_core::pki::keys::{KeyAlgorithm, PrivateKey};
use certana_core::pki::parse::{CertificateInfo, parse_certificate_pem, verify_issued_by};
use certana_core::pki::profiles::Subject;
use certana_core::vault::keywrap::{
    check_passphrase_strength, decrypt_key_pem, decrypt_key_pem_detect, encrypt_key_pem,
};
use certana_core::vault::rootbundle::{read_bundle, write_bundle};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::error::{CmdResult, CommandError};
use super::files::{write_private, write_public};
use crate::state::{AppState, Session};

#[derive(Debug, Deserialize)]
pub struct RootCreateRequest {
    pub name: String,
    pub subject: Subject,
    pub algorithm: KeyAlgorithm,
    pub validity_days: u32,
    pub crl_url: Option<String>,
    pub aia_url: Option<String>,
    pub passphrase: String,
    /// Where the `*.certana-root` bundle is written (USB stick …).
    pub bundle_path: String,
}

#[derive(Debug, Serialize)]
pub struct RootCreateResponse {
    pub ca: CaRecord,
    pub bundle_path: String,
    pub fingerprint_sha256: String,
}

fn bundle_path_with_ext(p: &str) -> PathBuf {
    let path = PathBuf::from(p);
    if path.extension().and_then(|e| e.to_str())
        == Some(certana_core::vault::rootbundle::BUNDLE_EXTENSION)
    {
        path
    } else {
        let mut os = path.into_os_string();
        os.push(".");
        os.push(certana_core::vault::rootbundle::BUNDLE_EXTENSION);
        PathBuf::from(os)
    }
}

/// Root ceremony: generate, write the encrypted bundle, register the certificate (no key) in the DB.
#[tauri::command]
pub async fn ca_create_root(
    state: State<'_, AppState>,
    req: RootCreateRequest,
) -> CmdResult<RootCreateResponse> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let passphrase = SecretString::from(req.passphrase);
        check_passphrase_strength(&passphrase)?;
        let spec = CaSpec {
            subject: req.subject,
            algorithm: req.algorithm,
            validity_days: req.validity_days,
            path_len: 1,
            urls: DistributionUrls { crl_url: req.crl_url.clone(), ca_issuers_url: req.aia_url.clone(), ocsp_url: None },
        };
        let root = create_root(&spec)?;
        let bundle = write_bundle(&req.name, &root.certificate_pem, &root.key, &passphrase)?;
        let bundle_path = bundle_path_with_ext(&req.bundle_path);
        write_private(&bundle_path, &bundle)?;
        drop(root.key);
        let info = parse_certificate_pem(&root.certificate_pem)?;
        let ca = inner.with_db(|db| {
            let ca = db.insert_ca(&NewCa { kind: CaKind::Root, name: req.name.clone(), cert_pem: root.certificate_pem.clone(), key_algo: req.algorithm, encrypted_key_pem: None, parent_ca_id: None, crl_url: req.crl_url.clone(), aia_url: req.aia_url.clone(), crl_days: 365 })?;
            db.audit("root.ceremony", "ca", ca.id, serde_json::json!({ "operation": "create", "bundle": bundle_path.display().to_string() }))?;
            Ok(ca)
        })?;
        Ok(RootCreateResponse { ca, bundle_path: bundle_path.display().to_string(), fingerprint_sha256: info.sha256_fingerprint })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Deserialize)]
pub struct RootImportRequest {
    pub name: String,
    /// PEM certificate file (required).
    pub cert_path: String,
    /// Optional PKCS#8 key (plain or encrypted PEM) – omit to register the certificate only.
    pub key_path: Option<String>,
    pub key_passphrase: Option<String>,
    /// Alternatively a PKCS#12 with key + certificate.
    pub p12_path: Option<String>,
    pub p12_password: Option<String>,
    /// Target bundle (required when a key is imported).
    pub bundle_path: Option<String>,
    pub bundle_passphrase: Option<String>,
    pub crl_url: Option<String>,
    pub aia_url: Option<String>,
}

#[tauri::command]
pub async fn ca_import_root(
    state: State<'_, AppState>,
    req: RootImportRequest,
) -> CmdResult<RootCreateResponse> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (cert_pem, key): (String, Option<PrivateKey>) = if let Some(p12) = &req.p12_path {
            let data = std::fs::read(p12)?;
            let (key, certs) = from_pkcs12(&data, req.p12_password.as_deref().unwrap_or(""))?;
            let cert = certs
                .into_iter()
                .find(|c| {
                    parse_certificate_pem(c)
                        .map(|i| i.is_ca && i.self_signed)
                        .unwrap_or(false)
                })
                .ok_or_else(|| {
                    CommandError::invalid("PKCS#12 contains no self-signed CA certificate")
                })?;
            (cert, Some(key))
        } else {
            let cert_pem = std::fs::read_to_string(&req.cert_path)?;
            let key = match &req.key_path {
                Some(kp) => {
                    let pem = std::fs::read_to_string(kp)?;
                    let key = if pem.contains("ENCRYPTED PRIVATE KEY") {
                        let pass =
                            SecretString::from(req.key_passphrase.clone().unwrap_or_default());
                        decrypt_key_pem_detect(&pem, &pass)?
                    } else {
                        PrivateKey::from_pkcs8_pem(&pem)?
                    };
                    Some(key)
                }
                None => None,
            };
            (cert_pem, key)
        };
        let info = parse_certificate_pem(&cert_pem)?;
        if !info.is_ca || !info.self_signed {
            return Err(CommandError::invalid(
                "the certificate is not a self-signed CA certificate",
            ));
        }
        let mut bundle_out = String::new();
        let key_algo = key
            .as_ref()
            .map(|k| k.algorithm())
            .unwrap_or(KeyAlgorithm::EcdsaP384);
        if let Some(key) = &key {
            if key.public_key_der()? != info.public_key_der {
                return Err(CommandError::invalid(
                    "private key does not belong to the certificate",
                ));
            }
            let bundle_path =
                bundle_path_with_ext(req.bundle_path.as_deref().ok_or_else(|| {
                    CommandError::invalid("bundle path required when importing a key")
                })?);
            let pass = SecretString::from(req.bundle_passphrase.clone().unwrap_or_default());
            let bundle = write_bundle(&req.name, &cert_pem, key, &pass)?;
            write_private(&bundle_path, &bundle)?;
            bundle_out = bundle_path.display().to_string();
        }
        let ca = inner.with_db(|db| {
            let ca = db.insert_ca(&NewCa {
                kind: CaKind::Root,
                name: req.name.clone(),
                cert_pem: cert_pem.clone(),
                key_algo,
                encrypted_key_pem: None,
                parent_ca_id: None,
                crl_url: req.crl_url.clone(),
                aia_url: req.aia_url.clone(),
                crl_days: 365,
            })?;
            db.audit(
                "root.import",
                "ca",
                ca.id,
                serde_json::json!({ "with_key": key.is_some(), "bundle": bundle_out }),
            )?;
            Ok(ca)
        })?;
        Ok(RootCreateResponse {
            ca,
            bundle_path: bundle_out,
            fingerprint_sha256: info.sha256_fingerprint,
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Serialize)]
pub struct BundleCheck {
    pub name: String,
    pub fingerprint_sha256: String,
    pub subject: String,
    pub matches_ca_id: Option<i64>,
}

/// Validate a bundle + passphrase without performing an operation (UI feedback in the ceremony dialog).
#[tauri::command]
pub async fn ca_open_root_bundle(
    state: State<'_, AppState>,
    bundle_path: String,
    passphrase: String,
) -> CmdResult<BundleCheck> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let bundle = read_bundle(
            &std::fs::read(&bundle_path)?,
            &SecretString::from(passphrase),
        )?;
        let fp = bundle.manifest.certificate_sha256.clone();
        let subject = parse_certificate_pem(&bundle.certificate_pem)?.subject;
        let matches = inner.with_db(|db| {
            Ok(db
                .list_cas()?
                .into_iter()
                .find(|c| c.cert_sha256 == fp)
                .map(|c| c.id))
        })?;
        Ok(BundleCheck {
            name: bundle.manifest.name.clone(),
            fingerprint_sha256: fp,
            subject,
            matches_ca_id: matches,
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Deserialize)]
pub struct IntermediateCreateRequest {
    pub name: String,
    pub subject: Subject,
    pub algorithm: KeyAlgorithm,
    pub validity_days: u32,
    pub crl_url: Option<String>,
    pub aia_url: Option<String>,
    pub crl_days: i64,
    /// Passphrase protecting the intermediate key in the database.
    pub passphrase: String,
    pub root_ca_id: i64,
    pub root_bundle_path: String,
    pub root_passphrase: String,
}

/// Create an intermediate and sign it right away via the root bundle (ceremony).
#[tauri::command]
pub async fn ca_create_intermediate(
    state: State<'_, AppState>,
    req: IntermediateCreateRequest,
) -> CmdResult<CaRecord> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let passphrase = SecretString::from(req.passphrase);
        check_passphrase_strength(&passphrase)?;
        let root_row = inner.with_db(|db| {
            db.get_ca(req.root_ca_id)?
                .ok_or_else(|| CommandError::invalid("root CA not found"))
        })?;
        let bundle = read_bundle(
            &std::fs::read(&req.root_bundle_path)?,
            &SecretString::from(req.root_passphrase),
        )?;
        if bundle.manifest.certificate_sha256 != root_row.cert_sha256 {
            return Err(CommandError::invalid(
                "the bundle does not belong to the selected root CA",
            ));
        }
        // CRL-DP/AIA of the *root* go into the intermediate certificate.
        let spec = CaSpec {
            subject: req.subject,
            algorithm: req.algorithm,
            validity_days: req.validity_days,
            path_len: 0,
            urls: DistributionUrls {
                crl_url: root_row.crl_url.clone(),
                ca_issuers_url: root_row.aia_url.clone(),
                ocsp_url: None,
            },
        };
        let inter = create_intermediate(&spec, &bundle.certificate_pem, &bundle.key)?;
        drop(bundle);
        let encrypted = encrypt_key_pem(&inter.key, &passphrase)?;
        inner.with_db(|db| {
            let ca = db.insert_ca(&NewCa {
                kind: CaKind::Intermediate,
                name: req.name.clone(),
                cert_pem: inter.certificate_pem.clone(),
                key_algo: req.algorithm,
                encrypted_key_pem: Some(encrypted.clone()),
                parent_ca_id: Some(req.root_ca_id),
                crl_url: req.crl_url.clone(),
                aia_url: req.aia_url.clone(),
                crl_days: req.crl_days,
            })?;
            db.audit(
                "root.ceremony",
                "ca",
                req.root_ca_id,
                serde_json::json!({ "operation": "sign_intermediate", "intermediate_id": ca.id }),
            )?;
            Ok(ca)
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Deserialize)]
pub struct IntermediateCsrRequest {
    pub name: String,
    pub subject: Subject,
    pub algorithm: KeyAlgorithm,
    pub passphrase: String,
    pub csr_path: String,
}

#[derive(Debug, Serialize)]
pub struct IntermediateCsrResponse {
    pub csr_path: String,
    pub csr_pem: String,
}

const PENDING_PREFIX: &str = "pending_intermediate.";

/// External-root flow, step 1: key + CSR; the encrypted key waits in settings until the signed
/// certificate comes back.
#[tauri::command]
pub async fn ca_create_intermediate_csr(
    state: State<'_, AppState>,
    req: IntermediateCsrRequest,
) -> CmdResult<IntermediateCsrResponse> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let passphrase = SecretString::from(req.passphrase);
        check_passphrase_strength(&passphrase)?;
        let spec = CaSpec { subject: req.subject, algorithm: req.algorithm, validity_days: 1825, path_len: 0, urls: DistributionUrls::default() };
        let (csr_pem, key) = intermediate_csr(&spec)?;
        let encrypted = encrypt_key_pem(&key, &passphrase)?;
        write_public(Path::new(&req.csr_path), csr_pem.as_bytes())?;
        inner.with_db(|db| {
            let pending = serde_json::json!({ "algorithm": req.algorithm, "encrypted_key_pem": encrypted, "csr_pem": csr_pem });
            db.set_setting(&format!("{PENDING_PREFIX}{}", req.name), &pending.to_string())?;
            db.audit("ca.intermediate_csr", "ca", &req.name, serde_json::json!({ "csr_path": req.csr_path }))?;
            Ok(())
        })?;
        Ok(IntermediateCsrResponse { csr_path: req.csr_path, csr_pem })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Deserialize)]
pub struct IntermediateImportRequest {
    pub name: String,
    pub cert_path: String,
    pub root_ca_id: i64,
    pub crl_url: Option<String>,
    pub aia_url: Option<String>,
    pub crl_days: i64,
}

/// External-root flow, step 2: import the signed intermediate certificate.
#[tauri::command]
pub fn ca_import_intermediate_cert(
    state: State<'_, AppState>,
    req: IntermediateImportRequest,
) -> CmdResult<CaRecord> {
    let cert_pem = std::fs::read_to_string(&req.cert_path)?;
    let info = parse_certificate_pem(&cert_pem)?;
    state.0.with_db(|db| {
        let key = format!("{PENDING_PREFIX}{}", req.name);
        let pending: serde_json::Value = serde_json::from_str(
            &db.get_setting(&key)?
                .ok_or_else(|| CommandError::invalid("no pending CSR for this name"))?,
        )
        .map_err(|e| CommandError::internal(e.to_string()))?;
        let algorithm: KeyAlgorithm = serde_json::from_value(pending["algorithm"].clone())
            .map_err(|e| CommandError::internal(e.to_string()))?;
        let encrypted = pending["encrypted_key_pem"]
            .as_str()
            .ok_or_else(|| CommandError::internal("pending entry corrupt"))?
            .to_string();
        let root = db
            .get_ca(req.root_ca_id)?
            .ok_or_else(|| CommandError::invalid("root CA not found"))?;
        if !verify_issued_by(&cert_pem, &root.cert_pem)? {
            return Err(CommandError::invalid(
                "certificate was not issued by the selected root",
            ));
        }
        if !info.is_ca {
            return Err(CommandError::invalid("certificate is not a CA certificate"));
        }
        let ca = db.insert_ca(&NewCa {
            kind: CaKind::Intermediate,
            name: req.name.clone(),
            cert_pem: cert_pem.clone(),
            key_algo: algorithm,
            encrypted_key_pem: Some(encrypted),
            parent_ca_id: Some(req.root_ca_id),
            crl_url: req.crl_url.clone(),
            aia_url: req.aia_url.clone(),
            crl_days: req.crl_days,
        })?;
        db.conn_delete_setting(&key)?;
        Ok(ca)
    })
}

#[derive(Debug, Serialize)]
pub struct SessionStatus {
    pub unlocked_ca_id: Option<i64>,
    pub idle_seconds_left: Option<u64>,
}

#[tauri::command]
pub async fn ca_unlock(
    state: State<'_, AppState>,
    ca_id: i64,
    passphrase: String,
) -> CmdResult<SessionStatus> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let passphrase = SecretString::from(passphrase);
        let ca = inner.with_db(|db| {
            db.get_ca(ca_id)?
                .ok_or_else(|| CommandError::invalid("CA not found"))
        })?;
        let encrypted = ca
            .encrypted_key_pem
            .as_deref()
            .ok_or_else(|| CommandError::invalid("this CA has no stored key (offline root)"))?;
        let key = decrypt_key_pem(encrypted, ca.key_algo, &passphrase)
            .map_err(|_| CommandError::new("wrong-passphrase", "wrong passphrase"))?;
        *inner
            .session
            .lock()
            .map_err(|_| CommandError::internal("session mutex poisoned"))? = Some(Session {
            ca_id,
            key,
            passphrase,
            last_used: Instant::now(),
        });
        inner.with_db(|db| Ok(db.audit("ca.unlock", "ca", ca_id, serde_json::json!({}))?))?;
        session_status(&inner)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[derive(Debug, Deserialize)]
pub struct ChangePassphraseRequest {
    pub ca_id: i64,
    pub old_passphrase: String,
    pub new_passphrase: String,
}

#[derive(Debug, Serialize)]
pub struct ChangePassphraseResponse {
    /// Number of leaf keys that were re-wrapped together with the CA key.
    pub rewrapped_leaf_keys: usize,
}

/// Change the passphrase of an issuing CA. The CA key and every leaf key stored for this CA are
/// wrapped with the CA passphrase, so all of them are decrypted with the old and re-encrypted
/// with the new passphrase in one transaction. An unlocked session for this CA is updated.
#[tauri::command]
pub async fn ca_change_passphrase(
    state: State<'_, AppState>,
    req: ChangePassphraseRequest,
) -> CmdResult<ChangePassphraseResponse> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let old = SecretString::from(req.old_passphrase);
        let new = SecretString::from(req.new_passphrase);
        check_passphrase_strength(&new)?;
        let (ca, certs) = inner.with_db(|db| {
            let ca = db
                .get_ca(req.ca_id)?
                .ok_or_else(|| CommandError::invalid("CA not found"))?;
            let certs = db.list_certificates(&certana_core::db::models::CertificateFilter {
                ca_id: Some(req.ca_id),
                ..Default::default()
            })?;
            Ok((ca, certs))
        })?;
        let encrypted = ca
            .encrypted_key_pem
            .as_deref()
            .ok_or_else(|| CommandError::invalid("this CA has no stored key (offline root)"))?;
        let ca_key = decrypt_key_pem(encrypted, ca.key_algo, &old)
            .map_err(|_| CommandError::new("wrong-passphrase", "wrong passphrase"))?;
        let new_ca_pem = encrypt_key_pem(&ca_key, &new)?;
        let mut leaf_keys = Vec::new();
        for cert in certs.iter().filter(|c| c.encrypted_key_pem.is_some()) {
            let enc = cert.encrypted_key_pem.as_deref().unwrap_or_default();
            let key = decrypt_key_pem_detect(enc, &old).map_err(|_| {
                CommandError::new(
                    "vault",
                    format!(
                        "leaf key of certificate {} cannot be opened with the old passphrase",
                        cert.id
                    ),
                )
            })?;
            leaf_keys.push((cert.id, encrypt_key_pem(&key, &new)?));
        }
        inner.with_db(|db| Ok(db.rewrap_ca_keys(req.ca_id, &new_ca_pem, &leaf_keys)?))?;
        if let Ok(mut guard) = inner.session.lock()
            && let Some(sess) = guard.as_mut().filter(|s| s.ca_id == req.ca_id)
        {
            sess.passphrase = new;
            sess.last_used = Instant::now();
        }
        Ok(ChangePassphraseResponse {
            rewrapped_leaf_keys: leaf_keys.len(),
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[tauri::command]
pub fn ca_lock(state: State<'_, AppState>) -> CmdResult<SessionStatus> {
    state.0.lock_session();
    session_status(&state.0)
}

#[tauri::command]
pub fn ca_session(state: State<'_, AppState>) -> CmdResult<SessionStatus> {
    state.0.expire_session_if_idle();
    session_status(&state.0)
}

fn session_status(inner: &crate::state::Inner) -> CmdResult<SessionStatus> {
    let timeout = inner.auto_lock_timeout();
    let guard = inner
        .session
        .lock()
        .map_err(|_| CommandError::internal("session mutex poisoned"))?;
    Ok(match guard.as_ref() {
        Some(s) => SessionStatus {
            unlocked_ca_id: Some(s.ca_id),
            idle_seconds_left: Some(timeout.saturating_sub(s.last_used.elapsed()).as_secs()),
        },
        None => SessionStatus {
            unlocked_ca_id: None,
            idle_seconds_left: None,
        },
    })
}

#[tauri::command]
pub fn ca_list(state: State<'_, AppState>) -> CmdResult<Vec<CaRecord>> {
    state.0.with_db(|db| Ok(db.list_cas()?))
}

#[derive(Debug, Serialize)]
pub struct CaDetail {
    pub ca: CaRecord,
    pub info: CertificateInfo,
    pub chain_pem: String,
}

#[tauri::command]
pub fn ca_info(state: State<'_, AppState>, ca_id: i64) -> CmdResult<CaDetail> {
    state.0.with_db(|db| {
        let ca = db
            .get_ca(ca_id)?
            .ok_or_else(|| CommandError::invalid("CA not found"))?;
        let info = parse_certificate_pem(&ca.cert_pem)?;
        let chain_pem = db
            .ca_ancestors(ca_id)?
            .iter()
            .map(|c| c.cert_pem.clone())
            .collect::<Vec<_>>()
            .join("");
        Ok(CaDetail {
            ca,
            info,
            chain_pem,
        })
    })
}

#[tauri::command]
pub fn ca_update_urls(
    state: State<'_, AppState>,
    ca_id: i64,
    crl_url: Option<String>,
    aia_url: Option<String>,
    crl_days: i64,
) -> CmdResult<CaRecord> {
    state.0.with_db(|db| {
        db.update_ca_urls(ca_id, crl_url.as_deref(), aia_url.as_deref(), crl_days)?;
        db.get_ca(ca_id)?
            .ok_or_else(|| CommandError::invalid("CA not found"))
    })
}

/// Export a CA certificate as PEM or DER (for trust distribution / AIA hosting).
#[tauri::command]
pub fn ca_export_cert(
    state: State<'_, AppState>,
    ca_id: i64,
    dest: String,
    format: String,
) -> CmdResult<String> {
    state.0.with_db(|db| {
        let ca = db
            .get_ca(ca_id)?
            .ok_or_else(|| CommandError::invalid("CA not found"))?;
        match format.as_str() {
            "der" => write_public(Path::new(&dest), &certificate_pem_to_der(&ca.cert_pem)?)?,
            _ => write_public(Path::new(&dest), ca.cert_pem.as_bytes())?,
        }
        Ok(dest.clone())
    })
}
