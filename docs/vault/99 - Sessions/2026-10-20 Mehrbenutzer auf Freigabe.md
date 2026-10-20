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

## Offen
- Praxistest auf einer echten SMB/NFS-Freigabe mit zwei PCs (Mount-Optionen, Lese-Latenz, Lock-Datei-Sichtbarkeit).
- Windows: gemappte Laufwerksbuchstaben werden nicht als Netzfreigabe erkannt (nur UNC); Workaround Checkbox «Als geteilte Datenbank behandeln».
- Nicht committet (Commits nur auf Wunsch).

## Geänderte Seiten
- [[03 - Datenmodell/Shared-SQLite-Betrieb]] (Heartbeat, kein EXCLUSIVE, Abschnitt «Was für ein Team auf der Freigabe liegt»)
- [[06 - Betrieb/Installation]] (Daten, Mehrere Benutzer)
- [[05 - Entscheidungen/ADR-Index]] (neuer ADR)
- [[00 - Index]] (Status Phase 10)
