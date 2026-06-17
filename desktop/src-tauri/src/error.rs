use serde::Serialize;

/// All errors returned to the frontend via Tauri commands.
/// Using String as error type so Tauri serialises it cleanly.
#[derive(Debug, Serialize)]
pub enum AppError {
    VaultLocked,
    NotFound,
    InvalidId,
    LockPoisoned,
    Vault(String),
    Serialization(String),
    Other(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::VaultLocked => write!(f, "vault is locked"),
            AppError::NotFound => write!(f, "item not found"),
            AppError::InvalidId => write!(f, "invalid id"),
            AppError::LockPoisoned => write!(f, "internal state error"),
            AppError::Vault(e) => write!(f, "{e}"),
            AppError::Serialization(e) => write!(f, "serialization: {e}"),
            AppError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl From<core_vault::VaultError> for AppError {
    fn from(e: core_vault::VaultError) -> Self {
        AppError::Vault(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Serialization(e.to_string())
    }
}

impl From<uuid::Error> for AppError {
    fn from(_: uuid::Error) -> Self {
        AppError::InvalidId
    }
}
