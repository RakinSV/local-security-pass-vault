# Правила безопасности — OS-уровень и защита от атак

## Защита процесса

### Linux
```rust
// В main() ДО любых других операций:
unsafe {
    // Запрет core dump и /proc/PID/mem для других процессов
    libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    
    // Запрет ptrace (gdb, strace не смогут attach)
    libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
}

// seccomp-bpf whitelist (через seccompiler crate):
// Разрешить только: read, write, open, close, mmap, mprotect,
//                   mlock, munlock, futex, exit_group, brk
// Запретить: ptrace, process_vm_readv, fork, exec (кроме явно нужных)
```

### Windows
```rust
// SetProcessMitigationPolicy при старте:
use windows::Win32::System::Threading::*;

// Запрет динамического кода (шеллкод инъекция)
SetProcessMitigationPolicy(
    ProcessDynamicCodePolicy,
    &PROCESS_MITIGATION_DYNAMIC_CODE_POLICY { Flags: 1 }, // ProhibitDynamicCode
    size_of::<PROCESS_MITIGATION_DYNAMIC_CODE_POLICY>()
);

// Запрет загрузки DLL из текущей директории
SetDllDirectoryW(None);

// Только подписанные DLL (блокирует DLL hijacking)
SetProcessMitigationPolicy(
    ProcessSignaturePolicy,
    &PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY { Flags: 1 },
    size_of::<PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY>()
);
```

## Защита файлов

### Атомарная запись (ОБЯЗАТЕЛЬНО для всех записей в vault.db)
```rust
pub fn atomic_write(target: &Path, data: &[u8]) -> Result<(), VaultError> {
    let tmp = target.with_extension("tmp");
    
    // 1. Записать во временный файл
    let mut file = File::create(&tmp)?;
    file.write_all(data)?;
    
    // 2. fsync — гарантия записи на физический диск
    file.sync_all()?;
    drop(file);
    
    // 3. Атомарный rename
    // Linux: rename() атомарен по POSIX
    // Windows: MoveFileExW с MOVEFILE_REPLACE_EXISTING
    std::fs::rename(&tmp, target)?;
    
    Ok(())
}
```

### Проверка symlink (ОБЯЗАТЕЛЬНО перед открытием vault.db)
```rust
pub fn safe_open_vault(path: &Path) -> Result<File, VaultError> {
    // lstat не следует по symlink
    let metadata = path.symlink_metadata()?;
    
    if metadata.file_type().is_symlink() {
        return Err(VaultError::SymlinkDetected(path.to_owned()));
    }
    
    // Открываем через O_NOFOLLOW флаг
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(VaultError::from)
    }
    
    #[cfg(windows)]
    {
        // FILE_FLAG_OPEN_REPARSE_POINT + проверка атрибутов
        // ...
    }
}
```

### Права доступа к файлам vault
```bash
# При создании vault:
chmod 700 ~/.vaultpass/          # директория: только владелец
chmod 600 ~/.vaultpass/vault.db  # файл: только владелец, только чтение/запись
chmod 400 ~/.vaultpass/vault.salt # соль: только чтение (создаётся один раз!)

# vault.db помечается READ-ONLY когда приложение закрыто:
# Linux: chmod 444 vault.db при exit, chmod 600 при открытии
# Windows: SetFileAttributes(FILE_ATTRIBUTE_READONLY) при exit
```

## Honeypot файл (обнаружение ransomware)

```rust
// При старте приложения:
// 1. Создать/проверить honeypot файл рядом с vault.db
//    ~/.vaultpass/vault_backup.db — содержит рандомные байты
// 2. Хранить SHA-256 хеш honeypot в памяти
// 3. При каждом unlock vault — перепроверить хеш
//    Если изменился → немедленно заблокировать vault и предупредить

pub struct HoneypotGuard {
    path: PathBuf,
    expected_hash: [u8; 32],
}

impl HoneypotGuard {
    pub fn check(&self) -> Result<(), VaultError> {
        let current_hash = sha256_file(&self.path)?;
        if !constant_time_eq(&current_hash, &self.expected_hash) {
            return Err(VaultError::PossibleRansomwareDetected);
        }
        Ok(())
    }
}
```

## Буфер обмена

```rust
// При копировании пароля:
// 1. Использовать CLIPBOARD_FORMAT_EXCLUDE_FROM_SYNC (Win10+)
//    → не синхронизируется в Microsoft/iCloud облако
// 2. Запустить таймер 30 секунд
// 3. По таймеру — очистить clipboard
// 4. При закрытии/блокировке — очистить немедленно

const CLIPBOARD_TTL_SECS: u64 = 30;

// Пометка "не синхронизировать в облако" (Windows):
// CF_EXCLUDEFROMCLOUDCLIPBOARD = RegisterClipboardFormat("CanIncludeInClipboardHistory")
// установить значение 0 → исключить из истории и синхронизации
```

## Защита экрана

```rust
// Windows — исключить окно из скриншотов, OBS, thumbnail кеша:
// При показе пароля (кнопка "показать"):
SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
// При скрытии пароля:
SetWindowDisplayAffinity(hwnd, WDA_NONE);

// При сворачивании окна (WM_SIZE с SIZE_MINIMIZED):
// → скрыть все видимые пароли (переключить в ••••)
// → применить WDA_EXCLUDEFROMCAPTURE к thumbnail

// Linux Wayland:
// zwp_linux_dmabuf_feedback_v1 — пометить surface как protected
// X11: нет надёжного решения, только auto-hide при потере фокуса
```

## Auto-lock

```rust
// Vault автоматически блокируется при:
// - N минут без активности (настраивается пользователем, default: 5 мин)
// - Сворачивании окна (опционально, по настройке)
// - Блокировке экрана ОС (WM_WTSSESSION_CHANGE на Windows)
// - Выходе из системы/сна

// При блокировке:
// 1. sodium_memzero(vault_key)     — обнулить ключ
// 2. sodium_munlock(vault_key)     — разблокировать страницу
// 3. EmptyClipboard()              — очистить буфер обмена
// 4. Скрыть все открытые пароли   — переключить в ••••
```

## DLL/SO hijacking (статическая линковка)

```toml
# core-vault/Cargo.toml
# libsodium линкуется СТАТИЧЕСКИ — нечего подменять

[features]
default = ["static"]
static = ["libsodium-sys/use-pkg-config"]

# Для Windows релизов: cargo build --target x86_64-pc-windows-msvc
# libsodium.lib включается в бинарь — нет внешних DLL зависимостей
```

## IPC аутентификация (расширение ↔ десктоп)

```
При первом сопряжении расширения с десктопом:
1. Десктоп генерирует Ed25519 ключевую пару
2. Публичный ключ передаётся расширению через QR-код (или ручной ввод)
3. Каждое IPC сообщение подписывается приватным ключом десктопа
4. Расширение верифицирует подпись перед обработкой ответа
5. Каждый запрос расширения содержит одноразовый токен (nonce)
   → защита от replay атак

Схема сообщения:
{
  "id": "uuid-nonce",
  "payload": { зашифрованный запрос },
  "signature": "ed25519-sig(id + payload, desktop_private_key)"
}
```

## Обработка ошибок (важно для безопасности)

```rust
// ПРАВИЛЬНО — единственная ошибка расшифровки:
Err(VaultError::DecryptionFailed)
// Не раскрывает: неверный пароль vs неверный MAC vs повреждённые данные

// ЗАПРЕЩЕНО — разные ошибки для разных причин отказа:
// Err(VaultError::WrongPassword)    // → oracle attack
// Err(VaultError::InvalidPadding)   // → padding oracle
// Err(VaultError::MacMismatch)      // → timing/oracle attack

// Время ответа на неверный пароль должно быть константным
// (Argon2id сам по себе медленный — это обеспечивается автоматически)
```

## Логирование — что НЕЛЬЗЯ логировать

```rust
// ЗАПРЕЩЕНО в любых логах (даже debug/trace):
// - Мастер-пароль или его хеш
// - Vault Key или Master Key (любые байты ключей)
// - Расшифрованные пароли из записей
// - Содержимое clipboard после копирования пароля
// - Полные URL с параметрами (могут содержать токены)

// МОЖНО логировать:
// - Операции без содержимого: "vault opened", "item created (id=uuid)"
// - Ошибки без деталей: "decryption failed" (без причины)
// - Метрики производительности: "argon2id: 847ms"
// - UUID записей (не их содержимое)

// Уровень логирования в продакшне: INFO (не DEBUG, не TRACE)
```

## OS Keychain / Secure Enclave

Vault Key после разблокировки ДОЛЖЕН храниться в OS Keychain,
а не только в памяти процесса. Реализация по платформам:

### Linux
Использовать libsecret (org.freedesktop.secrets).
Crate: secret-service = "3"
Атрибуты записи:
  service = "vaultpass"
  account = vault UUID (из vault.meta)
  label   = "VaultPass — Vault Key"

### Windows
Использовать Windows Credential Manager (DPAPI).
Crate: keyring = "2"
Target name: "VaultPass/VaultKey/{vault_uuid}"

### macOS
Использовать Security.framework Keychain.
Crate: keyring = "2" (единый API для macOS/Win/Linux)
Service: "vaultpass", Account: vault UUID

### Правила
- Vault Key записывается в Keychain ТОЛЬКО после успешного
  ввода мастер-пароля (unlock)
- При lock() — удалять запись из Keychain (не просто очищать память)
- При следующем unlock() — предлагать биометрию ОС вместо мастер-пароля,
  если Vault Key есть в Keychain (quick unlock flow)
- Если Keychain недоступен (headless, CI) — fallback на in-memory
- НИКОГДА не записывать мастер-пароль в Keychain — только производный ключ

## Авто-блокировка (Auto-lock)

Таймер запускается при каждом событии разблокировки.
Сбрасывается при любом взаимодействии пользователя с vault.

### Конфигурация (хранить в vault.meta, шифровать вместе с vault)
- auto_lock_timeout_secs: u32  — дефолт 300 (5 минут), 0 = выключено
- lock_on_minimize: bool        — дефолт true
- lock_on_screensaver: bool     — дефолт true

### Реализация в Tauri 2
```rust
// В Tauri app state
struct VaultState {
    vault_key: Mutex<Option<ZeroizeVec>>,  // zeroize crate
    lock_timer: Mutex<Option<JoinHandle<()>>>,
}

// При авто-блокировке
fn lock_vault(state: &VaultState) {
    let mut key = state.vault_key.lock().unwrap();
    if let Some(k) = key.take() {
        drop(k);  // ZeroizeVec реализует Drop -> zeroize
    }
    // Удалить из OS Keychain тоже
    keychain::delete_vault_key();
}
```

### События Tauri для триггера блокировки
- tauri::WindowEvent::Focused(false) + lock_on_minimize
- tauri::api::idle::idle_time() > auto_lock_timeout
- Системное событие screensaver/sleep через tauri-plugin-os

### Crates
- zeroize = "1" с feature = ["derive"] — для ZeroizeVec и #[derive(Zeroize)]
- tauri-plugin-os = "2" — для системных событий

### Правила
- После lock(): state.vault_key = None, OS Keychain запись удалена
- Если Tauri window closes — lock() вызывается в on_window_event
- Panic в крипто-операции → lock() вызывается в panic hook

## GPU VRAM residue (LeftoverLocals)

Уязвимость: GPU может читать VRAM предыдущих процессов.
Затрагивает WebGL в WebView.

### Правила для Tauri WebView
- Запретить WebGL в webview: в tauri.conf.json добавить
  "allowlist": { "shell": ..., "webgl": false }  — если Tauri поддерживает
- Альтернатива: CSP заголовок Content-Security-Policy запрещает
  небезопасные WebGL API: добавить в Tauri window config
  "dangerousUseScheme": false
- Пароли НИКОГДА не передаются в WebView как plaintext JS переменные —
  только через IPC команды с немедленным использованием и очисткой
- В content script расширения: после автозаполнения поля — вызвать
  field.value = '' через 100ms (очистка из DOM)
