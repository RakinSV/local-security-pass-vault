//! Высокоуровневая криптография vault: envelope encryption, шифрование записей,
//! поисковый индекс. Построена поверх безопасных обёрток `crate::sodium`.
//! Правила — `.claude/rules/crypto.md`.

use crate::error::{Result, VaultError};
use crate::sodium::{self, Key, KEY_LEN, NONCE_LEN};
use uuid::Uuid;

// Доменные метки (associated data) — обеспечивают доменную сепарацию шифртекстов
// и защиту от подмены блоба между разными назначениями.
const AD_VAULT_KEY: &[u8] = b"vaultpass:vault-key:v1";

/// Три независимых ключа, выведенных из мастер-пароля (domain separation).
/// `db_key` открывает SQLCipher-БД (внешний слой), `encryption_key` расшифровывает
/// Vault Key, `search_key` считает поисковый HMAC.
///
/// Раскладка KDF-выхода (96 байт, отклонение от 64 в crypto.md — одобрено):
///   0..32   → db_key
///   32..64  → encryption_key
///   64..96  → search_key
pub struct MasterKeys {
    pub db_key: Key,
    pub encryption_key: Key,
    pub search_key: Key,
}

/// Деривация мастер-ключей из пароля и соли (Argon2id, 96 байт → 3×32).
pub fn derive_master_keys(password: &[u8], salt: &[u8; sodium::SALT_LEN]) -> Result<MasterKeys> {
    let kdf = sodium::argon2id_derive::<96>(password, salt)?;
    let mut db_key = Key::zeroed();
    let mut encryption_key = Key::zeroed();
    let mut search_key = Key::zeroed();
    db_key
        .as_mut_bytes()
        .copy_from_slice(&kdf.as_bytes()[0..KEY_LEN]);
    encryption_key
        .as_mut_bytes()
        .copy_from_slice(&kdf.as_bytes()[KEY_LEN..2 * KEY_LEN]);
    search_key
        .as_mut_bytes()
        .copy_from_slice(&kdf.as_bytes()[2 * KEY_LEN..3 * KEY_LEN]);
    // kdf будет обнулён при drop.
    Ok(MasterKeys {
        db_key,
        encryption_key,
        search_key,
    })
}

/// Результат создания нового Vault Key: сам ключ (в памяти) + его зашифрованная
/// форма для хранения в заголовке vault.
pub struct WrappedVaultKey {
    pub vault_key: Key,
    pub encrypted: Vec<u8>,
    pub nonce: [u8; NONCE_LEN],
}

/// Генерирует случайный Vault Key и шифрует его `encryption_key` (envelope).
pub fn wrap_new_vault_key(encryption_key: &Key) -> Result<WrappedVaultKey> {
    let vault_key = Key::random()?;
    let nonce = sodium::gen_nonce()?;
    let encrypted = sodium::aead_seal(vault_key.as_bytes(), AD_VAULT_KEY, &nonce, encryption_key)?;
    Ok(WrappedVaultKey {
        vault_key,
        encrypted,
        nonce,
    })
}

/// Расшифровывает Vault Key из заголовка. Неверный мастер-пароль или подделка →
/// `DecryptionFailed` (без раскрытия причины).
pub fn unwrap_vault_key(
    encryption_key: &Key,
    encrypted: &[u8],
    nonce: &[u8; NONCE_LEN],
) -> Result<Key> {
    let bytes = sodium::aead_open(encrypted, AD_VAULT_KEY, nonce, encryption_key)?;
    if bytes.len() != KEY_LEN {
        return Err(VaultError::DecryptionFailed);
    }
    Key::from_slice(&bytes)
}

/// Перешифровывает существующий Vault Key новым `encryption_key` (смена
/// мастер-пароля). Записи НЕ трогаются — vault_key остаётся тем же (crypto.md).
pub fn rewrap_vault_key(
    new_encryption_key: &Key,
    vault_key: &Key,
) -> Result<(Vec<u8>, [u8; NONCE_LEN])> {
    let nonce = sodium::gen_nonce()?;
    let encrypted =
        sodium::aead_seal(vault_key.as_bytes(), AD_VAULT_KEY, &nonce, new_encryption_key)?;
    Ok((encrypted, nonce))
}

/// Поисковый хеш заголовка: HMAC-SHA256(lowercase(title), search_key).
/// Раскрывает факт совпадения, НЕ содержимое (crypto.md).
pub fn search_hash(title: &str, search_key: &Key) -> Result<[u8; sodium::HMAC_LEN]> {
    sodium::hmac_sha256(title.to_lowercase().as_bytes(), search_key)
}

/// Associated data для полей записи — привязывает шифртекст к UUID записи и
/// имени поля, исключая подмену блобов между записями/полями.
fn record_ad(item_id: &Uuid, field: &str) -> Vec<u8> {
    let mut ad = Vec::with_capacity(16 + 1 + field.len());
    ad.extend_from_slice(item_id.as_bytes());
    ad.push(b':');
    ad.extend_from_slice(field.as_bytes());
    ad
}

/// Шифрует поле записи. Возвращает (шифртекст, nonce) — для хранения в
/// отдельных колонках (например, payload_encrypted + payload_nonce).
pub fn encrypt_field(
    plaintext: &[u8],
    item_id: &Uuid,
    field: &str,
    vault_key: &Key,
) -> Result<(Vec<u8>, [u8; NONCE_LEN])> {
    let nonce = sodium::gen_nonce()?;
    let ad = record_ad(item_id, field);
    let ct = sodium::aead_seal(plaintext, &ad, &nonce, vault_key)?;
    Ok((ct, nonce))
}

/// Расшифровывает поле записи (nonce хранится в отдельной колонке).
pub fn decrypt_field(
    ciphertext: &[u8],
    nonce: &[u8; NONCE_LEN],
    item_id: &Uuid,
    field: &str,
    vault_key: &Key,
) -> Result<Vec<u8>> {
    let ad = record_ad(item_id, field);
    sodium::aead_open(ciphertext, &ad, nonce, vault_key)
}

/// Шифрует поле, у которого нет отдельной колонки под nonce (например, title):
/// nonce приклеивается в начало блоба → `nonce(24) || ciphertext`.
pub fn encrypt_field_inline(
    plaintext: &[u8],
    item_id: &Uuid,
    field: &str,
    vault_key: &Key,
) -> Result<Vec<u8>> {
    let (ct, nonce) = encrypt_field(plaintext, item_id, field, vault_key)?;
    let mut blob = Vec::with_capacity(NONCE_LEN + ct.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ct);
    Ok(blob)
}

/// Расшифровывает inline-поле формата `nonce(24) || ciphertext`.
pub fn decrypt_field_inline(
    blob: &[u8],
    item_id: &Uuid,
    field: &str,
    vault_key: &Key,
) -> Result<Vec<u8>> {
    if blob.len() < NONCE_LEN {
        return Err(VaultError::DecryptionFailed);
    }
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&blob[..NONCE_LEN]);
    decrypt_field(&blob[NONCE_LEN..], &nonce, item_id, field, vault_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SALT: [u8; 16] = [7u8; 16];

    #[test]
    fn master_keys_are_independent() {
        let mk = derive_master_keys(b"correct horse battery staple", &SALT).unwrap();
        assert_ne!(mk.db_key.as_bytes(), mk.encryption_key.as_bytes());
        assert_ne!(mk.encryption_key.as_bytes(), mk.search_key.as_bytes());
        assert_ne!(
            mk.db_key.as_bytes(),
            mk.search_key.as_bytes(),
            "ключи совпали — нет domain separation"
        );
    }

    #[test]
    fn kdf_deterministic_same_input() {
        let a = derive_master_keys(b"pw", &SALT).unwrap();
        let b = derive_master_keys(b"pw", &SALT).unwrap();
        assert_eq!(a.encryption_key.as_bytes(), b.encryption_key.as_bytes());
    }

    #[test]
    fn kdf_different_salt_differs() {
        let a = derive_master_keys(b"pw", &[1u8; 16]).unwrap();
        let b = derive_master_keys(b"pw", &[2u8; 16]).unwrap();
        assert_ne!(a.encryption_key.as_bytes(), b.encryption_key.as_bytes());
    }

    #[test]
    fn envelope_roundtrip() {
        let mk = derive_master_keys(b"master", &SALT).unwrap();
        let wrapped = wrap_new_vault_key(&mk.encryption_key).unwrap();
        let recovered =
            unwrap_vault_key(&mk.encryption_key, &wrapped.encrypted, &wrapped.nonce).unwrap();
        assert_eq!(recovered.as_bytes(), wrapped.vault_key.as_bytes());
    }

    #[test]
    fn envelope_wrong_password_fails() {
        let good = derive_master_keys(b"right", &SALT).unwrap();
        let bad = derive_master_keys(b"wrong", &SALT).unwrap();
        let wrapped = wrap_new_vault_key(&good.encryption_key).unwrap();
        assert!(matches!(
            unwrap_vault_key(&bad.encryption_key, &wrapped.encrypted, &wrapped.nonce),
            Err(VaultError::DecryptionFailed)
        ));
    }

    #[test]
    fn change_master_password_keeps_vault_key() {
        let old = derive_master_keys(b"old-pass", &SALT).unwrap();
        let new = derive_master_keys(b"new-pass", &SALT).unwrap();
        let wrapped = wrap_new_vault_key(&old.encryption_key).unwrap();

        // Перешифровываем vault_key новым ключом.
        let (enc2, nonce2) = rewrap_vault_key(&new.encryption_key, &wrapped.vault_key).unwrap();

        // Старый пароль больше не открывает новый блоб.
        assert!(unwrap_vault_key(&old.encryption_key, &enc2, &nonce2).is_err());
        // Новый — открывает, и vault_key тот же.
        let recovered = unwrap_vault_key(&new.encryption_key, &enc2, &nonce2).unwrap();
        assert_eq!(recovered.as_bytes(), wrapped.vault_key.as_bytes());
    }

    #[test]
    fn record_field_roundtrip() {
        let vk = Key::random().unwrap();
        let id = Uuid::new_v4();
        let (ct, nonce) = encrypt_field(b"s3cr3t", &id, "payload", &vk).unwrap();
        let pt = decrypt_field(&ct, &nonce, &id, "payload", &vk).unwrap();
        assert_eq!(pt, b"s3cr3t");
    }

    #[test]
    fn record_field_wrong_id_fails() {
        let vk = Key::random().unwrap();
        let id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let (ct, nonce) = encrypt_field(b"data", &id, "payload", &vk).unwrap();
        assert!(matches!(
            decrypt_field(&ct, &nonce, &other, "payload", &vk),
            Err(VaultError::DecryptionFailed)
        ));
    }

    #[test]
    fn inline_field_roundtrip() {
        let vk = Key::random().unwrap();
        let id = Uuid::new_v4();
        let blob = encrypt_field_inline("Мой банк".as_bytes(), &id, "title", &vk).unwrap();
        let pt = decrypt_field_inline(&blob, &id, "title", &vk).unwrap();
        assert_eq!(pt, "Мой банк".as_bytes());
    }

    #[test]
    fn search_hash_stable_and_case_insensitive() {
        let sk = Key::random().unwrap();
        let h1 = search_hash("GitHub", &sk).unwrap();
        let h2 = search_hash("github", &sk).unwrap();
        assert_eq!(h1, h2, "поиск должен быть регистронезависим");
    }

    #[test]
    fn search_hash_differs_by_key() {
        let sk1 = Key::random().unwrap();
        let sk2 = Key::random().unwrap();
        assert_ne!(
            search_hash("github", &sk1).unwrap(),
            search_hash("github", &sk2).unwrap()
        );
    }
}
