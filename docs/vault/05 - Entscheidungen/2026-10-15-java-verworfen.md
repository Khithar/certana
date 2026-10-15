---
type: adr
status: accepted
tags: [type/adr]
---

# ADR: Java/JavaFX verworfen

## Kontext
Die Umgebung des Autors ist Java-geprägt (Maven, IntelliJ, Liberica JDK mit JavaFX, bestehende JavaFX-App). Java + BouncyCastle wäre fachlich stark gewesen.

## Entscheidung
Kein Java. Ein GraalVM-native-image einer JavaFX-App ist zu gross; jpackage liefert kein echtes Binary, sondern ein gebündeltes Runtime. Gefordert ist ein kompaktes natives Binary, Linux-Fokus, Windows optional.

## Konsequenzen
Rust-Ökosystem für Krypto (siehe [[05 - Entscheidungen/2026-10-15-stack-rust-tauri]]); RSA-Keygen über separate Crate; AIA-Extension manuell.
