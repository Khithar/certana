// SPDX-License-Identifier: GPL-3.0-or-later
//! The offline root-CA bundle (`*.certana-root`): an age-encrypted (passphrase, scrypt) JSON
//! document containing the root certificate and the root key as PBES2-encrypted PKCS#8.
//! Two layers on purpose – an unpacked bundle still yields no plaintext key.

use std::io::{Read as _, Write as _};

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::keywrap::{check_passphrase_strength, decrypt_key_pem, encrypt_key_pem};
use crate::pki::keys::{KeyAlgorithm, PrivateKey};
use crate::pki::parse::parse_certificate_pem;
use crate::{CertanaError, Result};

pub const BUNDLE_EXTENSION: &str = "certana-root";
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootBundleManifest {
    pub format_version: u32,
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub certificate_sha256: String,
    pub created_at: OffsetDateTime,
    pub certana_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundleDocument {
    manifest: RootBundleManifest,
    certificate_pem: String,
    encrypted_key_pem: String,
}

/// Unlocked bundle contents (key in memory – drop as soon as the ceremony is done).
pub struct RootBundle {
    pub manifest: RootBundleManifest,
    pub certificate_pem: String,
    pub key: PrivateKey,
}

/// Serialise + encrypt. Returns ASCII-armored age bytes suitable for a file.
pub fn write_bundle(
    name: &str,
    certificate_pem: &str,
    key: &PrivateKey,
    passphrase: &SecretString,
) -> Result<Vec<u8>> {
    check_passphrase_strength(passphrase)?;
    let info = parse_certificate_pem(certificate_pem)?;
    if !info.is_ca {
        return Err(CertanaError::Invalid(
            "bundle certificate is not a CA certificate".into(),
        ));
    }
    let doc = BundleDocument {
        manifest: RootBundleManifest {
            format_version: FORMAT_VERSION,
            name: name.to_string(),
            algorithm: key.algorithm(),
            certificate_sha256: info.sha256_fingerprint,
            created_at: OffsetDateTime::now_utc(),
            certana_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        certificate_pem: certificate_pem.to_string(),
        encrypted_key_pem: encrypt_key_pem(key, passphrase)?,
    };
    let json = serde_json::to_vec_pretty(&doc).map_err(|e| CertanaError::Vault(e.to_string()))?;
    let encryptor = age::Encryptor::with_user_passphrase(passphrase.clone());
    let armored =
        age::armor::ArmoredWriter::wrap_output(Vec::new(), age::armor::Format::AsciiArmor)
            .map_err(|e| CertanaError::Vault(format!("armor: {e}")))?;
    let mut writer = encryptor
        .wrap_output(armored)
        .map_err(|e| CertanaError::Vault(format!("age: {e}")))?;
    writer.write_all(&json)?;
    let armored = writer
        .finish()
        .map_err(|e| CertanaError::Vault(format!("age: {e}")))?;
    armored
        .finish()
        .map_err(|e| CertanaError::Vault(format!("armor: {e}")))
}

/// Decrypt + verify a bundle. The same passphrase unlocks both layers.
pub fn read_bundle(data: &[u8], passphrase: &SecretString) -> Result<RootBundle> {
    let reader = age::armor::ArmoredReader::new(data);
    let decryptor = age::Decryptor::new(reader)
        .map_err(|e| CertanaError::Vault(format!("not a Certana root bundle: {e}")))?;
    let identity = age::scrypt::Identity::new(passphrase.clone());
    let mut plain = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| CertanaError::Vault("wrong passphrase or corrupted bundle".into()))?;
    let mut json = Vec::new();
    plain
        .read_to_end(&mut json)
        .map_err(|_| CertanaError::Vault("wrong passphrase or corrupted bundle".into()))?;
    let doc: BundleDocument = serde_json::from_slice(&json)
        .map_err(|e| CertanaError::Vault(format!("bundle content invalid: {e}")))?;
    if doc.manifest.format_version > FORMAT_VERSION {
        return Err(CertanaError::Vault(format!(
            "bundle format {} is newer than this Certana",
            doc.manifest.format_version
        )));
    }
    let info = parse_certificate_pem(&doc.certificate_pem)?;
    if info.sha256_fingerprint != doc.manifest.certificate_sha256 {
        return Err(CertanaError::Vault(
            "bundle certificate fingerprint mismatch".into(),
        ));
    }
    let key = decrypt_key_pem(&doc.encrypted_key_pem, doc.manifest.algorithm, passphrase)?;
    if key.public_key_der()? != info.public_key_der {
        return Err(CertanaError::Vault(
            "bundle key does not match certificate".into(),
        ));
    }
    Ok(RootBundle {
        manifest: doc.manifest,
        certificate_pem: doc.certificate_pem,
        key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pki::ca::{CaSpec, create_root};
    use crate::pki::profiles::Subject;

    #[test]
    fn bundle_roundtrip_and_wrong_passphrase() {
        let root = create_root(&CaSpec {
            subject: Subject::with_common_name("Bundle Root"),
            algorithm: KeyAlgorithm::EcdsaP384,
            validity_days: 365,
            path_len: 1,
            urls: Default::default(),
        })
        .unwrap();
        let pass = SecretString::from("a sufficiently long passphrase".to_string());
        let bytes = write_bundle("Bundle Root", &root.certificate_pem, &root.key, &pass).unwrap();
        assert!(
            std::str::from_utf8(&bytes)
                .unwrap()
                .starts_with("-----BEGIN AGE ENCRYPTED FILE-----")
        );
        let bundle = read_bundle(&bytes, &pass).unwrap();
        assert_eq!(bundle.manifest.name, "Bundle Root");
        assert_eq!(bundle.key.pkcs8_der(), root.key.pkcs8_der());
        assert_eq!(bundle.certificate_pem, root.certificate_pem);
        let wrong = SecretString::from("another long but wrong passphrase".to_string());
        assert!(matches!(
            read_bundle(&bytes, &wrong),
            Err(CertanaError::Vault(_))
        ));
        assert!(
            write_bundle(
                "x",
                &root.certificate_pem,
                &root.key,
                &SecretString::from("short".to_string())
            )
            .is_err()
        );
    }
}
