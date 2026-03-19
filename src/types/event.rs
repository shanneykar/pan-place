use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ActorRegistered,
    PanNodePlaced,
    PresenceRecorded,
    ConfirmationRecorded,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::ActorRegistered => "actor_registered",
            EventType::PanNodePlaced => "pan_node_placed",
            EventType::PresenceRecorded => "presence_recorded",
            EventType::ConfirmationRecorded => "confirmation_recorded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub event_id: String,
    pub entity_id: String,
    pub event_type: EventType,
    pub timestamp: i64,
    pub content: String,
    pub tags: Vec<String>,
    pub parent_hashes: Vec<String>,
    pub references_event: Option<String>,
    pub signature: String,
    pub actor_id: String,
}
