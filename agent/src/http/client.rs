//! HTTP client implementation

use reqwest::{Client, header};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error};

use crate::errors::AgentError;

/// HTTP client for backend communication
pub struct HttpClient {
    client: Client,
    base_url: String,
    device_id: Option<String>,
}

impl HttpClient {
    /// Create a new HTTP client
    pub async fn new(base_url: &str) -> Result<Self, AgentError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            device_id: None,
        })
    }

    /// Create a new HTTP client with device ID for authentication
    pub async fn with_device_id(base_url: &str, device_id: String) -> Result<Self, AgentError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            device_id: Some(device_id),
        })
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str, token: &str) -> Result<T, AgentError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("GET {}", url);

        let mut request = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token));
        
        // Add X-Device-ID header if device_id is set
        if let Some(device_id) = &self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP GET failed: {} - {}", status, body);
            return Err(AgentError::ConfigError(format!("{}: {}", status, body)));
        }

        let body = response.json().await?;
        Ok(body)
    }

    /// Make a POST request
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        token: &str,
        body: &B,
    ) -> Result<T, AgentError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("POST {}", url);

        let mut request = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(body);
        
        // Add X-Device-ID header if device_id is set
        if let Some(device_id) = &self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP POST failed: {} - {}", status, body);
            return Err(AgentError::ConfigError(format!("{}: {}", status, body)));
        }

        let body = response.json().await?;
        Ok(body)
    }

    /// Make a PUT request
    pub async fn put<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        token: &str,
        body: &B,
    ) -> Result<T, AgentError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("PUT {}", url);

        let mut request = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(body);
        
        // Add X-Device-ID header if device_id is set
        if let Some(device_id) = &self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP PUT failed: {} - {}", status, body);
            return Err(AgentError::ConfigError(format!("{}: {}", status, body)));
        }

        let body = response.json().await?;
        Ok(body)
    }

    /// Make a PATCH request
    pub async fn patch<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        token: &str,
        body: &B,
    ) -> Result<T, AgentError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("PATCH {}", url);

        let mut request = self
            .client
            .patch(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(body);
        
        // Add X-Device-ID header if device_id is set
        if let Some(device_id) = &self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP PATCH failed: {} - {}", status, body);
            return Err(AgentError::ConfigError(format!("{}: {}", status, body)));
        }

        let body = response.json().await?;
        Ok(body)
    }

    /// Activate a device with an activation token
    pub async fn activate_device(
        &self,
        activation_token: &str,
        device_name: &str,
        device_type: Option<&str>,
    ) -> Result<DeviceActivationResponse, AgentError> {
        let url = format!("{}/agent/devices/activate", self.base_url);
        debug!("POST {} (activation)", url);

        let body = serde_json::json!({
            "activation_token": activation_token,
            "device_name": device_name,
            "device_type": device_type,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Device activation failed: {} - {}", status, body);
            return Err(AgentError::AuthError(format!(
                "Activation failed: {} - {}",
                status, body
            )));
        }

        let body = response.json().await?;
        Ok(body)
    }

    /// Refresh device token
    pub async fn refresh_device_token(
        &self,
        device_id: &str,
        current_token: &str,
    ) -> Result<String, AgentError> {
        let url = format!("{}/agent/devices/{}/token/refresh", self.base_url, device_id);
        debug!("POST {} (token refresh)", url);

        let response = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", current_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Token refresh failed: {} - {}", status, body);
            return Err(AgentError::TokenError(format!(
                "Token refresh failed: {} - {}",
                status, body
            )));
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            token: String,
        }

        let body: TokenResponse = response.json().await?;
        Ok(body.token)
    }
}

/// Device activation response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeviceActivationResponse {
    pub device_id: String,
    pub owner_id: String,
    pub token: String,
    pub device_name: String,
}
