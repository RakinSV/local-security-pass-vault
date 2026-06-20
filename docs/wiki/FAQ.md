# FAQ

Frequently asked questions about LSPV.

---

## General

### Is LSPV really zero cloud?

Yes. The app makes **zero outbound network connections**. The vault file (`vault.db`) lives only on your local disk. No sync service, no analytics endpoint, no license server, no update check. You can verify this with a network monitor (Wireshark, Little Snitch, Windows Firewall log) — LSPV generates no network traffic.

### Does LSPV require an account or email?

No. There are no accounts, no registration, no email, no subscription. You install it and it works.

### Is it free forever?

Yes. LSPV is GPL-3.0 open source with no paid tier. The only way to support the project is with a Bitcoin donation.

### What platforms does LSPV support?

- **Windows 10/11** (x64) — primary platform
- **Linux** (x86_64 AppImage) — tested on Ubuntu 22.04+, Arch
- **macOS** — builds from source (not yet in releases, no code signing yet)
- **Mobile** — planned for v0.4 (Android + iOS via Tauri 2 Mobile)

---

## Security

### What happens if someone steals my vault.db file?

Without your master password, the file is useless. `vault.db` is encrypted with SQLCipher (AES-256 per page). The actual record data is additionally encrypted with XChaCha20-Poly1305. An attacker with only the file and no password sees random bytes.

Brute-forcing the master password is the only attack path. Argon2id with 256 MB RAM and 4 iterations means even an RTX 4090 can only test ~1 attempt per 2-4 seconds. A strong master password (6+ random words or 20+ chars) makes this economically infeasible.

### Can LSPV be breached like LastPass was?

No. LastPass was breached because they store encrypted vaults on their servers — attackers stole the server data. LSPV has no server. There is nothing to breach remotely. An attacker would need physical access to your machine to steal `vault.db`.

### Is my master password stored anywhere?

Never. The master password is used to derive keys via Argon2id and then immediately zeroed from RAM. It is never written to disk, never logged, never sent anywhere.

### Does LSPV prevent RAM dumps?

Partially. Keys are `mlock()`-ed (prevents the OS from paging them to swap/disk) and zeroed via `sodium_memzero()` on every lock. On Linux, `PR_SET_DUMPABLE=0` is called at startup — other processes cannot ptrace or read `/proc/PID/mem`. This significantly raises the bar, but cannot prevent an attacker with kernel-level access.

### What is the BIP-39 backup mnemonic?

A 24-word phrase (from the standard Bitcoin BIP-39 English wordlist) that acts as the encryption key for `.vbk` backup files. LSPV generates it once when you create a vault, shows it to you, and then **never stores it on disk**. Write it on paper. If you lose it, you cannot restore backups — but your vault.db still works fine with your master password.

### Can the browser extension phone home?

No. The extension has `connect-src 'none'` in its Content Security Policy. Browsers enforce this — the extension literally cannot make a network request. All communication goes via a local named pipe (Native Messaging) to the desktop process.

---

## Usage

### How do I create a new vault?

Launch LSPV → click **+ New Vault** → enter a name → set a master password → click **Create**. You will see the 24-word backup mnemonic — write it down before proceeding.

### How do I add a password?

Inside an unlocked vault → click **+ Add** (top right) → choose an item type → fill in the fields → click **Save**.

### How do I auto-fill in the browser?

Install the browser extension first (Settings → Browser → select your browser → Apply & Register → Load Unpacked in Chrome or Load Temporary Add-on in Firefox). Then when you visit a login page, click the LSPV extension icon in the toolbar and select the matching entry.

### How do I change my master password?

Settings → Security → Change master password. Enter your current password, enter the new password twice, click **Change**. This re-encrypts only the Vault Key — all individual records stay untouched.

### How do I export/backup my data?

**Encrypted backup (recommended):** Settings → Backup → Generate Phrase → write down 24 words → click Export .vbk

**Plain CSV (not encrypted):** Settings → Data → Export passwords → Export as CSV. Store this file securely; delete it when done.

### How do I import from Chrome or Firefox?

In Chrome: `chrome://password-manager/settings` → Export → Download file  
In Firefox: `about:logins` → three-dot menu → Export Logins  
Then: LSPV → Settings → Import → CSV → select the file.

### How do I move to a new computer?

1. Export a `.vbk` backup (Settings → Backup)
2. Install LSPV on the new computer
3. Create a new vault (or restore directly)
4. On the vault picker: Restore → paste 24-word mnemonic → select the `.vbk` file → choose destination

### What is the Trash?

When you delete an item, it goes to Trash (soft delete). The item is hidden from the main vault list but not permanently removed. To manage trash: Settings → Data → Trash. You can restore individual items or empty the trash permanently.

### How do I store TOTP/2FA codes?

Edit a Login item → find the **TOTP Secret** field → paste the base32 secret or click **Scan from Clipboard** if you have a QR code image copied. The live 6-digit code appears in item detail view.

---

## Troubleshooting

### The app won't open / crashes on startup

1. Check that you have the latest [Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) (Windows)
2. Check the log file: `%APPDATA%\lspv\lspv.log` (Windows) or `~/.local/share/lspv/lspv.log` (Linux)
3. [Open an issue](https://github.com/RakinSV/local-security-pass-vault/issues) with the log

### I forgot my master password

LSPV cannot recover a forgotten master password — that is by design. The encryption is one-way. If you have a backup `.vbk` file, you need the 24-word mnemonic to restore it. If you have neither, the vault data cannot be recovered.

### The browser extension is not auto-filling

1. Confirm the native host is registered: Settings → Browser → your browser should show **Registered** status
2. In Chrome, go to `chrome://extensions` → check that the extension is enabled
3. The domain must match: if you saved `github.com` but visiting `gist.github.com`, it will match (same eTLD+1). But if you saved `github.com` and visiting `github.io`, it will NOT match.
4. Check the extension popup — it may show "Vault is locked"

### I accidentally deleted an item

If the item is still in Trash (Settings → Data → Trash), click **Restore**. If the trash has been emptied, the data cannot be recovered.

### The auto-lock timer is too aggressive

Settings → Security → Auto-lock → change to a longer interval or **Never**. You can also uncheck **Lock when minimized**.

---

## Privacy

### Does LSPV collect any analytics or telemetry?

No. The binary contains no analytics SDK, no crash reporter, no telemetry. The only data that ever leaves your machine is data you explicitly send (e.g., browser extension sending credentials to the desktop native host — which is local, not remote).

### Is my vault file synchronized to any cloud?

Not by LSPV. If you have OneDrive, iCloud, Google Drive, or Dropbox sync on your `AppData` or home directory, they may pick up `vault.db`. The encrypted file is safe to sync (it's just encrypted bytes), but consider the exposure. LAN sync between your own devices is planned for v0.4.

---

## Building from Source

See [Build from Source](Build-from-Source.md) for full instructions.
