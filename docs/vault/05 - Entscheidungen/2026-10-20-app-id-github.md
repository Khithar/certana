---
type: adr
status: accepted
tags: [type/adr, area/build]
---

# ADR: App-ID `io.github.khithar.certana`

## Kontext
Die Tauri-`identifier` bestimmt den Webview-Profilordner (`~/.local/share/<id>/`), die Flatpak-App-ID und die AppStream-ID. Bisher stand dort `io.github.certana`, ein Platzhalter, der für Flathub falsch gebildet war (es muss `io.github.<user>.<repo>` sein). Zur Wahl stand die Domain der Firma des Entwicklers (`com.<firma>.certana`) oder der GitHub-Namensraum.

## Entscheidung
`io.github.khithar.certana` (Tauri) bzw. `io.github.khithar.Certana` (Flatpak/AppStream, letztes Glied gross). Begründung: Certana ist ein persönliches GPL-Projekt (Repo unter dem persönlichen GitHub-Account, Autoren «Certana Contributors»), die ID soll das Projekt und nicht die Firma abbilden und Firmenänderungen (Verkauf, Umfirmierung, Domainwechsel) überdauern. Flathub-Verifikation läuft über das Repo, ohne Domain-Nachweis. Umgestellt vor dem Release 1.0.2, damit kein Nutzer ausser dem Entwickler den Profilordner-Wechsel erlebt.

Nicht gewählt: `com.<firma>.certana`. Passend nur, wenn Certana ein Firmenprodukt mit der Firma als Copyright-Inhaberin würde; dann müssten auch Repo, Autorenzeile und README zur Firma gehören.

## Konsequenzen
- Webview-Ordner wandert; verloren gehen nur gemerkter DB-Pfad und Farbschema, keine PKI-Daten.
- Paketname `certana`, Startmenü, Deinstallation und `~/.local/share/certana` bleiben unverändert.
- Flatpak-Manifest und Metainfo tragen die ID im Dateinamen: `packaging/flatpak/io.github.khithar.Certana.yml`, `packaging/io.github.khithar.Certana.metainfo.xml`.
