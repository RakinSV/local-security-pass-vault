# Правила системы бэкапов VaultPass

## Концепция

Бэкап-файл должен быть защищён независимо от vault.db.
Даже если файл бэкапа утечёт — брутфорс невозможен при правильных
параметрах KDF.

## Ключ бэкапа: BIP-39 мнемоника

Пользователь при первом создании vault генерирует мнемонику из 24 слов
(BIP-39, английский wordlist). Это и есть ключ для расшифровки бэкапов.

Crate: bip39 = "2" (feature = "english")

### Генерация при создании vault
```rust
use bip39::{Mnemonic, Language};

// Генерация 24 слов (256 бит энтропии)
let mnemonic = Mnemonic::generate_in(Language::English, 24)?;
let phrase = mnemonic.to_string();  // показать пользователю ОДИН РАЗ

// Derive backup key из мнемоники
let seed = mnemonic.to_seed("");  // seed = 64 байта
let backup_key_material = &seed[..32];  // первые 32 байта
```

### Правила
- Мнемоника показывается пользователю ОДИН РАЗ при создании vault
- VaultPass НИКОГДА не сохраняет мнемонику на диск
- Пользователь записывает на бумагу / хранит в безопасном месте
- Для расшифровки бэкапа нужна мнемоника + файл бэкапа

## KDF для бэкапа (усиленный профиль)

Бэкап использует ОТДЕЛЬНЫЙ Argon2id профиль — значительно сильнее
чем для ежедневной разблокировки:

```rust
// Профиль для бэкапа — намеренно медленный
let backup_kdf_params = Params::new(
    4 * 1024 * 1024,  // m = 4GB RAM (4_194_304 KB)
    10,               // t = 10 итераций
    4,                // p = 4 параллельных потока
    Some(32),         // output = 32 байта
)?;

// Salt для бэкапа — новый random при каждом создании бэкапа
let backup_salt = randombytes_buf(32);

// Финальный ключ шифрования бэкапа
let backup_enc_key = argon2id(
    backup_key_material,  // из BIP-39 seed
    &backup_salt,
    &backup_kdf_params,
)?;
```

Почему 4GB: RTX 4090 имеет 24GB VRAM. При 4GB на попытку —
максимум 6 параллельных попыток на одну карту. При 10 итерациях
одна попытка занимает ~30-60 секунд. 1000 GPU = ~6000 попыток/минуту.
Словарь BIP-39 = 2048 слов, 24 слова = 2048^24 комбинаций.
Брутфорс физически невозможен.

## Формат бэкап-файла (.vbk)

Использовать формат age (rust crate: age = "0.10"):

```
[бэкап-файл .vbk]
├── age header (recipient: X25519 из backup_enc_key)
├── encrypted payload:
│   ├── magic: b"VAULTPASS_BACKUP_V1"
│   ├── vault_uuid: [u8; 16]
│   ├── created_at: i64 (unix timestamp)
│   ├── schema_version: u32
│   ├── blake3_checksum: [u8; 32]  ← checksum незашифрованного vault.db
│   └── vault_db_bytes: Vec<u8>    ← полный дамп vault.db
```

Crates:
- age = "0.10"
- blake3 = "1"

## Верификация целостности

При создании бэкапа:
```rust
let db_bytes = std::fs::read("vault.db")?;
let checksum = blake3::hash(&db_bytes);
// checksum включается в encrypted payload
```

При восстановлении:
```rust
// После расшифровки проверить checksum
let computed = blake3::hash(&payload.vault_db_bytes);
assert_eq!(computed.as_bytes(), &payload.blake3_checksum,
    "Backup integrity check failed — файл повреждён или подменён");
```

## Стратегия хранения (3-2-1)

VaultPass рекомендует пользователю (UI подсказки, не принуждение):
- 3 копии: локально + USB флешка + второй компьютер/NAS
- 2 разных носителя (диск и флешка — разные физические типы)
- 1 копия офлайн (флешка в ящике стола)

## Ротация бэкапов

Автоматическое создание бэкапа:
- При каждом изменении vault (добавление/редактирование/удаление записи)
- По расписанию: еженедельно (конфигурируется)

Хранить локально:
- 7 ежедневных бэкапов
- 4 еженедельных бэкапа
- Старые удалять автоматически (safe delete — перезапись нулями перед удалением)

Путь хранения: ~/.vaultpass/backups/vaultpass_{date}_{uuid8}.vbk

## UX при создании vault

1. Генерировать мнемонику
2. Показать пользователю 24 слова на экране
3. Попросить подтверждения: "Я записал мнемонику в безопасное место"
4. Опционально: тест — попросить ввести 3 случайных слова из списка
5. Только после подтверждения — завершить создание vault

## Команды Tauri IPC для бэкапов

```rust
#[tauri::command]
async fn create_backup(
    state: State<'_, VaultState>,
    export_path: PathBuf,
) -> Result<BackupMeta, VaultError>;

#[tauri::command]
async fn restore_from_backup(
    mnemonic_phrase: String,  // 24 слова через пробел
    backup_path: PathBuf,
    new_master_password: String,
) -> Result<(), VaultError>;

#[tauri::command]
async fn verify_backup(
    backup_path: PathBuf,
) -> Result<BackupMeta, VaultError>;  // проверить без расшифровки
```
