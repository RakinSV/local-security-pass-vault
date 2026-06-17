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

/// Load or generate an Ed25519 key pair stored in `dir/signing.sk` and `dir/signing.pk`.
pub fn load_or_generate(dir: &Path) -> Result<(PublicKey, SecretKey), String> {
    let sk_path = dir.join("signing.sk");
    let pk_path = dir.join("signing.pk");

    if sk_path.exists() && pk_path.exists() {
        let sk_bytes = fs::read(&sk_path).map_err(|e| e.to_string())?;
        let pk_bytes = fs::read(&pk_path).map_err(|e| e.to_string())?;

        if sk_bytes.len() != SK_LEN || pk_bytes.len() != PK_LEN {
            return Err("signing key files have wrong length".into());
        }
        let mut sk = Box::new([0u8; SK_LEN]);
        sk.copy_from_slice(&sk_bytes);
        let mut pk = [0u8; PK_LEN];
        pk.copy_from_slice(&pk_bytes);
        return Ok((pk, sk));
    }

    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let mut pk = [0u8; PK_LEN];
    let mut sk = Box::new([0u8; SK_LEN]);
    let rc = unsafe { crypto_sign_ed25519_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
    if rc != 0 {
        return Err("libsodium keypair generation failed".into());
    }

    fs::write(&sk_path, sk.as_ref()).map_err(|e| e.to_string())?;
    fs::write(&pk_path, pk.as_ref()).map_err(|e| e.to_string())?;
    // Restrict to owner-only — signing keys must never be world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&sk_path, perms.clone()).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&pk_path, perms).map_err(|e| e.to_string())?;
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
