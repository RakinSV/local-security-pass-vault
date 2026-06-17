# Стратегия тестирования безопасности

## Пирамида тестов

```
         [Pentest]         ← раз в версию (перед релизом)
       [Fuzzing CI]        ← еженедельно в CI
    [Integration Tests]    ← при каждом PR
  [Unit Tests + Vectors]   ← при каждом коммите (обязательно)
```

## Фаза 1 — Unit тесты (cargo test)

### Крипто-тесты (обязательные тест-векторы)
```rust
// core-vault/src/crypto/tests.rs

#[test]
fn argon2id_rfc9106_test_vector() {
    // Официальный вектор из RFC 9106 Appendix B
    let password = b"password";
    let salt = [0x02u8; 16];  // из вектора
    // ... сравнить с ожидаемым хешем из RFC
    // Если этот тест падает — библиотека некорректна
}

#[test]
fn xchacha20_different_nonce_every_save() {
    // Сохранить одну запись дважды — nonce должны отличаться
    let nonce1 = encrypt_item(&item, &vault_key).nonce;
    let nonce2 = encrypt_item(&item, &vault_key).nonce;
    assert_ne!(nonce1, nonce2, "nonce collision — critical bug");
}

#[test]
fn xchacha20_bit_flip_fails_decryption() {
    // 1 бит изменён в ciphertext → расшифровка должна вернуть ошибку
    let mut ciphertext = encrypt_item(&item, &vault_key).data;
    ciphertext[10] ^= 0x01;  // flip bit
    assert!(decrypt_item(&ciphertext, &vault_key).is_err());
}

#[test]
fn nonce_collision_test() {
    // Сгенерировать 1_000_000 nonce — нет повторений
    use std::collections::HashSet;
    let nonces: HashSet<[u8; 24]> = (0..1_000_000)
        .map(|_| gen_nonce())
        .collect();
    assert_eq!(nonces.len(), 1_000_000, "nonce collision detected");
}

#[test]
fn constant_time_comparison() {
    // Проверить что sodium_memcmp использован везде где нужно
    // (статический анализ через clippy lint или review)
    // Тест: сравнение двух одинаковых MAC должно возвращать true
    let mac1 = compute_mac(b"data", &key);
    let mac2 = compute_mac(b"data", &key);
    assert!(constant_time_eq(&mac1, &mac2));
}

#[test]
fn salt_uniqueness_per_vault() {
    // Создать 100 vault — все соли уникальны
    let salts: Vec<[u8; 32]> = (0..100).map(|_| gen_salt()).collect();
    let unique: std::collections::HashSet<_> = salts.iter().collect();
    assert_eq!(unique.len(), 100);
}

#[unsafe(test)]  // требует nightly или feature
fn memzero_actually_clears() {
    let mut key = [0xFFu8; 32];
    let key_ptr = key.as_ptr();
    sodium_memzero(&mut key);
    // Читаем память напрямую — должны быть нули
    unsafe {
        let slice = std::slice::from_raw_parts(key_ptr, 32);
        assert!(slice.iter().all(|&b| b == 0));
    }
}
```

### Тесты файловых операций
```rust
#[test]
fn atomic_write_survives_sigkill() {
    // Запустить запись в отдельном процессе, убить его в середине
    // Проверить что vault.db читается (старая версия, не corrupted)
    // Реализовать через std::process::Command + SIGKILL
}

#[test]
fn symlink_detection() {
    // Создать symlink вместо vault.db
    // Попытка открыть → VaultError::SymlinkDetected, не крэш
    let tmp = tempfile::tempdir().unwrap();
    let vault_path = tmp.path().join("vault.db");
    std::os::unix::fs::symlink("/etc/passwd", &vault_path).unwrap();
    assert!(matches!(
        safe_open_vault(&vault_path),
        Err(VaultError::SymlinkDetected(_))
    ));
}

#[test]
fn vault_uuid_verification() {
    // Скопировать vault.db от другого vault → должна быть ошибка TamperedVault
}
```

## Фаза 2 — Integration тесты

### Полный жизненный цикл
```rust
#[tokio::test]
async fn full_vault_lifecycle() {
    // 1. Создать vault с мастер-паролем "correct-horse-battery-staple"
    // 2. Добавить 1000 записей разных типов
    // 3. Сменить мастер-пароль на "new-secure-passphrase"
    // 4. Закрыть vault (выгрузить ключи из памяти)
    // 5. Открыть vault с новым паролем
    // 6. Проверить что все 1000 записей на месте и расшифрованы верно
    // 7. Открыть со старым паролем → VaultError::DecryptionFailed
}
```

### Тесты домена (расширение)
```typescript
// extension/tests/domain-matching.test.ts

const testCases = [
    // [vault_url, page_url, should_match]
    ["https://google.com",         "https://accounts.google.com",    true],
    ["https://google.com",         "https://google.com.evil.ru",     false],
    ["https://paypal.com",         "https://paypa1.com",             false],
    ["https://github.com",         "https://github.io",              false],
    ["https://amazon.co.uk",       "https://www.amazon.co.uk",       true],
    ["https://bank.com/login",     "https://bank.com/dashboard",     true],
    ["https://evil.com",           "https://notevil.com",            false],
];

test.each(testCases)('domain match: %s vs %s → %s', (vault, page, expected) => {
    expect(domainsMatch(vault, page)).toBe(expected);
});
```

### Тест буфера обмена
```rust
#[test]
#[ignore = "requires display"]  // запускать только локально, не в CI без display
fn clipboard_cleared_after_ttl() {
    copy_to_clipboard("super-secret-password");
    std::thread::sleep(Duration::from_secs(31));
    assert_eq!(read_clipboard(), "", "clipboard not cleared after TTL");
}
```

## Фаза 3 — Fuzzing (cargo fuzz)

```bash
# Установка:
cargo install cargo-fuzz

# Запуск фаззеров:
cargo fuzz run fuzz_vault_open         # открытие повреждённого vault
cargo fuzz run fuzz_item_deserialize   # десериализация произвольных данных
cargo fuzz run fuzz_domain_match       # парсинг произвольных URL
cargo fuzz run fuzz_crypto_decrypt     # расшифровка произвольного ciphertext
```

```rust
// fuzz/fuzz_targets/fuzz_vault_open.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Произвольный мусор вместо vault.db
    // Ожидаем: VaultError (любой), НЕ panic, НЕ undefined behavior
    let result = core_vault::open_vault_bytes(data, "any_password");
    // Не должно быть panic — проверяем только что нет UB
    let _ = result;
});
```

## Фаза 4 — Pentest (перед релизом)

### Memory forensics
```bash
# 1. Запустить vaultpass, разблокировать vault
# 2. Дамп памяти процесса:
sudo gcore $(pgrep vaultpass)
# или на Windows: ProcDump.exe -ma vaultpass.exe dump.dmp

# 3. Поиск ключей и паролей в дампе:
strings dump.* | grep -i "password\|secret\|key\|vault"
# Цель: пароли записей не найдены в открытом виде
# Допустимо: зашифрованные блобы (нечитаемые строки)
```

### Process injection test
```bash
# Linux — попытка ptrace:
sudo gdb -p $(pgrep vaultpass)
# Ожидается: "ptrace: Operation not permitted" (PR_SET_DUMPABLE=0)

# Windows — попытка ReadProcessMemory:
# Использовать Process Hacker или написать тест-процесс
# Ожидается: ACCESS DENIED
```

### Network — ноль сетевых запросов
```bash
# Запустить wireshark/tcpdump во время:
# 1. Разблокировки vault
# 2. Копирования пароля
# 3. Автозаполнения в браузере
# Ожидается: никакого DNS трафика от vaultpass (кроме разрешения доменов пользователя)
sudo tcpdump -i any -n host not 8.8.8.8 and proc vaultpass
```

### DLL hijacking test (Windows)
```cmd
REM Положить fake libsodium.dll рядом с vaultpass.exe
REM В fake DLL: логировать вызовы и возвращать фиктивные данные
copy evil_libsodium.dll .\libsodium.dll
.\vaultpass.exe
REM Ожидается: приложение работает с вшитой libsodium (статическая линковка)
REM evil_libsodium.dll не загружается
```

## CI Pipeline

```yaml
# .github/workflows/security.yml
name: Security

on: [push, pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-audit
      - run: cargo audit                    # CVE проверка

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --all-features      # включая крипто-векторы

  fuzz:
    runs-on: ubuntu-latest
    if: github.event_name == 'schedule'    # еженедельно
    steps:
      - uses: actions/checkout@v4
      - run: cargo +nightly fuzz run fuzz_vault_open -- -max_total_time=300
      - run: cargo +nightly fuzz run fuzz_item_deserialize -- -max_total_time=300

  clippy-security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo clippy -- -D warnings
        # Кастомные clippy lints для поиска:
        # - unwrap() в не-тестовом коде
        # - == сравнение типов [u8] (должно быть memcmp)
        # - TODO/FIXME в крипто-файлах
```

## Что проверять при code review

```
КРИПТО:
☐ Новый nonce при каждом encrypt (не переиспользуется)
☐ sodium_memzero() после использования ключей
☐ sodium_memcmp() для сравнения MAC/хешей
☐ randombytes_buf() для генерации случайных данных
☐ Нет прямого использования rand:: для крипто

ФАЙЛЫ:
☐ Атомарная запись (tmp → fsync → rename)
☐ lstat() + O_NOFOLLOW при открытии vault
☐ Проверка vault UUID после открытия

ОШИБКИ:
☐ Единственная ошибка расшифровки: DecryptionFailed
☐ Нет логирования ключей/паролей
☐ Нет unwrap() вне тестов

РАСШИРЕНИЕ:
☐ Нет сетевых запросов
☐ Нет кеширования паролей
☐ eTLD+1 сравнение доменов
☐ Только видимые поля заполняются
```
