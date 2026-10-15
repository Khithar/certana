---
type: session
status: active
tags: [type/session]
---

# Session 2026-10-15 – Bootstrap

Zurück zum [[00 - Index]].

## Getan
- Anforderungen geklärt und Plan freigegeben (`~/.claude/plans/pasted-content-id-9c43-wir-erstellen-glowing-hinton.md`).
- Stack-Entscheidung Rust + Tauri 2 + React ([[05 - Entscheidungen/2026-10-15-stack-rust-tauri]]), Java verworfen, GPL-3.0, v1-Umfang.
- Crate- und Tauri-Stand recherchiert und verifiziert ([[01 - Architektur/Krypto-Crates]], [[01 - Architektur/Build und Packaging]]).
- rustup (Rust 1.99) installiert, Tauri-Scaffold (React/TS) erzeugt, Cargo-Workspace mit `certana-core` angelegt; `cargo check` grün.
- Frontend-Abhängigkeiten installiert (react-router 7, i18next, TanStack Table, Tailwind 4, shadcn-Helfer, Tauri-Plugins dialog/notification/clipboard, vitest, eslint).
- Vault angelegt (dieser Stand), CLAUDE.md, README, Tauri-Konfiguration, Desktop-Template, Capabilities.

- Systempakete installiert (durch den Nutzer), `cargo check -p certana` grün.

## Phase 2 – Core-PKI (gleiche Session)
- `pki/keys.rs`: `KeyAlgorithm` (ECDSA P-256/P-384, Ed25519, RSA 2048/3072/4096), `PrivateKey` als zeroized PKCS#8 v1, redigiertes `Debug`.
- `pki/profiles.rs`: `Profile` (Server, Client, Smime, CodeSigning) mit KU/EKU/Laufzeiten/SAN-Regeln, `Subject`, `San`.
- `pki/ca.rs`: `create_root`, `create_intermediate`, `intermediate_csr`, 128-Bit-Zufallsserial, CRL-DP + AIA (AIA als DER via x509-cert).
- `pki/issue.rs`: `issue_with_new_key`, `issue_from_csr` (nur Public Key aus der CSR).
- `pki/csr.rs`, `pki/crl.rs` (RFC-5280-Gründe), `pki/export.rs` (Chain-Dateien, PKCS#12 modern/legacy, Server-Snippets), `pki/parse.rs` (`CertificateInfo` + Textdump, `verify_issued_by`).
- `vault/keywrap.rs` (PBES2 scrypt + AES-256-CBC, Passphrase-Policy ≥ 16), `vault/rootbundle.rs` (age + JSON).
- Tests: 15 Unit-Tests + 2 OpenSSL-Interop-Tests (`openssl verify`, `x509 -text`, `pkey`, `crl -text`, `verify -crl_check`, `pkcs12 -info`, `req -verify`, CSR von OpenSSL mit CA:TRUE wird korrekt ignoriert).
- Befund: Ed25519-PKCS#8 v2 von ring ist für OpenSSL 3.0 unlesbar → Normalisierung auf v1, siehe [[02 - PKI-Konzept/Schlüsselformate]].

## Phase 3 – DB-Schicht (gleiche Session)
- `db/sql/0001_init.sql` (settings, cas, certificates, revocations, crls, audit_log), `db/mod.rs` (`Database::open` mit Local/Shared-Modus, Pragmas, Migrationen, Backup, In-Memory für Tests).
- `db/fs.rs` Netzfreigabe-Erkennung, `db/lock.rs` JSON-Lock-Datei mit Stale-Übernahme, `db/models.rs` DTOs ohne Schlüsselmaterial, `db/settings.rs`, `db/audit.rs` (+ CSV), `db/repo_ca.rs`, `db/repo_cert.rs` (Filter, Revoke in Transaktion, `refresh_expired`), `db/repo_crl.rs`, `config.rs` (Datenverzeichnis, `CERTANA_DATA_DIR`).
- Tests: 7 Unit-Tests (Migrationen, WAL vs. DELETE, Lock/Read-only, Stale-Lock, Backup, Mount-Erkennung) + `tests/db_flow.rs` End-to-End. Gesamt 26 Tests grün.

## Phasen 4–7 – Commands und UI (gleiche Session)
- Backend: `state.rs` (Session mit Auto-Lock), `commands/{error,files,db,settings,ca,cert,crl,dashboard,audit}.rs`, Registrierung über `all_commands!`. Siehe [[01 - Architektur/Überblick#Tauri-Commands]].
- Frontend: API-Schicht, Komponenten, Kontext, alle Screens, Wizards und Dialoge, i18n en/de vollständig. Siehe [[04 - UI/Screens]].
- Tests: `src-tauri/tests/commands_flow.rs` (Mock-App: DB → Root-Zeremonie → Intermediate → Unlock → Ausstellen inkl. OpenSSL-CSR → Export mit 0600 + `openssl verify` → Revoke → CRL → Root-CRL → Renew → Dashboard → Backup). Frontend: tsc, eslint, vite build grün.
- ESLint: `react-hooks/set-state-in-effect` bewusst deaktiviert (Draft-Formulare synchronisieren Props per Effect).

## Phase 8 – Packaging/CI (gleiche Session)
- `.github/workflows/ci.yml`, `release.yml` (tauri-action@v1, SHA256SUMS + GPG), `src-tauri/deny.toml`, Flatpak-Manifest + AppStream-Metainfo.
- `npm run tauri build` lokal: Binary 9,9 MB, **.deb 4,3 MiB** (Depends `libwebkit2gtk-4.1-0, libgtk-3-0`, Desktop-Eintrag mit `Categories=Utility;Security;Network;`), .rpm 4,3 MiB, AppImage 82 MiB. Release-Binary startet (Smoke-Test 8 s).
- Gesamtstand Tests: certana-core 22 Unit + 2 DB-Flow + 2 OpenSSL-Interop, App 1 Command-Flow, Frontend 6 Vitest; clippy `-D warnings` und `cargo fmt --check` sauber.

## Offen
- .deb unter Mint installieren und Startmenü prüfen (sudo, durch den Nutzer).
- Manueller UI-Durchlauf in `npm run tauri dev`.
- Icons sind noch die Tauri-Platzhalter; eigenes Icon-Set via `npm run tauri icon <png>`.
- GitHub-Repo anlegen, `repository`/`homepage` in Cargo.toml, README, metainfo anpassen.
- GitHub-Repo-URL (`repository` in Cargo.toml/README ist Platzhalter `certana/certana`).
