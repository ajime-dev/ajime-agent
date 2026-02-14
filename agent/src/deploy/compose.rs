//! Docker Compose deployment executor

use std::path::Path;
use tokio::process::Command;
use tracing::{info, error, debug};
use crate::errors::AgentError;

pub async fn deploy_compose(target_dir: &str) -> Result<(), AgentError> {
    info!("Deploying with Docker Compose in: {}", target_dir);

    let path = Path::new(target_dir);
    if !path.exists() {
        return Err(AgentError::DeployError(format!("Target directory does not exist: {}", target_dir)));
    }

    // Run docker-compose up -d
    debug!("Running docker-compose up -d...");
    let status = Command::new("docker-compose")
        .current_dir(path)
        .args(["up", "-d"])
        .status()
        .await
        .map_err(|e| AgentError::DeployError(format!("Failed to run docker-compose: {}", e)))?;

    if !status.success() {
        // Try 'docker compose' (newer version)
        debug!("docker-compose failed, trying 'docker compose'...");
        let status = Command::new("docker")
            .current_dir(path)
            .args(["compose", "up", "-d"])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run docker compose: {}", e)))?;
        
        if !status.success() {
            return Err(AgentError::DeployError("Docker Compose failed".to_string()));
        }
    }

    info!("Successfully deployed Docker Compose application");
    Ok(())
}
