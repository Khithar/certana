// SPDX-License-Identifier: GPL-3.0-or-later
//! CRL publication for issuing CAs (session key) and roots (bundle ceremony).

use std::path::{Path, PathBuf};

use certana_core::db::Database;
use certana_core::db::models::CrlRecord;
use certana_core::db::settings::keys as setting_keys;
use certana_core::pki::crl::{RevokedEntry, build_crl};
use certana_core::pki::keys::PrivateKey;
use certana_core::vault::rootbundle::read_bundle;
use secrecy::SecretString;
use serde::Serialize;
use tauri::State;

use super::error::{CmdResult, CommandError};
use super::files::{safe_stem, write_public};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct CrlPublished {
    pub crl: CrlRecord,
    pub der_path: Option<String>,
    pub pem_path: Option<String>,
    pub revoked_count: usize,
}

fn publish(
    db: &Database,
    ca_id: i64,
    key: &PrivateKey,
    export_dir: Option<&str>,
) -> CmdResult<CrlPublished> {
    let ca = db
        .get_ca(ca_id)?
        .ok_or_else(|| CommandError::invalid("CA not found"))?;
    let pending = db.revocations_for_crl(ca_id, ca.crl_days.max(1))?;
    let entries: Vec<RevokedEntry> = pending
        .iter()
        .map(|(c, r)| RevokedEntry {
            serial_hex: c.serial_hex.clone(),
            revoked_at: r.revoked_at,
            reason: r.reason,
            invalidity_date: r.invalidity_date,
        })
        .collect();
    let number = db.next_crl_number(ca_id)?;
    let out = build_crl(
        &ca.cert_pem,
        key,
        number as u64,
        ca.crl_days.max(1) as u32,
        &entries,
    )?;
    let dir = export_dir
        .map(String::from)
        .or(db.get_setting(setting_keys::CRL_EXPORT_DIR)?)
        .filter(|d| !d.trim().is_empty());
    let (der_path, pem_path) = match dir {
        Some(d) => {
            let stem = safe_stem(&ca.name);
            let der: PathBuf = Path::new(&d).join(format!("{stem}.crl"));
            let pem: PathBuf = Path::new(&d).join(format!("{stem}.crl.pem"));
            write_public(&der, &out.der)?;
            write_public(&pem, out.pem.as_bytes())?;
            (
                Some(der.display().to_string()),
                Some(pem.display().to_string()),
            )
        }
        None => (None, None),
    };
    let crl = db.insert_crl(
        ca_id,
        number,
        out.this_update,
        out.next_update,
        &out.der,
        der_path.as_deref(),
    )?;
    Ok(CrlPublished {
        crl,
        der_path,
        pem_path,
        revoked_count: entries.len(),
    })
}

#[tauri::command]
pub fn crl_publish(
    state: State<'_, AppState>,
    ca_id: i64,
    export_dir: Option<String>,
) -> CmdResult<CrlPublished> {
    let inner = state.0.clone();
    inner.with_session(ca_id, |session| {
        inner.with_db(|db| publish(db, ca_id, &session.key, export_dir.as_deref()))
    })
}

#[tauri::command]
pub async fn crl_publish_root(
    state: State<'_, AppState>,
    ca_id: i64,
    bundle_path: String,
    passphrase: String,
    export_dir: Option<String>,
) -> CmdResult<CrlPublished> {
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let bundle = read_bundle(
            &std::fs::read(&bundle_path)?,
            &SecretString::from(passphrase),
        )?;
        inner.with_db(|db| {
            let ca = db
                .get_ca(ca_id)?
                .ok_or_else(|| CommandError::invalid("CA not found"))?;
            if ca.cert_sha256 != bundle.manifest.certificate_sha256 {
                return Err(CommandError::invalid(
                    "the bundle does not belong to this CA",
                ));
            }
            let res = publish(db, ca_id, &bundle.key, export_dir.as_deref())?;
            db.audit(
                "root.ceremony",
                "ca",
                ca_id,
                serde_json::json!({ "operation": "crl" }),
            )?;
            Ok(res)
        })
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[tauri::command]
pub fn crl_list(
    state: State<'_, AppState>,
    ca_id: i64,
    limit: Option<u32>,
) -> CmdResult<Vec<CrlRecord>> {
    state
        .0
        .with_db(|db| Ok(db.list_crls(ca_id, limit.unwrap_or(20))?))
}

/// Write the latest stored CRL of a CA to a file (DER or PEM by extension).
#[tauri::command]
pub fn crl_export(state: State<'_, AppState>, ca_id: i64, dest: String) -> CmdResult<String> {
    state.0.with_db(|db| {
        let crl = db
            .latest_crl(ca_id)?
            .ok_or_else(|| CommandError::invalid("no CRL published yet"))?;
        if dest.ends_with(".pem") {
            let pem = certana_core::pki::export::crl_der_to_pem(&crl.crl_der);
            write_public(Path::new(&dest), pem.as_bytes())?;
        } else {
            write_public(Path::new(&dest), &crl.crl_der)?;
        }
        Ok(dest.clone())
    })
}
