//! Deployment worker for orchestration

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use std::sync::Arc;

use tracing::{debug, error, info};

use crate::errors::AgentError;
use crate::http::client::HttpClient;
use crate::authn::token_mngr::{TokenManager, TokenManagerExt};
use crate::models::deployment::{Deployment, DeploymentStatusUpdate, DeploymentLog};
use crate::deploy::{docker, git, compose};

/// Deployer worker options
#[derive(Debug, Clone)]
pub struct Options {
    /// Polling interval
    pub interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
        }
    }
}

/// Run the deployer worker
pub async fn run<S, F>(
    options: &Options,
    http_client: Arc<HttpClient>,
    token_mngr: Arc<TokenManager>,
    sleep_fn: S,
    mut shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send>>,
) where
    S: Fn(Duration) -> F,
    F: Future<Output = ()>,
{
    info!("Deployer worker starting...");

    loop {
        // Check for shutdown
        tokio::select! {
            _ = &mut shutdown_signal => {
                info!("Deployer worker shutting down...");
                return;
            }
            _ = sleep_fn(options.interval) => {
                // Continue with check
            }
        }

        let device_id: String = match token_mngr.get_device_id().await {
            Ok(id) => id.to_string(),
            Err(_) => continue,
        };

        let token = match token_mngr.get_token().await {
            Ok(t) => t.raw,
            Err(_) => continue,
        };

        debug!("Checking for pending deployments...");

        // 1. Poll for pending deployments
        match http_client.get_pending_deployments(&device_id, &token).await {
            Ok(deployments) => {
                for deployment in deployments {
                    info!("Received deployment task: {} ({})", deployment.id, deployment.deployment_type);
                    
                    if let Err(e) = execute_deployment(deployment, http_client.clone(), &token).await {
                        error!("Deployment failed: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to poll for deployments: {}", e);
            }
        }
    }
}

async fn execute_deployment(
    deployment: Deployment, 
    http_client: Arc<HttpClient>, 
    token: &str
) -> Result<(), AgentError> {
    let id = deployment.id.clone();

    // 1. Mark as in_progress
    let _ = http_client.update_deployment_status(&id, token, DeploymentStatusUpdate {
        status: "in_progress".to_string(),
        error_message: None,
    }).await;

    // 2. Stream initial log
    let _ = http_client.send_deployment_log(&id, token, DeploymentLog {
        level: "info".to_string(),
        message: format!("Starting {} deployment...", deployment.deployment_type),
    }).await;

    // 3. Execute based on type
    let result = match deployment.deployment_type.as_str() {
        "docker" => {
            let image = deployment.config.get("image").and_then(|v| v.as_str()).unwrap_or("");
            let tag = deployment.config.get("tag").and_then(|v| v.as_str()).unwrap_or("latest");
            docker::deploy_docker(image, tag).await
        }
        "git" => {
            let repo_url = deployment.config.get("repo_url").and_then(|v| v.as_str()).unwrap_or("");
            let branch = deployment.config.get("branch").and_then(|v| v.as_str()).unwrap_or("main");
            let install_cmd = deployment.config.get("install_cmd").and_then(|v| v.as_str()).unwrap_or("");
            let run_cmd = deployment.config.get("run_cmd").and_then(|v| v.as_str()).unwrap_or("");
            let target_dir = format!("/etc/ajime/deployments/{}", deployment.id);
            git::deploy_git(repo_url, branch, install_cmd, run_cmd, &target_dir).await
        }
        "docker_compose" => {
            let target_dir = format!("/etc/ajime/deployments/{}", deployment.id);
            compose::deploy_compose(&target_dir).await
        }
        _ => Err(AgentError::DeployError(format!("Unsupported deployment type: {}", deployment.deployment_type))),
    };

    // 4. Update final status
    match result {
        Ok(_) => {
            let _ = http_client.update_deployment_status(&id, token, DeploymentStatusUpdate {
                status: "success".to_string(),
                error_message: None,
            }).await;
            
            let _ = http_client.send_deployment_log(&id, token, DeploymentLog {
                level: "info".to_string(),
                message: "Deployment completed successfully!".to_string(),
            }).await;
            Ok(())
        }
        Err(e) => {
            let _ = http_client.update_deployment_status(&id, token, DeploymentStatusUpdate {
                status: "failed".to_string(),
                error_message: Some(e.to_string()),
            }).await;
            
            let _ = http_client.send_deployment_log(&id, token, DeploymentLog {
                level: "error".to_string(),
                message: format!("Deployment failed: {}", e),
            }).await;
            Err(e)
        }
    }
}
