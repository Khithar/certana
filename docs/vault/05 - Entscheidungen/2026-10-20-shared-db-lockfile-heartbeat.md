---
type: adr
status: accepted
tags: [type/adr, area/datenmodell]
---

# ADR: Geteilte Datenbank – Lock-Datei mit Heartbeat statt exklusivem SQLite-Lock

## Kontext
Mehrere Benutzer sollen von verschiedenen PCs mit derselben Certana-Datenbank auf einer Netzfreigabe arbeiten. Der bisherige Shared-Modus (seit 1.0.0) hatte zwei Lücken: Die Lock-Datei wurde nur beim Öffnen geschrieben, nach 10 Minuten galt sie als verwaist und ein zweiter PC übernahm den Schreibzugriff, obwohl der erste noch schrieb. Und `locking_mode=EXCLUSIVE` hielt den SQLite-Lock dauerhaft, sodass lesende Instanzen auf Dateisystemen mit funktionierenden Byte-Range-Locks nach 5 s «database is locked» bekamen.

## Entscheidung
- Die Lock-Datei ist die einzige Quelle für «wer schreibt». Der Halter frischt sie alle 2 Minuten aus einem Hintergrund-Thread auf (`refreshed_at`); Verwaisung wird an der letzten Auffrischung gemessen.
- Stellt der Halter beim Auffrischen eine fremde Identität fest, markiert er den Lock als verloren. `ensure_writable` lehnt Schreibzugriffe ab, das UI zeigt «Schreibsperre verloren», Neu-Öffnen stellt den Zustand wieder her.
- Im Shared-Modus kein `locking_mode=EXCLUSIVE` mehr; es bleiben `journal_mode=DELETE` und `synchronous=FULL`. SQLite-Locks sind nur noch Sicherheitsnetz.
- Lock-Dateien von 1.0.x (ohne `refreshed_at`) werden weiter gelesen.

Nicht gewählt: Client-Server-Datenbank (zu schwer für ein Desktop-Werkzeug), Datei-Locks über `flock` (auf SMB/NFS nicht verlässlich), Heartbeat aus dem Frontend (Core soll UI-unabhängig korrekt bleiben).

## Konsequenzen
- Ein schlafender Laptop verliert nach 10 Minuten den Schreibzugriff an den nächsten Öffner, kann aber keine Schreibkonflikte mehr erzeugen.
- Lesende Instanzen sehen Änderungen des Schreibers nach dessen Commit, ohne Fehler.
- Tests: `db::lock::tests` (Heartbeat, Übernahme, Verwaisung nach `refreshed_at`, Legacy-Format), `db::tests::shared_mode_uses_lock_file_and_second_opener_is_read_only` prüft `locking_mode=normal`.
