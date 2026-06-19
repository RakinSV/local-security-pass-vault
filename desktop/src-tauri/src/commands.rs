use crate::error::AppError;
use crate::state::AppState;
use core_vault::models::ItemPayload;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};
use uuid::Uuid;

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatusDto {
    pub is_locked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSummaryDto {
    pub id: String,
    pub item_type: String,
    pub title: String,
    pub folder_id: Option<String>,
    pub favorite: bool,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: String,
    pub item_type: String,
    pub title: String,
    pub payload: serde_json::Value,
    pub folder_id: Option<String>,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn vault_status(state: State<'_, AppState>) -> Result<VaultStatusDto, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    Ok(VaultStatusDto { is_locked: guard.is_none() })
}

#[tauri::command]
pub async fn get_default_vault_dir(app: tauri::AppHandle) -> Result<String, AppError> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("vault").to_string_lossy().into_owned())
        .map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
pub async fn create_vault(
    state: State<'_, AppState>,
    dir_path: String,
    password: String,
    hint: Option<String>,
) -> Result<(), AppError> {
    let dir = PathBuf::from(&dir_path);
    // Vault::create saves internally.
    let vault = core_vault::Vault::create(&dir, password.as_bytes(), hint)?;
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    *guard = Some(vault);
    let mut dir_guard = state.vault_dir.lock().map_err(|_| AppError::LockPoisoned)?;
    *dir_guard = Some(dir);
    Ok(())
}

#[tauri::command]
pub async fn open_vault(
    state: State<'_, AppState>,
    dir_path: String,
    password: String,
) -> Result<(), AppError> {
    let dir = PathBuf::from(&dir_path);
    let vault = core_vault::Vault::open(&dir, password.as_bytes())?;

    // После успешного unlock — сохраняем Vault Key в OS Keychain для quick-unlock.
    // Non-fatal: если Keychain недоступен (headless, CI), игнорируем.
    let vault_uuid = vault.vault_id_str();
    if let Err(e) = crate::keychain::store_vault_key(&vault_uuid, vault.vault_key_bytes()) {
        eprintln!("keychain store failed (non-fatal): {e}");
    }

    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    *guard = Some(vault);
    let mut dir_guard = state.vault_dir.lock().map_err(|_| AppError::LockPoisoned)?;
    *dir_guard = Some(dir);
    Ok(())
}

#[tauri::command]
pub async fn lock_vault(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    // Удаляем Vault Key из OS Keychain перед drop vault.
    if let Some(vault) = guard.as_ref() {
        crate::keychain::delete_vault_key(&vault.vault_id_str());
    }
    *guard = None; // Drop triggers memzero via Key::drop
    Ok(())
}

#[tauri::command]
pub async fn list_items(state: State<'_, AppState>) -> Result<Vec<ItemSummaryDto>, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let items = vault.list_items()?;
    Ok(items
        .into_iter()
        .map(|i| ItemSummaryDto {
            id: i.id.to_string(),
            item_type: i.item_type.as_str().to_string(),
            title: i.title,
            folder_id: i.folder_id.map(|u| u.to_string()),
            favorite: i.favorite,
            updated_at: i.updated_at,
        })
        .collect())
}

#[tauri::command]
pub async fn get_item(state: State<'_, AppState>, id: String) -> Result<ItemDto, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    let item = vault.get_item(&uuid)?.ok_or(AppError::NotFound)?;
    let payload = serde_json::to_value(&item.payload)?;
    Ok(ItemDto {
        id: item.id.to_string(),
        item_type: item.item_type.as_str().to_string(),
        title: item.title,
        payload,
        folder_id: item.folder_id.map(|u| u.to_string()),
        favorite: item.favorite,
        created_at: item.created_at,
        updated_at: item.updated_at,
    })
}

#[tauri::command]
pub async fn create_item(
    state: State<'_, AppState>,
    title: String,
    payload: serde_json::Value,
    folder_id: Option<String>,
    favorite: bool,
) -> Result<String, AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let payload: ItemPayload = serde_json::from_value(payload)?;
    let folder_uuid = folder_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()?;
    let id = vault.add_item(&title, payload, folder_uuid, favorite)?;
    vault.save()?;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn update_item(
    state: State<'_, AppState>,
    id: String,
    title: String,
    payload: serde_json::Value,
    folder_id: Option<String>,
    favorite: bool,
) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    let mut item = vault.get_item(&uuid)?.ok_or(AppError::NotFound)?;
    item.title = title;
    item.payload = serde_json::from_value(payload)?;
    item.folder_id = folder_id.map(|s| Uuid::parse_str(&s)).transpose()?;
    item.favorite = favorite;
    vault.update_item(item)?;
    vault.save()?;
    Ok(())
}

#[tauri::command]
pub async fn delete_item(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    vault.delete_item(&uuid)?;
    vault.save()?;
    Ok(())
}

#[tauri::command]
pub async fn change_master_password(
    state: State<'_, AppState>,
    old_password: String,
    new_password: String,
) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    vault.change_master_password(old_password.as_bytes(), new_password.as_bytes())?;
    Ok(())
}

/// Returns the Ed25519 public key (hex) so the browser extension can verify pipe responses.
#[tauri::command]
pub async fn get_signing_public_key(state: State<'_, AppState>) -> Result<String, AppError> {
    state
        .sign_pk_hex
        .lock()
        .map_err(|_| AppError::LockPoisoned)?
        .clone()
        .ok_or(AppError::Other("signing key not ready".into()))
}

// ── Browser profiles ──────────────────────────────────────────────────────────

/// Returns all Chrome profiles that have ever connected to VaultPass, sorted newest-first.
#[tauri::command]
pub async fn get_profiles(
    app: tauri::AppHandle,
) -> Result<Vec<crate::profile_registry::Profile>, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(crate::profile_registry::list(&data_dir))
}

/// Set (or clear) a user-defined display name for a profile.
#[tauri::command]
pub async fn set_profile_name(
    app: tauri::AppHandle,
    profile_id: String,
    name: Option<String>,
) -> Result<(), AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    crate::profile_registry::set_name(&data_dir, &profile_id, name);
    Ok(())
}

// ── Browser integration ────────────────────────────────────────────────────────

/// Returns the current list of registered extension IDs (Chrome + Firefox).
#[tauri::command]
pub async fn get_browser_integrations(
    app: tauri::AppHandle,
) -> Result<crate::browser_integration::BrowserConfig, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(crate::browser_integration::load(&data_dir))
}

/// Saves the extension ID lists, writes the native messaging manifest JSON,
/// and registers it in the OS (registry on Windows, manifest files on Linux/Mac).
/// Returns the absolute path to the native host binary.
#[tauri::command]
pub async fn save_browser_integrations(
    app: tauri::AppHandle,
    chrome_ids: Vec<String>,
    firefox_ids: Vec<String>,
) -> Result<String, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    let config = crate::browser_integration::BrowserConfig { chrome_ids, firefox_ids };
    crate::browser_integration::install(&data_dir, &config).map_err(AppError::Other)
}

/// Returns the absolute path to the native host binary if it can be found, or null.
#[tauri::command]
pub async fn get_native_host_path() -> Option<String> {
    crate::browser_integration::find_native_host_binary()
        .map(|p| p.to_string_lossy().into_owned())
}

// ── CSV import ─────────────────────────────────────────────────────────────────

/// Parses a Chrome or Firefox CSV export and returns the preview rows.
#[tauri::command]
pub async fn parse_import_csv(
    content: String,
) -> Result<Vec<crate::csv_import::ImportRow>, AppError> {
    crate::csv_import::parse(&content).map_err(AppError::Other)
}

/// Inserts the given rows into the vault as Login items.
/// Returns the number of items imported.
#[tauri::command]
pub async fn import_items_from_csv(
    state: State<'_, AppState>,
    items: Vec<crate::csv_import::ImportRow>,
) -> Result<usize, AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;

    let count = items.len();
    for row in items {
        let payload = core_vault::models::ItemPayload::Login {
            url: row.url,
            username: row.username,
            password: row.password,
            totp_secret: None,
            notes: None,
            custom_fields: vec![],
            password_history: vec![],
        };
        vault.add_item(&row.title, payload, None, false)?;
    }
    if count > 0 {
        vault.save()?;
    }
    Ok(count)
}

// ── Backup (BIP-39 + BLAKE3, ADR-003) ─────────────────────────────────────────

/// Generates a 24-word BIP-39 mnemonic (256-bit entropy) for backup encryption.
/// Show to user ONCE — VaultPass never stores the mnemonic on disk.
#[tauri::command]
pub async fn generate_seed_phrase() -> Result<String, AppError> {
    Ok(core_vault::Vault::generate_backup_phrase()?)
}

/// Returns true if the phrase is a valid BIP-39 mnemonic (correct words + checksum).
#[tauri::command]
pub async fn validate_seed_phrase(phrase: String) -> bool {
    core_vault::backup::validate_mnemonic(&phrase)
}

/// Exports the vault to an encrypted backup file (v2 format, `.vbk`).
/// The vault must be unlocked. `seed_phrase` must be the 24-word BIP-39 mnemonic.
#[tauri::command]
pub async fn export_backup(
    state: State<'_, AppState>,
    backup_path: String,
    seed_phrase: String,
) -> Result<(), AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    vault.export_backup(std::path::Path::new(&backup_path), &seed_phrase)?;
    Ok(())
}

/// Restores a vault from a backup file (supports v1 and v2 formats).
/// The current vault does not need to be unlocked.
#[tauri::command]
pub async fn restore_backup(
    backup_path: String,
    dest_dir: String,
    seed_phrase: String,
) -> Result<(), AppError> {
    core_vault::Vault::restore_from_backup(
        std::path::Path::new(&backup_path),
        std::path::Path::new(&dest_dir),
        &seed_phrase,
    )?;
    Ok(())
}

// ── OS Keychain ────────────────────────────────────────────────────────────────

/// Returns true if a Vault Key is stored in the OS Keychain for `vault_uuid`.
/// Used by UI to decide whether to show the quick-unlock option.
#[tauri::command]
pub async fn keychain_has_key(vault_uuid: String) -> bool {
    crate::keychain::has_vault_key(&vault_uuid)
}

/// Removes the Vault Key for `vault_uuid` from the OS Keychain.
/// Useful for "disable quick unlock" setting.
#[tauri::command]
pub async fn keychain_delete_key(vault_uuid: String) -> Result<(), AppError> {
    crate::keychain::delete_vault_key(&vault_uuid);
    Ok(())
}
