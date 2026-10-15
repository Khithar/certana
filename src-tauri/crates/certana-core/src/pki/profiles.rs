// SPDX-License-Identifier: GPL-3.0-or-later
//! Issuance profiles: which key usages, extended key usages and validity each certificate
//! type gets. The profile – never the CSR – decides these extensions.

use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyUsagePurpose,
    SanType,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

use super::keys::KeyAlgorithm;
use crate::{CertanaError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// TLS server (Apache/nginx): serverAuth, DNS/IP SANs required.
    Server,
    /// TLS client authentication.
    Client,
    /// S/MIME e-mail protection: rfc822Name SAN required.
    Smime,
    /// Code signing.
    CodeSigning,
}

impl Profile {
    pub fn all() -> &'static [Profile] {
        &[
            Profile::Server,
            Profile::Client,
            Profile::Smime,
            Profile::CodeSigning,
        ]
    }

    pub fn default_validity_days(self) -> u32 {
        match self {
            Profile::Server => 397,
            Profile::Client => 365,
            Profile::Smime => 365,
            Profile::CodeSigning => 365,
        }
    }

    pub fn max_validity_days(self) -> u32 {
        match self {
            Profile::Server => 825,
            Profile::Client | Profile::Smime | Profile::CodeSigning => 1095,
        }
    }

    pub fn default_key_algorithm(self) -> KeyAlgorithm {
        match self {
            Profile::Server | Profile::Client => KeyAlgorithm::EcdsaP256,
            Profile::Smime | Profile::CodeSigning => KeyAlgorithm::Rsa3072,
        }
    }

    pub fn key_usages(self, key_is_rsa: bool) -> Vec<KeyUsagePurpose> {
        match self {
            Profile::Server => {
                let mut ku = vec![KeyUsagePurpose::DigitalSignature];
                if key_is_rsa {
                    ku.push(KeyUsagePurpose::KeyEncipherment);
                }
                ku
            }
            Profile::Client | Profile::CodeSigning => vec![KeyUsagePurpose::DigitalSignature],
            Profile::Smime => vec![
                KeyUsagePurpose::DigitalSignature,
                KeyUsagePurpose::KeyEncipherment,
            ],
        }
    }

    pub fn extended_key_usages(self) -> Vec<ExtendedKeyUsagePurpose> {
        match self {
            Profile::Server => vec![ExtendedKeyUsagePurpose::ServerAuth],
            Profile::Client => vec![ExtendedKeyUsagePurpose::ClientAuth],
            Profile::Smime => vec![ExtendedKeyUsagePurpose::EmailProtection],
            Profile::CodeSigning => vec![ExtendedKeyUsagePurpose::CodeSigning],
        }
    }

    /// Validate the subject alternative names against the profile rules.
    pub fn validate_sans(self, sans: &[San]) -> Result<()> {
        let has_host = sans.iter().any(|s| matches!(s, San::Dns(_) | San::Ip(_)));
        let has_mail = sans.iter().any(|s| matches!(s, San::Email(_)));
        match self {
            Profile::Server if !has_host => Err(CertanaError::Invalid(
                "server certificates need at least one DNS name or IP address".into(),
            )),
            Profile::Smime if !has_mail => Err(CertanaError::Invalid(
                "S/MIME certificates need an e-mail address".into(),
            )),
            _ => Ok(()),
        }
    }
}

/// Subject alternative name as the user enters it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum San {
    Dns(String),
    Ip(IpAddr),
    Email(String),
    Uri(String),
}

impl San {
    pub fn to_rcgen(&self) -> Result<SanType> {
        let conv = |s: &str| {
            s.try_into()
                .map_err(|_| CertanaError::Invalid(format!("not a valid ASCII name: {s}")))
        };
        Ok(match self {
            San::Dns(d) => SanType::DnsName(conv(d)?),
            San::Ip(ip) => SanType::IpAddress(*ip),
            San::Email(m) => SanType::Rfc822Name(conv(m)?),
            San::Uri(u) => SanType::URI(conv(u)?),
        })
    }

    pub fn display(&self) -> String {
        match self {
            San::Dns(d) => format!("DNS:{d}"),
            San::Ip(ip) => format!("IP:{ip}"),
            San::Email(m) => format!("email:{m}"),
            San::Uri(u) => format!("URI:{u}"),
        }
    }
}

/// Distinguished-name fields. Empty strings are omitted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub common_name: String,
    pub organization: String,
    pub organizational_unit: String,
    pub country: String,
    pub state: String,
    pub locality: String,
}

impl Subject {
    pub fn with_common_name(cn: impl Into<String>) -> Self {
        Self {
            common_name: cn.into(),
            ..Default::default()
        }
    }

    pub fn to_distinguished_name(&self) -> DistinguishedName {
        let mut dn = DistinguishedName::new();
        let mut push = |ty: DnType, v: &str| {
            if !v.trim().is_empty() {
                dn.push(ty, v.trim());
            }
        };
        push(DnType::CountryName, &self.country);
        push(DnType::StateOrProvinceName, &self.state);
        push(DnType::LocalityName, &self.locality);
        push(DnType::OrganizationName, &self.organization);
        push(DnType::OrganizationalUnitName, &self.organizational_unit);
        push(DnType::CommonName, &self.common_name);
        dn
    }
}

/// Apply a leaf profile to certificate params (everything except names, dates and serial).
pub(crate) fn apply_leaf_profile(
    params: &mut CertificateParams,
    profile: Profile,
    key_is_rsa: bool,
) {
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = profile.key_usages(key_is_rsa);
    params.extended_key_usages = profile.extended_key_usages();
    params.use_authority_key_identifier_extension = true;
}
