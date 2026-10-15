---
type: concept
status: active
tags: [type/concept, area/ui]
---

# Screens und Flüsse

Zurück zum [[00 - Index]]. Stack: React 19, Tailwind 4, eigene shadcn-artige Komponenten (`src/components/ui`), react-router 7 (HashRouter), react-i18next (en Default, de). Dark/Light folgt dem System; `:root`/`.dark` setzen `color-scheme`, damit native Form-Controls (Select-Popups in WebKitGTK) zum Theme passen, und `select, option` haben explizit Theme-Farben.

## Umsetzung (Stand 2026-10-15)

| Datei | Inhalt |
|---|---|
| `src/context/AppContext.tsx` | Globaler Zustand: `db`, `session`, `cas`, `settings`; `refresh()`, `openDb()`, `useCommand()` für Fehlerbehandlung; öffnet beim Start die zuletzt genutzte DB (localStorage). |
| `src/components/Layout.tsx` | Sidebar mit Navigation, DB-Status (Pfad, Modus, read-only), Session-Status mit Sperren-Button. |
| `src/components/UnlockDialog.tsx` | Passphrase-Dialog; `useEnsureUnlocked()` fragt vor Aktionen automatisch nach. |
| `src/pages/Dashboard.tsx` | Kennzahlen, CA-Tabelle mit CRL-Status, Ablaufliste 30 Tage, letzte Audit-Einträge, Desktop-Benachrichtigung beim Start. |
| `src/pages/Cas.tsx` + `CaDialogs.tsx` | CA-Liste, Detail (Textdump, URLs, CRL), Root-Zeremonie-Wizard (5 Schritte), Root-Import (PEM/PKCS#12/nur Zertifikat), ausstellende CA (Zeremonie / CSR / Import), Root-CRL-Zeremonie, Vertrauensverteilungs-Anleitungen. |
| `src/pages/Certificates.tsx` + `IssueWizard.tsx` | Tabelle mit Suche/Filter, Wizard Profil → Identität → Schlüssel (generieren/CSR) → Prüfen → Ausstellen. |
| `src/pages/CertificateDetail.tsx` | Details, Notizen, Textdump, Export-Dialog (Key/DER/PKCS#12, Snippets nginx/Apache), Widerruf, Erneuerung. |
| `src/pages/Revocations.tsx` | Widerrufe pro CA, CRL-Verlauf, veröffentlichen/exportieren. |
| `src/pages/Settings.tsx` | DB öffnen/anlegen/wechseln/Backup, Darstellung (Farbschema System/Hell/Dunkel, `localStorage`, `src/lib/theme.ts`), Subject-Vorgaben, Export-/CRL-Verzeichnis, Auto-Lock, Ablaufwarnung, Sprache, Audit-CSV, «Über Certana» (Version, Lizenz, Repo-Link via opener-Plugin). |
| `src/api/` | `types.ts` (DTO-Spiegel), `index.ts` (invoke-Wrapper), `dialogs.ts` (Dialog/Clipboard/Notification-Plugins). |

## Navigation (Sidebar)

1. **Dashboard** – CA-Status, ablaufende Zertifikate (30/14/7 Tage), CRL-Status, letzte Audit-Einträge, Schnellaktionen.
2. **CAs** – Root/Intermediates; Root erzeugen (Zeremonie), Root importieren, Intermediate erzeugen/signieren, Root exportieren, CRL erneuern, entsperren/sperren.
3. **Zertifikate** – Tabelle mit Suche/Filter (Profil, Status, Ablauf), Detail (Textdump, Extensions, Kette), Aktionen: exportieren, erneuern, widerrufen, Notizen.
4. **Widerrufe / CRL** – Revocation-Liste, CRL-Historie, Export-Pfad.
5. **Einstellungen** – DB öffnen/anlegen, Farbschema, Export-Verzeichnis, CRL/AIA-URLs, Subject-Defaults (O, OU, C, ST, L), Auto-Lock, Sprache, Benachrichtigungen, Backup, Über Certana.

CA-Detail (`Cas.tsx`): ausstellende CAs mit gespeichertem Schlüssel haben «Passphrase ändern» (`ChangePassphraseDialog` in `CaDialogs.tsx`). Ohne CRL-URL zeigt die CRL-Karte einen Hinweis, dass Clients keinen Widerruf prüfen; das Dashboard zeigt dann «nicht verwendet» statt einer fehlenden CRL.

## Wizard «Neues Zertifikat»

1. Profil (Server, Client, S/MIME, Code-Signing) – siehe [[02 - PKI-Konzept/Zertifikatsprofile]].
2. Identität – Hostnames/IPs/E-Mail als Chips, Subject aus Defaults vorbelegt.
3. Schlüssel – «Certana erzeugt» (Algorithmus wählen) oder «CSR importieren» (Datei/Einfügen, Felder-Vorschau).
4. Laufzeit & Review – Zusammenfassung aller Extensions.
5. Ausstellen & Export – Zielordner, Dateiset, PKCS#12-Passwort, Snippets für nginx/Apache, Kopieren-Buttons.

## Root-Zeremonie-Wizard

Warnhinweis (offline arbeiten) → Parameter → Passphrase → Bundle-Speicherort → Zusammenfassung mit Fingerprint → Backup-Erinnerung.

## i18n

JSON-Bundles `src/i18n/en.json`, `src/i18n/de.json`; Schlüssel nach Screen gruppiert (`dashboard.*`, `wizard.*`). Systemsprache als Default, Override in Einstellungen.
