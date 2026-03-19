pub mod actors;
pub mod events;
pub mod nodes;
pub mod query;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::store::PanStore;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<PanStore>,
}

pub fn router(store: Arc<PanStore>) -> Router {
    let state = AppState { store };
    Router::new()
        .route("/actors", post(actors::post_actor))
        .route("/nodes", post(nodes::post_node))
        .route("/events", post(events::post_event))
        .route("/actors/:actor_id/events", get(query::get_actor_events))
        .route("/nodes/:pan_id/events", get(query::get_node_events))
        .with_state(state)
}

pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
