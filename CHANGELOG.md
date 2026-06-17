# Changelog

All notable changes to VaultPass are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased] — public beta

### Security
- **C-2 FIXED**: Unix socket `/tmp/vaultpass.sock` restricted to 0600 immediately after bind — other local users can no longer connect
- **C-1 FIXED**: Ed25519 TOFU verification in extension — desktop public key stored on first `status` response; all subsequent signed responses verified via `crypto.subtle.verify('Ed25519', ...)`; falls back gracefully on Firefox < 130 / Chrome < 113
- **H-3 FIXED**: `signing.sk` and `signing.pk` written with 0600 permissions on Unix
- **H-4 FIXED**: `vault.meta` restricted to 0600 after creation
- **H-5 FIXED**: Content script `onMessage` rejects messages where `sender.id !== chrome.runtime.id`
- **M-2 FIXED**: `profile_id` validated (ASCII alnum + `-_`, ≤ 128 chars); `email` capped at 254 chars; `browser_type` capped at 16 chars
- **M-3 FIXED**: `atomic_write` sets 0600 on `.tmp` file before writing any data — zero world-readable window
- **M-4 FIXED**: `search` handler calls `vault.get_item()` once per item (was called twice, doubling decryption work)
- **L-1 FIXED**: Internal Rust `VaultError` messages no longer forwarded over IPC — replaced with opaque `"InternalError"` string

### Added
- **Build CI** (`build.yml`) — produces Windows NSIS installer and Linux x64 binary on `workflow_dispatch` / version tags; artifacts retained 30 days for testing

### Added (original)
- **Desktop app** (Tauri 2, Windows + Linux) — vault create/open/lock, item CRUD for Login, Card, Note, Identity, SSH Key
- **Browser extension** (MV3) for Chrome, Firefox 109+, Edge
- **Native Messaging IPC** — named pipe (Windows) / Unix socket (Linux) with Ed25519-signed responses
- **Shadow-DOM auto-fill prompt** — appears on login pages without user action; auto-dismisses after 20 s
- **Chrome profile identification** — per-profile UUID + Google account email via `chrome.identity`
- **Firefox support** — fixed extension ID `vaultpass@vaultpass.app` for stable native messaging
- **Browser type badges** — Settings shows Chrome / Firefox / Edge for each connected profile
- **In-app native host registration** — Settings → Browser → Apply & Register writes Windows registry (no PowerShell needed)
- **CSV import** — auto-detects Chrome and Firefox export formats; strips BOM
- **Profile registry** — connected browser profiles stored in `profiles.json`; user-renameable
- **Master password change** without re-encrypting vault items (Envelope Encryption)
- **Honeypot file** for ransomware detection
- **Argon2id KDF** (256 MB RAM · 4 iterations) + **SQLCipher** AES-256 database layer
- **XChaCha20-Poly1305** per-item AEAD with unique 192-bit nonce per save
- **HMAC-SHA256 search index** for encrypted title search
- **Clipboard auto-clear** (30 s TTL + `CF_EXCLUDEFROMCLOUDCLIPBOARD` on Windows)
- CI: cargo test · cargo audit · cargo clippy · fuzz (weekly) · TypeScript type check
- **37 unit/integration tests** including RFC 9106 Argon2id test vectors

---

*Pre-release — no versioned releases yet. Track progress in [Issues](https://github.com/Dex-vabster/local-security-pass-vault/issues).*
