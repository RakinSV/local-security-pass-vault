<div align="center">

```
  ╔══════════════════════════════════════════════════════════╗
  ║                                                          ║
  ║   ██╗     ███████╗██████╗ ██╗   ██╗                     ║
  ║   ██║     ██╔════╝██╔══██╗██║   ██║                     ║
  ║   ██║     ███████╗██████╔╝██║   ██║                     ║
  ║   ██║     ╚════██║██╔═══╝ ╚██╗ ██╔╝                     ║
  ║   ███████╗███████║██║      ╚████╔╝                      ║
  ║   ╚══════╝╚══════╝╚═╝       ╚═══╝                       ║
  ║                                                          ║
  ║        Local Security Pass Vault                         ║
  ║                                                          ║
  ║   ══ Zero Cloud · Zero Telemetry · 8 Layers Deep ══     ║
  ║                                                          ║
  ╚══════════════════════════════════════════════════════════╝
```

[![Build](https://img.shields.io/github/actions/workflow/status/RakinSV/local-security-pass-vault/security.yml?branch=main&label=CI&style=flat-square)](https://github.com/RakinSV/local-security-pass-vault/actions/workflows/security.yml)
[![Release](https://img.shields.io/github/v/release/RakinSV/local-security-pass-vault?include_prereleases&style=flat-square&label=release)](https://github.com/RakinSV/local-security-pass-vault/releases)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg?style=flat-square)](LICENSE)
[![Crypto: Argon2id + XChaCha20](https://img.shields.io/badge/crypto-Argon2id_%2B_XChaCha20--Poly1305-brightgreen?style=flat-square)](docs/adr/)
[![Platform](https://img.shields.io/badge/platform-Windows_%7C_Linux_%7C_macOS-lightgrey?style=flat-square)](https://github.com/RakinSV/local-security-pass-vault/releases)
[![cargo audit](https://img.shields.io/badge/cargo_audit-passing-success?style=flat-square)](https://github.com/RakinSV/local-security-pass-vault/actions)

**An open-source, offline-first password manager built in Rust.**  
Your vault never leaves your machine. No subscriptions, no accounts, no cloud, no tracking.

</div>

---

## Why LSPV?

Cloud password managers have a structural problem: they hold your passwords. LastPass was breached in 2022. 1Password, Bitwarden — all viable targets because they centralize what attackers want most.

LSPV takes the opposite approach. The vault never leaves your disk. There is no server to breach. The app makes zero outbound network connections. Your master password runs through Argon2id (256 MB RAM, 4 iterations) so fast-as-GPU brute force attacks are economically infeasible.

---

## Security Architecture — 4 Levels

LSPV is built in four independent security layers. Each layer is designed so that a failure in one does not compromise the others.

### Level 1 — Cryptographic Core

```
┌─────────────────────────────────────────────────────────────────┐
│  ARGON2ID KEY DERIVATION FUNCTION                               │
│                                                                 │
│  Input:  Master Password  +  32-byte random Salt                │
│  Params: m=256 MB · t=4 iterations · p=4 threads               │
│                                                                 │
│  Output:  ┌─────────────┬─────────────┬──────────────┐         │
│           │  db_key     │  enc_key    │  search_key  │         │
│           │  (32 bytes) │  (32 bytes) │  (32 bytes)  │         │
│           │  SQLCipher  │  Vault Key  │  HMAC index  │         │
│           └─────────────┴─────────────┴──────────────┘         │
│                                                                 │
│  RTX 4090 brute-force cost: ~2-4 seconds per attempt           │
└─────────────────────────────────────────────────────────────────┘

  Envelope Encryption (XChaCha20-Poly1305):
    enc_key  ──► decrypt Vault Key (stored in DB as ciphertext)
    Vault Key ──► encrypt/decrypt every individual record
    Unique 192-bit nonce per record save — never reused

  ✅ Constant-time operations  — sodium_memcmp() for MAC comparison
  ✅ Memory safety             — sodium_mlock() + sodium_memzero() on lock
  ✅ OS Keychain quick unlock  — DPAPI / libsecret / Keychain after first unlock
  ✅ Auto-lock                 — configurable idle timer + lock-on-minimize
```

### Level 1 — Storage Hardening

```
┌─────────────────────────────────────────────────────────────────┐
│  DATABASE: SQLCipher (AES-256 per page)                         │
│    vault.db stolen → reads as random noise without db_key       │
│                                                                 │
│  ATOMIC WRITES: tmp → fsync → rename()                         │
│    Crash mid-write leaves previous vault intact. Always.        │
│                                                                 │
│  FILESYSTEM HARDENING                                           │
│    O_NOFOLLOW — refuses to open symlinks (symlink attack block) │
│    readonly flag when vault is closed                           │
│    honeypot file — unauthorized writes trigger ransomware alert │
│                                                                 │
│  PROCESS-LEVEL PROTECTION                                       │
│    PR_SET_DUMPABLE=0 on Linux — no core dumps, no ptrace attach │
│    libsodium statically linked — zero DLL hijacking surface     │
└─────────────────────────────────────────────────────────────────┘
```

### Level 2 — Browser Extension

```
┌─────────────────────────────────────────────────────────────────┐
│  MV3 MANIFEST — ZERO NETWORK REQUESTS                           │
│    connect-src 'none'  — extension cannot phone home            │
│    frame-src 'none'    — no iframes                             │
│    worker-src 'none'   — no background Workers                  │
│    All data goes via IPC pipe to the desktop process only       │
│                                                                 │
│  ED25519 MUTUAL AUTHENTICATION                                  │
│    Every IPC response signed by desktop private key             │
│    Extension verifies signature before acting on data           │
│    Unique nonce per request — replay attacks impossible         │
│                                                                 │
│  DOMAIN MATCHING — eTLD+1 (tldts library)                      │
│    accounts.google.com ─ match ──► google.com vault entry       │
│    google.com.evil.ru  ─ NO  ──► rejected                      │
│    Stops subdomain spoofing at the algorithm level              │
│                                                                 │
│  THREAT MODEL COVERED: XSS injection · browser history leaks   │
│    screenshot caching · Accessibility API leaks · DNS channels  │
└─────────────────────────────────────────────────────────────────┘
```

### Level 3 — Encrypted Backups

```
┌─────────────────────────────────────────────────────────────────┐
│  24-WORD BIP-39 MNEMONIC                                        │
│    256 bits of entropy · standard English wordlist              │
│    Shown ONCE — LSPV never stores it on disk                    │
│    Write it on paper. Store the paper, not a photo.             │
│                                                                 │
│  BACKUP KDF — HARDENED ARGON2ID PROFILE                        │
│    m = 4 GB RAM · t = 10 iterations · p = 4 threads            │
│    RTX 4090 with 24 GB VRAM → max 6 parallel attempts          │
│    One attempt ≈ 30-60 seconds on high-end GPU                  │
│    2048²⁴ combinations → brute force is physically impossible   │
│                                                                 │
│  .VBK FILE FORMAT: XChaCha20-Poly1305 + BLAKE3 checksum        │
│    Integrity verified on restore — tampered file = hard reject  │
│                                                                 │
│  AUTO-ROTATION: 7 most recent copies kept in app_data/backups/ │
└─────────────────────────────────────────────────────────────────┘
```

### Threat Model (STRIDE/PASTA)

| Attack Vector | LSPV Mitigation |
|---------------|----------------|
| **Stolen vault.db** | SQLCipher AES-256 + XChaCha20-Poly1305 — unreadable blob |
| **RAM dump while locked** | `zeroize` + `mlock` — keys zero-wiped on every lock() |
| **RAM dump while unlocked** | `mlock` prevents swap; AEAD per-record limits blast radius |
| **Brute force master password** | Argon2id 256 MB ≈ 2-4s/attempt even on RTX 4090 |
| **DLL/SO hijacking** | libsodium statically linked — no external DLL surface |
| **GPU VRAM residue (LeftoverLocals)** | `sodium_mlock` + `PR_SET_DUMPABLE=0` documented mitigation |
| **Supply chain (XZ-utils style)** | `cargo audit` in CI · `cargo vet` planned · minimal deps |
| **Windows Cloud Clipboard** | `CF_EXCLUDEFROMCLOUDCLIPBOARD` — not synced to MS cloud |
| **Ransomware** | Honeypot file — unauthorized writes trigger vault lock |
| **Symlink attack on vault.db** | `lstat()` + `O_NOFOLLOW` — symlinks refused at open |
| **Browser extension XSS** | `connect-src 'none'` CSP — extension cannot reach internet |
| **IPC pipe squatting** | Ed25519 TOFU mutual auth — unsigned responses rejected |
| **Clipboard sniffing** | 30-second TTL + cloud sync excluded |

Full threat model: [`docs/threat-model.md`](docs/threat-model.md)

---

## Features

- **Zero cloud, zero telemetry** — no network socket ever opens from the desktop app
- **Offline-first** — works permanently without internet
- **Multi-vault** — separate encrypted databases for work / personal / family
- **6 item types** — Login · Card · Note · Identity · SSH Key · Server
- **Browser extension** — Chrome, Edge, Firefox via local native messaging (no WebSocket)
- **Auto-fill** — eTLD+1 domain matching, fills via native setter (not tracked by browser)
- **Source tagging** — import label tracks browser profile or CSV origin; filterable in sidebar
- **CSV import** — from Chrome and Firefox password exports
- **BIP-39 backup UI** — full Settings → Backup tab: 4×6 word grid, 3-word verification, export `.vbk`
- **Auto-backup rotation** — 7 most recent copies auto-saved on each export
- **OS Keychain** — quick unlock using Windows DPAPI / macOS Keychain / libsecret; removable from Settings
- **System tray** — Lock & Hide on minimize, left-click to toggle window
- **Autostart** — optional launch at system login
- **Ed25519 IPC signing** — extension verifies every native message
- **Memory protection** — keys `mlock()`-ed and zeroed before deallocation
- **Process hardening** — `PR_SET_DUMPABLE=0` on Linux at startup

---

## LSPV vs Cloud Password Managers

| Feature | **LSPV** | Bitwarden | 1Password | LastPass |
|---------|:--------:|:---------:|:---------:|:--------:|
| Zero cloud storage | ✅ | ❌ | ❌ | ❌ |
| Zero telemetry | ✅ | ❌ | ❌ | ❌ |
| Works offline, forever | ✅ | Limited | ❌ | ❌ |
| Free forever | ✅ | Free tier | $3/mo | $3/mo |
| Fully open source | ✅ | Partial | ❌ | ❌ |
| No account required | ✅ | ❌ | ❌ | ❌ |
| Argon2id KDF | ✅ | ✅ | ❌ | ❌ |
| Static crypto library | ✅ | ❌ | ❌ | ❌ |
| Browser extension | ✅ | ✅ | ✅ | ✅ |
| Auto-fill | ✅ | ✅ | ✅ | ✅ |
| BIP-39 backup | ✅ | ❌ | ❌ | ❌ |
| Mobile app | 🔜 v0.4 | ✅ | ✅ | ✅ |
| Hardware key (ESP32) | 🔜 v0.5 | ❌ | ❌ | ❌ |

---

## Quick Start

### Windows

1. Download `lspv-setup-x64.exe` from [Releases](https://github.com/RakinSV/local-security-pass-vault/releases)
2. Run the installer — no admin rights required (per-user NSIS install)
3. Launch **Local Security Pass Vault** from the Start menu
4. Click **+ New Vault** and set a strong master password

### Linux

```bash
chmod +x lspv-x86_64.AppImage
./lspv-x86_64.AppImage
```

---

## Browser Extension

LSPV talks to Chrome/Firefox via the [Native Messaging API](https://developer.chrome.com/docs/apps/nativeMessaging/) — a local named pipe, no WebSocket, no cloud relay.

### Chrome / Edge
1. In LSPV: **Settings → Browser → Chrome/Edge** — paste your extension ID from `chrome://extensions`
2. Click **Apply & Register** — writes native messaging manifest to the registry
3. Load unpacked: `chrome://extensions` → Developer mode → **Load unpacked** → `extension/dist/`
4. Pin the LSPV icon to the toolbar

### Firefox
1. **Settings → Browser → Firefox** → click **Add**
2. **Apply & Register**
3. `about:debugging` → This Firefox → **Load Temporary Add-on** → `extension/dist/manifest.json`

### How auto-fill works
- Popup shows items whose eTLD+1 domain matches the current tab
- **Fill** injects credentials via native setter — not tracked by browser history
- Content script detects login forms and shows an inline suggestion badge
- Every IPC response is Ed25519-signed; extension verifies before acting

---

## Backup & Recovery

> Settings → Backup tab

1. Click **Generate Phrase** — LSPV generates 24 BIP-39 words
2. Write them on paper. LSPV never stores the mnemonic on disk.
3. Tick the checkbox confirming you've written them down
4. Optionally: complete the 3-word spot-check to verify you wrote them correctly
5. Click **Export .vbk** → choose save location

To restore: paste your 24 words → pick the `.vbk` file → choose destination folder.

Auto-backups are saved automatically to `%APPDATA%/lspv/backups/` (Windows) or `~/.local/share/lspv/backups/` (Linux) on every manual export. The 7 most recent copies are kept.

---

## Multi-Vault

LSPV supports multiple independent encrypted databases. Each vault has its own master password, Vault Key, BIP-39 mnemonic, and OS Keychain entry.

Create: **+ New Vault** on the vault picker. Vaults can live on a local drive, external disk, or network share — any path your OS can open as a file.

---

## Build from Source

### Prerequisites

- [Rust 1.96+](https://rustup.rs/) + [Node.js 20+](https://nodejs.org/)
- **Windows:** Microsoft C++ Build Tools (MSVC 2019+)
- **Linux:** `apt install build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev libsecret-1-dev`

### Build

```bash
git clone https://github.com/RakinSV/local-security-pass-vault.git
cd local-security-pass-vault

# Core library + tests
cargo build && cargo test

# Desktop app (release)
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
cargo audit       # CVE scan of all Rust dependencies
cargo clippy      # Static analysis (-D warnings enforced in CI)
node extension/scripts/sri-check.js   # SRI integrity check on built extension
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
├── desktop/             # Tauri 2 desktop app (Windows + Linux + macOS)
│   ├── src/             # React + Tailwind frontend
│   │   └── pages/       # Settings (Security, Backup, Browser tabs), Vault, etc.
│   └── src-tauri/       # Rust — IPC commands, OS Keychain, tray, browser integration
├── extension/           # Browser extension (Chrome + Firefox, Manifest V3)
│   ├── src/
│   │   ├── background/  # Native messaging bridge + Ed25519 signature verification
│   │   ├── content/     # Auto-fill content script
│   │   └── popup/       # React popup
│   └── scripts/         # sri-check.js — build integrity verification
├── docs/
│   ├── adr/             # Architecture Decision Records
│   ├── wiki/            # GitHub wiki source files
│   └── threat-model.md  # Full STRIDE/PASTA threat model
└── CLAUDE.md            # Development rules + crypto constraints
```

---

## Architecture Decisions

- **[ADR-001](docs/adr/ADR-001-crypto-library.md)** — Why libsodium over ring / RustCrypto
- **[ADR-002](docs/adr/ADR-002-desktop-framework.md)** — Tauri 2, envelope encryption, Vault Key design
- **[ADR-003](docs/adr/ADR-003-backup-format.md)** — BIP-39 backup with Argon2id 4 GB + BLAKE3

---

## Contributing

PRs welcome. Hard rules:

1. **Crypto code** — read `.claude/rules/crypto.md` before touching `core-vault/src/crypto/`
2. **No new crypto deps** — libsodium only. No `openssl`, `ring`, or `argon2` crates without explicit security review
3. **Tests** — crypto changes require official test vectors (RFC 9106 for Argon2id, libsodium suite for XChaCha20)
4. **No telemetry** — any PR adding outbound network calls will be closed immediately
5. Run `cargo audit` before opening a PR

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide.

---

## Roadmap

### ✅ v0.2 — Security Hardening (complete)
- ✅ Full backup export/import UI (Settings → Backup: 4×6 word grid, 3-word verify, `.vbk` export)
- ✅ Auto-backup rotation (7 timestamped copies on each export)
- ✅ Auto-lock timer (configurable idle timeout, lock-on-minimize)
- ✅ OS Keychain status UI (Settings → Security: show cache status, Remove button)
- ✅ `PR_SET_DUMPABLE=0` explicitly called at startup on Linux
- ✅ SRI integrity check for browser extension in CI

### 🔜 v0.3 — Productivity
- [ ] TOTP generator (in-app 2FA codes from stored TOTP secrets)
- [ ] Password health report (duplicates, weak, breached — local check only, no network)
- [ ] LAN sync between devices (no cloud involved)
- [ ] Password generator (configurable length, charset, pronounceable options)

### 🔜 v0.4 — Mobile
- [ ] **Android app** — Tauri 2 Mobile, same Rust crypto core, same React UI
- [ ] **iOS app** — Tauri 2 Mobile for iPhone/iPad

### 🔜 v0.5 — Hardware Vault
- [ ] **ESP32 hardware key** — vault unlock requires the physical device via USB/BLE
- [ ] **M5StickC Plus2** — standalone portable vault with button-press unlock and BLE output

---

## Support the Project

LSPV is built in spare time, with no funding, no company, and no plan to monetize it. If it's useful to you, consider donating — it helps cover time and infrastructure.

**Bitcoin:**
```
bc1qwnkyez3nv86dry54dqfjjtav29qqq72h69pevw
```

No pressure. A GitHub star costs nothing and also helps.

---

## License

GPL-3.0 — see [LICENSE](LICENSE).

The GPL ensures this stays free and open. If you modify and distribute LSPV, your changes must be open source too.

---

<div align="center">

*Keywords: local password manager · offline password manager · open source password manager · zero knowledge password manager · Rust password manager · self-hosted password manager · no cloud password manager · Tauri password manager · libsodium · SQLCipher · Argon2id · XChaCha20-Poly1305*

</div>
