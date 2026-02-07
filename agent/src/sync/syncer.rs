//! Workflow synchronization

use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::authn::token_mngr::{TokenManager, TokenManagerExt};
use crate::cache::workflow::WorkflowCache;
use crate::deploy::fsm::FsmSettings;
use crate::errors::AgentError;
use crate::filesys::dir::Dir;
use crate::filesys::file::File;
use crate::http::client::HttpClient;
use crate::http::workflows::WorkflowDigest;
use crate::utils::{calc_exp_backoff, sha256_hash, CooldownOptions};

/// Sync state
#[derive(Debug, Clone)]
pub struct SyncState {
    pub last_attempted_sync_at: DateTime<Utc>,
    pub last_synced_at: DateTime<Utc>,
    pub cooldown_ends_at: DateTime<Utc>,
    pub err_streak: u32,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            last_attempted_sync_at: DateTime::<Utc>::MIN_UTC,
            last_synced_at: DateTime::<Utc>::MIN_UTC,
            cooldown_ends_at: DateTime::<Utc>::MIN_UTC,
            err_streak: 0,
        }
    }
}

impl SyncState {
    pub fn is_in_cooldown(&self) -> bool {
        Utc::now() < self.cooldown_ends_at
    }
}

/// Workflow syncer
pub struct Syncer {
    device_file: Arc<File>,
    http_client: Arc<HttpClient>,
    token_mngr: Arc<TokenManager>,
    workflow_cache: Arc<WorkflowCache>,
    deployment_dir: Dir,
    fsm_settings: FsmSettings,
    agent_version: String,
    state: RwLock<SyncState>,
    cooldown_options: CooldownOptions,
}

impl Syncer {
    /// Create a new syncer
    pub fn new(
        device_file: Arc<File>,
        http_client: Arc<HttpClient>,
        token_mngr: Arc<TokenManager>,
        workflow_cache: Arc<WorkflowCache>,
        deployment_dir: Dir,
        fsm_settings: FsmSettings,
        agent_version: String,
    ) -> Self {
        Self {
            device_file,
            http_client,
            token_mngr,
            workflow_cache,
            deployment_dir,
            fsm_settings,
            agent_version,
            state: RwLock::new(SyncState::default()),
            cooldown_options: CooldownOptions::default(),
        }
    }

    /// Trigger a sync
    pub async fn trigger_sync(&self) -> Result<(), AgentError> {
        // Check cooldown
        {
            let state = self.state.read().await;
            if state.is_in_cooldown() {
                debug!("Sync in cooldown, skipping...");
                return Ok(());
            }
        }

        // Update last attempted
        {
            let mut state = self.state.write().await;
            state.last_attempted_sync_at = Utc::now();
        }

        // Perform sync
        match self.sync_impl().await {
            Ok(_) => {
                let mut state = self.state.write().await;
                state.last_synced_at = Utc::now();
                state.err_streak = 0;
                info!("Sync completed successfully");
                Ok(())
            }
            Err(e) => {
                let mut state = self.state.write().await;
                state.err_streak += 1;
                
                // Calculate cooldown
                let cooldown = calc_exp_backoff(&self.cooldown_options, state.err_streak);
                state.cooldown_ends_at = Utc::now() + chrono::Duration::from_std(cooldown).unwrap();
                
                error!(
                    "Sync failed (attempt {}), cooldown until {}: {}",
                    state.err_streak, state.cooldown_ends_at, e
                );
                Err(e)
            }
        }
    }

    async fn sync_impl(&self) -> Result<(), AgentError> {
        info!("Starting workflow sync...");

        // Get device ID and token
        let device_id = self.token_mngr.get_device_id().await?;
        let token = self.token_mngr.get_token().await?;

        // Get local digests
        let local_digests: Vec<WorkflowDigest> = self
            .workflow_cache
            .digests()
            .into_iter()
            .map(|(id, digest)| WorkflowDigest {
                workflow_id: id,
                digest,
                updated_at: String::new(),
            })
            .collect();

        debug!("Local workflows: {}", local_digests.len());

        // Sync with backend
        let sync_response = self
            .http_client
            .sync_workflows(&device_id, &token.raw, &local_digests)
            .await?;

        info!(
            "Sync response: {} workflows, {} digests",
            sync_response.workflows.len(),
            sync_response.digests.len()
        );

        // Update cache with new workflows
        for workflow in sync_response.workflows {
            let digest = sha256_hash(serde_json::to_string(&workflow)?.as_bytes());
            info!("Caching workflow: {} ({})", workflow.name, workflow.id);
            self.workflow_cache.insert(workflow, digest);
        }

        // Remove workflows that are no longer assigned
        let remote_ids: std::collections::HashSet<_> = sync_response
            .digests
            .iter()
            .map(|d| d.workflow_id.clone())
            .collect();

        for local_id in self.workflow_cache.keys() {
            if !remote_ids.contains(&local_id) {
                info!("Removing workflow from cache: {}", local_id);
                self.workflow_cache.remove(&local_id);
            }
        }

        Ok(())
    }

    /// Get sync state
    pub async fn get_state(&self) -> SyncState {
        self.state.read().await.clone()
    }

    /// Get cached workflows
    pub fn get_cached_workflows(&self) -> Vec<String> {
        self.workflow_cache.keys()
    }
}
