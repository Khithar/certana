// SPDX-License-Identifier: GPL-3.0-or-later
use certana_core::CertanaError;
use serde::Serialize;

/// Error shape delivered to the frontend. `code` is stable for i18n, `message` is developer text.
#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid", message)
    }
}

impl From<CertanaError> for CommandError {
    fn from(e: CertanaError) -> Self {
        let code = match &e {
            CertanaError::Pki(_) => "pki",
            CertanaError::Vault(_) => "vault",
            CertanaError::Db(_) | CertanaError::Migration(_) => "db",
            CertanaError::Io(_) => "io",
            CertanaError::Invalid(_) => "invalid",
        };
        Self::new(code, e.to_string())
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::new("io", e.to_string())
    }
}

pub type CmdResult<T> = Result<T, CommandError>;
