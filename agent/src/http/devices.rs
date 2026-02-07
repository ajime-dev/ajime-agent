//! Device API client

use serde::{Deserialize, Serialize};

use crate::errors::AgentError;
use crate::http::client::HttpClient;
use crate::telemetry::SystemMetrics;

/// Device status update request
#[derive(Debug, Clone, Serialize)]
pub struct DeviceStatusUpdate {
    pub status: String,
    pub agent_version: String,
    pub last_sync_at: Option<u64>,
    pub metrics: Option<SystemMetrics>,
}

/// Device sync request
#[derive(Debug, Clone, Serialize)]
pub struct DeviceSyncRequest {
    pub agent_version: String,
    pub local_workflow_digests: Vec<String>,
}

/// Device sync response
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceSyncResponse {
    pub device_id: String,
    pub workflows_to_update: Vec<String>,
    pub workflows_to_remove: Vec<String>,
    pub settings_updated: bool,
}

impl HttpClient {
    /// Update device status
    pub async fn update_device_status(
        &self,
        device_id: &str,
        token: &str,
        status: &DeviceStatusUpdate,
    ) -> Result<(), AgentError> {
        let path = format!("/devices/{}/status", device_id);
        let _: serde_json::Value = self.put(&path, token, status).await?;
        Ok(())
    }

    /// Sync device with backend
    pub async fn sync_device(
        &self,
        device_id: &str,
        token: &str,
        request: &DeviceSyncRequest,
    ) -> Result<DeviceSyncResponse, AgentError> {
        let path = format!("/devices/{}/sync", device_id);
        self.post(&path, token, request).await
    }

    /// Report device telemetry
    pub async fn report_telemetry(
        &self,
        device_id: &str,
        token: &str,
        metrics: &SystemMetrics,
    ) -> Result<(), AgentError> {
        let path = format!("/devices/{}/telemetry", device_id);
        let _: serde_json::Value = self.post(&path, token, metrics).await?;
        Ok(())
    }

    /// Get device settings from backend
    pub async fn get_device_settings(
        &self,
        device_id: &str,
        token: &str,
    ) -> Result<DeviceSettings, AgentError> {
        let path = format!("/devices/{}/settings", device_id);
        self.get(&path, token).await
    }
}

/// Device settings from backend
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceSettings {
    pub polling_interval_secs: Option<u64>,
    pub enable_mqtt: Option<bool>,
    pub enable_telemetry: Option<bool>,
    pub log_level: Option<String>,
}
