/// Storage integration test: round-trip for Actor, PanNode, and Event through
/// both JSONL (source-of-truth) and SQLite (index), verifying they match.
use pan::{
    crypto::{actor_id_from_pubkey, derive_pan_id, generate_keypair, hash_event, HashInput},
    store::PanStore,
    types::{Event, EventType, NodeStatus, NodeType, PanNode},
};

// ── helpers ────────────────────────────────────────────────────────────────

fn fake_actor() -> (pan::types::Actor, ed25519_dalek::SigningKey) {
    let (signing_key, verifying_key) = generate_keypair();
    let pubkey: Vec<u8> = verifying_key.as_bytes().to_vec();
    let actor_id = actor_id_from_pubkey(&pubkey);
    let actor = pan::types::Actor {
        actor_id,
        pubkey,
        phone_dhash: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899".to_string(),
        created_at: 1_700_000_000_000,
    };
    (actor, signing_key)
}

fn actor_reg_event(actor: &pan::types::Actor) -> Event {
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
        signature: "00".repeat(64), // storage layer does not verify signatures
        actor_id: actor.actor_id.clone(),
    }
}

fn fake_node() -> PanNode {
    let lat = 35.6762_f64;
    let lon = 139.6503_f64;
    let placed_at = 1_700_000_001_000_i64;
    let pan_id = derive_pan_id(lat, lon, placed_at);
    PanNode {
        pan_id,
        lat,
        lon,
        radius_miles: 1.0,
        placed_at,
        node_type: NodeType::Fixed,
        status: NodeStatus::Active,
    }
}

fn node_placed_event(node: &PanNode, actor_id: &str) -> Event {
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
        signature: "00".repeat(64),
        actor_id: actor_id.to_string(),
    }
}

fn presence_event(actor: &pan::types::Actor, node: &PanNode, parent_id: &str) -> Event {
    let ts = node.placed_at + 5_000;
    let tags = vec!["outdoor".to_string(), "park".to_string()];
    let input = HashInput {
        entity_id: &node.pan_id,
        event_type: EventType::PresenceRecorded,
        timestamp: ts,
        content: "Checked in at Yoyogi Park",
        tags: &tags,
        parent_hashes: &[parent_id.to_string()],
        references_event: None,
    };
    let event_id = hash_event(&input);
    Event {
        event_id,
        entity_id: node.pan_id.clone(),
        event_type: EventType::PresenceRecorded,
        timestamp: ts,
        content: "Checked in at Yoyogi Park".to_string(),
        tags,
        parent_hashes: vec![parent_id.to_string()],
        references_event: None,
        signature: "00".repeat(64),
        actor_id: actor.actor_id.clone(),
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_actor_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let store = PanStore::new(tmp.path()).await.unwrap();

    let (actor, _sk) = fake_actor();
    let reg_event = actor_reg_event(&actor);

    store.write_actor(&actor, &reg_event).await.unwrap();

    // SQLite round-trip
    let from_db = store.get_actor(&actor.actor_id).await.unwrap();
    assert_eq!(from_db.actor_id, actor.actor_id);
    assert_eq!(from_db.pubkey, actor.pubkey);
    assert_eq!(from_db.phone_dhash, actor.phone_dhash);
    assert_eq!(from_db.created_at, actor.created_at);

    // JSONL round-trip
    let jsonl_events = store
        .read_actor_events_jsonl(&actor.actor_id)
        .await
        .unwrap();
    assert_eq!(jsonl_events.len(), 1);
    assert_eq!(jsonl_events[0], reg_event);

    // SQLite event round-trip
    let db_events = store.get_events_for_actor(&actor.actor_id).await.unwrap();
    assert_eq!(db_events.len(), 1);
    assert_eq!(db_events[0], reg_event);

    // JSONL == SQLite
    assert_eq!(jsonl_events[0], db_events[0]);
}

#[tokio::test]
async fn test_node_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let store = PanStore::new(tmp.path()).await.unwrap();

    // Need a registered actor to sign the placed event.
    let (actor, _sk) = fake_actor();
    let reg_event = actor_reg_event(&actor);
    store.write_actor(&actor, &reg_event).await.unwrap();

    let node = fake_node();
    let placed_event = node_placed_event(&node, &actor.actor_id);

    store.write_node(&node, &placed_event).await.unwrap();

    // SQLite round-trip
    let from_db = store.get_node(&node.pan_id).await.unwrap();
    assert_eq!(from_db.pan_id, node.pan_id);
    assert!((from_db.lat - node.lat).abs() < 1e-9);
    assert!((from_db.lon - node.lon).abs() < 1e-9);
    assert!((from_db.radius_miles - node.radius_miles).abs() < 1e-9);
    assert_eq!(from_db.placed_at, node.placed_at);
    assert_eq!(from_db.node_type, node.node_type);
    assert_eq!(from_db.status, node.status);

    // JSONL round-trip
    let jsonl_events = store.read_node_events_jsonl(&node.pan_id).await.unwrap();
    assert_eq!(jsonl_events.len(), 1);
    assert_eq!(jsonl_events[0], placed_event);

    // SQLite event round-trip
    let db_events = store
        .get_events_for_node(&node.pan_id, None, None, None)
        .await
        .unwrap();
    assert_eq!(db_events.len(), 1);
    assert_eq!(db_events[0], placed_event);

    // JSONL == SQLite
    assert_eq!(jsonl_events[0], db_events[0]);
}

#[tokio::test]
async fn test_event_round_trip_with_parent() {
    let tmp = tempfile::tempdir().unwrap();
    let store = PanStore::new(tmp.path()).await.unwrap();

    let (actor, _sk) = fake_actor();
    let reg_event = actor_reg_event(&actor);
    store.write_actor(&actor, &reg_event).await.unwrap();

    let node = fake_node();
    let placed_event = node_placed_event(&node, &actor.actor_id);
    store.write_node(&node, &placed_event).await.unwrap();

    // Presence event references the placed_event as parent.
    let presence = presence_event(&actor, &node, &placed_event.event_id);
    store.write_node_event(&presence).await.unwrap();

    // JSONL has both events for this node.
    let jsonl_events = store.read_node_events_jsonl(&node.pan_id).await.unwrap();
    assert_eq!(jsonl_events.len(), 2);
    assert_eq!(jsonl_events[1], presence);

    // SQLite events for this node.
    let db_events = store
        .get_events_for_node(&node.pan_id, None, None, None)
        .await
        .unwrap();
    assert_eq!(db_events.len(), 2);
    assert_eq!(db_events[1], presence);

    // Parent hashes survive the round-trip.
    assert_eq!(db_events[1].parent_hashes, vec![placed_event.event_id.clone()]);
    assert_eq!(jsonl_events[1].parent_hashes, vec![placed_event.event_id.clone()]);

    // Tags survive.
    assert_eq!(db_events[1].tags, vec!["outdoor", "park"]);

    // JSONL == SQLite for both events.
    assert_eq!(jsonl_events[0], db_events[0]);
    assert_eq!(jsonl_events[1], db_events[1]);

    // Existence checks.
    assert!(store.event_exists(&presence.event_id).await.unwrap());
    assert!(store.actor_exists(&actor.actor_id).await.unwrap());
    assert!(store.node_exists(&node.pan_id).await.unwrap());
    assert!(store
        .phone_dhash_exists(&actor.phone_dhash)
        .await
        .unwrap());

    // Timestamp query.
    let ts = store
        .get_event_timestamp(&placed_event.event_id)
        .await
        .unwrap();
    assert_eq!(ts, node.placed_at);
}

#[tokio::test]
async fn test_node_event_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let store = PanStore::new(tmp.path()).await.unwrap();

    let (actor, _sk) = fake_actor();
    let reg_event = actor_reg_event(&actor);
    store.write_actor(&actor, &reg_event).await.unwrap();

    let node = fake_node();
    let placed_event = node_placed_event(&node, &actor.actor_id);
    store.write_node(&node, &placed_event).await.unwrap();

    let presence = presence_event(&actor, &node, &placed_event.event_id);
    store.write_node_event(&presence).await.unwrap();

    // Filter by event_type.
    let only_placed = store
        .get_events_for_node(&node.pan_id, None, None, Some("pan_node_placed"))
        .await
        .unwrap();
    assert_eq!(only_placed.len(), 1);
    assert_eq!(only_placed[0].event_type, EventType::PanNodePlaced);

    // Filter by `from` timestamp — exclude the placed event.
    let after = store
        .get_events_for_node(&node.pan_id, Some(node.placed_at + 1), None, None)
        .await
        .unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].event_type, EventType::PresenceRecorded);

    // Filter by `to` timestamp — exclude the presence event.
    let before = store
        .get_events_for_node(&node.pan_id, None, Some(node.placed_at), None)
        .await
        .unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].event_type, EventType::PanNodePlaced);
}
