// SPDX-License-Identifier: GPL-3.0-or-later
//! File helpers. Private material is written with 0600 on Unix and never logged.

use std::path::Path;

use super::error::CmdResult;

fn ensure_parent(path: &Path) -> CmdResult<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn write_private(path: &Path, data: &[u8]) -> CmdResult<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    ensure_parent(path)?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(data)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn write_private(path: &Path, data: &[u8]) -> CmdResult<()> {
    ensure_parent(path)?;
    std::fs::write(path, data)?;
    Ok(())
}

pub fn write_public(path: &Path, data: &[u8]) -> CmdResult<()> {
    ensure_parent(path)?;
    std::fs::write(path, data)?;
    Ok(())
}

/// Turn a display name into a safe file stem.
pub fn safe_stem(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() {
        "certificate".into()
    } else {
        s
    }
}

/// Read a small text file chosen through the dialog plugin (CSR/PEM import). Capped at 1 MiB.
#[tauri::command]
pub fn read_text_file(path: String) -> CmdResult<String> {
    let meta = std::fs::metadata(&path)?;
    if meta.len() > 1024 * 1024 {
        return Err(super::error::CommandError::invalid(
            "file is larger than 1 MiB",
        ));
    }
    Ok(std::fs::read_to_string(&path)?)
}
