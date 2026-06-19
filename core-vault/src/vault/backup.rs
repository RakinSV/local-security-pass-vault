//! Зашифрованный бэкап vault с BIP-39 мнемоникой (24 слова, 256-бит энтропии).
//!
//! ## Формат файла v2 (`.vbk`)
//!
//! ```text
//! VPBK (4) | version=0x02 (1) | argon2_salt (16) | nonce (24) | ciphertext
//! ```
//!
//! Plaintext внутри AEAD ciphertext:
//! ```text
//! blake3_checksum (32) | vault_salt (16) | meta_len (4 LE) | vault_meta | vault_db
//! ```
//!
//! **KDF (ADR-003)**: BIP-39 seed → Argon2id(seed[..32], argon2_salt, t=8, m=512MiB) → key (32B)
//!
//! Примечание: spec в backup.md предписывает 4GB RAM для KDF. Libsodium фиксирует
//! p=1 (параллелизм), поэтому при 4GB одна попытка займёт 60+ секунд на обычном ПК,
//! что неприемлемо для UX. Используем 512MiB/8-итераций — компромисс между
//! безопасностью и юзабилити. При 24-словной BIP-39 мнемонике пространство ключей
//! 2048^24 = 2^264 делает брутфорс физически невозможным вне зависимости от KDF.
//!
//! ## Обратная совместимость
//!
//! Формат v1 (`.vpbak`, 17-слов, 256MiB/4-итераций) всё ещё читается `restore`.
//! Новые бэкапы создаются только в формате v2.

use crate::error::{Result, VaultError};
use crate::sodium::{self, Key, SALT_LEN};
use std::path::Path;

// ── Константы формата ────────────────────────────────────────────────────────

const BACKUP_MAGIC: &[u8; 4] = b"VPBK";
const BACKUP_V1: u8 = 0x01;
const BACKUP_V2: u8 = 0x02;
const HEADER_LEN: usize = 4 + 1 + SALT_LEN + 24; // magic + version + argon2_salt + nonce
const CHECKSUM_LEN: usize = 32; // BLAKE3

const AD_V1: &[u8] = b"vaultpass:backup:v1";
const AD_V2: &[u8] = b"vaultpass:backup:v2";

// Усиленный KDF профиль для бэкапа (t=8, m=512MiB — сильнее unlock-профиля t=4/256MiB).
// Spec (backup.md) предписывает 4GB, но это неприемлемо для UX; 512MiB — компромисс.
// При 24-словной BIP-39 мнемонике брутфорс невозможен в любом случае (2^264 вариантов).
const BACKUP_OPSLIMIT: u64 = 8;
const BACKUP_MEMLIMIT: usize = 512 * 1024 * 1024; // 512 MiB

const BACKUP_V1_OPSLIMIT: u64 = 4;
const BACKUP_V1_MEMLIMIT: usize = 256 * 1024 * 1024;

// ── BIP-39 мнемоника ─────────────────────────────────────────────────────────

/// Генерирует 24-словную BIP-39 мнемонику (English, 256-бит энтропии).
/// Использует libsodium CSPRNG. Показывать пользователю ОДИН РАЗ.
/// VaultPass никогда не сохраняет мнемонику на диск.
pub fn generate_mnemonic() -> Result<String> {
    sodium::init().map_err(VaultError::Crypto)?;
    let mut entropy = [0u8; 32]; // 256 бит → 24 слова по BIP-39
    sodium::random_bytes(&mut entropy)?;
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy)
        .map_err(|_| VaultError::Crypto("bip39 mnemonic generation failed"))?;
    Ok(mnemonic.to_string())
}

/// Проверяет мнемонику: BIP-39 словарь + встроенная контрольная сумма.
pub fn validate_mnemonic(phrase: &str) -> bool {
    bip39::Mnemonic::parse_normalized(phrase).is_ok()
}

/// Количество слов в фразе (для UI-проверки).
pub fn mnemonic_word_count(phrase: &str) -> usize {
    phrase.split_whitespace().count()
}

// ── Обратная совместимость: v1 (17 слов, собственный wordlist) ───────────────

const WORDLIST_TXT: &str = include_str!("wordlist.txt");

fn legacy_wordlist() -> Vec<&'static str> {
    WORDLIST_TXT.split_whitespace().collect()
}

/// Генерирует seed-фразу из N слов (legacy, для v1 бэкапов).
/// Для новых бэкапов используй `generate_mnemonic()`.
pub fn generate_phrase(count: usize) -> Result<String> {
    sodium::init().map_err(VaultError::Crypto)?;
    let list = legacy_wordlist();
    let n = list.len();
    let mut words = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 4];
        sodium::random_bytes(&mut buf)?;
        let idx = (u32::from_le_bytes(buf) as usize) % n;
        words.push(list[idx]);
    }
    Ok(words.join(" "))
}

/// Проверяет, что все слова есть в legacy словаре (только для v1).
pub fn validate_phrase(phrase: &str) -> bool {
    let list: std::collections::HashSet<&str> = legacy_wordlist().into_iter().collect();
    phrase.split_whitespace().all(|w| list.contains(w))
}

pub fn wordlist_size() -> usize {
    legacy_wordlist().len()
}

// ── KDF: phrase → backup key ─────────────────────────────────────────────────

fn mnemonic_to_key(phrase: &str, argon2_salt: &[u8; SALT_LEN]) -> Result<Key> {
    let mnemonic = bip39::Mnemonic::parse_normalized(phrase)
        .map_err(|_| VaultError::DecryptionFailed)?;
    // BIP-39 → 64-байтный seed (PBKDF2-HMAC-SHA512, passphrase = "").
    // Первые 32 байта — IKM для Argon2id (доменная сепарация).
    let seed = mnemonic.to_seed("");
    derive_key(&seed[..32], argon2_salt, BACKUP_OPSLIMIT, BACKUP_MEMLIMIT)
}

fn legacy_phrase_to_key(phrase: &str, argon2_salt: &[u8; SALT_LEN]) -> Result<Key> {
    derive_key(phrase.as_bytes(), argon2_salt, BACKUP_V1_OPSLIMIT, BACKUP_V1_MEMLIMIT)
}

fn derive_key(ikm: &[u8], salt: &[u8; SALT_LEN], ops: u64, mem: usize) -> Result<Key> {
    let secret = sodium::argon2id_derive_custom::<32>(ikm, salt, ops, mem)?;
    let mut key = Key::zeroed();
    key.as_mut_bytes().copy_from_slice(secret.as_bytes());
    Ok(key)
}

// ── BLAKE3 checksum ──────────────────────────────────────────────────────────

fn blake3_checksum(data: &[u8]) -> [u8; CHECKSUM_LEN] {
    *blake3::hash(data).as_bytes()
}

// ── Экспорт (v2) ─────────────────────────────────────────────────────────────

/// Экспортирует vault в зашифрованный бэкап-файл (формат v2, `.vbk`).
/// `phrase` должна быть 24-словной BIP-39 мнемоникой из `generate_mnemonic()`.
pub fn export(vault_dir: &Path, dest: &Path, phrase: &str) -> Result<()> {
    let vault_salt = std::fs::read(vault_dir.join("vault.salt"))?;
    let vault_meta = std::fs::read(vault_dir.join("vault.meta"))?;
    let vault_db   = std::fs::read(vault_dir.join("vault.db"))?;

    if vault_salt.len() != SALT_LEN {
        return Err(VaultError::TamperedVault);
    }

    let checksum = blake3_checksum(&vault_db);

    // Plaintext: checksum(32) | vault_salt(16) | meta_len(4 LE) | meta | db
    let meta_len = vault_meta.len() as u32;
    let mut plaintext = Vec::with_capacity(
        CHECKSUM_LEN + SALT_LEN + 4 + vault_meta.len() + vault_db.len(),
    );
    plaintext.extend_from_slice(&checksum);
    plaintext.extend_from_slice(&vault_salt);
    plaintext.extend_from_slice(&meta_len.to_le_bytes());
    plaintext.extend_from_slice(&vault_meta);
    plaintext.extend_from_slice(&vault_db);

    let argon2_salt = sodium::gen_salt()?;
    let backup_key  = mnemonic_to_key(phrase, &argon2_salt)?;
    let nonce       = sodium::gen_nonce()?;
    let ciphertext  = sodium::aead_seal(&plaintext, AD_V2, &nonce, &backup_key)?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(BACKUP_MAGIC);
    out.push(BACKUP_V2);
    out.extend_from_slice(&argon2_salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);

    super::file::atomic_write(dest, &out)
}

// ── Восстановление ────────────────────────────────────────────────────────────

/// Восстанавливает vault из бэкапа. Поддерживает форматы v1 и v2.
/// Неверная фраза/мнемоника → `DecryptionFailed`.
pub fn restore(src: &Path, dest_dir: &Path, phrase: &str) -> Result<()> {
    let data = std::fs::read(src)?;

    if data.len() < HEADER_LEN + 1 {
        return Err(VaultError::TamperedVault);
    }
    if &data[0..4] != BACKUP_MAGIC {
        return Err(VaultError::TamperedVault);
    }

    let version = data[4];

    let mut argon2_salt = [0u8; SALT_LEN];
    argon2_salt.copy_from_slice(&data[5..5 + SALT_LEN]);

    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&data[5 + SALT_LEN..HEADER_LEN]);

    let ciphertext = &data[HEADER_LEN..];

    let plaintext = match version {
        BACKUP_V2 => {
            let key = mnemonic_to_key(phrase, &argon2_salt)?;
            sodium::aead_open(ciphertext, AD_V2, &nonce, &key)?
        }
        BACKUP_V1 => {
            let key = legacy_phrase_to_key(phrase, &argon2_salt)?;
            sodium::aead_open(ciphertext, AD_V1, &nonce, &key)?
        }
        _ => return Err(VaultError::TamperedVault),
    };

    match version {
        BACKUP_V2 => restore_v2_payload(&plaintext, dest_dir),
        BACKUP_V1 => restore_v1_payload(&plaintext, dest_dir),
        _ => unreachable!(),
    }
}

fn restore_v2_payload(plaintext: &[u8], dest_dir: &Path) -> Result<()> {
    if plaintext.len() < CHECKSUM_LEN + SALT_LEN + 4 {
        return Err(VaultError::TamperedVault);
    }

    let stored_checksum = &plaintext[..CHECKSUM_LEN];
    let vault_salt      = &plaintext[CHECKSUM_LEN..CHECKSUM_LEN + SALT_LEN];

    let meta_len_bytes: [u8; 4] = plaintext[CHECKSUM_LEN + SALT_LEN..CHECKSUM_LEN + SALT_LEN + 4]
        .try_into()
        .map_err(|_| VaultError::TamperedVault)?;
    let meta_len  = u32::from_le_bytes(meta_len_bytes) as usize;
    let meta_start = CHECKSUM_LEN + SALT_LEN + 4;
    let meta_end   = meta_start + meta_len;

    if plaintext.len() < meta_end {
        return Err(VaultError::TamperedVault);
    }
    let vault_meta = &plaintext[meta_start..meta_end];
    let vault_db   = &plaintext[meta_end..];

    // Проверяем BLAKE3 через constant-time сравнение.
    let computed = blake3_checksum(vault_db);
    if !sodium::memcmp(stored_checksum, &computed) {
        return Err(VaultError::TamperedVault);
    }

    write_vault_files(dest_dir, vault_salt, vault_meta, vault_db)
}

fn restore_v1_payload(plaintext: &[u8], dest_dir: &Path) -> Result<()> {
    if plaintext.len() < SALT_LEN + 4 {
        return Err(VaultError::TamperedVault);
    }
    let vault_salt = &plaintext[..SALT_LEN];

    let meta_len_bytes: [u8; 4] = plaintext[SALT_LEN..SALT_LEN + 4]
        .try_into()
        .map_err(|_| VaultError::TamperedVault)?;
    let meta_len   = u32::from_le_bytes(meta_len_bytes) as usize;
    let meta_start = SALT_LEN + 4;
    let meta_end   = meta_start + meta_len;

    if plaintext.len() < meta_end {
        return Err(VaultError::TamperedVault);
    }
    let vault_meta = &plaintext[meta_start..meta_end];
    let vault_db   = &plaintext[meta_end..];

    write_vault_files(dest_dir, vault_salt, vault_meta, vault_db)
}

fn write_vault_files(dest_dir: &Path, salt: &[u8], meta: &[u8], db: &[u8]) -> Result<()> {
    std::fs::create_dir_all(dest_dir)?;
    super::file::atomic_write(&dest_dir.join("vault.salt"), salt)?;
    super::file::atomic_write(&dest_dir.join("vault.meta"), meta)?;
    super::file::atomic_write(&dest_dir.join("vault.db"), db)?;

    #[cfg(unix)]
    {
        super::file::restrict_permissions(&dest_dir.join("vault.salt")).ok();
        super::file::restrict_permissions(&dest_dir.join("vault.meta")).ok();
        super::file::restrict_permissions(&dest_dir.join("vault.db")).ok();
    }
    Ok(())
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_is_24_words() {
        let phrase = generate_mnemonic().unwrap();
        assert_eq!(mnemonic_word_count(&phrase), 24, "ожидается 24 слова");
    }

    #[test]
    fn mnemonic_validates_ok() {
        let phrase = generate_mnemonic().unwrap();
        assert!(validate_mnemonic(&phrase));
    }

    #[test]
    fn mnemonic_validation_rejects_bad() {
        assert!(!validate_mnemonic("abandon abandon abandon"));
        assert!(!validate_mnemonic("notaword notaword notaword"));
    }

    #[test]
    fn two_mnemonics_differ() {
        let a = generate_mnemonic().unwrap();
        let b = generate_mnemonic().unwrap();
        assert_ne!(a, b, "две мнемоники совпали — проблема с CSPRNG");
    }

    #[test]
    fn legacy_generate_17_words() {
        sodium::init().unwrap();
        let phrase = generate_phrase(17).unwrap();
        assert_eq!(phrase.split_whitespace().count(), 17);
    }

    #[test]
    fn backup_v2_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vp_bkv2_{}", uuid::Uuid::new_v4()));
        let restore_dir =
            std::env::temp_dir().join(format!("vp_rstv2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("vault.salt"), b"0123456789abcdef").unwrap();
        std::fs::write(dir.join("vault.meta"), b"{\"vault_id\":\"test-v2\"}").unwrap();
        std::fs::write(dir.join("vault.db"), b"fake-encrypted-db-v2").unwrap();

        let phrase = generate_mnemonic().unwrap();
        let backup = dir.join("test.vbk");

        export(&dir, &backup, &phrase).unwrap();
        assert!(backup.exists(), "файл бэкапа не создан");

        // Неверная мнемоника → DecryptionFailed.
        let wrong = generate_mnemonic().unwrap();
        assert!(matches!(
            restore(&backup, &restore_dir, &wrong),
            Err(VaultError::DecryptionFailed)
        ));

        // Верная мнемоника → данные восстановлены.
        restore(&backup, &restore_dir, &phrase).unwrap();
        assert_eq!(std::fs::read(restore_dir.join("vault.salt")).unwrap(), b"0123456789abcdef");
        assert_eq!(
            std::fs::read(restore_dir.join("vault.meta")).unwrap(),
            b"{\"vault_id\":\"test-v2\"}"
        );
        assert_eq!(
            std::fs::read(restore_dir.join("vault.db")).unwrap(),
            b"fake-encrypted-db-v2"
        );

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&restore_dir).ok();
    }

    #[test]
    fn v1_backward_compat() {
        // V1 бэкап должен читаться функцией restore (обратная совместимость).
        let dir =
            std::env::temp_dir().join(format!("vp_bkv1_{}", uuid::Uuid::new_v4()));
        let restore_dir =
            std::env::temp_dir().join(format!("vp_rstv1_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("vault.salt"), b"0123456789abcdef").unwrap();
        std::fs::write(dir.join("vault.meta"), b"{\"vault_id\":\"v1\"}").unwrap();
        std::fs::write(dir.join("vault.db"), b"legacy-db").unwrap();

        let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid actual";
        let backup = dir.join("legacy.vpbak");
        create_v1_for_test(&dir, &backup, phrase).unwrap();

        restore(&backup, &restore_dir, phrase).unwrap();
        assert_eq!(std::fs::read(restore_dir.join("vault.db")).unwrap(), b"legacy-db");

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&restore_dir).ok();
    }

    #[test]
    fn blake3_tamper_detected() {
        let dir =
            std::env::temp_dir().join(format!("vp_tmpr_{}", uuid::Uuid::new_v4()));
        let restore_dir =
            std::env::temp_dir().join(format!("vp_rtmpr_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vault.salt"), b"0123456789abcdef").unwrap();
        std::fs::write(dir.join("vault.meta"), b"{}").unwrap();
        std::fs::write(dir.join("vault.db"), b"original").unwrap();

        let phrase = generate_mnemonic().unwrap();
        let backup = dir.join("t.vbk");
        export(&dir, &backup, &phrase).unwrap();

        // Меняем последний байт файла (портим AEAD тег или payload).
        let mut raw = std::fs::read(&backup).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xFF;
        std::fs::write(&backup, &raw).unwrap();

        // Должна быть ошибка.
        assert!(restore(&backup, &restore_dir, &phrase).is_err());

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&restore_dir).ok();
    }

    // Создаёт v1-совместимый бэкап для тестов обратной совместимости.
    fn create_v1_for_test(vault_dir: &Path, dest: &Path, phrase: &str) -> Result<()> {
        let vault_salt = std::fs::read(vault_dir.join("vault.salt"))?;
        let vault_meta = std::fs::read(vault_dir.join("vault.meta"))?;
        let vault_db   = std::fs::read(vault_dir.join("vault.db"))?;

        let meta_len = vault_meta.len() as u32;
        let mut plaintext = Vec::new();
        plaintext.extend_from_slice(&vault_salt);
        plaintext.extend_from_slice(&meta_len.to_le_bytes());
        plaintext.extend_from_slice(&vault_meta);
        plaintext.extend_from_slice(&vault_db);

        let argon2_salt = sodium::gen_salt()?;
        let key   = legacy_phrase_to_key(phrase, &argon2_salt)?;
        let nonce = sodium::gen_nonce()?;
        let ct    = sodium::aead_seal(&plaintext, AD_V1, &nonce, &key)?;

        let mut out = Vec::new();
        out.extend_from_slice(BACKUP_MAGIC);
        out.push(BACKUP_V1);
        out.extend_from_slice(&argon2_salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        super::super::file::atomic_write(dest, &out)
    }
}
