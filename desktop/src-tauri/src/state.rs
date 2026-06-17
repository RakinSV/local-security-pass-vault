use core_vault::Vault;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
    pub vault_dir: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            vault: Mutex::new(None),
            vault_dir: Mutex::new(None),
        }
    }
}
