//! Deployment API client

use serde::{Deserialize, Serialize};
use crate::errors::AgentError;
use crate::http::client::HttpClient;
use crate::models::deployment::{Deployment, DeploymentStatusUpdate, DeploymentLog};

/// List of deployments response
#[derive(Debug, Clone, Deserialize)]
pub struct DeploymentListResponse {
    pub deployments: Vec<Deployment>,
}

impl HttpClient {
    /// Get pending deployments for this device
    pub async fn get_pending_deployments(
        &self,
        device_id: &str,
        token: &str,
    ) -> Result<Vec<Deployment>, AgentError> {
        let path = format!("/agent/devices/{}/deployments", device_id);
        let response: DeploymentListResponse = self.get(&path, token).await?;
        Ok(response.deployments)
    }

    /// Update deployment status
    pub async fn update_deployment_status(
        &self,
        deployment_id: &str,
        token: &str,
        status: DeploymentStatusUpdate,
    ) -> Result<(), AgentError> {
        let path = format!("/deployments/{}/status", deployment_id);
        let _: serde_json::Value = self.patch(&path, token, &status).await?;
        Ok(())
    }

    /// Send a deployment log
    pub async fn send_deployment_log(
        &self,
        deployment_id: &str,
        token: &str,
        log: DeploymentLog,
    ) -> Result<(), AgentError> {
        let path = format!("/deployments/{}/logs", deployment_id);
        let _: serde_json::Value = self.post(&path, token, &log).await?;
        Ok(())
    }
}
