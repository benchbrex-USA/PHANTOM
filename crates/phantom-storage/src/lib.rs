//! Phantom Storage: encrypted R2/S3 client, credential vault, zero-footprint state.
//!
//! Core Law 3: Zero local disk footprint — all state in remote encrypted storage.
//! Everything is encrypted with AES-256-GCM before leaving memory.
//! Remote servers only ever see opaque ciphertext.

pub mod errors;
pub mod r2_client;
pub mod state;
pub mod vault;

pub use errors::StorageError;
pub use r2_client::{BlobIndex, BlobMetadata, R2Client, R2Config, StorageUsage};
pub use state::{RemoteState, StateEntry, StateSummary};
pub use vault::{Vault, VaultEntry};
