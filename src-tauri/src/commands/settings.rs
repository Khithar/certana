// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::BTreeMap;

use tauri::State;

use super::error::CmdResult;
use crate::state::AppState;

#[tauri::command]
pub fn settings_all(state: State<'_, AppState>) -> CmdResult<BTreeMap<String, String>> {
    state
        .0
        .with_db(|db| Ok(db.all_settings()?.into_iter().collect()))
}

#[tauri::command]
pub fn settings_set(state: State<'_, AppState>, key: String, value: String) -> CmdResult<()> {
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(super::error::CommandError::invalid("invalid setting key"));
    }
    state.0.with_db(|db| Ok(db.set_setting(&key, &value)?))
}
