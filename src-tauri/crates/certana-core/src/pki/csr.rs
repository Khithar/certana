// SPDX-License-Identifier: GPL-3.0-or-later
//! PKCS#10 handling: parse what a requester sent (public key, subject, SANs) and build CSRs.

use rcgen::{CertificateParams, CertificateSigningRequestParams, DnType, DnValue, SanType};
use serde::{Deserialize, Serialize};

use super::keys::PrivateKey;
use super::profiles::{San, Subject};
use crate::{CertanaError, Result};

/// The parts of a CSR Certana is willing to take over. Everything else (CA flags, key usages,
/// validity) is decided by the profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrInfo {
    pub subject: Subject,
    pub sans: Vec<San>,
    pub key_algorithm: String,
    pub public_key_der: Vec<u8>,
}

pub fn parse_csr_pem(pem: &str) -> Result<CsrInfo> {
    let csr = CertificateSigningRequestParams::from_pem(pem)
        .map_err(|e| CertanaError::Invalid(format!("cannot parse CSR: {e}")))?;
    Ok(CsrInfo {
        subject: subject_from_params(&csr.params),
        sans: csr
            .params
            .subject_alt_names
            .iter()
            .filter_map(san_from_rcgen)
            .collect(),
        key_algorithm: format!("{:?}", csr.public_key.algorithm()),
        public_key_der: {
            use rcgen::PublicKeyData as _;
            csr.public_key.subject_public_key_info()
        },
    })
}

/// Build a CSR for a key Certana generated (e.g. when a third party should sign).
pub fn create_csr(subject: &Subject, sans: &[San], key: &PrivateKey) -> Result<String> {
    let mut params = CertificateParams::default();
    params.distinguished_name = subject.to_distinguished_name();
    params.subject_alt_names = sans.iter().map(San::to_rcgen).collect::<Result<_>>()?;
    let csr = params
        .serialize_request(&key.key_pair()?)
        .map_err(|e| CertanaError::Pki(format!("CSR creation failed: {e}")))?;
    csr.pem()
        .map_err(|e| CertanaError::Pki(format!("CSR encoding failed: {e}")))
}

pub(crate) fn subject_from_params(params: &CertificateParams) -> Subject {
    let mut s = Subject::default();
    for (ty, value) in params.distinguished_name.iter() {
        let v = match value {
            DnValue::Utf8String(x) => x.clone(),
            DnValue::PrintableString(x) => x.to_string(),
            DnValue::Ia5String(x) => x.to_string(),
            DnValue::TeletexString(x) => x.to_string(),
            _ => continue,
        };
        match ty {
            DnType::CommonName => s.common_name = v,
            DnType::OrganizationName => s.organization = v,
            DnType::OrganizationalUnitName => s.organizational_unit = v,
            DnType::CountryName => s.country = v,
            DnType::StateOrProvinceName => s.state = v,
            DnType::LocalityName => s.locality = v,
            _ => {}
        }
    }
    s
}

pub(crate) fn san_from_rcgen(san: &SanType) -> Option<San> {
    Some(match san {
        SanType::DnsName(d) => San::Dns(d.to_string()),
        SanType::IpAddress(ip) => San::Ip(*ip),
        SanType::Rfc822Name(m) => San::Email(m.to_string()),
        SanType::URI(u) => San::Uri(u.to_string()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pki::keys::KeyAlgorithm;

    #[test]
    fn csr_roundtrip() {
        let key = PrivateKey::generate(KeyAlgorithm::EcdsaP256).unwrap();
        let subject = Subject {
            common_name: "www.example.internal".into(),
            organization: "Example".into(),
            ..Default::default()
        };
        let sans = vec![
            San::Dns("www.example.internal".into()),
            San::Ip("10.0.0.1".parse().unwrap()),
        ];
        let pem = create_csr(&subject, &sans, &key).unwrap();
        assert!(pem.starts_with("-----BEGIN CERTIFICATE REQUEST-----"));
        let info = parse_csr_pem(&pem).unwrap();
        assert_eq!(info.subject, subject);
        assert_eq!(info.sans, sans);
        assert_eq!(info.public_key_der, key.public_key_der().unwrap());
    }
}
