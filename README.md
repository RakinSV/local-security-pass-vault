<div align="center">

# 🔐 VaultPass

**Local-first password manager. Zero cloud. Zero telemetry. Zero trust required.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/Dex-vabster/local-security-pass-vault/actions/workflows/security.yml/badge.svg)](https://github.com/Dex-vabster/local-security-pass-vault/actions)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](#installation)
[![Rust](https://img.shields.io/badge/crypto-libsodium%20%2B%20Argon2id-orange)](core-vault/)
[![MV3](https://img.shields.io/badge/extension-MV3%20%E2%80%94%20Chrome%20%7C%20Firefox%20%7C%20Edge-4285F4)](#browser-extension-setup)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](CONTRIBUTING.md)

[Features](#-features) · [Security Model](#-security-model) · [Install](#-installation) · [Build from Source](#-build-from-source) · [Browser Extension](#-browser-extension-setup) · [Contributing](#-contributing)

</div>

---

VaultPass is a **fully offline** password manager that stores everything on your machine and nowhere else. It is built on a Rust cryptographic core using the audited [libsodium](https://libsodium.org) library, exposed through a [Tauri 2](https://tauri.app) desktop application (~8 MB binary), and bridged to a Manifest V3 browser extension for **Chrome, Firefox, and Edge**.

> 🚧 **Public beta** — the core is solid and tested, but the UI and packaging are still maturing. Your feedback and bug reports are the fuel. See [Contributing](#-contributing).

---

## ✨ Features

| Category | What you get |
|---|---|
| **Vault items** | Login · Credit card · Secure note · Identity · SSH key |
| **Encryption** | XChaCha20-Poly1305 AEAD per item · SQLCipher (AES-256) file layer |
| **KDF** | Argon2id · 256 MB RAM · 4 iterations (libsodium OPSLIMIT/MEMLIMIT_SENSITIVE) |
| **Browser** | MV3 extension for Chrome, Firefox 109+, Edge · auto-fill shadow-DOM prompt |
| **Profiles** | Identifies each Chrome/Firefox profile separately · shows Google account email |
| **Import** | One-click CSV import from Chrome and Firefox password managers |
| **IPC security** | Ed25519-signed responses over named pipe / Unix socket |
| **No network** | Zero outbound requests — extension makes no `fetch()` calls |
| **Platforms** | Windows 10+ and Linux (x64) |

---

## 🔒 Security Model

```
Master Password
      │
      ▼ Argon2id KDF — 256 MB RAM, 4 iterations
      │
      ├─► db_key        (32 B) → opens SQLCipher database
      ├─► encryption_key (32 B) → decrypts Vault Key
      └─► search_key    (32 B) → HMAC-SHA256 search index

Vault Key (32 B, random at creation)
      │
      ▼ XChaCha20-Poly1305 — unique 192-bit nonce per save
      │
      └─► every vault item (stored in SQLCipher)
```

Key security decisions:

- **Envelope encryption** — changing your master password re-wraps only the Vault Key; no item re-encryption needed.
- **Memory hardening** — `mlock()` + `sodium_memzero()` on all key material; keys zeroed on vault lock.
- **Atomic writes** — all saves go through `tmp → fsync → rename`; corrupted writes are impossible.
- **Symlink guard** — vault file is opened with `O_NOFOLLOW`; symlink detected → immediate error.
- **Honeypot** — a sentinel file next to the vault detects ransomware modification at unlock.
- **No clipboard history** — clipboard auto-clears after 30 s; flagged with `CF_EXCLUDEFROMCLOUDCLIPBOARD` on Windows.
- **IPC authentication** — each native messaging response is Ed25519-signed by the desktop; the extension can verify.
- **eTLD+1 domain matching** — auto-fill uses [tldts](https://www.npmjs.com/package/tldts) to prevent subdomain injection attacks.

Full threat model: [`docs/threat-model.md`](docs/threat-model.md)

---

## 📥 Installation

### Windows (pre-built)

> Pre-built installers will be available in [Releases](https://github.com/Dex-vabster/local-security-pass-vault/releases) once v1.0 ships. Until then, please [build from source](#-build-from-source) — it takes about 5 minutes.

### Linux (pre-built)

Same as above — follow [build from source](#-build-from-source).

---

## 🛠 Build from Source

### Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | stable (MSVC on Windows) | `rustup show` |
| Node.js | 18+ | for the extension build |
| Visual Studio 2022 | C++ workload | Windows only |

```bash
git clone https://github.com/Dex-vabster/local-security-pass-vault.git
cd local-security-pass-vault
```

#### 1 — Build the crypto core + desktop app

```bash
# Windows (must be inside a VS Developer shell):
cargo build --release -p vaultpass-desktop

# Linux:
cargo build --release -p vaultpass-desktop
```

#### 2 — Build the native messaging host

```bash
cargo build --release -p vaultpass-native-host
```

#### 3 — Build the browser extension

```bash
cd extension
npm install
npm run build          # output → extension/dist/
```

#### 4 — Run the desktop app (dev mode)

```bash
cd desktop
npm install
npm run tauri dev
```

---

## 🌐 Browser Extension Setup

> The extension uses **Native Messaging** to talk to the desktop app. The desktop app registers the native host in the OS — no manual registry editing needed.

### Chrome / Edge

1. Open `chrome://extensions` (or `edge://extensions`)
2. Enable **Developer mode**
3. Click **Load unpacked** → select the `extension/dist/` folder
4. Copy the **Extension ID** shown under VaultPass
5. Open VaultPass desktop → **Settings → Browser** → paste the ID → **Apply & Register**

### Firefox

1. Open `about:debugging` → **This Firefox**
2. Click **Load Temporary Add-on** → select `extension/dist/manifest.json`
3. In VaultPass desktop → **Settings → Browser** → Firefox section → click the input field (auto-fills `vaultpass@vaultpass.app`) → **Add** → **Apply & Register**

> **Firefox note:** temporary add-ons are removed on restart. For permanent install, use Firefox Developer Edition which allows unsigned extensions, or package as a signed `.xpi`.

### Auto-fill in action

Once connected, open any login page — a shadow-DOM card appears top-right with matching credentials. Click **Fill** to autofill without opening the popup. The card auto-dismisses after 20 seconds.

---

## 🗂 Project Structure

```
vaultpass/
  core-vault/       Rust crate — crypto, DB, vault lifecycle (37 tests)
  desktop/          Tauri 2 app (UI + IPC server + commands)
    src/            React + Tailwind frontend
    src-tauri/      Rust backend
  extension/        MV3 browser extension (TypeScript + Vite)
    src/background/ Service worker, native messaging, profile detection
    src/content/    Shadow-DOM auto-fill prompt
    src/popup/      React popup UI
  native-host/      Rust binary — bridges browser ↔ Tauri named pipe
  docs/
    adr/            Architecture Decision Records
    threat-model.md Full threat model
  fuzz/             Cargo-fuzz targets (vault_open, item_deserialize)
  scripts/          Build and release helpers
```

---

## 🧪 Running Tests

```bash
# All unit + integration tests (includes crypto test vectors)
cargo test

# Security audit (CVE check)
cargo audit

# TypeScript type check
cd extension && npx tsc --noEmit
cd ../desktop && npx tsc --noEmit

# Fuzz (requires nightly + Linux)
cargo +nightly fuzz run fuzz_vault_open -- -max_total_time=60
```

---

## 🤝 Contributing

All contributions are welcome — bug reports, code, docs, design, security review.

**Quick start:**

```bash
git clone https://github.com/Dex-vabster/local-security-pass-vault.git
cd local-security-pass-vault
cargo test           # make sure tests pass
```

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide, especially the **security review checklist** if you touch crypto code.

Good first issues are tagged [`good first issue`](https://github.com/Dex-vabster/local-security-pass-vault/labels/good%20first%20issue).

---

## 🔏 Security Disclosures

**Please do not open a public issue for security vulnerabilities.**

Report them privately via GitHub's [Security Advisories](https://github.com/Dex-vabster/local-security-pass-vault/security/advisories/new) or email `s79504688425@gmail.com` with subject `[VaultPass Security]`.

See [SECURITY.md](SECURITY.md) for the full disclosure policy and response SLA.

---

## 📄 License

MIT — see [LICENSE](LICENSE).

---

<div align="center">

If VaultPass is useful to you, please ⭐ the repo — it helps others find it.

</div>
