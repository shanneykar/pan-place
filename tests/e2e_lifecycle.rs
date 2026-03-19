/// End-to-end lifecycle test for PAN layer-0.
///
/// Flow:
///   1. Register Actor 1 (valid keypair + signature)
///   2. Register Actor 2 (valid keypair + signature)
///   3. Actor 1 places a PAN node
///   4. Actor 1 records presence at that node
///   5. Actor 2 confirms Actor 1's presence
///   6. Query actor histories and verify
///   7. Query node history and verify
///   8. Error cases: invalid sig, duplicate, hash mismatch, missing parent
use std::sync::Arc;

use axum::http::StatusCode;
use axum_test::TestServer;
use ed25519_dalek::SigningKey;
use serde_json::{json, Value};

use pan::{
    api,
    crypto::{
        actor_id_from_pubkey, derive_pan_id, generate_keypair, hash_event, hash_node_placement,
        sign, HashInput,
    },
    store::PanStore,
    types::EventType,
};

// ── test harness ─────────────────────────────────────────────────────────────

async fn setup() -> (TestServer, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let store = PanStore::new(tmp.path()).await.unwrap();
    let app = api::router(Arc::new(store));
    let server = TestServer::new(app).unwrap();
    (server, tmp)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Register an actor and return (actor_id, signing_key).
async fn register_actor(server: &TestServer, phone_suffix: &str) -> (String, SigningKey) {
    let (sk, vk) = generate_keypair();
    let pubkey_bytes = vk.as_bytes().to_vec();
    let pubkey_hex = hex::encode(&pubkey_bytes);
    let actor_id = actor_id_from_pubkey(&pubkey_bytes);

    // Sign actor_id raw bytes.
    let id_bytes: [u8; 32] = hex::decode(&actor_id).unwrap().try_into().unwrap();
    let signature = sign(&sk, &id_bytes);

    let phone_dhash = format!("aabb{:0>60}", phone_suffix);

    let resp = server
        .post("/actors")
        .json(&json!({
            "actor_id": actor_id,
            "pubkey": pubkey_hex,
            "phone_dhash": phone_dhash,
            "signature": signature,
            "created_at": now_ms()
        }))
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json();
    assert_eq!(body["actor_id"], actor_id);

    (actor_id, sk)
}

/// Build and sign a generic event, returning the full JSON body for POST /events.
fn build_event_body(
    sk: &SigningKey,
    actor_id: &str,
    entity_id: &str,
    event_type: EventType,
    timestamp: i64,
    content: &str,
    tags: Vec<String>,
    parent_hashes: Vec<String>,
    references_event: Option<String>,
) -> Value {
    let input = HashInput {
        entity_id,
        event_type,
        timestamp,
        content,
        tags: &tags,
        parent_hashes: &parent_hashes,
        references_event: references_event.as_deref(),
    };
    let event_id = hash_event(&input);
    let id_bytes: [u8; 32] = hex::decode(&event_id).unwrap().try_into().unwrap();
    let signature = sign(sk, &id_bytes);

    json!({
        "event_id": event_id,
        "entity_id": entity_id,
        "event_type": event_type,
        "timestamp": timestamp,
        "content": content,
        "tags": tags,
        "parent_hashes": parent_hashes,
        "references_event": references_event,
        "signature": signature,
        "actor_id": actor_id
    })
}

// ── happy path ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_full_lifecycle() {
    let (server, _tmp) = setup().await;

    // ── 1. Register Actor 1 ────────────────────────────────────────────────
    let (actor1_id, sk1) = register_actor(&server, "1").await;

    // ── 2. Register Actor 2 ────────────────────────────────────────────────
    let (actor2_id, sk2) = register_actor(&server, "2").await;

    // ── 3. Actor 1 places a PAN node ───────────────────────────────────────
    let lat = 35.6762_f64;
    let lon = 139.6503_f64;
    let placed_at = now_ms();
    let node_hash = hash_node_placement(lat, lon, placed_at);
    let node_sig = sign(&sk1, &node_hash);
    let pan_id = derive_pan_id(lat, lon, placed_at);

    let resp = server
        .post("/nodes")
        .json(&json!({
            "lat": lat,
            "lon": lon,
            "radius_miles": 1.0,
            "node_type": "fixed",
            "actor_id": actor1_id,
            "placed_at": placed_at,
            "signature": node_sig
        }))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json();
    assert_eq!(body["pan_id"], pan_id);
    assert_eq!(body["placed_at"], placed_at);

    // Fetch the PanNodePlaced event_id for use as parent.
    let node_events_resp = server
        .get(&format!("/nodes/{}/events", pan_id))
        .await;
    node_events_resp.assert_status_ok();
    let ne: Value = node_events_resp.json();
    let placed_event_id = ne["events"][0]["event_id"].as_str().unwrap().to_string();
    assert_eq!(ne["events"][0]["event_type"], "pan_node_placed");

    // ── 4. Actor 1 records presence at the node ────────────────────────────
    let presence_ts = placed_at + 1_000;
    let presence_body = build_event_body(
        &sk1,
        &actor1_id,
        &pan_id,
        EventType::PresenceRecorded,
        presence_ts,
        "Actor 1 arrived at Yoyogi Park",
        vec!["outdoor".to_string()],
        vec![placed_event_id.clone()],
        None,
    );
    let resp = server.post("/events").json(&presence_body).await;
    resp.assert_status(StatusCode::CREATED);
    let presence_resp: Value = resp.json();
    assert_eq!(presence_resp["status"], "written");
    let presence_event_id = presence_resp["event_id"].as_str().unwrap().to_string();

    // ── 5. Actor 2 confirms Actor 1's presence ─────────────────────────────
    // Confirmation entity is the node's DAG.
    let confirm_ts = presence_ts + 2_000;
    let confirm_body = build_event_body(
        &sk2,
        &actor2_id,
        &pan_id,
        EventType::ConfirmationRecorded,
        confirm_ts,
        "Actor 2 confirms Actor 1 was here",
        vec![],
        vec![presence_event_id.clone()],
        Some(presence_event_id.clone()),
    );
    let resp = server.post("/events").json(&confirm_body).await;
    resp.assert_status(StatusCode::CREATED);
    let confirm_resp: Value = resp.json();
    assert_eq!(confirm_resp["status"], "written");
    let confirm_event_id = confirm_resp["event_id"].as_str().unwrap().to_string();

    // ── 6. Query Actor 1 history ───────────────────────────────────────────
    // Should include: ActorRegistered, PanNodePlaced, PresenceRecorded.
    let resp = server
        .get(&format!("/actors/{}/events", actor1_id))
        .await;
    resp.assert_status_ok();
    let actor1_events: Value = resp.json();
    assert_eq!(actor1_events["actor_id"], actor1_id);
    let a1_evts = actor1_events["events"].as_array().unwrap();
    assert_eq!(a1_evts.len(), 3, "Actor 1 should have 3 events");
    let a1_types: Vec<&str> = a1_evts
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(a1_types.contains(&"actor_registered"));
    assert!(a1_types.contains(&"pan_node_placed"));
    assert!(a1_types.contains(&"presence_recorded"));

    // ── 7. Query Actor 2 history ───────────────────────────────────────────
    // Should include: ActorRegistered, ConfirmationRecorded.
    let resp = server
        .get(&format!("/actors/{}/events", actor2_id))
        .await;
    resp.assert_status_ok();
    let actor2_events: Value = resp.json();
    let a2_evts = actor2_events["events"].as_array().unwrap();
    assert_eq!(a2_evts.len(), 2, "Actor 2 should have 2 events");
    let a2_types: Vec<&str> = a2_evts
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(a2_types.contains(&"actor_registered"));
    assert!(a2_types.contains(&"confirmation_recorded"));

    // ── 8. Query node history ──────────────────────────────────────────────
    // Should include: PanNodePlaced, PresenceRecorded, ConfirmationRecorded.
    let resp = server
        .get(&format!("/nodes/{}/events", pan_id))
        .await;
    resp.assert_status_ok();
    let node_events: Value = resp.json();
    assert_eq!(node_events["pan_id"], pan_id);
    let n_evts = node_events["events"].as_array().unwrap();
    assert_eq!(n_evts.len(), 3, "Node should have 3 events");
    let n_types: Vec<&str> = n_evts
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(n_types.contains(&"pan_node_placed"));
    assert!(n_types.contains(&"presence_recorded"));
    assert!(n_types.contains(&"confirmation_recorded"));

    // Verify confirmation event fields.
    let conf_evt = n_evts
        .iter()
        .find(|e| e["event_type"] == "confirmation_recorded")
        .unwrap();
    assert_eq!(conf_evt["event_id"], confirm_event_id);
    assert_eq!(conf_evt["actor_id"], actor2_id);
    assert_eq!(conf_evt["references_event"], presence_event_id);

    // ── 9. Node event filter: by event_type ───────────────────────────────
    let resp = server
        .get(&format!("/nodes/{}/events", pan_id))
        .add_query_param("event_type", "pan_node_placed")
        .await;
    resp.assert_status_ok();
    let filtered: Value = resp.json();
    assert_eq!(
        filtered["events"].as_array().unwrap().len(),
        1,
        "event_type filter should return 1 event"
    );

    // ── 10. Node event filter: from/to ─────────────────────────────────────
    let resp = server
        .get(&format!("/nodes/{}/events", pan_id))
        .add_query_param("from", presence_ts)
        .add_query_param("to", presence_ts)
        .await;
    resp.assert_status_ok();
    let range_filtered: Value = resp.json();
    assert_eq!(
        range_filtered["events"].as_array().unwrap().len(),
        1,
        "from/to filter should return 1 event"
    );
    assert_eq!(
        range_filtered["events"][0]["event_type"],
        "presence_recorded"
    );
}

// ── error cases ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_invalid_signature_on_actor_registration() {
    let (server, _tmp) = setup().await;

    let (sk, vk) = generate_keypair();
    let pubkey_hex = hex::encode(vk.as_bytes());
    let actor_id = actor_id_from_pubkey(vk.as_bytes());

    // Sign with a DIFFERENT key — wrong signature.
    let (wrong_sk, _) = generate_keypair();
    let id_bytes: [u8; 32] = hex::decode(&actor_id).unwrap().try_into().unwrap();
    let bad_sig = sign(&wrong_sk, &id_bytes);

    let resp = server
        .post("/actors")
        .json(&json!({
            "actor_id": actor_id,
            "pubkey": pubkey_hex,
            "phone_dhash": "cc00000000000000000000000000000000000000000000000000000000000001",
            "signature": bad_sig,
            "created_at": now_ms()
        }))
        .await;

    resp.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = resp.json();
    assert_eq!(body["error"], "invalid_signature");

    // Silence the unused variable warning.
    let _ = sk;
}

#[tokio::test]
async fn test_duplicate_event_returns_200() {
    let (server, _tmp) = setup().await;
    let (actor_id, sk) = register_actor(&server, "dup1").await;

    // Place a node to have a parent event.
    let lat = 10.0_f64;
    let lon = 20.0_f64;
    let placed_at = now_ms();
    let node_sig = sign(&sk, &hash_node_placement(lat, lon, placed_at));
    let pan_id = derive_pan_id(lat, lon, placed_at);

    server
        .post("/nodes")
        .json(&json!({
            "lat": lat, "lon": lon, "radius_miles": 1.0, "node_type": "fixed",
            "actor_id": actor_id, "placed_at": placed_at, "signature": node_sig
        }))
        .await
        .assert_status(StatusCode::CREATED);

    // Get PanNodePlaced event_id.
    let n_resp: Value = server
        .get(&format!("/nodes/{}/events", pan_id))
        .await
        .json();
    let parent_id = n_resp["events"][0]["event_id"].as_str().unwrap().to_string();

    let ts = placed_at + 500;
    let evt_body = build_event_body(
        &sk, &actor_id, &pan_id, EventType::PresenceRecorded,
        ts, "first presence", vec![], vec![parent_id], None,
    );

    // First submission → 201.
    let r1 = server.post("/events").json(&evt_body).await;
    r1.assert_status(StatusCode::CREATED);
    assert_eq!(r1.json::<Value>()["status"], "written");

    // Second submission (identical) → 200 duplicate.
    let r2 = server.post("/events").json(&evt_body).await;
    r2.assert_status(StatusCode::OK);
    let r2_body: Value = r2.json();
    assert_eq!(r2_body["status"], "duplicate");
}

#[tokio::test]
async fn test_hash_mismatch_returns_400() {
    let (server, _tmp) = setup().await;
    let (actor_id, sk) = register_actor(&server, "hash1").await;

    let lat = 1.0_f64;
    let lon = 2.0_f64;
    let placed_at = now_ms();
    server
        .post("/nodes")
        .json(&json!({
            "lat": lat, "lon": lon, "radius_miles": 1.0, "node_type": "fixed",
            "actor_id": actor_id, "placed_at": placed_at,
            "signature": sign(&sk, &hash_node_placement(lat, lon, placed_at))
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let pan_id = derive_pan_id(lat, lon, placed_at);
    let n_resp: Value = server
        .get(&format!("/nodes/{}/events", pan_id))
        .await
        .json();
    let parent_id = n_resp["events"][0]["event_id"].as_str().unwrap().to_string();

    let ts = placed_at + 1_000;
    let real_event_id = hash_event(&HashInput {
        entity_id: &pan_id,
        event_type: EventType::PresenceRecorded,
        timestamp: ts,
        content: "hello",
        tags: &[],
        parent_hashes: &[parent_id.clone()],
        references_event: None,
    });

    // Sign the real event_id so sig check passes, but submit wrong event_id.
    let id_bytes: [u8; 32] = hex::decode(&real_event_id).unwrap().try_into().unwrap();
    let signature = sign(&sk, &id_bytes);

    let wrong_event_id = "0".repeat(64);

    let resp = server
        .post("/events")
        .json(&json!({
            "event_id": wrong_event_id,
            "entity_id": pan_id,
            "event_type": "presence_recorded",
            "timestamp": ts,
            "content": "hello",
            "tags": [],
            "parent_hashes": [parent_id],
            "references_event": null,
            "signature": signature,
            "actor_id": actor_id
        }))
        .await;

    resp.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(resp.json::<Value>()["error"], "hash_mismatch");
}

#[tokio::test]
async fn test_missing_parent_returns_400() {
    let (server, _tmp) = setup().await;
    let (actor_id, sk) = register_actor(&server, "par1").await;

    let lat = 5.0_f64;
    let lon = 6.0_f64;
    let placed_at = now_ms();
    server
        .post("/nodes")
        .json(&json!({
            "lat": lat, "lon": lon, "radius_miles": 1.0, "node_type": "fixed",
            "actor_id": actor_id, "placed_at": placed_at,
            "signature": sign(&sk, &hash_node_placement(lat, lon, placed_at))
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let pan_id = derive_pan_id(lat, lon, placed_at);
    let nonexistent_parent = "d".repeat(64);
    let ts = placed_at + 1_000;

    let evt_body = build_event_body(
        &sk, &actor_id, &pan_id, EventType::PresenceRecorded,
        ts, "presence with bad parent",
        vec![], vec![nonexistent_parent], None,
    );

    let resp = server.post("/events").json(&evt_body).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(resp.json::<Value>()["error"], "parent_not_found");
}

#[tokio::test]
async fn test_unknown_actor_returns_404() {
    let (server, _tmp) = setup().await;

    let resp = server
        .get("/actors/0000000000000000000000000000000000000000000000000000000000000000/events")
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(resp.json::<Value>()["error"], "actor_not_found");
}

#[tokio::test]
async fn test_unknown_node_returns_404() {
    let (server, _tmp) = setup().await;

    let resp = server.get("/nodes/0000000000000000/events").await;
    resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(resp.json::<Value>()["error"], "node_not_found");
}

#[tokio::test]
async fn test_duplicate_phone_dhash_returns_409() {
    let (server, _tmp) = setup().await;

    let (sk1, vk1) = generate_keypair();
    let actor1_id = actor_id_from_pubkey(vk1.as_bytes());
    let id_bytes: [u8; 32] = hex::decode(&actor1_id).unwrap().try_into().unwrap();
    let sig1 = sign(&sk1, &id_bytes);

    let shared_dhash = "ff00000000000000000000000000000000000000000000000000000000000099";

    // First registration succeeds.
    server
        .post("/actors")
        .json(&json!({
            "actor_id": actor1_id,
            "pubkey": hex::encode(vk1.as_bytes()),
            "phone_dhash": shared_dhash,
            "signature": sig1,
            "created_at": now_ms()
        }))
        .await
        .assert_status(StatusCode::CREATED);

    // Second registration with same phone_dhash → 409.
    let (sk2, vk2) = generate_keypair();
    let actor2_id = actor_id_from_pubkey(vk2.as_bytes());
    let id_bytes2: [u8; 32] = hex::decode(&actor2_id).unwrap().try_into().unwrap();
    let sig2 = sign(&sk2, &id_bytes2);

    let resp = server
        .post("/actors")
        .json(&json!({
            "actor_id": actor2_id,
            "pubkey": hex::encode(vk2.as_bytes()),
            "phone_dhash": shared_dhash,
            "signature": sig2,
            "created_at": now_ms()
        }))
        .await;

    resp.assert_status(StatusCode::CONFLICT);
    assert_eq!(resp.json::<Value>()["error"], "phone_dhash_already_registered");
}
