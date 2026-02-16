//! MQTT client implementation

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::errors::AgentError;

/// MQTT broker address
#[derive(Debug, Clone)]
pub struct MqttAddress {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
}

impl Default for MqttAddress {
    fn default() -> Self {
        Self {
            host: "".to_string(),
            port: 8883,
            use_tls: true,
        }
    }
}

/// MQTT client wrapper
pub struct MqttClient {
    client: AsyncClient,
    eventloop: EventLoop,
    device_id: String,
}

impl MqttClient {
    /// Create a new MQTT client
    pub async fn new(
        address: &MqttAddress,
        device_id: &str,
        token: &str,
    ) -> Result<Self, AgentError> {
        if address.host.is_empty() {
            return Err(AgentError::MqttError("MQTT host is not configured".to_string()));
        }

        let client_id = format!("ajigent-{}", device_id);

        let mut options = MqttOptions::new(&client_id, &address.host, address.port);
        options.set_keep_alive(std::time::Duration::from_secs(30));
        options.set_credentials(device_id, token);

        // Note: TLS configuration would be added here in production
        // if address.use_tls {
        //     options.set_transport(Transport::tls(...));
        // }

        let (client, eventloop) = AsyncClient::new(options, 10);

        Ok(Self {
            client,
            eventloop,
            device_id: device_id.to_string(),
        })
    }

    /// Subscribe to device command topic
    pub async fn subscribe_commands(&self) -> Result<(), AgentError> {
        let topic = format!("ajime/device/{}/command", self.device_id);
        self.client
            .subscribe(&topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        info!("Subscribed to: {}", topic);
        Ok(())
    }

    /// Subscribe to workflow control topic
    pub async fn subscribe_workflow_control(&self, workflow_id: &str) -> Result<(), AgentError> {
        let topic = format!("ajime/workflow/{}/control", workflow_id);
        self.client
            .subscribe(&topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        info!("Subscribed to: {}", topic);
        Ok(())
    }

    /// Publish device status
    pub async fn publish_status(&self, status: &DeviceStatus) -> Result<(), AgentError> {
        let topic = format!("ajime/device/{}/status", self.device_id);
        let payload = serde_json::to_vec(status)
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        
        debug!("Published status to: {}", topic);
        Ok(())
    }

    /// Publish telemetry data
    pub async fn publish_telemetry(&self, telemetry: &serde_json::Value) -> Result<(), AgentError> {
        let topic = format!("ajime/device/{}/telemetry", self.device_id);
        let payload = serde_json::to_vec(telemetry)
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        
        self.client
            .publish(&topic, QoS::AtMostOnce, false, payload)
            .await
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        
        debug!("Published telemetry to: {}", topic);
        Ok(())
    }

    /// Poll for events
    pub async fn poll(&mut self) -> Result<Option<MqttMessage>, AgentError> {
        match self.eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                let topic = publish.topic.clone();
                let payload = publish.payload.to_vec();
                
                debug!("Received message on topic: {}", topic);
                
                Ok(Some(MqttMessage { topic, payload }))
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                info!("MQTT connected");
                Ok(None)
            }
            Ok(Event::Incoming(Packet::SubAck(_))) => {
                debug!("Subscription acknowledged");
                Ok(None)
            }
            Ok(_) => Ok(None),
            Err(e) => {
                warn!("MQTT poll error: {}", e);
                Err(AgentError::MqttError(e.to_string()))
            }
        }
    }

    /// Disconnect from broker
    pub async fn disconnect(&self) -> Result<(), AgentError> {
        self.client
            .disconnect()
            .await
            .map_err(|e| AgentError::MqttError(e.to_string()))?;
        info!("MQTT disconnected");
        Ok(())
    }
}

/// MQTT message
#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

impl MqttMessage {
    /// Parse payload as JSON
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, AgentError> {
        serde_json::from_slice(&self.payload).map_err(|e| AgentError::MqttError(e.to_string()))
    }
}

/// Device status for MQTT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub status: String,
    pub agent_version: String,
    pub uptime_secs: u64,
    pub workflows_deployed: usize,
    pub workflows_running: usize,
}

/// MQTT command from backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttCommand {
    pub command: String,
    pub payload: Option<serde_json::Value>,
}
