use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::{
    api::AppState,
    crypto::{hash_event, verify, HashInput},
    error::PanError,
    types::{Event, EventType},
};

pub async fn post_event(
    State(state): State<AppState>,
    Json(body): Json<Event>,
) -> Result<(StatusCode, Json<Value>), PanError> {
    // 1. actor_id must exist; fetch to get pubkey for sig verification.
    let actor = state.store.get_actor(&body.actor_id).await?;

    // 2. entity_id must exist in actors OR pan_nodes.
    let is_node = state.store.node_exists(&body.entity_id).await?;
    let is_actor = if !is_node {
        state.store.actor_exists(&body.entity_id).await?
    } else {
        false
    };
    if !is_node && !is_actor {
        return Err(PanError::ActorNotFound(format!(
            "entity not found: {}",
            body.entity_id
        )));
    }

    // 3. All parent_hashes must exist.
    for parent in &body.parent_hashes {
        if !state.store.event_exists(parent).await? {
            return Err(PanError::ParentNotFound(parent.clone()));
        }
    }

    // 4. Max 1 parent in layer-0.
    if body.parent_hashes.len() > 1 {
        return Err(PanError::TooManyParents);
    }

    // 10. Tags validation (done before hash for fast rejection of clearly invalid input).
    if body.tags.len() > 10 {
        return Err(PanError::InvalidTag("more than 10 tags".to_string()));
    }
    for tag in &body.tags {
        if tag.is_empty() || tag.chars().count() > 50 {
            return Err(PanError::InvalidTag(tag.clone()));
        }
    }

    // 11. Content length.
    if body.content.chars().count() > 2000 {
        return Err(PanError::ContentTooLong);
    }

    // 5. Recompute event_id — must match submitted.
    let computed = hash_event(&HashInput {
        entity_id: &body.entity_id,
        event_type: body.event_type,
        timestamp: body.timestamp,
        content: &body.content,
        tags: &body.tags,
        parent_hashes: &body.parent_hashes,
        references_event: body.references_event.as_deref(),
    });
    if computed != body.event_id {
        return Err(PanError::HashMismatch {
            computed,
            submitted: body.event_id.clone(),
        });
    }

    // 6. Signature valid over raw bytes of event_id hash.
    let event_id_bytes =
        hex::decode(&body.event_id).map_err(|_| PanError::InvalidHash)?;
    verify(&actor.pubkey, &event_id_bytes, &body.signature)?;

    // 7. timestamp > all parent timestamps.
    for parent in &body.parent_hashes {
        let parent_ts = state.store.get_event_timestamp(parent).await?;
        if body.timestamp <= parent_ts {
            return Err(PanError::TimestampNotForward);
        }
    }

    // 8. ConfirmationRecorded: references_event must be set and exist.
    if body.event_type == EventType::ConfirmationRecorded {
        match &body.references_event {
            None => return Err(PanError::MissingReference),
            Some(ref_id) => {
                if !state.store.event_exists(ref_id).await? {
                    return Err(PanError::ReferenceNotFound(ref_id.clone()));
                }
            }
        }
    }

    // 9. PresenceRecorded: entity_id must be a pan_id.
    if body.event_type == EventType::PresenceRecorded && !is_node {
        return Err(PanError::InvalidCoordinates);
    }

    // Duplicate: already exists → 200 idempotent.
    if state.store.event_exists(&body.event_id).await? {
        return Ok((
            StatusCode::OK,
            Json(json!({ "event_id": body.event_id, "status": "duplicate" })),
        ));
    }

    // Write.
    if is_node {
        state.store.write_node_event(&body).await?;
    } else {
        state.store.write_actor_event(&body).await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({ "event_id": body.event_id, "status": "written" })),
    ))
}
