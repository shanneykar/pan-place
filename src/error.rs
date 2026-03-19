use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
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

impl IntoResponse for PanError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            PanError::ActorNotFound(_) => (StatusCode::NOT_FOUND, "actor_not_found"),
            PanError::NodeNotFound(_) => (StatusCode::NOT_FOUND, "node_not_found"),
            PanError::EventNotFound(_) => (StatusCode::NOT_FOUND, "event_not_found"),
            PanError::InvalidSignature => (StatusCode::BAD_REQUEST, "invalid_signature"),
            PanError::InvalidHash => (StatusCode::BAD_REQUEST, "invalid_hash"),
            PanError::HashMismatch { .. } => (StatusCode::BAD_REQUEST, "hash_mismatch"),
            PanError::PhoneDhashAlreadyRegistered => {
                (StatusCode::CONFLICT, "phone_dhash_already_registered")
            }
            PanError::ActorAlreadyExists(_) => (StatusCode::CONFLICT, "actor_already_exists"),
            PanError::ParentNotFound(_) => (StatusCode::BAD_REQUEST, "parent_not_found"),
            PanError::TooManyParents => (StatusCode::BAD_REQUEST, "too_many_parents"),
            PanError::InvalidTag(_) => (StatusCode::BAD_REQUEST, "invalid_tag"),
            PanError::ContentTooLong => (StatusCode::BAD_REQUEST, "content_too_long"),
            PanError::TimestampNotForward => (StatusCode::BAD_REQUEST, "timestamp_not_forward"),
            PanError::MissingReference => (StatusCode::BAD_REQUEST, "missing_reference"),
            PanError::ReferenceNotFound(_) => (StatusCode::BAD_REQUEST, "reference_not_found"),
            PanError::InvalidCoordinates => (StatusCode::BAD_REQUEST, "invalid_coordinates"),
            PanError::StorageError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "storage_error")
            }
            PanError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database_error")
            }
        };
        (
            status,
            Json(json!({ "error": code, "message": self.to_string() })),
        )
            .into_response()
    }
}
