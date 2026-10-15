// SPDX-License-Identifier: GPL-3.0-or-later
use certana_core::db::models::{
    AuditRecord, CaRecord, CertStatus, CertificateFilter, CertificateRecord, CrlRecord,
};
use serde::Serialize;
use tauri::State;

use super::error::CmdResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct CaStatus {
    pub ca: CaRecord,
    pub latest_crl: Option<CrlRecord>,
    pub crl_expires_soon: bool,
    pub unlocked: bool,
}

#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    pub cas: Vec<CaStatus>,
    pub valid: usize,
    pub revoked: usize,
    pub expired: usize,
    pub expiring_30: Vec<CertificateRecord>,
    pub expiring_14: usize,
    pub expiring_7: usize,
    pub recent_audit: Vec<AuditRecord>,
}

#[tauri::command]
pub fn dashboard_summary(state: State<'_, AppState>) -> CmdResult<DashboardSummary> {
    let inner = state.0.clone();
    inner.expire_session_if_idle();
    let unlocked = inner
        .session
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|x| x.ca_id));
    inner.with_db(|db| {
        if !db.is_read_only() {
            db.refresh_expired()?;
        }
        let now = time::OffsetDateTime::now_utc();
        let cas = db
            .list_cas()?
            .into_iter()
            .map(|ca| {
                let latest_crl = db.latest_crl(ca.id)?;
                let crl_expires_soon = latest_crl
                    .as_ref()
                    .map(|c| c.next_update - now < time::Duration::days(2))
                    .unwrap_or(false);
                Ok(CaStatus {
                    unlocked: unlocked == Some(ca.id),
                    latest_crl,
                    crl_expires_soon,
                    ca,
                })
            })
            .collect::<CmdResult<Vec<_>>>()?;
        let count = |status: CertStatus| -> CmdResult<usize> {
            Ok(db
                .list_certificates(&CertificateFilter {
                    status: Some(status),
                    ..Default::default()
                })?
                .len())
        };
        let expiring_30 = db.list_certificates(&CertificateFilter {
            expiring_within_days: Some(30),
            ..Default::default()
        })?;
        let within = |days: i64| {
            expiring_30
                .iter()
                .filter(|c| c.not_after - now <= time::Duration::days(days))
                .count()
        };
        Ok(DashboardSummary {
            cas,
            valid: count(CertStatus::Valid)?,
            revoked: count(CertStatus::Revoked)?,
            expired: count(CertStatus::Expired)?,
            expiring_14: within(14),
            expiring_7: within(7),
            expiring_30,
            recent_audit: db.recent_audit(10)?,
        })
    })
}
