mod commands;
mod error;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise libsodium once before any vault operations.
    core_vault::init().expect("libsodium init failed");

    tauri::Builder::default()
        .manage(state::AppState::default())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
