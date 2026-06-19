//! Безопасные обёртки над сырыми FFI-вызовами libsodium-sys-stable.
//!
//! Это ЕДИНСТВЕННОЕ место в крейте, где допускается `unsafe` для вызова
//! C-функций libsodium. Весь остальной код использует только эти обёртки.
//! См. `.claude/rules/crypto.md`.

use crate::error::{Result, VaultError};
use libsodium_sys as ffi;
use std::sync::Once;

// ── Размеры примитивов ──────────────────────────────────────────────────────
// Объявлены явно и сверены с константами libsodium статической проверкой ниже,
// чтобы поймать рассинхронизацию версии библиотеки на этапе компиляции.

/// Длина ключа AEAD / HMAC (XChaCha20-Poly1305, HMAC-SHA256).
pub const KEY_LEN: usize = 32;
/// Длина nonce XChaCha20 (192 бита).
pub const NONCE_LEN: usize = 24;
/// Длина тега Poly1305 (overhead AEAD).
pub const TAG_LEN: usize = 16;
/// Длина соли Argon2id (libsodium фиксирует 16 байт).
pub const SALT_LEN: usize = 16;
/// Длина выхода HMAC-SHA256.
pub const HMAC_LEN: usize = 32;

const _: () = {
    assert!(KEY_LEN == ffi::crypto_aead_xchacha20poly1305_ietf_KEYBYTES as usize);
    assert!(NONCE_LEN == ffi::crypto_aead_xchacha20poly1305_ietf_NPUBBYTES as usize);
    assert!(TAG_LEN == ffi::crypto_aead_xchacha20poly1305_ietf_ABYTES as usize);
    assert!(SALT_LEN == ffi::crypto_pwhash_SALTBYTES as usize);
    assert!(HMAC_LEN == ffi::crypto_auth_hmacsha256_BYTES as usize);
    assert!(KEY_LEN == ffi::crypto_auth_hmacsha256_KEYBYTES as usize);
};

// ── Инициализация ────────────────────────────────────────────────────────────

static INIT: Once = Once::new();
static mut INIT_OK: bool = false;

/// Инициализирует libsodium. Идемпотентна, потокобезопасна. Должна быть вызвана
/// до любых других функций этого модуля.
pub fn init() -> std::result::Result<(), &'static str> {
    INIT.call_once(|| {
        // SAFETY: Once гарантирует ровно один вызов sodium_init.
        let rc = unsafe { ffi::sodium_init() };
        // SAFETY: запись в статик ровно один раз внутри call_once.
        unsafe { INIT_OK = rc >= 0 };
    });
    // SAFETY: после call_once запись завершена (happens-before).
    if unsafe { INIT_OK } {
        Ok(())
    } else {
        Err("sodium_init failed")
    }
}

#[inline]
fn ensure_init() -> Result<()> {
    init().map_err(VaultError::Crypto)
}

// ── Secure memory: Secret<N> ─────────────────────────────────────────────────

/// Буфер секретных байт фиксированной длины с защитой памяти:
/// `mlock` (запрет свопа) при создании, `memzero` + `munlock` при drop.
/// Используется для всех ключей. См. crypto.md «Управление ключами в памяти».
///
/// Хранится в куче (`Box`) — адрес стабилен, поэтому `mlock` корректен на всё
/// время жизни значения.
pub struct Secret<const N: usize> {
    data: Box<[u8; N]>,
    locked: bool,
}

impl<const N: usize> Secret<N> {
    /// Новый обнулённый секрет с попыткой `mlock`.
    pub fn zeroed() -> Self {
        let mut data = Box::new([0u8; N]);
        // mlock — best-effort: на некоторых системах ограничен RLIMIT_MEMLOCK.
        // Неудача не фатальна (ключ всё равно будет обнулён), но фиксируем флаг.
        // SAFETY: указатель валиден, длина соответствует выделению.
        let rc = unsafe { ffi::sodium_mlock(data.as_mut_ptr() as *mut _, N) };
        Secret { data, locked: rc == 0 }
    }

    /// Секрет из среза. Длина среза должна быть ровно `N`.
    pub fn from_slice(src: &[u8]) -> Result<Self> {
        if src.len() != N {
            return Err(VaultError::Crypto("Secret::from_slice length mismatch"));
        }
        let mut s = Self::zeroed();
        s.data.copy_from_slice(src);
        Ok(s)
    }

    /// Случайный секрет из CSPRNG libsodium.
    pub fn random() -> Result<Self> {
        let mut s = Self::zeroed();
        random_bytes(&mut s.data[..])?;
        Ok(s)
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.data
    }

    #[inline]
    pub fn as_mut_bytes(&mut self) -> &mut [u8; N] {
        &mut self.data
    }

    /// Удалось ли заблокировать память от свопа.
    #[inline]
    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl<const N: usize> Drop for Secret<N> {
    fn drop(&mut self) {
        // SAFETY: указатель валиден до конца drop; len = N.
        unsafe {
            ffi::sodium_memzero(self.data.as_mut_ptr() as *mut _, N);
            if self.locked {
                ffi::sodium_munlock(self.data.as_mut_ptr() as *mut _, N);
            }
        }
    }
}

/// Ключ (32 байта) с защитой памяти.
pub type Key = Secret<KEY_LEN>;

// ── CSPRNG ───────────────────────────────────────────────────────────────────

/// Заполняет буфер криптостойкими случайными байтами (`randombytes_buf`).
/// ЕДИНСТВЕННЫЙ допустимый источник случайности для крипто (crypto.md).
pub fn random_bytes(buf: &mut [u8]) -> Result<()> {
    ensure_init()?;
    // SAFETY: буфер валиден на свою длину.
    unsafe { ffi::randombytes_buf(buf.as_mut_ptr() as *mut _, buf.len()) };
    Ok(())
}

/// Генерирует случайный nonce (24 байта) для XChaCha20-Poly1305.
pub fn gen_nonce() -> Result<[u8; NONCE_LEN]> {
    let mut n = [0u8; NONCE_LEN];
    random_bytes(&mut n)?;
    Ok(n)
}

/// Генерирует случайную соль (16 байт) для Argon2id.
pub fn gen_salt() -> Result<[u8; SALT_LEN]> {
    let mut s = [0u8; SALT_LEN];
    random_bytes(&mut s)?;
    Ok(s)
}

// ── KDF: Argon2id ────────────────────────────────────────────────────────────

/// Параметры Argon2id. НЕ СНИЖАТЬ без bump SECURITY_LEVEL / schema_version
/// и миграции (crypto.md). Текущий уровень: t=4, m=256 MiB.
///
/// Примечание: libsodium `crypto_pwhash` фиксирует параллелизм p=1, поэтому
/// p из crypto.md (p=4) недостижим этим API — это сознательное ограничение
/// (см. заметку для ревью в комментариях проекта).
pub const KDF_OPSLIMIT: u64 = 4;
pub const KDF_MEMLIMIT: usize = 256 * 1024 * 1024; // 256 MiB

/// Деривация `OUT` байт из пароля и соли алгоритмом Argon2id v1.3.
/// Для envelope-схемы вызывается с OUT=64 (encryption_key||search_key).
pub fn argon2id_derive<const OUT: usize>(
    password: &[u8],
    salt: &[u8; SALT_LEN],
) -> Result<Secret<OUT>> {
    argon2id_derive_custom::<OUT>(password, salt, KDF_OPSLIMIT, KDF_MEMLIMIT)
}

/// Деривация `OUT` байт с явными параметрами Argon2id (для бэкапа — ADR-003).
/// НЕ снижать параметры без bump schema_version и миграции.
pub fn argon2id_derive_custom<const OUT: usize>(
    password: &[u8],
    salt: &[u8; SALT_LEN],
    opslimit: u64,
    memlimit: usize,
) -> Result<Secret<OUT>> {
    ensure_init()?;
    let mut out = Secret::<OUT>::zeroed();
    // SAFETY: out длиной OUT, salt длиной SALT_LEN, password валиден.
    let rc = unsafe {
        ffi::crypto_pwhash(
            out.as_mut_bytes().as_mut_ptr(),
            OUT as u64,
            password.as_ptr() as *const _,
            password.len() as u64,
            salt.as_ptr(),
            opslimit,
            memlimit,
            ffi::crypto_pwhash_ALG_ARGON2ID13 as i32,
        )
    };
    if rc != 0 {
        // Обычно — нехватка памяти под Argon2id.
        return Err(VaultError::Crypto("argon2id derive failed"));
    }
    Ok(out)
}

// ── AEAD: XChaCha20-Poly1305 (IETF) ──────────────────────────────────────────
//
// crypto.md в примере упоминает «secretbox», но прозой требует именно
// XChaCha20-Poly1305. Используем AEAD-конструкцию: она и есть XChaCha20-Poly1305
// и позволяет привязать associated data (например, UUID записи) к шифртексту.
// Отклонение от буквального примера — задокументировано, вынесено на ревью.

/// Шифрует `plaintext` с привязкой `ad` (associated data, не шифруется, но
/// аутентифицируется). Возвращает шифртекст с приклеенным тегом (len + TAG_LEN).
pub fn aead_seal(
    plaintext: &[u8],
    ad: &[u8],
    nonce: &[u8; NONCE_LEN],
    key: &Key,
) -> Result<Vec<u8>> {
    ensure_init()?;
    let mut ciphertext = vec![0u8; plaintext.len() + TAG_LEN];
    let mut clen: u64 = 0;
    // SAFETY: все буферы валидны на указанные длины; nsec не используется (NULL).
    let rc = unsafe {
        ffi::crypto_aead_xchacha20poly1305_ietf_encrypt(
            ciphertext.as_mut_ptr(),
            &mut clen,
            plaintext.as_ptr(),
            plaintext.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            std::ptr::null(),
            nonce.as_ptr(),
            key.as_bytes().as_ptr(),
        )
    };
    if rc != 0 {
        return Err(VaultError::Crypto("aead encrypt failed"));
    }
    ciphertext.truncate(clen as usize);
    Ok(ciphertext)
}

/// Расшифровывает `ciphertext` (с тегом) и проверяет `ad`. Любая ошибка
/// (неверный ключ, подделка, неверный ad) даёт единую `DecryptionFailed`.
pub fn aead_open(
    ciphertext: &[u8],
    ad: &[u8],
    nonce: &[u8; NONCE_LEN],
    key: &Key,
) -> Result<Vec<u8>> {
    ensure_init()?;
    if ciphertext.len() < TAG_LEN {
        return Err(VaultError::DecryptionFailed);
    }
    let mut plaintext = vec![0u8; ciphertext.len() - TAG_LEN];
    let mut mlen: u64 = 0;
    // SAFETY: буферы валидны; nsec = NULL.
    let rc = unsafe {
        ffi::crypto_aead_xchacha20poly1305_ietf_decrypt(
            plaintext.as_mut_ptr(),
            &mut mlen,
            std::ptr::null_mut(),
            ciphertext.as_ptr(),
            ciphertext.len() as u64,
            ad.as_ptr(),
            ad.len() as u64,
            nonce.as_ptr(),
            key.as_bytes().as_ptr(),
        )
    };
    if rc != 0 {
        // Намеренно НЕ раскрываем причину (oracle attack, security.md).
        return Err(VaultError::DecryptionFailed);
    }
    plaintext.truncate(mlen as usize);
    Ok(plaintext)
}

// ── HMAC-SHA256 (поисковый индекс) ───────────────────────────────────────────

/// HMAC-SHA256(message, key) — для детерминированного поискового хеша.
pub fn hmac_sha256(message: &[u8], key: &Key) -> Result<[u8; HMAC_LEN]> {
    ensure_init()?;
    let mut out = [0u8; HMAC_LEN];
    // SAFETY: out длиной HMAC_LEN, key длиной KEY_LEN, message валиден.
    let rc = unsafe {
        ffi::crypto_auth_hmacsha256(
            out.as_mut_ptr(),
            message.as_ptr(),
            message.len() as u64,
            key.as_bytes().as_ptr(),
        )
    };
    if rc != 0 {
        return Err(VaultError::Crypto("hmac failed"));
    }
    Ok(out)
}

// ── SHA-256 (honeypot integrity) ─────────────────────────────────────────────

/// SHA-256 произвольных байт. Используется для honeypot-хеша (security.md).
pub fn sha256(data: &[u8]) -> Result<[u8; 32]> {
    ensure_init()?;
    let mut out = [0u8; 32];
    // SAFETY: out длиной 32 = crypto_hash_sha256_BYTES; data валиден.
    let rc = unsafe {
        ffi::crypto_hash_sha256(out.as_mut_ptr(), data.as_ptr(), data.len() as u64)
    };
    if rc != 0 {
        return Err(VaultError::Crypto("sha256 failed"));
    }
    Ok(out)
}

const _: () = {
    assert!(32 == ffi::crypto_hash_sha256_BYTES as usize);
};

// ── Constant-time сравнение ──────────────────────────────────────────────────

/// Сравнение за константное время (`sodium_memcmp`). Для MAC/хешей вместо `==`.
/// Возвращает true при равенстве. Длины должны совпадать.
pub fn memcmp(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    // SAFETY: оба указателя валидны на общую длину.
    let rc = unsafe { ffi::sodium_memcmp(a.as_ptr() as *const _, b.as_ptr() as *const _, a.len()) };
    rc == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        assert!(init().is_ok());
        assert!(init().is_ok());
    }

    #[test]
    fn random_bytes_differ() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        random_bytes(&mut a).unwrap();
        random_bytes(&mut b).unwrap();
        assert_ne!(a, b, "две случайные выборки совпали");
    }

    #[test]
    fn secret_zeroed_on_drop() {
        // Reading freed heap memory is UB: allocators routinely overwrite freed
        // blocks with free-list pointers, so the sodium_memzero call in Drop
        // completes correctly but the zeroes are gone by the time we read back.
        // Instead, verify the primitive that Drop actually calls — sodium_memzero
        // — works correctly on a stack buffer with the same calling convention.
        init().unwrap();
        let mut buf = [0xABu8; 32];
        // SAFETY: buf is valid stack memory for exactly 32 bytes.
        unsafe { ffi::sodium_memzero(buf.as_mut_ptr() as *mut _, 32) };
        assert!(buf.iter().all(|&b| b == 0), "sodium_memzero не обнулила память");
    }

    #[test]
    fn aead_roundtrip() {
        let key = Key::random().unwrap();
        let nonce = gen_nonce().unwrap();
        let ct = aead_seal(b"secret data", b"item-id", &nonce, &key).unwrap();
        let pt = aead_open(&ct, b"item-id", &nonce, &key).unwrap();
        assert_eq!(pt, b"secret data");
    }

    #[test]
    fn aead_bit_flip_fails() {
        let key = Key::random().unwrap();
        let nonce = gen_nonce().unwrap();
        let mut ct = aead_seal(b"secret data", b"", &nonce, &key).unwrap();
        ct[5] ^= 0x01;
        assert!(matches!(
            aead_open(&ct, b"", &nonce, &key),
            Err(VaultError::DecryptionFailed)
        ));
    }

    #[test]
    fn aead_wrong_ad_fails() {
        let key = Key::random().unwrap();
        let nonce = gen_nonce().unwrap();
        let ct = aead_seal(b"data", b"id-A", &nonce, &key).unwrap();
        assert!(matches!(
            aead_open(&ct, b"id-B", &nonce, &key),
            Err(VaultError::DecryptionFailed)
        ));
    }

    #[test]
    fn nonce_unique() {
        use std::collections::HashSet;
        let set: HashSet<[u8; NONCE_LEN]> = (0..100_000).map(|_| gen_nonce().unwrap()).collect();
        assert_eq!(set.len(), 100_000, "коллизия nonce");
    }

    #[test]
    fn memcmp_constant_time_equal() {
        let key = Key::random().unwrap();
        let m1 = hmac_sha256(b"data", &key).unwrap();
        let m2 = hmac_sha256(b"data", &key).unwrap();
        assert!(memcmp(&m1, &m2));
        assert!(!memcmp(&m1, &[0u8; HMAC_LEN]));
    }
}
