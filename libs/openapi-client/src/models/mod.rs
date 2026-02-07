//! API models

use serde::{Deserialize, Serialize};

/// Device activation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateDeviceRequest {
    pub activation_token: String,
    pub device_name: String,
    pub device_type: Option<String>,
}

/// Device activation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateDeviceResponse {
    pub device_id: String,
    pub owner_id: String,
    pub token: String,
    pub device_name: String,
}

/// Token refresh response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshResponse {
    pub token: String,
    pub expires_at: String,
}

/// Device status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceStatus {
    Online,
    Offline,
    Connected,
    Error,
}

/// Device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub status: DeviceStatus,
    pub device_type: Option<String>,
    pub owner_id: String,
    pub capabilities: Vec<String>,
    pub metadata: serde_json::Value,
    pub last_seen: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub logic_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
