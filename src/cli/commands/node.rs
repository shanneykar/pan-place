use anyhow::Result;

use pan::crypto::{hash_node_placement, sign};

use crate::client::PanClient;
use crate::identity::{load_identity, signing_key_from_identity};

pub async fn place(lat: f64, lon: f64, radius: f64, node_type: &str) -> Result<()> {
    let identity = load_identity()?;
    let signing_key = signing_key_from_identity(&identity)?;

    let placed_at = chrono::Utc::now().timestamp_millis();
    let hash_bytes = hash_node_placement(lat, lon, placed_at);
    let signature = sign(&signing_key, &hash_bytes);

    let client = PanClient::new(&identity.server);
    let body = serde_json::json!({
        "lat": lat,
        "lon": lon,
        "radius_miles": radius,
        "node_type": node_type,
        "actor_id": identity.actor_id,
        "placed_at": placed_at,
        "signature": signature
    });

    let resp = client.post("/nodes", &body).await?;
    let pan_id = resp["pan_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("server did not return pan_id"))?;

    println!("Node placed. PAN ID: {}", pan_id);
    Ok(())
}
