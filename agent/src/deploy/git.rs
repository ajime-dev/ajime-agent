//! Git deployment executor

use std::path::Path;
use tokio::process::Command;
use tracing::{info, error, debug};
use crate::errors::AgentError;

/// Sync a git repository (clone or pull)
pub async fn sync_repository(
    repo_url: &str,
    branch: &str,
    target_dir: &str
) -> Result<(), AgentError> {
    info!("Syncing Git repository: {} (branch: {}) to {}", repo_url, branch, target_dir);

    // #region agent log
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
        use std::io::Write;
        writeln!(f, r#"{{"location":"git.rs:14","message":"Git sync started","data":{{"repo_url":"{}","branch":"{}","target_dir":"{}","exists":{}}},"timestamp":{},"hypothesisId":"H5"}}"#, repo_url, branch, target_dir, Path::new(target_dir).exists(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
    });
    // #endregion

    let path = Path::new(target_dir);

    // Clone or Pull
    if path.exists() {
        debug!("Target directory exists, pulling updates...");
        let status = Command::new("git")
            .current_dir(path)
            .args(["pull", "origin", branch])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run git pull: {}", e)))?;
        
        if !status.success() {
            // #region agent log
            let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
                use std::io::Write;
                writeln!(f, r#"{{"location":"git.rs:29","message":"Git pull failed","data":{{"target_dir":"{}"}},"timestamp":{},"hypothesisId":"H5"}}"#, target_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
            });
            // #endregion
            return Err(AgentError::DeployError("Git pull failed".to_string()));
        }
    } else {
        debug!("Cloning repository to {}...", target_dir);
        let status = Command::new("git")
            .args(["clone", "-b", branch, repo_url, target_dir])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run git clone: {}", e)))?;
        
        if !status.success() {
            // #region agent log
            let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
                use std::io::Write;
                writeln!(f, r#"{{"location":"git.rs:40","message":"Git clone failed","data":{{"repo_url":"{}","target_dir":"{}"}},"timestamp":{},"hypothesisId":"H5"}}"#, repo_url, target_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
            });
            // #endregion
            return Err(AgentError::DeployError("Git clone failed".to_string()));
        }
    }

    info!("Successfully synced Git repository");
    // #region agent log
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(r"c:\Users\shach\Desktop\Projects\Ajime\.cursor\debug.log").and_then(|mut f| {
        use std::io::Write;
        writeln!(f, r#"{{"location":"git.rs:44","message":"Git sync completed","data":{{"target_dir":"{}"}},"timestamp":{},"hypothesisId":"H5"}}"#, target_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
    });
    // #endregion
    Ok(())
}

pub async fn deploy_git(
    repo_url: &str, 
    branch: &str, 
    install_cmd: &str, 
    run_cmd: &str,
    target_dir: &str
) -> Result<(), AgentError> {
    info!("Deploying Git repository: {} (branch: {})", repo_url, branch);

    let path = Path::new(target_dir);

    // 1. Clone or Pull
    if path.exists() {
        debug!("Target directory exists, pulling updates...");
        let status = Command::new("git")
            .current_dir(path)
            .args(["pull", "origin", branch])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run git pull: {}", e)))?;
        
        if !status.success() {
            return Err(AgentError::DeployError("Git pull failed".to_string()));
        }
    } else {
        debug!("Cloning repository to {}...", target_dir);
        let status = Command::new("git")
            .args(["clone", "-b", branch, repo_url, target_dir])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run git clone: {}", e)))?;
        
        if !status.success() {
            return Err(AgentError::DeployError("Git clone failed".to_string()));
        }
    }

    // 2. Install dependencies
    if !install_cmd.is_empty() {
        info!("Running install command: {}", install_cmd);
        let status = Command::new("bash")
            .current_dir(path)
            .args(["-c", install_cmd])
            .status()
            .await
            .map_err(|e| AgentError::DeployError(format!("Failed to run install command: {}", e)))?;
        
        if !status.success() {
            return Err(AgentError::DeployError("Install command failed".to_string()));
        }
    }

    // 3. Run application (simplified: non-blocking or managed process would be better)
    if !run_cmd.is_empty() {
        info!("Starting application: {}", run_cmd);
        // Note: In production, this should be managed by a process supervisor
        let cmd = format!("nohup {} > app.log 2>&1 &", run_cmd);
        let _ = Command::new("bash")
            .current_dir(path)
            .args(["-c", &cmd])
            .status()
            .await;
    }

    info!("Successfully deployed Git repository");
    Ok(())
}
