use crate::ed25519_key;
use core_vault::Vault;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
    pub vault_dir: Mutex<Option<PathBuf>>,
    /// Ed25519 secret key for signing pipe responses (loaded at startup).
    pub sign_sk: Mutex<Option<ed25519_key::SecretKey>>,
    /// Ed25519 public key (hex) exposed via a Tauri command so the extension can pair.
    pub sign_pk_hex: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            vault: Mutex::new(None),
            vault_dir: Mutex::new(None),
            sign_sk: Mutex::new(None),
            sign_pk_hex: Mutex::new(None),
        }
    }
}
