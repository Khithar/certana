// SPDX-License-Identifier: GPL-3.0-or-later
//! Export helpers: PEM↔DER, chain assembly, PKCS#12 containers.

use p12_keystore::{
    Certificate as P12Cert, EncryptionAlgorithm, KeyStore, KeyStoreEntry, MacAlgorithm,
    PrivateKey as P12Key, PrivateKeyChain,
};
use serde::{Deserialize, Serialize};

use super::keys::{PrivateKey, pem_decode};
use crate::{CertanaError, Result};

pub fn certificate_pem_to_der(pem: &str) -> Result<Vec<u8>> {
    pem_decode("CERTIFICATE", pem)
}

/// All `CERTIFICATE` blocks in a PEM text, in order.
pub fn split_certificate_pems(pem: &str) -> Vec<String> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let mut out = Vec::new();
    let mut rest = pem;
    while let Some(s) = rest.find(BEGIN) {
        let Some(e) = rest[s..].find(END) else { break };
        out.push(format!("{}\n", rest[s..s + e + END.len()].trim()));
        rest = &rest[s + e + END.len()..];
    }
    out
}

/// `chain.pem` = issuing chain without the leaf and without the root; `fullchain.pem` = leaf + chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainFiles {
    pub cert: String,
    pub chain: String,
    pub fullchain: String,
}

pub fn build_chain(leaf_pem: &str, intermediates_pem: &[String]) -> ChainFiles {
    let chain: String = intermediates_pem
        .iter()
        .map(|s| format!("{}\n", s.trim()))
        .collect();
    ChainFiles {
        cert: format!("{}\n", leaf_pem.trim()),
        fullchain: format!("{}\n{chain}", leaf_pem.trim()),
        chain,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pkcs12Flavor {
    /// PBES2 with PBKDF2-HMAC-SHA256 + AES-256-CBC, MAC HMAC-SHA256 (OpenSSL 3 default, Windows 10+, modern Firefox).
    Modern,
    /// 3DES + SHA-1 for legacy importers (Windows Server 2016, old Java).
    Legacy,
}

/// Bundle key + leaf + chain into a password-protected PKCS#12 file.
pub fn to_pkcs12(
    friendly_name: &str,
    key: &PrivateKey,
    leaf_pem: &str,
    chain_pems: &[String],
    password: &str,
    flavor: Pkcs12Flavor,
) -> Result<Vec<u8>> {
    let p12_key = P12Key::from_der(key.pkcs8_der())
        .map_err(|e| CertanaError::Pki(format!("PKCS#12 key: {e}")))?;
    let mut certs = vec![
        P12Cert::from_der(&certificate_pem_to_der(leaf_pem)?)
            .map_err(|e| CertanaError::Pki(format!("PKCS#12 cert: {e}")))?,
    ];
    for c in chain_pems {
        certs.push(
            P12Cert::from_der(&certificate_pem_to_der(c)?)
                .map_err(|e| CertanaError::Pki(format!("PKCS#12 chain: {e}")))?,
        );
    }
    let local_key_id = {
        use sha2::Digest as _;
        sha1::Sha1::digest(certificate_pem_to_der(leaf_pem)?).to_vec()
    };
    let chain = PrivateKeyChain::new(local_key_id, p12_key, certs);
    let mut store = KeyStore::new();
    store.add_entry(friendly_name, KeyStoreEntry::PrivateKeyChain(chain));
    let writer = store.writer(password);
    let writer = match flavor {
        Pkcs12Flavor::Modern => writer
            .encryption_algorithm(EncryptionAlgorithm::PbeWithHmacSha256AndAes256)
            .mac_algorithm(MacAlgorithm::HmacSha256),
        Pkcs12Flavor::Legacy => writer
            .encryption_algorithm(EncryptionAlgorithm::PbeWithShaAnd3KeyTripleDesCbc)
            .mac_algorithm(MacAlgorithm::HmacSha1),
    };
    writer
        .write()
        .map_err(|e| CertanaError::Pki(format!("PKCS#12 encoding failed: {e}")))
}

/// Read key + certificates back from a PKCS#12 (used for root import).
pub fn from_pkcs12(data: &[u8], password: &str) -> Result<(PrivateKey, Vec<String>)> {
    let store = KeyStore::from_pkcs12(data, password, p12_keystore::Pkcs12ImportPolicy::default())
        .map_err(|e| CertanaError::Invalid(format!("cannot open PKCS#12: {e}")))?;
    let (_, chain) = store
        .private_key_chain()
        .ok_or_else(|| CertanaError::Invalid("PKCS#12 contains no private key".into()))?;
    let pem = super::keys::pem_encode("PRIVATE KEY", chain.key().as_der());
    let key = PrivateKey::from_pkcs8_pem(&pem)?;
    let certs = chain
        .certs()
        .iter()
        .map(|c| super::keys::pem_encode("CERTIFICATE", c.as_der()))
        .collect();
    Ok((key, certs))
}

/// Server configuration snippets shown after export.
pub fn server_snippets(base_dir: &str, name: &str) -> ServerSnippets {
    let dir = base_dir.trim_end_matches('/');
    ServerSnippets {
        nginx: format!(
            "ssl_certificate     {dir}/{name}.fullchain.pem;\nssl_certificate_key {dir}/{name}.key.pem;\nssl_protocols       TLSv1.2 TLSv1.3;\n"
        ),
        apache: format!(
            "SSLEngine on\nSSLCertificateFile    {dir}/{name}.fullchain.pem\nSSLCertificateKeyFile {dir}/{name}.key.pem\n# Apache < 2.4.8: additionally\n# SSLCertificateChainFile {dir}/{name}.chain.pem\n"
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSnippets {
    pub nginx: String,
    pub apache: String,
}

/// PEM-wrap a DER CRL (`X509 CRL` label).
pub fn crl_der_to_pem(der: &[u8]) -> String {
    super::keys::pem_encode("X509 CRL", der)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pki::issue::tests::test_ca;
    use crate::pki::issue::{IssueSpec, issue_with_new_key};
    use crate::pki::keys::KeyAlgorithm;
    use crate::pki::profiles::{Profile, San, Subject};

    #[test]
    fn pkcs12_roundtrip() {
        let (root, inter) = test_ca();
        let spec = IssueSpec {
            profile: Profile::Server,
            subject: Subject::default(),
            sans: vec![San::Dns("p12.example.internal".into())],
            validity_days: 30,
            urls: Default::default(),
        };
        let issued = issue_with_new_key(
            &spec,
            KeyAlgorithm::EcdsaP256,
            &inter.certificate_pem,
            &inter.key,
        )
        .unwrap();
        for flavor in [Pkcs12Flavor::Modern, Pkcs12Flavor::Legacy] {
            let p12 = to_pkcs12(
                "p12 test",
                issued.key.as_ref().unwrap(),
                &issued.certificate_pem,
                &[inter.certificate_pem.clone(), root.certificate_pem.clone()],
                "secret",
                flavor,
            )
            .unwrap();
            let (key, certs) = from_pkcs12(&p12, "secret").unwrap();
            assert_eq!(key.pkcs8_der(), issued.key.as_ref().unwrap().pkcs8_der());
            assert_eq!(certs.len(), 3);
        }
    }

    #[test]
    fn chain_files() {
        let files = build_chain(
            "-----BEGIN CERTIFICATE-----\nAAA\n-----END CERTIFICATE-----",
            &["-----BEGIN CERTIFICATE-----\nBBB\n-----END CERTIFICATE-----\n".into()],
        );
        assert_eq!(split_certificate_pems(&files.fullchain).len(), 2);
        assert_eq!(split_certificate_pems(&files.chain).len(), 1);
        assert!(files.fullchain.starts_with(files.cert.trim()));
    }
}
