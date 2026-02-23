//! Settings file management

use serde::{Deserialize, Serialize};

use crate::logs::LogLevel;

/// Agent settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Log level
    #[serde(default)]
    pub log_level: LogLevel,

    /// Backend configuration
    #[serde(default)]
    pub backend: BackendSettings,

    /// MQTT broker configuration
    #[serde(default)]
    pub mqtt_broker: MqttBrokerSettings,

    /// Whether the agent runs persistently
    #[serde(default = "default_true")]
    pub is_persistent: bool,

    /// Enable local HTTP server
    #[serde(default = "default_true")]
    pub enable_socket_server: bool,

    /// Enable MQTT worker
    #[serde(default = "default_true")]
    pub enable_mqtt_worker: bool,

    /// Enable polling worker
    #[serde(default = "default_true")]
    pub enable_poller: bool,

    /// Polling interval in seconds
    #[serde(default = "default_polling_interval")]
    pub polling_interval_secs: u64,

    /// Hardware configuration
    #[serde(default)]
    pub hardware: HardwareSettings,
}

fn default_true() -> bool {
    true
}

fn default_polling_interval() -> u64 {
    30
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            backend: BackendSettings::default(),
            mqtt_broker: MqttBrokerSettings::default(),
            is_persistent: true,
            enable_socket_server: true,
            enable_mqtt_worker: true,
            enable_poller: true,
            polling_interval_secs: 30,
            hardware: HardwareSettings::default(),
        }
    }
}

/// Backend API settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSettings {
    /// Base URL for the backend API
    #[serde(default = "default_backend_url")]
    pub base_url: String,
}

fn default_backend_url() -> String {
    "http://localhost:8000/api/v1".to_string()
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            base_url: default_backend_url(),
        }
    }
}

/// MQTT broker settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttBrokerSettings {
    /// Broker host
    #[serde(default = "default_mqtt_host")]
    pub host: String,

    /// Broker port
    #[serde(default = "default_mqtt_port")]
    pub port: u16,

    /// Use TLS
    #[serde(default = "default_true")]
    pub tls: bool,

    /// Optional path to a PEM-encoded CA certificate for broker TLS verification.
    /// When absent, the system certificate store is used.
    #[serde(default)]
    pub ca_cert_path: Option<String>,
}

fn default_mqtt_host() -> String {
    "".to_string()
}

fn default_mqtt_port() -> u16 {
    8883
}

impl Default for MqttBrokerSettings {
    fn default() -> Self {
        Self {
            host: default_mqtt_host(),
            port: default_mqtt_port(),
            tls: true,
            ca_cert_path: None,
        }
    }
}

/// Hardware settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSettings {
    /// Enable camera support
    #[serde(default)]
    pub enable_camera: bool,

    /// Enable GPIO support
    #[serde(default)]
    pub enable_gpio: bool,

    /// Camera device path
    #[serde(default = "default_camera_device")]
    pub camera_device: String,
}

fn default_camera_device() -> String {
    "/dev/video0".to_string()
}

impl Default for HardwareSettings {
    fn default() -> Self {
        Self {
            enable_camera: false,
            enable_gpio: false,
            camera_device: default_camera_device(),
        }
    }
}
