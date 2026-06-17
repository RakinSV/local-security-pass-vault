//! DDL схемы v1. Точно соответствует `.claude/rules/vault-schema.md`.

/// Полная схема vault (таблицы + индексы). Применяется к пустой in-memory БД.
pub const SCHEMA_V1: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE vault (
    id                  TEXT NOT NULL,
    schema_version      INTEGER NOT NULL,
    encrypted_vault_key BLOB NOT NULL,
    key_nonce           BLOB NOT NULL,
    created_at          INTEGER NOT NULL,
    hint                TEXT
);

CREATE TABLE items (
    id                  TEXT NOT NULL PRIMARY KEY,
    item_type           TEXT NOT NULL,
    title_encrypted     BLOB NOT NULL,
    title_search_hash   BLOB NOT NULL,
    payload_encrypted   BLOB NOT NULL,
    payload_nonce       BLOB NOT NULL,
    folder_id           TEXT,
    favorite            INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    lamport_clock       INTEGER NOT NULL DEFAULT 0,
    deleted             INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_items_type     ON items(item_type) WHERE deleted = 0;
CREATE INDEX idx_items_folder   ON items(folder_id) WHERE deleted = 0;
CREATE INDEX idx_items_updated  ON items(updated_at DESC);
CREATE INDEX idx_items_search   ON items(title_search_hash) WHERE deleted = 0;
CREATE INDEX idx_items_favorite ON items(favorite) WHERE favorite = 1 AND deleted = 0;

CREATE TABLE folders (
    id              TEXT NOT NULL PRIMARY KEY,
    name_encrypted  BLOB NOT NULL,
    name_nonce      BLOB NOT NULL,
    parent_id       TEXT,
    icon            TEXT,
    created_at      INTEGER NOT NULL
);
"#;
