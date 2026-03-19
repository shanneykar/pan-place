use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Fixed,
    Ephemeral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PanNode {
    pub pan_id: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_miles: f64,
    pub placed_at: i64,
    pub node_type: NodeType,
    pub status: NodeStatus,
}
