//! WebSocket Relay worker for real-time communication.
//!
//! Maintains a persistent connection to the backend relay endpoint. Incoming
//! commands are dispatched to handlers for: deployments, terminal sessions,
//! file operations, and network scanning.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Exponential backoff with full jitter.
/// Returns a delay in the range [0, min(cap, base * 2^attempt)].
fn backoff_delay(attempt: u32, base_secs: u64, cap_secs: u64) -> Duration {
    let exp = base_secs.saturating_mul(1u64.checked_shl(attempt.min(62)).unwrap_or(u64::MAX));
    let ceiling = exp.min(cap_secs);
    // Full jitter: pick uniformly from [0, ceiling]
    let jitter_ms = if ceiling > 0 {
        let ceiling_ms = ceiling.saturating_mul(1000);
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(12345) as u64;
        (seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33)
            % ceiling_ms
    } else {
        0
    };
    Duration::from_millis(jitter_ms)
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{handshake::client::generate_key, http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::authn::token_mngr::{TokenManager, TokenManagerExt};
use crate::errors::AgentError;
use crate::terminal::TerminalSession;

/// Alias for the WS outgoing message sender.
type WsTx = mpsc::UnboundedSender<Message>;

/// Shared terminal session map: session_id -> TerminalSession.
type Sessions = Arc<Mutex<HashMap<String, TerminalSession>>>;

/// Relay worker options.
#[derive(Debug, Clone)]
pub struct Options {
    /// Reconnect delay on failure.
    pub reconnect_delay: Duration,

    /// Heartbeat interval.
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

/// Run the relay worker. Reconnects automatically on failure with exponential
/// backoff and full jitter to prevent thundering-herd storms when the server
/// restarts across a large fleet.
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

    // Backoff state: resets to 0 on every successful connection.
    let mut attempt: u32 = 0;

    loop {
        // Exit immediately if shutdown has been signalled.
        tokio::select! {
            _ = &mut shutdown_signal => {
                info!("Relay worker shutting down...");
                return;
            }
            _ = async {} => {}
        }

        let device_id = match token_mngr.get_device_id().await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to get device ID: {}", e);
                let delay = backoff_delay(attempt, 2, 60);
                info!("Retrying in {:.1}s (attempt {})", delay.as_secs_f32(), attempt + 1);
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
                continue;
            }
        };

        let token = match token_mngr.get_token().await {
            Ok(t) => t.raw,
            Err(e) => {
                error!("Failed to get token: {}", e);
                let delay = backoff_delay(attempt, 2, 60);
                info!("Retrying in {:.1}s (attempt {})", delay.as_secs_f32(), attempt + 1);
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
                continue;
            }
        };

        info!("Connecting to relay: {} (attempt {})", relay_url, attempt + 1);

        let ws_key = generate_key();
        let request = Request::builder()
            .uri(relay_url.as_str())
            .method("GET")
            .header("Host", relay_url.host_str().unwrap_or("localhost"))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", &ws_key)
            .header("X-Device-ID", &device_id)
            .header("X-Device-Secret", &token)
            .body(())
            .unwrap();

        match connect_async(request).await {
            Ok((ws_stream, _)) => {
                info!("Connected to WebSocket relay");
                // Connection established — reset backoff counter.
                attempt = 0;

                let (ws_sink, mut ws_rx) = ws_stream.split();

                // Channel for sending outgoing WS messages from handlers
                let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

                // Spawn a task that forwards channel messages to the WS sink
                tokio::spawn(async move {
                    let mut sink = ws_sink;
                    while let Some(msg) = rx.recv().await {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                });

                // Terminal sessions are scoped to this connection
                let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));

                let mut heartbeat_tick = tokio::time::interval(options.heartbeat_interval);

                'inner: loop {
                    tokio::select! {
                        _ = &mut shutdown_signal => {
                            info!("Relay worker shutting down connection...");
                            return;
                        }
                        _ = heartbeat_tick.tick() => {
                            let ping = serde_json::json!({"type": "ping"}).to_string();
                            let _ = tx.send(Message::Text(ping.into()));
                        }
                        msg = ws_rx.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    handle_message(
                                        &text,
                                        tx.clone(),
                                        Arc::clone(&sessions),
                                    )
                                    .await;
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Relay closed connection");
                                    break 'inner;
                                }
                                Some(Err(e)) => {
                                    error!("Relay WebSocket error: {}", e);
                                    break 'inner;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let delay = backoff_delay(attempt, 2, 60);
                error!(
                    "Failed to connect to relay: {}. Retrying in {:.1}s (attempt {})",
                    e, delay.as_secs_f32(), attempt + 1
                );
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
                continue;
            }
        }

        // Graceful disconnect — apply a short jittered delay before reconnecting.
        let delay = backoff_delay(attempt, 2, 60);
        info!("Relay disconnected. Reconnecting in {:.1}s...", delay.as_secs_f32());
        tokio::time::sleep(delay).await;
        attempt = attempt.saturating_add(1);
    }
}

// ---------------------------------------------------------------------------
// URL helpers
// ---------------------------------------------------------------------------

fn build_relay_url(backend_url: &str) -> Result<Url, AgentError> {
    let mut url =
        Url::parse(backend_url).map_err(|e| AgentError::ConfigError(e.to_string()))?;

    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        _ => {
            return Err(AgentError::ConfigError(
                "Invalid backend URL scheme".to_string(),
            ))
        }
    };

    url.set_scheme(scheme)
        .map_err(|_| AgentError::ConfigError("Failed to set scheme".to_string()))?;

    url.set_path(&format!(
        "{}/agent-relay/ws",
        url.path().trim_end_matches('/')
    ));

    Ok(url)
}

// ---------------------------------------------------------------------------
// Message dispatcher
// ---------------------------------------------------------------------------

async fn handle_message(text: &str, tx: WsTx, sessions: Sessions) {
    debug!("Received relay message: {}", text);

    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(_) => return,
    };

    // The server wraps commands as:
    //   {"type": "command", "msg_id": "...", "command_type": "...", "payload": {...}}
    // Push messages have "type" set directly to the message type (e.g. "new_deployment").
    let msg_type = if msg.get("type").and_then(|t| t.as_str()) == Some("command") {
        msg.get("command_type").and_then(|t| t.as_str())
    } else {
        msg.get("type").and_then(|t| t.as_str())
    };

    // msg_id is used to correlate request/response pairs
    let msg_id = msg
        .get("msg_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let payload = &msg["payload"];

    match msg_type {
        // ── Legacy: deployment trigger (fire-and-forget) ──────────────────
        Some("new_deployment") => {
            let id = msg
                .get("deployment_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            info!("Real-time trigger: new deployment pending: {}", id);
        }

        Some("pong") => {
            debug!("Relay pong received");
        }

        // ── Terminal: create session ──────────────────────────────────────
        Some("terminal_create") => {
            let session_id = payload["session_id"]
                .as_str()
                .unwrap_or(&msg_id)
                .to_string();
            let cols = payload["cols"].as_u64().unwrap_or(80) as u16;
            let rows = payload["rows"].as_u64().unwrap_or(24) as u16;

            let resp = match TerminalSession::new(
                session_id.clone(),
                cols,
                rows,
                tx.clone(),
            ) {
                Ok(session) => {
                    sessions.lock().await.insert(session_id.clone(), session);
                    info!("Terminal session created: {}", session_id);
                    serde_json::json!({
                        "type": "response",
                        "msg_id": msg_id,
                        "result": { "session_id": session_id },
                        "error": null
                    })
                }
                Err(e) => {
                    error!("Terminal create failed: {}", e);
                    serde_json::json!({
                        "type": "response",
                        "msg_id": msg_id,
                        "result": null,
                        "error": e.to_string()
                    })
                }
            };
            let _ = tx.send(Message::Text(resp.to_string().into()));
        }

        // ── Terminal: send keystrokes ─────────────────────────────────────
        Some("terminal_input") => {
            let session_id = payload["session_id"].as_str().unwrap_or_default();
            let data_b64 = payload["data"].as_str().unwrap_or_default();

            if let Ok(bytes) = BASE64.decode(data_b64) {
                let sessions_guard = sessions.lock().await;
                if let Some(session) = sessions_guard.get(session_id) {
                    if let Err(e) = session.write_input(&bytes) {
                        warn!("Terminal input error for {}: {}", session_id, e);
                    }
                }
            }
        }

        // ── Terminal: close session ───────────────────────────────────────
        Some("terminal_close") => {
            let session_id = payload["session_id"].as_str().unwrap_or_default();
            sessions.lock().await.remove(session_id);
            info!("Terminal session closed: {}", session_id);
        }

        // ── File: list directory ──────────────────────────────────────────
        Some("file_list") => {
            let path = payload["path"].as_str().unwrap_or("/");
            let result = crate::filesys::relay::list_directory(path).await;
            send_response(&tx, &msg_id, result.map(|files| serde_json::json!({ "files": files })));
        }

        // ── File: read (returns Base64 content) ───────────────────────────
        Some("file_read") => {
            let path = payload["path"].as_str().unwrap_or("");
            let result = crate::filesys::relay::read_file(path).await;
            send_response(&tx, &msg_id, result.map(|content| serde_json::json!({ "content": content })));
        }

        // ── File: write (Base64-encoded content) ─────────────────────────
        Some("file_write") => {
            let path = payload["path"].as_str().unwrap_or("");
            let content = payload["content"].as_str().unwrap_or("");
            let result = crate::filesys::relay::write_file(path, content).await;
            send_response(&tx, &msg_id, result.map(|_| serde_json::json!({ "ok": true })));
        }

        // ── File: delete ──────────────────────────────────────────────────
        Some("file_delete") => {
            let path = payload["path"].as_str().unwrap_or("");
            let result = crate::filesys::relay::delete_path(path).await;
            send_response(&tx, &msg_id, result.map(|_| serde_json::json!({ "ok": true })));
        }

        // ── Network scan ──────────────────────────────────────────────────
        Some("scan_network") => {
            let subnet = payload["subnet"].as_str().unwrap_or("192.168.1.0/24");
            info!("Starting network scan on subnet: {}", subnet);
            let devices = crate::scanner::scan_subnet(subnet).await;
            send_response(
                &tx,
                &msg_id,
                Ok(serde_json::json!({ "devices": devices })),
            );
        }

        _ => {
            warn!("Unknown relay message type: {:?}", msg_type);
        }
    }
}

// ---------------------------------------------------------------------------
// Response helper
// ---------------------------------------------------------------------------

/// Send a standard request/response envelope back through the relay channel.
fn send_response(
    tx: &WsTx,
    msg_id: &str,
    result: Result<serde_json::Value, crate::errors::AgentError>,
) {
    let resp = match result {
        Ok(value) => serde_json::json!({
            "type": "response",
            "msg_id": msg_id,
            "result": value,
            "error": null
        }),
        Err(e) => serde_json::json!({
            "type": "response",
            "msg_id": msg_id,
            "result": null,
            "error": e.to_string()
        }),
    };
    let _ = tx.send(Message::Text(resp.to_string().into()));
}
