//! Docker deployment executor

use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, debug};
use crate::errors::AgentError;

pub async fn deploy_docker(image: &str, tag: &str, registry_token: Option<String>) -> Result<(), AgentError> {
    // Handle case where image already includes tag (e.g., from Ajime builder)
    let full_image = if image.contains(':') || tag.is_empty() {
        image.to_string()
    } else {
        format!("{}:{}", image, tag)
    };
    
    info!("Deploying Docker image: {}", full_image);

    // 1. Authenticate with GHCR if this is a ghcr.io image
    if full_image.starts_with("ghcr.io/") {
        debug!("Authenticating with GitHub Container Registry...");

        // Use token from deployment config first, fall back to environment variable
        let token_opt = registry_token
            .filter(|t| !t.is_empty())
            .or_else(|| std::env::var("GHCR_TOKEN").ok());

        if let Some(ghcr_token) = token_opt {
            let login_result: Result<bool, std::io::Error> = async {
                let mut child = Command::new("docker")
                    .args(["login", "ghcr.io", "-u", "ajime-agent", "--password-stdin"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(ghcr_token.as_bytes()).await?;
                }
                let output = child.wait_with_output().await?;
                Ok(output.status.success())
            }.await;

            match login_result {
                Ok(true) => {
                    debug!("Successfully authenticated with GHCR");
                }
                Ok(false) => {
                    debug!("GHCR authentication failed, attempting public pull");
                }
                Err(e) => {
                    debug!("Failed to run docker login: {}, attempting public pull", e);
                }
            }

        } else {
            debug!("GHCR_TOKEN not set, attempting public pull");
        }
    }

    // 2. Pull image
    debug!("Pulling image: {}", full_image);
    let pull_status = Command::new("docker")
        .args(["pull", &full_image])
        .status()
        .await
        .map_err(|e| AgentError::DeployError(format!("Failed to run docker pull: {}", e)))?;

    if !pull_status.success() {
        return Err(AgentError::DeployError(format!("Docker pull failed for {}", full_image)));
    }

    // 3. Stop existing container if any (simplified: using image name as container name)
    let container_name = full_image
        .split('/')
        .last()
        .unwrap_or(&full_image)
        .split(':')
        .next()
        .unwrap_or("container");
    
    debug!("Stopping existing container: {}", container_name);
    let _ = Command::new("docker")
        .args(["stop", container_name])
        .status()
        .await;
    
    let _ = Command::new("docker")
        .args(["rm", container_name])
        .status()
        .await;

    // 4. Run new container
    debug!("Running new container: {}", container_name);
    let run_status = Command::new("docker")
        .args(["run", "-d", "--name", container_name, "--restart", "unless-stopped", &full_image])
        .status()
        .await
        .map_err(|e| AgentError::DeployError(format!("Failed to run docker run: {}", e)))?;

    if !run_status.success() {
        return Err(AgentError::DeployError(format!("Docker run failed for {}", full_image)));
    }

    info!("Successfully deployed Docker image: {}", full_image);
    Ok(())
}
