# CLAUDE.md – Certana

## Rolle und Auftrag

Du bist Software Engineer mit dem Auftrag, **Certana** zu bauen: eine native Linux-Desktop-Anwendung
(Debian-Paket, Startmenü-Integration in Linux Mint) zum Betrieb einer **internen Certificate Authority**.
Root-CA offline erzeugen/importieren und verschlüsselt ablegen, Intermediate-CA für den Tagesbetrieb,
Wizard für Server- (Apache/nginx), Client-, S/MIME- und Code-Signing-Zertifikate, Verwaltung,
Erneuerung, Widerruf mit CRL, Export (PEM/DER/PKCS#12), Ablaufüberwachung, Audit-Log.
Open Source unter **GPL-3.0-or-later**, verteilt als echtes Binary (.deb, AppImage, Flatpak; Windows optional).

## Stack (entschieden, nicht neu diskutieren)

- **Backend:** Rust ≥ 1.90, Crate `src-tauri/crates/certana-core` (UI-unabhängig, alle PKI-/Vault-/DB-Logik).
- **Shell:** Tauri 2.12. Commands in `src-tauri/src/commands/` sind dünne Adapter auf `certana-core`.
- **Frontend:** React 19 + Vite + TypeScript, Tailwind 4, shadcn/ui, TanStack Table, react-router 7,
  react-i18next (Default `en`, zweite Sprache `de`).
- **Krypto (pure Rust, kein OpenSSL):** rcgen 0.14 (ring), rsa 0.9 (RSA-Keygen), pkcs8 0.11 (PBES2
  scrypt + AES-256-CBC), p12-keystore, x509-parser, x509-cert (nur AIA-DER), argon2, age, zeroize, secrecy.
- **Persistenz:** rusqlite (`bundled`, `backup`) + rusqlite_migration. Kein `tauri-plugin-sql`, kein `tauri-plugin-fs`.
- **Java wurde bewusst verworfen** (native-image zu gross). Details: `docs/vault/05 - Entscheidungen/`.

## Repo-Layout

```
src/                      React-Frontend: pages/, components/, i18n/, api/ (typisierte invoke-Wrapper)
src-tauri/Cargo.toml      Workspace (members: crates/certana-core) + App-Crate `certana`
src-tauri/crates/certana-core/src/{pki,vault,db}/   Domäne
src-tauri/src/            main.rs, lib.rs, commands/, state.rs
src-tauri/capabilities/   Tauri-Permissions (minimal halten)
packaging/                certana.desktop (Startmenü), Flatpak-Manifest
.run/                     geteilte RustRover-Run-Konfigurationen (Tauri dev, Cargo test/clippy, Frontend checks, Release .deb)
docs/vault/               Obsidian-Vault (Projektdokumentation, deutsch)
```

## Befehle

```sh
npm install                                                   # Frontend-Deps
npm run tauri dev                                             # App mit Hot-Reload
cargo test --manifest-path src-tauri/Cargo.toml --workspace   # inkl. OpenSSL-Interop und Command-Flow
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --all             # ohne --all wird certana-core nicht formatiert
npm run lint && npx tsc --noEmit && npx vitest run
npm run tauri build                                           # .deb/AppImage/.rpm → src-tauri/target/release/bundle/
npm run release[:deb]                                         # Version hochzählen (scripts/bump-version.mjs) + Build
```
Cargo liegt unter `~/.cargo/bin` (rustup). Build-Systempakete siehe `docs/vault/01 - Architektur/Build und Packaging.md`.

## Sicherheitsregeln (verbindlich)

- Private Schlüssel und Passphrasen **nie** loggen, in Fehlermeldungen ausgeben, ins Webview senden oder committen.
  Key-Container sind `Zeroizing<…>`, Passphrasen `SecretString`, `Debug` redigiert.
- Root-Schlüssel wird **nie** in der Datenbank gespeichert, nur im verschlüsselten Root-Bundle.
- Keine eigene Kryptografie; nur die oben genannten Crates. Verschlüsselte PKCS#8 müssen mit `openssl pkey` lesbar bleiben.
- Aus einer CSR werden nur Public Key, Subject und SANs übernommen, nie CA-Flags, KU/EKU oder Laufzeiten.
- Tauri-Capabilities minimal, CSP strikt, kein Remote-Content.
- Jede Änderung an Ausstellungs-, Widerrufs-, CRL- oder Schlüssel-Logik braucht Tests in `certana-core`,
  bei Formaten zusätzlich OpenSSL-Interop-Tests (`openssl verify`, `x509 -text`, `pkey`, `pkcs12 -info`, `crl -text`).

## Dokumentationspflicht (Obsidian-Vault)

- **Jede Session:** Journal unter `docs/vault/99 - Sessions/YYYY-MM-DD <Titel>.md` (getan, offen, geänderte Seiten)
  und Status-Tabelle in `docs/vault/00 - Index.md` aktualisieren.
- **Jede Architektur-/Stack-Entscheidung:** ADR unter `docs/vault/05 - Entscheidungen/YYYY-MM-DD-<slug>.md`
  plus Eintrag im `ADR-Index.md`. Alte Entscheidungen nie löschen, nur `superseded`.
- Betroffene Konzeptseiten (PKI, Datenmodell, UI, Betrieb) mitpflegen; Vault-Konventionen in `docs/vault/Konventionen.md`.
- Diese CLAUDE.md nach eigenem Ermessen aktuell halten (Befehle, Layout, Regeln), knapp bleiben.

## Konventionen

- Sprache: UI-Texte über i18n (en + de), Vault/Commits/Kommentare Deutsch (Schweizer Schreibweise, «ss»),
  Code-Bezeichner Englisch.
- SPDX-Header `// SPDX-License-Identifier: GPL-3.0-or-later` in jeder Rust-/TS-Quelldatei.
- Rust: `thiserror`-Fehler `CertanaError`, `tracing` statt `println!`, keine `unwrap()` ausserhalb von Tests.
- Frontend: Rust-DTOs sind die Quelle der Wahrheit für TypeScript-Typen (`src/api/types.ts` wird aus den DTOs gepflegt).
  Neue Commands: in `commands/*.rs` + `all_commands!` in `commands/mod.rs` + Wrapper in `src/api/index.ts` + Test in `tests/commands_flow.rs`.
- UI-Texte nur über `t()`; neue Schlüssel immer in `en.json` **und** `de.json`.
- Commits nur auf ausdrücklichen Wunsch.

## Status

Phasen 1–7 abgeschlossen (Bootstrap, Core-PKI, DB-Schicht, Commands, komplette UI). Phase 8 (Packaging/CI) weitgehend
fertig; 1.0.1 ist veröffentlicht (github.com/Khithar/certana, Release per Tag `v*`). Installationstest unter Mint bestanden, eigenes Icon
(`packaging/icon/`). Mehrbenutzerbetrieb auf Freigabe (Lock-Heartbeat, `lock_lost`) seit 2026-10-20, siehe
`docs/vault/03 - Datenmodell/Shared-SQLite-Betrieb.md`. Offen: Release-Signierung (GPG/Tauri-Secrets), Windows-Durchlauf, Praxistest auf echter Freigabe. Phasenplan und Status: `docs/vault/00 - Index.md`.
Wichtig: Schlüssel immer über `pki::keys::PrivateKey` erzeugen/laden (normalisiert auf PKCS#8 v1, sonst liest OpenSSL
Ed25519-Keys nicht).
