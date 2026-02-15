//! WebSocket Relay worker for real-time communication

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::authn::token_mngr::TokenManager;
use crate::errors::AgentError;

/// Relay worker options
#[derive(Debug, Clone)]
pub struct Options {
    /// Reconnect delay on failure
    pub reconnect_delay: Duration,

    /// Heartbeat interval
    pub heartbeat_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            reconnect_delay: Duration::from_secs(5),
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

/// Run the Relay worker
pub async fn run(
    options: &Options,
    token_mngr: Arc<TokenManager>,
    backend_url: String,
    mut shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send>>,
) {
    info!("Relay worker starting...");

    let relay_url = match build_relay_url(&backend_url) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to build relay URL: {}", e);
            return;
        }
    };

    loop {
        // Check for shutdown before attempting connection
        tokio::select! {
            _ = &mut shutdown_signal => {
                info!("Relay worker shutting down...");
                return;
            }
            _ = async {} => {} // Continue
        }

        let device_id = match token_mngr.get_device_id().await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to get device ID: {}", e);
                tokio::time::sleep(options.reconnect_delay).await;
                continue;
            }
        };

        let token = match token_mngr.get_token().await {
            Ok(t) => t.raw,
            Err(e) => {
                error!("Failed to get token: {}", e);
                tokio::time::sleep(options.reconnect_delay).await;
                continue;
            }
        };

        info!("Connecting to relay: {}", relay_url);
        
        // Prepare request with authentication headers
        let request = http::Request::builder()
            .uri(relay_url.as_str())
            .header("X-Device-ID", &device_id)
            .header("X-Device-Secret", &token) // Using token as secret for now
            .header("User-Agent", "Ajime-Agent")
            .body(())
            .unwrap();

        let connection = connect_async(request).await;

        match connection {
            Ok((mut ws_stream, _)) => {
                info!("✓ Connected to WebSocket Relay");
                
                let mut heartbeat_tick = tokio::time::interval(options.heartbeat_interval);
                
                loop {
                    tokio::select! {
                        _ = &mut shutdown_signal => {
                            info!("Relay worker shutting down connection...");
                            let _ = ws_stream.close(None).await;
                            return;
                        }
                        _ = heartbeat_tick.tick() => {
                            let ping = serde_json::json!({"type": "ping"}).to_string();
                            if let Err(e) = ws_stream.send(Message::Text(ping.into())).await {
                                warn!("Failed to send heartbeat: {}", e);
                                break;
                            }
                        }
                        msg = ws_stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    handle_message(&text).await;
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Relay closed connection");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("Relay WebSocket error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to relay: {}. Retrying in {:?}...", e, options.reconnect_delay);
            }
        }

        tokio::time::sleep(options.reconnect_delay).await;
    }
}

fn build_relay_url(backend_url: &str) -> Result<Url, AgentError> {
    let mut url = Url::parse(backend_url).map_err(|e| AgentError::ConfigError(e.to_string()))?;
    
    // Change http/https to ws/wss
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        _ => return Err(AgentError::ConfigError("Invalid backend URL scheme".to_string())),
    };
    
    url.set_scheme(scheme).map_err(|_| AgentError::ConfigError("Failed to set scheme".to_string()))?;
    
    // Append /agent-relay/ws
    url.set_path(&format!("{}/agent-relay/ws", url.path().trim_end_matches('/')));
    
    Ok(url)
}

async fn handle_message(text: &str) {
    debug!("Received relay message: {}", text);
    
    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(_) => return,
    };
    
    match msg.get("type").and_then(|t| t.as_str()) {
        Some("new_deployment") => {
            let deployment_id = msg.get("deployment_id").and_then(|v| v.as_str()).unwrap_or("unknown");
            info!("🚀 Real-time trigger: New deployment pending: {}", deployment_id);
            // The deployer worker poller will pick it up on next tick (or we could use a channel to trigger it immediately)
        }
        Some("pong") => {
            debug!("Relay pong received");
        }
        _ => {
            warn!("Unknown relay message type: {:?}", msg.get("type"));
        }
    }
}
