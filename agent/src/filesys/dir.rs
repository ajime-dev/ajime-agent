//! Directory operations

use std::path::{Path, PathBuf};

use tokio::fs;

use crate::errors::AgentError;

/// A directory wrapper with path
#[derive(Debug, Clone)]
pub struct Dir {
    path: PathBuf,
}

impl Dir {
    /// Create a new directory reference
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Get the directory path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if the directory exists
    pub async fn exists(&self) -> bool {
        fs::metadata(&self.path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Create the directory (and parents)
    pub async fn create(&self) -> Result<(), AgentError> {
        fs::create_dir_all(&self.path).await?;
        Ok(())
    }

    /// Delete the directory and all contents
    pub async fn delete(&self) -> Result<(), AgentError> {
        if self.exists().await {
            fs::remove_dir_all(&self.path).await?;
        }
        Ok(())
    }

    /// List files in the directory
    pub async fn list_files(&self) -> Result<Vec<PathBuf>, AgentError> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(&self.path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }

        Ok(files)
    }

    /// List subdirectories
    pub async fn list_dirs(&self) -> Result<Vec<PathBuf>, AgentError> {
        let mut dirs = Vec::new();
        let mut entries = fs::read_dir(&self.path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }

        Ok(dirs)
    }

    /// Get a file within this directory
    pub fn file(&self, name: &str) -> crate::filesys::file::File {
        crate::filesys::file::File::new(self.path.join(name))
    }

    /// Get a subdirectory
    pub fn subdir(&self, name: &str) -> Dir {
        Dir::new(self.path.join(name))
    }

    /// Create a temporary directory
    pub async fn create_temp_dir(prefix: &str) -> Result<Dir, AgentError> {
        let temp_dir = std::env::temp_dir().join(format!("{}-{}", prefix, uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).await?;
        Ok(Dir::new(temp_dir))
    }
}
