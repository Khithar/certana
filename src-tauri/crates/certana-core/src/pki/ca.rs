// SPDX-License-Identifier: GPL-3.0-or-later
//! Root and intermediate CA certificates plus the shared helpers for serials, validity and
//! the Authority-Information-Access extension.

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CustomExtension, IsCa, Issuer, KeyIdMethod,
    KeyPair, KeyUsagePurpose, SerialNumber,
};
use rustls_pki_types::CertificateDer;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use x509_cert::der::Encode as _;
use x509_cert::der::asn1::{Ia5String, ObjectIdentifier};
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::{AccessDescription, AuthorityInfoAccessSyntax};

use super::keys::{KeyAlgorithm, PrivateKey, pem_decode};
use super::profiles::Subject;
use crate::{CertanaError, Result};

/// Where relying parties can fetch the CRL and the issuer certificate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributionUrls {
    /// CRL distribution point, e.g. `http://pki.example.internal/issuing.crl`.
    pub crl_url: Option<String>,
    /// AIA caIssuers URL (the CA certificate), e.g. `http://pki.example.internal/issuing.crt`.
    pub ca_issuers_url: Option<String>,
    /// AIA OCSP URL (reserved; no responder in v1).
    pub ocsp_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaSpec {
    pub subject: Subject,
    pub algorithm: KeyAlgorithm,
    pub validity_days: u32,
    /// `pathlen` – 1 for a root that signs one intermediate level, 0 for an issuing CA.
    pub path_len: u8,
    /// URLs embedded into certificates issued *by* this CA's parent for this CA itself.
    pub urls: DistributionUrls,
}

/// A CA certificate together with its private key (unencrypted, in memory only).
pub struct CaMaterial {
    pub certificate_pem: String,
    pub serial_hex: String,
    pub key: PrivateKey,
}

/// Create a self-signed root CA.
pub fn create_root(spec: &CaSpec) -> Result<CaMaterial> {
    let key = PrivateKey::generate(spec.algorithm)?;
    let kp = key.key_pair()?;
    let mut params = ca_params(spec)?;
    params.use_authority_key_identifier_extension = false;
    let serial = params.serial_number.clone().expect("serial set");
    let cert = params
        .self_signed(&kp)
        .map_err(|e| CertanaError::Pki(format!("root signing failed: {e}")))?;
    Ok(CaMaterial {
        certificate_pem: cert.pem(),
        serial_hex: serial_to_hex(&serial),
        key,
    })
}

/// Create an intermediate CA signed by `issuer_cert_pem`/`issuer_key` (the root, during a
/// ceremony). Returns the new intermediate's certificate and key.
pub fn create_intermediate(
    spec: &CaSpec,
    issuer_cert_pem: &str,
    issuer_key: &PrivateKey,
) -> Result<CaMaterial> {
    let key = PrivateKey::generate(spec.algorithm)?;
    let kp = key.key_pair()?;
    let params = ca_params(spec)?;
    let cert = sign_with(&params, &kp, issuer_cert_pem, issuer_key)?;
    Ok(CaMaterial {
        certificate_pem: cert.pem(),
        serial_hex: serial_to_hex(&params.serial_number.expect("serial set")),
        key,
    })
}

/// Produce a PKCS#10 request for an intermediate so an *external* root can sign it.
pub fn intermediate_csr(spec: &CaSpec) -> Result<(String, PrivateKey)> {
    let key = PrivateKey::generate(spec.algorithm)?;
    let kp = key.key_pair()?;
    let params = ca_params(spec)?;
    let csr = params
        .serialize_request(&kp)
        .map_err(|e| CertanaError::Pki(format!("CSR creation failed: {e}")))?;
    let pem = csr
        .pem()
        .map_err(|e| CertanaError::Pki(format!("CSR encoding failed: {e}")))?;
    Ok((pem, key))
}

/// Sign `params` for public key `subject_key` using an existing CA certificate + key.
pub(crate) fn sign_with(
    params: &CertificateParams,
    subject_key: &impl rcgen::PublicKeyData,
    issuer_cert_pem: &str,
    issuer_key: &PrivateKey,
) -> Result<Certificate> {
    let issuer = issuer_from(issuer_cert_pem, issuer_key.key_pair()?)?;
    params
        .signed_by(subject_key, &issuer)
        .map_err(|e| CertanaError::Pki(format!("signing failed: {e}")))
}

pub(crate) fn issuer_from(issuer_cert_pem: &str, kp: KeyPair) -> Result<Issuer<'static, KeyPair>> {
    let der = pem_decode("CERTIFICATE", issuer_cert_pem)?;
    let der = CertificateDer::from(der);
    Issuer::from_ca_cert_der(&der, kp)
        .map_err(|e| CertanaError::Pki(format!("cannot load issuer certificate: {e}")))
}

fn ca_params(spec: &CaSpec) -> Result<CertificateParams> {
    if spec.subject.common_name.trim().is_empty() {
        return Err(CertanaError::Invalid("CA needs a common name".into()));
    }
    let mut params = CertificateParams::default();
    params.distinguished_name = spec.subject.to_distinguished_name();
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(spec.path_len));
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    params.use_authority_key_identifier_extension = true;
    params.key_identifier_method = KeyIdMethod::Sha256;
    params.serial_number = Some(random_serial());
    let (nb, na) = validity(spec.validity_days);
    params.not_before = nb;
    params.not_after = na;
    apply_urls(&mut params, &spec.urls)?;
    Ok(params)
}

/// 128-bit random serial with the top bit clear (positive INTEGER, ≥ 64 bit entropy).
pub(crate) fn random_serial() -> SerialNumber {
    use rand::RngCore as _;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes[0] &= 0x7f;
    bytes[0] |= 0x01; // never a leading zero byte → stable hex length
    SerialNumber::from_slice(&bytes)
}

pub(crate) fn serial_to_hex(serial: &SerialNumber) -> String {
    hex::encode(serial.to_bytes())
}

/// `notBefore` one minute in the past (clock skew), `notAfter` = now + days.
pub(crate) fn validity(days: u32) -> (OffsetDateTime, OffsetDateTime) {
    let now = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("valid");
    (
        now - Duration::minutes(1),
        now + Duration::days(i64::from(days)),
    )
}

/// Attach CRL-DP and AIA extensions. rcgen has no AIA builder, so it is DER-encoded via x509-cert.
pub(crate) fn apply_urls(params: &mut CertificateParams, urls: &DistributionUrls) -> Result<()> {
    if let Some(crl) = urls.crl_url.as_deref().filter(|s| !s.trim().is_empty()) {
        params.crl_distribution_points = vec![rcgen::CrlDistributionPoint {
            uris: vec![crl.trim().to_string()],
        }];
    }
    let mut descriptions = Vec::new();
    let mut add = |oid: &str, url: &Option<String>| -> Result<()> {
        if let Some(u) = url.as_deref().filter(|s| !s.trim().is_empty()) {
            let uri = Ia5String::new(u.trim())
                .map_err(|_| CertanaError::Invalid(format!("URL is not ASCII: {u}")))?;
            descriptions.push(AccessDescription {
                access_method: ObjectIdentifier::new_unwrap(oid),
                access_location: GeneralName::UniformResourceIdentifier(uri),
            });
        }
        Ok(())
    };
    add("1.3.6.1.5.5.7.48.1", &urls.ocsp_url)?; // id-ad-ocsp
    add("1.3.6.1.5.5.7.48.2", &urls.ca_issuers_url)?; // id-ad-caIssuers
    if !descriptions.is_empty() {
        let der = AuthorityInfoAccessSyntax(descriptions)
            .to_der()
            .map_err(|e| CertanaError::Pki(format!("AIA encoding failed: {e}")))?;
        params
            .custom_extensions
            .push(CustomExtension::from_oid_content(
                &[1, 3, 6, 1, 5, 5, 7, 1, 1],
                der,
            ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(cn: &str, alg: KeyAlgorithm, path_len: u8) -> CaSpec {
        CaSpec {
            subject: Subject {
                common_name: cn.into(),
                organization: "Example".into(),
                country: "CH".into(),
                ..Default::default()
            },
            algorithm: alg,
            validity_days: 3650,
            path_len,
            urls: DistributionUrls {
                crl_url: Some("http://pki.example.internal/root.crl".into()),
                ca_issuers_url: Some("http://pki.example.internal/root.crt".into()),
                ocsp_url: None,
            },
        }
    }

    #[test]
    fn root_and_intermediate_chain() {
        let root = create_root(&spec("Example Root CA", KeyAlgorithm::EcdsaP384, 1)).unwrap();
        let inter = create_intermediate(
            &spec("Example Issuing CA", KeyAlgorithm::EcdsaP256, 0),
            &root.certificate_pem,
            &root.key,
        )
        .unwrap();
        let info = crate::pki::parse::parse_certificate_pem(&inter.certificate_pem).unwrap();
        assert!(info.is_ca);
        assert_eq!(info.path_len, Some(0));
        assert_eq!(info.issuer_common_name.as_deref(), Some("Example Root CA"));
        assert_eq!(info.crl_urls, vec!["http://pki.example.internal/root.crl"]);
        assert_eq!(
            info.ca_issuers_urls,
            vec!["http://pki.example.internal/root.crt"]
        );
        assert_eq!(info.serial_hex, inter.serial_hex);
        assert_eq!(info.serial_hex.len(), 32);
    }

    #[test]
    fn rsa_root() {
        let root = create_root(&spec("RSA Root", KeyAlgorithm::Rsa2048, 1)).unwrap();
        let info = crate::pki::parse::parse_certificate_pem(&root.certificate_pem).unwrap();
        assert!(info.is_ca);
        assert!(info.self_signed);
    }
}
