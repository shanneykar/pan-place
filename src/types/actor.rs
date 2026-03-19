use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    pub actor_id: String,
    pub pubkey: Vec<u8>,
    pub phone_dhash: String,
    pub created_at: i64,
}
