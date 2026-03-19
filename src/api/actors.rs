use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    api::{now_ms, AppState},
    crypto::{actor_id_from_pubkey, hash_event, sign, verify, HashInput},
    error::PanError,
    types::{Actor, Event, EventType},
};

#[derive(Deserialize)]
pub struct PostActorRequest {
    pub actor_id: String,
    /// Hex-encoded raw 32-byte Ed25519 public key.
    pub pubkey: String,
    pub phone_dhash: String,
    /// Ed25519 signature over hex_decode(actor_id) — proves key ownership.
    pub signature: String,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct PostActorResponse {
    pub actor_id: String,
    pub created_at: i64,
}

pub async fn post_actor(
    State(state): State<AppState>,
    Json(body): Json<PostActorRequest>,
) -> Result<(StatusCode, Json<Value>), PanError> {
    // 1. Decode pubkey hex.
    let pubkey_bytes =
        hex::decode(&body.pubkey).map_err(|_| PanError::InvalidHash)?;
    if pubkey_bytes.len() != 32 {
        return Err(PanError::InvalidHash);
    }

    // 2. Verify actor_id derivation.
    let computed_actor_id = actor_id_from_pubkey(&pubkey_bytes);
    if computed_actor_id != body.actor_id {
        return Err(PanError::HashMismatch {
            computed: computed_actor_id,
            submitted: body.actor_id.clone(),
        });
    }

    // 3. phone_dhash not already registered.
    if state.store.phone_dhash_exists(&body.phone_dhash).await? {
        return Err(PanError::PhoneDhashAlreadyRegistered);
    }

    // 4. Verify signature: Ed25519_Verify(pubkey, hex_decode(actor_id), sig).
    let actor_id_bytes =
        hex::decode(&body.actor_id).map_err(|_| PanError::InvalidHash)?;
    verify(&pubkey_bytes, &actor_id_bytes, &body.signature)?;

    // 5. created_at plausibility: > 2020-01-01, < now + 5 minutes.
    const MIN_TS: i64 = 1_577_836_800_000; // 2020-01-01 UTC in ms
    let max_ts = now_ms() + 5 * 60 * 1_000;
    if body.created_at < MIN_TS || body.created_at > max_ts {
        return Err(PanError::StorageError(
            "created_at out of plausible range".to_string(),
        ));
    }

    // 6. actor_id not already registered.
    if state.store.actor_exists(&body.actor_id).await? {
        return Err(PanError::ActorAlreadyExists(body.actor_id.clone()));
    }

    let actor = Actor {
        actor_id: body.actor_id.clone(),
        pubkey: pubkey_bytes,
        phone_dhash: body.phone_dhash.clone(),
        created_at: body.created_at,
    };

    // Build the ActorRegistered event. The signature field reuses the
    // registration signature (the server has no private key).
    let reg_event = build_actor_reg_event(&actor, &body.signature);

    state.store.write_actor(&actor, &reg_event).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "actor_id": body.actor_id, "created_at": body.created_at })),
    ))
}

pub(crate) fn build_actor_reg_event(actor: &Actor, signature: &str) -> Event {
    let input = HashInput {
        entity_id: &actor.actor_id,
        event_type: EventType::ActorRegistered,
        timestamp: actor.created_at,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let event_id = hash_event(&input);
    Event {
        event_id,
        entity_id: actor.actor_id.clone(),
        event_type: EventType::ActorRegistered,
        timestamp: actor.created_at,
        content: String::new(),
        tags: vec![],
        parent_hashes: vec![],
        references_event: None,
        signature: signature.to_string(),
        actor_id: actor.actor_id.clone(),
    }
}

/// Sign the actor_id bytes with a signing key — used in tests.
pub fn sign_actor_id(
    signing_key: &ed25519_dalek::SigningKey,
    actor_id_hex: &str,
) -> Result<String, PanError> {
    let bytes = hex::decode(actor_id_hex).map_err(|_| PanError::InvalidHash)?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| PanError::InvalidHash)?;
    Ok(sign(signing_key, &arr))
}
