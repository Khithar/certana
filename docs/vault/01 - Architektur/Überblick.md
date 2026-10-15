---
type: concept
status: active
tags: [type/concept, area/architektur]
---

# Architektur-Überblick

Zurück zum [[00 - Index]]. Siehe auch [[01 - Architektur/Krypto-Crates]], [[01 - Architektur/Sicherheitsmodell]], [[01 - Architektur/Build und Packaging]].

## Stack

| Schicht | Technologie | Begründung |
|---|---|---|
| Backend / Domäne | Rust 1.90+, Crate `certana-core` | Echtes Binary, Memory-Safety, pure-Rust-Krypto ([[05 - Entscheidungen/2026-10-15-stack-rust-tauri]]) |
| Desktop-Shell | Tauri 2.12 | Erzeugt `.deb`/AppImage/`.rpm`/NSIS; Webview vom System, Binary 5–15 MB |
| Frontend | React 19, Vite, TypeScript, Tailwind 4, shadcn/ui, TanStack Table, react-i18next | Gleicher Frontend-Stack wie die übrigen Projekte des Autors |
| Persistenz | SQLite via rusqlite (`bundled`), Migrationen via rusqlite_migration | Eine Datei, teilbar mit Regeln ([[03 - Datenmodell/Shared-SQLite-Betrieb]]) |

## Schichten

```
React-UI  ──invoke()──▶  Tauri-Commands (src-tauri/src/commands/*)  ──▶  certana-core
                           dünne Adapter, DTOs, Fehler-Mapping           pki/ vault/ db/
```

- `certana-core` kennt weder Tauri noch UI und ist vollständig ohne Webview testbar.
- Tauri-Commands sind reine Adapter. Fehler werden als `CertanaError` typisiert und ohne Schlüsselmaterial an das Frontend gemeldet.
- **Private Schlüssel verlassen das Rust-Backend nie Richtung Webview.** Exporte schreibt das Backend direkt in Dateien, deren Pfad der Nutzer über den Dialog wählt.
- Der entsperrte Intermediate-Schlüssel lebt nur im `AppState` (Mutex) und wird nach Idle-Timeout mit `zeroize` gelöscht.

## Tauri-Commands (`src-tauri/src/commands/`)

| Modul | Commands |
|---|---|
| `db` | `db_default_path`, `db_open`, `db_info`, `db_close`, `db_backup` |
| `settings` | `settings_all`, `settings_set` |
| `ca` | `ca_list`, `ca_info`, `ca_create_root` (Zeremonie → Bundle), `ca_import_root`, `ca_open_root_bundle`, `ca_create_intermediate` (Zeremonie), `ca_create_intermediate_csr` / `ca_import_intermediate_cert` (externe Root), `ca_unlock`, `ca_lock`, `ca_session`, `ca_export_cert`, `ca_update_urls` |
| `cert` | `cert_profiles`, `cert_list`, `cert_get`, `cert_parse_csr`, `cert_issue`, `cert_export`, `cert_revoke`, `cert_renew`, `cert_update_notes` |
| `crl` | `crl_publish` (Session-Key), `crl_publish_root` (Bundle), `crl_list`, `crl_export` |
| `dashboard`, `audit`, `files` | `dashboard_summary`, `audit_recent`, `audit_export_csv`, `read_text_file` (CSR-Dateien, max. 1 MiB) |

Alle Commands liefern `Result<T, CommandError { code, message }>`; `code` ist stabil (`no-database`, `locked`, `wrong-passphrase`, `vault`, `invalid`, `pki`, `db`, `io`) und wird im Frontend übersetzt. Rechenintensive Commands (Keygen, Bundle, Export) laufen `async` in `spawn_blocking`.

**Session-Modell:** `state::Inner` hält `Mutex<Option<Database>>` und `Mutex<Option<Session>>`. Eine Session ist die entsperrte ausstellende CA: Schlüssel + Passphrase im Speicher, `last_used` für den Auto-Lock (Setting `security.auto_lock_minutes`, Default 15). Leaf-Schlüssel werden mit derselben Passphrase verschlüsselt, d. h. «CA entsperrt» = «ihre Zertifikatsschlüssel exportierbar».

End-to-End-Test der Command-Schicht auf dem Tauri-Mock-Runtime: `src-tauri/tests/commands_flow.rs`.

## Repo-Layout

```
certana/
├── CLAUDE.md, LICENSE, README.md, CHANGELOG.md
├── docs/vault/                 dieser Vault
├── src/                        React-Frontend (pages/, components/, i18n/, api/)
├── src-tauri/
│   ├── Cargo.toml              Workspace + App-Crate
│   ├── crates/certana-core/    Domäne (pki/, vault/, db/, config.rs, error.rs)
│   ├── src/                    main.rs, lib.rs, commands/, state.rs
│   ├── capabilities/           Tauri-Permissions
│   └── tauri.conf.json
├── packaging/                  certana.desktop, Flatpak-Manifest
└── .github/workflows/          ci.yml, release.yml
```
