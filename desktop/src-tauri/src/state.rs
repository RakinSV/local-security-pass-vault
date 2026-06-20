use crate::ed25519_key;
use core_vault::Vault;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
use std::time::Instant;

pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
    pub vault_dir: Mutex<Option<PathBuf>>,
    /// Ed25519 secret key for signing pipe responses (loaded at startup).
    pub sign_sk: Mutex<Option<ed25519_key::SecretKey>>,
    /// Ed25519 public key (hex) exposed via a Tauri command so the extension can pair.
    pub sign_pk_hex: Mutex<Option<String>>,
    /// Seconds of idle before auto-lock. 0 = disabled. Default: 300 (5 min).
    pub auto_lock_secs: AtomicU64,
    /// Lock vault when window is hidden/minimized.
    pub lock_on_minimize: AtomicBool,
    /// Timestamp of last user activity — reset by activity_ping and open_vault.
    pub last_activity: Mutex<Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            vault: Mutex::new(None),
            vault_dir: Mutex::new(None),
            sign_sk: Mutex::new(None),
            sign_pk_hex: Mutex::new(None),
            auto_lock_secs: AtomicU64::new(300),
            lock_on_minimize: AtomicBool::new(false),
            last_activity: Mutex::new(Instant::now()),
        }
    }
}

impl AppState {
    pub fn reset_activity(&self) {
        if let Ok(mut t) = self.last_activity.lock() {
            *t = Instant::now();
        }
    }

    pub fn idle_secs(&self) -> u64 {
        self.last_activity
            .lock()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn auto_lock_secs(&self) -> u64 {
        self.auto_lock_secs.load(Ordering::Relaxed)
    }

    pub fn lock_on_minimize(&self) -> bool {
        self.lock_on_minimize.load(Ordering::Relaxed)
    }
}
