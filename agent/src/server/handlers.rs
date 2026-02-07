//! HTTP request handlers

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::authn::token_mngr::TokenManagerExt;
use crate::server::state::ServerState;
use crate::storage::device::load_device;
use crate::telemetry::collect_metrics;
use crate::utils::version_info;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Health check handler
pub async fn health_handler() -> impl IntoResponse {
    let version = version_info();
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "ajime-agent".to_string(),
        version: version.version,
    })
}

/// Version response
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub git_hash: String,
    pub build_time: String,
}

/// Version handler
pub async fn version_handler() -> impl IntoResponse {
    let version = version_info();
    Json(VersionResponse {
        version: version.version,
        git_hash: version.git_hash,
        build_time: version.build_time,
    })
}

/// Device info response
#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub name: String,
    pub device_type: Option<String>,
    pub status: String,
    pub owner_id: String,
}

/// Device info handler
pub async fn device_handler(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state.activity_tracker.touch();

    let device = load_device(&state.device_file)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DeviceResponse {
        id: device.id,
        name: device.name,
        device_type: device.device_type,
        status: "online".to_string(),
        owner_id: device.owner_id,
    }))
}

/// Sync request
#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub force: Option<bool>,
}

/// Sync response
#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
}

/// Sync handler
pub async fn sync_handler(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state.activity_tracker.touch();

    match state.syncer.trigger_sync().await {
        Ok(_) => Ok(Json(SyncResponse {
            success: true,
            message: "Sync completed successfully".to_string(),
        })),
        Err(e) => Ok(Json(SyncResponse {
            success: false,
            message: format!("Sync failed: {}", e),
        })),
    }
}

/// Workflows response
#[derive(Debug, Serialize)]
pub struct WorkflowsResponse {
    pub workflows: Vec<WorkflowInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
}

/// Workflows handler
pub async fn workflows_handler(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state.activity_tracker.touch();

    let workflow_ids = state.syncer.get_cached_workflows();
    let workflows: Vec<WorkflowInfo> = workflow_ids
        .into_iter()
        .map(|id| {
            // Get workflow from cache
            if let Some(entry) = state.caches.workflows.get(&id) {
                WorkflowInfo {
                    id: entry.workflow.id.clone(),
                    name: entry.workflow.name.clone(),
                    status: "deployed".to_string(),
                }
            } else {
                WorkflowInfo {
                    id: id.clone(),
                    name: "Unknown".to_string(),
                    status: "unknown".to_string(),
                }
            }
        })
        .collect();

    let total = workflows.len();

    Ok(Json(WorkflowsResponse { workflows, total }))
}

/// Metrics response
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_percent: f32,
    pub disk_used: u64,
    pub disk_total: u64,
    pub disk_percent: f32,
    pub uptime_secs: u64,
    pub hostname: String,
}

/// Metrics handler
pub async fn metrics_handler(
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    state.activity_tracker.touch();

    let metrics = collect_metrics();

    Json(MetricsResponse {
        cpu_usage: metrics.cpu_usage,
        memory_used: metrics.memory_used,
        memory_total: metrics.memory_total,
        memory_percent: metrics.memory_percent,
        disk_used: metrics.disk_used,
        disk_total: metrics.disk_total,
        disk_percent: metrics.disk_percent,
        uptime_secs: metrics.uptime_secs,
        hostname: metrics.hostname,
    })
}
