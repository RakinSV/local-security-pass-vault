# Security Architecture

LSPV uses envelope encryption with two key layers and eight security controls.

## Key Hierarchy

```
Master Password
      │
      ▼
Argon2id KDF (256 MB RAM, t=4, p=4)
      │
      ├─► db_key [bytes 0..32]      → SQLCipher PRAGMA key (x'...')
      ├─► encryption_key [32..64]   → decrypts Vault Key
      └─► search_key [64..96]       → HMAC-SHA256 for title search index

Vault Key (32 bytes, random at creation)
      │
      ▼
XChaCha20-Poly1305 per record
      │
      ▼
SQLCipher database (AES-256 per page)
```

## Why Three Keys from One KDF

Deriving three mathematically independent keys from a single Argon2id pass avoids:

1. **Circular dependency ("chicken-and-egg")** — the database is encrypted with `db_key` derived directly from the master password; without it you can't open the DB to find the encrypted Vault Key.
2. **Key reuse** — `encryption_key` and `search_key` are distinct so a compromise of one doesn't affect the other.

## Layers

### Layer 1 — Argon2id KDF

- Parameters: memory=256 MB, time=4 iterations, parallelism=4
- Salt: 32 random bytes, stored in `vault.salt`, created once at vault creation and never changed
- Output: 96 bytes → split into three 32-byte keys

Changing the master password re-runs Argon2id to produce new keys, then re-encrypts only the Vault Key. Records are untouched.

### Layer 2 — Vault Key Encryption

- Algorithm: XChaCha20-Poly1305 (libsodium `secretbox`)
- Nonce: 192-bit, random, unique per save
- The encrypted Vault Key is stored in the `vault` table alongside its nonce

### Layer 3 — SQLCipher Database

- Each SQLite page is encrypted with AES-256 using `db_key`
- LSPV passes `db_key` as a raw key (`PRAGMA key = x'...'`) to bypass SQLCipher's built-in PBKDF2 (we've already done better KDF via Argon2id)
- Individual records in `items` table are additionally encrypted by the Vault Key

### Layer 4 — OS Keychain

After the first successful password unlock, LSPV stores the Vault Key in the OS native secret store:
- **Windows:** Windows Credential Manager (DPAPI-backed)
- **macOS:** Keychain via Security.framework
- **Linux:** libsecret (org.freedesktop.secrets)

On subsequent unlocks, LSPV offers quick-unlock using biometrics or PIN instead of re-deriving via Argon2id. If the keychain is unavailable (CI, headless), it falls back gracefully to password-only mode.

On `lock()`, the keychain entry is deleted — not just the in-memory key.

### Layer 5 — Memory Safety

- Vault Key pages are `mlock()`-ed to prevent them from being swapped to disk
- All key structs implement `zeroize::Zeroize` and are zeroed on `Drop`
- Passwords from the UI are processed in Rust, never stored as JS variables

### Layer 6 — Encrypted Backups

See [Backup & Recovery](Backup-and-Recovery.md) for the full format spec.

### Layer 7 — Browser IPC Trust

The browser extension connects to the desktop via a named pipe. To prevent a malicious local process from spoofing responses:

1. The desktop generates an Ed25519 key pair at first run
2. On the first successful `status` response, the extension stores the desktop's public key in `chrome.storage.local` (**Trust-On-First-Use**)
3. Every subsequent response from the desktop includes an Ed25519 signature over `id + JSON.stringify(data)`
4. The extension verifies the signature before processing any response
5. The stored public key is never overwritten automatically — only a deliberate user action can reset TOFU

### Layer 8 — Ransomware Detection

At startup, LSPV creates a honeypot file (`vault_backup.db`) next to `vault.db` containing random bytes, and stores its SHA-256 hash in memory. On every vault unlock, it recomputes the hash. If the file changed, it means something modified it externally — LSPV immediately locks the vault and warns the user.

## Atomic Writes

Vault saves are always atomic:
1. Write to `vault.db.tmp`
2. `fsync()` — ensure bytes reach disk
3. `rename(tmp, vault.db)` — atomic on POSIX; `MoveFileExW` on Windows

A crash at any step leaves the previous vault intact.

## Symlink Protection

Before opening any vault file, LSPV calls `lstat()` and refuses if the path is a symlink. On Unix, files are opened with `O_NOFOLLOW`.

## What Is Not Encrypted

For operational reasons, these fields are stored in plaintext:
- `item_type` — needed to filter without decrypting all records
- `folder_id`, `favorite`, `deleted` — filter flags
- `created_at`, `updated_at` — sort order and sync
- `title_search_hash` — HMAC(lowercase(title), search_key); reveals whether a search query matches, not the title itself
