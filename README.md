# Local Security Pass Vault (LSPV)

> **An open source, offline password manager that never phones home.**
> Your vault lives on your disk, encrypted with battle-tested algorithms, audited libraries, and 8 layers of protection.

[![Build](https://img.shields.io/github/actions/workflow/status/RakinSV/local-security-pass-vault/ci.yml?branch=main&label=build)](https://github.com/RakinSV/local-security-pass-vault/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/RakinSV/local-security-pass-vault?include_prereleases&label=release)](https://github.com/RakinSV/local-security-pass-vault/releases)
[![Crypto: Argon2id + XChaCha20](https://img.shields.io/badge/crypto-Argon2id%20%2B%20XChaCha20--Poly1305-brightgreen)](docs/adr/)
[![Platform: Windows Linux](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/RakinSV/local-security-pass-vault/releases)

---

## Why another password manager?

Cloud password managers have breached millions of accounts. LastPass leaked encrypted vaults in 2022. No subscription service can guarantee your passwords stay private — the moment data hits their server, you lose control.

LSPV takes a different approach: **the vault never leaves your machine.** No cloud sync, no telemetry, no analytics, no update pings. Zero outbound network traffic from the desktop app. Passwords are encrypted before they touch the database, and the keys exist only in RAM while the vault is unlocked.

---

## Security Architecture

```
╔══════════════════════════════════════════════════════════════════════════╗
║  🔐  LSPV — 8 layers of protection between your data and an attacker    ║
╠═══╦══════════════════════════════╦═══════════════════════════════════════╣
║ 1 ║  Master Password KDF         ║  Argon2id · 256 MB RAM · 4 iterations║
║ 2 ║  Vault Key Encryption        ║  XChaCha20-Poly1305 · unique nonce    ║
║ 3 ║  Database Encryption         ║  SQLCipher · AES-256 · per-page       ║
║ 4 ║  OS Keychain (quick unlock)  ║  DPAPI / macOS Keychain / libsecret   ║
║ 5 ║  Memory Safety               ║  mlock() · zeroize on drop            ║
║ 6 ║  Encrypted Backups           ║  BIP-39 · BLAKE3 checksum · XChaCha20 ║
║ 7 ║  Browser IPC Trust           ║  Ed25519 TOFU signatures              ║
║ 8 ║  Ransomware Detection        ║  Honeypot file + hash verification     ║
╚═══╩══════════════════════════════╩═══════════════════════════════════════╝
```

**Envelope encryption:** your master password runs through Argon2id (256 MB RAM, 4 iterations — deliberately slow) to produce a key that decrypts the Vault Key. The Vault Key decrypts individual records. Changing your master password re-encrypts only the Vault Key; all records stay untouched.

**Atomic writes:** vault saves go through temp file → `fsync` → atomic rename. A crash mid-write leaves the previous vault intact.

---

## Features

- **Zero cloud, zero telemetry** — no network socket ever opens from the desktop app
- **Offline-first** — works without internet, always
- **Multi-vault** — separate encrypted databases for work, personal, family
- **Browser extension** — auto-fill for Chrome, Edge, and Firefox via native messaging
- **Auto-fill** — domain-matched suggestions using eTLD+1 comparison (no subdomain confusion)
- **6 item types** — Login, Card, Note, Identity, SSH Key, Server (with SSH/password/token auth)
- **Source tagging** — import label tracks which browser profile or export a record came from; filterable in sidebar
- **CSV import** — from Chrome and Firefox password exports
- **BIP-39 backup** — 24-word mnemonic encrypts a portable `.vbk` backup file
- **OS Keychain** — quick unlock using DPAPI / Keychain / libsecret after first password entry
- **System tray** — close to tray, Lock & Hide, left-click to toggle window
- **Autostart** — optional launch at system login
- **Ed25519 IPC signing** — browser extension verifies every native message (TOFU model)
- **Memory protection** — keys are `mlock()`-ed and zeroed before deallocation

---

## LSPV vs Cloud Password Managers

| Feature | **LSPV** | Bitwarden | 1Password | LastPass |
|---------|:--------:|:---------:|:---------:|:--------:|
| Zero cloud storage | ✅ | ❌ | ❌ | ❌ |
| Zero telemetry | ✅ | ❌ | ❌ | ❌ |
| Works offline | ✅ | Limited | ❌ | ❌ |
| Free forever | ✅ | Free tier | $3/mo | $3/mo |
| Fully open source | ✅ | Partial | ❌ | ❌ |
| No account required | ✅ | ❌ | ❌ | ❌ |
| Argon2id KDF | ✅ | ✅ | ❌ | ❌ |
| Self-hosted option | ✅ | Yes (complex) | ❌ | ❌ |
| Team sharing | ❌ | ✅ | ✅ | ✅ |
| Mobile app | ❌ (roadmap) | ✅ | ✅ | ✅ |

---

## Quick Start

### Windows

1. Download `lspv-setup-x64.exe` from [Releases](https://github.com/RakinSV/local-security-pass-vault/releases)
2. Run the installer — no admin rights required (per-user install)
3. Launch **Local Security Pass Vault** from the Start menu
4. Create your first vault and set a strong master password

### Linux

```bash
# AppImage — no install required
chmod +x lspv-x86_64.AppImage
./lspv-x86_64.AppImage
```

---

## Browser Extension

LSPV communicates with Chrome/Firefox via the [Native Messaging API](https://developer.chrome.com/docs/apps/nativeMessaging/) — no WebSocket, no cloud relay, just a local named pipe.

### Chrome / Edge

1. In LSPV: **Settings → Browser → Chrome / Edge** — paste your extension ID from `chrome://extensions`
2. Click **Apply & Register** — writes the native messaging manifest to the registry
3. Load the extension: `chrome://extensions` → Developer mode → **Load unpacked** → `extension/dist/`
4. Pin the LSPV icon to your toolbar

### Firefox

1. In LSPV: **Settings → Browser → Firefox** → click **Add** (ID prefills as `lspv@lspv.app`)
2. Click **Apply & Register**
3. `about:debugging` → This Firefox → **Load Temporary Add-on** → `extension/dist/manifest.json`

### How auto-fill works

- Popup shows items whose eTLD+1 domain matches the current tab
- **Fill** injects credentials via native setter (not tracked by browser history)
- Content script detects login forms and shows an inline prompt when LSPV has a matching entry
- Every IPC response is Ed25519-signed; the extension verifies before acting on it

---

## Multi-Vault

LSPV supports multiple independent encrypted databases. Each vault has its own master password, Vault Key, BIP-39 mnemonic, and OS Keychain entry.

Create vaults via **+ New Vault** on the vault picker. Switch by locking the current vault and opening another. Vaults can live on a local drive, external disk, or network share — any path that your OS can open as a file.

---

## Backup & Recovery

LSPV uses a 24-word BIP-39 mnemonic as the backup encryption key. The mnemonic is shown **once** at vault creation — write it on paper and store it somewhere safe. LSPV never saves it to disk.

The `.vbk` backup format:
- Key derivation: Argon2id with 4 GB RAM, 10 iterations (extremely slow — brute force infeasible)
- Encryption: XChaCha20-Poly1305
- Integrity: BLAKE3 checksum inside the ciphertext

---

## Build from Source

### Prerequisites

- [Rust 1.78+](https://rustup.rs/) + [Node.js 20+](https://nodejs.org/)
- **Windows:** Microsoft C++ Build Tools + vcpkg for libsodium
- **Linux:** `apt install libsodium-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev`

### Build

```bash
git clone https://github.com/RakinSV/local-security-pass-vault.git
cd local-security-pass-vault

# Core library + tests
cargo build && cargo test

# Desktop app
cd desktop && npm install && npm run tauri build

# Browser extension
cd ../extension && npm install && npm run build
```

### Development

```bash
cd desktop && npm run tauri dev
```

### Security audit

```bash
cargo audit      # CVE scan
cargo clippy     # unwrap_used / expect_used are flagged as warnings
```

---

## Project Structure

```
local-security-pass-vault/
├── core-vault/          # Rust crate — crypto engine, data model, SQLCipher
│   └── src/
│       ├── crypto/      # Argon2id KDF, XChaCha20-Poly1305, HMAC search index
│       ├── backup/      # BIP-39 + BLAKE3 backup (.vbk format)
│       └── models.rs    # Item types, vault schema
├── desktop/             # Tauri 2 desktop app (Windows + Linux)
│   ├── src/             # React + Tailwind frontend
│   └── src-tauri/       # Rust — IPC commands, OS Keychain, tray, browser integration
├── extension/           # Browser extension (Chrome + Firefox, Manifest V3)
│   ├── src/
│   │   ├── background/  # Native messaging bridge + Ed25519 signature verification
│   │   ├── content/     # Auto-fill content script
│   │   └── popup/       # React popup
│   └── public/          # manifest.json, icons
├── docs/
│   ├── adr/             # Architecture Decision Records
│   └── threat-model.md  # Threat model
└── CLAUDE.md            # AI assistant context (development rules + constraints)
```

---

## Architecture Decisions

Key design choices are in [`docs/adr/`](docs/adr/):

- **ADR-001** — Why libsodium over ring / RustCrypto
- **ADR-002** — Envelope encryption and Vault Key design
- **ADR-003** — BIP-39 backup format with BLAKE3 integrity
- **ADR-004** — Named pipe IPC over localhost HTTP

---

## Contributing

PRs welcome. A few hard rules:

1. **Crypto code** — read `.claude/rules/crypto.md` before touching `core-vault/src/crypto/`. These rules exist because past violations are what cause breaches in other managers.
2. **No new crypto deps** — libsodium only. No openssl, ring, or argon2 crates without a security review.
3. **Tests** — any crypto change requires official test vectors (RFC 9106 for Argon2id, libsodium suite for XChaCha20).
4. **No telemetry** — any PR that adds outbound network calls will be closed.
5. Run `cargo audit` before opening a PR.

---

## Roadmap

- [ ] Backup export/import UI in Settings
- [ ] Auto-lock timer (configurable idle timeout)
- [ ] TOTP generator (in-app 2FA codes from stored secrets)
- [ ] Password health report (duplicates, weak, breached — local check only)
- [ ] LAN sync between devices (no cloud involved)
- [ ] Mobile companion app

---

## License

MIT — see [LICENSE](LICENSE).

---

*Keywords: local password manager · offline password manager · open source password manager · zero knowledge password manager · Rust password manager · self-hosted password manager · no cloud password manager · Tauri password manager · libsodium · SQLCipher*
