-- SPDX-License-Identifier: GPL-3.0-or-later
-- Certana schema v1. Times are RFC 3339 UTC text. Keys are always PBES2-encrypted PEM.

CREATE TABLE settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE cas (
    id                INTEGER PRIMARY KEY,
    kind              TEXT NOT NULL CHECK (kind IN ('root', 'intermediate')),
    name              TEXT NOT NULL UNIQUE,
    cert_pem          TEXT NOT NULL,
    cert_sha256       TEXT NOT NULL UNIQUE,
    subject           TEXT NOT NULL,
    not_before        TEXT NOT NULL,
    not_after         TEXT NOT NULL,
    key_algo          TEXT NOT NULL,
    encrypted_key_pem TEXT,                      -- NULL for an offline root
    parent_ca_id      INTEGER REFERENCES cas(id),
    crl_url           TEXT,
    aia_url           TEXT,
    crl_number        INTEGER NOT NULL DEFAULT 0,
    crl_days          INTEGER NOT NULL DEFAULT 7,
    created_at        TEXT NOT NULL
);

CREATE TABLE certificates (
    id                INTEGER PRIMARY KEY,
    ca_id             INTEGER NOT NULL REFERENCES cas(id),
    serial_hex        TEXT NOT NULL UNIQUE,
    profile           TEXT NOT NULL,
    subject           TEXT NOT NULL,
    sans_json         TEXT NOT NULL,
    not_before        TEXT NOT NULL,
    not_after         TEXT NOT NULL,
    cert_pem          TEXT NOT NULL,
    encrypted_key_pem TEXT,                      -- NULL when issued from a CSR
    csr_pem           TEXT,
    status            TEXT NOT NULL CHECK (status IN ('valid', 'revoked', 'expired')),
    notes             TEXT NOT NULL DEFAULT '',
    created_at        TEXT NOT NULL,
    created_by        TEXT NOT NULL
);
CREATE INDEX idx_certificates_ca     ON certificates(ca_id);
CREATE INDEX idx_certificates_expiry ON certificates(not_after);
CREATE INDEX idx_certificates_status ON certificates(status);

CREATE TABLE revocations (
    cert_id         INTEGER PRIMARY KEY REFERENCES certificates(id),
    revoked_at      TEXT NOT NULL,
    reason          TEXT NOT NULL,
    invalidity_date TEXT,
    comment         TEXT NOT NULL DEFAULT '',
    first_crl_id    INTEGER REFERENCES crls(id)
);

CREATE TABLE crls (
    id            INTEGER PRIMARY KEY,
    ca_id         INTEGER NOT NULL REFERENCES cas(id),
    crl_number    INTEGER NOT NULL,
    this_update   TEXT NOT NULL,
    next_update   TEXT NOT NULL,
    crl_der       BLOB NOT NULL,
    exported_path TEXT,
    UNIQUE (ca_id, crl_number)
);

CREATE TABLE audit_log (
    id           INTEGER PRIMARY KEY,
    ts           TEXT NOT NULL,
    actor        TEXT NOT NULL,
    host         TEXT NOT NULL,
    action       TEXT NOT NULL,
    target_type  TEXT NOT NULL,
    target_id    TEXT NOT NULL,
    details_json TEXT NOT NULL
);
CREATE INDEX idx_audit_ts ON audit_log(ts);
