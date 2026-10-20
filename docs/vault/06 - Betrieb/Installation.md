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

`flatpak install flathub io.github.khithar.Certana` (sobald auf Flathub veröffentlicht).

## Windows (optional)

`Certana_<version>_x64-setup.exe` (NSIS). WebView2 wird bei Bedarf nachgeladen. Ohne Code-Signing erscheint ein SmartScreen-Hinweis.

## Lokaler Build

`npm run tauri build` → `src-tauri/target/release/bundle/{deb,rpm,appimage}/`. Installationstest: `sudo apt install ./src-tauri/target/release/bundle/deb/Certana_0.1.0_amd64.deb`, Eintrag im Mint-Menü unter «Zubehör», Deinstallation `sudo apt remove certana`.

## Daten

- DB: `~/.local/share/certana/certana.db` (änderbar in den Einstellungen; Default per `CERTANA_DATA_DIR` verschiebbar).
- Root-Bundle: frei gewählter Ort, empfohlen USB-Stick; **nicht** im DB-Ordner.
- Zuletzt geöffneter DB-Pfad: `localStorage` des Webviews unter `~/.local/share/io.github.khithar.certana/`.
- Neustart «from scratch»: App beenden, `~/.local/share/certana` und das Root-Bundle löschen (optional `~/.local/share/io.github.khithar.certana`).

## Mehrere Benutzer (Freigabe)

Datenbank auf eine SMB/NFS-Freigabe legen und auf jedem Arbeitsplatz unter Einstellungen → Datenbank dieselbe Datei öffnen. Certana erkennt das Netzlaufwerk und arbeitet im geteilten Modus (ein Schreiber, weitere lesend). Details und was alles auf der Freigabe liegt: [[03 - Datenmodell/Shared-SQLite-Betrieb]].

**Wichtig:** Die Freigabe muss als Kernel-Mount (cifs/nfs) eingebunden sein. Der Weg über den Dateimanager («Mit Server verbinden», Pfad `/run/user/…/gvfs/…`) funktioniert nicht, weil gvfs keine wahlfreien Schreibzugriffe unterstützt; Certana lehnt solche Pfade ab.

Beispiel SMB unter Linux Mint:

```sh
sudo apt install cifs-utils
sudo mkdir -p /mnt/pki
sudo install -m 600 /dev/null /etc/cifs-pki.cred   # Inhalt: username=…, password=…, domain=…
sudo mount -t cifs //nas02.example/technik /mnt/pki -o credentials=/etc/cifs-pki.cred,uid=$(id -u),gid=$(id -g),file_mode=0600,dir_mode=0700,vers=3.0,nobrl
```

Dauerhaft in `/etc/fstab` (uid/gid des angemeldeten Benutzers eintragen):

```
//nas02.example/technik  /mnt/pki  cifs  credentials=/etc/cifs-pki.cred,uid=1000,gid=1000,file_mode=0600,dir_mode=0700,vers=3.0,nobrl,_netdev,nofail  0  0
```

**`nobrl` ist Pflicht.** Ohne diese Option schickt der Linux-cifs-Client SQLites Dateisperren als zwingende Byte-Range-Locks an den Server; jeder Schreibzugriff endet nach 5 s mit «database is locked» (Befund 2026-10-20 auf nas02, bestätigt mit dem Demo-Flow: ohne `nobrl` Fehler, mit `nobrl` 49 s für Root, Issuing CA und neun Zertifikate). Certana prüft die Mount-Optionen beim Öffnen und meldet fehlendes `nobrl` sofort. Die Einzelschreiber-Garantie kommt aus Certanas Lock-Datei, nicht aus SQLite-Locks.

Danach in Certana `/mnt/pki/SSL_certificates/SiriusCA/certana.db` öffnen. Auf jedem Arbeitsplatz denselben Mountpunkt verwenden, weil Export- und CRL-Verzeichnis in der Datenbank gespeichert sind.
