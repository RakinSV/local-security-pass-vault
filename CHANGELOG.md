# Changelog

All notable changes to VaultPass are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased] — public beta

### Added
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
