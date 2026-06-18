//! Зашифрованный бекап vault с защитой seed-фразой (17 слов).
//!
//! Формат файла `.vpbak`:
//!   `VPBK` (4) | version=0x01 (1) | argon2_salt (16) | nonce (24) | ciphertext
//!
//! Ключ шифрования: argon2id(seed_phrase, argon2_salt) → Key<32>
//! Открытый текст: vault.salt (16) ++ meta_len (4 LE) ++ vault.meta ++ vault.db
//! AD: b"vaultpass:backup:v1"

use crate::error::{Result, VaultError};
use crate::sodium::{self, Key, SALT_LEN};
use std::path::Path;

const WORDLIST: &str = include_str!("wordlist.txt");
const BACKUP_MAGIC: &[u8; 4] = b"VPBK";
const BACKUP_VERSION: u8 = 0x01;
const AD_BACKUP: &[u8] = b"vaultpass:backup:v1";
const HEADER_LEN: usize = 4 + 1 + SALT_LEN + 24; // magic+ver+salt+nonce

/// Возвращает слова из встроенного списка BIP-39 (2048 слов).
fn wordlist() -> Vec<&'static str> {
    WORDLIST.split_whitespace().collect()
}

/// Генерирует seed-фразу из `count` случайных слов (разделены пробелами).
/// По умолчанию используй 17 слов → ≥174 бит энтропии при 2048-словном списке.
pub fn generate_phrase(count: usize) -> Result<String> {
    sodium::init().map_err(VaultError::Crypto)?;
    let list = wordlist();
    let n = list.len();
    let mut words = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 4];
        sodium::random_bytes(&mut buf)?;
        // Равномерное распределение: берём 32 бита, берём остаток от n.
        let idx = (u32::from_le_bytes(buf) as usize) % n;
        words.push(list[idx]);
    }
    Ok(words.join(" "))
}

/// Проверяет, что все слова фразы присутствуют в словаре.
pub fn validate_phrase(phrase: &str) -> bool {
    let list: std::collections::HashSet<&str> = wordlist().into_iter().collect();
    phrase.split_whitespace().all(|w| list.contains(w))
}

/// Возвращает количество слов в словаре (для информации).
pub fn wordlist_size() -> usize {
    wordlist().len()
}

fn phrase_to_key(phrase: &str, salt: &[u8; SALT_LEN]) -> Result<Key> {
    let secret = sodium::argon2id_derive::<32>(phrase.as_bytes(), salt)?;
    let mut key = Key::zeroed();
    key.as_mut_bytes().copy_from_slice(secret.as_bytes());
    Ok(key)
}

/// Экспортирует vault в зашифрованный файл бекапа.
///
/// `vault_dir` — директория с `vault.db`, `vault.salt`, `vault.meta`.
/// `dest` — путь к создаваемому `.vpbak` файлу.
/// `phrase` — seed-фраза (17 слов или любая).
pub fn export(vault_dir: &Path, dest: &Path, phrase: &str) -> Result<()> {
    // Читаем все три файла из директории vault.
    let vault_salt = std::fs::read(vault_dir.join("vault.salt"))?;
    let vault_meta = std::fs::read(vault_dir.join("vault.meta"))?;
    let vault_db = std::fs::read(vault_dir.join("vault.db"))?;

    if vault_salt.len() != SALT_LEN {
        return Err(VaultError::TamperedVault);
    }

    // Собираем plaintext: salt_fixed(16) ++ meta_len(4 LE) ++ meta ++ db
    let meta_len = vault_meta.len() as u32;
    let mut plaintext = Vec::with_capacity(SALT_LEN + 4 + vault_meta.len() + vault_db.len());
    plaintext.extend_from_slice(&vault_salt);
    plaintext.extend_from_slice(&meta_len.to_le_bytes());
    plaintext.extend_from_slice(&vault_meta);
    plaintext.extend_from_slice(&vault_db);

    // KDF: производим ключ из seed-фразы.
    let argon2_salt = sodium::gen_salt()?;
    let backup_key = phrase_to_key(phrase, &argon2_salt)?;

    // Шифруем.
    let nonce = sodium::gen_nonce()?;
    let ciphertext = sodium::aead_seal(&plaintext, AD_BACKUP, &nonce, &backup_key)?;

    // Формируем файл.
    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(BACKUP_MAGIC);
    out.push(BACKUP_VERSION);
    out.extend_from_slice(&argon2_salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);

    super::file::atomic_write(dest, &out)
}

/// Восстанавливает vault из бекапа.
///
/// `src` — путь к `.vpbak` файлу.
/// `dest_dir` — куда записать восстановленные файлы vault.
/// `phrase` — seed-фраза для расшифровки.
///
/// Ошибка при неверной фразе → `DecryptionFailed`.
pub fn restore(src: &Path, dest_dir: &Path, phrase: &str) -> Result<()> {
    let data = std::fs::read(src)?;

    if data.len() < HEADER_LEN + 1 {
        return Err(VaultError::TamperedVault);
    }
    if &data[0..4] != BACKUP_MAGIC || data[4] != BACKUP_VERSION {
        return Err(VaultError::TamperedVault);
    }

    let mut argon2_salt = [0u8; SALT_LEN];
    argon2_salt.copy_from_slice(&data[5..5 + SALT_LEN]);

    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&data[5 + SALT_LEN..HEADER_LEN]);

    let ciphertext = &data[HEADER_LEN..];

    // Производим ключ и расшифровываем.
    let backup_key = phrase_to_key(phrase, &argon2_salt)?;
    let plaintext = sodium::aead_open(ciphertext, AD_BACKUP, &nonce, &backup_key)?;

    // Разбираем plaintext: vault_salt(16) ++ meta_len(4 LE) ++ meta ++ db
    if plaintext.len() < SALT_LEN + 4 {
        return Err(VaultError::TamperedVault);
    }
    let vault_salt = &plaintext[..SALT_LEN];
    let meta_len = u32::from_le_bytes(
        plaintext[SALT_LEN..SALT_LEN + 4].try_into().unwrap_or([0; 4]),
    ) as usize;
    let meta_start = SALT_LEN + 4;
    let meta_end = meta_start + meta_len;
    if plaintext.len() < meta_end {
        return Err(VaultError::TamperedVault);
    }
    let vault_meta = &plaintext[meta_start..meta_end];
    let vault_db = &plaintext[meta_end..];

    // Пишем файлы в dest_dir.
    std::fs::create_dir_all(dest_dir)?;
    super::file::atomic_write(&dest_dir.join("vault.salt"), vault_salt)?;
    super::file::atomic_write(&dest_dir.join("vault.meta"), vault_meta)?;
    super::file::atomic_write(&dest_dir.join("vault.db"), vault_db)?;

    #[cfg(unix)]
    {
        super::file::restrict_permissions(&dest_dir.join("vault.salt")).ok();
        super::file::restrict_permissions(&dest_dir.join("vault.meta")).ok();
        super::file::restrict_permissions(&dest_dir.join("vault.db")).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_generate_17_words() {
        sodium::init().unwrap();
        let phrase = generate_phrase(17).unwrap();
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), 17);
    }

    #[test]
    fn phrase_words_in_wordlist() {
        sodium::init().unwrap();
        let phrase = generate_phrase(17).unwrap();
        assert!(validate_phrase(&phrase), "generated phrase has words not in wordlist");
    }

    #[test]
    fn wordlist_has_enough_words() {
        assert!(wordlist_size() >= 512, "wordlist too small: {} words", wordlist_size());
    }

    #[test]
    fn backup_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vp_bkp_{}", uuid::Uuid::new_v4()));
        let restore_dir = std::env::temp_dir().join(format!("vp_rst_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        // Создаём тестовые файлы vault.
        std::fs::write(dir.join("vault.salt"), b"0123456789abcdef").unwrap();
        std::fs::write(dir.join("vault.meta"), b"{\"vault_id\":\"test\"}").unwrap();
        std::fs::write(dir.join("vault.db"), b"fake-encrypted-db-content").unwrap();

        let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid actual";
        let backup = dir.join("test.vpbak");

        export(&dir, &backup, phrase).unwrap();
        assert!(backup.exists());

        // Неверная фраза.
        assert!(matches!(
            restore(&backup, &restore_dir, "wrong phrase here and some more words to match count"),
            Err(VaultError::DecryptionFailed)
        ));

        // Верная фраза.
        restore(&backup, &restore_dir, phrase).unwrap();
        assert_eq!(std::fs::read(restore_dir.join("vault.salt")).unwrap(), b"0123456789abcdef");
        assert_eq!(
            std::fs::read(restore_dir.join("vault.meta")).unwrap(),
            b"{\"vault_id\":\"test\"}"
        );
        assert_eq!(
            std::fs::read(restore_dir.join("vault.db")).unwrap(),
            b"fake-encrypted-db-content"
        );

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&restore_dir).ok();
    }
}
