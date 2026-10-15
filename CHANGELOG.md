# Changelog

All notable changes to Certana are documented here. Format follows Keep a Changelog; versions follow SemVer.

## [Unreleased]

## [1.0.0] - 2026-10-15

First release: internal certificate authority for Linux desktops (.deb, AppImage, .rpm).

### Added
- Project bootstrap: Tauri 2 + React scaffold, `certana-core` crate skeleton, Obsidian vault, GPL-3.0 license.
- Core PKI (`certana-core`): key generation (ECDSA, Ed25519, RSA), root/intermediate CA creation, issuance profiles
  (server, client, S/MIME, code signing), CSR handling, CRL building, PEM/DER/PKCS#12 export, certificate parsing,
  PBES2 key encryption and the encrypted offline root bundle. Verified against the OpenSSL CLI.
- Persistence (`certana-core::db`): SQLite schema with migrations, CA/certificate/CRL repositories, append-only audit
  log with CSV export, network-share detection with exclusive locking and a writer lock file, online backup.
- Desktop app: Tauri commands with an auto-locking issuing-CA session; React UI with dashboard, CA management
  (root ceremony, root import, issuing CA via ceremony/CSR/import, trust-distribution guide), certificate wizard
  (server/client/S-MIME/code-signing, generated key or CSR), export packages with nginx/Apache snippets, revocation,
  CRL publication, renewal, settings and audit export. English and German UI.
- Packaging: CI/release workflows, cargo-deny policy, Flatpak manifest.
- Change the passphrase of an issuing CA (`ca_change_passphrase`): CA key and all stored certificate keys are
  re-encrypted in one transaction, audited as `ca.change_passphrase`.
- Theme switch (follow system / light / dark) and an «About Certana» card with version, licence and repository
  link in Settings.
- Release helper `npm run release[:deb]` that bumps the version in all manifests before building.

### Changed
- Running without a CRL distribution point is now a first-class choice: the CRL URL is documented as optional,
  CA pages explain the consequence, and the dashboard shows «not used» instead of a missing-CRL badge.
- Clearer wording around passphrases: the unlock dialog and the certificate wizard state that the passphrase
  belongs to the issuing CA and that exported keys are unencrypted PKCS#8.

### Fixed
- Native select popups were unreadable in dark mode (white text on white) because no `color-scheme` was set.
