// SPDX-License-Identifier: GPL-3.0-or-later
//! Leaf certificate issuance from a profile, either with a freshly generated key or from a CSR.

use rcgen::{CertificateParams, CertificateSigningRequestParams};
use serde::{Deserialize, Serialize};

use super::ca::{DistributionUrls, apply_urls, random_serial, serial_to_hex, sign_with, validity};
use super::keys::{KeyAlgorithm, PrivateKey};
use super::profiles::{Profile, San, Subject, apply_leaf_profile};
use crate::{CertanaError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSpec {
    pub profile: Profile,
    pub subject: Subject,
    pub sans: Vec<San>,
    pub validity_days: u32,
    /// URLs of the *issuing* CA (CRL-DP, AIA) embedded into the leaf.
    pub urls: DistributionUrls,
}

/// Result of an issuance. `key` is `None` when the request came from a CSR.
pub struct Issued {
    pub certificate_pem: String,
    pub serial_hex: String,
    pub key: Option<PrivateKey>,
}

/// Generate a key and issue a certificate for it.
pub fn issue_with_new_key(
    spec: &IssueSpec,
    algorithm: KeyAlgorithm,
    issuer_cert_pem: &str,
    issuer_key: &PrivateKey,
) -> Result<Issued> {
    let key = PrivateKey::generate(algorithm)?;
    let kp = key.key_pair()?;
    let params = leaf_params(spec, algorithm.is_rsa())?;
    let cert = sign_with(&params, &kp, issuer_cert_pem, issuer_key)?;
    Ok(Issued {
        certificate_pem: cert.pem(),
        serial_hex: serial_to_hex(params.serial_number.as_ref().expect("serial set")),
        key: Some(key),
    })
}

/// Issue for the public key inside a CSR. Only the public key is taken from the CSR; subject and
/// SANs come from `spec` (the UI pre-fills them from the CSR after the user confirms).
pub fn issue_from_csr(
    spec: &IssueSpec,
    csr_pem: &str,
    issuer_cert_pem: &str,
    issuer_key: &PrivateKey,
) -> Result<Issued> {
    let csr = CertificateSigningRequestParams::from_pem(csr_pem)
        .map_err(|e| CertanaError::Invalid(format!("cannot parse CSR: {e}")))?;
    let key_is_rsa = csr.public_key.algorithm() == &rcgen::PKCS_RSA_SHA256
        || csr.public_key.algorithm() == &rcgen::PKCS_RSA_SHA384
        || csr.public_key.algorithm() == &rcgen::PKCS_RSA_SHA512;
    let params = leaf_params(spec, key_is_rsa)?;
    let cert = sign_with(&params, &csr.public_key, issuer_cert_pem, issuer_key)?;
    Ok(Issued {
        certificate_pem: cert.pem(),
        serial_hex: serial_to_hex(params.serial_number.as_ref().expect("serial set")),
        key: None,
    })
}

fn leaf_params(spec: &IssueSpec, key_is_rsa: bool) -> Result<CertificateParams> {
    spec.profile.validate_sans(&spec.sans)?;
    if spec.validity_days == 0 || spec.validity_days > spec.profile.max_validity_days() {
        return Err(CertanaError::Invalid(format!(
            "validity must be between 1 and {} days for this profile",
            spec.profile.max_validity_days()
        )));
    }
    let mut params = CertificateParams::default();
    let mut subject = spec.subject.clone();
    if subject.common_name.trim().is_empty() {
        subject.common_name = spec
            .sans
            .first()
            .map(|s| match s {
                San::Dns(d) | San::Email(d) | San::Uri(d) => d.clone(),
                San::Ip(ip) => ip.to_string(),
            })
            .unwrap_or_default();
    }
    params.distinguished_name = subject.to_distinguished_name();
    params.subject_alt_names = spec.sans.iter().map(San::to_rcgen).collect::<Result<_>>()?;
    apply_leaf_profile(&mut params, spec.profile, key_is_rsa);
    params.serial_number = Some(random_serial());
    let (nb, na) = validity(spec.validity_days);
    params.not_before = nb;
    params.not_after = na;
    apply_urls(&mut params, &spec.urls)?;
    Ok(params)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::pki::ca::{CaSpec, create_intermediate, create_root};
    use crate::pki::csr::create_csr;
    use crate::pki::parse::parse_certificate_pem;

    pub(crate) fn test_ca() -> (crate::pki::ca::CaMaterial, crate::pki::ca::CaMaterial) {
        let mk = |cn: &str, pl: u8| CaSpec {
            subject: Subject::with_common_name(cn),
            algorithm: KeyAlgorithm::EcdsaP256,
            validity_days: 3650,
            path_len: pl,
            urls: DistributionUrls::default(),
        };
        let root = create_root(&mk("Test Root", 1)).unwrap();
        let inter =
            create_intermediate(&mk("Test Issuing", 0), &root.certificate_pem, &root.key).unwrap();
        (root, inter)
    }

    fn server_spec() -> IssueSpec {
        IssueSpec {
            profile: Profile::Server,
            subject: Subject {
                organization: "Example".into(),
                ..Default::default()
            },
            sans: vec![
                San::Dns("www.example.internal".into()),
                San::Ip("192.168.1.10".parse().unwrap()),
            ],
            validity_days: 397,
            urls: DistributionUrls {
                crl_url: Some("http://pki.example.internal/issuing.crl".into()),
                ca_issuers_url: Some("http://pki.example.internal/issuing.crt".into()),
                ocsp_url: None,
            },
        }
    }

    #[test]
    fn server_cert_with_new_key() {
        let (_root, inter) = test_ca();
        let issued = issue_with_new_key(
            &server_spec(),
            KeyAlgorithm::EcdsaP256,
            &inter.certificate_pem,
            &inter.key,
        )
        .unwrap();
        let info = parse_certificate_pem(&issued.certificate_pem).unwrap();
        assert!(!info.is_ca);
        assert_eq!(
            info.subject_common_name.as_deref(),
            Some("www.example.internal")
        );
        assert_eq!(info.sans.len(), 2);
        assert_eq!(info.extended_key_usages, vec!["serverAuth"]);
        assert!(info.key_usages.contains(&"digitalSignature".to_string()));
        assert!(!info.key_usages.contains(&"keyEncipherment".to_string()));
        assert_eq!(info.issuer_common_name.as_deref(), Some("Test Issuing"));
        assert!(issued.key.is_some());
    }

    #[test]
    fn rsa_server_cert_gets_key_encipherment() {
        let (_root, inter) = test_ca();
        let issued = issue_with_new_key(
            &server_spec(),
            KeyAlgorithm::Rsa2048,
            &inter.certificate_pem,
            &inter.key,
        )
        .unwrap();
        let info = parse_certificate_pem(&issued.certificate_pem).unwrap();
        assert!(info.key_usages.contains(&"keyEncipherment".to_string()));
    }

    #[test]
    fn csr_issuance_ignores_requested_flags() {
        let (_root, inter) = test_ca();
        let requester_key = PrivateKey::generate(KeyAlgorithm::EcdsaP256).unwrap();
        let csr = create_csr(
            &Subject::with_common_name("mail.example.internal"),
            &[San::Dns("mail.example.internal".into())],
            &requester_key,
        )
        .unwrap();
        let spec = IssueSpec {
            profile: Profile::Server,
            subject: Subject::with_common_name("mail.example.internal"),
            sans: vec![San::Dns("mail.example.internal".into())],
            validity_days: 90,
            urls: DistributionUrls::default(),
        };
        let issued = issue_from_csr(&spec, &csr, &inter.certificate_pem, &inter.key).unwrap();
        let info = parse_certificate_pem(&issued.certificate_pem).unwrap();
        assert!(!info.is_ca);
        assert_eq!(info.public_key_der, requester_key.public_key_der().unwrap());
        assert!(issued.key.is_none());
    }

    #[test]
    fn profile_validation() {
        let (_root, inter) = test_ca();
        let mut spec = server_spec();
        spec.sans.clear();
        assert!(
            issue_with_new_key(
                &spec,
                KeyAlgorithm::EcdsaP256,
                &inter.certificate_pem,
                &inter.key
            )
            .is_err()
        );
        let mut spec = server_spec();
        spec.validity_days = 900;
        assert!(
            issue_with_new_key(
                &spec,
                KeyAlgorithm::EcdsaP256,
                &inter.certificate_pem,
                &inter.key
            )
            .is_err()
        );
        let smime = IssueSpec {
            profile: Profile::Smime,
            subject: Subject::with_common_name("Alice"),
            sans: vec![San::Email("alice@example.internal".into())],
            validity_days: 365,
            urls: DistributionUrls::default(),
        };
        let issued = issue_with_new_key(
            &smime,
            KeyAlgorithm::Rsa2048,
            &inter.certificate_pem,
            &inter.key,
        )
        .unwrap();
        assert_eq!(
            parse_certificate_pem(&issued.certificate_pem)
                .unwrap()
                .extended_key_usages,
            vec!["emailProtection"]
        );
    }
}
