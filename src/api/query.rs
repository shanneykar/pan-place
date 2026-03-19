use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{api::AppState, error::PanError};

pub async fn get_actor_events(
    State(state): State<AppState>,
    Path(actor_id): Path<String>,
) -> Result<(StatusCode, Json<Value>), PanError> {
    // Verify actor exists.
    if !state.store.actor_exists(&actor_id).await? {
        return Err(PanError::ActorNotFound(actor_id));
    }

    let events = state.store.get_events_for_actor(&actor_id).await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "actor_id": actor_id, "events": events })),
    ))
}

#[derive(Deserialize)]
pub struct NodeEventsQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub event_type: Option<String>,
}

pub async fn get_node_events(
    State(state): State<AppState>,
    Path(pan_id): Path<String>,
    Query(params): Query<NodeEventsQuery>,
) -> Result<(StatusCode, Json<Value>), PanError> {
    // Verify node exists.
    if !state.store.node_exists(&pan_id).await? {
        return Err(PanError::NodeNotFound(pan_id));
    }

    let events = state
        .store
        .get_events_for_node(
            &pan_id,
            params.from,
            params.to,
            params.event_type.as_deref(),
        )
        .await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "pan_id": pan_id, "events": events })),
    ))
}
