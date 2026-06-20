# Feature Guide

Complete guide to all LSPV features.

## Vault Management

### Multi-Vault

LSPV supports multiple separate encrypted databases. Each vault has its own master password, its own SQLCipher encryption key, and its own OS Keychain entry.

**Create a vault:** Click the `+` button on the vault picker screen → choose a name → set a master password → click **Create**.

**Switch vaults:** Return to the vault picker (click the vault name in the top bar or close the current vault). Your previously unlocked vault is locked automatically.

**Move items between vaults:** Export from one vault as CSV → import into the other vault. There is no merge/sync — vaults are intentionally isolated.

---

## Item Types

LSPV stores 6 types of items:

| Type | Fields |
|------|--------|
| **Login** | Name, URL, Username, Password, TOTP Secret, Notes, Custom Fields |
| **Card** | Cardholder Name, Card Number, Expiry, CVV, Notes |
| **Secure Note** | Name, Notes |
| **Identity** | First/Last Name, Email, Phone, Address, Notes |
| **SSH Key** | Name, Public Key, Private Key, Comment, Notes |
| **Server** | Name, Hostname, Port, Username, Password, Notes |

---

## Custom Fields (Login items)

Custom fields let you attach extra data to any Login entry — API keys, security questions, recovery codes, PINs.

**Add a custom field:**
1. Open any Login item → click **Edit**
2. Scroll to the **Custom Fields** section
3. Click **+ Add Field**
4. Enter a name and value
5. Toggle **Hidden** if you want the value masked by default
6. Click **Save**

Custom field values are encrypted the same way as passwords — XChaCha20-Poly1305 with a unique nonce per save.

---

## TOTP / 2FA

Store TOTP secrets alongside your login credentials and get live 6-digit codes without a separate app.

**Add a TOTP secret:**
1. Open or create a Login item → click **Edit**
2. In the **TOTP Secret** field, paste your base32-encoded secret (usually starts with a `otpauth://` URI or a raw base32 string)
3. If you have a QR code: copy the image to clipboard, then click **Scan from Clipboard** — LSPV will extract the secret automatically
4. Click **Save**

**Use the code:** In item detail view, the TOTP section shows the 6-digit code with a countdown ring. Click the code to copy it. A new code is generated every 30 seconds.

---

## Password Generator

Generate strong passwords directly from the Add/Edit item form.

**Open the generator:** In any password field, click the dice icon 🎲

**Options:**
- **Length** — slider from 8 to 64 characters
- **Uppercase** — A–Z
- **Lowercase** — a–z  
- **Digits** — 0–9
- **Symbols** — `!@#$%^&*()_+-=[]{};':"|<>?,./`

The **strength meter** shows entropy in bits. Aim for ≥ 80 bits for standard accounts, ≥ 128 bits for high-value accounts.

Click **Use** to fill the password field. The generated password is not stored anywhere until you click **Save** on the item.

---

## Password History

LSPV saves a snapshot of the previous password every time you change it.

**View history:**
1. Open a Login item → item detail view
2. Click **Password History** (appears below the password field)
3. Each entry shows the masked password and the date it was changed
4. Click any entry to reveal or copy the old password

Password history entries are encrypted with the same Vault Key as all other records.

---

## Favorites

Star items you access frequently to find them instantly.

**Star an item:** In item detail view, click the ☆ icon in the top bar → it turns ★.

**Filter to favorites:** In the sidebar, click **Favorites** under the filter section. The vault list shows only starred items.

---

## Folders

Organize items into folders (categories).

**Create a folder:**
1. Settings → Data tab
2. Click **Create** next to the 📁 Folders section
3. Enter a folder name → click **Save**

**Assign an item to a folder:**
1. Open or create any item → click **Edit**
2. In the **Folder** dropdown, select a folder
3. Click **Save**

**Filter by folder:** In the sidebar, click a folder name under the Folders section.

---

## Trash Bin

When you delete an item, it goes to the Trash instead of being permanently removed.

**View trash:**
1. Settings → Data tab → click **Trash (N)** to expand
2. All soft-deleted items are listed

**Restore an item:** Click **Restore** next to the item.

**Permanently delete one item:** Click **Delete** next to the item — this is irreversible.

**Empty trash:** Click **Empty Trash** — permanently deletes all items in trash. This is irreversible.

Items in trash are still encrypted in the database with the `deleted = 1` flag set. They do not appear in vault search or the main list.

---

## Password Health Report

Scan your entire vault for password security issues.

**Run a health check:**
1. Settings → Data tab → click **🔍 Password Health**
2. Click **Run health check**

The scanner detects:
- **Weak passwords** — fewer than 12 characters OR fewer than 2 character classes
- **Duplicate passwords** — same password used across multiple accounts (massive risk if one account is breached)
- **Old passwords** — passwords not changed in more than 6 months

Results show the item name and the specific issue. Click any item to go directly to it.

---

## CSV Export

Export all your Login items to a CSV file compatible with Chrome, Firefox, and other password managers.

**Export:**
1. Settings → Data tab → click **📤 Export passwords**
2. Click **Export as CSV**
3. Choose a save location

**Format:**
```
name,url,username,password,note
GitHub,https://github.com,myuser,s3cret,personal account
```

**Security note:** The exported CSV is **not encrypted**. Store it securely and delete it after use. Do not email it or upload it to cloud storage.

---

## CSV Import

Import passwords from Chrome, Firefox, Bitwarden, or 1Password.

**From Chrome/Firefox:** Settings → Import → CSV → select your exported CSV file.

**From Bitwarden:** Export as JSON (unencrypted) or CSV from Bitwarden → import CSV in LSPV.

**From 1Password:** Export as 1PIF or CSV from 1Password → import in LSPV.

---

## Search

Type in the search bar at the top of the vault list to search items.

Search uses an **HMAC index** — it searches without decrypting every record. Only the title is indexed. URL, username, and notes are NOT in the search index (they are encrypted-at-rest without indexing).

---

## Sidebar Filters

The left sidebar lets you filter the vault list:

| Filter | Shows |
|--------|-------|
| All Items | Everything (not in trash) |
| Favorites | ★ starred items only |
| Logins / Cards / Notes… | By item type |
| Folders | Items in that folder |
| Source Tags | Items imported from a specific browser profile |

---

## Settings: General

- **Start with system** — launch LSPV on login (Windows autostart / Linux systemd user unit)
- **Minimize to tray** — closing the window goes to system tray instead of quitting

---

## Settings: Security

- **Auto-lock timer** — lock the vault after N minutes of inactivity (1 min / 5 min / 15 min / 30 min / 1 hour / Never)
- **Lock when minimized** — lock immediately when the app window is minimized
- **OS Keychain** — shows whether the vault key is cached in Windows Credential Manager / macOS Keychain / libsecret. Click **Remove** to delete the cached entry (next unlock will require master password)
- **Change master password** — enter current password, new password (×2), click Change

---

## Settings: Backup

See [Backup & Recovery](Backup-and-Recovery.md) for full documentation.

---

## Settings: Browser

See [Browser Extension](Browser-Extension.md) for full documentation.

---

## Settings: Data

All data management tools in one place:

| Section | Function |
|---------|----------|
| 📁 Folders | Create / delete folders |
| 🗑 Trash (N) | View, restore, or purge deleted items |
| 🔍 Password Health | Run a vault-wide password health scan |
| 📤 Export passwords | Export as CSV |

---

## Settings: About

- LSPV version
- Technology stack (Crypto / Desktop / Database / Backups / Keychain)
- GitHub repository link
- Contact email
- Bitcoin donation address
