use anyhow::Result;

use pan::crypto::{hash_event, sign, HashInput};
use pan::types::EventType;

use crate::client::PanClient;
use crate::identity::{load_identity, load_last_event, save_last_event, signing_key_from_identity};

pub async fn record(pan_id: &str) -> Result<()> {
    let identity = load_identity()?;
    let signing_key = signing_key_from_identity(&identity)?;

    let ts = chrono::Utc::now().timestamp_millis();
    let content = format!("Presence at node {}", pan_id);
    let parent_hashes: Vec<String> = load_last_event().into_iter().collect();

    let input = HashInput {
        entity_id: &identity.actor_id,
        event_type: EventType::PresenceRecorded,
        timestamp: ts,
        content: &content,
        tags: &[],
        parent_hashes: &parent_hashes,
        references_event: None,
    };
    let event_id = hash_event(&input);

    let id_bytes: [u8; 32] = hex::decode(&event_id)?.try_into()
        .map_err(|_| anyhow::anyhow!("event_id wrong length"))?;
    let signature = sign(&signing_key, &id_bytes);

    let body = serde_json::json!({
        "event_id": event_id,
        "entity_id": identity.actor_id,
        "event_type": "presence_recorded",
        "timestamp": ts,
        "content": content,
        "tags": [],
        "parent_hashes": parent_hashes,
        "references_event": null,
        "signature": signature,
        "actor_id": identity.actor_id
    });

    let client = PanClient::new(&identity.server);
    let resp = client.post("/events", &body).await?;

    let returned_id = resp["event_id"].as_str().unwrap_or(&event_id).to_string();
    save_last_event(&returned_id)?;

    println!("Presence recorded. Event: {}", returned_id);
    Ok(())
}
