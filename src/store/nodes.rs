use sqlx::{Row, SqlitePool};

use crate::error::PanError;
use crate::types::{NodeStatus, NodeType, PanNode};

pub async fn insert_node(pool: &SqlitePool, node: &PanNode) -> Result<(), PanError> {
    sqlx::query(
        "INSERT INTO pan_nodes (pan_id, lat, lon, radius_miles, placed_at, node_type, status) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&node.pan_id)
    .bind(node.lat)
    .bind(node.lon)
    .bind(node.radius_miles)
    .bind(node.placed_at)
    .bind(node_type_to_str(node.node_type))
    .bind(node_status_to_str(node.status))
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_node(pool: &SqlitePool, pan_id: &str) -> Result<PanNode, PanError> {
    let row = sqlx::query(
        "SELECT pan_id, lat, lon, radius_miles, placed_at, node_type, status \
         FROM pan_nodes WHERE pan_id = ?",
    )
    .bind(pan_id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Err(PanError::NodeNotFound(pan_id.to_string())),
        Some(row) => Ok(PanNode {
            pan_id: row.get("pan_id"),
            lat: row.get("lat"),
            lon: row.get("lon"),
            radius_miles: row.get("radius_miles"),
            placed_at: row.get("placed_at"),
            node_type: parse_node_type(row.get("node_type"))?,
            status: parse_node_status(row.get("status"))?,
        }),
    }
}

pub async fn node_exists(pool: &SqlitePool, pan_id: &str) -> Result<bool, PanError> {
    let row = sqlx::query("SELECT 1 FROM pan_nodes WHERE pan_id = ?")
        .bind(pan_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

fn node_type_to_str(t: NodeType) -> &'static str {
    match t {
        NodeType::Fixed => "fixed",
        NodeType::Ephemeral => "ephemeral",
    }
}

fn node_status_to_str(s: NodeStatus) -> &'static str {
    match s {
        NodeStatus::Active => "active",
        NodeStatus::Archived => "archived",
    }
}

fn parse_node_type(s: &str) -> Result<NodeType, PanError> {
    match s {
        "fixed" => Ok(NodeType::Fixed),
        "ephemeral" => Ok(NodeType::Ephemeral),
        other => Err(PanError::StorageError(format!("unknown node_type: {}", other))),
    }
}

fn parse_node_status(s: &str) -> Result<NodeStatus, PanError> {
    match s {
        "active" => Ok(NodeStatus::Active),
        "archived" => Ok(NodeStatus::Archived),
        other => Err(PanError::StorageError(format!("unknown node status: {}", other))),
    }
}
