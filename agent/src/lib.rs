//! Ajime Agent Library
//!
//! Core modules for the Ajime edge agent.

pub mod app;
pub mod authn;
pub mod cache;
pub mod deploy;
pub mod errors;
pub mod filesys;
pub mod hardware;
pub mod http;
pub mod installer;
pub mod logs;
pub mod models;
pub mod mqtt;
pub mod server;
pub mod services;
pub mod storage;
pub mod sync;
pub mod telemetry;
pub mod utils;
pub mod workers;

/// Macro for creating trace information
#[macro_export]
macro_rules! trace {
    () => {
        format!("{}:{}", file!(), line!())
    };
}
