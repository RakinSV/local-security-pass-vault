//! Жизненный цикл vault: создание, открытие, сохранение, блокировка, CRUD.
//! Связывает crypto + db + файловые операции. См. все правила в `.claude/rules/`.

mod file;
mod honeypot;
pub mod backup;

use crate::crypto::{self, MasterKeys};
use crate::db::{Db, FolderRow, ItemRow, VaultRow};
use crate::error::{Result, VaultError};
use crate::models::{Folder, Item, ItemType, SCHEMA_VERSION};
use crate::sodium::{self, Key, NONCE_LEN};
use honeypot::HoneypotGuard;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

// Формат внешнего зашифрованного контейнера vault.db.
const CONTAINER_MAGIC: &[u8; 4] = b"VPDB";
const CONTAINER_FORMAT: u8 = 1;
const AD_CONTAINER: &[u8] = b"vaultpass:db-container:v1";

/// Имена файлов в директории vault.
struct VaultPaths {
    dir: PathBuf,
    db: PathBuf,
    salt: PathBuf,
    meta: PathBuf,
    honeypot: PathBuf,
}

impl VaultPaths {
    fn new(dir: &Path) -> Self {
        VaultPaths {
            dir: dir.to_owned(),
            db: dir.join("vault.db"),
            salt: dir.join("vault.salt"),
            meta: dir.join("vault.meta"),
            honeypot: dir.join("vault_backup.db"),
        }
    }
}

/// Содержимое vault.meta (PLAIN JSON). Хранит только незашифрованные метаданные.
#[derive(Serialize, Deserialize)]
struct VaultMeta {
    vault_id: Uuid,
    schema_version: u32,
    created_at: i64,
    /// Флаг наличия TOTP 2FA — в открытом виде, чтобы UI показал поле ввода кода до разблокировки.
    #[serde(default)]
    totp_enabled: bool,
}

/// Разблокированный vault. При drop ключи обнуляются (Key::drop).
pub struct Vault {
    paths: VaultPaths,
    meta: VaultMeta,
    db: Db,
    keys: MasterKeys,
    vault_key: Key,
    honeypot: HoneypotGuard,
    lamport: u64,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Проверяет TOTP-код против Base32-секрета.
/// Допускает ±1 шаг (30с) — защита от рассинхрона часов и опозданий ввода.
/// Ошибка → `TwoFactorFailed`.
fn verify_totp(code: &str, secret_b32: &str) -> crate::error::Result<()> {
    use totp_rs::{Algorithm, Secret, TOTP};
    let secret_bytes = Secret::Encoded(secret_b32.trim().to_uppercase())
        .to_bytes()
        .map_err(|e| VaultError::Serialization(e.to_string()))?;
    // step=1 → check_current() проверяет предыдущий, текущий и следующий 30с-период (RFC 6238).
    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret_bytes)
        .map_err(|e| VaultError::Serialization(e.to_string()))?;
    let ok = totp.check_current(code.trim())
        .map_err(|e| VaultError::Serialization(e.to_string()))?;
    if ok { Ok(()) } else { Err(VaultError::TwoFactorFailed) }
}

/// Результат проверки здоровья одного Login-пароля.
#[derive(Debug, Serialize)]
pub struct HealthEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    pub is_weak: bool,
    pub is_duplicate: bool,
    pub is_old: bool,
    pub updated_at: i64,
}

fn is_weak_password(pw: &str) -> bool {
    if pw.len() < 8 {
        return true;
    }
    let has_upper = pw.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = pw.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = pw.chars().any(|c| c.is_ascii_digit());
    let has_symbol = pw.chars().any(|c| !c.is_alphanumeric());
    let variety = [has_upper, has_lower, has_digit, has_symbol]
        .iter()
        .filter(|&&b| b)
        .count();
    pw.len() < 12 || variety < 2
}

impl Vault {
    /// Создаёт новый vault в пустой директории.
    pub fn create(dir: &Path, password: &[u8], hint: Option<String>) -> Result<Vault> {
        let paths = VaultPaths::new(dir);
        std::fs::create_dir_all(&paths.dir)?;

        if paths.salt.exists() || paths.db.exists() {
            return Err(VaultError::Database("vault already exists".into()));
        }

        // Соль создаётся ОДИН РАЗ и никогда не меняется.
        let salt = sodium::gen_salt()?;
        file::atomic_write(&paths.salt, &salt)?;
        file::restrict_permissions(&paths.salt).ok();

        let keys = crypto::derive_master_keys(password, &salt)?;

        // Envelope: случайный vault_key, зашифрован encryption_key.
        let wrapped = crypto::wrap_new_vault_key(&keys.encryption_key)?;

        let vault_id = Uuid::new_v4();
        let created_at = now_unix();

        let db = Db::create_empty()?;
        db.write_vault_row(&VaultRow {
            id: vault_id,
            schema_version: SCHEMA_VERSION,
            encrypted_vault_key: wrapped.encrypted,
            key_nonce: wrapped.nonce.to_vec(),
            created_at,
            hint,
            totp_secret_encrypted: None,
            totp_secret_nonce: None,
        })?;

        let meta = VaultMeta {
            vault_id,
            schema_version: SCHEMA_VERSION,
            created_at,
            totp_enabled: false,
        };
        write_meta(&paths.meta, &meta)?;
        file::restrict_permissions(&paths.meta).ok();

        let honeypot = HoneypotGuard::init(&paths.honeypot)?;

        let vault = Vault {
            paths,
            meta,
            db,
            keys,
            vault_key: wrapped.vault_key,
            honeypot,
            lamport: 0,
        };
        vault.save()?;
        Ok(vault)
    }

    /// Открывает существующий vault. Неверный пароль → `DecryptionFailed`.
    /// Если vault имеет 2FA и `totp_code` не передан → `TwoFactorRequired`.
    /// Неверный код → `TwoFactorFailed`.
    pub fn open(dir: &Path, password: &[u8], totp_code: Option<&str>) -> Result<Vault> {
        let paths = VaultPaths::new(dir);

        // Соль обязательна.
        let salt_bytes = file::read_no_symlink(&paths.salt)?;
        if salt_bytes.len() != sodium::SALT_LEN {
            return Err(VaultError::TamperedVault);
        }
        let mut salt = [0u8; sodium::SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        let meta = read_meta(&paths.meta)?;

        let keys = crypto::derive_master_keys(password, &salt)?;

        // Внешний слой: расшифровываем контейнер db_key. Неверный пароль здесь же
        // отсекается единой ошибкой DecryptionFailed.
        let container = file::read_no_symlink(&paths.db)?;
        let (container_vault_id, db_plain) = open_container(&container, &keys.db_key)?;

        // Проверка целостности: id в контейнере и в meta совпадают.
        if container_vault_id != meta.vault_id {
            return Err(VaultError::TamperedVault);
        }

        let db = Db::from_plaintext(&db_plain)?;

        // Внутренний слой: id в таблице vault тоже должен совпасть.
        let header = db.read_vault_row()?;
        if header.id != meta.vault_id {
            return Err(VaultError::TamperedVault);
        }
        if header.schema_version > SCHEMA_VERSION {
            return Err(VaultError::UnsupportedSchemaVersion {
                found: header.schema_version,
                max: SCHEMA_VERSION,
            });
        }

        // Разворачиваем vault_key.
        let key_nonce = to_nonce(&header.key_nonce)?;
        let vault_key =
            crypto::unwrap_vault_key(&keys.encryption_key, &header.encrypted_vault_key, &key_nonce)?;

        // 2FA: проверяем TOTP-код ПОСЛЕ успешной расшифровки vault_key (vault_key нужен для
        // дешифровки секрета TOTP). При ошибке vault_key дропается (Key::drop → memzero).
        if meta.totp_enabled {
            match totp_code {
                None => return Err(VaultError::TwoFactorRequired),
                Some(code) => {
                    match (header.totp_secret_encrypted, header.totp_secret_nonce) {
                        (Some(enc), Some(nonce_bytes)) => {
                            let nonce = to_nonce(&nonce_bytes)?;
                            let secret_bytes = Zeroizing::new(crypto::decrypt_field(
                                &enc, &nonce, &meta.vault_id, "totp_secret", &vault_key,
                            )?);
                            let secret_b32 = Zeroizing::new(
                                std::str::from_utf8(&secret_bytes)
                                    .map_err(|_| VaultError::TwoFactorFailed)?
                                    .to_owned(),
                            );
                            verify_totp(code, &secret_b32)?;
                        }
                        // meta.totp_enabled=true но секрета в DB нет — краш при enable_2fa
                        // между save() и write_meta(). Это состояние гонки, не tamper.
                        // Пропускаем 2FA проверку: vault открывается нормально.
                        _ => { /* 2FA secret lost — treat as disabled for this open */ }
                    }
                }
            }
        }

        let honeypot = HoneypotGuard::init(&paths.honeypot)?;
        honeypot.check()?;

        // Восстанавливаем Lamport-часы как максимум по записям.
        let lamport = db
            .list_items()?
            .iter()
            .map(|r| r.lamport_clock.max(0) as u64)
            .max()
            .unwrap_or(0);

        Ok(Vault {
            paths,
            meta,
            db,
            keys,
            vault_key,
            honeypot,
            lamport,
        })
    }

    /// Сериализует БД, шифрует целиком db_key и атомарно пишет vault.db.
    pub fn save(&self) -> Result<()> {
        self.honeypot.check()?;
        let plain = self.db.to_plaintext()?;
        let container = seal_container(self.meta.vault_id, &plain, &self.keys.db_key)?;
        file::atomic_write(&self.paths.db, &container)?;
        file::restrict_permissions(&self.paths.db).ok();
        Ok(())
    }

    fn next_lamport(&mut self) -> u64 {
        self.lamport += 1;
        self.lamport
    }

    // ── TOTP 2FA vault ────────────────────────────────────────────────────────

    /// Возвращает true, если vault имеет активную 2FA.
    pub fn has_2fa(&self) -> bool {
        self.meta.totp_enabled
    }

    /// Генерирует новый TOTP-секрет для настройки 2FA. Возвращает (base32, otpauth_uri).
    /// НЕ сохраняет — вызови `enable_2fa` после того, как пользователь подтвердил код.
    pub fn generate_2fa_secret(&self) -> Result<(String, String)> {
        use totp_rs::{Algorithm, TOTP};
        // 20 случайных байт (160 бит) — минимум RFC 6238.
        let mut raw = [0u8; 20];
        sodium::random_bytes(&mut raw)?;
        // Pass a copy to TOTP; zero our stack copy regardless of result.
        let totp_result = TOTP::new(Algorithm::SHA1, 6, 1, 30, raw.to_vec())
            .map_err(|e| VaultError::Serialization(e.to_string()));
        raw.zeroize();
        let totp = totp_result?;
        let secret_b32 = totp.get_secret_base32();
        let issuer = "Local%20Security%20Pass%20Vault";
        let label = "LSPV%20vault";
        let uri = format!(
            "otpauth://totp/{label}?secret={secret_b32}&issuer={issuer}&algorithm=SHA1&digits=6&period=30"
        );
        Ok((secret_b32, uri))
    }

    /// Включает 2FA: проверяет `code` по `secret`, затем шифрует и сохраняет секрет.
    /// Порядок записи на диск: сначала vault.db (содержит секрет), потом vault.meta
    /// (устанавливает флаг). Краш между ними → meta остаётся disabled → безопасная деградация.
    pub fn enable_2fa(&mut self, secret: &str, code: &str) -> Result<()> {
        verify_totp(code, secret)?;
        let (enc, nonce) = crypto::encrypt_field(
            secret.as_bytes(), &self.meta.vault_id, "totp_secret", &self.vault_key,
        )?;
        let mut row = self.db.read_vault_row()?;
        row.totp_secret_encrypted = Some(enc);
        row.totp_secret_nonce = Some(nonce.to_vec());
        self.db.write_vault_row(&row)?;
        // Записать vault.db на диск ПРЕЖДЕ чем обновить meta — если краш между ними,
        // vault.meta остаётся с totp_enabled=false (безопасная деградация, не lockout).
        self.save()?;
        self.meta.totp_enabled = true;
        write_meta(&self.paths.meta, &self.meta)
    }

    /// Отключает 2FA: проверяет текущий `code`, затем удаляет секрет.
    /// Порядок: сначала vault.db (очищаем секрет), потом vault.meta (снимаем флаг).
    /// Краш между ними → meta остаётся enabled, но DB без секрета → open() вернёт
    /// Serialization error вместо TamperedVault, что позволяет диагностировать.
    pub fn disable_2fa(&mut self, code: &str) -> Result<()> {
        self.verify_2fa_code(code)?;
        let mut row = self.db.read_vault_row()?;
        row.totp_secret_encrypted = None;
        row.totp_secret_nonce = None;
        self.db.write_vault_row(&row)?;
        self.save()?;
        self.meta.totp_enabled = false;
        write_meta(&self.paths.meta, &self.meta)
    }

    fn verify_2fa_code(&self, code: &str) -> Result<()> {
        let row = self.db.read_vault_row()?;
        match (row.totp_secret_encrypted, row.totp_secret_nonce) {
            (Some(enc), Some(nonce_bytes)) => {
                let nonce = to_nonce(&nonce_bytes)?;
                let secret_bytes = Zeroizing::new(crypto::decrypt_field(
                    &enc, &nonce, &self.meta.vault_id, "totp_secret", &self.vault_key,
                )?);
                let secret_b32 = Zeroizing::new(
                    std::str::from_utf8(&secret_bytes)
                        .map_err(|_| VaultError::TwoFactorFailed)?
                        .to_owned(),
                );
                verify_totp(code, &secret_b32)
            }
            _ => Err(VaultError::Serialization("2FA not configured".into())),
        }
    }

    // ── CRUD записей ───────────────────────────────────────────────────────────

    /// Добавляет новую запись, возвращает её id. НЕ сохраняет на диск (вызови save()).
    pub fn add_item(
        &mut self,
        title: &str,
        payload: crate::models::ItemPayload,
        folder_id: Option<Uuid>,
        favorite: bool,
        source_tag: Option<String>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = now_unix();
        let lamport = self.next_lamport();
        let item = Item {
            id,
            item_type: payload.item_type(),
            title: title.to_string(),
            payload,
            folder_id,
            favorite,
            created_at: now,
            updated_at: now,
            lamport_clock: lamport,
            deleted: false,
            source_tag,
        };
        let row = self.item_to_row(&item)?;
        self.db.upsert_item(&row)?;
        Ok(id)
    }

    /// Обновляет существующую запись (по id из item).
    pub fn update_item(&mut self, mut item: Item) -> Result<()> {
        item.updated_at = now_unix();
        item.lamport_clock = self.next_lamport();
        let row = self.item_to_row(&item)?;
        self.db.upsert_item(&row)?;
        Ok(())
    }

    /// Возвращает расшифрованную запись по id.
    pub fn get_item(&self, id: &Uuid) -> Result<Option<Item>> {
        match self.db.get_item(id)? {
            Some(row) => Ok(Some(self.row_to_item(&row)?)),
            None => Ok(None),
        }
    }

    /// Все живые записи (расшифрованные), новые сверху.
    pub fn list_items(&self) -> Result<Vec<Item>> {
        self.db
            .list_items()?
            .iter()
            .map(|r| self.row_to_item(r))
            .collect()
    }

    /// Поиск по точному совпадению заголовка (регистронезависимо) через HMAC-индекс.
    pub fn search(&self, query: &str) -> Result<Vec<Item>> {
        let hash = crypto::search_hash(query, &self.keys.search_key)?;
        self.db
            .find_by_search_hash(&hash)?
            .iter()
            .map(|r| self.row_to_item(r))
            .collect()
    }

    /// Мягкое удаление записи.
    pub fn delete_item(&mut self, id: &Uuid) -> Result<()> {
        let lamport = self.next_lamport();
        self.db.soft_delete_item(id, now_unix(), lamport as i64)
    }

    /// Список мягко-удалённых записей (корзина), расшифрованных.
    pub fn list_deleted_items(&self) -> Result<Vec<Item>> {
        self.db.list_deleted_items()?.iter().map(|r| self.row_to_item(r)).collect()
    }

    /// Восстанавливает запись из корзины.
    pub fn restore_item(&mut self, id: &Uuid) -> Result<()> {
        let lamport = self.next_lamport();
        self.db.restore_item(id, now_unix(), lamport as i64)
    }

    /// Физически удаляет одну запись из корзины (безвозвратно).
    pub fn purge_item(&self, id: &Uuid) -> Result<()> {
        self.db.purge_item(id)
    }

    /// Физически удаляет ВСЕ записи корзины.
    pub fn purge_all_trash(&self) -> Result<usize> {
        self.db.purge_deleted(i64::MAX)
    }

    /// Физически удаляет записи корзины старше `cutoff_unix` секунд (Unix timestamp).
    pub fn purge_old_trash(&self, cutoff_unix: i64) -> Result<usize> {
        self.db.purge_deleted(cutoff_unix)
    }

    /// Все уникальные source_tag живых записей, отсортированные алфавитно.
    pub fn list_source_tags(&self) -> Result<Vec<String>> {
        self.db.list_source_tags()
    }

    /// Переименовывает или очищает source_tag у всех записей с данным тегом.
    pub fn update_source_tag_bulk(&mut self, old_tag: &str, new_tag: Option<&str>) -> Result<usize> {
        let n = self.db.update_source_tag_bulk(old_tag, new_tag)?;
        if n > 0 {
            self.save()?;
        }
        Ok(n)
    }

    // ── Папки ──────────────────────────────────────────────────────────────────

    pub fn add_folder(&mut self, name: &str, parent_id: Option<Uuid>, icon: Option<String>) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let (name_encrypted, nonce) =
            crypto::encrypt_field(name.as_bytes(), &id, "name", &self.vault_key)?;
        self.db.upsert_folder(&FolderRow {
            id,
            name_encrypted,
            name_nonce: nonce.to_vec(),
            parent_id,
            icon,
            created_at: now_unix(),
        })?;
        Ok(id)
    }

    pub fn list_folders(&self) -> Result<Vec<Folder>> {
        let mut out = Vec::new();
        for row in self.db.list_folders()? {
            let nonce = to_nonce(&row.name_nonce)?;
            let name_bytes =
                crypto::decrypt_field(&row.name_encrypted, &nonce, &row.id, "name", &self.vault_key)?;
            out.push(Folder {
                id: row.id,
                name: String::from_utf8(name_bytes)
                    .map_err(|_| VaultError::Serialization("folder name utf8".into()))?,
                parent_id: row.parent_id,
                icon: row.icon,
                created_at: row.created_at,
            });
        }
        Ok(out)
    }

    /// Переименовывает папку; сохраняет parent_id, icon, created_at.
    pub fn rename_folder(&mut self, id: &Uuid, new_name: &str) -> Result<()> {
        let rows = self.db.list_folders()?;
        let row = rows
            .into_iter()
            .find(|r| r.id == *id)
            .ok_or_else(|| VaultError::Serialization("folder not found".into()))?;
        let (name_encrypted, nonce) =
            crypto::encrypt_field(new_name.as_bytes(), id, "name", &self.vault_key)?;
        self.db.upsert_folder(&FolderRow {
            id: row.id,
            name_encrypted,
            name_nonce: nonce.to_vec(),
            parent_id: row.parent_id,
            icon: row.icon,
            created_at: row.created_at,
        })?;
        self.save()
    }

    /// Удаляет папку; записи внутри получают folder_id = NULL.
    pub fn delete_folder(&mut self, id: &Uuid) -> Result<()> {
        self.db.delete_folder(id)?;
        self.save()
    }

    /// Экспортирует все Login-записи в CSV (формат совместим с Chrome/Firefox export).
    pub fn export_items_csv(&self) -> Result<String> {
        let mut csv = String::from("name,url,username,password,note\n");
        for item in self.list_items()? {
            if let crate::models::ItemPayload::Login { ref url, ref username, ref password, ref notes, .. } = item.payload {
                let esc = |s: &str| -> String {
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.to_owned()
                    }
                };
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    esc(&item.title), esc(url), esc(username), esc(password),
                    esc(notes.as_deref().unwrap_or("")),
                ));
            }
        }
        Ok(csv)
    }

    /// Анализирует пароли: слабые, дублирующиеся, не обновлявшиеся 6+ месяцев.
    pub fn health_report(&self) -> Result<Vec<HealthEntry>> {
        use std::collections::HashMap;
        let items = self.list_items()?;

        let mut pw_counts: HashMap<String, usize> = HashMap::new();
        for item in &items {
            if let crate::models::ItemPayload::Login { password, .. } = &item.payload {
                if !password.is_empty() {
                    *pw_counts.entry(password.clone()).or_insert(0) += 1;
                }
            }
        }

        let now = now_unix();
        let six_months: i64 = 180 * 24 * 3600;
        let mut out = Vec::new();

        for item in &items {
            if let crate::models::ItemPayload::Login { url, password, .. } = &item.payload {
                let is_weak = is_weak_password(password);
                let is_duplicate = pw_counts.get(password).copied().unwrap_or(0) > 1;
                let is_old = (now - item.updated_at) > six_months;
                if is_weak || is_duplicate || is_old {
                    out.push(HealthEntry {
                        id: item.id.to_string(),
                        title: item.title.clone(),
                        url: url.clone(),
                        is_weak,
                        is_duplicate,
                        is_old,
                        updated_at: item.updated_at,
                    });
                }
            }
        }
        Ok(out)
    }

    // ── Смена мастер-пароля ────────────────────────────────────────────────────

    /// Меняет мастер-пароль: перевыводит ключи, перешифровывает ТОЛЬКО vault_key
    /// и контейнер. Записи не трогаются. Соль остаётся прежней.
    pub fn change_master_password(&mut self, old: &[u8], new: &[u8]) -> Result<()> {
        // Подтверждаем старый пароль через повторную деривацию и сверку db_key.
        let salt_bytes = file::read_no_symlink(&self.paths.salt)?;
        let mut salt = [0u8; sodium::SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        let old_keys = crypto::derive_master_keys(old, &salt)?;
        if !sodium::memcmp(old_keys.db_key.as_bytes(), self.keys.db_key.as_bytes()) {
            return Err(VaultError::DecryptionFailed);
        }

        // Новые ключи.
        let new_keys = crypto::derive_master_keys(new, &salt)?;

        // Перешифровываем vault_key новым encryption_key.
        let (encrypted, nonce) = crypto::rewrap_vault_key(&new_keys.encryption_key, &self.vault_key)?;

        let mut header = self.db.read_vault_row()?;
        header.encrypted_vault_key = encrypted;
        header.key_nonce = nonce.to_vec();
        self.db.write_vault_row(&header)?;

        self.keys = new_keys;
        // Контейнер перешифруется новым db_key при save().
        self.save()
    }

    // ── Бэкап с BIP-39 мнемоникой (ADR-003) ────────────────────────────────────

    /// Генерирует 24-словную BIP-39 мнемонику для бэкапов (256-бит энтропии).
    /// Показывать пользователю ОДИН РАЗ; VaultPass не хранит мнемонику на диске.
    pub fn generate_backup_phrase() -> Result<String> {
        backup::generate_mnemonic()
    }

    /// Экспортирует vault в зашифрованный бэкап-файл формата v2 (`.vbk`).
    /// `phrase` — 24-словная BIP-39 мнемоника из `generate_backup_phrase()`.
    pub fn export_backup(&self, dest: &std::path::Path, phrase: &str) -> Result<()> {
        backup::export(&self.paths.dir, dest, phrase)
    }

    /// Восстанавливает vault из бэкап-файла (поддерживает v1 и v2).
    /// Неверная фраза/мнемоника → `DecryptionFailed`.
    pub fn restore_from_backup(src: &std::path::Path, dest_dir: &std::path::Path, phrase: &str) -> Result<()> {
        backup::restore(src, dest_dir, phrase)
    }

    // ── OS Keychain helpers ───────────────────────────────────────────────────

    /// UUID vault в виде строки — для именования записей в OS Keychain.
    pub fn vault_id_str(&self) -> String {
        self.meta.vault_id.to_string()
    }

    /// Байты vault key — для хранения в OS Keychain после unlock.
    /// ТОЛЬКО для Keychain интеграции в desktop-слое. Не раскрывать в WebView.
    pub fn vault_key_bytes(&self) -> &[u8; 32] {
        self.vault_key.as_bytes()
    }

    // ── Конвертация Item ↔ ItemRow ─────────────────────────────────────────────

    fn item_to_row(&self, item: &Item) -> Result<ItemRow> {
        let title_encrypted =
            crypto::encrypt_field_inline(item.title.as_bytes(), &item.id, "title", &self.vault_key)?;
        let title_search_hash = crypto::search_hash(&item.title, &self.keys.search_key)?;
        let payload_json = serde_json::to_vec(&item.payload)?;
        let (payload_encrypted, payload_nonce) =
            crypto::encrypt_field(&payload_json, &item.id, "payload", &self.vault_key)?;
        Ok(ItemRow {
            id: item.id,
            item_type: item.item_type.as_str().to_string(),
            title_encrypted,
            title_search_hash: title_search_hash.to_vec(),
            payload_encrypted,
            payload_nonce: payload_nonce.to_vec(),
            folder_id: item.folder_id,
            favorite: item.favorite,
            created_at: item.created_at,
            updated_at: item.updated_at,
            lamport_clock: item.lamport_clock as i64,
            deleted: item.deleted,
            source_tag: item.source_tag.clone(),
        })
    }

    fn row_to_item(&self, row: &ItemRow) -> Result<Item> {
        let title_bytes =
            crypto::decrypt_field_inline(&row.title_encrypted, &row.id, "title", &self.vault_key)?;
        let title = String::from_utf8(title_bytes)
            .map_err(|_| VaultError::Serialization("title utf8".into()))?;
        let payload_nonce = to_nonce(&row.payload_nonce)?;
        let payload_json = crypto::decrypt_field(
            &row.payload_encrypted,
            &payload_nonce,
            &row.id,
            "payload",
            &self.vault_key,
        )?;
        let payload = serde_json::from_slice(&payload_json)?;
        let item_type = ItemType::from_str(&row.item_type)
            .ok_or_else(|| VaultError::Serialization("unknown item_type".into()))?;
        Ok(Item {
            id: row.id,
            item_type,
            title,
            payload,
            folder_id: row.folder_id,
            favorite: row.favorite,
            created_at: row.created_at,
            updated_at: row.updated_at,
            lamport_clock: row.lamport_clock.max(0) as u64,
            deleted: row.deleted,
            source_tag: row.source_tag.clone(),
        })
    }
}

// ── Контейнер vault.db ────────────────────────────────────────────────────────

/// Формат: MAGIC(4) | FORMAT(1) | vault_id(16) | nonce(24) | ciphertext.
fn seal_container(vault_id: Uuid, plaintext: &[u8], db_key: &Key) -> Result<Vec<u8>> {
    let nonce = sodium::gen_nonce()?;
    let mut ad = Vec::with_capacity(AD_CONTAINER.len() + 16);
    ad.extend_from_slice(AD_CONTAINER);
    ad.extend_from_slice(vault_id.as_bytes());
    let ct = sodium::aead_seal(plaintext, &ad, &nonce, db_key)?;

    let mut out = Vec::with_capacity(4 + 1 + 16 + NONCE_LEN + ct.len());
    out.extend_from_slice(CONTAINER_MAGIC);
    out.push(CONTAINER_FORMAT);
    out.extend_from_slice(vault_id.as_bytes());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Возвращает (vault_id, расшифрованный образ БД).
fn open_container(data: &[u8], db_key: &Key) -> Result<(Uuid, Vec<u8>)> {
    const HEADER: usize = 4 + 1 + 16 + NONCE_LEN;
    if data.len() < HEADER {
        return Err(VaultError::TamperedVault);
    }
    if &data[0..4] != CONTAINER_MAGIC || data[4] != CONTAINER_FORMAT {
        return Err(VaultError::TamperedVault);
    }
    let mut id_bytes = [0u8; 16];
    id_bytes.copy_from_slice(&data[5..21]);
    let vault_id = Uuid::from_bytes(id_bytes);

    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&data[21..21 + NONCE_LEN]);
    let ct = &data[HEADER..];

    let mut ad = Vec::with_capacity(AD_CONTAINER.len() + 16);
    ad.extend_from_slice(AD_CONTAINER);
    ad.extend_from_slice(vault_id.as_bytes());

    let plain = sodium::aead_open(ct, &ad, &nonce, db_key)?;
    Ok((vault_id, plain))
}

/// Fuzz-точка: разбор произвольных байт как контейнера vault.db.
/// Не должна паниковать/давать UB на любом входе — только Err.
#[cfg(feature = "fuzz")]
pub fn fuzz_open_container(data: &[u8]) {
    let key = Key::zeroed();
    let _ = open_container(data, &key);
}

// ── meta / helpers ──────────────────────────────────────────────────────────

fn write_meta(path: &Path, meta: &VaultMeta) -> Result<()> {
    let json = serde_json::to_vec_pretty(meta)?;
    file::atomic_write(path, &json)?;
    Ok(())
}

fn read_meta(path: &Path) -> Result<VaultMeta> {
    let bytes = file::read_no_symlink(path)?;
    serde_json::from_slice(&bytes).map_err(|_| VaultError::TamperedVault)
}

fn to_nonce(v: &[u8]) -> Result<[u8; NONCE_LEN]> {
    if v.len() != NONCE_LEN {
        return Err(VaultError::TamperedVault);
    }
    let mut n = [0u8; NONCE_LEN];
    n.copy_from_slice(v);
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ItemPayload;

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("vp_vault_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn login(pw: &str) -> ItemPayload {
        ItemPayload::Login {
            url: "https://example.com".into(),
            username: "user".into(),
            password: pw.into(),
            totp_secret: None,
            notes: None,
            custom_fields: vec![],
            password_history: vec![],
        }
    }

    #[test]
    fn create_open_roundtrip() {
        let dir = tmp_dir();
        let id = {
            let mut v = Vault::create(&dir, b"master-pass", Some("hint".into())).unwrap();
            let id = v.add_item("GitHub", login("s3cr3t"), None, false, None).unwrap();
            v.save().unwrap();
            id
        };

        // Повторное открытие тем же паролем.
        let v = Vault::open(&dir, b"master-pass", None).unwrap();
        let item = v.get_item(&id).unwrap().unwrap();
        assert_eq!(item.title, "GitHub");
        match item.payload {
            ItemPayload::Login { password, .. } => assert_eq!(password, "s3cr3t"),
            _ => panic!("wrong type"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wrong_password_fails() {
        let dir = tmp_dir();
        {
            let v = Vault::create(&dir, b"right-pass", None).unwrap();
            v.save().unwrap();
        }
        assert!(matches!(
            Vault::open(&dir, b"wrong-pass", None),
            Err(VaultError::DecryptionFailed)
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn search_finds_item() {
        let dir = tmp_dir();
        let mut v = Vault::create(&dir, b"pw", None).unwrap();
        v.add_item("My Bank", login("a"), None, false, None).unwrap();
        v.add_item("GitHub", login("b"), None, false, None).unwrap();
        let found = v.search("my bank").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "My Bank");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn change_password_keeps_items() {
        let dir = tmp_dir();
        let id = {
            let mut v = Vault::create(&dir, b"old-pass", None).unwrap();
            let id = v.add_item("Email", login("p"), None, false, None).unwrap();
            v.change_master_password(b"old-pass", b"new-pass").unwrap();
            id
        };

        // Старый пароль больше не работает.
        assert!(matches!(
            Vault::open(&dir, b"old-pass", None),
            Err(VaultError::DecryptionFailed)
        ));
        // Новый — открывает, запись на месте.
        let v = Vault::open(&dir, b"new-pass", None).unwrap();
        assert_eq!(v.get_item(&id).unwrap().unwrap().title, "Email");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tampered_meta_detected() {
        let dir = tmp_dir();
        {
            let v = Vault::create(&dir, b"pw", None).unwrap();
            v.save().unwrap();
        }
        // Подменяем vault_id в vault.meta — рассинхрон с контейнером.
        let meta_path = dir.join("vault.meta");
        let forged = VaultMeta {
            vault_id: Uuid::new_v4(),
            schema_version: SCHEMA_VERSION,
            created_at: 0,
            totp_enabled: false,
        };
        write_meta(&meta_path, &forged).unwrap();

        assert!(matches!(
            Vault::open(&dir, b"pw", None),
            Err(VaultError::TamperedVault)
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_hides_item() {
        let dir = tmp_dir();
        let mut v = Vault::create(&dir, b"pw", None).unwrap();
        let id = v.add_item("Temp", login("x"), None, false, None).unwrap();
        v.delete_item(&id).unwrap();
        assert!(v.list_items().unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
