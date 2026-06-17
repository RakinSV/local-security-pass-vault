mod commands;
mod ed25519_key;
mod error;
mod pipe_server;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    core_vault::init().expect("libsodium init failed");

    tauri::Builder::default()
        .manage(state::AppState::default())
        .setup(|app| {
            // Load (or generate) Ed25519 signing key pair
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("cannot resolve app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            match ed25519_key::load_or_generate(&data_dir) {
                Ok((pk, sk)) => {
                    let state = app.state::<state::AppState>();
                    *state.sign_sk.lock().unwrap() = Some(sk);
                    *state.sign_pk_hex.lock().unwrap() =
                        Some(ed25519_key::public_key_hex(&pk));
                }
                Err(e) => eprintln!("Warning: could not load signing key: {e}"),
            }

            // Start named-pipe IPC server for the browser extension
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                pipe_server::run(handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::get_default_vault_dir,
            commands::create_vault,
            commands::open_vault,
            commands::lock_vault,
            commands::list_items,
            commands::get_item,
            commands::create_item,
            commands::update_item,
            commands::delete_item,
            commands::change_master_password,
            commands::get_signing_public_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
