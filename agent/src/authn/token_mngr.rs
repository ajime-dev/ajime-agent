//! Token manager for device authentication

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::authn::device_token::DeviceToken;
use crate::errors::AgentError;
use crate::filesys::file::File;
use crate::http::client::HttpClient;
use crate::storage::device::{load_device, save_device, Device};

/// Token manager trait for testability
#[async_trait]
pub trait TokenManagerExt: Send + Sync {
    /// Get the current token
    async fn get_token(&self) -> Result<DeviceToken, AgentError>;

    /// Refresh the token
    async fn refresh_token(&self) -> Result<DeviceToken, AgentError>;

    /// Get the device ID
    async fn get_device_id(&self) -> Result<String, AgentError>;
}

/// Token manager implementation
pub struct TokenManager {
    device_file: Arc<File>,
    http_client: Arc<HttpClient>,
    cached_token: RwLock<Option<DeviceToken>>,
}

impl TokenManager {
    /// Create a new token manager
    pub async fn new(
        device_file: Arc<File>,
        http_client: Arc<HttpClient>,
    ) -> Result<Self, AgentError> {
        let manager = Self {
            device_file,
            http_client,
            cached_token: RwLock::new(None),
        };

        // Load initial token
        manager.load_token().await?;

        Ok(manager)
    }

    /// Load token from device file
    async fn load_token(&self) -> Result<DeviceToken, AgentError> {
        let device = load_device(&self.device_file).await?;
        let token = DeviceToken::from_raw(device.token)?;

        let mut cached = self.cached_token.write().await;
        *cached = Some(token.clone());

        Ok(token)
    }

    /// Save token to device file
    async fn save_token(&self, token: &DeviceToken) -> Result<(), AgentError> {
        let mut device = load_device(&self.device_file).await?;
        device.token = token.raw.clone();
        save_device(&self.device_file, &device).await?;

        let mut cached = self.cached_token.write().await;
        *cached = Some(token.clone());

        Ok(())
    }
}

#[async_trait]
impl TokenManagerExt for TokenManager {
    async fn get_token(&self) -> Result<DeviceToken, AgentError> {
        // Try to get from cache first
        {
            let cached = self.cached_token.read().await;
            if let Some(token) = cached.as_ref() {
                return Ok(token.clone());
            }
        }

        // Load from file
        self.load_token().await
    }

    async fn refresh_token(&self) -> Result<DeviceToken, AgentError> {
        info!("Refreshing device token...");

        let current_token = self.get_token().await?;
        let device_id = current_token.device_id().to_string();

        // Call backend to refresh token
        let new_token_raw = self
            .http_client
            .refresh_device_token(&device_id, &current_token.raw)
            .await?;

        let new_token = DeviceToken::from_raw(new_token_raw)?;

        // Save the new token
        self.save_token(&new_token).await?;

        info!("Token refreshed successfully, expires at: {}", new_token.expires_at());

        Ok(new_token)
    }

    async fn get_device_id(&self) -> Result<String, AgentError> {
        let token = self.get_token().await?;
        Ok(token.device_id().to_string())
    }
}
