//! Docker Compose deployment executor

use std::path::Path;
use tokio::process::Command;
use tracing::{info, error, debug};
use crate::errors::AgentError;

pub async fn deploy_compose(target_dir: &str) -> Result<(), AgentError> {
    info!("Deploying with Docker Compose in: {}", target_dir);

    // #region agent log
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
        use std::io::Write;
        writeln!(f, r#"{{"location":"compose.rs:9","message":"Docker Compose started","data":{{"target_dir":"{}","exists":{}}},"timestamp":{},"hypothesisId":"H4"}}"#, target_dir, Path::new(target_dir).exists(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
    });
    // #endregion

    let path = Path::new(target_dir);
    if !path.exists() {
        // #region agent log
        let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
            use std::io::Write;
            writeln!(f, r#"{{"location":"compose.rs:13","message":"Target directory does not exist","data":{{"target_dir":"{}"}},"timestamp":{},"hypothesisId":"H4"}}"#, target_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
        });
        // #endregion
        return Err(AgentError::DeployError(format!("Target directory does not exist: {}", target_dir)));
    }

    // Run docker-compose up -d
    debug!("Running docker-compose up -d...");
    let status = Command::new("docker-compose")
        .current_dir(path)
        .args(["up", "-d", "--build"])
        .status()
        .await
        .map_err(|e| AgentError::DeployError(format!("Failed to run docker-compose: {}", e)))?;

    if !status.success() {
        // Try 'docker compose' (newer version)
        debug!("docker-compose failed, trying 'docker compose'...");
        let status = Command::new("docker")
            .current_dir(path)
            .args(["compose", "up", "-d", "--build"])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run docker compose: {}", e)))?;
        
        if !status.success() {
            // #region agent log
            let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
                use std::io::Write;
                writeln!(f, r#"{{"location":"compose.rs:36","message":"Docker Compose failed","data":{{"target_dir":"{}"}},"timestamp":{},"hypothesisId":"H4"}}"#, target_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
            });
            // #endregion
            return Err(AgentError::DeployError("Docker Compose failed".to_string()));
        }
    }

    info!("Successfully deployed Docker Compose application");
    // #region agent log
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
        use std::io::Write;
        writeln!(f, r#"{{"location":"compose.rs:40","message":"Docker Compose completed","data":{{"target_dir":"{}"}},"timestamp":{},"hypothesisId":"H4"}}"#, target_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
    });
    // #endregion
    Ok(())
}
