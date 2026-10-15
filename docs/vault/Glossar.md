---
type: reference
status: active
tags: [type/reference]
---

# Glossar

| Begriff | Bedeutung |
|---|---|
| **Root-CA** | Oberste Zertifizierungsstelle; selbstsigniert, Schlüssel offline im [[02 - PKI-Konzept/Root-CA Offline|Root-Bundle]]. |
| **Intermediate-CA / Issuing-CA** | Von der Root signierte Zwischenstelle, stellt Endzertifikate aus. Schlüssel verschlüsselt in der DB. |
| **Leaf / Endzertifikat** | Zertifikat für Server, Client, Mailadresse oder Code-Signatur; darf nicht signieren (`CA:FALSE`). |
| **CSR** | Certificate Signing Request (PKCS#10): öffentlicher Schlüssel + gewünschte Namen, vom Antragsteller signiert. |
| **CRL** | Certificate Revocation List: signierte Liste widerrufener Seriennummern mit Gültigkeitsfenster. |
| **CRL-DP** | CRL Distribution Point: URL im Zertifikat, unter der Clients die CRL abholen. |
| **AIA** | Authority Information Access: URL zum Issuer-Zertifikat (und optional OCSP). |
| **SAN** | Subject Alternative Name: DNS-Namen, IPs, E-Mail-Adressen; für Server-Zertifikate Pflicht. |
| **KU / EKU** | Key Usage / Extended Key Usage: erlaubte Verwendungszwecke eines Schlüssels. |
| **pathlen** | Basic-Constraints-Beschränkung, wie viele CA-Ebenen unterhalb erlaubt sind. |
| **PKCS#8** | Standardformat für private Schlüssel; mit PBES2 verschlüsselbar. |
| **PBES2** | Passwortbasierte Verschlüsselung (RFC 8018): KDF (scrypt/PBKDF2) + Cipher (AES-256-CBC). |
| **PKCS#12 / .p12 / .pfx** | Container für Schlüssel + Zertifikatskette, passwortgeschützt; für Windows/Browser-Import. |
| **Root-Zeremonie** | Geführter Ablauf in Certana, bei dem das Root-Bundle kurz entsperrt wird (Intermediate signieren, Root-CRL). |
| **Root-Bundle** | Verschlüsselte Datei `*.certana-root` mit Root-Schlüssel, Root-Zertifikat und Metadaten. |

Zurück zum [[00 - Index]].
