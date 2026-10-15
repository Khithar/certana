// SPDX-License-Identifier: GPL-3.0-or-later
//! End-to-end persistence flow: CA → certificate → revoke → CRL → audit.

use certana_core::db::models::{CaKind, CertStatus, CertificateFilter, NewCa, NewCertificate};
use certana_core::db::settings::keys as setting_keys;
use certana_core::db::{Database, OpenOptions};
use certana_core::pki::ca::{CaSpec, DistributionUrls, create_intermediate, create_root};
use certana_core::pki::crl::{Reason, RevokedEntry, build_crl};
use certana_core::pki::issue::{IssueSpec, issue_with_new_key};
use certana_core::pki::keys::KeyAlgorithm;
use certana_core::pki::profiles::{Profile, San, Subject};
use certana_core::vault::keywrap::{decrypt_key_pem, encrypt_key_pem};
use secrecy::SecretString;

fn ca_spec(cn: &str, path_len: u8) -> CaSpec {
    CaSpec {
        subject: Subject::with_common_name(cn),
        algorithm: KeyAlgorithm::EcdsaP256,
        validity_days: 3650,
        path_len,
        urls: DistributionUrls::default(),
    }
}

#[test]
fn full_flow() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("flow.db"), OpenOptions::default()).unwrap();
    let pass = SecretString::from("flow-test-passphrase-123".to_string());

    // Offline root: certificate only, no key in the DB.
    let root = create_root(&ca_spec("Flow Root", 1)).unwrap();
    let root_row = db
        .insert_ca(&NewCa {
            kind: CaKind::Root,
            name: "Flow Root".into(),
            cert_pem: root.certificate_pem.clone(),
            key_algo: KeyAlgorithm::EcdsaP256,
            encrypted_key_pem: None,
            parent_ca_id: None,
            crl_url: None,
            aia_url: None,
            crl_days: 365,
        })
        .unwrap();
    assert!(!root_row.has_key);
    assert_eq!(root_row.kind, CaKind::Root);

    // Issuing CA with encrypted key.
    let inter = create_intermediate(
        &ca_spec("Flow Issuing", 0),
        &root.certificate_pem,
        &root.key,
    )
    .unwrap();
    let inter_row = db
        .insert_ca(&NewCa {
            kind: CaKind::Intermediate,
            name: "Flow Issuing".into(),
            cert_pem: inter.certificate_pem.clone(),
            key_algo: KeyAlgorithm::EcdsaP256,
            encrypted_key_pem: Some(encrypt_key_pem(&inter.key, &pass).unwrap()),
            parent_ca_id: Some(root_row.id),
            crl_url: Some("http://pki/issuing.crl".into()),
            aia_url: None,
            crl_days: 7,
        })
        .unwrap();
    assert!(inter_row.has_key);
    let json = serde_json::to_value(&inter_row).unwrap();
    assert!(
        json.get("encrypted_key_pem").is_none(),
        "encrypted key must not be serialised"
    );
    assert_eq!(
        db.ca_ancestors(inter_row.id)
            .unwrap()
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        vec![root_row.id]
    );
    let unlocked = decrypt_key_pem(
        inter_row.encrypted_key_pem.as_deref().unwrap(),
        inter_row.key_algo,
        &pass,
    )
    .unwrap();
    assert_eq!(unlocked.pkcs8_der(), inter.key.pkcs8_der());

    // Two leaf certificates.
    let mut ids = Vec::new();
    for (host, profile) in [
        ("www.flow.internal", Profile::Server),
        ("vpn.flow.internal", Profile::Client),
    ] {
        let spec = IssueSpec {
            profile,
            subject: Subject::default(),
            sans: vec![San::Dns(host.into())],
            validity_days: 30,
            urls: DistributionUrls::default(),
        };
        let issued = issue_with_new_key(
            &spec,
            KeyAlgorithm::EcdsaP256,
            &inter.certificate_pem,
            &inter.key,
        )
        .unwrap();
        let row = db
            .insert_certificate(&NewCertificate {
                ca_id: inter_row.id,
                serial_hex: issued.serial_hex.clone(),
                profile,
                sans: spec.sans.clone(),
                cert_pem: issued.certificate_pem.clone(),
                encrypted_key_pem: Some(
                    encrypt_key_pem(issued.key.as_ref().unwrap(), &pass).unwrap(),
                ),
                csr_pem: None,
                notes: "".into(),
            })
            .unwrap();
        assert_eq!(row.status, CertStatus::Valid);
        assert_eq!(row.sans, spec.sans);
        assert!(row.has_key);
        ids.push(row.id);
    }
    assert_eq!(
        db.list_certificates(&CertificateFilter::default())
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        db.list_certificates(&CertificateFilter {
            profile: Some(Profile::Client),
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
    assert_eq!(
        db.list_certificates(&CertificateFilter {
            search: Some("VPN".into()),
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
    assert_eq!(
        db.list_certificates(&CertificateFilter {
            expiring_within_days: Some(60),
            ..Default::default()
        })
        .unwrap()
        .len(),
        2
    );
    assert_eq!(
        db.list_certificates(&CertificateFilter {
            expiring_within_days: Some(10),
            ..Default::default()
        })
        .unwrap()
        .len(),
        0
    );
    let by_serial = db
        .get_certificate_by_serial(
            &db.get_certificate(ids[0])
                .unwrap()
                .unwrap()
                .serial_hex
                .to_uppercase(),
        )
        .unwrap();
    assert!(by_serial.is_some());

    // Duplicate serial is rejected, CA certs are rejected in the leaf table.
    let dup = db.get_certificate(ids[0]).unwrap().unwrap();
    assert!(
        db.insert_certificate(&NewCertificate {
            ca_id: inter_row.id,
            serial_hex: dup.serial_hex.clone(),
            profile: Profile::Server,
            sans: vec![],
            cert_pem: dup.cert_pem.clone(),
            encrypted_key_pem: None,
            csr_pem: None,
            notes: "".into()
        })
        .is_err()
    );
    assert!(
        db.insert_certificate(&NewCertificate {
            ca_id: inter_row.id,
            serial_hex: inter.serial_hex.clone(),
            profile: Profile::Server,
            sans: vec![],
            cert_pem: inter.certificate_pem.clone(),
            encrypted_key_pem: None,
            csr_pem: None,
            notes: "".into()
        })
        .is_err()
    );

    // Revoke one, build and store a CRL.
    let rev = db
        .revoke_certificate(ids[0], Reason::KeyCompromise, None, "laptop stolen")
        .unwrap();
    assert_eq!(
        db.get_certificate(ids[0]).unwrap().unwrap().status,
        CertStatus::Revoked
    );
    assert!(
        db.revoke_certificate(ids[0], Reason::Superseded, None, "")
            .is_err()
    );
    assert_eq!(
        db.list_certificates(&CertificateFilter {
            status: Some(CertStatus::Revoked),
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
    let pending = db.revocations_for_crl(inter_row.id, 7).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].1.reason, Reason::KeyCompromise);
    let number = db.next_crl_number(inter_row.id).unwrap();
    assert_eq!(number, 1);
    let entries: Vec<RevokedEntry> = pending
        .iter()
        .map(|(c, r)| RevokedEntry {
            serial_hex: c.serial_hex.clone(),
            revoked_at: r.revoked_at,
            reason: r.reason,
            invalidity_date: r.invalidity_date,
        })
        .collect();
    let crl = build_crl(
        &inter.certificate_pem,
        &inter.key,
        number as u64,
        7,
        &entries,
    )
    .unwrap();
    let crl_row = db
        .insert_crl(
            inter_row.id,
            number,
            crl.this_update,
            crl.next_update,
            &crl.der,
            Some("/tmp/issuing.crl"),
        )
        .unwrap();
    assert_eq!(db.latest_crl(inter_row.id).unwrap().unwrap().id, crl_row.id);
    assert_eq!(
        db.get_revocation(ids[0]).unwrap().unwrap().first_crl_id,
        Some(crl_row.id)
    );
    assert_eq!(rev.cert_id, ids[0]);
    assert_eq!(db.next_crl_number(inter_row.id).unwrap(), 2);

    // Settings and audit.
    db.set_setting(setting_keys::ORGANIZATION, "Flow AG")
        .unwrap();
    db.set_setting(setting_keys::ORGANIZATION, "Flow Holding AG")
        .unwrap();
    assert_eq!(
        db.get_setting(setting_keys::ORGANIZATION)
            .unwrap()
            .as_deref(),
        Some("Flow Holding AG")
    );
    let audit = db.recent_audit(100).unwrap();
    let actions: Vec<&str> = audit.iter().map(|a| a.action.as_str()).collect();
    for a in ["ca.create", "cert.issue", "cert.revoke", "crl.publish"] {
        assert!(
            actions.contains(&a),
            "missing audit action {a}: {actions:?}"
        );
    }
    let csv = db.audit_csv().unwrap();
    assert!(csv.starts_with("id,ts,actor,host,action") && csv.contains("laptop stolen"));
    assert!(
        !csv.contains("BEGIN"),
        "audit must never contain PEM material"
    );

    // Expiry refresh: nothing expires yet.
    assert_eq!(db.refresh_expired().unwrap(), 0);
}

#[test]
fn read_only_rejects_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ro.db");
    Database::open(&path, OpenOptions::default()).unwrap();
    let db = Database::open(
        &path,
        OpenOptions {
            read_only: true,
            force_shared: false,
        },
    )
    .unwrap();
    assert!(db.is_read_only());
    assert!(db.set_setting("x", "y").is_err());
    assert!(db.audit("x", "y", 1, serde_json::json!({})).is_err());
    assert!(db.list_cas().unwrap().is_empty());
}
