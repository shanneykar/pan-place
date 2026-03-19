use std::sync::Arc;

use pan::{api, store::PanStore};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let data_dir = std::env::var("PAN_DATA_DIR").unwrap_or_else(|_| "data".to_string());

    let store = PanStore::new(&data_dir)
        .await
        .expect("Failed to initialise store");
    let store = Arc::new(store);

    let app = api::router(store);

    let addr = std::env::var("PAN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind {}: {}", addr, e));

    tracing::info!("PAN layer-0 listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
