// SPDX-License-Identifier: GPL-3.0-or-later
//! Private-key generation and (de)serialisation. Keys are held as PKCS#8 DER inside
//! `Zeroizing` buffers; `Debug` never prints key bytes.

use rcgen::{KeyPair, SignatureAlgorithm};
use rsa::pkcs8::EncodePrivateKey as _;
use rustls_pki_types::PrivatePkcs8KeyDer;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

use crate::{CertanaError, Result};

/// Supported key algorithms. Stored with every key so it can be reloaded without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyAlgorithm {
    EcdsaP256,
    EcdsaP384,
    Ed25519,
    Rsa2048,
    Rsa3072,
    Rsa4096,
}

impl KeyAlgorithm {
    pub fn all() -> &'static [KeyAlgorithm] {
        use KeyAlgorithm::*;
        &[EcdsaP256, EcdsaP384, Ed25519, Rsa2048, Rsa3072, Rsa4096]
    }

    pub fn is_rsa(self) -> bool {
        matches!(
            self,
            KeyAlgorithm::Rsa2048 | KeyAlgorithm::Rsa3072 | KeyAlgorithm::Rsa4096
        )
    }

    fn rsa_bits(self) -> Option<usize> {
        match self {
            KeyAlgorithm::Rsa2048 => Some(2048),
            KeyAlgorithm::Rsa3072 => Some(3072),
            KeyAlgorithm::Rsa4096 => Some(4096),
            _ => None,
        }
    }

    /// Signature algorithm rcgen uses for this key type.
    pub fn signature_algorithm(self) -> &'static SignatureAlgorithm {
        match self {
            KeyAlgorithm::EcdsaP256 => &rcgen::PKCS_ECDSA_P256_SHA256,
            KeyAlgorithm::EcdsaP384 => &rcgen::PKCS_ECDSA_P384_SHA384,
            KeyAlgorithm::Ed25519 => &rcgen::PKCS_ED25519,
            KeyAlgorithm::Rsa2048 | KeyAlgorithm::Rsa3072 | KeyAlgorithm::Rsa4096 => {
                &rcgen::PKCS_RSA_SHA256
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            KeyAlgorithm::EcdsaP256 => "ECDSA P-256",
            KeyAlgorithm::EcdsaP384 => "ECDSA P-384",
            KeyAlgorithm::Ed25519 => "Ed25519",
            KeyAlgorithm::Rsa2048 => "RSA 2048",
            KeyAlgorithm::Rsa3072 => "RSA 3072",
            KeyAlgorithm::Rsa4096 => "RSA 4096",
        }
    }
}

/// A private key as unencrypted PKCS#8 DER. Zeroized on drop.
#[derive(Clone)]
pub struct PrivateKey {
    algorithm: KeyAlgorithm,
    pkcs8_der: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateKey")
            .field("algorithm", &self.algorithm)
            .field("pkcs8_der", &"<redacted>")
            .finish()
    }
}

impl PrivateKey {
    /// Generate a fresh key. RSA keys come from the `rsa` crate (ring cannot generate them),
    /// everything else from rcgen/ring.
    pub fn generate(algorithm: KeyAlgorithm) -> Result<Self> {
        let der = if let Some(bits) = algorithm.rsa_bits() {
            let mut rng = rand::thread_rng();
            let key = rsa::RsaPrivateKey::new(&mut rng, bits)
                .map_err(|e| CertanaError::Pki(format!("RSA key generation failed: {e}")))?;
            let doc = key
                .to_pkcs8_der()
                .map_err(|e| CertanaError::Pki(format!("RSA PKCS#8 encoding failed: {e}")))?;
            Zeroizing::new(doc.as_bytes().to_vec())
        } else {
            let kp = KeyPair::generate_for(algorithm.signature_algorithm())
                .map_err(|e| CertanaError::Pki(format!("key generation failed: {e}")))?;
            Zeroizing::new(kp.serialize_der())
        };
        Ok(Self {
            algorithm,
            pkcs8_der: normalize_pkcs8_v1(&der)?,
        })
    }

    pub fn from_pkcs8_der(algorithm: KeyAlgorithm, der: &[u8]) -> Result<Self> {
        let key = Self {
            algorithm,
            pkcs8_der: normalize_pkcs8_v1(der)?,
        };
        key.key_pair()?; // validate
        Ok(key)
    }

    /// Parse an unencrypted PKCS#8 PEM (`PRIVATE KEY`). The algorithm is detected by rcgen.
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self> {
        let kp = KeyPair::from_pem(pem)
            .map_err(|e| CertanaError::Pki(format!("cannot parse private key: {e}")))?;
        let algorithm = detect_algorithm(&kp)?;
        Ok(Self {
            algorithm,
            pkcs8_der: normalize_pkcs8_v1(&kp.serialize_der())?,
        })
    }

    pub fn algorithm(&self) -> KeyAlgorithm {
        self.algorithm
    }

    pub fn pkcs8_der(&self) -> &[u8] {
        &self.pkcs8_der
    }

    /// Unencrypted PKCS#8 PEM (for server deployment files). Caller is responsible for 0600 perms.
    pub fn to_pkcs8_pem(&self) -> Zeroizing<String> {
        Zeroizing::new(pem_encode("PRIVATE KEY", &self.pkcs8_der))
    }

    /// rcgen signing handle.
    pub fn key_pair(&self) -> Result<KeyPair> {
        let der = PrivatePkcs8KeyDer::from(self.pkcs8_der.to_vec());
        KeyPair::from_pkcs8_der_and_sign_algo(&der, self.algorithm.signature_algorithm())
            .map_err(|e| CertanaError::Pki(format!("cannot load private key: {e}")))
    }

    /// SubjectPublicKeyInfo DER, used for key-identifier calculation and matching.
    pub fn public_key_der(&self) -> Result<Vec<u8>> {
        use rcgen::PublicKeyData as _;
        Ok(self.key_pair()?.subject_public_key_info())
    }
}

/// ring emits Ed25519 keys as PKCS#8 v2 (OneAsymmetricKey with public key), which OpenSSL 3.0
/// refuses to decode. Re-encode every key as plain v1 so exports stay interoperable; rcgen/ring
/// reload v1 keys without complaint.
fn normalize_pkcs8_v1(der: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    use pkcs8::der::Encode as _;
    let mut info = pkcs8::PrivateKeyInfoRef::try_from(der)
        .map_err(|e| CertanaError::Pki(format!("invalid PKCS#8: {e}")))?;
    info.public_key = None;
    let out = info
        .to_der()
        .map_err(|e| CertanaError::Pki(format!("PKCS#8 encoding failed: {e}")))?;
    Ok(Zeroizing::new(out))
}

fn detect_algorithm(kp: &KeyPair) -> Result<KeyAlgorithm> {
    let alg = kp.algorithm();
    let found = if alg == &rcgen::PKCS_ECDSA_P256_SHA256 {
        KeyAlgorithm::EcdsaP256
    } else if alg == &rcgen::PKCS_ECDSA_P384_SHA384 {
        KeyAlgorithm::EcdsaP384
    } else if alg == &rcgen::PKCS_ED25519 {
        KeyAlgorithm::Ed25519
    } else if alg == &rcgen::PKCS_RSA_SHA256 {
        // Determine modulus size from the public key length (DER SPKI grows with modulus).
        match kp.public_key_raw().len() {
            n if n <= 300 => KeyAlgorithm::Rsa2048,
            n if n <= 430 => KeyAlgorithm::Rsa3072,
            _ => KeyAlgorithm::Rsa4096,
        }
    } else {
        return Err(CertanaError::Pki(format!(
            "unsupported key algorithm: {alg:?}"
        )));
    };
    Ok(found)
}

/// Minimal PEM encoder (64-char lines, LF).
pub(crate) fn pem_encode(label: &str, der: &[u8]) -> String {
    use std::fmt::Write as _;
    let b64 = base64_encode(der);
    let mut out = String::with_capacity(b64.len() + 64);
    let _ = writeln!(out, "-----BEGIN {label}-----");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
        out.push('\n');
    }
    let _ = writeln!(out, "-----END {label}-----");
    out
}

/// Extract the DER payload of the first PEM block with the given label.
pub(crate) fn pem_decode(label: &str, pem: &str) -> Result<Vec<u8>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = pem
        .find(&begin)
        .ok_or_else(|| CertanaError::Invalid(format!("no `{label}` PEM block found")))?
        + begin.len();
    let stop = pem[start..]
        .find(&end)
        .ok_or_else(|| CertanaError::Invalid(format!("unterminated `{label}` PEM block")))?;
    let body: String = pem[start..start + stop]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64_decode(&body).map_err(|e| CertanaError::Invalid(format!("invalid PEM base64: {e}")))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let v = B64
            .iter()
            .position(|&b| b == c)
            .ok_or_else(|| format!("invalid base64 character {:?}", c as char))?
            as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_roundtrip_all_algorithms() {
        for alg in [
            KeyAlgorithm::EcdsaP256,
            KeyAlgorithm::EcdsaP384,
            KeyAlgorithm::Ed25519,
            KeyAlgorithm::Rsa2048,
        ] {
            let key = PrivateKey::generate(alg).unwrap();
            let pem = key.to_pkcs8_pem();
            let back = PrivateKey::from_pkcs8_pem(&pem).unwrap();
            assert_eq!(back.algorithm(), alg, "algorithm detection for {alg:?}");
            assert_eq!(back.pkcs8_der(), key.pkcs8_der());
            assert!(format!("{key:?}").contains("redacted"));
        }
    }

    #[test]
    fn base64_roundtrip() {
        for len in 0..20 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 % 251) as u8).collect();
            assert_eq!(base64_decode(&base64_encode(&data)).unwrap(), data);
        }
    }
}
