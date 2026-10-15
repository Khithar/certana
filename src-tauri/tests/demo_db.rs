// SPDX-License-Identifier: GPL-3.0-or-later
//! Baut eine Demo-Datenbank für README-Screenshots. Läuft nur, wenn CERTANA_DEMO_DIR gesetzt ist;
//! sonst ist der Test ein No-op. Siehe docs/vault/01 - Architektur/Build und Packaging.md.
use certana_core::pki::crl::Reason;
use certana_core::pki::keys::KeyAlgorithm;
use certana_core::pki::profiles::{Profile, San, Subject};
use certana_lib::commands::ca::*;
use certana_lib::commands::cert::*;
use certana_lib::commands::db::*;
use certana_lib::commands::settings::*;
use certana_lib::state::AppState;
use tauri::Manager as _;

fn subj(cn: &str) -> Subject {
    Subject {
        common_name: cn.into(),
        organization: "Example AG".into(),
        country: "CH".into(),
        locality: "Zürich".into(),
        ..Default::default()
    }
}

#[test]
fn build_demo_db() {
    let Ok(dir) = std::env::var("CERTANA_DEMO_DIR") else {
        return;
    };
    let p = |n: &str| format!("{dir}/{n}");
    let app = tauri::test::mock_app();
    app.manage(AppState::default());
    let state = app.state::<AppState>();
    db_open(state.clone(), Some(p("certana.db")), None).unwrap();
    for (k, v) in [
        ("subject.organization", "Example AG"),
        ("subject.country", "CH"),
        ("subject.locality", "Zürich"),
        ("notify.expiry_warn_days", "14"),
        ("export.dir", "/srv/pki/export"),
    ] {
        settings_set(state.clone(), k.into(), v.into()).unwrap();
    }
    let root = tauri::async_runtime::block_on(ca_create_root(
        state.clone(),
        RootCreateRequest {
            name: "Example Root CA 2026".into(),
            subject: subj("Example Root CA 2026"),
            algorithm: KeyAlgorithm::EcdsaP384,
            validity_days: 7300,
            crl_url: None,
            aia_url: None,
            passphrase: "demorootpassphrase2026".into(),
            bundle_path: p("example-root"),
        },
    ))
    .unwrap();
    let inter = tauri::async_runtime::block_on(ca_create_intermediate(
        state.clone(),
        IntermediateCreateRequest {
            name: "Example Issuing CA".into(),
            subject: subj("Example Issuing CA"),
            algorithm: KeyAlgorithm::EcdsaP256,
            validity_days: 3650,
            crl_url: None,
            aia_url: None,
            crl_days: 30,
            passphrase: "demoissuingpassphrase2026".into(),
            root_ca_id: root.ca.id,
            root_bundle_path: root.bundle_path.clone(),
            root_passphrase: "demorootpassphrase2026".into(),
        },
    ))
    .unwrap();
    tauri::async_runtime::block_on(ca_unlock(
        state.clone(),
        inter.id,
        "demoissuingpassphrase2026".into(),
    ))
    .unwrap();
    let issue = |profile: Profile,
                 cn: &str,
                 sans: Vec<San>,
                 days: u32,
                 algo: KeyAlgorithm,
                 notes: Option<&str>| {
        tauri::async_runtime::block_on(cert_issue(
            state.clone(),
            IssueRequest {
                ca_id: inter.id,
                profile,
                subject: subj(cn),
                sans,
                validity_days: days,
                key: KeySource::Generate { algorithm: algo },
                notes: notes.map(String::from),
            },
        ))
        .unwrap()
    };
    let dns = |s: &str| San::Dns(s.into());
    issue(
        Profile::Server,
        "intranet.example.internal",
        vec![
            dns("intranet.example.internal"),
            dns("www.example.internal"),
            San::Ip("10.10.0.20".parse().unwrap()),
        ],
        397,
        KeyAlgorithm::EcdsaP256,
        Some("nginx on srv-web-01, renewed yearly"),
    );
    issue(
        Profile::Server,
        "git.example.internal",
        vec![dns("git.example.internal")],
        397,
        KeyAlgorithm::EcdsaP256,
        None,
    );
    issue(
        Profile::Server,
        "mail.example.internal",
        vec![
            dns("mail.example.internal"),
            dns("imap.example.internal"),
            dns("smtp.example.internal"),
        ],
        365,
        KeyAlgorithm::Rsa2048,
        Some("Postfix/Dovecot – RSA for legacy clients"),
    );
    issue(
        Profile::Server,
        "monitoring.example.internal",
        vec![dns("monitoring.example.internal")],
        12,
        KeyAlgorithm::EcdsaP256,
        None,
    );
    issue(
        Profile::Server,
        "wiki.example.internal",
        vec![dns("wiki.example.internal")],
        5,
        KeyAlgorithm::EcdsaP256,
        None,
    );
    let old = issue(
        Profile::Server,
        "vpn.example.internal",
        vec![dns("vpn.example.internal")],
        90,
        KeyAlgorithm::EcdsaP256,
        None,
    );
    issue(
        Profile::Client,
        "Jane Doe",
        vec![San::Email("jane.doe@example.internal".into())],
        730,
        KeyAlgorithm::EcdsaP256,
        None,
    );
    issue(
        Profile::Smime,
        "Jane Doe",
        vec![San::Email("jane.doe@example.internal".into())],
        730,
        KeyAlgorithm::Rsa2048,
        None,
    );
    issue(
        Profile::CodeSigning,
        "Example Build Server",
        vec![],
        1095,
        KeyAlgorithm::EcdsaP384,
        Some("Signs internal installers"),
    );
    cert_revoke(
        state.clone(),
        old.id,
        Reason::Superseded,
        Some("Replaced by new VPN gateway".into()),
    )
    .unwrap();
    ca_lock(state.clone()).unwrap();
    db_close(state.clone()).unwrap();
}
