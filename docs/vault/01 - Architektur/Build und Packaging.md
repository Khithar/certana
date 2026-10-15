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
npm run tauri build              # .deb, AppImage, .rpm unter src-tauri/target/release/bundle/
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
