//! Device file management

use serde::{Deserialize, Serialize};

use crate::errors::AgentError;
use crate::filesys::file::File;

/// Device information stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device ID
    pub id: String,

    /// Device name
    pub name: String,

    /// Owner user ID
    pub owner_id: String,

    /// Device JWT token
    pub token: String,

    /// Device type (e.g., "raspberry_pi", "jetson_nano")
    pub device_type: Option<String>,

    /// Device capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Device metadata
    #[serde(default)]
    pub metadata: serde_json::Value,

    /// Activation timestamp (Unix epoch seconds)
    pub activated_at: u64,

    /// Last sync timestamp
    pub last_sync_at: Option<u64>,
}

impl Device {
    /// Create a new device
    pub fn new(id: String, name: String, owner_id: String, token: String) -> Self {
        Self {
            id,
            name,
            owner_id,
            token,
            device_type: None,
            capabilities: Vec::new(),
            metadata: serde_json::Value::Null,
            activated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_sync_at: None,
        }
    }
}

/// Assert that the device has been activated
pub async fn assert_activated(device_file: &File) -> Result<Device, AgentError> {
    if !device_file.exists().await {
        return Err(AgentError::DeviceNotActivated(
            "Device file does not exist".to_string(),
        ));
    }

    let device: Device = device_file.read_json().await.map_err(|e| {
        AgentError::DeviceNotActivated(format!("Failed to read device file: {}", e))
    })?;

    if device.id.is_empty() {
        return Err(AgentError::DeviceNotActivated(
            "Device ID is empty".to_string(),
        ));
    }

    if device.token.is_empty() {
        return Err(AgentError::DeviceNotActivated(
            "Device token is empty".to_string(),
        ));
    }

    Ok(device)
}

/// Load device from file
pub async fn load_device(device_file: &File) -> Result<Device, AgentError> {
    device_file.read_json().await
}

/// Save device to file
pub async fn save_device(device_file: &File, device: &Device) -> Result<(), AgentError> {
    device_file.write_json(device).await
}
