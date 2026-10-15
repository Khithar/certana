# Certana

**Certana** is a native Linux desktop application for running your own internal Certificate Authority:
create an offline root CA (or import one), operate an intermediate CA, issue server certificates for
Apache/nginx as well as client, S/MIME and code-signing certificates, manage renewals, revoke and
publish CRLs, and keep an eye on expiry dates.

- Built with Rust + Tauri 2, React and SQLite – ships as a small native binary (`.deb`, AppImage, Flatpak; Windows optional).
- Private keys never leave the Rust process; the root key is stored only in an encrypted offline bundle.
- Pure-Rust cryptography (rcgen, pkcs8, p12-keystore) – no OpenSSL dependency, OpenSSL-compatible files.
- UI in English and German.

> Status: pre-release (0.1.0). Core PKI, persistence, commands and UI are implemented and tested; packaging is being finalised.

## Building from source

Linux Mint 22 / Ubuntu 24.04:

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh     # Rust >= 1.90
npm install
npm run tauri dev        # development
npm run tauri build      # .deb / AppImage / .rpm in src-tauri/target/release/bundle/
```

## Tests

```sh
cargo test --manifest-path src-tauri/Cargo.toml --workspace   # core PKI, OpenSSL interop, DB flow, command flow
npm run lint && npm run typecheck
```

## Documentation

Project documentation lives in the Obsidian vault under `docs/vault/` (German).

## License

GPL-3.0-or-later – see [LICENSE](LICENSE).
