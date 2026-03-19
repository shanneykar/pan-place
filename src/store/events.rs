use sqlx::{Row, SqlitePool};

use crate::error::PanError;
use crate::types::{Event, EventType};

pub async fn insert_event(pool: &SqlitePool, event: &Event) -> Result<(), PanError> {
    let tags_json =
        serde_json::to_string(&event.tags).map_err(|e| PanError::StorageError(e.to_string()))?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO events \
         (event_id, entity_id, event_type, timestamp, content, tags, references_event, signature, actor_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&event.event_id)
    .bind(&event.entity_id)
    .bind(event.event_type.as_str())
    .bind(event.timestamp)
    .bind(&event.content)
    .bind(&tags_json)
    .bind(&event.references_event)
    .bind(&event.signature)
    .bind(&event.actor_id)
    .execute(&mut *tx)
    .await?;

    for parent_hash in &event.parent_hashes {
        sqlx::query(
            "INSERT INTO event_parents (event_id, parent_hash) VALUES (?, ?)",
        )
        .bind(&event.event_id)
        .bind(parent_hash)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn event_exists(pool: &SqlitePool, event_id: &str) -> Result<bool, PanError> {
    let row = sqlx::query("SELECT 1 FROM events WHERE event_id = ?")
        .bind(event_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

/// Events where entity_id OR actor_id matches — full actor history.
pub async fn get_events_for_actor(
    pool: &SqlitePool,
    actor_id: &str,
) -> Result<Vec<Event>, PanError> {
    let rows = sqlx::query(
        "SELECT event_id, entity_id, event_type, timestamp, content, tags, \
                references_event, signature, actor_id \
         FROM events WHERE entity_id = ? OR actor_id = ? \
         ORDER BY timestamp ASC",
    )
    .bind(actor_id)
    .bind(actor_id)
    .fetch_all(pool)
    .await?;

    assemble_events(pool, rows).await
}

/// Events where entity_id matches, with optional time/type filters — node history.
pub async fn get_events_for_node(
    pool: &SqlitePool,
    pan_id: &str,
    from: Option<i64>,
    to: Option<i64>,
    event_type: Option<&str>,
) -> Result<Vec<Event>, PanError> {
    // Build the query dynamically based on which filters are present.
    let mut sql = String::from(
        "SELECT event_id, entity_id, event_type, timestamp, content, tags, \
                references_event, signature, actor_id \
         FROM events WHERE entity_id = ?",
    );
    if from.is_some() {
        sql.push_str(" AND timestamp >= ?");
    }
    if to.is_some() {
        sql.push_str(" AND timestamp <= ?");
    }
    if event_type.is_some() {
        sql.push_str(" AND event_type = ?");
    }
    sql.push_str(" ORDER BY timestamp ASC");

    let mut q = sqlx::query(&sql).bind(pan_id);
    if let Some(f) = from {
        q = q.bind(f);
    }
    if let Some(t) = to {
        q = q.bind(t);
    }
    if let Some(et) = event_type {
        q = q.bind(et);
    }

    let rows = q.fetch_all(pool).await?;
    assemble_events(pool, rows).await
}

/// Get the timestamp of a single event (used for TimestampNotForward validation).
pub async fn get_event_timestamp(
    pool: &SqlitePool,
    event_id: &str,
) -> Result<i64, PanError> {
    let row = sqlx::query("SELECT timestamp FROM events WHERE event_id = ?")
        .bind(event_id)
        .fetch_optional(pool)
        .await?;
    match row {
        None => Err(PanError::EventNotFound(event_id.to_string())),
        Some(row) => Ok(row.get("timestamp")),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn assemble_events(
    pool: &SqlitePool,
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Vec<Event>, PanError> {
    let mut events = Vec::with_capacity(rows.len());

    for row in rows {
        let event_id: String = row.get("event_id");
        let tags_json: String = row.get("tags");
        let tags: Vec<String> = serde_json::from_str(&tags_json)
            .map_err(|e| PanError::StorageError(format!("parse tags: {}", e)))?;

        let parent_rows =
            sqlx::query("SELECT parent_hash FROM event_parents WHERE event_id = ?")
                .bind(&event_id)
                .fetch_all(pool)
                .await?;
        let parent_hashes: Vec<String> = parent_rows.iter().map(|r| r.get("parent_hash")).collect();

        let event_type_str: &str = row.get("event_type");
        let event = Event {
            event_id,
            entity_id: row.get("entity_id"),
            event_type: parse_event_type(event_type_str)?,
            timestamp: row.get("timestamp"),
            content: row.get("content"),
            tags,
            parent_hashes,
            references_event: row.get("references_event"),
            signature: row.get("signature"),
            actor_id: row.get("actor_id"),
        };
        events.push(event);
    }

    Ok(events)
}

fn parse_event_type(s: &str) -> Result<EventType, PanError> {
    match s {
        "actor_registered" => Ok(EventType::ActorRegistered),
        "pan_node_placed" => Ok(EventType::PanNodePlaced),
        "presence_recorded" => Ok(EventType::PresenceRecorded),
        "confirmation_recorded" => Ok(EventType::ConfirmationRecorded),
        other => Err(PanError::StorageError(format!("unknown event_type: {}", other))),
    }
}
