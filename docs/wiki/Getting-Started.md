# Getting Started

## Installation

### Windows

1. Download `lspv-setup-x64.exe` from [Releases](https://github.com/RakinSV/local-security-pass-vault/releases)
2. Run the installer — no administrator rights required (per-user install via NSIS)
3. Launch **Local Security Pass Vault** from the Start menu or system tray

### Linux

```bash
chmod +x lspv-x86_64.AppImage
./lspv-x86_64.AppImage
```

For a permanent install:

```bash
mv lspv-x86_64.AppImage ~/.local/bin/lspv
chmod +x ~/.local/bin/lspv
```

## Creating Your First Vault

1. Click **+ New Vault** on the vault picker screen
2. Choose a location on your disk — default is inside the app data directory
3. Set a strong master password (minimum 8 characters; no built-in maximum)
4. Optionally add a password hint — stored in plaintext, so keep it vague

The app runs Argon2id with 256 MB RAM and 4 iterations to derive your vault key. This takes 1–3 seconds depending on your hardware — that's intentional.

## System Tray

LSPV lives in the system tray. Closing the main window hides it; the vault stays unlocked.

- **Left-click** tray icon — toggle window visibility
- **Right-click** tray icon — menu: Show Window / Lock & Hide / Quit

**Lock & Hide** immediately zeroes the vault key in memory, removes it from the OS Keychain, and hides the window. The app stays running in the tray.

## Autostart

Enable in **Settings → General → Start with system** to launch LSPV automatically at login.

- **Windows:** adds a registry entry under `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`
- **Linux:** creates `~/.config/autostart/lspv.desktop`
- **macOS:** creates `~/Library/LaunchAgents/com.lspv.app.plist`

## Adding Items

Click **+** in the vault list or use **Add item** to create:

| Type | Fields |
|------|--------|
| Login | URL, username, password, TOTP secret, notes, custom fields |
| Card | Cardholder, number, expiry, CVV, notes |
| Note | Markdown content |
| Identity | Name, email, phone, address, passport, notes |
| SSH Key | Private key (PEM), public key, passphrase, notes |

## Quick Unlock

After unlocking with your master password, LSPV stores the vault key in the OS Keychain. On the next unlock, you can use Windows Hello (PIN/fingerprint) or macOS Touch ID instead of typing your full password.

To remove the cached key: **Settings → Security → OS Keychain → Remove**.

## Next Steps

- [Browser Extension Setup](Browser-Extension.md)
- [Multi-Vault Guide](Multi-Vault.md)
- [Backup & Recovery](Backup-and-Recovery.md)
- [Security Architecture](Security-Architecture.md)
