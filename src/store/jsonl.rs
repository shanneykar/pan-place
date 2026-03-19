use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use crate::error::PanError;
use crate::types::Event;

pub fn actor_jsonl_path(data_dir: &Path, actor_id: &str) -> PathBuf {
    data_dir.join("actors").join(format!("{}.jsonl", actor_id))
}

pub fn node_jsonl_path(data_dir: &Path, pan_id: &str) -> PathBuf {
    data_dir.join("nodes").join(format!("{}.jsonl", pan_id))
}

/// Append a single event as a JSON line to the given file path.
/// Creates the file if it does not exist.
pub async fn append_event(path: &Path, event: &Event) -> Result<(), PanError> {
    let mut line =
        serde_json::to_string(event).map_err(|e| PanError::StorageError(e.to_string()))?;
    line.push('\n');

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .map_err(|e| PanError::StorageError(format!("open {}: {}", path.display(), e)))?;

    file.write_all(line.as_bytes())
        .await
        .map_err(|e| PanError::StorageError(format!("write {}: {}", path.display(), e)))?;

    Ok(())
}

/// Read all events from a JSONL file. Returns empty vec if file does not exist.
pub async fn read_entity_events(path: &Path) -> Result<Vec<Event>, PanError> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PanError::StorageError(format!("read {}: {}", path.display(), e)))?;

    let mut events = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(line)
            .map_err(|e| PanError::StorageError(format!("parse jsonl: {}", e)))?;
        events.push(event);
    }

    Ok(events)
}
