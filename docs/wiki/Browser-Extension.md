# Browser Extension Setup

LSPV's browser extension communicates with the desktop app via the [Native Messaging API](https://developer.chrome.com/docs/apps/nativeMessaging/) — a local named pipe, no network involved.

## Architecture

```
Browser Tab
    │
    ▼
Content Script (content.js)
    │  chrome.runtime.sendMessage
    ▼
Background Service Worker (background.js)
    │  chrome.runtime.connectNative("com.vaultpass.native")
    ▼
Named Pipe (Windows) / Unix Socket (Linux)
    │
    ▼
LSPV Desktop (native host binary)
    │
    ▼
Vault (SQLCipher)
```

Every response from the desktop is Ed25519-signed. The extension verifies the signature before using the data. See [Security Architecture — Layer 7](Security-Architecture.md#layer-7--browser-ipc-trust) for details.

## Chrome / Edge

### Step 1 — Register the native host

1. Open LSPV desktop
2. Go to **Settings → Browser → Chrome / Edge**
3. Open `chrome://extensions`, enable **Developer mode**, copy the extension ID
4. Paste the ID in LSPV and click **Add**, then **Apply & Register**

This writes the native messaging manifest to the Windows registry:
```
HKCU\SOFTWARE\Google\Chrome\NativeMessagingHosts\com.vaultpass.native
```

### Step 2 — Load the extension

1. `chrome://extensions` → **Load unpacked**
2. Select the `extension/dist/` folder from the LSPV repository
3. Pin the LSPV icon to the toolbar

### Step 3 — Verify

Click the LSPV extension icon. If the vault is unlocked, you should see your items. If you see "LSPV desktop is not running," check that the desktop app is open.

## Firefox

### Step 1 — Register the native host

1. Open LSPV desktop → **Settings → Browser → Firefox**
2. Click **Add** (the ID `lspv@lspv.app` prefills automatically)
3. Click **Apply & Register**

This copies a native messaging manifest to:
- **Windows:** `%APPDATA%\Mozilla\NativeMessagingHosts\com.vaultpass.native.json`
- **Linux:** `~/.mozilla/native-messaging-hosts/com.vaultpass.native.json`

### Step 2 — Load the extension

1. `about:debugging` → **This Firefox** → **Load Temporary Add-on**
2. Select `extension/dist/manifest.json`

> Note: Temporary add-ons are removed when Firefox closes. For persistent install, the extension needs to be signed via AMO or installed as a policy.

## Auto-fill

### How it works

1. You open a web page
2. The content script detects visible login forms
3. It sends a `DETECT_FORM` message to the background worker with the current URL
4. The background worker queries the desktop: "do you have a login for this domain?"
5. If yes, an inline prompt appears near the username/password fields
6. Clicking a suggestion injects the credentials using the native input value setter (not tracked by browser history)

### Domain matching

LSPV matches on eTLD+1 (effective top-level domain + one), not the full URL. Examples:

| Vault URL | Page URL | Match? |
|-----------|----------|--------|
| `https://google.com` | `https://accounts.google.com` | ✅ |
| `https://google.com` | `https://google.com.evil.ru` | ❌ |
| `https://paypal.com` | `https://paypa1.com` | ❌ |
| `https://github.com` | `https://github.io` | ❌ |
| `https://amazon.co.uk` | `https://www.amazon.co.uk` | ✅ |

### Security notes

- The extension never caches passwords in memory, localStorage, or `chrome.storage`
- Every credential request is a fresh IPC call to the desktop
- Credentials are injected and immediately discarded — no variable holds them after fill
- The content script runs in an **isolated world** — page JavaScript cannot read its variables

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| "LSPV desktop is not running" | Open the desktop app and unlock your vault |
| Extension can't find the native host | Re-register in Settings → Browser → Apply & Register |
| Extension shows items but Fill doesn't work | Make sure LSPV has permission to `activeTab` in `chrome://extensions` |
| Popup is blank / spinner forever | Check DevTools console in the extension popup for errors |
| Auto-fill prompt doesn't appear | Some sites block content script injection — use the popup manually |

## Connected Profiles

Each Chrome profile (e.g., personal, work) that connects to LSPV gets its own entry in **Settings → Browser → Connected profiles**. You can give each profile a friendly name to identify which machine or account it belongs to.
