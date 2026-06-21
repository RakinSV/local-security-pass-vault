/// Browser extension installer — detects browsers, enumerates their profiles,
/// and installs the LSPV extension via the appropriate OS mechanism:
///
/// Chrome / Edge / Brave (Windows):
///   HKCU\Software\<browser-key>\Extensions\<ext-id>
///     "path"    = "<path-to-extension.crx>"
///     "version" = "<version>"
///
/// Firefox (Windows):
///   Copies extension.xpi into each selected profile's extensions/ directory.
///   Firefox picks it up on next launch (may show unsigned-extension warning
///   unless you use Firefox Developer Edition or AMO-signed XPI).
///
/// Linux / macOS stubs are included but only profile detection is implemented;
/// the install step falls back to a helpful error message.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedBrowser {
    pub id: String,       // "chrome" | "edge" | "brave" | "firefox"
    pub name: String,
    pub installed: bool,
    pub profiles: Vec<BrowserProfile>,
    pub supports_per_profile: bool, // true for Firefox only
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    pub id: String,       // "Default", "Profile 1", or Firefox profile ID
    pub name: String,
    pub path: String,     // Absolute path to profile directory
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub browser_id: String,
    pub profile_ids: Vec<String>, // Only used for Firefox
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub browser_id: String,
    pub success: bool,
    pub message: String,
}

// ── Chrome extension ID for LSPV ─────────────────────────────────────────────
// Generated from the RSA public key in extension/signing-key.pem.
// If the key file doesn't exist yet, this is a placeholder.
const LSPV_CHROME_EXT_ID: &str = "lspv-local-security-pass-vault";
const LSPV_FIREFOX_EXT_ID: &str = "lspv@lspv.app"; // from manifest browser_specific_settings
const LSPV_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Browser detection ─────────────────────────────────────────────────────────

pub fn detect_browsers() -> Vec<DetectedBrowser> {
    vec![
        detect_chromium("chrome", "Google Chrome", chromium_data_dir("chrome")),
        detect_chromium("edge",   "Microsoft Edge", chromium_data_dir("edge")),
        detect_chromium("brave",  "Brave",           chromium_data_dir("brave")),
        detect_firefox(),
    ]
}

fn detect_chromium(id: &str, name: &str, data_dir: Option<PathBuf>) -> DetectedBrowser {
    let installed = data_dir.as_deref().map_or(false, |d| d.exists());
    let profiles = if installed {
        chromium_profiles(data_dir.as_deref().expect("checked above"))
    } else {
        vec![]
    };
    DetectedBrowser {
        id: id.into(),
        name: name.into(),
        installed,
        profiles,
        supports_per_profile: true, // Show profiles for info; registry install is still per-user
    }
}

fn detect_firefox() -> DetectedBrowser {
    let ff_dir = firefox_dir();
    let installed = ff_dir.as_deref().map_or(false, |d| d.exists());
    let profiles = if installed {
        firefox_profiles(ff_dir.as_deref().expect("checked above"))
    } else {
        vec![]
    };
    DetectedBrowser {
        id: "firefox".into(),
        name: "Firefox".into(),
        installed,
        profiles,
        supports_per_profile: true,
    }
}

// ── OS-specific path helpers ──────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn chromium_data_dir(browser: &str) -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let base = PathBuf::from(local);
    let path = match browser {
        "chrome" => base.join("Google").join("Chrome").join("User Data"),
        "edge"   => base.join("Microsoft").join("Edge").join("User Data"),
        "brave"  => base.join("BraveSoftware").join("Brave-Browser").join("User Data"),
        _ => return None,
    };
    Some(path)
}

#[cfg(target_os = "linux")]
fn chromium_data_dir(browser: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let base = PathBuf::from(&home).join(".config");
    let path = match browser {
        "chrome" => base.join("google-chrome"),
        "edge"   => base.join("microsoft-edge"),
        "brave"  => base.join("BraveSoftware").join("Brave-Browser"),
        _ => return None,
    };
    Some(path)
}

#[cfg(target_os = "macos")]
fn chromium_data_dir(browser: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let base = PathBuf::from(&home).join("Library").join("Application Support");
    let path = match browser {
        "chrome" => base.join("Google").join("Chrome"),
        "edge"   => base.join("Microsoft Edge"),
        "brave"  => base.join("BraveSoftware").join("Brave-Browser"),
        _ => return None,
    };
    Some(path)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn chromium_data_dir(_browser: &str) -> Option<PathBuf> { None }

// Firefox directory

#[cfg(target_os = "windows")]
fn firefox_dir() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(appdata).join("Mozilla").join("Firefox"))
}

#[cfg(target_os = "linux")]
fn firefox_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".mozilla").join("firefox"))
}

#[cfg(target_os = "macos")]
fn firefox_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join("Library").join("Application Support").join("Firefox"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn firefox_dir() -> Option<PathBuf> { None }

// ── Profile enumeration ───────────────────────────────────────────────────────

/// Reads Chrome/Edge/Brave profiles from `<user_data>/Local State` JSON.
fn chromium_profiles(user_data: &Path) -> Vec<BrowserProfile> {
    let local_state_path = user_data.join("Local State");
    let Ok(raw) = std::fs::read_to_string(&local_state_path) else {
        return vec![];
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return vec![];
    };
    let Some(info_cache) = json
        .get("profile")
        .and_then(|p| p.get("info_cache"))
        .and_then(|v| v.as_object())
    else {
        return vec![];
    };

    let mut profiles: Vec<BrowserProfile> = info_cache
        .iter()
        .filter_map(|(id, info)| {
            let name = info.get("name")?.as_str().unwrap_or(id).to_string();
            let path = user_data.join(id).to_string_lossy().into_owned();
            Some(BrowserProfile { id: id.clone(), name, path })
        })
        .collect();

    // Default profile first, then alphabetical
    profiles.sort_by(|a, b| {
        if a.id == "Default" { std::cmp::Ordering::Less }
        else if b.id == "Default" { std::cmp::Ordering::Greater }
        else { a.id.cmp(&b.id) }
    });
    profiles
}

/// Reads Firefox profiles from `profiles.ini`.
fn firefox_profiles(ff_dir: &Path) -> Vec<BrowserProfile> {
    let ini_path = ff_dir.join("profiles.ini");
    let Ok(content) = std::fs::read_to_string(&ini_path) else {
        return vec![];
    };

    let mut profiles = Vec::new();
    let mut current_name = String::new();
    let mut current_path_rel: Option<String> = None;
    let mut current_is_relative = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            // Flush previous profile
            if !current_name.is_empty() {
                if let Some(rel) = current_path_rel.take() {
                    let abs = if current_is_relative {
                        ff_dir.join(&rel)
                    } else {
                        PathBuf::from(&rel)
                    };
                    let id = abs
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| rel.clone());
                    profiles.push(BrowserProfile {
                        id,
                        name: current_name.clone(),
                        path: abs.to_string_lossy().into_owned(),
                    });
                }
            }
            current_name.clear();
            current_is_relative = false;
            current_path_rel = None;
        } else if let Some(v) = line.strip_prefix("Name=") {
            current_name = v.to_string();
        } else if let Some(v) = line.strip_prefix("Path=") {
            current_path_rel = Some(v.to_string());
        } else if line == "IsRelative=1" {
            current_is_relative = true;
        }
    }
    // Flush last
    if !current_name.is_empty() {
        if let Some(rel) = current_path_rel {
            let abs = if current_is_relative {
                ff_dir.join(&rel)
            } else {
                PathBuf::from(&rel)
            };
            let id = abs
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or(rel);
            profiles.push(BrowserProfile { id, name: current_name, path: abs.to_string_lossy().into_owned() });
        }
    }
    profiles
}

// ── Extension installation ─────────────────────────────────────────────────────

/// Finds the extension artifact (CRX for Chrome, XPI for Firefox) in the
/// directory next to the running executable.
fn find_ext_file(filename: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // Check next to exe, then in resources/ subdirectory
    for candidate in [dir.join(filename), dir.join("resources").join(filename)] {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn install_extension(requests: &[InstallRequest]) -> Vec<InstallResult> {
    let crx_path = find_ext_file("extension.crx");
    let xpi_path = find_ext_file("extension.xpi");

    requests
        .iter()
        .map(|req| {
            let result = match req.browser_id.as_str() {
                "chrome" | "edge" | "brave" => {
                    install_chromium(&req.browser_id, crx_path.as_deref())
                }
                "firefox" => install_firefox(xpi_path.as_deref(), &req.profile_ids),
                other => Err(format!("Unknown browser: {other}")),
            };
            match result {
                Ok(msg) => InstallResult { browser_id: req.browser_id.clone(), success: true,  message: msg },
                Err(e)  => InstallResult { browser_id: req.browser_id.clone(), success: false, message: e  },
            }
        })
        .collect()
}

// ── Chrome / Edge / Brave installation ───────────────────────────────────────

#[cfg(target_os = "windows")]
fn install_chromium(browser_id: &str, crx_path: Option<&Path>) -> Result<String, String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let crx = crx_path.ok_or_else(|| {
        "extension.crx not found next to the application. \
         Please re-download the latest LSPV installer.".to_string()
    })?;
    let crx_str = crx.to_string_lossy();

    // Each Chromium-based browser uses a different registry base path
    let reg_base = match browser_id {
        "chrome" => r"Software\Google\Chrome\Extensions",
        "edge"   => r"Software\Microsoft\Edge\Extensions",
        "brave"  => r"Software\BraveSoftware\Brave-Browser\Extensions",
        other    => return Err(format!("Unsupported browser: {other}")),
    };

    let ext_key_path = format!(r"{reg_base}\{LSPV_CHROME_EXT_ID}");

    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey_with_flags(&ext_key_path, KEY_SET_VALUE)
        .map_err(|e| format!("Failed to create registry key: {e}"))?;

    key.set_value("path",    &crx_str.as_ref())
        .map_err(|e| format!("Failed to set path: {e}"))?;
    key.set_value("version", &LSPV_VERSION)
        .map_err(|e| format!("Failed to set version: {e}"))?;

    Ok(format!(
        "Extension registered in registry. Restart {browser} to activate.",
        browser = match browser_id { "edge" => "Edge", "brave" => "Brave", _ => "Chrome" }
    ))
}

#[cfg(target_os = "linux")]
fn install_chromium(browser_id: &str, crx_path: Option<&Path>) -> Result<String, String> {
    let crx = crx_path.ok_or_else(|| {
        "extension.crx not found. Please re-download the latest LSPV AppImage.".to_string()
    })?;

    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let ext_dir = match browser_id {
        "chrome" => PathBuf::from(&home).join(".config/google-chrome/External Extensions"),
        "edge"   => PathBuf::from(&home).join(".config/microsoft-edge/External Extensions"),
        "brave"  => PathBuf::from(&home).join(".config/BraveSoftware/Brave-Browser/External Extensions"),
        other    => return Err(format!("Unsupported browser: {other}")),
    };

    std::fs::create_dir_all(&ext_dir)
        .map_err(|e| format!("Cannot create External Extensions dir: {e}"))?;

    let json = serde_json::json!({
        "external_crx": crx.to_string_lossy(),
        "external_version": LSPV_VERSION,
    });
    let json_path = ext_dir.join(format!("{LSPV_CHROME_EXT_ID}.json"));
    std::fs::write(&json_path, serde_json::to_string_pretty(&json).unwrap_or_default())
        .map_err(|e| format!("Cannot write External Extensions JSON: {e}"))?;

    Ok(format!("Extension registered. Restart the browser to activate."))
}

#[cfg(target_os = "macos")]
fn install_chromium(browser_id: &str, crx_path: Option<&Path>) -> Result<String, String> {
    let crx = crx_path.ok_or_else(|| {
        "extension.crx not found. Please re-download the latest LSPV DMG.".to_string()
    })?;

    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let ext_dir = match browser_id {
        "chrome" => PathBuf::from(&home).join("Library/Application Support/Google/Chrome/External Extensions"),
        "edge"   => PathBuf::from(&home).join("Library/Application Support/Microsoft Edge/External Extensions"),
        "brave"  => PathBuf::from(&home).join("Library/Application Support/BraveSoftware/Brave-Browser/External Extensions"),
        other    => return Err(format!("Unsupported browser: {other}")),
    };

    std::fs::create_dir_all(&ext_dir)
        .map_err(|e| format!("Cannot create External Extensions dir: {e}"))?;

    let json = serde_json::json!({
        "external_crx": crx.to_string_lossy(),
        "external_version": LSPV_VERSION,
    });
    let json_path = ext_dir.join(format!("{LSPV_CHROME_EXT_ID}.json"));
    std::fs::write(&json_path, serde_json::to_string_pretty(&json).unwrap_or_default())
        .map_err(|e| format!("Cannot write External Extensions JSON: {e}"))?;

    Ok("Extension registered. Restart the browser to activate.".into())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn install_chromium(_browser_id: &str, _crx_path: Option<&Path>) -> Result<String, String> {
    Err("Browser extension installation not supported on this platform.".into())
}

// ── Firefox installation ──────────────────────────────────────────────────────

fn install_firefox(xpi_path: Option<&Path>, profile_ids: &[String]) -> Result<String, String> {
    let xpi = xpi_path.ok_or_else(|| {
        "extension.xpi not found. Please re-download the latest LSPV installer.".to_string()
    })?;

    if profile_ids.is_empty() {
        return Err("No Firefox profiles selected.".into());
    }

    let ff_dir = firefox_dir()
        .ok_or_else(|| "Firefox directory not found.".to_string())?;
    let profiles = firefox_profiles(&ff_dir);

    let mut installed_count = 0;
    let mut errors: Vec<String> = vec![];

    for pid in profile_ids {
        let profile = profiles.iter().find(|p| &p.id == pid);
        let profile_path = match profile {
            Some(p) => PathBuf::from(&p.path),
            None => {
                errors.push(format!("Profile {pid} not found"));
                continue;
            }
        };

        if !profile_path.exists() {
            errors.push(format!("Profile {pid}: directory does not exist"));
            continue;
        }

        let ext_dir = profile_path.join("extensions");
        if let Err(e) = std::fs::create_dir_all(&ext_dir) {
            errors.push(format!("Profile {pid}: cannot create extensions dir: {e}"));
            continue;
        }

        let dest = ext_dir.join(format!("{LSPV_FIREFOX_EXT_ID}.xpi"));
        if let Err(e) = std::fs::copy(xpi, &dest) {
            errors.push(format!("Profile {pid}: cannot copy XPI: {e}"));
            continue;
        }

        installed_count += 1;
    }

    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    Ok(format!(
        "Extension installed to {installed_count} Firefox profile(s). \
         Restart Firefox to activate. If Firefox shows an unsigned extension warning, \
         visit addons.mozilla.org for the signed version."
    ))
}
