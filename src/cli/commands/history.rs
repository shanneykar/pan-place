use anyhow::Result;
use chrono::{TimeZone, Utc};
use serde_json::Value;

use crate::client::PanClient;
use crate::identity::load_identity;

pub async fn actor(id: Option<&str>) -> Result<()> {
    let identity = load_identity()?;
    let actor_id = id.unwrap_or(&identity.actor_id);

    let client = PanClient::new(&identity.server);
    let resp = client.get(&format!("/actors/{}/events", actor_id)).await?;

    let events = resp["events"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected response from server"))?;

    print_event_table(events);
    Ok(())
}

pub async fn node(
    pan_id: &str,
    from: Option<i64>,
    to: Option<i64>,
    event_type: Option<&str>,
) -> Result<()> {
    let identity = load_identity()?;
    let client = PanClient::new(&identity.server);

    let mut path = format!("/nodes/{}/events", pan_id);
    let mut sep = '?';
    if let Some(f) = from {
        path.push_str(&format!("{}from={}", sep, f));
        sep = '&';
    }
    if let Some(t) = to {
        path.push_str(&format!("{}to={}", sep, t));
        sep = '&';
    }
    if let Some(et) = event_type {
        path.push_str(&format!("{}event_type={}", sep, et));
    }

    let resp = client.get(&path).await?;

    let events = resp["events"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected response from server"))?;

    print_event_table(events);
    Ok(())
}

fn print_event_table(events: &[Value]) {
    if events.is_empty() {
        println!("(no events)");
        return;
    }

    println!(
        "{:<20} {:<22} {:<40} {:<16} {}",
        "TIMESTAMP", "TYPE", "CONTENT", "TAGS", "REFS"
    );
    println!("{}", "-".repeat(110));

    for event in events {
        let ts_ms = event["timestamp"].as_i64().unwrap_or(0);
        let ts_str = Utc
            .timestamp_millis_opt(ts_ms)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "?".to_string());

        let event_type = event["event_type"].as_str().unwrap_or("?");

        let content_raw = event["content"].as_str().unwrap_or("");
        let content = if content_raw.chars().count() > 40 {
            format!("{}…", content_raw.chars().take(39).collect::<String>())
        } else {
            content_raw.to_string()
        };

        let tags_str = event["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();

        let refs_str = event["references_event"]
            .as_str()
            .map(|r| r.chars().take(8).collect::<String>())
            .unwrap_or_else(|| "—".to_string());

        println!(
            "{:<20} {:<22} {:<40} {:<16} {}",
            ts_str, event_type, content, tags_str, refs_str
        );
    }
}
