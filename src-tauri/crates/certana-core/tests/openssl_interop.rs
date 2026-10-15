// SPDX-License-Identifier: GPL-3.0-or-later
//! Interoperability with the OpenSSL CLI: every artefact Certana writes must be readable and
//! verifiable by `openssl`. Skipped when no `openssl` binary is on PATH.

use std::fs;
use std::path::Path;
use std::process::Command;

use certana_core::pki::ca::{CaSpec, DistributionUrls, create_intermediate, create_root};
use certana_core::pki::crl::{Reason, RevokedEntry, build_crl};
use certana_core::pki::csr::create_csr;
use certana_core::pki::export::{Pkcs12Flavor, build_chain, to_pkcs12};
use certana_core::pki::issue::{IssueSpec, issue_from_csr, issue_with_new_key};
use certana_core::pki::keys::{KeyAlgorithm, PrivateKey};
use certana_core::pki::profiles::{Profile, San, Subject};
use certana_core::vault::keywrap::encrypt_key_pem;
use secrecy::SecretString;

fn openssl_available() -> bool {
    Command::new("openssl")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn openssl(dir: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new("openssl")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn openssl");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn ca_spec(cn: &str, alg: KeyAlgorithm, path_len: u8) -> CaSpec {
    CaSpec {
        subject: Subject {
            common_name: cn.into(),
            organization: "Interop Test".into(),
            country: "CH".into(),
            ..Default::default()
        },
        algorithm: alg,
        validity_days: 3650,
        path_len,
        urls: DistributionUrls {
            crl_url: Some("http://pki.test.internal/ca.crl".into()),
            ca_issuers_url: Some("http://pki.test.internal/ca.crt".into()),
            ocsp_url: None,
        },
    }
}

#[test]
fn full_hierarchy_verifies_with_openssl() {
    if !openssl_available() {
        eprintln!("openssl not found – skipping interop test");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    for (root_alg, inter_alg, leaf_alg) in [
        (
            KeyAlgorithm::EcdsaP384,
            KeyAlgorithm::EcdsaP256,
            KeyAlgorithm::EcdsaP256,
        ),
        (
            KeyAlgorithm::Rsa2048,
            KeyAlgorithm::Rsa2048,
            KeyAlgorithm::Rsa2048,
        ),
        (
            KeyAlgorithm::Ed25519,
            KeyAlgorithm::Ed25519,
            KeyAlgorithm::EcdsaP256,
        ),
    ] {
        let root = create_root(&ca_spec("Interop Root", root_alg, 1)).unwrap();
        let inter = create_intermediate(
            &ca_spec("Interop Issuing", inter_alg, 0),
            &root.certificate_pem,
            &root.key,
        )
        .unwrap();
        let spec = IssueSpec {
            profile: Profile::Server,
            subject: Subject {
                organization: "Interop Test".into(),
                ..Default::default()
            },
            sans: vec![
                San::Dns("www.test.internal".into()),
                San::Dns("test.internal".into()),
                San::Ip("10.1.2.3".parse().unwrap()),
            ],
            validity_days: 397,
            urls: DistributionUrls {
                crl_url: Some("http://pki.test.internal/issuing.crl".into()),
                ca_issuers_url: Some("http://pki.test.internal/issuing.crt".into()),
                ocsp_url: None,
            },
        };
        let leaf = issue_with_new_key(&spec, leaf_alg, &inter.certificate_pem, &inter.key).unwrap();

        fs::write(dir.join("root.pem"), &root.certificate_pem).unwrap();
        fs::write(dir.join("inter.pem"), &inter.certificate_pem).unwrap();
        let files = build_chain(
            &leaf.certificate_pem,
            std::slice::from_ref(&inter.certificate_pem),
        );
        fs::write(dir.join("leaf.pem"), &files.cert).unwrap();
        fs::write(dir.join("fullchain.pem"), &files.fullchain).unwrap();
        fs::write(
            dir.join("leaf.key.pem"),
            leaf.key.as_ref().unwrap().to_pkcs8_pem().as_str(),
        )
        .unwrap();

        // Chain verification exactly as a TLS client would do it.
        let (ok, out) = openssl(
            dir,
            &[
                "verify",
                "-CAfile",
                "root.pem",
                "-untrusted",
                "inter.pem",
                "leaf.pem",
            ],
        );
        assert!(
            ok,
            "openssl verify failed for {root_alg:?}/{inter_alg:?}/{leaf_alg:?}:\n{out}"
        );

        // Extension dump must show what we intended.
        let (ok, text) = openssl(dir, &["x509", "-in", "leaf.pem", "-noout", "-text"]);
        assert!(ok, "{text}");
        for needle in [
            "CA:FALSE",
            "TLS Web Server Authentication",
            "DNS:www.test.internal",
            "IP Address:10.1.2.3",
            "http://pki.test.internal/issuing.crl",
            "CA Issuers - URI:http://pki.test.internal/issuing.crt",
            "Digital Signature",
        ] {
            assert!(text.contains(needle), "missing `{needle}` in:\n{text}");
        }
        let (_, text) = openssl(dir, &["x509", "-in", "inter.pem", "-noout", "-text"]);
        assert!(text.contains("CA:TRUE, pathlen:0"), "{text}");
        assert!(text.contains("Certificate Sign, CRL Sign"), "{text}");

        // Private key + certificate must match according to OpenSSL.
        let (ok, k) = openssl(dir, &["pkey", "-in", "leaf.key.pem", "-pubout"]);
        assert!(ok, "{k}");
        let (ok, c) = openssl(dir, &["x509", "-in", "leaf.pem", "-noout", "-pubkey"]);
        assert!(ok, "{c}");
        assert_eq!(k.trim(), c.trim(), "key/cert mismatch");

        // Encrypted PKCS#8 must open with the standard tooling.
        let pass = SecretString::from("interop-passphrase-123456".to_string());
        fs::write(
            dir.join("enc.key.pem"),
            encrypt_key_pem(&inter.key, &pass).unwrap(),
        )
        .unwrap();
        let (ok, out) = openssl(
            dir,
            &[
                "pkey",
                "-in",
                "enc.key.pem",
                "-passin",
                "pass:interop-passphrase-123456",
                "-noout",
                "-check",
            ],
        );
        assert!(
            ok || out.contains("Key is valid") || out.is_empty(),
            "openssl pkey on encrypted {inter_alg:?} key failed:\n{out}"
        );
        let (ok, _) = openssl(
            dir,
            &[
                "pkey",
                "-in",
                "enc.key.pem",
                "-passin",
                "pass:wrong-passphrase-000000",
                "-noout",
            ],
        );
        assert!(!ok, "wrong passphrase must fail");

        // CRL signed by the intermediate.
        let crl = build_crl(
            &inter.certificate_pem,
            &inter.key,
            42,
            7,
            &[RevokedEntry {
                serial_hex: leaf.serial_hex.clone(),
                revoked_at: time::OffsetDateTime::now_utc(),
                reason: Reason::Superseded,
                invalidity_date: None,
            }],
        )
        .unwrap();
        fs::write(dir.join("issuing.crl"), &crl.der).unwrap();
        let (ok, text) = openssl(
            dir,
            &[
                "crl",
                "-in",
                "issuing.crl",
                "-inform",
                "DER",
                "-noout",
                "-text",
                "-CAfile",
                "inter.pem",
            ],
        );
        assert!(ok, "{text}");
        assert!(text.contains("Superseded"), "{text}");
        assert!(
            text.to_lowercase()
                .contains(&leaf.serial_hex.to_lowercase()),
            "serial missing in CRL:\n{text}"
        );
        let (ok, out) = openssl(
            dir,
            &[
                "verify",
                "-crl_check",
                "-CAfile",
                "root.pem",
                "-untrusted",
                "inter.pem",
                "-CRLfile",
                "issuing.crl",
                "leaf.pem",
            ],
        );
        assert!(
            !ok && out.contains("revoked"),
            "revoked leaf must fail verification:\n{out}"
        );

        // PKCS#12 in both flavours.
        for (flavor, name) in [
            (Pkcs12Flavor::Modern, "modern.p12"),
            (Pkcs12Flavor::Legacy, "legacy.p12"),
        ] {
            let p12 = to_pkcs12(
                "interop",
                leaf.key.as_ref().unwrap(),
                &leaf.certificate_pem,
                std::slice::from_ref(&inter.certificate_pem),
                "p12secret",
                flavor,
            )
            .unwrap();
            fs::write(dir.join(name), p12).unwrap();
            let args: Vec<&str> = if flavor == Pkcs12Flavor::Legacy {
                vec![
                    "pkcs12",
                    "-legacy",
                    "-in",
                    name,
                    "-passin",
                    "pass:p12secret",
                    "-info",
                    "-nodes",
                ]
            } else {
                vec![
                    "pkcs12",
                    "-in",
                    name,
                    "-passin",
                    "pass:p12secret",
                    "-info",
                    "-nodes",
                ]
            };
            let (ok, out) = openssl(dir, &args);
            assert!(ok, "openssl pkcs12 ({name}) failed:\n{out}");
            assert!(
                out.contains("BEGIN PRIVATE KEY") && out.matches("BEGIN CERTIFICATE").count() == 2,
                "{out}"
            );
        }
    }
}

#[test]
fn csr_signed_by_openssl_generated_request() {
    if !openssl_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    // The requester uses OpenSSL and asks for CA:TRUE – which Certana must ignore.
    fs::write(dir.join("req.cnf"), "[req]\ndistinguished_name=dn\nreq_extensions=ext\nprompt=no\n[dn]\nCN=evil.test.internal\n[ext]\nbasicConstraints=CA:TRUE\nkeyUsage=keyCertSign\n").unwrap();
    let (ok, out) = openssl(
        dir,
        &[
            "req", "-new", "-newkey", "rsa:2048", "-nodes", "-keyout", "req.key", "-out",
            "req.csr", "-config", "req.cnf",
        ],
    );
    assert!(ok, "{out}");
    let csr_pem = fs::read_to_string(dir.join("req.csr")).unwrap();

    let root = create_root(&ca_spec("CSR Root", KeyAlgorithm::EcdsaP256, 1)).unwrap();
    let inter = create_intermediate(
        &ca_spec("CSR Issuing", KeyAlgorithm::EcdsaP256, 0),
        &root.certificate_pem,
        &root.key,
    )
    .unwrap();
    let spec = IssueSpec {
        profile: Profile::Server,
        subject: Subject::with_common_name("evil.test.internal"),
        sans: vec![San::Dns("evil.test.internal".into())],
        validity_days: 30,
        urls: DistributionUrls::default(),
    };
    let issued = issue_from_csr(&spec, &csr_pem, &inter.certificate_pem, &inter.key).unwrap();
    fs::write(dir.join("leaf.pem"), &issued.certificate_pem).unwrap();
    fs::write(dir.join("root.pem"), &root.certificate_pem).unwrap();
    fs::write(dir.join("inter.pem"), &inter.certificate_pem).unwrap();
    let (ok, text) = openssl(dir, &["x509", "-in", "leaf.pem", "-noout", "-text"]);
    assert!(
        ok && text.contains("CA:FALSE") && !text.contains("Certificate Sign"),
        "{text}"
    );
    let (ok, out) = openssl(
        dir,
        &[
            "verify",
            "-CAfile",
            "root.pem",
            "-untrusted",
            "inter.pem",
            "leaf.pem",
        ],
    );
    assert!(ok, "{out}");
    // And the OpenSSL-generated key matches the issued certificate.
    let (_, k) = openssl(dir, &["pkey", "-in", "req.key", "-pubout"]);
    let (_, c) = openssl(dir, &["x509", "-in", "leaf.pem", "-noout", "-pubkey"]);
    assert_eq!(k.trim(), c.trim());

    // Conversely, a Certana CSR must verify with `openssl req`.
    let key = PrivateKey::generate(KeyAlgorithm::EcdsaP256).unwrap();
    let csr = create_csr(
        &Subject::with_common_name("certana.test.internal"),
        &[San::Dns("certana.test.internal".into())],
        &key,
    )
    .unwrap();
    fs::write(dir.join("certana.csr"), csr).unwrap();
    let (ok, out) = openssl(
        dir,
        &["req", "-in", "certana.csr", "-noout", "-verify", "-text"],
    );
    assert!(ok && out.contains("DNS:certana.test.internal"), "{out}");
}
