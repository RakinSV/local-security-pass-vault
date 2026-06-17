# Contributing to VaultPass

Thank you for considering contributing! VaultPass is a security-critical project, so we hold code to a higher standard than a typical app — especially anything touching cryptography, key handling, or the browser extension.

## Ways to Contribute

| Type | What helps most right now |
|------|--------------------------|
| 🐛 Bug reports | Unexpected crashes, UI glitches, wrong auto-fill behaviour |
| 🔒 Security review | Crypto code audit, IPC protocol review, extension CSP review |
| 🌍 Translations | README and UI strings in other languages |
| 🧪 Testing | Testing on different Linux distros, Firefox versions, Chrome profiles |
| 📖 Docs | Improving installation guides, adding screenshots |
| 💻 Code | See [Good First Issues](https://github.com/Dex-vabster/local-security-pass-vault/labels/good%20first%20issue) |

## Development Setup

```bash
git clone https://github.com/Dex-vabster/local-security-pass-vault.git
cd local-security-pass-vault

# Build everything
cargo build

# Run tests (must pass before any PR)
cargo test
cargo clippy -- -D warnings

# Extension
cd extension && npm install && npm run build && npx tsc --noEmit

# Desktop frontend
cd desktop && npm install && npx tsc --noEmit
```

## Pull Request Process

1. **Fork** the repository and create a branch: `git checkout -b fix/your-description`
2. Make your changes
3. Ensure **all tests pass**: `cargo test`
4. Ensure **clippy is clean**: `cargo clippy -- -D warnings`
5. For extension changes: `npm run build && npx tsc --noEmit`
6. Open a PR against `main` with a clear description of what and why

Small, focused PRs are preferred over large omnibus changes.

## Security Review Checklist

If your PR touches cryptography or key handling, reviewers will check:

```
☐ No rand::thread_rng() for key material (use libsodium randombytes_buf)
☐ sodium_memzero() after all key use
☐ sodium_memcmp() for MAC/hash comparisons (not ==)
☐ New nonce generated on every encrypt call
☐ No key material in logs (even debug/trace)
☐ Atomic write (tmp → fsync → rename) for any vault file write
```

For browser extension changes:

```
☐ No fetch() / XHR / WebSocket calls from the extension
☐ No password caching in extension storage
☐ Domain comparison uses eTLD+1 via tldts (not string includes)
☐ Only visible form fields are filled
☐ Every IPC request includes a unique nonce
```

## Commit Style

Use conventional commits:

```
feat: add TOTP code display in item detail
fix: content script fails to detect SPA login forms
docs: add Linux build instructions
chore: bump libsodium-sys-stable to 0.2.1
```

## Architecture Decisions

Before making large architectural changes, please open an issue first to discuss. The `docs/adr/` directory contains Architecture Decision Records explaining why certain choices were made.

## Code of Conduct

Be kind, be constructive. Security debates should focus on threat models and mitigations, not personal preferences.
