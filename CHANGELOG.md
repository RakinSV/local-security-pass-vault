# Changelog

All notable changes to VaultPass are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [0.2.4] — 2026-06-23

This release closes every finding from three independent security audits and
graduates VaultPass from public beta to the first stable release.

### Security

- **TOTP fail-safe**: missing TOTP secret now returns `TwoFactorFailed` instead
  of silently bypassing 2FA — protects against data corruption scenarios where
  `totp_enabled=true` but the encrypted secret was never written
- **Windows named pipe DACL** (`win_dacl` module): pipe `\\.\pipe\vaultpass`
  now restricted via raw Windows security descriptors to the current user only —
  other local users on the same machine cannot connect
- **Ed25519 signing key in OS keychain**: private key moved from plaintext file
  to DPAPI (Windows) / libsecret (Linux) / Keychain (macOS) via `keyring = "2"`;
  file is kept as fallback with auto-migration on first launch
- **Path traversal guard** (`validate_user_path`): backup export and restore
  paths now reject `..` components and require absolute paths — prevents directory
  traversal on malicious input
- **Honeypot random size** (2–10 KiB): `vault_backup.db` decoy is now a random
  size on each creation, preventing ransomware from whitelisting a known fixed-size
  file
- **TOTP brute-force rate-limit**: per-connection counter increments on each
  wrong 6-digit code; connection dropped after 5 consecutive failures
- **HIBP button disclosure**: "Check breaches" button now shows a tooltip
  explaining that the first 5 hex characters of the SHA-1 hash are sent to
  `api.pwnedpasswords.com` (k-anonymity) — network activity is opt-in and visible
- **Backup directory permissions** (Linux/macOS): `~/.local/share/lspv/backups/`
  created with `0o700` so other local users cannot read backup files
- **Ed25519 TOFU comment**: grace-period window (first connection) documented
  in `native.ts` with a clear explanation of the trust boundary
- **CI reproducibility**: `--locked` flag added to `cargo audit`, `cargo test`,
  and `cargo clippy` — ensures CI uses the exact dependency versions in
  `Cargo.lock` and never silently upgrades
- **Conditional prerelease flag**: GitHub releases tagged `vX.Y.Z` are now
  published as stable; only tags containing `-alpha`, `-beta`, or `-rc` receive
  the prerelease marker
- **Dead identity code removed**: `chrome.identity.getProfileUserInfo()` call
  removed from `extension/src/background/profile.ts` — it always failed silently
  (no `"identity"` permission in manifest) and attempted to read the Google
  account email without user consent
- **Atomic profile registry writes**: `profiles.json` now written via
  `tmp → rename` to prevent partial-write corruption on crash

### Changed

- **Version sync**: all four components (`core-vault`, `desktop/src-tauri`,
  `native-host`, `extension`) now share a single version (`0.2.4`) — workspace
  root Cargo.toml is the single source of truth via `version.workspace = true`
- **Native host retry**: connect retry loop extended from 8 × 500 ms to
  20 × 500 ms (10 s total); constants `CONNECT_RETRIES` / `CONNECT_SLEEP_MS`
  extracted for easy future tuning
- **Auto-lock comment**: 30-second poll granularity is documented as intentional
  — actual lock happens 0–30 s after the timeout expires, which is by design

### Added (v0.2.4 hardening, already shipped in beta)

- **Windows process mitigations**: `SetDllDirectoryW("")` strips CWD from DLL
  search path; `ProcessDynamicCodePolicy` blocks shellcode injection;
  `ProcessSignaturePolicy` requires Microsoft-signed DLLs
- **Linux seccomp-BPF blacklist**: blocks `ptrace`, `process_vm_readv`,
  `process_vm_writev` with `KillThread`; default-Allow for GTK/WebKit syscalls
- **Screen capture protection**: `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`
  applied when a password is revealed; removed when hidden — excludes the window
  from OBS, screenshots, and Windows thumbnail cache
- **HIBP breach check**: k-anonymity SHA-1 prefix lookup against
  `api.pwnedpasswords.com` — password never leaves the device; opt-in per item
- **Vault 2FA unlock** (TOTP): Settings → Security enables a second factor;
  TOTP secret is encrypted with the vault key and included in `.vbk` backups
- **2FA setup QR code**: SVG generated offline for scan by any authenticator app
- **Folder rename**: inline edit with Enter/Esc keyboard confirmation
- **Drag-and-drop**: move items between folders via native HTML5 drag-and-drop
- **Bitwarden JSON import**: logins, secure notes, cards, identities
- **Themes**: Dark / Light / System mode with live switching
- **Accent colors**: 8 presets + custom color picker, persisted across sessions
- **Compile-time KDF guards**: `const` block asserts `KDF_OPSLIMIT >= 2` and
  `KDF_MEMLIMIT >= 64 MiB` — misconfigured parameters are a build error, not a
  runtime surprise
- **Panic hook**: `std::panic::set_hook` zeroes vault keys before crashing to
  prevent key material remaining in memory after an unexpected crash
- **Mutex deadlock safety**: `pipe_server` uses `try_lock()` with an immediate
  error instead of blocking forever on a poisoned mutex

---

## [0.2.3-beta] — 2026-05 (beta)

### Added

- **TOTP / 2FA for entries**: live 6-digit codes with countdown ring; QR scan
  from clipboard via `arboard` + `rqrr`
- **Browser Extension Installer**: auto-detect Chrome/Edge/Firefox/Brave;
  one-click native host registration (Windows registry / Linux JSON)
- **Custom fields**: hidden or visible custom fields on any Login entry
- **Password history**: auto-saved on password change; collapsed section in
  item detail view
- **Favorites**: star items; sidebar filter
- **Trash bin**: soft delete with restore and purge; "Empty trash" button
- **Folders**: create / rename / delete; sidebar filter; drag-and-drop
- **Password health report**: weak (< 12 chars or < 2 char classes), duplicate,
  and old (> 6 months) password detection
- **CSV export**: Chrome / Firefox-compatible format (name, url, username,
  password, note)
- **Linux CI**: AppImage build via GitHub Actions for every version tag

### Fixed

- **Panic hook**: vault key zeroed before crash via `std::panic::set_hook`
- **Pipe server mutex safety**: `try_lock()` replaces blocking `lock()` to
  avoid deadlock if a command handler panics while holding the state mutex
- **Content script timing**: switched to `document_start` with
  `DOMContentLoaded` defer — avoids race on single-page app navigations
- **`mlock` failure logging**: `mlock` errors now logged to stderr instead of
  silently ignored — visible in crash reports and `RUST_LOG=debug` output
- **`rename_folder` missing save**: `vault.save()` added after folder rename so
  changes persist to disk

### Security

- **TOTP brute-force** (H-5): connection closed after 5 consecutive wrong
  6-digit codes per pipe connection
- **IPC lock call** (H-1): `pipe_server` lock command calls `lock_vault_internal`
  which clears clipboard, marks vault read-only, removes keychain entry, and
  emits the `vault-locked` Tauri event
- **Extension permissions reduced** (H-2): `tabs`, `storage`, `identity`
  removed from manifest — only `activeTab`, `nativeMessaging`, `clipboardWrite`
- **Content script isolation** (H-3): `onMessage` rejects any message where
  `sender.id !== chrome.runtime.id`

---

## [0.2.2-beta] — 2026-04 (beta)

### Added

- **Full backup UI**: Settings → Backup — 4×6 word grid display, 3-word
  confirmation step, `.vbk` export and restore
- **Auto-backup rotation**: 7 most recent timestamped copies kept automatically
  at `%APPDATA%\lspv\backups\` (Windows) or `~/.local/share/lspv/backups/` (Linux)
- **Auto-lock timer**: configurable idle timeout (1 / 5 / 15 / 30 / 60 min /
  Never); 30-second poll granularity
- **Lock on minimize** (optional)
- **OS Keychain status UI**: Settings → Security shows cache status and a
  "Remove" button to evict the cached key
- **`PR_SET_DUMPABLE=0`** called at startup on Linux — prevents ptrace attach
  and core dumps
- **SRI integrity check**: `scripts/sri-check.js` verifies SHA-256 hashes of
  built extension JS in CI

### Security

- **IPC Unix socket restricted** (C-2): `/tmp/vaultpass.sock` set to 0600
  immediately after bind
- **Ed25519 TOFU** (C-1): extension pins the desktop public key on the first
  `status` response; all subsequent signed responses are verified via
  `crypto.subtle.verify('Ed25519', ...)`
- **Signing key file permissions** (H-3): `signing.sk` / `signing.pk` written
  with 0600 on Unix
- **`vault.meta` permissions** (H-4): restricted to 0600 after creation
- **Profile input validation** (M-2): `profile_id` validated (ASCII alnum +
  `-_`, ≤ 128 chars); `email` capped at 254 chars; `browser_type` at 16 chars
- **Atomic tmp permissions** (M-3): `atomic_write` sets 0600 on `.tmp` file
  before writing — zero world-readable window
- **Opaque IPC errors** (L-1): internal `VaultError` messages no longer
  forwarded over IPC — replaced with an opaque `"InternalError"` string
- **Compile-time KDF guards**: `const` assert ensures `KDF_OPSLIMIT >= 2` and
  `KDF_MEMLIMIT >= 64 MiB`

---

## [0.2.0-beta] — 2026-03 (beta)

Initial public beta. Core vault, browser extension, and CI established.

### Added

- **Desktop app** (Tauri 2, Windows + Linux): vault create / open / lock; item
  CRUD for Login, Card, Note, Identity, SSH Key, Server
- **Argon2id KDF** (256 MB RAM, 4 iterations) — RTX 4090 ≈ 2–4 s per attempt
- **XChaCha20-Poly1305 AEAD** per-item encryption with unique 192-bit nonce per
  save
- **SQLCipher** AES-256 page-level database encryption
- **Envelope encryption**: `db_key ‖ enc_key ‖ search_key` (96 bytes) derived
  from Argon2id; vault key re-encrypted on master password change
- **HMAC-SHA256 search index** for encrypted title search without decrypting
  every record
- **BIP-39 backup** (24 words, 256-bit entropy) + `.vbk` format
  (XChaCha20-Poly1305 + BLAKE3 checksum)
- **Browser extension** (MV3) for Chrome, Firefox 109+, Edge
- **Native Messaging IPC**: Windows named pipe + Linux Unix socket with
  Ed25519-signed responses
- **Shadow-DOM auto-fill prompt** on login pages; auto-dismisses after 20 s
- **eTLD+1 domain matching** via `tldts` — subdomain matching, homograph
  rejection
- **Clipboard TTL**: 30-second auto-clear + `CF_EXCLUDEFROMCLOUDCLIPBOARD` on
  Windows
- **OS Keychain quick-unlock**: DPAPI (Windows) / libsecret (Linux) / Keychain
  (macOS) via `keyring = "2"`
- **Honeypot file**: `vault_backup.db` with random content; BLAKE3 hash checked
  on every unlock — unauthorized modification triggers ransomware alert
- **System tray**: minimize to tray, lock / quit from context menu
- **Autostart** (optional): launch at system login
- **Bitwarden JSON import** + **CSV import** (Chrome / Firefox format)
- **CSV export** (Chrome / Firefox-compatible)
- **Password generator**: 8–64 chars, uppercase / lowercase / digits / symbols
  toggles, entropy meter
- **Password health report**: weak / duplicate / old password detection
- **TOTP entry codes**: live 6-digit codes with countdown ring; QR scan from
  clipboard
- **Custom fields** on Login entries (hidden or visible)
- **Password history**: auto-saved on password change
- **Favorites** + sidebar filter
- **Trash bin**: soft delete with restore / purge
- **Folders**: create / rename / delete; sidebar filter; drag-and-drop
- **Themes**: Dark / Light / System mode
- **Accent colors**: 8 presets + custom color picker
- **CI** (`security.yml`): `cargo audit`, `cargo test` (Ubuntu + Windows),
  `cargo clippy`, `tsc`, extension build, SRI check, weekly fuzzing
- **54 Rust tests** including RFC 9106 Argon2id vectors and HMAC-SHA256 RFC 4231
  vectors; **7 TypeScript tests** (eTLD+1 domain matching)

---

[0.2.4]: https://github.com/RakinSV/local-security-pass-vault/releases/tag/v0.2.4
[0.2.3-beta]: https://github.com/RakinSV/local-security-pass-vault/releases/tag/v0.2.3-beta
[0.2.2-beta]: https://github.com/RakinSV/local-security-pass-vault/releases/tag/v0.2.2-beta
[0.2.0-beta]: https://github.com/RakinSV/local-security-pass-vault/releases/tag/v0.2.0-beta
