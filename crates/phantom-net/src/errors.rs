use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("peer connection failed: {0}")]
    ConnectionFailed(String),

    #[error("peer not found: {peer_id}")]
    PeerNotFound { peer_id: String },

    #[error("peer already connected: {peer_id}")]
    PeerAlreadyConnected { peer_id: String },

    #[error("CRDT sync failed: {0}")]
    SyncFailed(String),

    #[error("CRDT merge conflict: {0}")]
    MergeConflict(String),

    #[error("DHT lookup failed: {0}")]
    DhtLookupFailed(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("dial error: {0}")]
    DialError(String),

    #[error("listen error: {0}")]
    ListenError(String),

    #[error("mesh not started")]
    MeshNotStarted,

    #[error("message too large: {size} bytes (max: {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("authentication failed for peer {peer_id}")]
    AuthFailed { peer_id: String },

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for NetError {
    fn from(e: serde_json::Error) -> Self {
        NetError::Serialization(e.to_string())
    }
}
