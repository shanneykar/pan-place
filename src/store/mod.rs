pub mod actors;
pub mod events;
pub mod jsonl;
pub mod nodes;
pub mod sqlite;

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::error::PanError;
use crate::types::{Actor, Event, PanNode};

/// Combined storage handle: JSONL source-of-truth + SQLite index.
pub struct PanStore {
    pub pool: SqlitePool,
    pub data_dir: PathBuf,
}

impl PanStore {
    /// Create a new PanStore rooted at `data_dir`.
    /// Creates the directory tree and runs SQLite migrations.
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self, PanError> {
        let data_dir = data_dir.as_ref().to_path_buf();

        tokio::fs::create_dir_all(data_dir.join("actors"))
            .await
            .map_err(|e| PanError::StorageError(e.to_string()))?;
        tokio::fs::create_dir_all(data_dir.join("nodes"))
            .await
            .map_err(|e| PanError::StorageError(e.to_string()))?;
        tokio::fs::create_dir_all(data_dir.join("index"))
            .await
            .map_err(|e| PanError::StorageError(e.to_string()))?;

        let pool = sqlite::init_pool(&data_dir).await?;

        Ok(Self { pool, data_dir })
    }

    // -----------------------------------------------------------------------
    // Actor writes
    // -----------------------------------------------------------------------

    /// Insert actor into SQLite and append its registration event to JSONL.
    pub async fn write_actor(&self, actor: &Actor, reg_event: &Event) -> Result<(), PanError> {
        actors::insert_actor(&self.pool, actor).await?;
        let path = jsonl::actor_jsonl_path(&self.data_dir, &actor.actor_id);
        jsonl::append_event(&path, reg_event).await?;
        events::insert_event(&self.pool, reg_event).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Node writes
    // -----------------------------------------------------------------------

    /// Insert node into SQLite and append its placement event to JSONL.
    pub async fn write_node(&self, node: &PanNode, placed_event: &Event) -> Result<(), PanError> {
        nodes::insert_node(&self.pool, node).await?;
        let path = jsonl::node_jsonl_path(&self.data_dir, &node.pan_id);
        jsonl::append_event(&path, placed_event).await?;
        events::insert_event(&self.pool, placed_event).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Event writes (for PresenceRecorded / ConfirmationRecorded)
    // -----------------------------------------------------------------------

    /// Append an actor-entity event: write to actor's JSONL + SQLite.
    pub async fn write_actor_event(&self, event: &Event) -> Result<(), PanError> {
        let path = jsonl::actor_jsonl_path(&self.data_dir, &event.entity_id);
        jsonl::append_event(&path, event).await?;
        events::insert_event(&self.pool, event).await?;
        Ok(())
    }

    /// Append a node-entity event: write to node's JSONL + SQLite.
    pub async fn write_node_event(&self, event: &Event) -> Result<(), PanError> {
        let path = jsonl::node_jsonl_path(&self.data_dir, &event.entity_id);
        jsonl::append_event(&path, event).await?;
        events::insert_event(&self.pool, event).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Reads — SQLite
    // -----------------------------------------------------------------------

    pub async fn get_actor(&self, actor_id: &str) -> Result<Actor, PanError> {
        actors::get_actor(&self.pool, actor_id).await
    }

    pub async fn get_node(&self, pan_id: &str) -> Result<PanNode, PanError> {
        nodes::get_node(&self.pool, pan_id).await
    }

    pub async fn actor_exists(&self, actor_id: &str) -> Result<bool, PanError> {
        actors::actor_exists(&self.pool, actor_id).await
    }

    pub async fn node_exists(&self, pan_id: &str) -> Result<bool, PanError> {
        nodes::node_exists(&self.pool, pan_id).await
    }

    pub async fn event_exists(&self, event_id: &str) -> Result<bool, PanError> {
        events::event_exists(&self.pool, event_id).await
    }

    pub async fn phone_dhash_exists(&self, phone_dhash: &str) -> Result<bool, PanError> {
        actors::phone_dhash_exists(&self.pool, phone_dhash).await
    }

    pub async fn get_events_for_actor(&self, actor_id: &str) -> Result<Vec<Event>, PanError> {
        events::get_events_for_actor(&self.pool, actor_id).await
    }

    pub async fn get_events_for_node(
        &self,
        pan_id: &str,
        from: Option<i64>,
        to: Option<i64>,
        event_type: Option<&str>,
    ) -> Result<Vec<Event>, PanError> {
        events::get_events_for_node(&self.pool, pan_id, from, to, event_type).await
    }

    pub async fn get_event_timestamp(&self, event_id: &str) -> Result<i64, PanError> {
        events::get_event_timestamp(&self.pool, event_id).await
    }

    // -----------------------------------------------------------------------
    // Reads — JSONL
    // -----------------------------------------------------------------------

    pub async fn read_actor_events_jsonl(&self, actor_id: &str) -> Result<Vec<Event>, PanError> {
        let path = jsonl::actor_jsonl_path(&self.data_dir, actor_id);
        jsonl::read_entity_events(&path).await
    }

    pub async fn read_node_events_jsonl(&self, pan_id: &str) -> Result<Vec<Event>, PanError> {
        let path = jsonl::node_jsonl_path(&self.data_dir, pan_id);
        jsonl::read_entity_events(&path).await
    }
}
