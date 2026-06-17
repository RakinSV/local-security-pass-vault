# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| `main` branch | ✅ Always latest |
| Tagged releases | ✅ Latest release |
| Older releases | ❌ No backports |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

### Option 1 — GitHub Private Advisory (preferred)

Use GitHub's built-in [Security Advisory](https://github.com/Dex-vabster/local-security-pass-vault/security/advisories/new) feature. Your report is visible only to maintainers until a fix is released.

### Option 2 — Email

Send to **s79504688425@gmail.com** with subject line `[VaultPass Security]`.

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (optional)

## Response SLA

| Step | Target |
|------|--------|
| Acknowledgement | 48 hours |
| Triage + severity assessment | 5 business days |
| Fix (for Critical/High) | 14 days |
| Public disclosure | After fix is released |

## Scope

### In scope
- Cryptographic weaknesses (KDF parameters, cipher choice, nonce reuse)
- Memory disclosure of vault keys or decrypted passwords
- Authentication bypass (opening vault without master password)
- Native messaging IPC vulnerabilities
- Browser extension content script injection or XSS
- Domain matching bypass (allowing fill on wrong site)

### Out of scope
- Attacks requiring physical access to an already-unlocked vault
- Clipboard history (mitigated by `CF_EXCLUDEFROMCLOUDCLIPBOARD`; OS-level clipboard managers are out of scope)
- Social engineering
- Vulnerabilities in upstream dependencies (please report those upstream)

## Bug Bounty

This is a personal open-source project. There is no monetary bug bounty, but credited security researchers will be acknowledged in the release notes and `SECURITY.md`.

## Security Architecture

See [`docs/threat-model.md`](docs/threat-model.md) and the [Security Model](README.md#-security-model) section of the README.
