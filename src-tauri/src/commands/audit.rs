// SPDX-License-Identifier: GPL-3.0-or-later
use certana_core::db::models::AuditRecord;
use tauri::State;

use super::error::CmdResult;
use super::files::write_public;
use crate::state::AppState;

#[tauri::command]
pub fn audit_recent(state: State<'_, AppState>, limit: Option<u32>) -> CmdResult<Vec<AuditRecord>> {
    state
        .0
        .with_db(|db| Ok(db.recent_audit(limit.unwrap_or(50).min(5000))?))
}

#[tauri::command]
pub fn audit_export_csv(state: State<'_, AppState>, dest: String) -> CmdResult<String> {
    state.0.with_db(|db| {
        let csv = db.audit_csv()?;
        write_public(std::path::Path::new(&dest), csv.as_bytes())?;
        Ok(dest.clone())
    })
}
