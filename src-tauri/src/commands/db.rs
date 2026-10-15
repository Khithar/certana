// SPDX-License-Identifier: GPL-3.0-or-later
use certana_core::db::{Database, OpenInfo, OpenOptions};
use tauri::State;

use super::error::CmdResult;
use crate::state::AppState;

#[tauri::command]
pub fn db_default_path() -> String {
    certana_core::config::default_db_path()
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
pub fn db_open(
    state: State<'_, AppState>,
    path: Option<String>,
    force_shared: Option<bool>,
) -> CmdResult<OpenInfo> {
    let path = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(certana_core::config::default_db_path);
    let db = Database::open(
        &path,
        OpenOptions {
            force_shared: force_shared.unwrap_or(false),
            read_only: false,
        },
    )?;
    if !db.is_read_only() {
        let expired = db.refresh_expired()?;
        if expired > 0 {
            tracing::info!(expired, "marked certificates as expired");
        }
    }
    let info = db.info().clone();
    state.0.lock_session();
    *state.0.db()? = Some(db);
    tracing::info!(path = %info.path.display(), mode = ?info.mode, read_only = info.read_only, "database opened");
    Ok(info)
}

#[tauri::command]
pub fn db_info(state: State<'_, AppState>) -> CmdResult<Option<OpenInfo>> {
    Ok(state.0.db()?.as_ref().map(|d| d.info().clone()))
}

#[tauri::command]
pub fn db_close(state: State<'_, AppState>) -> CmdResult<()> {
    state.0.lock_session();
    *state.0.db()? = None;
    Ok(())
}

#[tauri::command]
pub fn db_backup(state: State<'_, AppState>, dest: String) -> CmdResult<String> {
    state.0.with_db(|db| {
        db.backup_to(&dest)?;
        if !db.is_read_only() {
            db.audit(
                "db.backup",
                "database",
                "-",
                serde_json::json!({ "dest": dest }),
            )?;
        }
        Ok(dest.clone())
    })
}
