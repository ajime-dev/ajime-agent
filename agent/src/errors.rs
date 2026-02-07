//! Error types for the Ajime agent

use thiserror::Error;

/// Main error type for the Ajime agent
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Token error: {0}")]
    TokenError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("Deployment error: {0}")]
    DeployError(String),

    #[error("MQTT error: {0}")]
    MqttError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Shutdown error: {0}")]
    ShutdownError(String),

    #[error("Device not activated: {0}")]
    DeviceNotActivated(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Hardware error: {0}")]
    HardwareError(String),

    #[error("Workflow error: {0}")]
    WorkflowError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for AgentError {
    fn from(err: anyhow::Error) -> Self {
        AgentError::Internal(err.to_string())
    }
}
