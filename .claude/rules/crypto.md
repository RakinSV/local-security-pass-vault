# Правила криптографии — НАРУШАТЬ НЕЛЬЗЯ

> Эти правила применяются ко всему коду в `core-vault/src/crypto/`.
> Любое отклонение требует явного комментария с обоснованием и ревью.

## Абсолютные запреты

```
НИКОГДА не использовать:
  - rand::thread_rng()          → использовать randombytes_buf() из libsodium
  - Math.random() в JS/TS       → использовать crypto.getRandomValues()
  - std::time для seed           → предсказуемый PRNG, уязвим к брутфорсу
  - memset() для обнуления ключей → компилятор оптимизирует, использовать sodium_memzero()
  - == для сравнения MAC/хешей  → timing attack, использовать sodium_memcmp()
  - AES-128                     → минимум AES-256, лучше XChaCha20
  - ECB режим                   → всегда AEAD (XChaCha20-Poly1305)
  - MD5, SHA-1 для безопасности → SHA-256 минимум, для MAC — HMAC-SHA256
  - нативный CBC без MAC        → padding oracle, только AEAD
```

## Обязательные алгоритмы

### KDF (деривация ключа из мастер-пароля)
```rust
// ПРАВИЛЬНО:
use sodiumoxide::crypto::pwhash::argon2id13;

let salt = argon2id13::Salt::gen();  // randombytes_buf внутри
let key = argon2id13::derive_key(
    &mut master_key_bytes,
    password.as_bytes(),
    &salt,
    argon2id13::OPSLIMIT_SENSITIVE,  // t=4 итерации
    argon2id13::MEMLIMIT_SENSITIVE,  // m=256MB
).expect("KDF failed");

// ПАРАМЕТРЫ НЕ СНИЖАТЬ без изменения SECURITY_LEVEL в vault.meta
// Текущий минимум: m=256MB, t=4, p=4
// При изменении параметров — bump schema_version и добавить миграцию
```

### Envelope Encryption (два уровня ключей)
```
Мастер-пароль + Salt
        │
        ▼
   Argon2id KDF (256MB, 4 iter)
        │
        ├──► Encryption Key (32B) — расшифровывает Vault Key
        └──► Search Key (32B)     — HMAC для поискового индекса

Vault Key (32B, случайный при создании vault)
        │
        ▼
   XChaCha20-Poly1305 (через libsodium secretbox)
        │
        ▼
   Зашифрованные записи (каждая с уникальным nonce)

Смена мастер-пароля = перешифровать ТОЛЬКО encrypted_vault_key.
Записи не трогать — vault_key остаётся тем же.
```

### Шифрование записей
```rust
// ПРАВИЛЬНО — XChaCha20-Poly1305:
use sodiumoxide::crypto::secretbox;

// Новый nonce при КАЖДОМ сохранении (даже если данные не изменились)
let nonce = secretbox::gen_nonce();  // 192-bit, randombytes_buf внутри
let ciphertext = secretbox::seal(plaintext, &nonce, &vault_key);

// НЕПРАВИЛЬНО — счётчик как nonce:
// let nonce = Nonce::from_counter(self.counter); // ЗАПРЕЩЕНО
```

### Деривация с domain separation
```rust
// Три независимых ключа из одного KDF вывода (96 байт total):
// Байты 0..32  → db_key         (ключ SQLCipher — внешний слой шифрования файла)
// Байты 32..64 → encryption_key (для Vault Key)
// Байты 64..96 → search_key     (для HMAC поискового индекса)
//
// Это гарантирует математическую независимость ключей.
// Утечка одного ключа не даёт информации о других.
//
// ПРИМЕЧАНИЕ (ревизия 2026-06): изначально было 64 байта (2 ключа). Добавлен
// db_key, т.к. таблица vault с encrypted_vault_key лежит ВНУТРИ SQLCipher-БД —
// ключ для её открытия должен выводиться напрямую из мастер-пароля, а не из
// vault_key (проблема «курица-яйцо»). SQLCipher получает db_key как raw key
// (PRAGMA key с x'...'), минуя собственный PBKDF2.
```

### Поиск по зашифрованным данным
```rust
// Поисковый индекс: HMAC-SHA256(lowercase(title), search_key)
// Хранится в открытом виде в title_search_hash.
// Позволяет поиск БЕЗ расшифровки всех записей.
//
// При поиске: вычислить HMAC запроса, искать совпадение в индексе.
// Раскрывает: факт совпадения. НЕ раскрывает: содержимое заголовка.
use sodiumoxide::crypto::auth::hmacsha256;
let tag = hmacsha256::authenticate(title.to_lowercase().as_bytes(), &search_key);
```

## Управление ключами в памяти

```rust
// 1. Блокировка от свопа — сразу при получении ключа:
sodiumoxide::utils::mlock(&mut key_bytes).expect("mlock failed");

// 2. Обнуление — сразу после использования:
sodiumoxide::utils::memzero(&mut key_bytes);

// 3. Время жизни ключей в памяти:
//    - Мастер-пароль: только на время Argon2id вычисления
//    - Master Key: только на время расшифровки Vault Key  
//    - Vault Key: всё время пока vault разблокирован (в защищённой странице)
//    - Расшифрованный пароль записи: только пока поле видно пользователю

// 4. На Windows дополнительно:
//    VirtualLock() для страниц с Vault Key
//    CryptProtectMemory() для дополнительного шифрования в памяти
```

## Работа с файлами vault

```rust
// ВСЕГДА атомарная запись:
// 1. Записать в vault.db.tmp
// 2. fsync() — гарантия записи на диск
// 3. rename(tmp, vault.db) — атомарна по POSIX, MoveFileExW на Windows

// ВСЕГДА проверять symlink перед открытием:
// lstat() вместо stat() — если S_ISLNK → отказ с ошибкой VaultError::SymlinkDetected

// ВСЕГДА проверять UUID vault при открытии:
// vault.meta содержит vault_id, vault.db содержит тот же id в заголовке
// Несовпадение → VaultError::TamperedVault
```

## Запрет внешних зависимостей для крипто

```toml
# В core-vault/Cargo.toml — ТОЛЬКО эти крипто-зависимости:
sodiumoxide = { version = "0.2", features = ["use-pkg-config"] }
# ИЛИ (предпочтительно для статической линковки):
libsodium-sys = { version = "0.9", features = ["use-pkg-config"] }

# ЗАПРЕЩЕНО добавлять без ревью:
# openssl = ...       → используем libsodium
# ring = ...          → не содержит Argon2id
# argon2 = ...        → отдельная реализация, не аудирована как libsodium
# aes = ...           → используем XChaCha20
```

## Тест-векторы — обязательно

Каждая крипто-функция должна иметь тест с официальными тест-векторами:
- Argon2id: RFC 9106 Appendix B
- XChaCha20-Poly1305: libsodium test suite vectors
- HMAC-SHA256: RFC 4231

```rust
#[cfg(test)]
mod tests {
    // Тест 1: официальный тест-вектор
    // Тест 2: изменение 1 бита → другой результат
    // Тест 3: разные salt → разные ключи
    // Тест 4: после memzero — память обнулена (unsafe тест)
}
```
