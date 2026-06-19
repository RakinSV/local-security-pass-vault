# Backup & Recovery

LSPV uses a 24-word BIP-39 mnemonic phrase as the encryption key for backup files. The mnemonic is generated when you create a vault and shown **once** — LSPV never stores it on disk.

## Why BIP-39?

- 256 bits of entropy from a standardized, human-verifiable wordlist
- Easy to write down on paper without transcription errors
- Widely understood — no proprietary format
- The same mnemonic can regenerate the backup key deterministically

## Backup File Format (.vbk)

```
lspv_backup_YYYY-MM-DD_XXXXXXXX.vbk
├── Header: magic bytes b"LSPV_BACKUP_V2"
├── KDF params: Argon2id (m=4GB, t=10, p=4) + random salt
├── Encrypted payload (XChaCha20-Poly1305):
│   ├── Vault UUID
│   ├── Created timestamp
│   ├── Schema version
│   ├── BLAKE3 checksum of vault.db
│   └── Full vault.db bytes
```

**Key derivation for backup** uses much stronger Argon2id parameters than normal unlock (4 GB RAM vs 256 MB, 10 iterations vs 4). This is intentional: backup files may be stored on USB drives or cloud storage, so brute-forcing the mnemonic must be computationally infeasible even with large GPU farms.

## Export a Backup

> Backup UI is on the roadmap for v0.3. Currently available via the Rust API.

```rust
vault.export_backup(
    Path::new("/path/to/backup.vbk"),
    &seed_phrase,  // your 24-word mnemonic
)?;
```

## Restore from Backup

```rust
Vault::restore_from_backup(
    Path::new("/path/to/backup.vbk"),
    Path::new("/destination/directory"),
    &seed_phrase,
)?;
```

LSPV verifies the BLAKE3 checksum inside the ciphertext after decryption. If the file was corrupted or tampered with, restoration fails with an integrity error — it will not silently restore a broken vault.

## Recommended Backup Strategy (3-2-1)

| Copy | Location | Notes |
|------|----------|-------|
| 1 | Local drive | Automatic on each vault change (roadmap v0.3) |
| 2 | USB flash drive | Offline copy, update weekly |
| 3 | Second computer or NAS | LAN-accessible, no cloud |

Never store the mnemonic in the same physical location as the backup file. If an attacker finds both, they can decrypt the backup.

## Mnemonic Safety

- Write the 24 words on paper with a permanent marker
- Store the paper in a fireproof safe or safety deposit box
- Consider splitting: give 12 words to a trusted person and keep 12 yourself (requires both to restore)
- Never photograph the mnemonic with your phone — photos sync to cloud
- Never type it into any website or app other than LSPV

## Version Compatibility

| Format | Supported |
|--------|-----------|
| v1 (XChaCha20 + Argon2id 256 MB) | ✅ read + write |
| v2 (XChaCha20 + Argon2id 4 GB + BLAKE3) | ✅ read + write |
