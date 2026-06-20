# Build from Source

Complete instructions for building LSPV from source on Windows, Linux, and macOS.

---

## Prerequisites

### All platforms

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.96+ | [rustup.rs](https://rustup.rs/) |
| Node.js | 20+ | [nodejs.org](https://nodejs.org/) |
| npm | 10+ | Bundled with Node.js |

### Windows

- **Microsoft C++ Build Tools** (MSVC 2019+) — [Download](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  - Select: "Desktop development with C++"
  - Alternative: Full Visual Studio Community (includes MSVC)
- **WebView2 Runtime** — pre-installed on Windows 11, available from Microsoft for Windows 10

### Linux

```bash
# Ubuntu / Debian
sudo apt install build-essential \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  libsecret-1-dev \
  pkg-config

# Arch Linux
sudo pacman -S base-devel webkit2gtk-4.1 gtk3 libayatana-appindicator libsecret

# Fedora
sudo dnf install @development-tools webkit2gtk4.1-devel \
  gtk3-devel libappindicator-gtk3-devel libsecret-devel
```

### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

---

## Clone the Repository

```bash
git clone https://github.com/RakinSV/local-security-pass-vault.git
cd local-security-pass-vault
```

---

## Build the Core Library

```bash
# Build all Rust crates (core-vault, desktop backend)
cargo build

# Run all tests including crypto test vectors
cargo test

# Security audit — check for known CVEs in dependencies
cargo audit
```

---

## Build the Desktop App

```bash
cd desktop

# Install JavaScript dependencies
npm install

# Development mode (hot reload, debug build)
npm run tauri dev

# Production release build
npm run tauri build
```

### Release artifacts (after `tauri build`)

**Windows:**
```
desktop/src-tauri/target/release/bundle/nsis/Local Security Pass Vault_*_x64-setup.exe
desktop/src-tauri/target/release/bundle/msi/Local Security Pass Vault_*_x64_en-US.msi
```

**Linux:**
```
desktop/src-tauri/target/release/bundle/appimage/local-security-pass-vault_*.AppImage
desktop/src-tauri/target/release/bundle/deb/local-security-pass-vault_*_amd64.deb
```

**macOS:**
```
desktop/src-tauri/target/release/bundle/dmg/Local Security Pass Vault_*.dmg
desktop/src-tauri/target/release/bundle/macos/Local Security Pass Vault.app
```

---

## Build the Browser Extension

```bash
cd extension

# Install dependencies
npm install

# Development build (watch mode)
npm run dev

# Production build
npm run build

# SRI integrity check on built files
node scripts/sri-check.js
```

Built extension is in `extension/dist/`. Load it in Chrome via `chrome://extensions` → Developer mode → Load unpacked.

---

## Run the Test Suite

```bash
# All Rust tests
cargo test

# Specific crate
cargo test -p core-vault

# Run with output (verbose)
cargo test -- --nocapture

# Specific test
cargo test crypto::tests::argon2id_rfc9106_test_vector

# Extension unit tests
cd extension && npm test
```

---

## Static Analysis

```bash
# Rust lints (zero warnings enforced in CI)
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check

# CVE scan
cargo audit
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LSPV_LOG` | `info` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `LSPV_DATA_DIR` | Platform default | Override vault storage directory |

Platform defaults:
- **Windows:** `%APPDATA%\lspv\`
- **Linux:** `~/.local/share/lspv/`
- **macOS:** `~/Library/Application Support/lspv/`

---

## Development Workflow

### Backend-only changes (Rust)

```bash
# Fast iteration — no frontend rebuild
cargo test -p core-vault

# Test Tauri commands
cd desktop && cargo test -p lspv-desktop
```

### Frontend-only changes (React)

```bash
cd desktop && npm run tauri dev
# App reloads automatically on .tsx/.css changes
```

### Adding a new Tauri command

1. Add the command function in `desktop/src-tauri/src/commands/`
2. Register it in `desktop/src-tauri/src/lib.rs` in the `.invoke_handler(tauri::generate_handler![...])`
3. Call it from React via `invoke('command_name', { args })`

---

## CI Pipeline

CI runs on every push and pull request:

```yaml
# .github/workflows/security.yml
jobs:
  - cargo audit       # CVE scan
  - cargo clippy      # static analysis
  - cargo test        # all tests on Ubuntu + Windows
  - sri-check         # extension SRI integrity
  - fuzz (weekly)     # cargo fuzz on vault_open + item_deserialize
```

---

## Crypto Rules (read before touching core-vault)

See `.claude/rules/crypto.md` for mandatory rules:
- Never use `rand::thread_rng()` — use `randombytes_buf()` from libsodium
- Fresh 192-bit nonce on every record save — never reuse
- `sodium_memzero()` on every key after use — never `memset()`
- `sodium_memcmp()` for MAC comparison — never `==`
- Only libsodium for crypto — no `openssl`, `ring`, or separate `argon2` crate

---

## Project Structure

```
local-security-pass-vault/
├── core-vault/               # Rust crypto core
│   └── src/
│       ├── crypto/           # Argon2id KDF, XChaCha20, HMAC search index
│       ├── backup/           # BIP-39 + BLAKE3 backup (.vbk format)
│       ├── models.rs         # Item types, vault schema
│       └── db.rs             # SQLCipher integration
├── desktop/                  # Tauri 2 desktop app
│   ├── src/                  # React + Tailwind UI
│   │   ├── pages/
│   │   │   ├── VaultList.tsx
│   │   │   ├── ItemForm.tsx
│   │   │   ├── ItemDetail.tsx
│   │   │   └── Settings/
│   │   │       ├── General.tsx
│   │   │       ├── Security.tsx
│   │   │       ├── Backup.tsx
│   │   │       ├── Browser.tsx
│   │   │       ├── Import.tsx
│   │   │       ├── Data.tsx    # Folders, Trash, Health, CSV export
│   │   │       └── About.tsx
│   └── src-tauri/            # Rust backend
│       ├── src/
│       │   ├── commands/     # Tauri IPC commands
│       │   ├── browser/      # Native messaging host
│       │   └── tray.rs       # System tray
│       └── tauri.conf.json
├── extension/                # Browser extension (MV3)
│   ├── src/
│   │   ├── background/       # Native messaging bridge + Ed25519 verify
│   │   ├── content/          # Auto-fill content script
│   │   └── popup/            # React popup
│   └── scripts/
│       └── sri-check.js      # SRI integrity verification
├── docs/
│   ├── adr/                  # Architecture Decision Records
│   ├── screenshots/          # App screenshots
│   ├── wiki/                 # GitHub Wiki pages
│   ├── index.html            # GitHub Pages landing
│   └── threat-model.md
└── .claude/rules/            # Developer crypto & security rules
    ├── crypto.md
    ├── security.md
    ├── vault-schema.md
    ├── browser-extension.md
    ├── testing.md
    └── backup.md
```
