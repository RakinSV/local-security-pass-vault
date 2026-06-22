mod browser_integration;
mod ext_installer;
mod commands;
mod csv_import;
mod ed25519_key;
mod error;
mod keychain;
mod pipe_server;
mod profile_registry;
mod state;

use std::time::Duration;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

/// Hardens the process against memory forensics and code injection attacks.
fn harden_process() {
    #[cfg(target_os = "linux")]
    // SAFETY: prctl is always safe to call with these arguments.
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }

    #[cfg(target_os = "linux")]
    harden_linux_seccomp();

    #[cfg(windows)]
    // SAFETY: Windows API calls at startup before any threads are spawned.
    unsafe {
        harden_windows();
    }
}

/// Installs a seccomp-BPF blacklist that kills the thread on dangerous
/// memory-forensics syscalls (ptrace, process_vm_readv/writev).
#[cfg(target_os = "linux")]
fn harden_linux_seccomp() {
    use seccompiler::{apply_filter, BpfProgram, SeccompAction, SeccompFilter};
    use std::collections::BTreeMap;

    let arch = if cfg!(target_arch = "x86_64") {
        seccompiler::TargetArch::x86_64
    } else if cfg!(target_arch = "aarch64") {
        seccompiler::TargetArch::aarch64
    } else {
        return; // unsupported arch — skip seccomp rather than break startup
    };

    // Blacklist: kill the thread if these syscalls are invoked.
    let mut rules: BTreeMap<i64, Vec<seccompiler::SeccompRule>> = BTreeMap::new();
    for &nr in &[
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
    ] {
        rules.insert(nr as i64, vec![]);
    }

    let filter = match SeccompFilter::new(
        rules,
        SeccompAction::Allow,      // default: allow all other syscalls
        SeccompAction::KillThread, // action on blacklisted syscall
        arch,
    ) {
        Ok(f) => f,
        Err(e) => { eprintln!("seccomp filter build failed: {e}"); return; }
    };

    let prog: BpfProgram = match filter.try_into() {
        Ok(p) => p,
        Err(e) => { eprintln!("seccomp BPF compile failed: {e}"); return; }
    };

    if let Err(e) = apply_filter(&prog) {
        eprintln!("seccomp apply failed: {e}");
    }
}

/// Applies Windows-specific process-level hardening at startup.
#[cfg(windows)]
unsafe fn harden_windows() {
    use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;
    use windows_sys::Win32::System::Threading::{
        SetProcessMitigationPolicy, ProcessDynamicCodePolicy, ProcessSignaturePolicy,
    };

    // Strip the current directory from the DLL search path to prevent
    // DLL planting attacks (attacker drops a DLL next to our .exe).
    let empty: [u16; 1] = [0];
    SetDllDirectoryW(empty.as_ptr());

    // Prohibit dynamic code (VirtualAlloc + mark-executable) — blocks
    // shellcode injection and JIT-spray attacks against our process.
    #[repr(C)]
    struct DynCodePolicy { flags: u32 }
    let dcp = DynCodePolicy { flags: 1 }; // ProhibitDynamicCode = bit 0
    SetProcessMitigationPolicy(
        ProcessDynamicCodePolicy,
        &dcp as *const _ as *const std::ffi::c_void,
        std::mem::size_of::<DynCodePolicy>(),
    );

    // Require Microsoft-signed DLLs — prevents loading unsigned DLLs
    // that could have been placed by an attacker (DLL hijacking).
    #[repr(C)]
    struct SigPolicy { flags: u32 }
    let sp = SigPolicy { flags: 1 }; // MicrosoftSignedOnly = bit 0
    SetProcessMitigationPolicy(
        ProcessSignaturePolicy,
        &sp as *const _ as *const std::ffi::c_void,
        std::mem::size_of::<SigPolicy>(),
    );
}

// ── Screen-lock watchers ──────────────────────────────────────────────────────

/// Windows: polling thread that detects screensaver activation and workstation
/// lock by calling GetSystemMetrics / OpenInputDesktop every 5 seconds.
/// Does not require a message-loop window — purely polling.
#[cfg(windows)]
fn start_windows_session_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        use std::sync::atomic::Ordering;

        extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
            fn OpenInputDesktop(dwFlags: u32, fInherit: i32, dwDesiredAccess: u32) -> isize;
            fn CloseDesktop(hDesktop: isize) -> i32;
        }
        // SM_SCREENSAVERRUNNING = 114: non-zero while the screensaver is running.
        const SM_SCREENSAVERRUNNING: i32 = 114;
        // OpenInputDesktop needs at minimum DESKTOP_SWITCHDESKTOP (0x0100).
        const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;

        let mut was_protected = false;

        loop {
            std::thread::sleep(Duration::from_secs(5));

            // Screensaver running?
            let screensaver = unsafe { GetSystemMetrics(SM_SCREENSAVERRUNNING) } != 0;

            // Workstation locked? OpenInputDesktop returns NULL when the session
            // is on the secure desktop (Win+L or Ctrl+Alt+Del locked screen).
            let hdesk = unsafe { OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP) };
            let locked = hdesk == 0;
            if hdesk != 0 {
                unsafe { CloseDesktop(hdesk); }
            }

            let protected = screensaver || locked;
            if protected && !was_protected {
                was_protected = true;
                let state = app.state::<state::AppState>();
                if state.lock_on_screensaver.load(Ordering::Relaxed) {
                    lock_vault_internal(&app);
                }
            } else if !protected {
                was_protected = false;
            }
        }
    });
}

/// Linux: async task that subscribes to org.freedesktop.ScreenSaver (and
/// org.gnome.ScreenSaver) D-Bus ActiveChanged signals via zbus.
#[cfg(target_os = "linux")]
async fn watch_linux_screensaver(app: tauri::AppHandle) {
    use std::sync::atomic::Ordering;

    if let Err(e) = watch_linux_screensaver_inner(&app).await {
        eprintln!("[screensaver] D-Bus watcher exited: {e}");
    }
}

#[cfg(target_os = "linux")]
async fn watch_linux_screensaver_inner(
    app: &tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    use zbus::{Connection, MatchRule, MessageStream};
    use futures_util::StreamExt as _;
    use std::sync::atomic::Ordering;

    let conn = Connection::session().await?;

    for iface in &["org.freedesktop.ScreenSaver", "org.gnome.ScreenSaver"] {
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(*iface)?
            .member("ActiveChanged")?
            .build();
        conn.add_match_rule(rule).await.ok();
    }

    let mut stream = MessageStream::from(&conn);
    while let Some(msg) = stream.next().await {
        let msg = msg?;
        // Filter: only handle ActiveChanged signals
        if msg.header().member().map(|m| m.as_str()) != Some("ActiveChanged") {
            continue;
        }
        // The body is a single bool: true = screensaver/lock active.
        if let Ok(active) = msg.body().deserialize::<bool>() {
            if active {
                let state = app.state::<state::AppState>();
                if state.lock_on_screensaver.load(Ordering::Relaxed) {
                    lock_vault_internal(app);
                }
            }
        }
    }

    Ok(())
}

/// Помечает vault.db read-only (0444 на Unix, FILE_ATTRIBUTE_READONLY на Windows).
/// Вызывать ПОСЛЕ закрытия SQLCipher-соединения.
fn set_db_readonly(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444));
    }
    #[cfg(windows)]
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_readonly(true);
        let _ = std::fs::set_permissions(path, perms);
    }
    #[cfg(not(any(unix, windows)))]
    let _ = path;
}

/// Очищает системный буфер обмена (best-effort, без паники при ошибке).
fn clear_clipboard_sync() {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn OpenClipboard(hWndNewOwner: isize) -> i32;
            fn EmptyClipboard() -> i32;
            fn CloseClipboard() -> i32;
        }
        // SAFETY: стандартные Win32 clipboard API, не требуют синхронизации между
        // потоками если вызываются при активном desktop-приложении.
        unsafe {
            if OpenClipboard(0) != 0 {
                EmptyClipboard();
                CloseClipboard();
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            cb.clear().ok();
        }
    }
}

/// Locks the vault (if open) and emits `vault-locked` to the frontend.
fn lock_vault_internal(app: &tauri::AppHandle) {
    let state = app.state::<state::AppState>();

    // 1. Выгружаем vault из памяти (guard дропается → SQLCipher закрывает файл).
    let vault_id: Option<String> = {
        let x = match state.vault.lock() {
            Ok(mut guard) => {
                let id = guard.as_ref().map(|v| v.vault_id_str());
                *guard = None;
                id
            }
            Err(_) => None,
        }; x
    };

    // 2. Помечаем vault.db read-only (SQLCipher уже закрыл файл).
    if let Ok(dir_guard) = state.vault_dir.lock() {
        if let Some(ref dir) = *dir_guard {
            set_db_readonly(&dir.join("vault.db"));
        }
    }

    // 3. Очищаем буфер обмена.
    clear_clipboard_sync();

    // 4. Удаляем ключ из keychain и уведомляем frontend.
    if let Some(id) = vault_id {
        keychain::delete_vault_key(&id);
        app.emit("vault-locked", ()).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    harden_process();
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

            // ── Screen-lock / screensaver watchers ────────────────────────────
            #[cfg(windows)]
            start_windows_session_watcher(app.handle().clone());

            #[cfg(target_os = "linux")]
            {
                let handle_ss = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    watch_linux_screensaver(handle_ss).await;
                });
            }

            // ── Auto-lock idle checker (every 30 s) ───────────────────────────
            let handle_al = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    let state = handle_al.state::<state::AppState>();
                    let timeout = state.auto_lock_secs();
                    if timeout == 0 { continue; }
                    let is_open = state.vault.lock().map(|g| g.is_some()).unwrap_or(false);
                    if is_open && state.idle_secs() >= timeout {
                        lock_vault_internal(&handle_al);
                    }
                }
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

            // ── Close window → hide to tray (+ optional lock) ────────────────
            if let Some(main_win) = app.get_webview_window("main") {
                let w = main_win.clone();
                let h = app.handle().clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let state = h.state::<state::AppState>();
                        if state.lock_on_minimize() {
                            lock_vault_internal(&h);
                        }
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
            commands::list_source_tags,
            commands::bulk_retag_items,
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
            commands::get_auto_lock_settings,
            commands::set_auto_lock_settings,
            commands::activity_ping,
            commands::keychain_vault_status,
            commands::list_auto_backups,
            commands::pick_backup_file,
            commands::pick_backup_save_path,
            commands::generate_totp,
            commands::decode_qr_from_clipboard,
            commands::detect_browsers_for_extension,
            commands::install_extension_to_browsers,
            commands::list_deleted_items,
            commands::restore_item,
            commands::purge_item,
            commands::purge_all_trash,
            commands::list_folders,
            commands::add_folder,
            commands::delete_folder,
            commands::rename_folder,
            commands::import_bitwarden_json,
            commands::vault_requires_2fa,
            commands::vault_has_2fa,
            commands::setup_vault_2fa,
            commands::confirm_vault_2fa,
            commands::disable_vault_2fa,
            commands::get_health_report,
            commands::pick_csv_save_path,
            commands::export_items_csv,
            commands::set_screen_capture_protection,
            commands::check_password_breach,
            commands::copy_to_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
