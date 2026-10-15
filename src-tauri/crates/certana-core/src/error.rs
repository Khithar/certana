// SPDX-License-Identifier: GPL-3.0-or-later
use thiserror::Error;

/// All errors surfaced by certana-core. Messages never contain key material or passphrases.
#[derive(Debug, Error)]
pub enum CertanaError {
    #[error("PKI error: {0}")]
    Pki(String),
    #[error("key vault error: {0}")]
    Vault(String),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, CertanaError>;
