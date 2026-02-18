//! PTY-based terminal session for remote access via the relay channel.
//!
//! Each session spawns a shell inside a pseudo-terminal and forwards I/O
//! through the WebSocket relay sender channel.

use std::io::Read;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{info, warn};

use crate::errors::AgentError;

/// An active terminal session backed by a PTY.
pub struct TerminalSession {
    /// Write end of the PTY master — protected by a mutex so it can be used
    /// from async context without blocking the executor.
    writer: Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>,
}

impl TerminalSession {
    /// Spawn a new shell in a PTY and start forwarding output through `tx`.
    ///
    /// Returns immediately; output is streamed asynchronously via the channel.
    pub fn new(
        session_id: String,
        cols: u16,
        rows: u16,
        tx: mpsc::UnboundedSender<Message>,
    ) -> Result<Self, AgentError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AgentError::Internal(format!("openpty failed: {e}")))?;

        // Detect available shell
        let shell = if std::path::Path::new("/bin/bash").exists() {
            "/bin/bash"
        } else {
            "/bin/sh"
        };

        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");

        // Spawn shell inside the slave PTY (slave is consumed here)
        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AgentError::Internal(format!("spawn_command failed: {e}")))?;

        // Obtain the read and write halves of the master PTY
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AgentError::Internal(format!("clone_reader failed: {e}")))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AgentError::Internal(format!("take_writer failed: {e}")))?;

        let writer = Arc::new(std::sync::Mutex::new(writer));

        // Spawn a blocking thread to read PTY output and forward it
        let sid = session_id.clone();
        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];

            info!("Terminal read loop started for session {}", sid);

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = BASE64.encode(&buf[..n]);
                        let msg = serde_json::json!({
                            "type": "terminal_output",
                            "session_id": &sid,
                            "data": data,
                        })
                        .to_string();

                        if tx.send(Message::Text(msg.into())).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Terminal read error for session {}: {}", sid, e);
                        break;
                    }
                }
            }

            // Notify the server that this session has ended
            let close_msg = serde_json::json!({
                "type": "terminal_closed",
                "session_id": &sid,
            })
            .to_string();
            let _ = tx.send(Message::Text(close_msg.into()));

            info!("Terminal read loop ended for session {}", sid);
        });

        Ok(Self { writer })
    }

    /// Write raw bytes (keystrokes) into the PTY.
    pub fn write_input(&self, data: &[u8]) -> Result<(), AgentError> {
        use std::io::Write;
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| AgentError::Internal("Terminal writer lock poisoned".into()))?;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }
}
