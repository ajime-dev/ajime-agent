//! HTTP client implementation

use reqwest::{Client, header};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error};

use crate::errors::AgentError;

/// HTTP client for backend communication
pub struct HttpClient {
    client: Client,
    base_url: String,
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

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP GET failed: {} - {}", status, body);
            return Err(AgentError::HttpError(reqwest::Error::from(
                std::io::Error::new(std::io::ErrorKind::Other, format!("{}: {}", status, body)),
            )));
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

        let response = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP POST failed: {} - {}", status, body);
            return Err(AgentError::HttpError(reqwest::Error::from(
                std::io::Error::new(std::io::ErrorKind::Other, format!("{}: {}", status, body)),
            )));
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

        let response = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("HTTP PUT failed: {} - {}", status, body);
            return Err(AgentError::HttpError(reqwest::Error::from(
                std::io::Error::new(std::io::ErrorKind::Other, format!("{}: {}", status, body)),
            )));
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
        let url = format!("{}/devices/activate", self.base_url);
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
        let url = format!("{}/devices/{}/token/refresh", self.base_url, device_id);
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
