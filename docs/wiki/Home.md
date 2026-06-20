# LSPV Wiki

**Local Security Pass Vault** вЂ” open-source, offline-first password manager.  
Zero cloud. Zero telemetry. 8 layers of protection between your data and an attacker.

---

## Quick Links

| Guide | Description |
|-------|-------------|
| [Getting Started](Getting-Started.md) | Install, create vault, system tray, quick unlock |
| [Feature Guide](Feature-Guide.md) | All features: TOTP, custom fields, folders, trash, health report, CSV |
| [Browser Extension](Browser-Extension.md) | Chrome / Edge / Firefox auto-fill setup |
| [Backup & Recovery](Backup-and-Recovery.md) | BIP-39 mnemonic, export `.vbk`, restore, 3-2-1 strategy |
| [Security Architecture](Security-Architecture.md) | 8-layer crypto model, what is stored where, threat model |
| [Build from Source](Build-from-Source.md) | Prerequisites, compile, dev mode, CI |
| [FAQ](FAQ.md) | Common questions and troubleshooting |
| [Security Review Checklist](Security-Review-Checklist.md) | Code-verified audit of all security properties |

---

## What is LSPV?

A password manager that runs entirely on your machine. The vault is an encrypted SQLite database on your disk. The master password never leaves your RAM вЂ” it is used to derive keys via Argon2id and then immediately zeroed.

No account. No cloud. No subscription. No telemetry.

---

## Security At a Glance

```
Layer 1 вЂ” Key Derivation    Argon2id (256 MB RAM, 4 iter) в†’ 3 independent keys
Layer 2 вЂ” Vault Key Enc.    XChaCha20-Poly1305, unique nonce per save
Layer 3 вЂ” Record Encryption XChaCha20-Poly1305, unique nonce per record per save
Layer 4 вЂ” Database          SQLCipher AES-256 per page
Layer 5 вЂ” OS Keychain       DPAPI / libsecret / Keychain вЂ” quick unlock, deleted on lock
Layer 6 вЂ” Memory Safety     mlock() + zeroize, PR_SET_DUMPABLE=0, no swap
Layer 7 вЂ” Clipboard         30s TTL, CF_EXCLUDEFROMCLOUDCLIPBOARD
Layer 8 вЂ” Backup            BIP-39 + Argon2id 4 GB + XChaCha20-Poly1305 + BLAKE3
```

---

## 4 Security Levels

### Level 1 вЂ” Cryptographic Core

Every master password runs through **Argon2id** (m=256 MB, t=4, p=4) to produce three independent 32-byte keys: one for SQLCipher, one to decrypt the Vault Key, one for the search index HMAC.

The **Vault Key** is a random 256-bit key generated once at vault creation. It encrypts every individual record with **XChaCha20-Poly1305** (a fresh 192-bit random nonce per save). Changing your master password re-encrypts only the Vault Key вЂ” all records stay untouched.

Memory discipline: keys are `mlock()`-ed (OS cannot page them to disk) and zeroed via `sodium_memzero()` on every lock. All MAC comparisons use `sodium_memcmp()` (constant-time).

### Level 1 вЂ” Storage Hardening

Vault writes go through `tmp в†’ fsync в†’ rename()` вЂ” an atomic sequence that survives power loss. The file is never partially written.

Before opening, `lstat()` + `O_NOFOLLOW` reject symlinks. A honeypot file detects unauthorized external writes (ransomware indicator). `PR_SET_DUMPABLE=0` is called at startup on Linux вЂ” no core dumps, no ptrace attach from other processes.

### Level 2 вЂ” Browser Extension

The extension has `connect-src 'none'` in its CSP вЂ” it literally cannot make a network request. All data flows via a local named pipe (Native Messaging) to the desktop process, which holds the keys.

Every IPC response is signed with the desktop's Ed25519 private key. The extension verifies the signature before acting. Each request carries a UUID nonce вЂ” replay attacks are rejected.

Domain matching uses `tldts` for eTLD+1 comparison: `accounts.google.com` matches `google.com`, `google.com.evil.ru` does not.

### Level 3 вЂ” Encrypted Backups

A 24-word BIP-39 mnemonic (256 bits entropy) is the backup encryption key. LSPV shows it once and never stores it. The backup KDF is a hardened Argon2id profile: **4 GB RAM, 10 iterations** вЂ” deliberately brutal to make GPU brute-force of a stolen `.vbk` file economically impossible.

Auto-backup: each manual export also saves a timestamped copy to `app_data/backups/`. The 7 most recent copies are kept automatically.

---

## Threat Model (STRIDE/PASTA)

| Attack | Mitigation |
|--------|-----------|
| Stolen vault.db | SQLCipher + XChaCha20 вЂ” unreadable without password |
| RAM dump (locked) | zeroize + mlock вЂ” keys wiped on lock() |
| RAM dump (unlocked) | mlock prevents swap; per-record AEAD limits exposure |
| Brute-force password | Argon2id 256 MB в‰€ 2-4 sec/attempt on RTX 4090 |
| DLL/SO hijacking | libsodium statically linked |
| GPU VRAM residue | sodium_mlock + PR_SET_DUMPABLE=0 |
| Supply chain (XZ-style) | cargo audit in CI, minimal deps |
| Windows Cloud Clipboard | CF_EXCLUDEFROMCLOUDCLIPBOARD |
| Ransomware | Honeypot file triggers immediate lock |
| Symlink attack | lstat() + O_NOFOLLOW |
| Extension XSS | connect-src 'none' CSP |
| IPC replay | Ed25519 + UUID nonce per request |

Full threat model: [`docs/threat-model.md`](../threat-model.md)

---

## Support the Project

LSPV is maintained without funding. If it's useful, consider a Bitcoin donation:

```
bc1qwnkyez3nv86dry54dqfjjtav29qqq72h69pevw
```

A GitHub star also helps the project get discovered.
