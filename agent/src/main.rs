//! Ajime Agent - Entry Point
//!
//! A lightweight edge agent for robotics workflow orchestration.
//! Runs on edge devices (Raspberry Pi, Jetson) and syncs with Ajime web_server.

use std::collections::HashMap;
use std::env;

use ajigent::app::options::{AppOptions, LifecycleOptions};
use ajigent::app::run::run;
use ajigent::installer::install::install;
use ajigent::logs::{init_logging, LogOptions};
use ajigent::mqtt::client::MqttAddress;
use ajigent::storage::device::assert_activated;
use ajigent::storage::layout::StorageLayout;
use ajigent::storage::settings::Settings;
use ajigent::utils::{version_info, run_diagnostic};
use ajigent::workers::mqtt;

use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut cli_args: HashMap<String, String> = HashMap::new();

    for arg in args.iter().skip(1) {
        if let Some((key, value)) = arg.split_once('=') {
            // Handle --key=value format
            let clean_key = key.trim_start_matches('-');
            cli_args.insert(clean_key.to_string(), value.to_string());
        } else if arg.starts_with("--") {
            // Handle standalone flags like --version
            let clean_key = arg.trim_start_matches('-');
            cli_args.insert(clean_key.to_string(), "true".to_string());
        }
    }

    // Print version and exit
    let version = version_info();
    if cli_args.contains_key("version") {
        println!("{}", serde_json::to_string_pretty(&version).unwrap());
        return;
    }

    // Run diagnostics
    if cli_args.contains_key("diagnostic") || cli_args.contains_key("diag") {
        run_diagnostic().await;
        return;
    }

    // Run the installer
    if cli_args.contains_key("install") {
        return install(&cli_args).await;
    }

    // Run the agent starting here

    // Check the agent has been activated
    let layout = StorageLayout::default();
    let device_file = layout.device_file();
    if let Err(e) = assert_activated(&device_file).await {
        error!("Device is not yet activated: {}", e);
        error!("Run: ajigent --install --token=<activation_token>");
        return;
    }

    // Retrieve the settings file
    let settings_file = layout.settings_file();
    let settings = match settings_file.read_json::<Settings>().await {
        Ok(settings) => settings,
        Err(e) => {
            error!("Unable to read settings file: {}", e);
            return;
        }
    };

    // Initialize logging
    let log_options = LogOptions {
        log_level: settings.log_level.clone(),
        ..Default::default()
    };
    if let Err(e) = init_logging(log_options) {
        println!("Failed to initialize logging: {e}");
    }

    // Run the server
    let options = AppOptions {
        lifecycle: LifecycleOptions {
            is_persistent: settings.is_persistent,
            ..Default::default()
        },
        backend_base_url: settings.backend.base_url.clone(),
        enable_socket_server: settings.enable_socket_server,
        enable_mqtt_worker: settings.enable_mqtt_worker,
        enable_poller: settings.enable_poller,
        mqtt_worker: mqtt::Options {
            broker_address: MqttAddress {
                host: settings.mqtt_broker.host.clone(),
                port: settings.mqtt_broker.port,
                use_tls: settings.mqtt_broker.tls,
                ca_cert_path: settings.mqtt_broker.ca_cert_path.clone(),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    info!("Running Ajime Agent with options: {:?}", options);
    let result = run(version.version, options, await_shutdown_signal()).await;
    if let Err(e) = result {
        error!("Failed to run the agent: {e}");
    }
}

async fn await_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {
                info!("SIGTERM received, shutting down...");
            }
            _ = sigint.recv() => {
                info!("SIGINT received, shutting down...");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received, shutting down...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        info!("Ctrl+C received, shutting down...");
    }
}
