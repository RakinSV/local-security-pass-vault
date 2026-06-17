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

    let server = ServerOptions::new()
        .pipe_mode(PipeMode::Byte)
        .create(PIPE_NAME)?;

    server.connect().await?;

    let mut stream = server;
    let app = app.clone();

    tokio::spawn(async move {
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

            let resp = handle(&app, &msg_buf).await;
            let resp_bytes = serde_json::to_vec(&resp).unwrap_or_default();

            let resp_len = (resp_bytes.len() as u32).to_le_bytes();
            if stream.write_all(&resp_len).await.is_err() {
                break;
            }
            if stream.write_all(&resp_bytes).await.is_err() {
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
            loop {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).await.is_err() { break; }
                let len = u32::from_le_bytes(len_buf) as usize;
                if len == 0 || len > 1024 * 1024 { break; }
                let mut msg_buf = vec![0u8; len];
                if stream.read_exact(&mut msg_buf).await.is_err() { break; }
                let resp = handle(&app, &msg_buf).await;
                let resp_bytes = serde_json::to_vec(&resp).unwrap_or_default();
                let resp_len = (resp_bytes.len() as u32).to_le_bytes();
                if stream.write_all(&resp_len).await.is_err() { break; }
                if stream.write_all(&resp_bytes).await.is_err() { break; }
            }
        });
    }
}

// ── Request handler ──────────────────────────────────────────────────────────

async fn handle(app: &AppHandle, raw: &[u8]) -> PipeResponse {
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
    let sk_guard = state.sign_sk.lock().unwrap();
    let sk = match sk_guard.as_ref() {
        Some(k) => k.clone(),
        None => return PipeResponse::err(&id, "signing key not available"),
    };
    drop(sk_guard);

    match req.action.as_str() {
        "status" => {
            let vault_guard = state.vault.lock().unwrap();
            let is_locked = vault_guard.is_none();
            let item_count: usize = if let Some(v) = vault_guard.as_ref() {
                v.list_items().map(|l| l.len()).unwrap_or(0)
            } else {
                0
            };
            drop(vault_guard);
            // Include public key so the extension can do TOFU signature verification.
            let pk_hex = state.sign_pk_hex.lock().unwrap().clone().unwrap_or_default();
            PipeResponse::ok(
                &id,
                serde_json::json!({
                    "isLocked": is_locked,
                    "itemCount": item_count,
                    "signingPublicKey": pk_hex
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

            let vault_guard = state.vault.lock().unwrap();
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

            let vault_guard = state.vault.lock().unwrap();
            let vault = match vault_guard.as_ref() {
                Some(v) => v,
                None => return PipeResponse::err(&id, "VaultLocked"),
            };

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
            *state.vault.lock().unwrap() = None;
            *state.vault_dir.lock().unwrap() = None;
            PipeResponse::ok(&id, Value::Null, &sk)
        }

        other => PipeResponse::err(&id, format!("unknown action: {other}")),
    }
}
