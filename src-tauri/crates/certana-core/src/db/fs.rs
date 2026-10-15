// SPDX-License-Identifier: GPL-3.0-or-later
//! Detect whether a path lives on a network file system (where SQLite WAL must not be used).

use std::path::Path;

const NETWORK_FS: &[&str] = &[
    "cifs",
    "smb3",
    "smbfs",
    "nfs",
    "nfs4",
    "afs",
    "9p",
    "ncpfs",
    "fuse.sshfs",
    "fuse.rclone",
    "fuse.gvfsd-fuse",
    "davfs",
    "fuse.davfs2",
];

/// Linux: longest-prefix match of the canonical path against `/proc/mounts`.
/// Other platforms: UNC paths (`\\server\share`) count as network, everything else as local.
pub fn is_network_path(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else {
            return false;
        };
        let probe = existing_ancestor(path);
        let Ok(canonical) = probe.canonicalize() else {
            return false;
        };
        is_network_in_mounts(&mounts, &canonical)
    }
    #[cfg(not(target_os = "linux"))]
    {
        path.to_string_lossy().starts_with("\\\\")
    }
}

/// The deepest existing ancestor (the file itself may not exist yet).
fn existing_ancestor(path: &Path) -> std::path::PathBuf {
    let mut p = path.to_path_buf();
    while !p.exists() {
        match p.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => p = parent.to_path_buf(),
            _ => return std::path::PathBuf::from("."),
        }
    }
    p
}

pub(crate) fn is_network_in_mounts(mounts: &str, canonical: &Path) -> bool {
    let path_str = canonical.to_string_lossy();
    let mut best: Option<(usize, &str)> = None;
    for line in mounts.lines() {
        let mut parts = line.split_whitespace();
        let (Some(_dev), Some(mount), Some(fstype)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let mount = mount.replace("\\040", " ");
        let matches =
            mount == "/" || path_str == mount || path_str.starts_with(&format!("{mount}/"));
        if matches && best.map(|(len, _)| mount.len() >= len).unwrap_or(true) {
            let _ = best.insert((mount.len(), fstype));
        }
    }
    best.map(|(_, fstype)| NETWORK_FS.iter().any(|n| fstype.eq_ignore_ascii_case(n)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "/dev/sda1 / ext4 rw 0 0\n//nas/share /mnt/nas cifs rw 0 0\nserver:/export /mnt/nfs nfs4 rw 0 0\n/dev/sdb1 /mnt/nasty\\040dir ext4 rw 0 0\n";

    #[test]
    fn detects_network_mounts_by_longest_prefix() {
        assert!(is_network_in_mounts(
            MOUNTS,
            Path::new("/mnt/nas/certana.db")
        ));
        assert!(is_network_in_mounts(MOUNTS, Path::new("/mnt/nfs/x/y.db")));
        assert!(!is_network_in_mounts(
            MOUNTS,
            Path::new("/home/u/certana.db")
        ));
        assert!(!is_network_in_mounts(
            MOUNTS,
            Path::new("/mnt/nasty dir/certana.db")
        ));
        assert!(!is_network_in_mounts(
            MOUNTS,
            Path::new("/mnt/nas2/certana.db")
        ));
    }
}
