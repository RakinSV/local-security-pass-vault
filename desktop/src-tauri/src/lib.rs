mod browser_integration;
mod commands;
mod csv_import;
mod ed25519_key;
mod error;
mod keychain;
mod pipe_server;
mod profile_registry;
mod state;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    core_vault::init().expect("libsodium init failed");

    tauri::Builder::default()
        .manage(state::AppState::default())
        .setup(|app| {
            // ── Ed25519 signing key ───────────────────────────────────────────
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

            // ── Named-pipe IPC for browser extension ──────────────────────────
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                pipe_server::run(handle).await;
            });

            // ── System tray ───────────────────────────────────────────────────
            let show_item = MenuItemBuilder::with_id("show", "Show Window").build(app)?;
            let lock_item = MenuItemBuilder::with_id("lock", "Lock && Hide").build(app)?;
            let sep       = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .items(&[&show_item, &lock_item, &sep, &quit_item])
                .build()?;

            if let Some(icon) = app.default_window_icon().cloned() {
                TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("Local Security Pass Vault")
                    .menu(&tray_menu)
                    .on_menu_event(|app, event| {
                        match event.id().as_ref() {
                            "show" => {
                                if let Some(w) = app.get_webview_window("main") {
                                    w.show().ok();
                                    w.set_focus().ok();
                                }
                            }
                            "lock" => {
                                let state = app.state::<state::AppState>();
                                if let Ok(mut guard) = state.vault.lock() {
                                    if let Some(vault) = guard.as_ref() {
                                        keychain::delete_vault_key(&vault.vault_id_str());
                                    }
                                    *guard = None;
                                }
                                app.emit("vault-locked", ()).ok();
                                if let Some(w) = app.get_webview_window("main") {
                                    w.hide().ok();
                                }
                            }
                            "quit" => app.exit(0),
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                if w.is_visible().unwrap_or(false) {
                                    w.hide().ok();
                                } else {
                                    w.show().ok();
                                    w.set_focus().ok();
                                }
                            }
                        }
                    })
                    .build(app)?;
            }

            // ── Close window → hide to tray ───────────────────────────────────
            if let Some(main_win) = app.get_webview_window("main") {
                let w = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        w.hide().ok();
                    }
                });
            }

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
            commands::get_browser_integrations,
            commands::save_browser_integrations,
            commands::get_native_host_path,
            commands::parse_import_csv,
            commands::import_items_from_csv,
            commands::get_profiles,
            commands::set_profile_name,
            commands::generate_seed_phrase,
            commands::validate_seed_phrase,
            commands::export_backup,
            commands::restore_backup,
            commands::suggest_vault_dir,
            commands::pick_folder,
            commands::open_github,
            commands::keychain_has_key,
            commands::keychain_delete_key,
            commands::get_autostart,
            commands::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
