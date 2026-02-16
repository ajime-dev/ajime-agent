//! Workflow API client

use serde::{Deserialize, Serialize};

use crate::errors::AgentError;
use crate::http::client::HttpClient;
use crate::models::workflow::Workflow;

/// Workflow list response
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowSummary>,
    pub total: usize,
}

/// Workflow summary (without full graph data)
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub logic_hash: Option<String>,
    pub updated_at: String,
}

/// Workflow sync response
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowSyncResponse {
    pub workflows: Vec<Workflow>,
    pub digests: Vec<WorkflowDigest>,
}

/// Workflow digest for change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDigest {
    pub workflow_id: String,
    pub digest: String,
    pub updated_at: String,
}

impl HttpClient {
    /// Get workflows assigned to this device
    pub async fn get_device_workflows(
        &self,
        device_id: &str,
        token: &str,
    ) -> Result<WorkflowListResponse, AgentError> {
        let path = format!("/agent/devices/{}/workflows", device_id);
        self.get(&path, token).await
    }

    /// Get a specific workflow by ID
    pub async fn get_workflow(
        &self,
        workflow_id: &str,
        token: &str,
    ) -> Result<Workflow, AgentError> {
        let path = format!("/agent/workflows/{}", workflow_id);
        self.get(&path, token).await
    }

    /// Get workflow digests for change detection
    pub async fn get_workflow_digests(
        &self,
        device_id: &str,
        token: &str,
    ) -> Result<Vec<WorkflowDigest>, AgentError> {
        let path = format!("/agent/devices/{}/workflows/digests", device_id);
        self.get(&path, token).await
    }

    /// Sync workflows (get full data for changed workflows)
    pub async fn sync_workflows(
        &self,
        device_id: &str,
        token: &str,
        local_digests: &[WorkflowDigest],
    ) -> Result<WorkflowSyncResponse, AgentError> {
        let path = format!("/agent/devices/{}/workflows/sync", device_id);
        self.post(&path, token, &local_digests).await
    }

    /// Report workflow execution status
    pub async fn report_workflow_status(
        &self,
        device_id: &str,
        workflow_id: &str,
        token: &str,
        status: &WorkflowStatusReport,
    ) -> Result<(), AgentError> {
        let path = format!("/agent/devices/{}/workflows/{}/status", device_id, workflow_id);
        let _: serde_json::Value = self.post(&path, token, status).await?;
        Ok(())
    }
}

/// Workflow status report
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatusReport {
    pub status: String,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub node_statuses: Vec<NodeStatusReport>,
}

/// Node status report
#[derive(Debug, Clone, Serialize)]
pub struct NodeStatusReport {
    pub node_id: String,
    pub status: String,
    pub error: Option<String>,
    pub outputs: Option<serde_json::Value>,
}
