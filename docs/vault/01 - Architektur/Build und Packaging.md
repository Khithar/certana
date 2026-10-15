---
type: reference
status: active
tags: [type/reference, area/build]
---

# Build und Packaging

Zurück zu [[01 - Architektur/Überblick]]. Betrieb: [[06 - Betrieb/Installation]], [[06 - Betrieb/Release-Prozess]].

## Voraussetzungen (Linux Mint 22 / Ubuntu 24.04)

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust ≥ 1.90 (apt-rustc ist zu alt)
```
Node 22 für das Frontend.

## Befehle

```sh
npm install                      # Frontend-Abhängigkeiten
npm run tauri dev                # Entwicklung mit Hot-Reload
cargo test -p certana-core --manifest-path src-tauri/Cargo.toml
npm run tauri build              # .deb, AppImage, .rpm unter src-tauri/target/release/bundle/ (ohne Versions-Bump)
npm run release                  # Version hochzählen (Patch) + tauri build
npm run release:deb              # Version hochzählen + nur .deb bauen
npm run bump -- minor            # nur Version erhöhen: patch | minor | major
```

## Bundle-Konfiguration (`src-tauri/tauri.conf.json`)

- `productName: "Certana"`, `identifier: io.github.certana`, `bundle.category: "Utility"`.
- `.deb`: `section: utils`, eigenes `desktopTemplate` (`packaging/certana.desktop`) mit `Categories=Utility;Security;Network;` → Mint-Startmenü unter «Zubehör» und «Systemverwaltung».
- Release-Profil: `lto`, `codegen-units=1`, `opt-level="s"`, `strip` → Binary 5–15 MB, `.deb` 3–10 MB.
- AppImage bündelt GTK/WebKit (70–120 MB); auf `ubuntu-22.04` bauen (glibc).

## Kanäle

1. **GitHub Releases**: `.deb`, AppImage, `.rpm`, optional Windows-NSIS, `SHA256SUMS` + GPG-Signatur.
2. **Flathub**: Manifest unter `packaging/flatpak/`, `org.gnome.Platform` ≥ 49.
3. Kein Snap (Mint blockiert snapd). Optional später eigenes APT-Repo.

## Icon
Quelle `packaging/icon/certana.svg` (Urkunde mit Wachssiegel, Häkchen und Bändern; Farben Indigo/Creme/Bernstein/Türkis). Rendern ohne rsvg: Vordergrund via ImageMagick-MSVG, Verlauf separat (`gradient:` + Rundeck-Maske), komponiert zu `packaging/icon/certana-1024.png`; danach `npm run tauri icon packaging/icon/certana-1024.png` erzeugt `src-tauri/icons/` (PNG, ICNS, ICO, Store-Logos).

## Demo-Daten und Screenshots
`CERTANA_DEMO_DIR=/pfad cargo test --manifest-path src-tauri/Cargo.toml --test demo_db` baut eine Demo-Datenbank (Passphrasen `demorootpassphrase2026` / `demoissuingpassphrase2026`). App dazu mit `HOME=<leer> CERTANA_DATA_DIR=/pfad target/release/certana` starten, damit die echte DB und der WebKit-Speicher unberührt bleiben. Screenshots liegen in `docs/screenshots/`. Achtung: Release-Binaries nur über `npm run tauri build -- --no-bundle` bauen, `cargo build --release` allein lädt die Dev-URL.

## Versionierung
Die Version steht in `package.json`, `src-tauri/tauri.conf.json` und `[workspace.package]` in `src-tauri/Cargo.toml`. `scripts/bump-version.mjs` hält alle drei synchron und zieht `Cargo.lock` nach. Entscheidung: [[05 - Entscheidungen/2026-10-15-versionierung-pro-build]].
