//! Docker deployment executor

use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error, debug};
use crate::errors::AgentError;

pub async fn deploy_docker(image: &str, tag: &str) -> Result<(), AgentError> {
    let full_image = format!("{}:{}", image, tag);
    info!("Deploying Docker image: {}", full_image);

    // 1. Pull image
    debug!("Pulling image: {}", full_image);
    let pull_status = Command::new("docker")
        .args(["pull", &full_image])
        .status()
        .await
        .map_err(|e| AgentError::DeployError(format!("Failed to run docker pull: {}", e)))?;

    if !pull_status.success() {
        return Err(AgentError::DeployError(format!("Docker pull failed for {}", full_image)));
    }

    // 2. Stop existing container if any (simplified: using image name as container name)
    let container_name = image.split('/').last().unwrap_or(image);
    debug!("Stopping existing container: {}", container_name);
    let _ = Command::new("docker")
        .args(["stop", container_name])
        .status()
        .await;
    
    let _ = Command::new("docker")
        .args(["rm", container_name])
        .status()
        .await;

    // 3. Run new container
    debug!("Running new container: {}", container_name);
    let run_status = Command::new("docker")
        .args(["run", "-d", "--name", container_name, &full_image])
        .status()
        .await
        .map_err(|e| AgentError::DeployError(format!("Failed to run docker run: {}", e)))?;

    if !run_status.success() {
        return Err(AgentError::DeployError(format!("Docker run failed for {}", full_image)));
    }

    info!("Successfully deployed Docker image: {}", full_image);
    Ok(())
}
