//! OS Keychain интеграция для хранения Vault Key после unlock.
//!
//! Сохраняет 32-байтный Vault Key в нативное хранилище ОС:
//! - **Windows**: Credential Manager (DPAPI)
//! - **Linux**: Secret Service (libsecret / GNOME Keyring / KWallet)
//! - **macOS**: Keychain
//!
//! Vault Key хранится как hex-строка (64 символа).
//! Если Keychain недоступен (headless, CI) — функции тихо возвращают ошибку/None.
//!
//! ## Правила (security.md)
//! - Vault Key записывается ТОЛЬКО после успешного ввода мастер-пароля (unlock).
//! - При lock() → запись удаляется из Keychain.
//! - Мастер-пароль в Keychain НИКОГДА не записывается.

const SERVICE: &str = "vaultpass";

fn entry_name(vault_uuid: &str) -> String {
    format!("VaultKey/{vault_uuid}")
}

/// Сохраняет 32-байтный Vault Key в OS Keychain.
/// Вызывать только после успешного unlock (ввода мастер-пароля).
/// Ошибка возвращается как строка — не критична, fallback на in-memory.
pub fn store_vault_key(vault_uuid: &str, key_bytes: &[u8; 32]) -> Result<(), String> {
    let hex_key = hex::encode(key_bytes);
    keyring::Entry::new(SERVICE, &entry_name(vault_uuid))
        .map_err(|e| e.to_string())?
        .set_password(&hex_key)
        .map_err(|e| e.to_string())
}

/// Загружает Vault Key из OS Keychain (для quick-unlock).
/// Возвращает None если ключ не найден или Keychain недоступен.
pub fn load_vault_key(vault_uuid: &str) -> Option<[u8; 32]> {
    let hex_key = keyring::Entry::new(SERVICE, &entry_name(vault_uuid))
        .ok()?
        .get_password()
        .ok()?;

    let bytes = hex::decode(&hex_key).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

/// Удаляет Vault Key из OS Keychain. Вызывать при lock().
/// Тихо игнорирует ошибки (ключ уже удалён или Keychain недоступен).
pub fn delete_vault_key(vault_uuid: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, &entry_name(vault_uuid)) {
        let _ = entry.delete_password();
    }
}

/// Проверяет, доступен ли Vault Key в Keychain (для UI quick-unlock индикатора).
pub fn has_vault_key(vault_uuid: &str) -> bool {
    load_vault_key(vault_uuid).is_some()
}
