//! Integration: полный жизненный цикл vault на 1000 записях.
//! Соответствует сценарию из `.claude/rules/testing.md` (Фаза 2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::manual_is_multiple_of)]

use core_vault::models::{CustomField, ItemPayload};
use core_vault::{Vault, VaultError};
use uuid::Uuid;

fn tmp_dir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vp_life_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Разнотипный payload по индексу — покрывает все 5 типов записей.
fn payload_for(i: usize) -> ItemPayload {
    match i % 5 {
        0 => ItemPayload::Login {
            url: format!("https://site{i}.example"),
            username: format!("user{i}"),
            password: format!("p@ss-{i}-{}", "x".repeat(i % 17)),
            totp_secret: if i % 3 == 0 { Some("JBSWY3DPEHPK3PXP".into()) } else { None },
            notes: None,
            custom_fields: vec![CustomField {
                label: "pin".into(),
                value: format!("{:04}", i % 10000),
                hidden: true,
            }],
            password_history: vec![],
        },
        1 => ItemPayload::Card {
            cardholder: format!("Holder {i}"),
            number: format!("4111111111111{:03}", i % 1000),
            expiry_month: ((i % 12) + 1) as u8,
            expiry_year: 2030,
            cvv: format!("{:03}", i % 1000),
            notes: None,
        },
        2 => ItemPayload::Note {
            content: format!("# Заметка {i}\nСодержимое с юникодом: ключ-{i} 🔐"),
        },
        3 => ItemPayload::Identity {
            first_name: Some(format!("Имя{i}")),
            last_name: Some(format!("Фамилия{i}")),
            email: Some(format!("id{i}@example.com")),
            phone: None,
            address: None,
            passport: Some(format!("AB{:06}", i)),
            notes: None,
        },
        _ => ItemPayload::SshKey {
            private_key: format!("-----BEGIN KEY-----\nfake-key-material-{i}\n-----END KEY-----"),
            public_key: Some(format!("ssh-ed25519 AAAA{i}")),
            passphrase: None,
            notes: None,
        },
    }
}

const N: usize = 1000;

#[test]
fn full_vault_lifecycle_1000_items() {
    let dir = tmp_dir();

    // 1. Создать vault и наполнить 1000 записями разных типов.
    let mut ids = Vec::with_capacity(N);
    {
        let mut v = Vault::create(&dir, b"correct-horse-battery-staple", None).unwrap();
        for i in 0..N {
            let id = v
                .add_item(&format!("Item {i}"), payload_for(i), None, i % 7 == 0, None)
                .unwrap();
            ids.push(id);
        }
        v.save().unwrap();

        // 2. Сменить мастер-пароль (записи не должны пострадать).
        v.change_master_password(b"correct-horse-battery-staple", b"new-secure-passphrase")
            .unwrap();
    } // vault закрыт — ключи обнулены при drop

    // 3. Старый пароль больше не открывает.
    assert!(matches!(
        Vault::open(&dir, b"correct-horse-battery-staple", None),
        Err(VaultError::DecryptionFailed)
    ));

    // 4. Открыть новым паролем и проверить все 1000 записей.
    let v = Vault::open(&dir, b"new-secure-passphrase", None).unwrap();
    let items = v.list_items().unwrap();
    assert_eq!(items.len(), N, "потеряны записи после смены пароля");

    for (i, id) in ids.iter().enumerate() {
        let item = v.get_item(id).unwrap().expect("запись пропала");
        assert_eq!(item.title, format!("Item {i}"));
        assert_eq!(item.payload, payload_for(i), "payload запис{i} повреждён");
    }

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn reopened_vault_search_works() {
    let dir = tmp_dir();
    {
        let mut v = Vault::create(&dir, b"pw", None).unwrap();
        for i in 0..50 {
            v.add_item(&format!("Site {i}"), payload_for(i), None, false, None)
                .unwrap();
        }
        v.save().unwrap();
    }
    let v = Vault::open(&dir, b"pw", None).unwrap();
    let found = v.search("site 42").unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Site 42");
    std::fs::remove_dir_all(&dir).ok();
}
