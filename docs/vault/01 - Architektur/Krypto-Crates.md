---
type: reference
status: active
tags: [type/reference, area/architektur, area/pki]
---

# Krypto- und Persistenz-Crates

Zurück zu [[01 - Architektur/Überblick]]. Stand der Verifikation: 2026-10-15 (docs.rs/crates.io).

| Zweck | Crate | Version | Einstiegspunkt |
|---|---|---|---|
| Zertifikate, CSR, CRL erzeugen | `rcgen` (Features `pem`, `ring`, `x509-parser`, `zeroize`) | 0.14 | `CertificateParams`, `IsCa::Ca(BasicConstraints::Constrained(n))`, `Issuer`, `CertificateSigningRequestParams::from_pem`, `CertificateRevocationListParams`, `RevocationReason` |
| RSA-Schlüssel erzeugen | `rsa` | 0.9 | `RsaPrivateKey::new` → PKCS#8-DER → `rcgen::KeyPair::from_pkcs8_der_and_sign_algo` (ring signiert RSA, erzeugt aber keine RSA-Keys) |
| ECDSA P-256/P-384, Ed25519 | `rcgen` | – | `KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256 …)` |
| Schlüssel verschlüsseln | `pkcs8` (+`pkcs5`, Feature `encryption`) | 0.11 | `to_pkcs8_encrypted_pem` – scrypt (N=2^15, r=8, p=1) + AES-256-CBC, OpenSSL-3-kompatibel. **Nicht** `scrypt_aes256gcm` (nicht in RFC 8018) |
| Root-Bundle-Container | `age` (Feature `armor`) | 0.12 | `Encryptor::with_user_passphrase` |
| PKCS#12 lesen/schreiben | `p12-keystore` | 0.4 | `KeyStore::writer(pw)` – Default PBES2/AES-256 + HMAC-SHA256; Legacy-Schalter 3DES/SHA1 |
| Zertifikate parsen | `x509-parser` | 0.18 | `X509Certificate::from_der`, `ParsedExtension` |
| AIA-Extension als DER | `x509-cert` | 0.3 | `ext::pkix::AuthorityInfoAccessSyntax` + `der::Encode` → `rcgen::CustomExtension::from_oid_content` |
| Passphrase-KDF | `argon2` | 0.6 | Argon2id, `hash_password_into` |
| Speicher-Hygiene | `zeroize`, `secrecy` | 1.x / 0.10 | `Zeroizing<Vec<u8>>`, `SecretString` |
| SQLite | `rusqlite` (`bundled`, `backup`) + `rusqlite_migration` | 0.40 / 2.6 | `Migrations::to_latest` |
| OCSP (nicht v1) | `x509-ocsp` | 0.2 | WIP-Status, siehe [[05 - Entscheidungen/ADR-Index]] |

**Bewusst nicht:** `aws_lc_rs` (CMake/C-Build auf Windows), SQLCipher (zieht OpenSSL), `tauri-plugin-sql` (SQL im Webview), `tauri-plugin-fs` (Dateizugriff nur über eigene Commands).
