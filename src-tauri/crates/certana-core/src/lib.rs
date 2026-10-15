// SPDX-License-Identifier: GPL-3.0-or-later
//! Certana core: PKI operations, encrypted key handling and SQLite persistence.
//! This crate knows nothing about Tauri or the UI.

pub mod config;
pub mod db;
pub mod error;
pub mod pki;
pub mod vault;

pub use error::{CertanaError, Result};
