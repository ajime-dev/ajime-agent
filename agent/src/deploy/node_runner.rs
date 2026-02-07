//! Node runner implementations

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info};

use crate::errors::AgentError;
use crate::models::workflow::Node;

/// Node runner trait
#[async_trait]
pub trait NodeRunner: Send + Sync {
    /// Execute the node with given inputs
    async fn execute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError>;

    /// Get the node type
    fn node_type(&self) -> &str;

    /// Stop the node
    async fn stop(&self) -> Result<(), AgentError> {
        Ok(())
    }
}

/// Factory for creating node runners
pub struct NodeRunnerFactory;

impl NodeRunnerFactory {
    /// Create a node runner for the given node
    pub fn create(node: &Node) -> Result<Arc<dyn NodeRunner>, AgentError> {
        let runner: Arc<dyn NodeRunner> = match node.node_type.as_str() {
            "camera" | "camera_capture" => Arc::new(CameraNodeRunner::new(node)?),
            "gpio_read" | "gpio_input" => Arc::new(GpioReadNodeRunner::new(node)?),
            "gpio_write" | "gpio_output" => Arc::new(GpioWriteNodeRunner::new(node)?),
            "delay" | "timer" => Arc::new(DelayNodeRunner::new(node)?),
            "http_request" => Arc::new(HttpRequestNodeRunner::new(node)?),
            "log" | "debug" => Arc::new(LogNodeRunner::new(node)?),
            _ => Arc::new(PassthroughNodeRunner::new(node)?),
        };

        Ok(runner)
    }
}

/// Camera capture node runner
pub struct CameraNodeRunner {
    node_id: String,
    device: String,
    width: u32,
    height: u32,
}

impl CameraNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        let device = node
            .data
            .config
            .get("device")
            .and_then(|v| v.as_str())
            .unwrap_or("/dev/video0")
            .to_string();

        let width = node
            .data
            .config
            .get("width")
            .and_then(|v| v.as_u64())
            .unwrap_or(640) as u32;

        let height = node
            .data
            .config
            .get("height")
            .and_then(|v| v.as_u64())
            .unwrap_or(480) as u32;

        Ok(Self {
            node_id: node.id.clone(),
            device,
            width,
            height,
        })
    }
}

#[async_trait]
impl NodeRunner for CameraNodeRunner {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        debug!("Camera capture: {} ({}x{})", self.device, self.width, self.height);
        
        // In production, this would capture from the camera
        // For now, return a placeholder
        let mut outputs = HashMap::new();
        outputs.insert("frame".to_string(), Value::String("base64_frame_data".to_string()));
        outputs.insert("timestamp".to_string(), Value::Number(chrono::Utc::now().timestamp().into()));
        
        Ok(outputs)
    }

    fn node_type(&self) -> &str {
        "camera"
    }
}

/// GPIO read node runner
pub struct GpioReadNodeRunner {
    node_id: String,
    pin: u8,
}

impl GpioReadNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        let pin = node
            .data
            .config
            .get("pin")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| AgentError::ConfigError("GPIO pin not specified".to_string()))? as u8;

        Ok(Self {
            node_id: node.id.clone(),
            pin,
        })
    }
}

#[async_trait]
impl NodeRunner for GpioReadNodeRunner {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        debug!("GPIO read: pin {}", self.pin);
        
        // In production, this would read from GPIO
        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), Value::Bool(false));
        
        Ok(outputs)
    }

    fn node_type(&self) -> &str {
        "gpio_read"
    }
}

/// GPIO write node runner
pub struct GpioWriteNodeRunner {
    node_id: String,
    pin: u8,
}

impl GpioWriteNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        let pin = node
            .data
            .config
            .get("pin")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| AgentError::ConfigError("GPIO pin not specified".to_string()))? as u8;

        Ok(Self {
            node_id: node.id.clone(),
            pin,
        })
    }
}

#[async_trait]
impl NodeRunner for GpioWriteNodeRunner {
    async fn execute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        let value = inputs
            .get("value")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!("GPIO write: pin {} = {}", self.pin, value);
        
        // In production, this would write to GPIO
        let mut outputs = HashMap::new();
        outputs.insert("success".to_string(), Value::Bool(true));
        
        Ok(outputs)
    }

    fn node_type(&self) -> &str {
        "gpio_write"
    }
}

/// Delay node runner
pub struct DelayNodeRunner {
    node_id: String,
    delay_ms: u64,
}

impl DelayNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        let delay_ms = node
            .data
            .config
            .get("delay_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000);

        Ok(Self {
            node_id: node.id.clone(),
            delay_ms,
        })
    }
}

#[async_trait]
impl NodeRunner for DelayNodeRunner {
    async fn execute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        debug!("Delay: {}ms", self.delay_ms);
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        Ok(inputs)
    }

    fn node_type(&self) -> &str {
        "delay"
    }
}

/// HTTP request node runner
pub struct HttpRequestNodeRunner {
    node_id: String,
    url: String,
    method: String,
}

impl HttpRequestNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        let url = node
            .data
            .config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ConfigError("HTTP URL not specified".to_string()))?
            .to_string();

        let method = node
            .data
            .config
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_string();

        Ok(Self {
            node_id: node.id.clone(),
            url,
            method,
        })
    }
}

#[async_trait]
impl NodeRunner for HttpRequestNodeRunner {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        debug!("HTTP {}: {}", self.method, self.url);
        
        // In production, this would make the HTTP request
        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), Value::Number(200.into()));
        outputs.insert("body".to_string(), Value::String("{}".to_string()));
        
        Ok(outputs)
    }

    fn node_type(&self) -> &str {
        "http_request"
    }
}

/// Log node runner
pub struct LogNodeRunner {
    node_id: String,
    prefix: String,
}

impl LogNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        let prefix = node
            .data
            .config
            .get("prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("[LOG]")
            .to_string();

        Ok(Self {
            node_id: node.id.clone(),
            prefix,
        })
    }
}

#[async_trait]
impl NodeRunner for LogNodeRunner {
    async fn execute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        info!("{} {:?}", self.prefix, inputs);
        Ok(inputs)
    }

    fn node_type(&self) -> &str {
        "log"
    }
}

/// Passthrough node runner (for unknown node types)
pub struct PassthroughNodeRunner {
    node_id: String,
    node_type: String,
}

impl PassthroughNodeRunner {
    pub fn new(node: &Node) -> Result<Self, AgentError> {
        Ok(Self {
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
        })
    }
}

#[async_trait]
impl NodeRunner for PassthroughNodeRunner {
    async fn execute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, AgentError> {
        debug!("Passthrough node ({}): {:?}", self.node_type, inputs);
        Ok(inputs)
    }

    fn node_type(&self) -> &str {
        &self.node_type
    }
}
