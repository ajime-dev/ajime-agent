//! Storage layout configuration

use std::path::PathBuf;

use crate::filesys::dir::Dir;
use crate::filesys::file::File;

/// Storage layout for the agent
#[derive(Debug, Clone)]
pub struct StorageLayout {
    /// Base directory for all storage
    pub base_dir: PathBuf,
}

impl StorageLayout {
    /// Create a new storage layout
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Get the device file path
    pub fn device_file(&self) -> File {
        File::new(self.base_dir.join("device.json"))
    }

    /// Get the settings file path
    pub fn settings_file(&self) -> File {
        File::new(self.base_dir.join("settings.json"))
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> Dir {
        Dir::new(self.base_dir.join("cache"))
    }

    /// Get the workflows cache directory
    pub fn workflows_cache_dir(&self) -> Dir {
        Dir::new(self.base_dir.join("cache").join("workflows"))
    }

    /// Get the configs cache directory
    pub fn configs_cache_dir(&self) -> Dir {
        Dir::new(self.base_dir.join("cache").join("configs"))
    }

    /// Get the deployment directory
    pub fn deployment_dir(&self) -> Dir {
        Dir::new(self.base_dir.join("deployments"))
    }

    /// Get the logs directory
    pub fn logs_dir(&self) -> Dir {
        Dir::new(self.base_dir.join("logs"))
    }

    /// Get the tokens directory (for secure token storage)
    pub fn tokens_dir(&self) -> Dir {
        Dir::new(self.base_dir.join("tokens"))
    }

    /// Setup the storage layout (create directories)
    pub async fn setup(&self) -> Result<(), crate::errors::AgentError> {
        self.cache_dir().create().await?;
        self.workflows_cache_dir().create().await?;
        self.configs_cache_dir().create().await?;
        self.deployment_dir().create().await?;
        self.logs_dir().create().await?;
        self.tokens_dir().create().await?;
        Ok(())
    }
}

impl Default for StorageLayout {
    fn default() -> Self {
        // Use /etc/ajime on Linux, or user home directory on other platforms
        #[cfg(target_os = "linux")]
        let base_dir = PathBuf::from("/etc/ajime");

        #[cfg(not(target_os = "linux"))]
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ajime");

        Self::new(base_dir)
    }
}

// Add dirs crate functionality inline for cross-platform support
#[cfg(not(target_os = "linux"))]
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}
