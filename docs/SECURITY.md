# Phantom Security Model

## Key Hierarchy

```
Passphrase (user-memorized)
  └─ MasterKey (Argon2id derivation, never stored)
       ├─ Ed25519 Signing Key (signs licenses)
       ├─ Ed25519 Verifying Key (embedded in binary)
       └─ License Token (PH1-<base62_payload>-<base62_signature>)
            └─ SessionKey (HKDF-SHA256 from license bytes + timestamp)
                 └─ AgentKey (HKDF-SHA256 from session + agent_id + task_id)
                      └─ AES-256-GCM encryption key (per-blob)
```

### MasterKey

- Derived from passphrase using Argon2id (m=64MB, t=3, p=4)
- Never persisted to disk or remote storage
- Used only in `phantom master` subcommands
- Zeroized on drop via `zeroize` crate

### License Tokens

- Format: `PH1-<base62_payload>-<base62_signature>`
- Payload: JSON with `sub`, `org`, `tier`, `caps`, `iat`, `exp`, `iid` fields
- Signature: Ed25519 over raw payload bytes
- Verification uses embedded public key (no network required)
- `iid` (installation ID): 16 random bytes, hex-encoded, unique per activation

### SessionKey

- Derived via HKDF-SHA256: `ikm = license_bytes || timestamp_bytes`
- Salt: `b"phantom-session-v1"`
- Info: `b"session-key"`
- 32 bytes, Zeroize on drop

### AgentKey

- Derived from SessionKey via HKDF-SHA256
- Info: `agent_id || ":" || task_id`
- Carries `AgentPermissions` bitflags (16 bits)
- Each agent role gets a predefined permission set

## Encryption

### AES-256-GCM

- All data encrypted before leaving memory
- 12-byte random nonce per encryption operation
- Associated Authenticated Data (AAD) = storage key path
- AAD prevents blob-swapping attacks (moving encrypted blob from one key to another)

### EncryptedBlob Format

```json
{
  "nonce": "<hex-encoded 12 bytes>",
  "ciphertext": "<base64-encoded ciphertext + 16-byte auth tag>"
}
```

## Threat Model

### What Phantom Protects Against

| Threat | Mitigation |
|--------|-----------|
| Stolen license key | Expiry enforcement, per-installation `iid`, remote revocation via master key |
| Compromised remote storage | All blobs AES-256-GCM encrypted; server only sees ciphertext |
| Blob swapping on storage | AAD = key path; decryption fails if blob is moved |
| Agent privilege escalation | Per-agent permission bitflags enforced at AgentKey level |
| Memory dumps | Zeroize on drop for all key material (MasterKey, SessionKey, AgentKey) |
| Binary tampering | Ed25519 verifying key embedded at compile time |
| Credential theft | Credentials stored in encrypted Vault; never written to local disk |

### Trust Boundaries

1. **User ↔ Phantom**: Passphrase and license key are the only secrets the user provides
2. **Phantom ↔ Cloud Providers**: API tokens stored in encrypted Vault, transmitted over TLS
3. **Phantom ↔ R2 Storage**: All data encrypted client-side before upload
4. **Agent ↔ Agent**: Agents communicate via in-process channels; each agent has scoped permissions

### Zero Local Footprint (Core Law 3)

- No credentials, state, or project data written to local disk
- All persistent state stored in Cloudflare R2 (encrypted)
- On exit or kill, no trace remains on the machine
- `phantom master destroy` wipes all remote state

## Dependency Security

- Ed25519: `ed25519-dalek` (pure Rust, no OpenSSL)
- AES-256-GCM: `aes-gcm` (RustCrypto, constant-time)
- Argon2id: `argon2` (RustCrypto)
- HKDF: `hkdf` + `sha2` (RustCrypto)
- Key zeroization: `zeroize` crate with `ZeroizeOnDrop` derive
