# Changelog

All notable changes to Certana are documented here. Format follows Keep a Changelog; versions follow SemVer.

## [Unreleased]

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
