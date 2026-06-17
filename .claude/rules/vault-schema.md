# Схема данных vault

## Файловая система

```
~/.vaultpass/
  vault.db          — SQLCipher БД (AES-256-CBC на каждую страницу)
  vault.salt         — 32 байта random (создаётся ОДИН РАЗ, никогда не меняется)
  vault.meta         — JSON: vault_id (UUID), schema_version, created_at
  vault.db.tmp       — временный файл при записи (удаляется после rename)
  vault_backup.db    — honeypot файл (рандомные байты, для обнаружения ransomware)
  sync.log           — Lamport timestamps для P2P синхронизации (v2)

# НЕ хранить в этой директории:
# - Любые plaintext копии паролей
# - Незашифрованные экспорты
# - Ключи или их производные
```

## SQLite таблицы

### vault (1 строка — метаданные хранилища)
```sql
CREATE TABLE vault (
    id                  TEXT NOT NULL,        -- UUID хранилища
    schema_version      INTEGER NOT NULL,     -- версия схемы (для миграций)
    encrypted_vault_key BLOB NOT NULL,        -- [ENC] Vault Key зашифрован Master Key
    key_nonce           BLOB NOT NULL,        -- nonce для расшифровки vault_key (24 байта)
    created_at          INTEGER NOT NULL,     -- Unix timestamp
    hint                TEXT                  -- [PLAIN] подсказка к мастер-паролю (не сам пароль!)
);
```

### items (записи хранилища)
```sql
CREATE TABLE items (
    id                  TEXT NOT NULL PRIMARY KEY,  -- UUID
    item_type           TEXT NOT NULL,              -- [PLAIN][IDX] login|card|note|identity|ssh_key
    title_encrypted     BLOB NOT NULL,              -- [ENC] название записи
    title_search_hash   BLOB NOT NULL,              -- [IDX] HMAC-SHA256(lowercase(title), search_key)
    payload_encrypted   BLOB NOT NULL,              -- [ENC] JSON со всеми полями
    payload_nonce       BLOB NOT NULL,              -- nonce для payload (24 байта, уникален для каждого save)
    folder_id           TEXT,                       -- [PLAIN][IDX] UUID папки (NULL = корень)
    favorite            INTEGER NOT NULL DEFAULT 0, -- [PLAIN] 0/1
    created_at          INTEGER NOT NULL,           -- [PLAIN] Unix timestamp
    updated_at          INTEGER NOT NULL,           -- [PLAIN][IDX] для сортировки и sync
    lamport_clock       INTEGER NOT NULL DEFAULT 0, -- [PLAIN] логические часы для P2P sync
    deleted             INTEGER NOT NULL DEFAULT 0  -- [PLAIN] soft delete (0/1)
);

CREATE INDEX idx_items_type     ON items(item_type) WHERE deleted = 0;
CREATE INDEX idx_items_folder   ON items(folder_id) WHERE deleted = 0;
CREATE INDEX idx_items_updated  ON items(updated_at DESC);
CREATE INDEX idx_items_search   ON items(title_search_hash) WHERE deleted = 0;
CREATE INDEX idx_items_favorite ON items(favorite) WHERE favorite = 1 AND deleted = 0;
```

### folders
```sql
CREATE TABLE folders (
    id              TEXT NOT NULL PRIMARY KEY,  -- UUID
    name_encrypted  BLOB NOT NULL,              -- [ENC] название папки
    name_nonce      BLOB NOT NULL,              -- nonce для name (24 байта)
    parent_id       TEXT,                       -- UUID родительской папки (NULL = корень)
    icon            TEXT,                       -- [PLAIN] emoji или имя иконки
    created_at      INTEGER NOT NULL
);
```

## Rust типы (core-vault/src/models.rs)

```rust
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Заголовок хранилища — загружается при открытии vault
pub struct VaultHeader {
    pub id: Uuid,
    pub schema_version: u32,
    pub encrypted_vault_key: Vec<u8>,   // зашифрован Master Key
    pub key_nonce: [u8; 24],
    pub created_at: i64,
    pub hint: Option<String>,
}

/// Запись в памяти (РАСШИФРОВАННАЯ — только пока vault открыт)
#[derive(Clone)]
pub struct Item {
    pub id: Uuid,
    pub item_type: ItemType,
    pub title: String,           // расшифрованное название
    pub payload: ItemPayload,    // расшифрованные данные
    pub folder_id: Option<Uuid>,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub lamport_clock: u64,
    pub deleted: bool,
}

/// Типы записей
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Login,
    Card,
    Note,
    Identity,
    SshKey,
}

/// Данные записи — полиморфная структура
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemPayload {
    Login {
        url: String,
        username: String,
        password: String,
        totp_secret: Option<String>,     // Base32 TOTP seed для 2FA
        notes: Option<String>,
        custom_fields: Vec<CustomField>,
        password_history: Vec<PasswordHistoryEntry>,
    },
    Card {
        cardholder: String,
        number: String,                  // хранится целиком, маскируется в UI
        expiry_month: u8,
        expiry_year: u16,
        cvv: String,
        notes: Option<String>,
    },
    Note {
        content: String,                 // markdown
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
        private_key: String,             // PEM формат
        public_key: Option<String>,
        passphrase: Option<String>,
        notes: Option<String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub label: String,
    pub value: String,
    pub hidden: bool,                    // скрытое поле (как пароль)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PasswordHistoryEntry {
    pub password: String,
    pub changed_at: i64,                 // Unix timestamp
}
```

## Правила работы с данными

### Что шифруется
- `title_encrypted` — название записи
- `payload_encrypted` — все поля записи (JSON сериализованный ItemPayload)
- `name_encrypted` — название папки
- `encrypted_vault_key` — Vault Key зашифрован Master Key

### Что НЕ шифруется (осознанный компромисс: нужно для фильтрации без расшифровки)
- `item_type` — тип записи
- `folder_id` — принадлежность к папке
- `favorite`, `deleted` — флаги
- `created_at`, `updated_at` — временные метки
- `lamport_clock` — для синхронизации
- `title_search_hash` — HMAC для поиска (раскрывает факт совпадения, не содержимое)

### Soft delete
```
deleted = 1 → запись скрыта, НЕ удалена физически
Причина: при P2P sync физически удалённая запись "воскреснет" с другого устройства.
Физическое удаление только через явную команду "очистить корзину".
При очистке: DELETE FROM items WHERE deleted = 1 AND updated_at < (now - 30 days)
```

### Schema версии
```
v1 (текущая): базовая схема
v2: добавить tags для записей (тег шифрован отдельно, хеш для фильтрации)
v3: multi-vault support (несколько независимых хранилищ)

Миграции: НИКОГДА не создавать plaintext копию данных в процессе миграции.
Миграция = atomic: открыть транзакцию → применить изменения → закрыть транзакцию.
```

## VaultError — типы ошибок

```rust
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("decryption failed")]        // ЕДИНСТВЕННАЯ ошибка расшифровки — без деталей
    DecryptionFailed,

    #[error("vault file not found")]
    NotFound,

    #[error("symlink detected — refusing to open")]
    SymlinkDetected(std::path::PathBuf),

    #[error("vault integrity check failed — file may be tampered")]
    TamperedVault,

    #[error("possible ransomware activity detected")]
    PossibleRansomwareDetected,

    #[error("schema version {found} not supported (max: {max})")]
    UnsupportedSchemaVersion { found: u32, max: u32 },

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ВАЖНО: никогда не раскрывать внутренние детали через Display
// DecryptionFailed — одна ошибка для: неверный пароль + повреждённый vault + MAC mismatch
```
