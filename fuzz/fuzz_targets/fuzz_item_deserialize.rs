#![no_main]
//! Фаззинг загрузки произвольного образа SQLite (sqlite3_deserialize).
//! Цель: НЕТ panic / UB — только Err на повреждённых данных.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = core_vault::db::Db::from_plaintext(data);
});
