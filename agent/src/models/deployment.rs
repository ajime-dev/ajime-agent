//! Deployment models

use serde::{Deserialize, Serialize};

/// A deployment task received from the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Unique deployment ID
    pub id: String,
    
    /// Device ID this deployment is for
    pub device_id: String,
    
    /// Type of deployment: 'docker', 'git', 'docker_compose'
    pub deployment_type: String,
    
    /// Deployment configuration
    pub config: serde_json::Value,
    
    /// Current status
    pub status: String,
}

/// Status update to send back to the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStatusUpdate {
    /// New status
    pub status: String,
    
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Log entry to stream to the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLog {
    /// Log level: 'info', 'warn', 'error', 'debug'
    pub level: String,
    
    /// Log message
    pub message: String,
}
