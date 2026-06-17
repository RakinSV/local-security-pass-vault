//! Тест-векторы из официальных источников (см. .claude/rules/testing.md).
//!
//! Примечание по Argon2id: libsodium `crypto_pwhash` фиксирует параллелизм p=1,
//! поэтому многолейновые тест-векторы RFC 9106 (p=4) этим API невоспроизводимы.
//! Здесь проверяем то, что воспроизводимо: HMAC-SHA256 (RFC 4231) и свойства
//! Argon2id (детерминизм/чувствительность к входу — в unit-тестах crypto).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use core_vault::sodium::{self, Key};

/// RFC 4231, Test Case 2 для HMAC-SHA256.
/// Key = "Jefe", Data = "what do ya want for nothing?"
/// HMAC-SHA256 = 5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
#[test]
fn hmac_sha256_rfc4231_case2() {
    sodium::init().expect("sodium init");

    // HMAC-SHA256 в libsodium требует ключ ровно 32 байта: "Jefe" дополняется
    // нулями справа до KEYBYTES — это эквивалентно стандартному HMAC, т.к.
    // ключ короче размера блока хешируется/паддится. libsodium crypto_auth_hmacsha256
    // принимает фиксированный 32-байтный ключ, поэтому используем low-level HMAC
    // через дополнение нулями (RFC 2104 short-key padding).
    let mut key_bytes = [0u8; 32];
    key_bytes[..4].copy_from_slice(b"Jefe");
    let key = Key::from_slice(&key_bytes).expect("key");

    let mac = sodium::hmac_sha256(b"what do ya want for nothing?", &key).expect("hmac");
    let expected =
        hex::decode("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843").unwrap();
    assert_eq!(mac.as_slice(), expected.as_slice());
}

/// Чувствительность HMAC: смена 1 бита сообщения → другой тег.
#[test]
fn hmac_sha256_bit_sensitivity() {
    sodium::init().expect("sodium init");
    let key = Key::from_slice(&[0x0bu8; 32]).expect("key");
    let a = sodium::hmac_sha256(b"message", &key).expect("hmac a");
    let b = sodium::hmac_sha256(b"messagf", &key).expect("hmac b");
    assert_ne!(a, b);
}
