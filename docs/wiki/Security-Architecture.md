# LSPV Security Architecture — 8 Layers of Protection

> **Zero cloud. Zero telemetry. Zero trust.**  
> Every password you store passes through 8 independent security layers before touching disk.

---

## The 8-Layer Model

```
╔══════════════════════════════════════════════════════════════════════╗
║                     YOU & YOUR MASTER PASSWORD                       ║
╚══════════════════════╦═══════════════════════════════════════════════╝
                       │
         ┌─────────────▼──────────────────────────────────────────┐
         │  LAYER 1 — KEY DERIVATION (Argon2id)                   │
         │                                                         │
         │  Master Password + 32-byte random Salt                  │
         │       ↓  Argon2id  (256 MB RAM · 4 iterations)          │
         │  ┌──────────────┬──────────────┬──────────────┐         │
         │  │  db_key      │  enc_key     │ search_key   │         │
         │  │  (32 bytes)  │  (32 bytes)  │  (32 bytes)  │         │
         │  │  SQLCipher   │  Vault Key   │  HMAC index  │         │
         │  └──────────────┴──────────────┴──────────────┘         │
         │  Brute-force cost: ~2-4 seconds per attempt on RTX 4090  │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 2 — ENVELOPE ENCRYPTION                           │
         │                                                          │
         │  Random Vault Key (32 bytes, generated ONCE at create)   │
         │       ↓  XChaCha20-Poly1305 (libsodium secretbox)        │
         │  encrypted_vault_key  stored in vault table              │
         │                                                          │
         │  Password change = re-encrypt Vault Key only             │
         │  Records untouched — Vault Key stays the same            │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 3 — RECORD ENCRYPTION (XChaCha20-Poly1305)        │
         │                                                          │
         │  Each record encrypted INDEPENDENTLY with:               │
         │  • Unique 192-bit nonce (new on every save)              │
         │  • Associated data: item_id + field_name (domain sep.)   │
         │  • Authenticated: MAC covers ciphertext + AD             │
         │                                                          │
         │  Flip 1 bit → authentication fails → record rejected     │
         │  No two records share a nonce — ever                     │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 4 — DATABASE ENCRYPTION (SQLCipher)               │
         │                                                          │
         │  The .db file is AES-256-CBC encrypted page-by-page      │
         │  Key = db_key (from Argon2id, never stored on disk)      │
         │  Opening the file without db_key → reads as random noise │
         │                                                          │
         │  Even if vault.db is stolen → unreadable binary blob     │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 5 — OS KEYCHAIN (Windows Credential Manager)      │
         │                                                          │
         │  After unlock: Vault Key cached in OS Keychain           │
         │  Enables quick re-open without re-entering password      │
         │  On lock(): Keychain entry DELETED, not just cleared      │
         │  Keychain protected by Windows DPAPI (hardware-backed)   │
         │                                                          │
         │  No Keychain access = no quick unlock = full Argon2id    │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 6 — MEMORY PROTECTION                             │
         │                                                          │
         │  mlock()   — keys pinned, OS cannot swap to disk         │
         │  zeroize   — keys overwritten with 0x00 on lock/drop     │
         │  Rust ownership — no dangling key references possible    │
         │  No core dumps, no ptrace attach (PR_SET_DUMPABLE=0)     │
         │                                                          │
         │  Auto-lock: 5 min idle · on minimize · on sleep          │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 7 — CLIPBOARD PROTECTION                          │
         │                                                          │
         │  Copied password → auto-cleared after 30 seconds         │
         │  CF_EXCLUDEFROMCLOUDCLIPBOARD — not synced to MS cloud   │
         │  Lock event → clipboard cleared immediately              │
         │  Passwords never logged, never in URL params             │
         └─────────────┬───────────────────────────────────────────┘
                       │
         ┌─────────────▼───────────────────────────────────────────┐
         │  LAYER 8 — OFFLINE BACKUP (BIP-39 + age encryption)      │
         │                                                          │
         │  24-word mnemonic (256-bit BIP-39 entropy)               │
         │  Shown ONCE at vault creation — never stored on disk      │
         │  Backup KDF: Argon2id 4 GB RAM · 10 iterations           │
         │  Format: age-encrypted .vbk + BLAKE3 integrity check     │
         │                                                          │
         │  Even with file stolen: 4 GB/attempt ≈ 30s on RTX 4090  │
         │  2048²⁴ combinations = brute-force physically impossible  │
         └─────────────────────────────────────────────────────────┘
```

---

## What is stored where

| Data | Location | Protected by |
|------|----------|-------------|
| Master password | **Nowhere** — never stored | Stays in your head |
| Argon2id salt | `vault.salt` (disk) | Public — harmless without password |
| Encrypted Vault Key | `vault.db` → table `vault` | Layers 1 + 4 |
| Encrypted records | `vault.db` → table `items` | Layers 2 + 3 + 4 |
| Vault Key (unlocked) | RAM only | Layers 5 + 6 |
| Decrypted passwords | RAM, shown in UI | Layers 6 + 7 |
| Backup mnemonic | Paper / your memory | Never on any device |

---

## What is PLAIN (readable without decryption)

By design — needed for filtering without decrypting every record:

| Field | Why plain |
|-------|-----------|
| `item_type` | login / card / note / identity / ssh_key / server | Filter sidebar |
| `folder_id` | Folder membership | Organize without decrypt |
| `favorite`, `deleted` | UI flags | Sort / soft-delete |
| `created_at`, `updated_at` | Timestamps | Sort, sync |
| `source_tag` | Import label (browser profile name) | Filter by origin |
| `title_search_hash` | HMAC-SHA256 of title | Search without revealing content |

**Everything else is encrypted.** Title, URL, username, password, notes, SSH keys, tokens — all ciphertext at rest.

---

## Threat Model

| Attack vector | Mitigation |
|---------------|-----------|
| Stolen `vault.db` | SQLCipher AES-256 + XChaCha20-Poly1305 — unreadable without password |
| RAM dump while **locked** | `zeroize` + `mlock` — keys zero-wiped on lock |
| RAM dump while **unlocked** | `mlock` prevents swap to disk; AEAD per-field limits exposure |
| Brute-force master password | Argon2id 256 MB ≈ 2-4s/attempt even on high-end GPU |
| Clipboard sniffing | 30 s TTL + cloud clipboard sync excluded |
| Ransomware | Honeypot file — unauthorized writes trigger alert |
| Symlink attack on vault.db | `lstat()` + `O_NOFOLLOW` before open |
| Replay attack (browser ext.) | Unique nonce per IPC + Ed25519 signature verification |
| DLL hijacking (Windows) | libsodium statically linked — zero external DLLs |
| Browser extension XSS | `connect-src 'none'` CSP — zero network from extension |

---

## Cryptographic Primitives

```
Key derivation:     Argon2id         libsodium  (RFC 9106 compliant)
Symmetric AEAD:     XChaCha20-Poly1305  libsodium crypto_aead_xchacha20poly1305_ietf
Search index MAC:   HMAC-SHA256      libsodium crypto_auth_hmacsha256
Database layer:     AES-256-CBC      SQLCipher (page-level)
Backup encryption:  age + Argon2id   4 GB profile
Backup integrity:   BLAKE3           content-addressed
Browser↔Desktop:    Ed25519          libsodium crypto_sign_ed25519
RNG:                OS CSPRNG        libsodium randombytes_buf()
```

> All cryptography goes through **libsodium** — the most widely audited  
> portable crypto library. Zero home-grown crypto. Zero `rand::thread_rng()`.

---

## Key Lifetime

```
Master password  ──►  Argon2id KDF  ──►  Master Key
                           │                  │
                        [~2-4s]          [zeroed immediately]
                                              │
                                         3 derived keys
                                        /      |       \
                                   db_key   enc_key  search_key
                                      │        │
                                 SQLCipher  decrypt Vault Key
                                             │
                                        [Vault Key]  ◄─── lives in RAM + Keychain
                                             │              while vault is OPEN
                                        encrypt/decrypt     zeroed on lock()
                                          all records
```
