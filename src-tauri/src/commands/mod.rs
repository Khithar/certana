// SPDX-License-Identifier: GPL-3.0-or-later
//! Tauri commands: thin adapters over `certana_core`. No key material crosses this boundary.

pub mod app;
pub mod audit;
pub mod ca;
pub mod cert;
pub mod crl;
pub mod dashboard;
pub mod db;
pub mod error;
pub mod files;
pub mod settings;

/// Everything registered with `tauri::generate_handler!`.
#[macro_export]
macro_rules! all_commands {
    () => {
        tauri::generate_handler![
            $crate::commands::app::app_info,
            $crate::commands::files::read_text_file,
            $crate::commands::db::db_default_path,
            $crate::commands::db::db_check_path,
            $crate::commands::db::db_open,
            $crate::commands::db::db_info,
            $crate::commands::db::db_close,
            $crate::commands::db::db_backup,
            $crate::commands::settings::settings_all,
            $crate::commands::settings::settings_set,
            $crate::commands::ca::ca_list,
            $crate::commands::ca::ca_create_root,
            $crate::commands::ca::ca_import_root,
            $crate::commands::ca::ca_open_root_bundle,
            $crate::commands::ca::ca_create_intermediate,
            $crate::commands::ca::ca_create_intermediate_csr,
            $crate::commands::ca::ca_import_intermediate_cert,
            $crate::commands::ca::ca_unlock,
            $crate::commands::ca::ca_lock,
            $crate::commands::ca::ca_change_passphrase,
            $crate::commands::ca::ca_session,
            $crate::commands::ca::ca_export_cert,
            $crate::commands::ca::ca_update_urls,
            $crate::commands::ca::ca_info,
            $crate::commands::crl::crl_publish,
            $crate::commands::crl::crl_publish_root,
            $crate::commands::crl::crl_list,
            $crate::commands::crl::crl_export,
            $crate::commands::cert::cert_list,
            $crate::commands::cert::cert_get,
            $crate::commands::cert::cert_parse_csr,
            $crate::commands::cert::cert_issue,
            $crate::commands::cert::cert_export,
            $crate::commands::cert::cert_revoke,
            $crate::commands::cert::cert_renew,
            $crate::commands::cert::cert_update_notes,
            $crate::commands::cert::cert_profiles,
            $crate::commands::dashboard::dashboard_summary,
            $crate::commands::audit::audit_recent,
            $crate::commands::audit::audit_export_csv,
        ]
    };
}
