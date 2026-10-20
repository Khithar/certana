---
type: concept
status: active
tags: [type/concept, area/datenmodell]
---

# Shared-SQLite-Betrieb

Zurück zu [[03 - Datenmodell/Tabellen]]. Entscheidung: [[05 - Entscheidungen/2026-10-20-shared-db-lockfile-heartbeat]].

SQLite-WAL funktioniert nicht über Netzwerkdateisysteme (Shared-Memory-Index), und SMB/NFS-Locking gilt laut sqlite.org als unzuverlässig. Certana unterstützt eine geteilte DB deshalb mit klaren Regeln. Die Garantie «nur ein Schreiber» liefert nicht SQLite, sondern die eigene Lock-Datei.

| Situation | Verhalten |
|---|---|
| DB auf lokalem Dateisystem (Default `~/.local/share/certana/certana.db`) | `journal_mode=WAL`, normales Locking |
| DB auf cifs/smb3/nfs/nfs4/sshfs/davfs-Mount (Erkennung über `/proc/mounts`, längster Mount-Prefix; `db::fs`) oder `OpenOptions::force_shared` | `journal_mode=DELETE`, `synchronous=FULL`, `busy_timeout=5000`; **kein** `locking_mode=EXCLUSIVE` mehr, damit lesende Instanzen zwischen den Transaktionen des Schreibers lesen können |
| Lock-Datei `<db>.lock` (Host, User, PID, `acquired_at`, `refreshed_at`) existiert und ist < 10 min alt (gemessen an `refreshed_at`, sonst `acquired_at`) | zweite Instanz öffnet **read-only** mit Hinweis «geöffnet von user@host» |
| Lock-Datei älter als 10 min, oder gleicher Host und Prozess tot | gilt als verwaist, wird übernommen |
| Heartbeat | Der Halter schreibt die Lock-Datei alle 2 min neu (`lock::REFRESH_EVERY`, eigener Thread im `LockGuard`). Findet er dabei eine fremde Identität vor (Rechner war > 10 min im Ruhezustand), setzt er `lock_lost`; `ensure_writable` lehnt danach alle Schreibzugriffe ab, `OpenInfo.lock_lost` zeigt es im UI, Neu-Öffnen behebt es. |
| Guard-Drop | löscht die Lock-Datei nur, wenn sie noch die eigene Identität trägt |

Implementiert in `db/mod.rs` (`Database::open`, `info()`, `ensure_writable`), `db/lock.rs` (JSON-Lock, `STALE_AFTER` = 10 min, `acquire_with` für Tests mit kurzem Intervall) und `db/fs.rs`. Unter Windows/macOS gilt nur ein UNC-Pfad als Netzfreigabe; gemappte Laufwerksbuchstaben müssen über «Als geteilte Datenbank behandeln» markiert werden.

## Was für ein Team auf der Freigabe liegt

- **Datenbank** `certana.db` plus `certana.db.lock` und (während Transaktionen) `certana.db-journal`. Alle Arbeitsplätze öffnen dieselbe Datei; der Pfad wird pro Arbeitsplatz in `localStorage` gemerkt und beim Start wieder geöffnet. Schlägt das fehl (Freigabe nicht gemountet), zeigt der Datenbank-Screen den Fehler und behält den Pfad, statt stillschweigend die lokale Default-DB anzubieten.
- **Einstellungen** (Subject-Vorgaben, Export-/CRL-Verzeichnis, Sprache, Auto-Lock) liegen in der DB und gelten damit für alle. Pfade müssen auf allen Arbeitsplätzen gleich gemountet sein.
- **Root-Bundle** (`*.certana-root`): frei gewählter Pfad, wird bei jeder Root-Zeremonie abgefragt. Darf auf der Freigabe liegen (age-verschlüsselt), empfohlen bleibt ein USB-Stick.
- **Nicht geteilt:** Farbschema und zuletzt geöffneter Pfad (pro Arbeitsplatz), entsperrte CA-Sitzung (nur im Speicher).
- Vorbelegung für alle Benutzer: `CERTANA_DATA_DIR=/mnt/pki/certana` (z. B. in `/etc/environment`) verschiebt den Default-Pfad auf die Freigabe.

Ergänzend: «Backup»-Funktion (`Database::backup_to`) (rusqlite Online-Backup-API) für konsistente Kopien auf die Freigabe und ein Hinweis im Settings-Screen, dass das Root-Bundle separat gesichert wird.
