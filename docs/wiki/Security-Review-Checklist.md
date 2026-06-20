# LSPV — Security Architecture Review Checklist

> Status legend:
> - ✅ **Documented / Designed** — spec exists, implementation path is clear
> - ⚠️ **Partial / Needs clarification** — mentioned in rules, concrete design missing
> - ❌ **Not touched** — not designed yet, required for production

---

## Level 1 — Cryptography Core

| Status | Component | Notes |
|--------|-----------|-------|
| ✅ | **Argon2id KDF** | m=256 MB, t=4, p=4. Inputs: master-password + 32B random salt. Outputs: `db_key` + `enc_key` + `search_key` (domain separation). Spec in `crypto.md`. |
| ✅ | **XChaCha20-Poly1305 AEAD** | Envelope encryption: Master Key encrypts Vault Key; Vault Key encrypts every record. Nonce via `randombytes_buf()` only — never counters. |
| ✅ | **Constant-time operations** | `sodium_memcmp()` for all MAC/hash comparisons. Rule enforced in `crypto.md`. |
| ✅ | **Memory safety** | `sodium_mlock()` — keys cannot be swapped to disk. `sodium_memzero()` for cleanup (compiler-resistant, unlike `memset`). |
| ⚠️ | **OS Keychain / Secure Enclave** | Critical path. Mentioned in architecture diagram but not detailed in docs. Need explicit spec per platform: `keyring`/libsecret on Linux, Credential Manager (DPAPI) on Windows, Security.framework on macOS. |
| ⚠️ | **Auto-lock + zeroize** | Requirement stated (`sodium_memzero` on timeout). Concrete Tauri timer mechanism (`tauri-plugin-os` idle events + `Mutex<Option<ZeroizeVec>>`) not fully designed. |

---

## Level 1 — Storage

| Status | Component | Notes |
|--------|-----------|-------|
| ✅ | **SQLCipher (AES-256 page-level)** | `rusqlite` + `sqlcipher` feature. Tables: `vault`, `items`, `folders`, `audit_log`, `sync_meta`. DB key = `db_key` from Argon2id — never stored on disk. |
| ✅ | **Atomic write** | `.tmp` → `fsync()` → `rename()`. Never write directly to `vault.db`. Protection against corruption on crash. |
| ✅ | **Filesystem hardening** | `O_NOFOLLOW` (symlink attack), readonly flags when app is closed, honeypot file (`vault_backup.db`) for ransomware detection. |
| ✅ | **Process-level protection** | `PR_SET_DUMPABLE=0` on Linux (no core dumps, no ptrace). `SetProcessMitigationPolicy` on Windows. Guards against process memory dump attacks. |

---

## Level 2 — Browser Extension

| Status | Component | Notes |
|--------|-----------|-------|
| ✅ | **MV3 manifest, zero network requests** | Extension never stores passwords. `connect-src 'none'` in CSP. All data flows exclusively via IPC with the Tauri process. |
| ✅ | **Ed25519 mutual authentication for IPC** | Mutual auth between extension and desktop process. Protects against IPC pipe squatting. Each message carries a unique nonce (replay protection). |
| ✅ | **eTLD+1 domain matching** | Correct domain comparison via `tldts` (publicsuffix.org). Prevents subdomain spoofing (`google.com.evil.ru` does not match `google.com`). |
| ✅ | **Extension threat model** | Covered: XSS injection, browser history leaks, screenshot caching, Accessibility API leaks, DNS side channels. |
| ⚠️ | **CSP + Subresource Integrity** | CSP headers and SRI hashes mentioned as requirements in `browser-extension.md`. Concrete `manifest.json` CSP string and hash-verification CI step not written. |

---

## Level 3 — Backups

| Status | Component | Notes |
|--------|-----------|-------|
| ⚠️ | **BIP-39 mnemonic as backup key** | Described in `backup.md` (24 words, 256-bit entropy). As an end-to-end implemented mechanism (UI flow, derive key from seed, show-once screen) — not yet built. |
| ❌ | **Argon2id enhanced profile for backups** | Separate KDF profile (4 GB RAM, 10 iterations) required for offline backup files. Not implemented. Must be distinct from the vault-unlock profile (256 MB) to make brute-force infeasible without the 24 words. |
| ❌ | **age + .vbk backup format** | Backup file format not finalized. Spec: `age`-encrypted container with magic header `VAULTPASS_BACKUP_V1`, vault UUID, BLAKE3 checksum, raw `vault.db` bytes. Not implemented. |
| ❌ | **BLAKE3 checksum + backup rotation** | 3-2-1 strategy (local + USB + remote offline), 7 daily / 4 weekly retention, BLAKE3 integrity check on restore — not designed or implemented. |

---

## Threat Model (STRIDE / PASTA)

| Status | Vector | Mitigation |
|--------|--------|-----------|
| ✅ | **DLL/SO hijacking** | `libsodium` statically linked. Zero runtime DLL dependencies. |
| ✅ | **GPU VRAM residue (LeftoverLocals), crash dumps** | `sodium_mlock()` prevents swap; `PR_SET_DUMPABLE=0` blocks dumps. Partial — WebGL disabled in WebView recommended. |
| ✅ | **Supply chain (XZ-utils style), WebView CVEs** | `cargo-audit` in CI, dependency minimization, `cargo-vet` planned. |
| ✅ | **Windows Cloud Clipboard, evil maid, rubber hose** | 30s clipboard TTL, `CF_EXCLUDEFROMCLOUDCLIPBOARD`, honeypot file, all documented in `threat-model.md`. |

---

## Open Items — Priority Order

Items requiring design/implementation before v1.0 stable release:

1. **OS Keychain full spec** — per-platform implementation details; quick-unlock flow (biometric after first password entry)
2. **Auto-lock Tauri mechanism** — `tauri-plugin-os` idle detection; `ZeroizeVec` in `Mutex<Option<...>>`; on-sleep hook
3. **Backup pipeline** — age format + 4 GB Argon2id profile + BLAKE3 + rotation policy + UI (show mnemonic once, verify 3 words)
4. **CSP + SRI in CI** — hash all extension `.js` files in build step; fail CI on mismatch
