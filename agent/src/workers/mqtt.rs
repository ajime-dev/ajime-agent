//! MQTT worker for real-time communication

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tracing::{debug, error, info, warn};

use crate::authn::token_mngr::TokenManagerExt;
use crate::filesys::file::File;
use crate::mqtt::client::{MqttAddress, MqttClient, MqttCommand};
use crate::mqtt::topics::Topics;
use crate::sync::syncer::Syncer;

/// MQTT worker options
#[derive(Debug, Clone)]
pub struct Options {
    /// MQTT broker address
    pub broker_address: MqttAddress,

    /// Reconnect delay on failure
    pub reconnect_delay: Duration,

    /// Max reconnect attempts before giving up
    pub max_reconnect_attempts: u32,

    /// Status publish interval
    pub status_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            broker_address: MqttAddress::default(),
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            status_interval: Duration::from_secs(60),
        }
    }
}

/// Run the MQTT worker
pub async fn run<S, T, F>(
    options: &Options,
    token_mngr: &T,
    syncer: &Syncer,
    device_file: &File,
    sleep_fn: S,
    shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send>>,
) where
    S: Fn(Duration) -> F,
    F: Future<Output = ()>,
    T: TokenManagerExt,
{
    if options.broker_address.host.is_empty() {
        info!("MQTT host not configured, MQTT worker will not start.");
        return;
    }

    info!("MQTT worker starting...");

    let mut reconnect_attempts = 0;

    loop {
        // Check for shutdown
        tokio::select! {
            _ = &mut Box::pin(std::future::pending::<()>()) => {},
            _ = tokio::time::sleep(Duration::from_millis(100)) => {},
        }

        // Get device ID and token
        let device_id = match token_mngr.get_device_id().await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to get device ID: {}", e);
                sleep_fn(options.reconnect_delay).await;
                continue;
            }
        };

        let token = match token_mngr.get_token().await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to get token: {}", e);
                sleep_fn(options.reconnect_delay).await;
                continue;
            }
        };

        // Connect to MQTT broker
        info!("Connecting to MQTT broker: {}:{}", options.broker_address.host, options.broker_address.port);
        let mut client = match MqttClient::new(&options.broker_address, &device_id, &token.raw).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create MQTT client: {}", e);
                reconnect_attempts += 1;
                if reconnect_attempts >= options.max_reconnect_attempts {
                    error!("Max reconnect attempts reached, giving up");
                    return;
                }
                sleep_fn(options.reconnect_delay).await;
                continue;
            }
        };

        // Subscribe to topics
        if let Err(e) = client.subscribe_commands().await {
            error!("Failed to subscribe to commands: {}", e);
            sleep_fn(options.reconnect_delay).await;
            continue;
        }

        reconnect_attempts = 0;
        info!("MQTT worker connected and subscribed");

        // Main event loop
        loop {
            match client.poll().await {
                Ok(Some(msg)) => {
                    debug!("Received MQTT message on topic: {}", msg.topic);
                    
                    if Topics::is_command_topic(&msg.topic) {
                        if let Ok(command) = msg.parse_json::<MqttCommand>() {
                            handle_command(&command, syncer).await;
                        }
                    } else if Topics::is_control_topic(&msg.topic) {
                        if let Some(workflow_id) = Topics::parse_workflow_id(&msg.topic) {
                            if let Ok(command) = msg.parse_json::<MqttCommand>() {
                                handle_workflow_control(&workflow_id, &command, syncer).await;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No message, continue
                }
                Err(e) => {
                    warn!("MQTT poll error: {}, reconnecting...", e);
                    break;
                }
            }

            // Small delay to prevent busy loop
            sleep_fn(Duration::from_millis(10)).await;
        }

        // Reconnect delay
        sleep_fn(options.reconnect_delay).await;
    }
}

async fn handle_command(command: &MqttCommand, syncer: &Syncer) {
    info!("Handling command: {}", command.command);

    match command.command.as_str() {
        "sync" => {
            info!("Sync command received, triggering sync...");
            if let Err(e) = syncer.trigger_sync().await {
                error!("Sync failed: {}", e);
            }
        }
        "restart" => {
            info!("Restart command received");
            // In production, this would trigger a graceful restart
        }
        "update_settings" => {
            info!("Update settings command received");
            // Handle settings update
        }
        _ => {
            warn!("Unknown command: {}", command.command);
        }
    }
}

async fn handle_workflow_control(workflow_id: &str, command: &MqttCommand, syncer: &Syncer) {
    info!("Handling workflow control for {}: {}", workflow_id, command.command);

    match command.command.as_str() {
        "start" => {
            info!("Start workflow: {}", workflow_id);
            // Start workflow execution
        }
        "stop" => {
            info!("Stop workflow: {}", workflow_id);
            // Stop workflow execution
        }
        "pause" => {
            info!("Pause workflow: {}", workflow_id);
            // Pause workflow execution
        }
        "resume" => {
            info!("Resume workflow: {}", workflow_id);
            // Resume workflow execution
        }
        _ => {
            warn!("Unknown workflow control command: {}", command.command);
        }
    }
}
