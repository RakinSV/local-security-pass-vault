#![no_main]
//! Фаззинг разбора контейнера vault.db. Цель: НЕТ panic / UB на любом мусоре —
//! только корректный Err. См. `.claude/rules/testing.md`.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    core_vault::vault::fuzz_open_container(data);
});
