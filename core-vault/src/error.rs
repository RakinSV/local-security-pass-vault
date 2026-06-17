//! Типы ошибок vault. См. `.claude/rules/vault-schema.md` и `security.md`.
//!
//! КРИТИЧНО: расшифровка имеет ЕДИНСТВЕННУЮ ошибку — `DecryptionFailed`.
//! Она покрывает: неверный пароль + повреждённый vault + несовпадение MAC.
//! Раскрытие причины = oracle-атака (см. security.md «Обработка ошибок»).

use std::path::PathBuf;

/// Результат операций крейта.
pub type Result<T> = std::result::Result<T, VaultError>;

#[derive(Debug)]
pub enum VaultError {
    /// Единственная ошибка расшифровки — БЕЗ деталей о причине.
    DecryptionFailed,

    /// Файл vault не найден.
    NotFound,

    /// Обнаружен symlink на месте vault.db — отказ в открытии.
    SymlinkDetected(PathBuf),

    /// Проверка целостности vault не пройдена (несовпадение UUID и т.п.).
    TamperedVault,

    /// Honeypot-файл изменён — возможная активность ransomware.
    PossibleRansomwareDetected,

    /// Версия схемы не поддерживается.
    UnsupportedSchemaVersion { found: u32, max: u32 },

    /// libsodium не инициализирована или вернула ошибку.
    Crypto(&'static str),

    /// Ошибка сериализации/десериализации payload.
    Serialization(String),

    /// Ошибка БД (SQLite). Деталь — только в Debug, не в Display.
    Database(String),

    /// Ошибка ввода-вывода.
    Io(std::io::Error),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Намеренно одинаковый текст без деталей.
            VaultError::DecryptionFailed => write!(f, "decryption failed"),
            VaultError::NotFound => write!(f, "vault file not found"),
            VaultError::SymlinkDetected(_) => {
                write!(f, "symlink detected — refusing to open")
            }
            VaultError::TamperedVault => {
                write!(f, "vault integrity check failed — file may be tampered")
            }
            VaultError::PossibleRansomwareDetected => {
                write!(f, "possible ransomware activity detected")
            }
            VaultError::UnsupportedSchemaVersion { found, max } => {
                write!(f, "schema version {found} not supported (max: {max})")
            }
            // Не раскрываем внутреннюю строку наружу в Display для крипто-ошибок —
            // деталь доступна только в Debug для разработчика.
            VaultError::Crypto(_) => write!(f, "crypto operation failed"),
            VaultError::Serialization(_) => write!(f, "serialization error"),
            VaultError::Database(_) => write!(f, "database error"),
            VaultError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<std::io::Error> for VaultError {
    fn from(e: std::io::Error) -> Self {
        // ENOENT отображаем в NotFound, остальное — в Io.
        if e.kind() == std::io::ErrorKind::NotFound {
            VaultError::NotFound
        } else {
            VaultError::Io(e)
        }
    }
}

impl From<rusqlite::Error> for VaultError {
    fn from(e: rusqlite::Error) -> Self {
        VaultError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for VaultError {
    fn from(e: serde_json::Error) -> Self {
        VaultError::Serialization(e.to_string())
    }
}
