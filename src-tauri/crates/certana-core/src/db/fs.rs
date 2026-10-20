// SPDX-License-Identifier: GPL-3.0-or-later
//! Detect what kind of file system a database path lives on: network mounts (where SQLite WAL
//! must not be used) and mounts SQLite cannot work on at all.

use std::path::Path;

use serde::{Deserialize, Serialize};

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

/// A mount SQLite is known not to work on. `Database::open` refuses these with an explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageIssue {
    /// gvfs FUSE mount: no random-access writes.
    Gvfs,
    /// Linux cifs mount without `nobrl`: SQLite's byte-range locks are sent to the server as
    /// mandatory locks and collide with its own reads, every write ends in «database is locked».
    CifsWithoutNobrl,
}

impl StorageIssue {
    pub fn explain(self, path: &Path) -> String {
        match self {
            StorageIssue::Gvfs => format!(
                "{} lies on a gvfs mount (file manager «Connect to server»), which does not support \
                 random-access writes; SQLite cannot work there. Mount the share with cifs or nfs \
                 (e.g. via /etc/fstab) and open the database from that mount point.",
                path.display()
            ),
            StorageIssue::CifsWithoutNobrl => format!(
                "{} lies on a cifs mount without the «nobrl» option; SQLite's locks collide with \
                 the server's mandatory byte-range locks and every write fails with «database is \
                 locked». Remount the share with «-o …,nobrl» (Certana guards the file with its own \
                 lock file, so no byte-range locks are needed).",
                path.display()
            ),
        }
    }
}

/// Linux: longest-prefix match of the canonical path against `/proc/mounts`.
/// Other platforms: UNC paths (`\\server\share`) count as network, everything else as local.
pub fn is_network_path(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        mount_entry_of(path).is_some_and(|(fstype, _)| is_network_fstype(&fstype))
    }
    #[cfg(not(target_os = "linux"))]
    {
        path.to_string_lossy().starts_with("\\\\")
    }
}

/// Whether the path lives on a mount SQLite cannot work on.
pub fn storage_issue(path: &Path) -> Option<StorageIssue> {
    #[cfg(target_os = "linux")]
    {
        mount_entry_of(path).and_then(|(fstype, options)| issue_for(&fstype, &options))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        None
    }
}

fn issue_for(fstype: &str, options: &str) -> Option<StorageIssue> {
    if fstype.eq_ignore_ascii_case(GVFS_FS) {
        return Some(StorageIssue::Gvfs);
    }
    if (fstype.eq_ignore_ascii_case("cifs") || fstype.eq_ignore_ascii_case("smb3"))
        && !options.split(',').any(|o| o == "nobrl")
    {
        return Some(StorageIssue::CifsWithoutNobrl);
    }
    None
}

#[cfg(target_os = "linux")]
fn mount_entry_of(path: &Path) -> Option<(String, String)> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    let canonical = existing_ancestor(path).canonicalize().ok()?;
    mount_entry(&mounts, &canonical).map(|(f, o)| (f.to_owned(), o.to_owned()))
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

/// File-system type and mount options of the mount holding `canonical` (longest matching
/// mount point), parsed from `/proc/mounts` content.
fn mount_entry<'a>(mounts: &'a str, canonical: &Path) -> Option<(&'a str, &'a str)> {
    let path_str = canonical.to_string_lossy();
    let mut best: Option<(usize, &str, &str)> = None;
    for line in mounts.lines() {
        let mut parts = line.split_whitespace();
        let (Some(_dev), Some(mount), Some(fstype), Some(options)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let mount = mount.replace("\\040", " ");
        let matches =
            mount == "/" || path_str == mount || path_str.starts_with(&format!("{mount}/"));
        if matches && best.map(|(len, _, _)| mount.len() >= len).unwrap_or(true) {
            let _ = best.insert((mount.len(), fstype, options));
        }
    }
    best.map(|(_, fstype, options)| (fstype, options))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "/dev/sda1 / ext4 rw 0 0\n//nas/share /mnt/nas cifs rw,vers=3.0 0 0\n//nas/ok /mnt/nas-ok cifs rw,vers=3.0,nobrl 0 0\nserver:/export /mnt/nfs nfs4 rw 0 0\n/dev/sdb1 /mnt/nasty\\040dir ext4 rw 0 0\ngvfsd-fuse /run/user/1000/gvfs fuse.gvfsd-fuse rw 0 0\n";

    fn network(p: &str) -> bool {
        mount_entry(MOUNTS, Path::new(p)).is_some_and(|(f, _)| is_network_fstype(f))
    }

    fn issue(p: &str) -> Option<StorageIssue> {
        mount_entry(MOUNTS, Path::new(p)).and_then(|(f, o)| issue_for(f, o))
    }

    #[test]
    fn detects_network_mounts_by_longest_prefix() {
        assert!(network("/mnt/nas/certana.db"));
        assert!(network("/mnt/nfs/x/y.db"));
        assert!(!network("/home/u/certana.db"));
        assert!(!network("/mnt/nasty dir/certana.db"));
        assert!(!network("/mnt/nas2/certana.db"));
    }

    #[test]
    fn gvfs_and_cifs_without_nobrl_are_issues() {
        assert!(network("/run/user/1000/gvfs/smb-share:server=nas,share=x/certana.db"));
        assert_eq!(
            issue("/run/user/1000/gvfs/smb-share:server=nas,share=x/certana.db"),
            Some(StorageIssue::Gvfs)
        );
        assert_eq!(
            issue("/mnt/nas/certana.db"),
            Some(StorageIssue::CifsWithoutNobrl)
        );
        assert_eq!(issue("/mnt/nas-ok/certana.db"), None);
        assert_eq!(issue("/mnt/nfs/certana.db"), None);
        assert_eq!(issue("/home/u/certana.db"), None);
    }
}
