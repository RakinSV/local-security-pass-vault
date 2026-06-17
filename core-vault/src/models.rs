//! Модель данных vault. См. `.claude/rules/vault-schema.md`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Текущая версия схемы БД.
pub const SCHEMA_VERSION: u32 = 1;

/// Заголовок хранилища — загружается при открытии vault (таблица `vault`).
#[derive(Clone, Debug)]
pub struct VaultHeader {
    pub id: Uuid,
    pub schema_version: u32,
    /// Vault Key, зашифрованный Master Key (encryption_key из KDF).
    pub encrypted_vault_key: Vec<u8>,
    pub key_nonce: [u8; 24],
    pub created_at: i64,
    /// Подсказка к мастер-паролю (PLAIN, не сам пароль!).
    pub hint: Option<String>,
}

/// Тип записи (PLAIN — нужен для фильтрации без расшифровки).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Login,
    Card,
    Note,
    Identity,
    SshKey,
}

impl ItemType {
    /// Стабильное строковое представление для колонки `item_type`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemType::Login => "login",
            ItemType::Card => "card",
            ItemType::Note => "note",
            ItemType::Identity => "identity",
            ItemType::SshKey => "ssh_key",
        }
    }

    // Инхерентный хелпер: возвращает Option, а не Result как FromStr — поэтому
    // намеренно не реализуем трейт.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "login" => Some(ItemType::Login),
            "card" => Some(ItemType::Card),
            "note" => Some(ItemType::Note),
            "identity" => Some(ItemType::Identity),
            "ssh_key" => Some(ItemType::SshKey),
            _ => None,
        }
    }
}

/// Запись в памяти (РАСШИФРОВАННАЯ — только пока vault открыт).
#[derive(Clone, Debug)]
pub struct Item {
    pub id: Uuid,
    pub item_type: ItemType,
    pub title: String,
    pub payload: ItemPayload,
    pub folder_id: Option<Uuid>,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub lamport_clock: u64,
    pub deleted: bool,
}

/// Полиморфные данные записи. Сериализуется в JSON в `payload_encrypted`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemPayload {
    Login {
        url: String,
        username: String,
        password: String,
        totp_secret: Option<String>,
        notes: Option<String>,
        #[serde(default)]
        custom_fields: Vec<CustomField>,
        #[serde(default)]
        password_history: Vec<PasswordHistoryEntry>,
    },
    Card {
        cardholder: String,
        number: String,
        expiry_month: u8,
        expiry_year: u16,
        cvv: String,
        notes: Option<String>,
    },
    Note {
        content: String,
    },
    Identity {
        first_name: Option<String>,
        last_name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
        address: Option<String>,
        passport: Option<String>,
        notes: Option<String>,
    },
    SshKey {
        private_key: String,
        public_key: Option<String>,
        passphrase: Option<String>,
        notes: Option<String>,
    },
}

impl ItemPayload {
    /// Тип записи, соответствующий варианту payload.
    pub fn item_type(&self) -> ItemType {
        match self {
            ItemPayload::Login { .. } => ItemType::Login,
            ItemPayload::Card { .. } => ItemType::Card,
            ItemPayload::Note { .. } => ItemType::Note,
            ItemPayload::Identity { .. } => ItemType::Identity,
            ItemPayload::SshKey { .. } => ItemType::SshKey,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CustomField {
    pub label: String,
    pub value: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PasswordHistoryEntry {
    pub password: String,
    pub changed_at: i64,
}

/// Папка (таблица `folders`).
#[derive(Clone, Debug)]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub icon: Option<String>,
    pub created_at: i64,
}
