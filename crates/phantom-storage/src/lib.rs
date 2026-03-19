//! Phantom Storage: encrypted R2/S3 client, credential vault, zero-footprint state.
//!
//! Core Law 3: Zero local disk footprint — all state in remote encrypted storage.
//! Everything is encrypted with AES-256-GCM before leaving memory.
//! Remote servers only ever see opaque ciphertext.

pub mod vault;
pub mod r2_client;
pub mod state;
pub mod errors;

pub use errors::StorageError;
pub use vault::{Vault, VaultEntry};
pub use r2_client::{R2Client, R2Config, BlobIndex, BlobMetadata, StorageUsage};
pub use state::{RemoteState, StateEntry, StateSummary};
