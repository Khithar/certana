---
type: concept
status: active
tags: [type/concept, area/pki]
---

# Intermediate-CA (Issuing-CA)

Zurück zu [[02 - PKI-Konzept/Root-CA Offline]]. Profile: [[02 - PKI-Konzept/Zertifikatsprofile]].

| Eigenschaft | Wert |
|---|---|
| Schlüssel | ECDSA P-384 (Default) oder RSA 4096 |
| Laufzeit | 5 Jahre (Default), max. Root-Restlaufzeit |
| basicConstraints | `CA:TRUE, pathlen:0` (kritisch) |
| keyUsage | `keyCertSign, cRLSign` (kritisch) |
| CRL-DP / AIA | aus Settings, z. B. `http://pki.example.internal/issuing.crl`, `…/issuing.crt` |
| Schlüsselablage | `cas.encrypted_key_pkcs8` (PBES2), Sitzungs-Passphrase, Auto-Lock |

## Ablauf «Intermediate erzeugen»

1. Name, Parameter, Passphrase für die Intermediate (unabhängig von der Root-Passphrase).
2. Certana erzeugt Schlüssel + CSR.
3. Signieren: (a) sofort per [[02 - PKI-Konzept/Root-CA Offline#Zeremonie-Dialog (Signieren, Root-CRL)|Root-Zeremonie]] oder (b) CSR exportieren, extern signieren, Zertifikat importieren.
4. Intermediate wird `cas.kind = intermediate`, `parent_ca_id` = Root.

## Betrieb

- Mehrere Intermediates möglich (z. B. «Server-CA», «User-CA»), jede mit eigener CRL-Nummer und Passphrase.
- Entsperren pro Sitzung; optional Passphrase im OS-Keyring (Default aus).
- Erneuerung vor Ablauf: neue Intermediate ausstellen, alte bleibt für CRL-Signierung bis zum letzten Leaf-Ablauf erhalten.
