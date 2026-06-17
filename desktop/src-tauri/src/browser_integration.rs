use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BrowserConfig {
    pub chrome_ids: Vec<String>,
    pub firefox_ids: Vec<String>,
}

const CONFIG_FILE: &str = "browser_config.json";
const MANIFEST_FILE: &str = "com.vaultpass.native.json";

pub fn load(data_dir: &Path) -> BrowserConfig {
    let path = data_dir.join(CONFIG_FILE);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(data_dir: &Path, config: &BrowserConfig) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(data_dir.join(CONFIG_FILE), json)
}

/// Find the native host binary.
/// In production: looks next to the app executable.
/// In development: walks up the directory tree looking for target/{release,debug}/.
pub fn find_native_host_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?.to_path_buf();

    #[cfg(windows)]
    let bin = "vaultpass-native-host.exe";
    #[cfg(not(windows))]
    let bin = "vaultpass-native-host";

    let candidate = exe_dir.join(bin);
    if candidate.exists() {
        return Some(candidate);
    }

    // Development: walk up looking for target/{release,debug}/
    let mut dir = exe_dir;
    for _ in 0..8 {
        for profile in &["release", "debug"] {
            let c = dir.join(profile).join(bin);
            if c.exists() {
                return Some(c);
            }
        }
        match dir.parent().map(|p| p.to_path_buf()) {
            Some(p) => dir = p,
            None => break,
        }
    }

    None
}

/// Write the native messaging manifest + register it in the OS.
/// Returns the native host binary path on success.
pub fn install(data_dir: &Path, config: &BrowserConfig) -> Result<String, String> {
    save_config(data_dir, config).map_err(|e| e.to_string())?;

    let host_path = find_native_host_binary().ok_or_else(|| {
        "Native host binary not found.\nBuild it first: cargo build -p vaultpass-native-host --release".to_owned()
    })?;

    write_manifest(data_dir, &host_path, config).map_err(|e| e.to_string())?;
    register_os(data_dir)?;

    Ok(host_path.to_string_lossy().into_owned())
}

fn write_manifest(data_dir: &Path, host_path: &Path, config: &BrowserConfig) -> std::io::Result<()> {
    let allowed_origins: Vec<String> = config.chrome_ids.iter()
        .map(|id| format!("chrome-extension://{id}/"))
        .collect();

    let manifest = serde_json::json!({
        "name": "com.vaultpass.native",
        "description": "VaultPass native messaging host",
        "path": host_path,
        "type": "stdio",
        "allowed_origins": allowed_origins,
        "allowed_extensions": config.firefox_ids,
    });

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(data_dir.join(MANIFEST_FILE), json)
}

#[cfg(windows)]
fn register_os(data_dir: &Path) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let manifest_path = data_dir
        .join(MANIFEST_FILE)
        .to_string_lossy()
        .into_owned();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for subkey in &[
        r"Software\Google\Chrome\NativeMessagingHosts\com.vaultpass.native",
        r"Software\Microsoft\Edge\NativeMessagingHosts\com.vaultpass.native",
        r"Software\Mozilla\NativeMessagingHosts\com.vaultpass.native",
    ] {
        let (key, _disp) = hkcu.create_subkey(subkey).map_err(|e| e.to_string())?;
        key.set_value("", &manifest_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn register_os(data_dir: &Path) -> Result<(), String> {
    let src = data_dir.join(MANIFEST_FILE);
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return Ok(());
    }
    let targets = [
        format!("{home}/.config/google-chrome/NativeMessagingHosts"),
        format!("{home}/.config/chromium/NativeMessagingHosts"),
        format!("{home}/.mozilla/native-messaging-hosts"),
    ];
    for t in &targets {
        let dir = std::path::Path::new(t);
        if std::fs::create_dir_all(dir).is_ok() {
            let _ = std::fs::copy(&src, dir.join(MANIFEST_FILE));
        }
    }
    Ok(())
}
