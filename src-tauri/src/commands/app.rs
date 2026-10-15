// SPDX-License-Identifier: GPL-3.0-or-later
use serde::Serialize;

#[derive(Serialize)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub license: &'static str,
}

/// Static application metadata for the About dialog.
#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Certana",
        version: env!("CARGO_PKG_VERSION"),
        license: "GPL-3.0-or-later",
    }
}
