//! File operations

use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::errors::AgentError;

/// A file wrapper with path
#[derive(Debug, Clone)]
pub struct File {
    path: PathBuf,
}

impl File {
    /// Create a new file reference
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if the file exists
    pub async fn exists(&self) -> bool {
        fs::metadata(&self.path).await.is_ok()
    }

    /// Read file contents as string
    pub async fn read_string(&self) -> Result<String, AgentError> {
        let mut file = fs::File::open(&self.path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        Ok(contents)
    }

    /// Read file contents as bytes
    pub async fn read_bytes(&self) -> Result<Vec<u8>, AgentError> {
        let mut file = fs::File::open(&self.path).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;
        Ok(contents)
    }

    /// Read file as JSON
    pub async fn read_json<T: DeserializeOwned>(&self) -> Result<T, AgentError> {
        let contents = self.read_string().await?;
        let value = serde_json::from_str(&contents)?;
        Ok(value)
    }

    /// Write string to file
    pub async fn write_string(&self, contents: &str) -> Result<(), AgentError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&self.path).await?;
        file.write_all(contents.as_bytes()).await?;
        file.sync_all().await?;
        Ok(())
    }

    /// Write bytes to file
    pub async fn write_bytes(&self, contents: &[u8]) -> Result<(), AgentError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&self.path).await?;
        file.write_all(contents).await?;
        file.sync_all().await?;
        Ok(())
    }

    /// Write JSON to file
    pub async fn write_json<T: Serialize>(&self, value: &T) -> Result<(), AgentError> {
        let contents = serde_json::to_string_pretty(value)?;
        self.write_string(&contents).await
    }

    /// Delete the file
    pub async fn delete(&self) -> Result<(), AgentError> {
        if self.exists().await {
            fs::remove_file(&self.path).await?;
        }
        Ok(())
    }

    /// Set file permissions to owner-read/write only (0o600) on Unix.
    ///
    /// A no-op on non-Unix platforms.
    pub async fn set_permissions_600(&self) -> Result<(), AgentError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = fs::metadata(&self.path).await?;
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.path, perms).await?;
        }
        Ok(())
    }

    /// Atomic write using a temporary file
    pub async fn write_atomic(&self, contents: &[u8]) -> Result<(), AgentError> {
        let temp_path = self.path.with_extension("tmp");

        // Write to temp file
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(contents).await?;
        file.sync_all().await?;
        drop(file);

        // Rename to target
        fs::rename(&temp_path, &self.path).await?;
        Ok(())
    }
}
