use crate::error::AppError;
use crate::state::AppState;
use core_vault::models::{ItemPayload, PasswordHistoryEntry};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
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
    pub subtitle: Option<String>,
    pub folder_id: Option<String>,
    pub favorite: bool,
    pub updated_at: i64,
    pub source_tag: Option<String>,
}

fn subtitle_for(payload: &ItemPayload) -> Option<String> {
    match payload {
        ItemPayload::Login { url, username, .. } => {
            let domain = url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap_or("")
                .trim_end_matches(':')
                .to_string();
            match (!domain.is_empty(), !username.is_empty()) {
                (true,  true)  => Some(format!("{domain} · {username}")),
                (true,  false) => Some(domain),
                (false, true)  => Some(username.clone()),
                (false, false) => None,
            }
        }
        ItemPayload::Card { cardholder, number, .. } => {
            let last4 = if number.len() >= 4 { &number[number.len() - 4..] } else { number.as_str() };
            if cardholder.is_empty() {
                Some(format!("•••• {last4}"))
            } else {
                Some(format!("{cardholder} · •••• {last4}"))
            }
        }
        ItemPayload::Server { host, username, .. } => match username {
            Some(u) if !u.is_empty() => Some(format!("{u}@{host}")),
            _ => Some(host.clone()),
        },
        ItemPayload::Identity { first_name, last_name, email, .. } => {
            let name_parts: Vec<&str> = [first_name.as_deref(), last_name.as_deref()]
                .iter()
                .filter_map(|s| s.filter(|v| !v.is_empty()))
                .collect();
            if !name_parts.is_empty() {
                Some(name_parts.join(" "))
            } else {
                email.clone()
            }
        }
        ItemPayload::SshKey { public_key, .. } => {
            public_key.as_ref().and_then(|k| {
                let comment = k.split_whitespace().nth(2)?;
                Some(comment[..comment.len().min(30)].to_string())
            })
        }
        ItemPayload::Note { .. } => None,
    }
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
    pub source_tag: Option<String>,
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
    totp_code: Option<String>,
) -> Result<(), AppError> {
    let dir = PathBuf::from(&dir_path);
    let vault = core_vault::Vault::open(&dir, password.as_bytes(), totp_code.as_deref())?;

    // После успешного unlock — сохраняем Vault Key в OS Keychain для quick-unlock.
    // Non-fatal: если Keychain недоступен (headless, CI), игнорируем.
    let vault_uuid = vault.vault_id_str();
    if let Err(e) = crate::keychain::store_vault_key(&vault_uuid, vault.vault_key_bytes()) {
        eprintln!("keychain store failed (non-fatal): {e}");
    }

    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    *guard = Some(vault);
    state.reset_activity();
    let mut dir_guard = state.vault_dir.lock().map_err(|_| AppError::LockPoisoned)?;
    *dir_guard = Some(dir);

    // Auto-purge trash items older than 30 days on every vault open
    {
        const THIRTY_DAYS_SECS: u64 = 30 * 24 * 60 * 60;
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let cutoff = (now.as_secs().saturating_sub(THIRTY_DAYS_SECS)) as i64;
            if let Some(v) = guard.as_ref() {
                let _ = v.purge_old_trash(cutoff);
            }
        }
    }

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
        .map(|i| {
            let subtitle = subtitle_for(&i.payload);
            ItemSummaryDto {
                id: i.id.to_string(),
                item_type: i.item_type.as_str().to_string(),
                title: i.title,
                subtitle,
                folder_id: i.folder_id.map(|u| u.to_string()),
                favorite: i.favorite,
                updated_at: i.updated_at,
                source_tag: i.source_tag,
            }
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
        source_tag: item.source_tag,
    })
}

#[tauri::command]
pub async fn create_item(
    state: State<'_, AppState>,
    title: String,
    payload: serde_json::Value,
    folder_id: Option<String>,
    favorite: bool,
    source_tag: Option<String>,
) -> Result<String, AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let payload: ItemPayload = serde_json::from_value(payload)?;
    let folder_uuid = folder_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()?;
    let id = vault.add_item(&title, payload, folder_uuid, favorite, source_tag)?;
    vault.save()?;
    Ok(id.to_string())
}

/// For Login items: if the password changed, prepend the old password to history (max 10 entries).
/// All other item types pass through unchanged.
fn with_password_history(old: &ItemPayload, mut new: ItemPayload) -> ItemPayload {
    if let ItemPayload::Login { password: old_pw, password_history: old_hist, .. } = old {
        if let ItemPayload::Login { password: new_pw, password_history: hist, .. } = &mut new {
            if old_pw != new_pw && !new_pw.is_empty() {
                let mut merged = old_hist.clone();
                merged.insert(0, PasswordHistoryEntry {
                    password: old_pw.clone(),
                    changed_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                });
                merged.truncate(10);
                *hist = merged;
            }
        }
    }
    new
}

#[tauri::command]
pub async fn update_item(
    state: State<'_, AppState>,
    id: String,
    title: String,
    payload: serde_json::Value,
    folder_id: Option<String>,
    favorite: bool,
    source_tag: Option<String>,
) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    let mut item = vault.get_item(&uuid)?.ok_or(AppError::NotFound)?;
    item.title = title;
    let new_payload: ItemPayload = serde_json::from_value(payload)?;
    item.payload = with_password_history(&item.payload.clone(), new_payload);
    item.folder_id = folder_id.map(|s| Uuid::parse_str(&s)).transpose()?;
    item.favorite = favorite;
    item.source_tag = source_tag;
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
    source_tag: Option<String>,
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
        vault.add_item(&row.title, payload, None, false, source_tag.clone())?;
    }
    if count > 0 {
        vault.save()?;
    }
    Ok(count)
}

/// Returns all distinct non-null source_tag values from live items.
#[tauri::command]
pub async fn list_source_tags(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    Ok(vault.list_source_tags()?)
}

/// Renames (or clears) a source_tag across all live items with that tag.
#[tauri::command]
pub async fn bulk_retag_items(
    state: State<'_, AppState>,
    old_tag: String,
    new_tag: Option<String>,
) -> Result<usize, AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    Ok(vault.update_source_tag_bulk(&old_tag, new_tag.as_deref())?)
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
/// A timestamped copy is also saved to app_data_dir/backups/ (last 7 retained).
#[tauri::command]
pub async fn export_backup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    backup_path: String,
    seed_phrase: String,
) -> Result<(), AppError> {
    {
        let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
        let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
        vault.export_backup(std::path::Path::new(&backup_path), &seed_phrase)?;
    }
    // Non-fatal: save a timestamped copy and rotate (keep 7 newest)
    let _ = auto_save_backup(&app, std::path::Path::new(&backup_path));
    Ok(())
}

fn auto_save_backup(app: &tauri::AppHandle, source: &std::path::Path) -> Result<(), AppError> {
    let backups_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?
        .join("backups");
    std::fs::create_dir_all(&backups_dir)
        .map_err(|e| AppError::Other(e.to_string()))?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = backups_dir.join(format!("lspv_{ts}.vbk"));
    std::fs::copy(source, &dest)
        .map_err(|e| AppError::Other(e.to_string()))?;

    // Keep only the 7 newest auto-saves
    let mut entries: Vec<_> = match std::fs::read_dir(&backups_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("vbk"))
            .collect(),
        Err(_) => return Ok(()),
    };
    entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    for old in entries.iter().take(entries.len().saturating_sub(7)) {
        std::fs::remove_file(old.path()).ok();
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackupEntry {
    pub path: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

/// Lists auto-saved backup copies from app_data_dir/backups/, newest first.
#[tauri::command]
pub async fn list_auto_backups(app: tauri::AppHandle) -> Result<Vec<AutoBackupEntry>, AppError> {
    let backups_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?
        .join("backups");

    if !backups_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries: Vec<AutoBackupEntry> = match std::fs::read_dir(&backups_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("vbk"))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let created_at = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                Some(AutoBackupEntry {
                    path: e.path().to_string_lossy().into_owned(),
                    size_bytes: meta.len(),
                    created_at,
                })
            })
            .collect(),
        Err(_) => vec![],
    };

    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(entries)
}

/// Opens a native file-picker dialog for selecting a .vbk backup file.
#[tauri::command]
pub async fn pick_backup_file() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Select VaultPass backup file")
        .add_filter("VaultPass Backup", &["vbk"])
        .pick_file()
        .await
        .map(|h| h.path().to_string_lossy().into_owned())
}

/// Opens a native save-file dialog for choosing a .vbk export path.
#[tauri::command]
pub async fn pick_backup_save_path() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Save backup as")
        .add_filter("VaultPass Backup", &["vbk"])
        .set_file_name("vault-backup.vbk")
        .save_file()
        .await
        .map(|h| h.path().to_string_lossy().into_owned())
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

// ── Utility ────────────────────────────────────────────────────────────────────

/// Returns a suggested directory path for a new vault based on the app data dir.
/// Path format: <app_data>/vaults/<sanitized_name>
#[tauri::command]
pub async fn suggest_vault_dir(app: tauri::AppHandle, name: String) -> Result<String, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let safe = if safe.is_empty() { "vault".to_string() } else { safe };
    Ok(base.join("vaults").join(safe).to_string_lossy().into_owned())
}

/// Opens a native folder-picker dialog and returns the chosen directory path.
#[tauri::command]
pub async fn pick_folder() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose vault folder")
        .pick_folder()
        .await
        .map(|h| h.path().to_string_lossy().into_owned())
}

/// Opens the project GitHub page in the default system browser.
#[tauri::command]
pub async fn open_github() -> Result<(), AppError> {
    webbrowser::open("https://github.com/RakinSV/local-security-pass-vault")
        .map_err(|e| AppError::Other(e.to_string()))
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

// ── Autostart ──────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
const AUTOSTART_REG_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(target_os = "windows")]
const AUTOSTART_REG_NAME: &str = "LocalSecurityPassVault";

fn get_exe_path() -> Result<String, AppError> {
    std::env::current_exe()
        .map_err(|e| AppError::Other(e.to_string()))
        .map(|p| p.to_string_lossy().into_owned())
}

/// Returns true if the app is registered to launch at OS startup.
#[tauri::command]
pub async fn get_autostart() -> bool {
    get_autostart_impl()
}

/// Enables or disables launching the app at OS startup.
#[tauri::command]
pub async fn set_autostart(enable: bool) -> Result<(), AppError> {
    set_autostart_impl(enable)
}

#[cfg(target_os = "windows")]
fn get_autostart_impl() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(AUTOSTART_REG_KEY)
        .and_then(|k| k.get_value::<String, _>(AUTOSTART_REG_NAME))
        .is_ok()
}

#[cfg(target_os = "windows")]
fn set_autostart_impl(enable: bool) -> Result<(), AppError> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;
    let exe = get_exe_path()?;
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(AUTOSTART_REG_KEY, KEY_SET_VALUE)
        .map_err(|e| AppError::Other(e.to_string()))?;
    if enable {
        key.set_value(AUTOSTART_REG_NAME, &exe)
            .map_err(|e| AppError::Other(e.to_string()))
    } else {
        let _ = key.delete_value(AUTOSTART_REG_NAME);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn get_autostart_impl() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::Path::new(&format!(
        "{}/Library/LaunchAgents/com.lspv.app.plist",
        home
    ))
    .exists()
}

#[cfg(target_os = "macos")]
fn set_autostart_impl(enable: bool) -> Result<(), AppError> {
    let exe  = get_exe_path()?;
    let home = std::env::var("HOME")
        .map_err(|_| AppError::Other("HOME env var not set".into()))?;
    let dir  = format!("{}/Library/LaunchAgents", home);
    let path = format!("{}/com.lspv.app.plist", dir);
    if enable {
        std::fs::create_dir_all(&dir).ok();
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\"><dict>\n\
             <key>Label</key><string>com.lspv.app</string>\n\
             <key>ProgramArguments</key><array><string>{exe}</string></array>\n\
             <key>RunAtLoad</key><true/>\n\
             <key>KeepAlive</key><false/>\n\
             </dict></plist>\n"
        );
        std::fs::write(&path, plist).map_err(|e| AppError::Other(e.to_string()))
    } else {
        let _ = std::fs::remove_file(&path);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn get_autostart_impl() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::Path::new(&format!("{}/.config/autostart/lspv.desktop", home)).exists()
}

#[cfg(target_os = "linux")]
fn set_autostart_impl(enable: bool) -> Result<(), AppError> {
    let exe  = get_exe_path()?;
    let home = std::env::var("HOME")
        .map_err(|_| AppError::Other("HOME env var not set".into()))?;
    let dir  = format!("{}/.config/autostart", home);
    let path = format!("{}/lspv.desktop", dir);
    if enable {
        std::fs::create_dir_all(&dir).ok();
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Local Security Pass Vault\n\
             Exec={exe}\nHidden=false\nNoDisplay=false\n\
             X-GNOME-Autostart-enabled=true\n"
        );
        std::fs::write(&path, content).map_err(|e| AppError::Other(e.to_string()))
    } else {
        let _ = std::fs::remove_file(&path);
        Ok(())
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn get_autostart_impl() -> bool { false }

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn set_autostart_impl(_enable: bool) -> Result<(), AppError> {
    Err(AppError::Other("Autostart not supported on this platform".into()))
}

// ── Auto-lock ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLockSettings {
    pub secs: u64,
    pub lock_on_minimize: bool,
}

/// Returns the current auto-lock settings.
#[tauri::command]
pub async fn get_auto_lock_settings(state: State<'_, AppState>) -> Result<AutoLockSettings, AppError> {
    Ok(AutoLockSettings {
        secs: state.auto_lock_secs.load(Ordering::Relaxed),
        lock_on_minimize: state.lock_on_minimize.load(Ordering::Relaxed),
    })
}

/// Updates the auto-lock timeout and lock-on-minimize flag.
/// `secs = 0` disables auto-lock.
#[tauri::command]
pub async fn set_auto_lock_settings(
    state: State<'_, AppState>,
    secs: u64,
    lock_on_minimize: bool,
) -> Result<(), AppError> {
    state.auto_lock_secs.store(secs, Ordering::Relaxed);
    state.lock_on_minimize.store(lock_on_minimize, Ordering::Relaxed);
    Ok(())
}

/// Called by the frontend on user interaction to reset the idle timer.
#[tauri::command]
pub async fn activity_ping(state: State<'_, AppState>) -> Result<(), AppError> {
    state.reset_activity();
    Ok(())
}

// ── TOTP 2FA ──────────────────────────────────────────────────────────────────

/// Generates the current 6-digit TOTP code from a Base32 secret.
/// Returns the code and the number of seconds until the next code.
#[tauri::command]
pub async fn generate_totp(secret: String) -> Result<core_vault::totp::TotpCode, AppError> {
    core_vault::totp::generate(&secret).map_err(AppError::Other)
}

/// Reads an image from the system clipboard and decodes any QR code found in it.
/// If the QR content is an `otpauth://` URI, extracts and returns the Base32 secret.
/// Returns `{ secret, issuer, account }` on success.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrResult {
    pub secret: String,
    pub issuer: String,
    pub account: String,
    pub raw_uri: String,
}

#[tauri::command]
pub async fn decode_qr_from_clipboard() -> Result<QrResult, AppError> {
    // arboard must run on a thread with clipboard access (not the Tokio executor thread)
    tokio::task::spawn_blocking(|| {
        // 1. Read image from clipboard
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| AppError::Other(format!("clipboard error: {e}")))?;
        let img_data = clipboard.get_image()
            .map_err(|_| AppError::Other(
                "No image found on clipboard. Copy the QR code image first.".into()
            ))?;

        // 2. Convert arboard RGBA bytes → image::GrayImage (luma8) for rqrr
        let rgba = image::RgbaImage::from_raw(
            img_data.width as u32,
            img_data.height as u32,
            img_data.bytes.into_owned(),
        ).ok_or_else(|| AppError::Other("Failed to decode clipboard image.".into()))?;

        let luma = image::DynamicImage::ImageRgba8(rgba).into_luma8();

        // 3. Detect and decode QR grids
        let mut prepared = rqrr::PreparedImage::prepare(luma);
        let grids = prepared.detect_grids();
        if grids.is_empty() {
            return Err(AppError::Other(
                "No QR code found in the clipboard image. \
                 Make sure you copied the QR code image.".into()
            ));
        }

        // Try each grid, return first successful decode
        let mut last_err = String::new();
        for grid in grids {
            match grid.decode() {
                Ok((_meta, content)) => {
                    let content = content.trim();
                    // Parse otpauth:// URI
                    let (secret, issuer, account) =
                        core_vault::totp::parse_otpauth_uri(content)
                            .map_err(|e| AppError::Other(format!(
                                "QR decoded but content is not a TOTP URI: {e}. Content: {content}"
                            )))?;
                    return Ok(QrResult {
                        secret,
                        issuer,
                        account,
                        raw_uri: content.to_string(),
                    });
                }
                Err(e) => last_err = e.to_string(),
            }
        }
        Err(AppError::Other(format!("QR code found but could not be decoded: {last_err}")))
    })
    .await
    .map_err(|e| AppError::Other(format!("clipboard task panicked: {e}")))?
}

// ── Browser Extension Installer ────────────────────────────────────────────────

/// Detects all installed Chromium-based browsers and Firefox, along with their profiles.
#[tauri::command]
pub async fn detect_browsers_for_extension(
) -> Result<Vec<crate::ext_installer::DetectedBrowser>, AppError> {
    Ok(tokio::task::spawn_blocking(crate::ext_installer::detect_browsers)
        .await
        .map_err(|e| AppError::Other(format!("detect task panicked: {e}")))?)
}

/// Installs the LSPV browser extension to the requested browsers / Firefox profiles.
#[tauri::command]
pub async fn install_extension_to_browsers(
    requests: Vec<crate::ext_installer::InstallRequest>,
) -> Result<Vec<crate::ext_installer::InstallResult>, AppError> {
    Ok(tokio::task::spawn_blocking(move || {
        crate::ext_installer::install_extension(&requests)
    })
    .await
    .map_err(|e| AppError::Other(format!("install task panicked: {e}")))?)
}

// ── Keychain vault status ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeychainVaultStatus {
    pub vault_open: bool,
    pub vault_uuid: Option<String>,
    pub has_cached_key: bool,
}

/// Returns whether the currently-open vault has its key cached in the OS Keychain.
/// Used by Settings UI to show quick-unlock status and allow removal.
#[tauri::command]
pub async fn keychain_vault_status(
    state: State<'_, AppState>,
) -> Result<KeychainVaultStatus, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    match guard.as_ref() {
        None => Ok(KeychainVaultStatus {
            vault_open: false,
            vault_uuid: None,
            has_cached_key: false,
        }),
        Some(vault) => {
            let uuid = vault.vault_id_str();
            let has_cached_key = crate::keychain::has_vault_key(&uuid);
            Ok(KeychainVaultStatus {
                vault_open: true,
                vault_uuid: Some(uuid),
                has_cached_key,
            })
        }
    }
}

// ── Trash (soft-delete management) ────────────────────────────────────────────

/// Lists all soft-deleted items (the trash bin).
#[tauri::command]
pub async fn list_deleted_items(state: State<'_, AppState>) -> Result<Vec<ItemSummaryDto>, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let items = vault.list_deleted_items()?;
    Ok(items
        .into_iter()
        .map(|i| {
            let subtitle = subtitle_for(&i.payload);
            ItemSummaryDto {
                id: i.id.to_string(),
                item_type: i.item_type.as_str().to_string(),
                title: i.title,
                subtitle,
                folder_id: i.folder_id.map(|u| u.to_string()),
                favorite: i.favorite,
                updated_at: i.updated_at,
                source_tag: i.source_tag,
            }
        })
        .collect())
}

/// Restores a soft-deleted item back to the vault.
#[tauri::command]
pub async fn restore_item(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    vault.restore_item(&uuid)?;
    vault.save()?;
    Ok(())
}

/// Permanently deletes one item from the trash bin.
#[tauri::command]
pub async fn purge_item(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    vault.purge_item(&uuid)?;
    vault.save()?;
    Ok(())
}

/// Permanently deletes ALL items from the trash bin. Returns count deleted.
#[tauri::command]
pub async fn purge_all_trash(state: State<'_, AppState>) -> Result<usize, AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let n = vault.purge_all_trash()?;
    vault.save()?;
    Ok(n)
}

// ── Folders ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderDto {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub created_at: i64,
}

/// Lists all folders.
#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>) -> Result<Vec<FolderDto>, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let folders = vault.list_folders()?;
    Ok(folders
        .into_iter()
        .map(|f| FolderDto {
            id: f.id.to_string(),
            name: f.name,
            icon: f.icon,
            created_at: f.created_at,
        })
        .collect())
}

/// Creates a new folder. Returns the new folder's UUID string.
#[tauri::command]
pub async fn add_folder(
    state: State<'_, AppState>,
    name: String,
    icon: Option<String>,
) -> Result<String, AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let id = vault.add_folder(&name, None, icon)?;
    vault.save()?;
    Ok(id.to_string())
}

/// Deletes a folder. Items inside get folder_id = NULL (moved to root).
#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    vault.delete_folder(&uuid)?;
    Ok(())
}

/// Renames a folder.
#[tauri::command]
pub async fn rename_folder(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    let uuid = Uuid::parse_str(&id)?;
    vault.rename_folder(&uuid, &name)?;
    Ok(())
}

// ── Bitwarden JSON import ──────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct BitwardenJson {
    items: Vec<BitwardenItem>,
}

#[derive(Debug, serde::Deserialize)]
struct BitwardenItem {
    #[serde(rename = "type")]
    item_type: u8,
    name: String,
    notes: Option<String>,
    login: Option<BitwardenLogin>,
    card: Option<BitwardenCard>,
    identity: Option<BitwardenIdentity>,
}

#[derive(Debug, serde::Deserialize)]
struct BitwardenLogin {
    username: Option<String>,
    password: Option<String>,
    uris: Option<Vec<BitwardenUri>>,
    totp: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BitwardenUri {
    uri: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitwardenCard {
    cardholder_name: Option<String>,
    number: Option<String>,
    exp_month: Option<String>,
    exp_year: Option<String>,
    code: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitwardenIdentity {
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    address1: Option<String>,
    passport_number: Option<String>,
}

/// Imports items from a Bitwarden JSON export. Returns count of imported items.
#[tauri::command]
pub async fn import_bitwarden_json(
    state: State<'_, AppState>,
    content: String,
    source_tag: Option<String>,
) -> Result<usize, AppError> {
    let bw: BitwardenJson = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Invalid Bitwarden JSON: {e}")))?;

    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;

    let mut count = 0usize;
    for bw_item in bw.items {
        let payload = match bw_item.item_type {
            1 => {
                let login = bw_item.login.unwrap_or(BitwardenLogin {
                    username: None,
                    password: None,
                    uris: None,
                    totp: None,
                });
                let url = login
                    .uris
                    .as_ref()
                    .and_then(|u| u.first())
                    .map(|u| u.uri.clone())
                    .unwrap_or_default();
                ItemPayload::Login {
                    url,
                    username: login.username.unwrap_or_default(),
                    password: login.password.unwrap_or_default(),
                    totp_secret: login.totp,
                    notes: bw_item.notes,
                    custom_fields: vec![],
                    password_history: vec![],
                }
            }
            2 => ItemPayload::Note {
                content: bw_item.notes.unwrap_or_default(),
            },
            3 => {
                let card = bw_item.card.unwrap_or(BitwardenCard {
                    cardholder_name: None,
                    number: None,
                    exp_month: None,
                    exp_year: None,
                    code: None,
                });
                let exp_month: u8 = card
                    .exp_month
                    .as_deref()
                    .and_then(|m| m.parse().ok())
                    .unwrap_or(1);
                let exp_year: u16 = card
                    .exp_year
                    .as_deref()
                    .and_then(|y| y.parse().ok())
                    .unwrap_or(2025);
                ItemPayload::Card {
                    cardholder: card.cardholder_name.unwrap_or_default(),
                    number: card.number.unwrap_or_default(),
                    expiry_month: exp_month,
                    expiry_year: exp_year,
                    cvv: card.code.unwrap_or_default(),
                    notes: bw_item.notes,
                }
            }
            4 => {
                let id = bw_item.identity.unwrap_or(BitwardenIdentity {
                    first_name: None,
                    last_name: None,
                    email: None,
                    phone: None,
                    address1: None,
                    passport_number: None,
                });
                ItemPayload::Identity {
                    first_name: id.first_name,
                    last_name: id.last_name,
                    email: id.email,
                    phone: id.phone,
                    address: id.address1,
                    passport: id.passport_number,
                    notes: bw_item.notes,
                }
            }
            _ => continue,
        };
        vault.add_item(&bw_item.name, payload, None, false, source_tag.clone())?;
        count += 1;
    }
    vault.save()?;
    Ok(count)
}

// ── Password health report ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthEntryDto {
    pub id: String,
    pub title: String,
    pub url: String,
    pub is_weak: bool,
    pub is_duplicate: bool,
    pub is_old: bool,
    pub updated_at: i64,
}

// ── Vault 2FA ─────────────────────────────────────────────────────────────────

/// Checks if the vault at `dir_path` requires a TOTP code to unlock.
/// Reads vault.meta without decrypting the vault — safe to call before open.
#[tauri::command]
pub async fn vault_requires_2fa(dir_path: String) -> Result<bool, AppError> {
    let meta_path = std::path::Path::new(&dir_path).join("vault.meta");
    // Refuse symlinks — same policy as vault open.
    let lstat = meta_path.symlink_metadata().map_err(|_| AppError::NotFound)?;
    if lstat.file_type().is_symlink() {
        return Err(AppError::Other("symlink detected on vault.meta".into()));
    }
    let bytes = std::fs::read(&meta_path).map_err(|_| AppError::NotFound)?;
    let meta: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| AppError::Serialization(e.to_string()))?;
    Ok(meta
        .get("totp_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// Returns true if the currently open vault has 2FA enabled.
#[tauri::command]
pub async fn vault_has_2fa(state: State<'_, AppState>) -> Result<bool, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    Ok(vault.has_2fa())
}

#[derive(Debug, serde::Serialize)]
pub struct VaultTwoFaSetup {
    pub secret: String,
    pub uri: String,
    pub qr_svg: String,
}

fn uri_to_qr_svg(uri: &str) -> String {
    use qrcode::{Color, QrCode};
    let Ok(code) = QrCode::new(uri.as_bytes()) else {
        return String::new();
    };
    let w = code.width();
    let cell = 8usize;
    let margin = 4usize;
    let size = (w + 2 * margin) * cell;
    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}"><rect width="{size}" height="{size}" fill="white"/>"#
    );
    for row in 0..w {
        for col in 0..w {
            if code[(row, col)] == Color::Dark {
                let x = (col + margin) * cell;
                let y = (row + margin) * cell;
                s.push_str(&format!(
                    r#"<rect x="{x}" y="{y}" width="{cell}" height="{cell}" fill="black"/>"#
                ));
            }
        }
    }
    s.push_str("</svg>");
    s
}

/// Generates a fresh TOTP secret for 2FA setup. Returns `{secret, uri, qr_svg}`.
/// Does NOT enable 2FA yet — call `confirm_vault_2fa` after the user scans and verifies.
#[tauri::command]
pub async fn setup_vault_2fa(
    state: State<'_, AppState>,
) -> Result<VaultTwoFaSetup, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let (secret, uri) = vault.generate_2fa_secret()?;
    let qr_svg = uri_to_qr_svg(&uri);
    Ok(VaultTwoFaSetup { secret, uri, qr_svg })
}

/// Enables vault 2FA by verifying `code` against `secret`, then storing the encrypted secret.
#[tauri::command]
pub async fn confirm_vault_2fa(
    state: State<'_, AppState>,
    secret: String,
    code: String,
) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    vault.enable_2fa(&secret, &code)?;
    Ok(())
}

/// Disables vault 2FA. Requires the current TOTP code as verification.
#[tauri::command]
pub async fn disable_vault_2fa(
    state: State<'_, AppState>,
    code: String,
) -> Result<(), AppError> {
    let mut guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_mut().ok_or(AppError::VaultLocked)?;
    vault.disable_2fa(&code)?;
    Ok(())
}

/// Analyses all Login items and returns health issues.
#[tauri::command]
pub async fn get_health_report(state: State<'_, AppState>) -> Result<Vec<HealthEntryDto>, AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let entries = vault.health_report()?;
    Ok(entries
        .into_iter()
        .map(|e| HealthEntryDto {
            id: e.id,
            title: e.title,
            url: e.url,
            is_weak: e.is_weak,
            is_duplicate: e.is_duplicate,
            is_old: e.is_old,
            updated_at: e.updated_at,
        })
        .collect())
}

// ── CSV Export ─────────────────────────────────────────────────────────────────

/// Opens a native save-file dialog for choosing a CSV export path.
#[tauri::command]
pub async fn pick_csv_save_path() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Export passwords as CSV")
        .add_filter("CSV", &["csv"])
        .set_file_name("passwords_export.csv")
        .save_file()
        .await
        .map(|h| h.path().to_string_lossy().into_owned())
}

/// Exports all Login items to a CSV file at the given path.
#[tauri::command]
pub async fn export_items_csv(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    let guard = state.vault.lock().map_err(|_| AppError::LockPoisoned)?;
    let vault = guard.as_ref().ok_or(AppError::VaultLocked)?;
    let csv = vault.export_items_csv()?;
    std::fs::write(&path, csv.as_bytes())
        .map_err(|e| AppError::Other(format!("Failed to write CSV: {e}")))?;
    Ok(())
}

// ── Screen capture protection ──────────────────────────────────────────────────

/// On Windows: applies or removes WDA_EXCLUDEFROMCAPTURE so the window is
/// invisible to screen-capture tools (OBS, PrintScreen, thumbnail cache)
/// while a plaintext password is visible in the UI.
///
/// On other platforms this is a no-op — the frontend still calls it
/// so the code path is the same everywhere.
#[tauri::command]
pub async fn set_screen_capture_protection(
    window: tauri::WebviewWindow,
    enabled: bool,
) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity;

        const WDA_NONE: u32               = 0x0000_0000;
        const WDA_EXCLUDEFROMCAPTURE: u32 = 0x0000_0011;

        let handle = window
            .window_handle()
            .map_err(|e| AppError::Other(e.to_string()))?;

        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            let hwnd = h.hwnd.get();
            let affinity = if enabled { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
            // SAFETY: hwnd is the handle of our own window — always valid here.
            unsafe { SetWindowDisplayAffinity(hwnd, affinity); }
        }
    }

    let _ = (window, enabled); // suppress unused-variable warnings on non-Windows
    Ok(())
}

// ── HaveIBeenPwned k-anonymity breach check ────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HibpResult {
    pub pwned_count: u64,   // 0 = not found in any breach
    pub checked: bool,      // false if the check could not be performed
}

/// Checks whether a password appears in the HaveIBeenPwned breach database
/// using the k-anonymity API — only the first 5 characters of the SHA-1 hash
/// are transmitted; the server never sees the full password.
///
/// Returns pwned_count = 0 when not found.  checked = false when offline.
#[tauri::command]
pub async fn check_password_breach(password: String) -> Result<HibpResult, AppError> {
    use sha1::{Digest, Sha1};

    // Compute SHA-1 locally — never sent in full.
    let hash_bytes = Sha1::digest(password.as_bytes());
    let hash_hex: String = hash_bytes.iter().map(|b| format!("{b:02X}")).collect();
    let prefix  = &hash_hex[..5];
    let suffix  = &hash_hex[5..];

    // k-anonymity: send only the 5-char prefix, receive all matching suffixes.
    let url = format!("https://api.pwnedpasswords.com/range/{prefix}");

    let body = tokio::task::spawn_blocking(move || {
        ureq::get(&url)
            .set("User-Agent", "VaultPass/0.2 (github.com/RakinSV/AirVault_RSV)")
            .set("Add-Padding", "true") // prevents traffic analysis via response size
            .call()
            .ok()
            .and_then(|r| r.into_string().ok())
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?;

    let Some(body) = body else {
        return Ok(HibpResult { pwned_count: 0, checked: false });
    };

    // Response lines: "SUFFIX:COUNT\r\n"
    let pwned_count = body.lines().find_map(|line| {
        let (s, count) = line.split_once(':')?;
        if s.eq_ignore_ascii_case(suffix) {
            count.trim().parse::<u64>().ok()
        } else {
            None
        }
    }).unwrap_or(0);

    Ok(HibpResult { pwned_count, checked: true })
}
