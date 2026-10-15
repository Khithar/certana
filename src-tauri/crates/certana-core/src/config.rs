// SPDX-License-Identifier: GPL-3.0-or-later
//! Where Certana keeps its own files by default.

use std::path::PathBuf;

/// `$XDG_DATA_HOME/certana` (Linux), `%APPDATA%\certana` (Windows), `~/Library/Application Support/certana` (macOS).
pub fn default_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CERTANA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("certana");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("certana");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .and_then(|p| {
            if p.is_absolute() {
                Ok(p)
            } else {
                Err(std::env::VarError::NotPresent)
            }
        })
    {
        return xdg.join("certana");
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    home.join(".local").join("share").join("certana")
}

pub fn default_db_path() -> PathBuf {
    default_data_dir().join("certana.db")
}

#[cfg(test)]
mod tests {
    #[test]
    fn env_override_wins() {
        // SAFETY: test-only, single-threaded access to this variable.
        unsafe { std::env::set_var("CERTANA_DATA_DIR", "/tmp/certana-test") };
        assert_eq!(
            super::default_db_path(),
            std::path::PathBuf::from("/tmp/certana-test/certana.db")
        );
        unsafe { std::env::remove_var("CERTANA_DATA_DIR") };
    }
}
