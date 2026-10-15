// SPDX-License-Identifier: GPL-3.0-or-later
//! Passphrase-based private-key encryption: PKCS#8 / PBES2 with scrypt + AES-256-CBC.
//! The output is a standard `ENCRYPTED PRIVATE KEY` PEM that OpenSSL 3 can read.

use pkcs8::der::SecretDocument;
use pkcs8::der::pem::LineEnding;
use pkcs8::{EncryptedPrivateKeyInfoRef, PrivateKeyInfoRef};
use secrecy::{ExposeSecret as _, SecretString};
use zeroize::Zeroizing;

use crate::pki::keys::{KeyAlgorithm, PrivateKey};
use crate::{CertanaError, Result};

const LABEL: &str = "ENCRYPTED PRIVATE KEY";

/// Minimum passphrase length enforced for CA keys.
pub const MIN_PASSPHRASE_LEN: usize = 16;

pub fn check_passphrase_strength(passphrase: &SecretString) -> Result<()> {
    let p = passphrase.expose_secret();
    if p.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(CertanaError::Invalid(format!(
            "passphrase must have at least {MIN_PASSPHRASE_LEN} characters"
        )));
    }
    Ok(())
}

/// Encrypt to PEM (scrypt N=2^15, r=8, p=1, AES-256-CBC – the pkcs8 crate defaults).
pub fn encrypt_key_pem(key: &PrivateKey, passphrase: &SecretString) -> Result<String> {
    let pki = PrivateKeyInfoRef::try_from(key.pkcs8_der())
        .map_err(|e| CertanaError::Vault(format!("invalid PKCS#8: {e}")))?;
    let doc = pki
        .encrypt(passphrase.expose_secret().as_bytes())
        .map_err(|e| CertanaError::Vault(format!("encryption failed: {e}")))?;
    let pem = doc
        .to_pem(LABEL, LineEnding::LF)
        .map_err(|e| CertanaError::Vault(format!("PEM encoding failed: {e}")))?;
    Ok(pem.to_string())
}

/// Decrypt an `ENCRYPTED PRIVATE KEY` PEM. Wrong passphrases surface as `CertanaError::Vault`.
pub fn decrypt_key_pem(
    pem: &str,
    algorithm: KeyAlgorithm,
    passphrase: &SecretString,
) -> Result<PrivateKey> {
    let (label, doc) = SecretDocument::from_pem(pem)
        .map_err(|e| CertanaError::Vault(format!("invalid encrypted key PEM: {e}")))?;
    if label != LABEL {
        return Err(CertanaError::Vault(format!(
            "expected `{LABEL}` block, found `{label}`"
        )));
    }
    let epki = EncryptedPrivateKeyInfoRef::try_from(doc.as_bytes())
        .map_err(|e| CertanaError::Vault(format!("invalid EncryptedPrivateKeyInfo: {e}")))?;
    let plain = epki
        .decrypt(passphrase.expose_secret().as_bytes())
        .map_err(|_| CertanaError::Vault("wrong passphrase or corrupted key".into()))?;
    let der: Zeroizing<Vec<u8>> = plain.to_bytes();
    PrivateKey::from_pkcs8_der(algorithm, &der)
}

/// Decrypt when the algorithm is unknown (imports): try to detect it from the key itself.
pub fn decrypt_key_pem_detect(pem: &str, passphrase: &SecretString) -> Result<PrivateKey> {
    let (_, doc) = SecretDocument::from_pem(pem)
        .map_err(|e| CertanaError::Vault(format!("invalid encrypted key PEM: {e}")))?;
    let epki = EncryptedPrivateKeyInfoRef::try_from(doc.as_bytes())
        .map_err(|e| CertanaError::Vault(format!("invalid EncryptedPrivateKeyInfo: {e}")))?;
    let plain = epki
        .decrypt(passphrase.expose_secret().as_bytes())
        .map_err(|_| CertanaError::Vault("wrong passphrase or corrupted key".into()))?;
    let pem = plain
        .to_pem("PRIVATE KEY", LineEnding::LF)
        .map_err(|e| CertanaError::Vault(format!("PEM encoding failed: {e}")))?;
    PrivateKey::from_pkcs8_pem(&pem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = PrivateKey::generate(KeyAlgorithm::EcdsaP256).unwrap();
        let pass = SecretString::from("correct horse battery staple".to_string());
        let pem = encrypt_key_pem(&key, &pass).unwrap();
        assert!(pem.starts_with("-----BEGIN ENCRYPTED PRIVATE KEY-----"));
        let back = decrypt_key_pem(&pem, KeyAlgorithm::EcdsaP256, &pass).unwrap();
        assert_eq!(back.pkcs8_der(), key.pkcs8_der());
        let detected = decrypt_key_pem_detect(&pem, &pass).unwrap();
        assert_eq!(detected.algorithm(), KeyAlgorithm::EcdsaP256);
        let wrong = SecretString::from("not the passphrase at all".to_string());
        assert!(matches!(
            decrypt_key_pem(&pem, KeyAlgorithm::EcdsaP256, &wrong),
            Err(CertanaError::Vault(_))
        ));
    }

    #[test]
    fn passphrase_policy() {
        assert!(check_passphrase_strength(&SecretString::from("short".to_string())).is_err());
        assert!(
            check_passphrase_strength(&SecretString::from("sixteen characters!!".to_string()))
                .is_ok()
        );
    }
}
