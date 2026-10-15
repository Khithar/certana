---
type: concept
status: active
tags: [type/concept, area/pki, area/sicherheit]
---

# Bedrohungsmodell

Zurück zu [[01 - Architektur/Sicherheitsmodell]].

| Bedrohung | Gegenmassnahme |
|---|---|
| Diebstahl der DB-Datei (Freigabe, Backup) | Alle Schlüssel PBES2-verschlüsselt; Root-Schlüssel gar nicht enthalten |
| Diebstahl des Root-Bundles | Doppelte Verschlüsselung (age + PBES2), starke Passphrase, Bundle offline |
| Kompromittierter Arbeitsplatz während Sitzung | Auto-Lock, Root nur während Zeremonie im Speicher, zeroize |
| Manipulierte CSR (CA-Flag, fremde EKU) | CSR liefert nur Public Key/Subject/SAN; Profil bestimmt Extensions |
| Doppelte Seriennummern | 128 Bit Zufall + UNIQUE-Constraint |
| Zwei Schreiber auf Netzfreigabe | Exclusive-Locking + Lock-Datei, zweite Instanz read-only ([[03 - Datenmodell/Shared-SQLite-Betrieb]]) |
| Schlüssel im Log / in Fehlermeldungen | Redigiertes `Debug`, Fehlertexte ohne Material, kein Secret im Webview |
| Supply-Chain (Crates, npm) | Lockfiles, `cargo deny`, `cargo audit`, SBOM, signierte Releases |
| Vergessene Passphrase | Nicht wiederherstellbar – UI-Hinweis, Backup-Empfehlung; bewusste Entscheidung, kein Recovery-Hintertürchen |
