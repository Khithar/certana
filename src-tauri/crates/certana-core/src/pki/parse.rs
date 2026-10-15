// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only certificate inspection for lists, detail views and validation.

use ::time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use x509_parser::prelude::*;

use super::keys::pem_decode;
use crate::{CertanaError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub subject: String,
    pub subject_common_name: Option<String>,
    pub issuer: String,
    pub issuer_common_name: Option<String>,
    pub serial_hex: String,
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
    pub is_ca: bool,
    pub path_len: Option<u32>,
    pub self_signed: bool,
    pub key_usages: Vec<String>,
    pub extended_key_usages: Vec<String>,
    pub sans: Vec<String>,
    pub crl_urls: Vec<String>,
    pub ca_issuers_urls: Vec<String>,
    pub ocsp_urls: Vec<String>,
    pub signature_algorithm: String,
    pub public_key_algorithm: String,
    pub public_key_der: Vec<u8>,
    pub sha256_fingerprint: String,
    pub sha1_fingerprint: String,
    /// Human-readable dump similar to `openssl x509 -text`.
    pub text: String,
}

pub fn parse_certificate_pem(pem: &str) -> Result<CertificateInfo> {
    parse_certificate_der(&pem_decode("CERTIFICATE", pem)?)
}

pub fn parse_certificate_der(der: &[u8]) -> Result<CertificateInfo> {
    let (_, cert) = X509Certificate::from_der(der)
        .map_err(|e| CertanaError::Invalid(format!("cannot parse certificate: {e}")))?;
    let cn = |name: &X509Name<'_>| {
        name.iter_common_name()
            .next()
            .and_then(|a| a.as_str().ok().map(str::to_string))
    };

    let (is_ca, path_len) = match cert.basic_constraints() {
        Ok(Some(bc)) => (bc.value.ca, bc.value.path_len_constraint),
        _ => (false, None),
    };
    let key_usages = match cert.key_usage() {
        Ok(Some(ku)) => {
            let k = ku.value;
            let mut v = Vec::new();
            if k.digital_signature() {
                v.push("digitalSignature")
            }
            if k.non_repudiation() {
                v.push("nonRepudiation")
            }
            if k.key_encipherment() {
                v.push("keyEncipherment")
            }
            if k.data_encipherment() {
                v.push("dataEncipherment")
            }
            if k.key_agreement() {
                v.push("keyAgreement")
            }
            if k.key_cert_sign() {
                v.push("keyCertSign")
            }
            if k.crl_sign() {
                v.push("crlSign")
            }
            if k.encipher_only() {
                v.push("encipherOnly")
            }
            if k.decipher_only() {
                v.push("decipherOnly")
            }
            v.into_iter().map(String::from).collect()
        }
        _ => Vec::new(),
    };
    let extended_key_usages = match cert.extended_key_usage() {
        Ok(Some(eku)) => {
            let e = eku.value;
            let mut v: Vec<String> = Vec::new();
            if e.any {
                v.push("any".into())
            }
            if e.server_auth {
                v.push("serverAuth".into())
            }
            if e.client_auth {
                v.push("clientAuth".into())
            }
            if e.code_signing {
                v.push("codeSigning".into())
            }
            if e.email_protection {
                v.push("emailProtection".into())
            }
            if e.time_stamping {
                v.push("timeStamping".into())
            }
            if e.ocsp_signing {
                v.push("ocspSigning".into())
            }
            v.extend(e.other.iter().map(|o| o.to_string()));
            v
        }
        _ => Vec::new(),
    };
    let sans = match cert.subject_alternative_name() {
        Ok(Some(san)) => san
            .value
            .general_names
            .iter()
            .map(general_name_to_string)
            .collect(),
        _ => Vec::new(),
    };
    let mut crl_urls = Vec::new();
    let mut ca_issuers_urls = Vec::new();
    let mut ocsp_urls = Vec::new();
    for ext in cert.extensions() {
        match ext.parsed_extension() {
            ParsedExtension::CRLDistributionPoints(points) => {
                for p in points.iter() {
                    if let Some(DistributionPointName::FullName(names)) = &p.distribution_point {
                        crl_urls.extend(names.iter().filter_map(uri_of));
                    }
                }
            }
            ParsedExtension::AuthorityInfoAccess(aia) => {
                for d in aia.accessdescs.iter() {
                    let Some(uri) = uri_of(&d.access_location) else {
                        continue;
                    };
                    if d.access_method == oid_registry::OID_PKIX_ACCESS_DESCRIPTOR_CA_ISSUERS {
                        ca_issuers_urls.push(uri);
                    } else if d.access_method == oid_registry::OID_PKIX_ACCESS_DESCRIPTOR_OCSP {
                        ocsp_urls.push(uri);
                    }
                }
            }
            _ => {}
        }
    }
    let registry = oid_registry::OidRegistry::default()
        .with_all_crypto()
        .with_x509();
    let alg_name = |oid: &oid_registry::Oid<'_>| {
        registry
            .get(oid)
            .map(|e| e.sn().to_string())
            .unwrap_or_else(|| oid.to_string())
    };
    let serial_hex = hex::encode(cert.raw_serial());
    let sha256 = hex::encode(sha2::Sha256::digest(der));
    let sha1 = hex::encode(sha1::Sha1::digest(der));
    let not_before = cert.validity().not_before.to_datetime();
    let not_after = cert.validity().not_after.to_datetime();
    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();
    let self_signed = cert.subject() == cert.issuer() && cert.verify_signature(None).is_ok();
    let signature_algorithm = alg_name(&cert.signature_algorithm.algorithm);
    let public_key_algorithm = alg_name(&cert.public_key().algorithm.algorithm);

    let mut text = String::new();
    {
        use std::fmt::Write as _;
        let _ = writeln!(text, "Certificate:");
        let _ = writeln!(text, "    Version: {}", cert.version());
        let _ = writeln!(text, "    Serial Number: {}", colon_hex(&serial_hex));
        let _ = writeln!(text, "    Signature Algorithm: {signature_algorithm}");
        let _ = writeln!(text, "    Issuer: {issuer}");
        let _ = writeln!(text, "    Validity");
        let _ = writeln!(text, "        Not Before: {not_before}");
        let _ = writeln!(text, "        Not After : {not_after}");
        let _ = writeln!(text, "    Subject: {subject}");
        let _ = writeln!(text, "    Public Key: {public_key_algorithm}");
        let _ = writeln!(text, "    X509v3 extensions:");
        let _ = writeln!(
            text,
            "        Basic Constraints: CA:{}{}",
            if is_ca { "TRUE" } else { "FALSE" },
            path_len
                .map(|p| format!(", pathlen:{p}"))
                .unwrap_or_default()
        );
        if !key_usages.is_empty() {
            let _ = writeln!(text, "        Key Usage: {}", key_usages.join(", "));
        }
        if !extended_key_usages.is_empty() {
            let _ = writeln!(
                text,
                "        Extended Key Usage: {}",
                extended_key_usages.join(", ")
            );
        }
        if !sans.is_empty() {
            let _ = writeln!(
                text,
                "        Subject Alternative Name: {}",
                sans.join(", ")
            );
        }
        if !crl_urls.is_empty() {
            let _ = writeln!(
                text,
                "        CRL Distribution Points: {}",
                crl_urls.join(", ")
            );
        }
        if !ca_issuers_urls.is_empty() {
            let _ = writeln!(
                text,
                "        Authority Information Access: CA Issuers - {}",
                ca_issuers_urls.join(", ")
            );
        }
        if !ocsp_urls.is_empty() {
            let _ = writeln!(
                text,
                "        Authority Information Access: OCSP - {}",
                ocsp_urls.join(", ")
            );
        }
        let _ = writeln!(text, "    SHA-256 Fingerprint: {}", colon_hex(&sha256));
        let _ = writeln!(text, "    SHA-1 Fingerprint: {}", colon_hex(&sha1));
    }

    Ok(CertificateInfo {
        subject_common_name: cn(cert.subject()),
        issuer_common_name: cn(cert.issuer()),
        subject,
        issuer,
        serial_hex,
        not_before,
        not_after,
        is_ca,
        path_len,
        self_signed,
        key_usages,
        extended_key_usages,
        sans,
        crl_urls,
        ca_issuers_urls,
        ocsp_urls,
        signature_algorithm,
        public_key_algorithm,
        public_key_der: cert.public_key().raw.to_vec(),
        sha256_fingerprint: sha256,
        sha1_fingerprint: sha1,
        text,
    })
}

/// Verify that `child` was signed by `parent` (signature + issuer/subject match).
pub fn verify_issued_by(child_pem: &str, parent_pem: &str) -> Result<bool> {
    let child_der = pem_decode("CERTIFICATE", child_pem)?;
    let parent_der = pem_decode("CERTIFICATE", parent_pem)?;
    let (_, child) =
        X509Certificate::from_der(&child_der).map_err(|e| CertanaError::Invalid(e.to_string()))?;
    let (_, parent) =
        X509Certificate::from_der(&parent_der).map_err(|e| CertanaError::Invalid(e.to_string()))?;
    Ok(child.issuer() == parent.subject()
        && child.verify_signature(Some(parent.public_key())).is_ok())
}

fn uri_of(name: &GeneralName<'_>) -> Option<String> {
    match name {
        GeneralName::URI(u) => Some((*u).to_string()),
        _ => None,
    }
}

fn general_name_to_string(name: &GeneralName<'_>) -> String {
    match name {
        GeneralName::DNSName(d) => format!("DNS:{d}"),
        GeneralName::IPAddress(ip) => match ip.len() {
            4 => format!("IP:{}", std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
            16 => {
                let mut o = [0u8; 16];
                o.copy_from_slice(ip);
                format!("IP:{}", std::net::Ipv6Addr::from(o))
            }
            _ => "IP:?".into(),
        },
        GeneralName::RFC822Name(m) => format!("email:{m}"),
        GeneralName::URI(u) => format!("URI:{u}"),
        other => format!("{other:?}"),
    }
}

pub(crate) fn colon_hex(hex: &str) -> String {
    hex.as_bytes()
        .chunks(2)
        .map(|c| std::str::from_utf8(c).unwrap_or("?").to_uppercase())
        .collect::<Vec<_>>()
        .join(":")
}
