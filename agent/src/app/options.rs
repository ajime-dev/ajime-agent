//! Application configuration options

use std::time::Duration;

use crate::deploy::fsm::FsmSettings;
use crate::storage::layout::StorageLayout;
use crate::workers::{mqtt, poller, token_refresh};

/// Main application options
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// Lifecycle configuration
    pub lifecycle: LifecycleOptions,

    /// Backend API base URL
    pub backend_base_url: String,

    /// Storage configuration
    pub storage: StorageOptions,

    /// Enable local HTTP server
    pub enable_socket_server: bool,

    /// Enable MQTT worker
    pub enable_mqtt_worker: bool,

    /// Enable polling worker
    pub enable_poller: bool,

    /// Server configuration
    pub server: ServerOptions,

    /// MQTT worker options
    pub mqtt_worker: mqtt::Options,

    /// Poller worker options
    pub poller: poller::Options,

    /// Token refresh worker options
    pub token_refresh_worker: token_refresh::Options,

    /// FSM deployment settings
    pub fsm_settings: FsmSettings,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            lifecycle: LifecycleOptions::default(),
            backend_base_url: "https://api.ajime.io/agent/v1".to_string(),
            storage: StorageOptions::default(),
            enable_socket_server: true,
            enable_mqtt_worker: true,
            enable_poller: true,
            server: ServerOptions::default(),
            mqtt_worker: mqtt::Options::default(),
            poller: poller::Options::default(),
            token_refresh_worker: token_refresh::Options::default(),
            fsm_settings: FsmSettings::default(),
        }
    }
}

/// Lifecycle options for the agent
#[derive(Debug, Clone)]
pub struct LifecycleOptions {
    /// Whether the agent runs persistently (as a service)
    pub is_persistent: bool,

    /// Idle timeout before shutdown (non-persistent mode)
    pub idle_timeout: Duration,

    /// Interval to check for idle timeout
    pub idle_timeout_poll_interval: Duration,

    /// Maximum runtime before shutdown (non-persistent mode)
    pub max_runtime: Duration,

    /// Maximum delay for graceful shutdown
    pub max_shutdown_delay: Duration,
}

impl Default for LifecycleOptions {
    fn default() -> Self {
        Self {
            is_persistent: true,
            idle_timeout: Duration::from_secs(300),           // 5 minutes
            idle_timeout_poll_interval: Duration::from_secs(10),
            max_runtime: Duration::from_secs(3600),           // 1 hour
            max_shutdown_delay: Duration::from_secs(30),
        }
    }
}

/// Storage configuration options
#[derive(Debug, Clone)]
pub struct StorageOptions {
    /// Storage layout paths
    pub layout: StorageLayout,

    /// Cache capacities
    pub cache_capacities: CacheCapacities,
}

impl Default for StorageOptions {
    fn default() -> Self {
        Self {
            layout: StorageLayout::default(),
            cache_capacities: CacheCapacities::default(),
        }
    }
}

/// Cache capacity configuration
#[derive(Debug, Clone, Copy)]
pub struct CacheCapacities {
    /// Maximum workflow cache entries
    pub workflows: u64,

    /// Maximum config cache entries
    pub configs: u64,
}

impl Default for CacheCapacities {
    fn default() -> Self {
        Self {
            workflows: 100,
            configs: 100,
        }
    }
}

/// Local HTTP server options
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Host to bind to
    pub host: String,

    /// Port to listen on
    pub port: u16,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}
