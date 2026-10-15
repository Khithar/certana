---
type: moc
status: active
tags: [type/moc, status/active]
---

# Certana – Vault-Index

Certana ist eine native Linux-Desktop-Anwendung (Rust + Tauri 2) zum Betrieb einer **internen Certificate Authority**: Root-CA offline, Intermediate-CA für den Tagesbetrieb, Wizard für Server-, Client-, S/MIME- und Code-Signing-Zertifikate, Revocation mit CRL, Export und Ablaufüberwachung. Open Source unter GPL-3.0-or-later, verteilt als `.deb`, AppImage und Flatpak.

**Repo:** `/home/palu/Documents/intellij/certana` · **Stand:** siehe [[99 - Sessions/2026-10-15 Bootstrap]]

**Konventionen dieses Vaults:** [[Konventionen]] · **Begriffe:** [[Glossar]]

## Bereiche

- [[01 - Architektur/Überblick]] – Schichten, Crates, Sicherheitsmodell, Build & Packaging.
- [[02 - PKI-Konzept/Root-CA Offline]] – Zeremonie, Bundle, Import; weiter zu [[02 - PKI-Konzept/Intermediate-CA]], [[02 - PKI-Konzept/Zertifikatsprofile]], [[02 - PKI-Konzept/Revocation und CRL]], [[02 - PKI-Konzept/Schlüsselformate]], [[02 - PKI-Konzept/Vertrauensverteilung]], [[02 - PKI-Konzept/Bedrohungsmodell]].
- [[03 - Datenmodell/Tabellen]] – SQLite-Schema; [[03 - Datenmodell/Shared-SQLite-Betrieb]].
- [[04 - UI/Screens]] – Navigation, Wizard-Flows, i18n.
- [[05 - Entscheidungen/ADR-Index]] – Architecture Decision Records.
- [[06 - Betrieb/Installation]] · [[06 - Betrieb/Release-Prozess]].
- [[99 - Sessions/2026-10-15 Bootstrap]] – Arbeitsjournal.
- [[99 - Sessions/2026-10-15 UI-Fix Select Dark-Mode]] – Select-Popups im Dark-Mode lesbar.

## Status-Übersicht

| Phase | Inhalt | Status |
|---|---|---|
| 1 | Bootstrap: Toolchain, Scaffold, Vault, CLAUDE.md | fertig |
| 2 | Core-PKI in `certana-core` (17 Tests inkl. OpenSSL-Interop) | fertig |
| 3 | DB-Schicht (Migrationen, Repositories, Audit, Shared-Mode, Backup) | fertig |
| 4 | Tauri-Commands + UI-Gerüst | fertig |
| 5 | CA-Flows (Root-Zeremonie, Import, Intermediate, Unlock/Auto-Lock) | fertig |
| 6 | Zertifikats-Wizard, Verwaltung, Export, Renew | fertig |
| 7 | Revocation, CRL, Dashboard, Benachrichtigung | fertig |
| 8 | Packaging/CI (deb/AppImage/rpm gebaut, GitHub Actions, Flatpak-Manifest) | fertig bis auf Installationstest; manueller UI-Durchlauf laufend (Select-Dark-Mode-Fix 2026-10-15) |
| 9 | Doku-Abschluss, Release 1.0.0 (Passphrase-Wechsel, Theme, About, CRL optional umgesetzt; offen: Installationstest, Icon, Release-Secrets) | laufend |
