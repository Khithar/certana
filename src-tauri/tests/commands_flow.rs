// SPDX-License-Identifier: GPL-3.0-or-later
//! End-to-end exercise of the Tauri command layer on a mock app: database → root ceremony →
//! issuing CA → unlock → issue (generated key + CSR) → export → revoke → CRL → renew → dashboard.

use std::process::Command;

use certana_core::pki::crl::Reason;
use certana_core::pki::keys::KeyAlgorithm;
use certana_core::pki::profiles::{Profile, San, Subject};
use certana_lib::commands::ca::*;
use certana_lib::commands::cert::*;
use certana_lib::commands::crl::*;
use certana_lib::commands::dashboard::dashboard_summary;
use certana_lib::commands::db::*;
use certana_lib::commands::settings::*;
use certana_lib::state::AppState;
use tauri::Manager as _;

fn subject(cn: &str) -> Subject {
    Subject {
        common_name: cn.into(),
        organization: "Flow Org".into(),
        country: "CH".into(),
        ..Default::default()
    }
}

#[test]
fn command_flow() {
    let app = tauri::test::mock_app();
    app.manage(AppState::default());
    let state = app.state::<AppState>();
    let dir = tempfile::tempdir().unwrap();
    let p = |n: &str| dir.path().join(n).display().to_string();

    // --- database
    let info = db_open(state.clone(), Some(p("certana.db")), None).unwrap();
    assert!(!info.read_only);
    settings_set(
        state.clone(),
        "subject.organization".into(),
        "Flow Org".into(),
    )
    .unwrap();
    assert_eq!(
        settings_all(state.clone()).unwrap()["subject.organization"],
        "Flow Org"
    );
    settings_set(state.clone(), "crl.export_dir".into(), p("www")).unwrap();

    // --- root ceremony
    let root = tauri::async_runtime::block_on(ca_create_root(
        state.clone(),
        RootCreateRequest {
            name: "Flow Root".into(),
            subject: subject("Flow Root CA"),
            algorithm: KeyAlgorithm::EcdsaP384,
            validity_days: 3650,
            crl_url: Some("http://pki/root.crl".into()),
            aia_url: None,
            passphrase: "root-passphrase-0123456789".into(),
            bundle_path: p("root"),
        },
    ))
    .unwrap();
    assert!(root.bundle_path.ends_with(".certana-root"));
    assert!(std::path::Path::new(&root.bundle_path).exists());
    assert!(!root.ca.has_key, "root key must not be in the DB");
    let check = tauri::async_runtime::block_on(ca_open_root_bundle(
        state.clone(),
        root.bundle_path.clone(),
        "root-passphrase-0123456789".into(),
    ))
    .unwrap();
    assert_eq!(check.matches_ca_id, Some(root.ca.id));
    assert!(
        tauri::async_runtime::block_on(ca_open_root_bundle(
            state.clone(),
            root.bundle_path.clone(),
            "wrong-passphrase-00000000".into()
        ))
        .is_err()
    );

    // --- issuing CA via ceremony
    let inter = tauri::async_runtime::block_on(ca_create_intermediate(
        state.clone(),
        IntermediateCreateRequest {
            name: "Flow Issuing".into(),
            subject: subject("Flow Issuing CA"),
            algorithm: KeyAlgorithm::EcdsaP256,
            validity_days: 1825,
            crl_url: Some("http://pki/issuing.crl".into()),
            aia_url: Some("http://pki/issuing.crt".into()),
            crl_days: 7,
            passphrase: "issuing-passphrase-0123456789".into(),
            root_ca_id: root.ca.id,
            root_bundle_path: root.bundle_path.clone(),
            root_passphrase: "root-passphrase-0123456789".into(),
        },
    ))
    .unwrap();
    assert!(inter.has_key);
    let detail = ca_info(state.clone(), inter.id).unwrap();
    assert_eq!(detail.info.path_len, Some(0));
    assert_eq!(detail.info.crl_urls, vec!["http://pki/root.crl"]);
    assert_eq!(ca_list(state.clone()).unwrap().len(), 2);

    // --- locked: issuing must fail with code `locked`
    let issue_req = |sans: Vec<San>, key: KeySource| IssueRequest {
        ca_id: inter.id,
        profile: Profile::Server,
        subject: Subject::default(),
        sans,
        validity_days: 397,
        key,
        notes: None,
    };
    let err = tauri::async_runtime::block_on(cert_issue(
        state.clone(),
        issue_req(
            vec![San::Dns("a.flow.internal".into())],
            KeySource::Generate {
                algorithm: KeyAlgorithm::EcdsaP256,
            },
        ),
    ))
    .unwrap_err();
    assert_eq!(err.code, "locked");
    assert_eq!(
        tauri::async_runtime::block_on(ca_unlock(
            state.clone(),
            inter.id,
            "nope-nope-nope-nope".into()
        ))
        .unwrap_err()
        .code,
        "wrong-passphrase"
    );
    let s = tauri::async_runtime::block_on(ca_unlock(
        state.clone(),
        inter.id,
        "issuing-passphrase-0123456789".into(),
    ))
    .unwrap();
    assert_eq!(s.unlocked_ca_id, Some(inter.id));

    // --- issue with generated key and from an OpenSSL CSR
    let c1 = tauri::async_runtime::block_on(cert_issue(
        state.clone(),
        issue_req(
            vec![
                San::Dns("www.flow.internal".into()),
                San::Ip("10.0.0.1".parse().unwrap()),
            ],
            KeySource::Generate {
                algorithm: KeyAlgorithm::EcdsaP256,
            },
        ),
    ))
    .unwrap();
    assert!(c1.has_key);
    let csr_pem = {
        let ok = Command::new("openssl")
            .args([
                "req",
                "-new",
                "-newkey",
                "ec",
                "-pkeyopt",
                "ec_paramgen_curve:P-256",
                "-nodes",
                "-keyout",
                &p("srv.key"),
                "-out",
                &p("srv.csr"),
                "-subj",
                "/CN=srv.flow.internal",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            Some(std::fs::read_to_string(p("srv.csr")).unwrap())
        } else {
            None
        }
    };
    let c2 = match &csr_pem {
        Some(csr) => {
            let parsed = cert_parse_csr(csr.clone()).unwrap();
            assert_eq!(parsed.subject.common_name, "srv.flow.internal");
            Some(
                tauri::async_runtime::block_on(cert_issue(
                    state.clone(),
                    issue_req(
                        vec![San::Dns("srv.flow.internal".into())],
                        KeySource::Csr {
                            csr_pem: csr.clone(),
                        },
                    ),
                ))
                .unwrap(),
            )
        }
        None => None,
    };
    if let Some(c2) = &c2 {
        assert!(!c2.has_key);
    }
    assert_eq!(
        cert_list(state.clone(), None).unwrap().len(),
        if c2.is_some() { 2 } else { 1 }
    );

    // --- export package + OpenSSL verification
    let exp = tauri::async_runtime::block_on(cert_export(
        state.clone(),
        ExportRequest {
            cert_id: c1.id,
            dest_dir: p("out"),
            base_name: None,
            include_key: true,
            include_der: true,
            include_p12: true,
            p12_password: Some("p12pw".into()),
            p12_legacy: false,
        },
    ))
    .unwrap();
    assert!(exp.key_written);
    assert!(
        exp.files
            .iter()
            .any(|f| f.ends_with("www.flow.internal.key.pem"))
    );
    assert!(
        exp.files
            .iter()
            .any(|f| f.ends_with("www.flow.internal.fullchain.pem"))
    );
    assert!(
        exp.files
            .iter()
            .any(|f| f.ends_with("www.flow.internal.p12"))
    );
    assert!(exp.snippets.nginx.contains("ssl_certificate_key"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(dir.path().join("out/www.flow.internal.key.pem"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
    ca_export_cert(state.clone(), root.ca.id, p("out/root.pem"), "pem".into()).unwrap();
    if Command::new("openssl").arg("version").output().is_ok() {
        let out = Command::new("openssl")
            .args([
                "verify",
                "-CAfile",
                &p("out/root.pem"),
                "-untrusted",
                &p("out/www.flow.internal.chain.pem"),
                &p("out/www.flow.internal.crt.pem"),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // --- revoke + CRL (exported to crl.export_dir)
    cert_revoke(
        state.clone(),
        c1.id,
        Reason::KeyCompromise,
        Some("test".into()),
    )
    .unwrap();
    let published = crl_publish(state.clone(), inter.id, None).unwrap();
    assert_eq!(published.revoked_count, 1);
    assert!(
        published
            .der_path
            .as_deref()
            .unwrap()
            .ends_with("Flow_Issuing.crl")
    );
    assert!(std::path::Path::new(published.der_path.as_deref().unwrap()).exists());
    assert_eq!(crl_list(state.clone(), inter.id, None).unwrap().len(), 1);
    let got = cert_get(state.clone(), c1.id).unwrap();
    assert_eq!(
        got.revocation.as_ref().unwrap().first_crl_id,
        Some(published.crl.id)
    );

    // --- root CRL via ceremony
    let root_crl = tauri::async_runtime::block_on(crl_publish_root(
        state.clone(),
        root.ca.id,
        root.bundle_path.clone(),
        "root-passphrase-0123456789".into(),
        None,
    ))
    .unwrap();
    assert_eq!(root_crl.revoked_count, 0);

    // --- renew (new key) and dashboard
    if let Some(c2) = &c2 {
        let renewed = tauri::async_runtime::block_on(cert_renew(
            state.clone(),
            RenewRequest {
                cert_id: c2.id,
                validity_days: 100,
                new_key_algorithm: Some(KeyAlgorithm::EcdsaP256),
                revoke_old: true,
            },
        ))
        .unwrap();
        assert!(renewed.has_key);
        assert_eq!(
            cert_get(state.clone(), c2.id).unwrap().record.status,
            certana_core::db::models::CertStatus::Revoked
        );
    }
    let dash = dashboard_summary(state.clone()).unwrap();
    assert_eq!(dash.cas.len(), 2);
    assert!(dash.revoked >= 1);
    assert!(dash.cas.iter().any(|c| c.unlocked));
    assert!(
        dash.recent_audit
            .iter()
            .any(|a| a.action == "root.ceremony")
    );

    // --- lock + backup + close
    assert_eq!(ca_lock(state.clone()).unwrap().unlocked_ca_id, None);
    db_backup(state.clone(), p("backup.db")).unwrap();
    assert!(std::path::Path::new(&p("backup.db")).exists());
    db_close(state.clone()).unwrap();
    assert!(db_info(state.clone()).unwrap().is_none());
}
