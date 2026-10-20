---
datum: 2026-10-20
tags: [session, datenmodell, betrieb]
---

# Session 2026-10-20 – Mehrbenutzerbetrieb auf Freigabe

## Getan
- Frage des Nutzers: Wo liegen Root- und Issuing-CAs, um «from scratch» zu testen? Antwort: DB `~/.local/share/certana/certana.db` (+ wal/shm/lock), Root-Bundle am frei gewählten Ort (hier `testing.certana-root` im Repo, gitignored), Webview-State unter `~/.local/share/io.github.certana/`. Nachgetragen in [[06 - Betrieb/Installation#Daten]].
- Anforderung: Datenverzeichnis muss wählbar sein, damit mehrere Benutzer von verschiedenen PCs arbeiten. Befund: Der DB-Pfad war bereits in Einstellungen → Datenbank wählbar (Netzlaufwerk-Erkennung, Lock-Datei, read-only für zweite Instanz). Beim Review aber zwei echte Lücken gefunden und behoben:
  1. **Lock ohne Heartbeat:** `acquired_at` wurde nie aufgefrischt, nach 10 min übernahm ein zweiter PC den Schreibzugriff, obwohl der erste noch schrieb. Jetzt `LockGuard` mit Heartbeat-Thread (alle 2 min, `refreshed_at`), Verwaisung nach letzter Auffrischung, Übernahme-Erkennung → `lock_lost`, `ensure_writable` lehnt ab, `OpenInfo.lock_lost` im UI («Schreibsperre verloren», neu öffnen).
  2. **`locking_mode=EXCLUSIVE`** im Shared-Modus blockierte lesende Instanzen; entfernt (DELETE-Journal + `synchronous=FULL` bleiben).
- UI: Datenbank-Karte mit Team-Hinweis, Pfad-Hint, Lock-verloren-Warnung; `db_info` wird mit der Session alle 15 s gepollt. Beim Start bleibt der zuletzt geöffnete Pfad stehen und der Fehler wird gezeigt, wenn das Öffnen scheitert (Freigabe nicht gemountet), statt stillschweigend die lokale Default-DB anzubieten.
- Tests: `db::lock::tests` (4 neue: Heartbeat, Übernahme, Verwaisung nach `refreshed_at`, Legacy-Lock ohne `refreshed_at`), Shared-Test prüft `locking_mode=normal`. cargo test/clippy/fmt, eslint, tsc, vitest grün; i18n en/de ergänzt.
- ADR [[05 - Entscheidungen/2026-10-20-shared-db-lockfile-heartbeat]], CHANGELOG «Unreleased», README-Absatz zu Team-Betrieb und `CERTANA_DATA_DIR`.

## Praxistest auf SMB (gleiche Session)
- Nutzer legte die DB auf `/run/user/1000/gvfs/smb-share:…/SSL_certificates/SiriusCA/certana.db` (Nemo «Mit Server verbinden»). Root-Zeremonie: Bundle wurde geschrieben, DB-Insert scheiterte mit «unable to open database file»; `certana.db-journal` blieb liegen.
- Nachgestellt mit `CERTANA_DEMO_DIR` auf dem Share (`disk I/O error` beim ersten Setting) und per strace: `pwrite64` auf `certana.db` → `EOPNOTSUPP`. gvfs-FUSE kann keine wahlfreien Schreibzugriffe, SQLite ist dort grundsätzlich unbrauchbar (Python-Probe lief nur zufällig, weil sequentiell geschrieben wurde).
- Fix: `db::fs::is_gvfs_path` (Mount-Typ `fuse.gvfsd-fuse`), `Database::open` verweigert mit Hinweis auf cifs/nfs; Datenbank-Karte zeigt bei `/gvfs/`-Pfaden eine Fehlermeldung und sperrt «Öffnen». Test `gvfs_is_recognised_as_network_and_as_gvfs`. Installationsseite mit cifs-Mount- und fstab-Beispiel.
- Zu bereinigen auf dem Share: `certana.db`, `certana.db-journal`, `certana.db.lock` und die Probe-Verzeichnisse `_probe`, `_probe2` (gvfs lieferte beim Löschen I/O-Fehler). Das Bundle `Sirius-Root-CA.certana-root` ist gültig, die Root ist aber nicht in der DB registriert; Zeremonie nach dem cifs-Mount wiederholen.

## cifs ohne nobrl (gleiche Session)
- Nach Umstellung auf einen cifs-Kernel-Mount: «migration error … database is locked» nach 5 s beim Anlegen der DB. Mit Demo-Flow reproduziert; Ursache: Mount ohne `nobrl`, SQLites fcntl-Locks werden als Mandatory-Locks zum NAS geschickt. Mit `nobrl` läuft der Demo-Flow durch (49 s), Nutzer hat das .deb auf dem neuen Mount erfolgreich getestet.
- `fs::StorageIssue` (Gvfs, CifsWithoutNobrl) ersetzt `is_gvfs_path`; `Database::open` verweigert beide, Command `db_check_path` + `api.dbCheckPath` prüfen den Pfad in der Datenbank-Karte (debounced) und zeigen die übersetzte Erklärung, Öffnen-Button gesperrt. Tests in `db::fs::tests`, Flow-Test prüft `db_check_path`. Installationsseite: `nobrl` in Mount- und fstab-Beispiel.

## App-ID (gleiche Session)
- Frage des Nutzers zum Ordner `~/.local/share/io.github.certana/`: das ist der Webview-Profilordner aus der Tauri-`identifier`. Zuerst `com.siriusag.certana` erwogen (Firma gehört dem Entwickler), nach Abwägung Entscheid für `io.github.khithar.certana`: persönliches GPL-Projekt, ID soll Firmenänderungen überdauern, Flathub-Nachweis über das Repo. ADR [[05 - Entscheidungen/2026-10-20-app-id-github]], Packaging-Dateien umbenannt, Metainfo mit `developer` Certana Contributors und Release-Einträgen 1.0.0–1.0.2, CHANGELOG.
- Nebenbei: fstab-/Mount-Anleitung für die nas02-Shares (IPsec, `nofail`, `x-systemd.automount`) und Variante mit Passwortfenster beim Login (zenity + root-Helfer + sudoers, kein Passwort auf der Platte) im Scratchpad erstellt; nicht Teil des Repos.

## Offen
- Praxistest auf einem cifs-Kernel-Mount mit zwei PCs (Mount-Optionen, Lese-Latenz, Lock-Datei-Sichtbarkeit).
- Windows: gemappte Laufwerksbuchstaben werden nicht als Netzfreigabe erkannt (nur UNC); Workaround Checkbox «Als geteilte Datenbank behandeln».
- Nicht committet (Commits nur auf Wunsch).

## Geänderte Seiten
- [[03 - Datenmodell/Shared-SQLite-Betrieb]] (Heartbeat, kein EXCLUSIVE, Abschnitt «Was für ein Team auf der Freigabe liegt»)
- [[06 - Betrieb/Installation]] (Daten, Mehrere Benutzer)
- [[05 - Entscheidungen/ADR-Index]] (neuer ADR)
- [[00 - Index]] (Status Phase 10)
