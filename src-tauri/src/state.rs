// SPDX-License-Identifier: GPL-3.0-or-later
//! Process-wide state: the opened database and the unlocked issuing-CA session.
//! Key material lives only here, behind a mutex, and is dropped (zeroized) on lock or idle timeout.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use certana_core::db::Database;
use certana_core::db::settings::keys as setting_keys;
use certana_core::pki::keys::PrivateKey;
use secrecy::SecretString;

use crate::commands::error::CommandError;

pub const DEFAULT_AUTO_LOCK_MINUTES: u64 = 15;

/// An unlocked issuing CA. The passphrase is kept so leaf keys can be wrapped with it.
pub struct Session {
    pub ca_id: i64,
    pub key: PrivateKey,
    pub passphrase: SecretString,
    pub last_used: Instant,
}

#[derive(Default)]
pub struct Inner {
    pub db: Mutex<Option<Database>>,
    pub session: Mutex<Option<Session>>,
}

#[derive(Clone, Default)]
pub struct AppState(pub Arc<Inner>);

impl Inner {
    pub fn db(&self) -> Result<MutexGuard<'_, Option<Database>>, CommandError> {
        self.db
            .lock()
            .map_err(|_| CommandError::internal("database mutex poisoned"))
    }

    /// Run `f` with the open database or fail with `no-database`.
    pub fn with_db<T>(
        &self,
        f: impl FnOnce(&Database) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        let guard = self.db()?;
        let db = guard
            .as_ref()
            .ok_or_else(|| CommandError::new("no-database", "no database is open"))?;
        f(db)
    }

    pub fn auto_lock_timeout(&self) -> Duration {
        let minutes = self
            .with_db(|db| {
                Ok(db
                    .get_setting(setting_keys::AUTO_LOCK_MINUTES)
                    .ok()
                    .flatten())
            })
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_AUTO_LOCK_MINUTES);
        Duration::from_secs(minutes.max(1) * 60)
    }

    /// Drop the session if it has been idle for longer than the configured timeout.
    pub fn expire_session_if_idle(&self) {
        let timeout = self.auto_lock_timeout();
        if let Ok(mut s) = self.session.lock()
            && s.as_ref()
                .is_some_and(|sess| sess.last_used.elapsed() > timeout)
        {
            tracing::info!("auto-locking issuing CA after idle timeout");
            *s = None;
        }
    }

    /// Access the unlocked session for `ca_id`, refreshing its idle timer.
    pub fn with_session<T>(
        &self,
        ca_id: i64,
        f: impl FnOnce(&Session) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        self.expire_session_if_idle();
        let mut guard = self
            .session
            .lock()
            .map_err(|_| CommandError::internal("session mutex poisoned"))?;
        let sess = guard.as_mut().filter(|s| s.ca_id == ca_id).ok_or_else(|| {
            CommandError::new("locked", "the issuing CA is locked – unlock it first")
        })?;
        sess.last_used = Instant::now();
        f(sess)
    }

    pub fn lock_session(&self) {
        if let Ok(mut s) = self.session.lock() {
            *s = None;
        }
    }
}
