//! Logging configuration

use std::path::PathBuf;

use tracing::Level;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::errors::AgentError;

/// Log level configuration
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn to_level(&self) -> Level {
        match self {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }

    pub fn to_filter_string(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl serde::Serialize for LogLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_filter_string())
    }
}

impl<'de> serde::Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Logging options
#[derive(Debug, Clone)]
pub struct LogOptions {
    /// Log level
    pub log_level: LogLevel,

    /// Write logs to stdout
    pub stdout: bool,

    /// Log directory for file output
    pub log_dir: PathBuf,

    /// Enable JSON format
    pub json_format: bool,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            stdout: true,
            log_dir: PathBuf::from("/var/log/ajime"),
            json_format: false,
        }
    }
}

/// Initialize logging
pub fn init_logging(options: LogOptions) -> Result<(), AgentError> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(options.log_level.to_filter_string()));

    let subscriber = tracing_subscriber::registry().with(filter);

    if options.stdout {
        if options.json_format {
            subscriber
                .with(fmt::layer().json())
                .try_init()
                .map_err(|e| AgentError::ConfigError(e.to_string()))?;
        } else {
            subscriber
                .with(fmt::layer())
                .try_init()
                .map_err(|e| AgentError::ConfigError(e.to_string()))?;
        }
    }

    Ok(())
}
