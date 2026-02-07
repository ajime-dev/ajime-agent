//! Agent API models

use serde::{Deserialize, Serialize};

/// Health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Version response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
    pub git_hash: String,
    pub build_time: String,
}

/// Device response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResponse {
    pub id: String,
    pub name: String,
    pub device_type: Option<String>,
    pub status: String,
    pub owner_id: String,
}

/// Sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
}

/// Workflow list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowSummary>,
    pub total: usize,
}

/// Workflow summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub status: String,
}

/// Metrics response
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Workflow start request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStartRequest {
    pub workflow_id: String,
}

/// Workflow control response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowControlResponse {
    pub success: bool,
    pub workflow_id: String,
    pub status: String,
    pub message: Option<String>,
}
