---
type: reference
status: active
tags: [type/reference, area/betrieb]
---

# Installation

Zurück zum [[00 - Index]]. Build: [[01 - Architektur/Build und Packaging]].

## Linux Mint / Ubuntu / Debian (.deb)

```sh
sudo apt install ./certana_<version>_amd64.deb
```
Startmenü: «Zubehör → Certana» (Kategorien Utility/Security/Network). Deinstallation `sudo apt remove certana`.

## AppImage

```sh
chmod +x Certana_<version>_amd64.AppImage && ./Certana_<version>_amd64.AppImage
```

## Flatpak

`flatpak install flathub io.github.certana.Certana` (sobald auf Flathub veröffentlicht).

## Windows (optional)

`Certana_<version>_x64-setup.exe` (NSIS). WebView2 wird bei Bedarf nachgeladen. Ohne Code-Signing erscheint ein SmartScreen-Hinweis.

## Lokaler Build

`npm run tauri build` → `src-tauri/target/release/bundle/{deb,rpm,appimage}/`. Installationstest: `sudo apt install ./src-tauri/target/release/bundle/deb/Certana_0.1.0_amd64.deb`, Eintrag im Mint-Menü unter «Zubehör», Deinstallation `sudo apt remove certana`.

## Daten

- DB: `~/.local/share/certana/certana.db` (änderbar in den Einstellungen).
- Root-Bundle: frei gewählter Ort, empfohlen USB-Stick; **nicht** im DB-Ordner.
