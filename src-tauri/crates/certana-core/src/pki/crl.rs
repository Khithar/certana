// SPDX-License-Identifier: GPL-3.0-or-later
//! Certificate revocation lists.

use rcgen::{
    CertificateRevocationListParams, KeyIdMethod, RevocationReason, RevokedCertParams, SerialNumber,
};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use super::ca::issuer_from;
use super::keys::PrivateKey;
use crate::{CertanaError, Result};

/// RFC 5280 reason codes offered by Certana (no `certificateHold` flow in v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    Unspecified,
    KeyCompromise,
    CaCompromise,
    AffiliationChanged,
    Superseded,
    CessationOfOperation,
    PrivilegeWithdrawn,
}

impl Reason {
    pub fn all() -> &'static [Reason] {
        use Reason::*;
        &[
            Unspecified,
            KeyCompromise,
            CaCompromise,
            AffiliationChanged,
            Superseded,
            CessationOfOperation,
            PrivilegeWithdrawn,
        ]
    }

    fn to_rcgen(self) -> RevocationReason {
        match self {
            Reason::Unspecified => RevocationReason::Unspecified,
            Reason::KeyCompromise => RevocationReason::KeyCompromise,
            Reason::CaCompromise => RevocationReason::CaCompromise,
            Reason::AffiliationChanged => RevocationReason::AffiliationChanged,
            Reason::Superseded => RevocationReason::Superseded,
            Reason::CessationOfOperation => RevocationReason::CessationOfOperation,
            Reason::PrivilegeWithdrawn => RevocationReason::PrivilegeWithdrawn,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedEntry {
    pub serial_hex: String,
    pub revoked_at: OffsetDateTime,
    pub reason: Reason,
    pub invalidity_date: Option<OffsetDateTime>,
}

pub struct CrlOutput {
    pub der: Vec<u8>,
    pub pem: String,
    pub this_update: OffsetDateTime,
    pub next_update: OffsetDateTime,
}

/// Build and sign a CRL for the CA identified by `issuer_cert_pem`/`issuer_key`.
pub fn build_crl(
    issuer_cert_pem: &str,
    issuer_key: &PrivateKey,
    crl_number: u64,
    validity_days: u32,
    revoked: &[RevokedEntry],
) -> Result<CrlOutput> {
    let this_update = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("valid")
        - Duration::minutes(1);
    let next_update = this_update + Duration::days(i64::from(validity_days.max(1)));
    let revoked_certs = revoked
        .iter()
        .map(|r| {
            let bytes = hex::decode(&r.serial_hex)
                .map_err(|_| CertanaError::Invalid(format!("bad serial {}", r.serial_hex)))?;
            Ok(RevokedCertParams {
                serial_number: SerialNumber::from_slice(&bytes),
                revocation_time: r.revoked_at,
                reason_code: Some(r.reason.to_rcgen()),
                invalidity_date: r.invalidity_date,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let params = CertificateRevocationListParams {
        this_update,
        next_update,
        crl_number: SerialNumber::from_slice(&crl_number.to_be_bytes()),
        issuing_distribution_point: None,
        revoked_certs,
        key_identifier_method: KeyIdMethod::Sha256,
    };
    let issuer = issuer_from(issuer_cert_pem, issuer_key.key_pair()?)?;
    let crl = params
        .signed_by(&issuer)
        .map_err(|e| CertanaError::Pki(format!("CRL signing failed: {e}")))?;
    Ok(CrlOutput {
        der: crl.der().to_vec(),
        pem: crl
            .pem()
            .map_err(|e| CertanaError::Pki(format!("CRL encoding failed: {e}")))?,
        this_update,
        next_update,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pki::issue::tests::test_ca;
    use x509_parser::prelude::*;

    #[test]
    fn crl_lists_revoked_serials() {
        let (_root, inter) = test_ca();
        let entry = RevokedEntry {
            serial_hex: "0123456789abcdef0123456789abcdef".into(),
            revoked_at: OffsetDateTime::now_utc().replace_nanosecond(0).unwrap(),
            reason: Reason::KeyCompromise,
            invalidity_date: None,
        };
        let out = build_crl(&inter.certificate_pem, &inter.key, 7, 7, &[entry]).unwrap();
        let (_, crl) = CertificateRevocationList::from_der(&out.der).unwrap();
        assert_eq!(crl.iter_revoked_certificates().count(), 1);
        let rc = crl.iter_revoked_certificates().next().unwrap();
        assert_eq!(
            rc.raw_serial_as_string().replace(':', ""),
            "0123456789abcdef0123456789abcdef"
        );
        assert_eq!(crl.crl_number().map(|n| n.to_string()), Some("7".into()));
        assert!(out.pem.starts_with("-----BEGIN X509 CRL-----"));
    }
}
