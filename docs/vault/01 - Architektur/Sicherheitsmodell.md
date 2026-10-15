---
type: concept
status: active
tags: [type/concept, area/architektur, area/sicherheit]
---

# Sicherheitsmodell

Zurück zu [[01 - Architektur/Überblick]]. Bedrohungen: [[02 - PKI-Konzept/Bedrohungsmodell]].

## Schlüssel im Ruhezustand

| Schlüssel | Ort | Schutz |
|---|---|---|
| Root-CA | `*.certana-root`-Bundle, **nie in der DB** | PKCS#8-PBES2 (scrypt + AES-256-CBC), zusätzlich age-Container; Passphrase ≥ 16 Zeichen |
| Intermediate-CA | Spalte `cas.encrypted_key_pkcs8` | PKCS#8-PBES2, Sitzungs-Passphrase, Auto-Lock |
| Leaf (von Certana erzeugt) | `certificates.encrypted_key_pkcs8` | PKCS#8-PBES2 mit derselben Sitzungs-Passphrase; beim Export entschlüsselt in Datei |
| Leaf (CSR-Import) | nicht vorhanden | Schlüssel bleibt beim Antragsteller |

## Schlüssel im Speicher

- Alle Container `Zeroizing<…>`; Passphrasen `SecretString`.
- Entsperrter Intermediate-Key im `AppState` hinter Mutex, Idle-Timeout (Default 15 min), manuelles Sperren.
- Root-Key existiert nur für die Dauer einer Zeremonie-Operation.

## Tauri-Grenze

- Capabilities minimal: `core:default`, `dialog:allow-open/save`, `notification:default`, `clipboard-manager:allow-write-text`, `opener:allow-open-url`.
- Kein `fs`-Plugin, kein `sql`-Plugin, strikte CSP ohne Remote-Quellen.
- Dateien werden nur über eigene Commands geschrieben (`commands/files.rs`): private Dateien mit 0600, `read_text_file` auf 1 MiB begrenzt.
- Das Frontend erhält nur Metadaten (Subject, SANs, Laufzeiten, Fingerprints, PEM von **Zertifikaten**), nie private Schlüssel.

## Logging und Audit

- `tracing` ohne Secrets: Key-Typen implementieren `Debug` nur redigiert.
- Audit-Tabelle append-only mit OS-User, Hostname, Aktion, Ziel, Zeitstempel.
