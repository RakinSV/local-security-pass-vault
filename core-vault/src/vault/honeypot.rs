//! Honeypot-файл для обнаружения ransomware. См. security.md.
//!
//! Рядом с vault.db лежит `vault_backup.db` со случайными байтами. Его SHA-256
//! запоминается в памяти. Если при разблокировке хеш изменился — кто-то (вероятно
//! ransomware) перешифровал/изменил файл → отказ с `PossibleRansomwareDetected`.

use crate::error::{Result, VaultError};
use crate::sodium;
use crate::vault::file;
use std::path::{Path, PathBuf};

pub struct HoneypotGuard {
    path: PathBuf,
    expected_hash: [u8; 32],
}

impl HoneypotGuard {
    /// Создаёт honeypot, если его нет, и запоминает хеш. Если файл уже есть —
    /// просто берёт его текущий хеш как ожидаемый.
    pub fn init(path: &Path) -> Result<Self> {
        let data = match file::read_no_symlink(path) {
            Ok(d) => d,
            Err(VaultError::NotFound) => {
                // Создаём honeypot со случайным содержимым.
                let mut buf = vec![0u8; 4096];
                sodium::random_bytes(&mut buf)?;
                file::atomic_write(path, &buf)?;
                file::restrict_permissions(path).ok();
                buf
            }
            Err(e) => return Err(e),
        };
        let expected_hash = sodium::sha256(&data)?;
        Ok(HoneypotGuard {
            path: path.to_owned(),
            expected_hash,
        })
    }

    /// Перепроверяет целостность honeypot. Вызывать при каждой разблокировке.
    pub fn check(&self) -> Result<()> {
        let data = file::read_no_symlink(&self.path)?;
        let current = sodium::sha256(&data)?;
        if !sodium::memcmp(&current, &self.expected_hash) {
            return Err(VaultError::PossibleRansomwareDetected);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honeypot_detects_tampering() {
        let dir = std::env::temp_dir().join(format!("vp_hp_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let hp = dir.join("vault_backup.db");

        let guard = HoneypotGuard::init(&hp).unwrap();
        assert!(guard.check().is_ok());

        // Имитируем ransomware: перезаписываем файл.
        file::atomic_write(&hp, b"encrypted-by-ransomware").unwrap();
        assert!(matches!(
            guard.check(),
            Err(VaultError::PossibleRansomwareDetected)
        ));

        std::fs::remove_dir_all(&dir).ok();
    }
}
