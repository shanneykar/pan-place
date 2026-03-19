use thiserror::Error;

#[derive(Debug, Error)]
pub enum PanError {
    #[error("Actor not found: {0}")]
    ActorNotFound(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid hash")]
    InvalidHash,

    #[error("Hash mismatch: computed {computed}, submitted {submitted}")]
    HashMismatch { computed: String, submitted: String },

    #[error("Phone dhash already registered")]
    PhoneDhashAlreadyRegistered,

    #[error("Actor already exists: {0}")]
    ActorAlreadyExists(String),

    #[error("Parent event not found: {0}")]
    ParentNotFound(String),

    #[error("Too many parents (max 1 in layer-0)")]
    TooManyParents,

    #[error("Invalid tag: {0}")]
    InvalidTag(String),

    #[error("Content too long (max 2000 chars)")]
    ContentTooLong,

    #[error("Timestamp must be greater than parent timestamps")]
    TimestampNotForward,

    #[error("Confirmation events must have references_event set")]
    MissingReference,

    #[error("Referenced event not found: {0}")]
    ReferenceNotFound(String),

    #[error("Invalid coordinates")]
    InvalidCoordinates,

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
