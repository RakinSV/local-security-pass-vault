use libsodium_sys::{
    crypto_sign_ed25519_PUBLICKEYBYTES, crypto_sign_ed25519_SECRETKEYBYTES,
    crypto_sign_ed25519_BYTES, crypto_sign_ed25519_keypair,
    crypto_sign_ed25519_detached,
};
use std::fs;
use std::path::Path;

pub const PK_LEN: usize = crypto_sign_ed25519_PUBLICKEYBYTES as usize;
pub const SK_LEN: usize = crypto_sign_ed25519_SECRETKEYBYTES as usize;
pub const SIG_LEN: usize = crypto_sign_ed25519_BYTES as usize;

pub type PublicKey = [u8; PK_LEN];
pub type SecretKey = Box<[u8; SK_LEN]>;

const KEYRING_SERVICE: &str = "vaultpass-signing";
const KEYRING_ACCOUNT: &str = "ed25519-sk";

// Attempt to load the SK from the OS keychain (DPAPI on Windows, Keychain on
// macOS, libsecret on Linux).  Returns None on any error (service unavailable,
// no entry stored, wrong size).
fn sk_keyring_load() -> Option<SecretKey> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).ok()?;
    let hex = entry.get_password().ok()?;
    let bytes = hex::decode(&hex).ok()?;
    if bytes.len() != SK_LEN { return None; }
    let mut sk = Box::new([0u8; SK_LEN]);
    sk.copy_from_slice(&bytes);
    Some(sk)
}

// Store the SK in the OS keychain.  Returns false if the keychain is unavailable.
fn sk_keyring_store(sk: &SecretKey) -> bool {
    let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT) else {
        return false;
    };
    entry.set_password(&hex::encode(sk.as_ref())).is_ok()
}

/// Load or generate an Ed25519 key pair.
///
/// Priority order:
/// 1. OS keychain (DPAPI / Keychain / libsecret) — preferred, OS-encrypted.
/// 2. Plaintext file `signing.sk` — legacy fallback; migrated to keychain on
///    first successful load.
///
/// On fresh generation, the SK is stored in the keychain.  If the keychain is
/// unavailable (headless environments), the plaintext file is used with 0o600
/// permissions on Unix.
pub fn load_or_generate(dir: &Path) -> Result<(PublicKey, SecretKey), String> {
    let sk_path = dir.join("signing.sk");
    let pk_path = dir.join("signing.pk");

    // ── Case 1: public key exists → load or migrate secret key ───────────────
    if pk_path.exists() {
        let pk_bytes = fs::read(&pk_path).map_err(|e| e.to_string())?;
        if pk_bytes.len() == PK_LEN {
            let mut pk = [0u8; PK_LEN];
            pk.copy_from_slice(&pk_bytes);

            // Prefer keychain.
            if let Some(sk) = sk_keyring_load() {
                // Migration: remove the plaintext file if it still exists.
                if sk_path.exists() { let _ = fs::remove_file(&sk_path); }
                return Ok((pk, sk));
            }

            // Fallback: plaintext file (migrate it to keychain if possible).
            if sk_path.exists() {
                let sk_bytes = fs::read(&sk_path).map_err(|e| e.to_string())?;
                if sk_bytes.len() == SK_LEN {
                    let mut sk = Box::new([0u8; SK_LEN]);
                    sk.copy_from_slice(&sk_bytes);
                    // Migrate: on success remove the plaintext file.
                    if sk_keyring_store(&sk) {
                        let _ = fs::remove_file(&sk_path);
                    }
                    return Ok((pk, sk));
                }
            }
        }
    }

    // ── Case 2: generate a fresh key pair ─────────────────────────────────────
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let mut pk = [0u8; PK_LEN];
    let mut sk = Box::new([0u8; SK_LEN]);
    let rc = unsafe { crypto_sign_ed25519_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
    if rc != 0 {
        return Err("libsodium keypair generation failed".into());
    }

    fs::write(&pk_path, pk.as_ref()).map_err(|e| e.to_string())?;

    // Store SK in keychain; fall back to plaintext file if unavailable.
    if !sk_keyring_store(&sk) {
        fs::write(&sk_path, sk.as_ref()).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&sk_path, perms.clone()).map_err(|e| e.to_string())?;
            std::fs::set_permissions(&pk_path, perms).map_err(|e| e.to_string())?;
        }
    }

    Ok((pk, sk))
}

/// Sign `id + json_data` and return hex-encoded 64-byte signature.
pub fn sign_response(id: &str, data: &str, sk: &SecretKey) -> String {
    let msg = format!("{id}{data}");
    let msg_bytes = msg.as_bytes();
    let mut sig = [0u8; SIG_LEN];
    let mut sig_len: u64 = 0;
    unsafe {
        crypto_sign_ed25519_detached(
            sig.as_mut_ptr(),
            &mut sig_len,
            msg_bytes.as_ptr(),
            msg_bytes.len() as u64,
            sk.as_ptr(),
        );
    }
    hex::encode(&sig[..sig_len as usize])
}

/// Return the hex-encoded public key.
pub fn public_key_hex(pk: &PublicKey) -> String {
    hex::encode(pk)
}
