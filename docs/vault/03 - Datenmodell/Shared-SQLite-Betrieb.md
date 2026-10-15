---
type: concept
status: active
tags: [type/concept, area/datenmodell]
---

# Shared-SQLite-Betrieb

Zurück zu [[03 - Datenmodell/Tabellen]].

SQLite-WAL funktioniert nicht über Netzwerkdateisysteme (Shared-Memory-Index), und SMB/NFS-Locking gilt laut sqlite.org als unzuverlässig. Certana unterstützt eine geteilte DB deshalb mit klaren Regeln:

| Situation | Verhalten |
|---|---|
| DB auf lokalem Dateisystem (Default `~/.local/share/certana/certana.db`) | `journal_mode=WAL`, normales Locking |
| DB auf cifs/smb3/nfs/nfs4/sshfs/davfs-Mount (Erkennung über `/proc/mounts`, längster Mount-Prefix; `db::fs`) oder `OpenOptions::force_shared` | `journal_mode=DELETE`, `locking_mode=EXCLUSIVE`, `busy_timeout=5000` |
| Lock-Datei `<db>.lock` (Host, User, PID, Zeit) existiert und ist < 10 min alt | zweite Instanz öffnet **read-only** mit Hinweis «geöffnet von user@host» |
| Lock-Datei älter als 10 min | gilt als verwaist, wird übernommen (Hinweis im UI) |

Implementiert in `db/mod.rs` (`Database::open`), `db/lock.rs` (JSON-Lock, `STALE_AFTER` = 10 min, Guard löscht nur den eigenen Lock) und `db/fs.rs`. Unter Windows/macOS gilt nur ein UNC-Pfad als Netzfreigabe.

Ergänzend: «Backup»-Funktion (`Database::backup_to`) (rusqlite Online-Backup-API) für konsistente Kopien auf die Freigabe und ein Hinweis im Settings-Screen, dass das Root-Bundle separat gesichert wird.
