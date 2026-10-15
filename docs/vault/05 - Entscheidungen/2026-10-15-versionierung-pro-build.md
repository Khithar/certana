---
type: adr
status: accepted
tags: [type/adr, area/build]
---

# ADR: Versionsnummer wird pro Release-Build hochgezählt

## Kontext
Die Version steht an drei Stellen (`package.json`, `src-tauri/tauri.conf.json`, `[workspace.package]` in `src-tauri/Cargo.toml`) und bestimmt den Paketnamen des .deb. Wird sie nicht erhöht, erkennt `apt` beim Nachinstallieren kein Update, und zwei Builds sind nicht unterscheidbar.

## Entscheidung
`scripts/bump-version.mjs` erhöht die Version (Default Patch, optional `minor`/`major`) in allen drei Dateien und zieht `Cargo.lock` mit `cargo update --workspace --offline` nach. `npm run release` bzw. `npm run release:deb` führen Bump und `tauri build` in einem Schritt aus. `npm run tauri build` bleibt unverändert verfügbar, falls ein Build ohne Bump gewünscht ist.

Nicht gewählt: Bump in `beforeBuildCommand`, da die Tauri-CLI die Konfiguration vor diesem Hook liest und die alte Version im Bundle landen würde.

## Konsequenzen
- Jeder Release-Build erzeugt eine neue Patch-Version; Minor/Major werden bewusst per Argument gesetzt.
- Die Versionsänderung ist ein normaler Arbeitsbaum-Diff und wird mit dem zugehörigen Stand committet.
- Der GitHub-Release-Workflow (Tag `v*`) bleibt unberührt und nutzt die Version aus `tauri.conf.json`.
