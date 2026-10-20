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

/// GNOME/gvfs FUSE mounts (file manager «Connect to server», `/run/user/<uid>/gvfs/…`). They
/// reject random-access writes (`pwrite` → EOPNOTSUPP), so SQLite cannot run on them at all.
const GVFS_FS: &str = "fuse.gvfsd-fuse";

/// Linux: longest-prefix match of the canonical path against `/proc/mounts`.
/// Other platforms: UNC paths (`\\server\share`) count as network, everything else as local.
pub fn is_network_path(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        mount_fstype_of(path).is_some_and(|fstype| is_network_fstype(&fstype))
    }
    #[cfg(not(target_os = "linux"))]
    {
        path.to_string_lossy().starts_with("\\\\")
    }
}

/// Whether the path lives on a gvfs FUSE mount, which SQLite cannot write to.
pub fn is_gvfs_path(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        mount_fstype_of(path).is_some_and(|fstype| fstype.eq_ignore_ascii_case(GVFS_FS))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        false
    }
}

#[cfg(target_os = "linux")]
fn mount_fstype_of(path: &Path) -> Option<String> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    let canonical = existing_ancestor(path).canonicalize().ok()?;
    mount_fstype(&mounts, &canonical).map(str::to_owned)
}

fn is_network_fstype(fstype: &str) -> bool {
    NETWORK_FS.iter().any(|n| fstype.eq_ignore_ascii_case(n))
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

#[cfg(test)]
fn is_network_in_mounts(mounts: &str, canonical: &Path) -> bool {
    mount_fstype(mounts, canonical).is_some_and(is_network_fstype)
}

/// File-system type of the mount holding `canonical` (longest matching mount point).
fn mount_fstype<'a>(mounts: &'a str, canonical: &Path) -> Option<&'a str> {
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
    best.map(|(_, fstype)| fstype)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "/dev/sda1 / ext4 rw 0 0\n//nas/share /mnt/nas cifs rw 0 0\nserver:/export /mnt/nfs nfs4 rw 0 0\n/dev/sdb1 /mnt/nasty\\040dir ext4 rw 0 0\ngvfsd-fuse /run/user/1000/gvfs fuse.gvfsd-fuse rw 0 0\n";

    #[test]
    fn gvfs_is_recognised_as_network_and_as_gvfs() {
        let p = Path::new("/run/user/1000/gvfs/smb-share:server=nas,share=x/dir/certana.db");
        assert!(is_network_in_mounts(MOUNTS, p));
        assert_eq!(mount_fstype(MOUNTS, p), Some(GVFS_FS));
        assert_eq!(mount_fstype(MOUNTS, Path::new("/mnt/nas/a.db")), Some("cifs"));
    }

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
