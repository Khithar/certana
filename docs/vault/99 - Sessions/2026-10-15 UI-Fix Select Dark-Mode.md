---
datum: 2026-10-15
tags: [session, ui, bugfix]
---

# Session 2026-10-15 – UI-Fix: Select-Dropdown im Dark-Mode

## Getan
- Befund aus dem manuellen UI-Durchlauf: Im Dialog «Neue Root-CA (Zeremonie)» war das Key-Algorithmus-Dropdown unlesbar (weisse Schrift auf weissem Hintergrund).
- Ursache: Das native `<option>`-Popup von WebKitGTK erbt im Dark-Mode die helle Schriftfarbe, zeichnet den Popup-Hintergrund aber mit dem System-Schema (hell), weil kein `color-scheme` gesetzt war.
- Fix in `src/index.css`: `color-scheme: light` auf `:root`, `color-scheme: dark` auf `.dark`; `select, option, optgroup` erhalten explizit `bg-background text-foreground`.
- Fix in `src/components/ui/index.tsx`: `fieldBase` setzt `text-foreground` explizit (betrifft Input, Textarea, Select).
- Betrifft damit alle Select-Felder der App (CA-Dialoge, Wizard, Filter, Settings, Widerruf).
- `npx tsc --noEmit` und `npm run lint` grün.

## Versionierung pro Build (gleiche Session)
- Frage des Nutzers geklärt: Die beim Ausstellen abgefragte Passphrase ist die der ausstellenden CA; exportierte Schlüssel sind unverschlüsseltes PKCS#8 (Apache/nginx ohne Passwort). Zertifikate werden nicht gelöscht, sondern widerrufen.
- `scripts/bump-version.mjs` + npm-Skripte `bump`, `release`, `release:deb`; ESLint-Block für `scripts/**/*.mjs` mit Node-Globals. Erster Lauf: 0.1.0 → 0.1.1.
- ADR [[05 - Entscheidungen/2026-10-15-versionierung-pro-build]], Build-Seite, CLAUDE.md und README (Abschnitt «Releasing») ergänzt.

## Betriebsentscheid CRL (gleiche Session)
- CRL-Fragen besprochen (Laufzeit, Betrieb ohne Distribution Point, sporadischer Upload, FTP). Entscheid des Nutzers: interner Betrieb **ohne CRL**, keine CRL-URL in den CAs. Upload-Funktionen und Vorwarnung bewusst nicht gebaut. Siehe [[02 - PKI-Konzept/Revocation und CRL#Betrieb ohne CRL]].

## Richtung 1.0.0 (gleiche Session)
Nutzer will auf 1.0.0 und erstes Release. Ideenliste besprochen; umgesetzt wurden:
- **Farbschema-Umschalter:** `src/lib/theme.ts` (System/Hell/Dunkel in `localStorage`), Karte «Darstellung» in den Einstellungen, auch ohne offene DB sichtbar.
- **Repo-URL:** `https://github.com/Khithar/certana` in `src-tauri/Cargo.toml` und Metainfo (homepage + bugtracker). `AppInfo` liefert jetzt `repository` aus `CARGO_PKG_REPOSITORY`.
- **(6) Passphrase-Texte:** Unlock-Dialog, Wizard-Schlüsselschritt und Ausstellen-Hinweis sagen klar, dass die Passphrase der CA gehört und exportierte Schlüssel unverschlüsselt sind.
- **(7) Betrieb ohne CRL:** CRL-URL-Hint «optional», Info-Alert in der CRL-Karte ohne URL, «Entsperren nötig»-Warnung nur mit URL, Dashboard «nicht verwendet».
- **(8) Über Certana:** Karte in den Einstellungen mit Version, Lizenz, Repo-Link (opener-Plugin).
- **(9) Passphrase der ausstellenden CA ändern:** `Database::rewrap_ca_keys` (Transaktion CA- + Leaf-Schlüssel, Audit `ca.change_passphrase`) mit Unit-Test; Command `ca_change_passphrase` (alte Passphrase prüfen → alle Leaf-Schlüssel mit `decrypt_key_pem_detect` öffnen → neu wrappen → Session aktualisieren); Flow-Test erweitert (falsche alte Passphrase, Wechsel, alte scheitert beim Unlock, neue funktioniert, Export danach). `ChangePassphraseDialog` in `CaDialogs.tsx`, Button im CA-Header.
- CHANGELOG «Unreleased» ergänzt (Added/Changed/Fixed).
- Git-Remote `origin` → `https://github.com/Khithar/certana.git` gesetzt (noch nichts gepusht). RustRover: MCP-Server «rustrover» via Auto-Configure (global), `.mcp.json` in `.gitignore`; geteilte Run-Konfigurationen in `.run/` (Tauri dev, Cargo test, Cargo clippy, Frontend checks, Release .deb).
- Stand: cargo test 23+1+2+2 grün, clippy sauber, tsc/eslint/vitest grün, i18n-Parität en/de geprüft.

## Release 1.0.0 (gleiche Session)
- Push nach GitHub über SSH (GitHub-Initialcommit per Merge eingebunden, `-X ours`).
- `npm run bump -- major` → 1.0.0, CHANGELOG `[1.0.0] - 2026-10-15`, lokaler .deb-Build als Probe, Tag `v1.0.0` gepusht → `release.yml` baut deb/AppImage/rpm/NSIS und legt einen **Draft**-Release an, der manuell veröffentlicht wird. Signierung (Tauri/GPG) ist ohne Secrets übersprungen.

## Screenshots und dabei gefundene Fehler (gleiche Session)
- Demo-Datenbank per `src-tauri/tests/demo_db.rs` (läuft nur mit `CERTANA_DEMO_DIR`): Root, Issuing CA, neun Zertifikate (Server/Client/S-MIME/Code-Signing, auslaufende, widerrufene, Notizen). App mit isoliertem `HOME` und `CERTANA_DATA_DIR` gestartet, Eingaben per python-xlib/XTest, Fenster per `import -window` aufgenommen, Audit-Actor in der Demo-DB anonymisiert. Ergebnis: `docs/screenshots/*.png` (1200×950, Dark-Mode), eingebunden in der README.
- Dabei behoben: **«Invalid Date»** in CA-/Zertifikatsdetails (`CertificateInfo` serialisierte `OffsetDateTime` nicht als RFC 3339; Test `certificate_info_dates_serialize_as_rfc3339`). **Lock nach Absturz:** gleicher Host + toter Prozess gilt sofort als verwaist (`lock::process_alive`, Test `dead_process_lock_on_same_host_is_taken_over`). Select-Text in WebKitGTK abgeschnitten (`py-0`), CA-Detailkarte volle Breite, CRL-Historie Spaltenabstand, CRL-Serve-Hinweis nur mit URL.
- Hinweis: `cargo build --release` ohne Tauri-CLI bettet das Frontend nicht ein (lädt devUrl); für Binaries immer `npm run tauri build [-- --no-bundle]`.

## Abschluss
- **1.0.0** und **1.0.1** als GitHub-Releases veröffentlicht (deb, rpm, AppImage, Windows-Installer, SHA256SUMS): https://github.com/Khithar/certana/releases. README mit Screenshots, Installation, Quick start und Windows-Hinweis.

## Icon und Installationstest (gleiche Session)
- Nutzer hat das .deb unter Linux Mint installiert: funktioniert, Startmenü-Eintrag vorhanden.
- Eigenes Icon: Siegel mit gezacktem Rand, Häkchen und Bändern auf blauem Verlauf (Primärfarbe der App). Quelle `packaging/icon/certana.svg`; da kein rsvg-convert vorhanden, Hintergrund per ImageMagick gerendert und mit dem Vordergrund komponiert (`certana-1024.png`). Icon-Satz via `npm run tauri icon packaging/icon/certana-1024.png` → `src-tauri/icons/`.

## Offen
- Noch offen: Release-Signierung (GPG-Secret, Fingerprint in README), Windows-Durchlauf. Icon landet mit dem nächsten Release in den Paketen.
- Später: Tastaturbedienung/ARIA in Dialogen, Löschen widerrufener/abgelaufener Testzertifikate.

## Geänderte Seiten
- [[04 - UI/Screens]] (Hinweis zu nativen Form-Controls und `color-scheme`)
- [[00 - Index]] (Status)
- [[01 - Architektur/Build und Packaging]] (Release-Befehle, Versionierung)
- [[05 - Entscheidungen/ADR-Index]] (neuer ADR)
- [[01 - Architektur/Überblick]] (Command `ca_change_passphrase`)
- [[03 - Datenmodell/Tabellen]] (Audit-Aktionen)
- [[02 - PKI-Konzept/Revocation und CRL]] (Betrieb ohne CRL)
