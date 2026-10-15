---
type: reference
status: active
tags: [type/reference, area/pki]
---

# Schlüssel- und Dateiformate

Zurück zum [[00 - Index]]. Crates: [[01 - Architektur/Krypto-Crates]].

| Artefakt | Format | Dateiname | Erzeugt mit |
|---|---|---|---|
| Privater Schlüssel (Export) | PKCS#8 PEM, unverschlüsselt (Server lesen keine Passphrase beim Start) oder PBES2 | `name.key.pem` | `pkcs8` |
| Schlüssel in DB / Bundle | PKCS#8 PBES2: scrypt (N=2^15, r=8, p=1) + AES-256-CBC | – | `pkcs8` (OpenSSL-3-kompatibel: `openssl pkey -in …`) |
| Zertifikat | PEM (`CERTIFICATE`) und DER | `name.crt.pem`, `name.der` | `rcgen` |
| Kette | PEM: Intermediate(s) | `chain.pem` | – |
| Vollkette | PEM: Leaf + Intermediate(s) | `fullchain.pem` | – |
| Root-Zertifikat | PEM/DER | `root.crt.pem`, `root.der` | – |
| PKCS#12 | PBES2 PBKDF2-SHA256 + AES-256-CBC + HMAC-SHA256; Legacy-Schalter 3DES/SHA1 | `name.p12` | `p12-keystore` |
| CSR | PKCS#10 PEM | `name.csr.pem` | `rcgen` |
| CRL | DER und PEM | `issuing.crl`, `issuing.crl.pem` | `rcgen` |
| Root-Bundle | age (Passphrase, ASCII-Armor) um ein JSON-Dokument (Manifest, Zertifikat, PBES2-Schlüssel) | `name.certana-root` | `age` |

## PKCS#8 immer als Version 1

ring/rcgen erzeugen Ed25519-Schlüssel als PKCS#8 **v2** (OneAsymmetricKey mit eingebettetem Public Key). OpenSSL 3.0 verweigert diese Dateien («No supported data to decode»). `certana-core` normalisiert deshalb **jeden** Schlüssel beim Erzeugen und Import auf PKCS#8 v1 (`keys::normalize_pkcs8_v1`); rcgen lädt v1 problemlos. Gefunden durch den Interop-Test `tests/openssl_interop.rs`.

Interop-Tests laufen gegen die OpenSSL-CLI (`tests/openssl_interop.rs`, übersprungen wenn `openssl` fehlt): `openssl verify`, `openssl x509 -text`, `openssl pkey`, `openssl pkcs12 -info`, `openssl crl -text`.
