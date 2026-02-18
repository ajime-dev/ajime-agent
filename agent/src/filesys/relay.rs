//! File system operations exposed through the relay channel.
//!
//! All file content is Base64-encoded so it can be safely embedded in JSON
//! messages over the WebSocket relay.

use std::time::UNIX_EPOCH;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::errors::AgentError;

/// Metadata for a single file or directory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Last-modified time as a Unix timestamp in seconds (None if unavailable).
    pub modified: Option<u64>,
}

/// List the contents of a directory.
///
/// Entries are sorted: directories first, then files, both alphabetically.
pub async fn list_directory(path: &str) -> Result<Vec<FileEntry>, AgentError> {
    let mut read_dir = fs::read_dir(path).await?;
    let mut entries = Vec::new();

    while let Some(entry) = read_dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let full_path = entry.path().to_string_lossy().into_owned();
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        entries.push(FileEntry {
            name,
            path: full_path,
            is_dir: metadata.is_dir(),
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            modified,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}

/// Read a file and return its contents as a Base64-encoded string.
pub async fn read_file(path: &str) -> Result<String, AgentError> {
    let bytes = fs::read(path).await?;
    Ok(BASE64.encode(&bytes))
}

/// Write Base64-encoded `content` to `path`, creating parent directories as needed.
pub async fn write_file(path: &str, content_b64: &str) -> Result<(), AgentError> {
    let bytes = BASE64
        .decode(content_b64)
        .map_err(|e| AgentError::ValidationError(format!("Invalid base64: {e}")))?;

    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(path, &bytes).await?;
    Ok(())
}

/// Delete a file or directory (recursive for directories).
pub async fn delete_path(path: &str) -> Result<(), AgentError> {
    let metadata = fs::metadata(path).await?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).await?;
    } else {
        fs::remove_file(path).await?;
    }
    Ok(())
}
