//! HTTP server setup

use std::future::Future;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::app::options::ServerOptions;
use crate::errors::AgentError;
use crate::server::handlers::{
    device_handler, health_handler, metrics_handler, sync_handler, version_handler,
    workflows_handler,
};
use crate::server::state::ServerState;

/// Start the HTTP server
pub async fn serve(
    options: &ServerOptions,
    state: Arc<ServerState>,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) -> Result<JoinHandle<Result<(), AgentError>>, AgentError> {
    let app = Router::new()
        // Health and version
        .route("/health", get(health_handler))
        .route("/version", get(version_handler))
        // Device
        .route("/device", get(device_handler))
        .route("/device/sync", post(sync_handler))
        // Workflows
        .route("/workflows/deployed", get(workflows_handler))
        // Telemetry
        .route("/telemetry/metrics", get(metrics_handler))
        // State and middleware
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", options.host, options.port);
    info!("Starting HTTP server on {}", addr);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| AgentError::ServerError(e.to_string()))?;

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| AgentError::ServerError(e.to_string()))
    });

    Ok(handle)
}
