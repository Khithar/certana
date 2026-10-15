---
type: reference
status: active
tags: [type/reference, area/betrieb]
---

# Release-Prozess

Zurück zum [[00 - Index]].

1. Version in `package.json`, `src-tauri/Cargo.toml` (Workspace) und `src-tauri/tauri.conf.json` anheben, `CHANGELOG.md` ergänzen.
2. Tag `v<version>` pushen → `release.yml`: Matrix `ubuntu-22.04` + `windows-latest`, `tauri-apps/tauri-action@v1`, Draft-Release mit deb/AppImage/rpm/NSIS.
3. Folge-Job: `SHA256SUMS` erzeugen, GPG-signieren (`--detach-sign --armor`), hochladen. Release-Key-Fingerprint steht im README.
4. Draft prüfen, `.deb` auf Mint testen (Installation, Startmenü, Deinstallation), Release veröffentlichen.
5. Flatpak-Manifest aktualisieren und Flathub-PR stellen.

Workflows: `.github/workflows/ci.yml` (fmt, clippy, test, cargo-deny, Frontend lint/typecheck/build) und `release.yml`. Lizenz-Policy in `src-tauri/deny.toml`. Flatpak: `packaging/flatpak/io.github.certana.Certana.yml` + `packaging/io.github.certana.Certana.metainfo.xml`.

Secrets in GitHub: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (Updater-Artefakte), `GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`.
