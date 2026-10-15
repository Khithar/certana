---
type: adr
status: accepted
tags: [type/adr]
---

# ADR: Rust + Tauri 2 + React

## Kontext
Alternativen: Rust + Slint (nativ gerendert, mehr UI-Aufwand), .NET + Avalonia (40–70 MB, dotnet nicht installiert), Python + Qt (kein echtes Binary), Go + Wails (Go nicht installiert, Fyne-Optik). Verifizierter Stand der Crates und von Tauri in [[01 - Architektur/Krypto-Crates]] und [[01 - Architektur/Build und Packaging]].

## Entscheidung
- Backend Rust (`certana-core` ohne UI-Abhängigkeit), Desktop-Shell Tauri 2.12, Frontend React 19 + Vite + Tailwind 4 + shadcn/ui (derselbe Frontend-Stack wie in den anderen Projekten des Autors).
- Pure-Rust-Krypto: rcgen 0.14, rsa 0.9, pkcs8 0.11, p12-keystore, x509-parser; kein OpenSSL, kein `aws_lc_rs`.
- SQLite via rusqlite `bundled`; kein `tauri-plugin-sql`, kein `tauri-plugin-fs`.

## Konsequenzen
- Binary 5–15 MB, `.deb` 3–10 MB, `tauri build` liefert deb/AppImage/rpm/NSIS.
- UI läuft in WebKitGTK; Mint bringt `libwebkit2gtk-4.1-0` mit.
- Private Schlüssel bleiben im Rust-Prozess, das Webview sieht nur Metadaten.
