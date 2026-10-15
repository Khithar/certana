// SPDX-License-Identifier: GPL-3.0-or-later
//! Row types. They are also the DTOs the Tauri commands return (serde), minus key material:
//! encrypted keys are never serialised to the frontend (`#[serde(skip_serializing)]`).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::pki::crl::Reason;
use crate::pki::keys::KeyAlgorithm;
use crate::pki::profiles::{Profile, San};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaKind {
    Root,
    Intermediate,
}

impl CaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CaKind::Root => "root",
            CaKind::Intermediate => "intermediate",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "root" => Some(CaKind::Root),
            "intermediate" => Some(CaKind::Intermediate),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaRecord {
    pub id: i64,
    pub kind: CaKind,
    pub name: String,
    pub cert_pem: String,
    pub cert_sha256: String,
    pub subject: String,
    #[serde(with = "time::serde::rfc3339")]
    pub not_before: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub not_after: OffsetDateTime,
    pub key_algo: KeyAlgorithm,
    /// PBES2-encrypted PEM; `None` for an offline root. Never sent to the UI.
    #[serde(skip_serializing)]
    pub encrypted_key_pem: Option<String>,
    /// Whether a key is stored at all (what the UI gets instead of the key).
    pub has_key: bool,
    pub parent_ca_id: Option<i64>,
    pub crl_url: Option<String>,
    pub aia_url: Option<String>,
    pub crl_number: i64,
    pub crl_days: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Everything needed to insert a CA; id/created_at are assigned by the database.
#[derive(Debug, Clone)]
pub struct NewCa {
    pub kind: CaKind,
    pub name: String,
    pub cert_pem: String,
    pub key_algo: KeyAlgorithm,
    pub encrypted_key_pem: Option<String>,
    pub parent_ca_id: Option<i64>,
    pub crl_url: Option<String>,
    pub aia_url: Option<String>,
    pub crl_days: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CertStatus {
    Valid,
    Revoked,
    Expired,
}

impl CertStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CertStatus::Valid => "valid",
            CertStatus::Revoked => "revoked",
            CertStatus::Expired => "expired",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "valid" => Some(CertStatus::Valid),
            "revoked" => Some(CertStatus::Revoked),
            "expired" => Some(CertStatus::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRecord {
    pub id: i64,
    pub ca_id: i64,
    pub serial_hex: String,
    pub profile: Profile,
    pub subject: String,
    pub sans: Vec<San>,
    #[serde(with = "time::serde::rfc3339")]
    pub not_before: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub not_after: OffsetDateTime,
    pub cert_pem: String,
    #[serde(skip_serializing)]
    pub encrypted_key_pem: Option<String>,
    pub has_key: bool,
    pub csr_pem: Option<String>,
    pub status: CertStatus,
    pub notes: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct NewCertificate {
    pub ca_id: i64,
    pub serial_hex: String,
    pub profile: Profile,
    pub sans: Vec<San>,
    pub cert_pem: String,
    pub encrypted_key_pem: Option<String>,
    pub csr_pem: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRecord {
    pub cert_id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub revoked_at: OffsetDateTime,
    pub reason: Reason,
    #[serde(with = "time::serde::rfc3339::option")]
    pub invalidity_date: Option<OffsetDateTime>,
    pub comment: String,
    pub first_crl_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrlRecord {
    pub id: i64,
    pub ca_id: i64,
    pub crl_number: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub this_update: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub next_update: OffsetDateTime,
    #[serde(skip_serializing)]
    pub crl_der: Vec<u8>,
    pub exported_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub ts: OffsetDateTime,
    pub actor: String,
    pub host: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub details: serde_json::Value,
}

/// Filter for certificate listings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CertificateFilter {
    pub ca_id: Option<i64>,
    pub profile: Option<Profile>,
    pub status: Option<CertStatus>,
    /// Case-insensitive substring match on subject, SANs and serial.
    pub search: Option<String>,
    /// Only certificates expiring within this many days (and not yet expired/revoked).
    pub expiring_within_days: Option<i64>,
}

pub(crate) fn enum_str<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(String::from))
        .expect("unit enum serialises to a string")
}

pub(crate) fn enum_parse<T: serde::de::DeserializeOwned>(s: &str) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
