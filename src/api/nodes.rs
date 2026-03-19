use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    api::{now_ms, AppState},
    crypto::{derive_pan_id, hash_event, hash_node_placement, verify, HashInput},
    error::PanError,
    types::{Event, EventType, NodeStatus, NodeType, PanNode},
};

#[derive(Deserialize)]
pub struct PostNodeRequest {
    pub lat: f64,
    pub lon: f64,
    #[serde(default = "default_radius")]
    pub radius_miles: f64,
    #[serde(default = "default_node_type")]
    pub node_type: NodeType,
    pub actor_id: String,
    /// Client-provided placement timestamp — must be included so the
    /// client can compute the signature before sending the request.
    pub placed_at: i64,
    /// Ed25519 sig over blake3(lat_f64_be || lon_f64_be || placed_at_i64_be).
    pub signature: String,
}

fn default_radius() -> f64 {
    1.0
}
fn default_node_type() -> NodeType {
    NodeType::Fixed
}

pub async fn post_node(
    State(state): State<AppState>,
    Json(body): Json<PostNodeRequest>,
) -> Result<(StatusCode, Json<Value>), PanError> {
    // 1. actor_id must exist.
    let actor = state.store.get_actor(&body.actor_id).await?;

    // 2. Coordinate bounds.
    if !(-90.0..=90.0).contains(&body.lat) || !(-180.0..=180.0).contains(&body.lon) {
        return Err(PanError::InvalidCoordinates);
    }

    // 3. placed_at plausibility (within 5 minutes of now, and after 2020).
    const MIN_TS: i64 = 1_577_836_800_000;
    let max_ts = now_ms() + 5 * 60 * 1_000;
    if body.placed_at < MIN_TS || body.placed_at > max_ts {
        return Err(PanError::StorageError(
            "placed_at out of plausible range".to_string(),
        ));
    }

    // 4. Verify signature over blake3(lat_f64_be || lon_f64_be || placed_at_i64_be).
    let hash_bytes = hash_node_placement(body.lat, body.lon, body.placed_at);
    verify(&actor.pubkey, &hash_bytes, &body.signature)?;

    // 5. Derive pan_id.
    let pan_id = derive_pan_id(body.lat, body.lon, body.placed_at);

    let node = PanNode {
        pan_id: pan_id.clone(),
        lat: body.lat,
        lon: body.lon,
        radius_miles: body.radius_miles,
        placed_at: body.placed_at,
        node_type: body.node_type,
        status: NodeStatus::Active,
    };

    let placed_event = build_node_placed_event(&node, &body.actor_id, &body.signature);

    state.store.write_node(&node, &placed_event).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "pan_id": pan_id, "placed_at": body.placed_at })),
    ))
}

pub(crate) fn build_node_placed_event(node: &PanNode, actor_id: &str, signature: &str) -> Event {
    let input = HashInput {
        entity_id: &node.pan_id,
        event_type: EventType::PanNodePlaced,
        timestamp: node.placed_at,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let event_id = hash_event(&input);
    Event {
        event_id,
        entity_id: node.pan_id.clone(),
        event_type: EventType::PanNodePlaced,
        timestamp: node.placed_at,
        content: String::new(),
        tags: vec![],
        parent_hashes: vec![],
        references_event: None,
        signature: signature.to_string(),
        actor_id: actor_id.to_string(),
    }
}
