use crate::ed25519_key;
use crate::error::AppError;
use core_vault::Vault;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex, MutexGuard,
};
use std::time::Instant;

// ── Windows DPAPI: RAII guard для доступа к vault ────────────────────────────
//
// При захвате: расшифровывает все ключи vault (`CryptUnprotectMemory`).
// При drop: шифрует обратно (`CryptProtectMemory`).
// На не-Windows: просто оборачивает MutexGuard без дополнительных операций.
//
// Все Tauri-команды, работающие с vault, должны использовать
// `state.lock_vault_for_use()` вместо `state.vault.lock()`.

pub struct VaultLockGuard<'a> {
    pub inner: MutexGuard<'a, Option<Vault>>,
}

impl<'a> std::ops::Deref for VaultLockGuard<'a> {
    type Target = Option<Vault>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> std::ops::DerefMut for VaultLockGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a> Drop for VaultLockGuard<'a> {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(v) = self.inner.as_mut() {
            v.protect_keys();
        }
    }
}

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
    /// Lock vault when the OS screen is locked or screensaver activates.
    pub lock_on_screensaver: AtomicBool,
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
            lock_on_screensaver: AtomicBool::new(true),
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

    pub fn lock_on_screensaver(&self) -> bool {
        self.lock_on_screensaver.load(Ordering::Relaxed)
    }

    /// Захватывает мьютекс vault и подготавливает ключи для использования.
    ///
    /// На Windows: вызывает `CryptUnprotectMemory` перед тем, как вернуть guard.
    /// При drop guard автоматически вызывает `CryptProtectMemory` обратно.
    ///
    /// Использовать вместо `state.vault.lock()` во всех Tauri-командах.
    pub fn lock_vault_for_use(&self) -> Result<VaultLockGuard<'_>, AppError> {
        let mut inner = self
            .vault
            .lock()
            .map_err(|_| AppError::LockPoisoned)?;

        #[cfg(windows)]
        if let Some(v) = inner.as_mut() {
            if !v.unprotect_keys() {
                return Err(AppError::Other(
                    "DPAPI vault key decryption failed — vault may be corrupted in memory"
                        .into(),
                ));
            }
        }

        Ok(VaultLockGuard { inner })
    }
}
