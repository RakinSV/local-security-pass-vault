//! core-vault — криптографическое ядро и модель данных VaultPass.
//!
//! Все крипто-операции проходят через libsodium (статически слинкован).
//! Правила, которые НЕЛЬЗЯ нарушать, — в `.claude/rules/crypto.md`.

pub mod crypto;
pub mod db;
pub mod error;
pub mod models;
pub mod sodium;
pub mod vault;

pub use error::{Result, VaultError};
pub use vault::Vault;
pub use vault::backup;

/// Инициализация libsodium. Должна быть вызвана один раз при старте приложения,
/// ДО любых крипто-операций. Идемпотентна и потокобезопасна.
pub fn init() -> std::result::Result<(), &'static str> {
    sodium::init()
}
