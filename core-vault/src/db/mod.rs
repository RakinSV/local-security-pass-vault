//! Слой хранения: in-memory SQLite. Оперирует СЫРЫМИ зашифрованными строками —
//! криптография живёт в модуле `vault`, не здесь. Файл БД на диске шифруется
//! целиком (db_key + XChaCha20-Poly1305) при сохранении (см. `vault`).

mod schema;

use crate::error::{Result, VaultError};
use rusqlite::serialize::OwnedData;
use rusqlite::{params, Connection, DatabaseName, OptionalExtension};
use std::ptr::NonNull;
use uuid::Uuid;

/// Сырая строка таблицы `items` — все ENC-поля как зашифрованные байты.
pub struct ItemRow {
    pub id: Uuid,
    pub item_type: String,
    pub title_encrypted: Vec<u8>,
    pub title_search_hash: Vec<u8>,
    pub payload_encrypted: Vec<u8>,
    pub payload_nonce: Vec<u8>,
    pub folder_id: Option<Uuid>,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub lamport_clock: i64,
    pub deleted: bool,
    pub source_tag: Option<String>,
}

/// Сырая строка таблицы `folders`.
pub struct FolderRow {
    pub id: Uuid,
    pub name_encrypted: Vec<u8>,
    pub name_nonce: Vec<u8>,
    pub parent_id: Option<Uuid>,
    pub icon: Option<String>,
    pub created_at: i64,
}

/// Сырая строка таблицы `vault` (заголовок).
pub struct VaultRow {
    pub id: Uuid,
    pub schema_version: u32,
    pub encrypted_vault_key: Vec<u8>,
    pub key_nonce: Vec<u8>,
    pub created_at: i64,
    pub hint: Option<String>,
    /// Зашифрованный Base32-секрет TOTP (vault_key + encrypt_field). None = 2FA выключена.
    pub totp_secret_encrypted: Option<Vec<u8>>,
    /// Nonce для totp_secret_encrypted (24 байта).
    pub totp_secret_nonce: Option<Vec<u8>>,
}

/// In-memory соединение с БД vault.
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Создаёт пустую БД в памяти и применяет текущую схему (v1 + все миграции).
    pub fn create_empty() -> Result<Db> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(schema::SCHEMA_V1)?;
        let db = Db { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Загружает БД из расшифрованных байт (формат sqlite3_serialize).
    pub fn from_plaintext(bytes: &[u8]) -> Result<Db> {
        let mut conn = Connection::open_in_memory()?;
        let owned = owned_from_slice(bytes)?;
        // read_only = false → БД resizeable, можно писать.
        conn.deserialize(DatabaseName::Main, owned, false)?;
        let db = Db { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Сериализует БД в байты для последующего шифрования и записи на диск.
    pub fn to_plaintext(&self) -> Result<Vec<u8>> {
        let data = self.conn.serialize(DatabaseName::Main)?;
        Ok(data.to_vec())
    }

    // ── migrations ───────────────────────────────────────────────────────────

    /// Идемпотентные миграции: добавляет колонки, отсутствующие в старых vault-файлах.
    fn run_migrations(&self) -> Result<()> {
        let has_source_tag: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('items') WHERE name = 'source_tag'",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        if has_source_tag == 0 {
            self.conn.execute_batch(schema::MIGRATE_V1_TO_V2)?;
        }
        let has_totp: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('vault') WHERE name = 'totp_secret_encrypted'",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        if has_totp == 0 {
            self.conn.execute_batch(schema::MIGRATE_V2_TO_V3)?;
        }
        Ok(())
    }

    // ── vault header ─────────────────────────────────────────────────────────

    pub fn write_vault_row(&self, row: &VaultRow) -> Result<()> {
        // Таблица vault — одна строка: очищаем и вставляем.
        self.conn.execute("DELETE FROM vault", [])?;
        self.conn.execute(
            "INSERT INTO vault
                (id, schema_version, encrypted_vault_key, key_nonce, created_at, hint,
                 totp_secret_encrypted, totp_secret_nonce)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                row.id.to_string(),
                row.schema_version,
                row.encrypted_vault_key,
                row.key_nonce,
                row.created_at,
                row.hint,
                row.totp_secret_encrypted,
                row.totp_secret_nonce,
            ],
        )?;
        Ok(())
    }

    pub fn read_vault_row(&self) -> Result<VaultRow> {
        self.conn
            .query_row(
                "SELECT id, schema_version, encrypted_vault_key, key_nonce, created_at, hint,
                        totp_secret_encrypted, totp_secret_nonce
                 FROM vault LIMIT 1",
                [],
                |r| {
                    Ok(VaultRow {
                        id: parse_uuid(&r.get::<_, String>(0)?)?,
                        schema_version: r.get(1)?,
                        encrypted_vault_key: r.get(2)?,
                        key_nonce: r.get(3)?,
                        created_at: r.get(4)?,
                        hint: r.get(5)?,
                        totp_secret_encrypted: r.get(6)?,
                        totp_secret_nonce: r.get(7)?,
                    })
                },
            )
            .optional()?
            .ok_or(VaultError::TamperedVault)
    }

    // ── items ────────────────────────────────────────────────────────────────

    /// Вставка или обновление записи (по PRIMARY KEY id).
    pub fn upsert_item(&self, row: &ItemRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO items
                (id, item_type, title_encrypted, title_search_hash, payload_encrypted,
                 payload_nonce, folder_id, favorite, created_at, updated_at,
                 lamport_clock, deleted, source_tag)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET
                item_type=excluded.item_type,
                title_encrypted=excluded.title_encrypted,
                title_search_hash=excluded.title_search_hash,
                payload_encrypted=excluded.payload_encrypted,
                payload_nonce=excluded.payload_nonce,
                folder_id=excluded.folder_id,
                favorite=excluded.favorite,
                updated_at=excluded.updated_at,
                lamport_clock=excluded.lamport_clock,
                deleted=excluded.deleted,
                source_tag=excluded.source_tag",
            params![
                row.id.to_string(),
                row.item_type,
                row.title_encrypted,
                row.title_search_hash,
                row.payload_encrypted,
                row.payload_nonce,
                row.folder_id.map(|u| u.to_string()),
                row.favorite as i64,
                row.created_at,
                row.updated_at,
                row.lamport_clock,
                row.deleted as i64,
                row.source_tag,
            ],
        )?;
        Ok(())
    }

    pub fn get_item(&self, id: &Uuid) -> Result<Option<ItemRow>> {
        self.conn
            .query_row(
                "SELECT id, item_type, title_encrypted, title_search_hash, payload_encrypted,
                        payload_nonce, folder_id, favorite, created_at, updated_at,
                        lamport_clock, deleted, source_tag
                 FROM items WHERE id = ?1",
                params![id.to_string()],
                map_item_row,
            )
            .optional()
            .map_err(VaultError::from)
    }

    /// Все живые (не удалённые) записи, новые сверху.
    pub fn list_items(&self) -> Result<Vec<ItemRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, title_encrypted, title_search_hash, payload_encrypted,
                    payload_nonce, folder_id, favorite, created_at, updated_at,
                    lamport_clock, deleted, source_tag
             FROM items WHERE deleted = 0 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], map_item_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Поиск по точному совпадению поискового HMAC (idx_items_search).
    pub fn find_by_search_hash(&self, hash: &[u8]) -> Result<Vec<ItemRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, title_encrypted, title_search_hash, payload_encrypted,
                    payload_nonce, folder_id, favorite, created_at, updated_at,
                    lamport_clock, deleted, source_tag
             FROM items WHERE title_search_hash = ?1 AND deleted = 0",
        )?;
        let rows = stmt.query_map(params![hash], map_item_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Все уникальные non-null source_tag живых записей.
    pub fn list_source_tags(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT source_tag FROM items WHERE deleted = 0 AND source_tag IS NOT NULL ORDER BY source_tag",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Переименовывает (или очищает) source_tag у всех записей с указанным тегом.
    pub fn update_source_tag_bulk(&self, old_tag: &str, new_tag: Option<&str>) -> Result<usize> {
        let n = self.conn.execute(
            "UPDATE items SET source_tag = ?2 WHERE source_tag = ?1 AND deleted = 0",
            params![old_tag, new_tag],
        )?;
        Ok(n)
    }

    /// Мягкое удаление (deleted = 1). Физически не удаляет (см. vault-schema.md).
    pub fn soft_delete_item(&self, id: &Uuid, updated_at: i64, lamport_clock: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET deleted = 1, updated_at = ?2, lamport_clock = ?3 WHERE id = ?1",
            params![id.to_string(), updated_at, lamport_clock],
        )?;
        Ok(())
    }

    /// Физическая очистка корзины: удалить записи, помеченные deleted, старше cutoff.
    pub fn purge_deleted(&self, older_than: i64) -> Result<usize> {
        let n = self.conn.execute(
            "DELETE FROM items WHERE deleted = 1 AND updated_at <= ?1",
            params![older_than],
        )?;
        Ok(n)
    }

    /// Список удалённых записей (deleted = 1), новые сверху.
    pub fn list_deleted_items(&self) -> Result<Vec<ItemRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, title_encrypted, title_search_hash, payload_encrypted, payload_nonce,
                    folder_id, favorite, created_at, updated_at, lamport_clock, deleted, source_tag
             FROM items WHERE deleted = 1 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], map_item_row)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    /// Восстанавливает запись из корзины (deleted → 0).
    pub fn restore_item(&self, id: &Uuid, updated_at: i64, lamport_clock: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET deleted = 0, updated_at = ?2, lamport_clock = ?3 WHERE id = ?1",
            params![id.to_string(), updated_at, lamport_clock],
        )?;
        Ok(())
    }

    /// Физическое удаление одной записи (безвозвратно).
    pub fn purge_item(&self, id: &Uuid) -> Result<()> {
        self.conn.execute("DELETE FROM items WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    // ── folders ──────────────────────────────────────────────────────────────

    pub fn upsert_folder(&self, row: &FolderRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO folders (id, name_encrypted, name_nonce, parent_id, icon, created_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET
                name_encrypted=excluded.name_encrypted,
                name_nonce=excluded.name_nonce,
                parent_id=excluded.parent_id,
                icon=excluded.icon",
            params![
                row.id.to_string(),
                row.name_encrypted,
                row.name_nonce,
                row.parent_id.map(|u| u.to_string()),
                row.icon,
                row.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_folders(&self) -> Result<Vec<FolderRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name_encrypted, name_nonce, parent_id, icon, created_at FROM folders",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(FolderRow {
                id: parse_uuid(&r.get::<_, String>(0)?)?,
                name_encrypted: r.get(1)?,
                name_nonce: r.get(2)?,
                parent_id: parse_uuid_opt(r.get::<_, Option<String>>(3)?)?,
                icon: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Удаляет папку; записи внутри получают folder_id = NULL.
    pub fn delete_folder(&self, id: &Uuid) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET folder_id = NULL WHERE folder_id = ?1",
            params![id.to_string()],
        )?;
        self.conn.execute("DELETE FROM folders WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Маппинг строки `items` в `ItemRow`.
fn map_item_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ItemRow> {
    Ok(ItemRow {
        id: parse_uuid(&r.get::<_, String>(0)?)?,
        item_type: r.get(1)?,
        title_encrypted: r.get(2)?,
        title_search_hash: r.get(3)?,
        payload_encrypted: r.get(4)?,
        payload_nonce: r.get(5)?,
        folder_id: parse_uuid_opt(r.get::<_, Option<String>>(6)?)?,
        favorite: r.get::<_, i64>(7)? != 0,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
        lamport_clock: r.get(10)?,
        deleted: r.get::<_, i64>(11)? != 0,
        source_tag: r.get(12)?,
    })
}

fn parse_uuid(s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad uuid")),
        )
    })
}

fn parse_uuid_opt(s: Option<String>) -> rusqlite::Result<Option<Uuid>> {
    match s {
        Some(s) => Ok(Some(parse_uuid(&s)?)),
        None => Ok(None),
    }
}

/// Создаёт `OwnedData` для `deserialize`: память ДОЛЖНА быть выделена
/// `sqlite3_malloc`, т.к. SQLite освобождает её через `sqlite3_free`.
fn owned_from_slice(data: &[u8]) -> Result<OwnedData> {
    let len = data.len();
    if len == 0 {
        return Err(VaultError::Database("empty database image".into()));
    }
    // SAFETY: выделяем через sqlite3_malloc64, копируем байты, передаём владение
    // SQLite через OwnedData::from_raw_nonnull (контракт: указатель от sqlite3_malloc).
    unsafe {
        let ptr = rusqlite::ffi::sqlite3_malloc64(len as u64) as *mut u8;
        let nn = NonNull::new(ptr)
            .ok_or_else(|| VaultError::Database("sqlite3_malloc failed".into()))?;
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, len);
        Ok(OwnedData::from_raw_nonnull(nn, len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(id: Uuid) -> ItemRow {
        ItemRow {
            id,
            item_type: "login".into(),
            title_encrypted: vec![1, 2, 3],
            title_search_hash: vec![9; 32],
            payload_encrypted: vec![4, 5, 6],
            payload_nonce: vec![7; 24],
            folder_id: None,
            favorite: false,
            created_at: 1000,
            updated_at: 1000,
            lamport_clock: 1,
            deleted: false,
            source_tag: None,
        }
    }

    #[test]
    fn empty_db_has_schema() {
        let db = Db::create_empty().unwrap();
        assert!(db.list_items().unwrap().is_empty());
    }

    #[test]
    fn upsert_get_item() {
        let db = Db::create_empty().unwrap();
        let id = Uuid::new_v4();
        db.upsert_item(&sample_item(id)).unwrap();
        let got = db.get_item(&id).unwrap().unwrap();
        assert_eq!(got.payload_encrypted, vec![4, 5, 6]);
        assert_eq!(got.title_search_hash, vec![9; 32]);
    }

    #[test]
    fn soft_delete_hides_item() {
        let db = Db::create_empty().unwrap();
        let id = Uuid::new_v4();
        db.upsert_item(&sample_item(id)).unwrap();
        db.soft_delete_item(&id, 2000, 2).unwrap();
        assert!(db.list_items().unwrap().is_empty());
        // запись всё ещё физически существует
        assert!(db.get_item(&id).unwrap().unwrap().deleted);
    }

    #[test]
    fn search_by_hash() {
        let db = Db::create_empty().unwrap();
        let id = Uuid::new_v4();
        db.upsert_item(&sample_item(id)).unwrap();
        let found = db.find_by_search_hash(&[9u8; 32]).unwrap();
        assert_eq!(found.len(), 1);
        assert!(db.find_by_search_hash(&[0u8; 32]).unwrap().is_empty());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let db = Db::create_empty().unwrap();
        let id = Uuid::new_v4();
        db.upsert_item(&sample_item(id)).unwrap();
        let bytes = db.to_plaintext().unwrap();
        assert!(!bytes.is_empty());

        let db2 = Db::from_plaintext(&bytes).unwrap();
        let got = db2.get_item(&id).unwrap().unwrap();
        assert_eq!(got.payload_encrypted, vec![4, 5, 6]);
    }
}
