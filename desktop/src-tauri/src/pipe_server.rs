//! Windows named-pipe IPC server for the browser extension ↔ native host ↔ Tauri bridge.
//!
//! Protocol (both directions): [u32 LE length][JSON bytes]
//!
//! Requests:
//!   { "id": "<uuid>", "action": "status"|"search"|"get_credentials"|"lock", "payload": … }
//!
//! Responses:
//!   { "id": "<uuid>", "success": true|false, "data": …, "error": "…", "signature": "<hex>" }

use crate::{ed25519_key, state::AppState};
use core_vault::models::ItemPayload;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};

pub const PIPE_NAME: &str = r"\\.\pipe\vaultpass";

// ── Windows: owner-only DACL for the named pipe ──────────────────────────────
//
// We use raw `extern "system"` declarations instead of importing from
// windows-sys feature flags to avoid feature-gating complexities.
// All APIs have been stable since Windows XP.
//
// The Box allocation keeps sd_buf + acl_buf at a stable heap address so that
// the pointer stored in sa.lp_security_descriptor remains valid.

#[cfg(windows)]
#[allow(non_snake_case)]
mod win_dacl {
    use std::ffi::c_void;
    use std::mem;

    #[link(name = "Kernel32")]
    extern "system" {
        pub fn GetCurrentProcess() -> isize;
        pub fn CloseHandle(hObject: isize) -> i32;
    }
    #[link(name = "Advapi32")]
    extern "system" {
        pub fn OpenProcessToken(
            ProcessHandle: isize, DesiredAccess: u32, TokenHandle: *mut isize,
        ) -> i32;
        pub fn GetTokenInformation(
            TokenHandle: isize, TokenInformationClass: i32,
            TokenInformation: *mut c_void, TokenInformationLength: u32,
            ReturnLength: *mut u32,
        ) -> i32;
        pub fn GetLengthSid(pSid: *mut c_void) -> u32;
        pub fn InitializeSecurityDescriptor(
            pSD: *mut c_void, dwRevision: u32,
        ) -> i32;
        pub fn InitializeAcl(
            pAcl: *mut c_void, nAclLength: u32, dwAclRevision: u32,
        ) -> i32;
        pub fn AddAccessAllowedAce(
            pAcl: *mut c_void, dwAceRevision: u32,
            AccessMask: u32, pSid: *mut c_void,
        ) -> i32;
        pub fn SetSecurityDescriptorDacl(
            pSD: *mut c_void, bDaclPresent: i32,
            pDacl: *mut c_void, bDaclDefaulted: i32,
        ) -> i32;
    }

    // TOKEN_USER = { Sid: *mut c_void, Attributes: u32 }
    #[repr(C)]
    struct SidAndAttributes { sid: *mut c_void, _attributes: u32 }
    #[repr(C)]
    struct TokenUser { user: SidAndAttributes }

    // SECURITY_ATTRIBUTES (#[repr(C)] — same layout as Win32 struct)
    #[repr(C)]
    pub struct SecurityAttributes {
        pub n_length:               u32,
        pub lp_security_descriptor: *mut c_void,
        pub b_inherit_handle:       i32,
    }

    // SECURITY_DESCRIPTOR is opaque; 64 bytes is safely larger than 40 (64-bit)
    // or 20 (32-bit).
    const SD_BUF: usize = 64;

    pub struct OwnerOnlySD {
        pub(super) sd:      [u8; SD_BUF],
        pub(super) acl_buf: [u8; 256],
        pub sa:             SecurityAttributes,
    }

    /// Build an owner-only SECURITY_ATTRIBUTES for a named pipe.
    /// Returns None on any failure; caller falls back to the default ACL.
    pub fn build() -> Option<Box<OwnerOnlySD>> {
        unsafe {
            // 1. Open process token to get the current user's SID.
            let mut token: isize = 0;
            if OpenProcessToken(GetCurrentProcess(), 0x0008 /* TOKEN_QUERY */, &mut token) == 0 {
                return None;
            }
            let mut needed: u32 = 0;
            GetTokenInformation(token, 1 /* TokenUser */, std::ptr::null_mut(), 0, &mut needed);
            let mut tok_buf = vec![0u8; needed as usize];
            let ok = GetTokenInformation(
                token, 1, tok_buf.as_mut_ptr().cast(), needed, &mut needed,
            );
            CloseHandle(token);
            if ok == 0 || needed == 0 { return None; }

            let user    = &*(tok_buf.as_ptr() as *const TokenUser);
            let sid     = user.user.sid;
            let sid_len = GetLengthSid(sid);
            if sid_len == 0 { return None; }

            // 2. Heap-allocate — internal pointers are stable while the Box lives.
            let mut r = Box::new(OwnerOnlySD {
                sd:      [0u8; SD_BUF],
                acl_buf: [0u8; 256],
                sa:      SecurityAttributes {
                    n_length: mem::size_of::<SecurityAttributes>() as u32,
                    lp_security_descriptor: std::ptr::null_mut(),
                    b_inherit_handle: 0,
                },
            });

            // 3. Blank security descriptor.
            if InitializeSecurityDescriptor(
                r.sd.as_mut_ptr().cast(), 1 /* SECURITY_DESCRIPTOR_REVISION */,
            ) == 0 { return None; }

            // 4. Empty ACL (256 bytes >> one ACE for one user SID).
            if InitializeAcl(
                r.acl_buf.as_mut_ptr().cast(), r.acl_buf.len() as u32,
                2 /* ACL_REVISION */,
            ) == 0 { return None; }

            // 5. Grant GENERIC_ALL to the current user.
            if AddAccessAllowedAce(
                r.acl_buf.as_mut_ptr().cast(), 2 /* ACL_REVISION */,
                0x1000_0000u32, /* GENERIC_ALL */ sid,
            ) == 0 { return None; }

            // 6. Attach DACL to the security descriptor.
            if SetSecurityDescriptorDacl(
                r.sd.as_mut_ptr().cast(),
                1,                             // bDaclPresent = TRUE
                r.acl_buf.as_mut_ptr().cast(), // pDacl
                0,                             // bDaclDefaulted = FALSE
            ) == 0 { return None; }

            // 7. Point sa at sd — both in the same Box, so the pointer is stable.
            r.sa.lp_security_descriptor = r.sd.as_mut_ptr().cast();
            Some(r)
        }
    }
}

#[derive(Deserialize)]
struct PipeRequest {
    id: String,
    action: String,
    payload: Option<Value>,
    #[serde(default)]
    profile_id: Option<String>,
    #[serde(default)]
    profile_email: Option<String>,
    #[serde(default)]
    browser_type: Option<String>,
    #[serde(default)]
    totp_code: Option<String>,
}

#[derive(Serialize)]
struct PipeResponse {
    id: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

impl PipeResponse {
    fn ok(id: &str, data: Value, sk: &ed25519_key::SecretKey) -> Self {
        let data_str = data.to_string();
        let sig = ed25519_key::sign_response(id, &data_str, sk);
        PipeResponse {
            id: id.to_owned(),
            success: true,
            data: Some(data),
            error: None,
            signature: Some(sig),
        }
    }

    fn err(id: &str, msg: impl Into<String>) -> Self {
        PipeResponse {
            id: id.to_owned(),
            success: false,
            data: None,
            error: Some(msg.into()),
            signature: None,
        }
    }
}

/// Start the pipe server in an async task. Accepts one client at a time.
pub async fn run(app: AppHandle) {
    #[cfg(windows)]
    {
        loop {
            match accept_one(&app).await {
                Ok(()) => {}
                Err(e) => eprintln!("[pipe] server error: {e}"),
            }
        }
    }

    #[cfg(not(windows))]
    {
        // Unix: use a Unix domain socket at /tmp/vaultpass.sock
        run_unix(app).await;
    }
}

// ── Windows implementation ──────────────────────────────────────────────────

#[cfg(windows)]
async fn accept_one(app: &AppHandle) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Restrict the pipe to the current user — prevents other local Windows
    // users from connecting and querying vault credentials.
    // The block scope ensures `_sd` (which contains *mut c_void, hence !Send)
    // is dropped before the first .await point, keeping the future Send.
    let server = {
        let _sd = win_dacl::build();
        let mut opts = ServerOptions::new();
        opts.pipe_mode(PipeMode::Byte);
        if let Some(ref sd) = _sd {
            // SAFETY: Windows copies the SD into the kernel during CreateNamedPipe;
            // `_sd` need not live beyond this call.
            unsafe {
                opts.create_with_security_attributes_raw(
                    PIPE_NAME,
                    &sd.sa as *const win_dacl::SecurityAttributes as *mut _,
                )?
            }
        } else {
            eprintln!("[pipe] could not build owner-only DACL — pipe open to all local users");
            opts.create(PIPE_NAME)?
        }
        // _sd drops here
    };

    server.connect().await?;

    let mut stream = server;
    let app = app.clone();

    tokio::spawn(async move {
        let mut totp_fail_count: u8 = 0;
        loop {
            // Read 4-byte length prefix
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {
                break;
            }
            let len = u32::from_le_bytes(len_buf) as usize;
            if len == 0 || len > 1024 * 1024 {
                break;
            }

            let mut msg_buf = vec![0u8; len];
            if stream.read_exact(&mut msg_buf).await.is_err() {
                break;
            }

            let resp = handle(&app, &msg_buf, &mut totp_fail_count).await;
            let resp_bytes = serde_json::to_vec(&resp).unwrap_or_default();

            let resp_len = (resp_bytes.len() as u32).to_le_bytes();
            if stream.write_all(&resp_len).await.is_err() {
                break;
            }
            if stream.write_all(&resp_bytes).await.is_err() {
                break;
            }

            // Drop connection after 5 consecutive TOTP failures to prevent brute force.
            if totp_fail_count >= 5 {
                break;
            }
        }
    });

    Ok(())
}

// ── Unix implementation ──────────────────────────────────────────────────────

#[cfg(not(windows))]
async fn run_unix(app: AppHandle) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;

    const SOCK_PATH: &str = "/tmp/vaultpass.sock";
    let _ = std::fs::remove_file(SOCK_PATH);

    let listener = match UnixListener::bind(SOCK_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[pipe] cannot bind {SOCK_PATH}: {e}");
            return;
        }
    };
    // Restrict socket to owner only — prevents other local users from connecting.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(SOCK_PATH, std::fs::Permissions::from_mode(0o600))
        {
            eprintln!("[pipe] failed to restrict socket permissions: {e}");
        }
    }

    loop {
        let Ok((mut stream, _)) = listener.accept().await else { continue };
        let app = app.clone();
        tokio::spawn(async move {
            let mut totp_fail_count: u8 = 0;
            loop {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).await.is_err() { break; }
                let len = u32::from_le_bytes(len_buf) as usize;
                if len == 0 || len > 1024 * 1024 { break; }
                let mut msg_buf = vec![0u8; len];
                if stream.read_exact(&mut msg_buf).await.is_err() { break; }
                let resp = handle(&app, &msg_buf, &mut totp_fail_count).await;
                let resp_bytes = serde_json::to_vec(&resp).unwrap_or_default();
                let resp_len = (resp_bytes.len() as u32).to_le_bytes();
                if stream.write_all(&resp_len).await.is_err() { break; }
                if stream.write_all(&resp_bytes).await.is_err() { break; }
                if totp_fail_count >= 5 { break; }
            }
        });
    }
}

// ── Request handler ──────────────────────────────────────────────────────────

async fn handle(app: &AppHandle, raw: &[u8], totp_fail_count: &mut u8) -> PipeResponse {
    let req: PipeRequest = match serde_json::from_slice(raw) {
        Ok(r) => r,
        Err(e) => {
            return PipeResponse::err("unknown", format!("bad json: {e}"));
        }
    };
    let id = req.id.clone();

    // Register/update the browser profile that sent this request
    if let Some(ref pid) = req.profile_id {
        if let Ok(data_dir) = app.path().app_data_dir() {
            crate::profile_registry::upsert(
                &data_dir,
                pid,
                req.profile_email.as_deref(),
                req.browser_type.as_deref(),
            );
        }
    }

    let state = app.state::<AppState>();
    let sk_guard = state.sign_sk.lock().unwrap_or_else(|e| e.into_inner());
    let sk = match sk_guard.as_ref() {
        Some(k) => k.clone(),
        None => return PipeResponse::err(&id, "signing key not available"),
    };
    drop(sk_guard);

    match req.action.as_str() {
        "status" => {
            let vault_guard = state.vault.lock().unwrap_or_else(|e| e.into_inner());
            let is_locked = vault_guard.is_none();
            let has_2fa = vault_guard.as_ref().map(|v| v.has_2fa()).unwrap_or(false);
            let item_count: usize = if let Some(v) = vault_guard.as_ref() {
                v.list_items().map(|l| l.len()).unwrap_or(0)
            } else {
                0
            };
            drop(vault_guard);
            // Include public key so the extension can do TOFU signature verification.
            let pk_hex = state.sign_pk_hex.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
            PipeResponse::ok(
                &id,
                serde_json::json!({
                    "isLocked": is_locked,
                    "itemCount": item_count,
                    "signingPublicKey": pk_hex,
                    "has2fa": has_2fa,
                }),
                &sk,
            )
        }

        "search" => {
            let query = req.payload
                .as_ref()
                .and_then(|p| p.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let page_url = req.payload
                .as_ref()
                .and_then(|p| p.get("pageUrl"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();

            let vault_guard = state.vault.lock().unwrap_or_else(|e| e.into_inner());
            let vault = match vault_guard.as_ref() {
                Some(v) => v,
                None => return PipeResponse::err(&id, "VaultLocked"),
            };

            let all = match vault.list_items() {
                Ok(l) => l,
                Err(_) => return PipeResponse::err(&id, "InternalError"),
            };

            let mut summaries = Vec::new();
            for summary in all {
                if !query.is_empty() && !summary.title.to_lowercase().contains(&query) {
                    continue;
                }
                // Fetch full item once to extract URL and username for login entries.
                let (url, username): (Option<String>, Option<String>) =
                    match vault.get_item(&summary.id) {
                        Ok(Some(item)) => match item.payload {
                            ItemPayload::Login { url, username, .. } => {
                                (Some(url), Some(username))
                            }
                            _ => (None, None),
                        },
                        _ => (None, None),
                    };

                summaries.push(serde_json::json!({
                    "id": summary.id.to_string(),
                    "itemType": format!("{:?}", summary.item_type).to_lowercase(),
                    "title": summary.title,
                    "url": url,
                    "username": username,
                    "favorite": summary.favorite,
                    "pageUrl": page_url,
                }));
            }

            drop(vault_guard);
            PipeResponse::ok(&id, Value::Array(summaries), &sk)
        }

        "get_credentials" => {
            let item_id_str = req.payload
                .as_ref()
                .and_then(|p| p.get("itemId"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let uuid = match uuid::Uuid::parse_str(item_id_str) {
                Ok(u) => u,
                Err(_) => return PipeResponse::err(&id, "invalid item id"),
            };

            let vault_guard = state.vault.lock().unwrap_or_else(|e| e.into_inner());
            let vault = match vault_guard.as_ref() {
                Some(v) => v,
                None => return PipeResponse::err(&id, "VaultLocked"),
            };

            // If vault has 2FA, require a valid TOTP code with every credential request.
            if vault.has_2fa() {
                match req.totp_code.as_deref() {
                    None | Some("") => {
                        drop(vault_guard);
                        return PipeResponse::err(&id, "TotpRequired");
                    }
                    Some(code) => {
                        if vault.verify_2fa_code(code).is_err() {
                            *totp_fail_count += 1;
                            drop(vault_guard);
                            return PipeResponse::err(&id, "TotpInvalid");
                        }
                        *totp_fail_count = 0;
                    }
                }
            }

            let item = match vault.get_item(&uuid) {
                Ok(Some(i)) => i,
                Ok(None) => return PipeResponse::err(&id, "not found"),
                Err(_) => return PipeResponse::err(&id, "InternalError"),
            };
            drop(vault_guard);

            match item.payload {
                ItemPayload::Login { username, password, .. } => {
                    PipeResponse::ok(
                        &id,
                        serde_json::json!({ "username": username, "password": password }),
                        &sk,
                    )
                }
                _ => PipeResponse::err(&id, "item is not a login"),
            }
        }

        "lock" => {
            crate::lock_vault_internal(app);
            PipeResponse::ok(&id, Value::Null, &sk)
        }

        other => PipeResponse::err(&id, format!("unknown action: {other}")),
    }
}
