//! Device installation and activation

use std::collections::HashMap;

use tracing::{error, info, warn};

use crate::http::client::HttpClient;
use crate::logs::{init_logging, LogOptions};
use crate::storage::device::Device;
use crate::storage::layout::StorageLayout;
use crate::storage::settings::Settings;
use crate::utils::version_info;

/// Run the installation process
pub async fn install(cli_args: &HashMap<String, String>) {
    match install_impl(cli_args).await {
        Ok(_) => {
            info!("Installation successful");
            println!("\n[SUCCESS] Ajime Agent installed and activated successfully!");
            println!("Start the agent with: systemctl start ajime-agent");
        }
        Err(e) => {
            error!("Installation failed: {:?}", e);
            eprintln!("\n[ERROR] Installation failed: {}", e);
            std::process::exit(1);
        }
    }
}

async fn install_impl(cli_args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize temporary logging
    let log_options = LogOptions {
        stdout: true,
        ..Default::default()
    };
    let _ = init_logging(log_options);

    println!("Ajime Agent Installer");
    println!("=====================");
    println!();

    // Get activation token
    let token_env_var = "AJIME_ACTIVATION_TOKEN";
    let activation_token = cli_args
        .get("token")
        .cloned()
        .or_else(|| std::env::var(token_env_var).ok())
        .ok_or_else(|| {
            format!(
                "Missing activation token. Provide via --token=<token> or {} environment variable",
                token_env_var
            )
        })?;

    // Get device name
    let device_name = cli_args
        .get("name")
        .cloned()
        .or_else(|| get_hostname())
        .unwrap_or_else(|| "ajime-device".to_string());

    // Get device type
    let device_type = cli_args
        .get("type")
        .cloned()
        .or_else(|| detect_device_type());

    println!("Device name: {}", device_name);
    if let Some(ref dt) = device_type {
        println!("Device type: {}", dt);
    }
    println!();

    // Setup storage layout
    let layout = StorageLayout::default();
    println!("Setting up storage at: {:?}", layout.base_dir);
    layout.setup().await?;

    // Get backend URL from args or use default
    let backend_url = cli_args
        .get("backend")
        .cloned()
        .unwrap_or_else(|| "https://api.ajime.io/agent/v1".to_string());

    println!("Backend URL: {}", backend_url);
    println!();

    // Create HTTP client and activate device
    println!("Activating device...");
    let http_client = HttpClient::new(&backend_url).await?;
    let activation_response = http_client
        .activate_device(&activation_token, &device_name, device_type.as_deref())
        .await?;

    println!("Device activated!");
    println!("  Device ID: {}", activation_response.device_id);
    println!("  Owner ID: {}", activation_response.owner_id);
    println!();

    // Create and save device file
    let device = Device::new(
        activation_response.device_id.clone(),
        activation_response.device_name.clone(),
        activation_response.owner_id.clone(),
        activation_response.token.clone(),
    );

    let device_file = layout.device_file();
    device_file.write_json(&device).await?;
    println!("Device credentials saved to: {:?}", device_file.path());

    // Create and save settings file
    let mut settings = Settings::default();
    settings.backend.base_url = backend_url;

    let settings_file = layout.settings_file();
    settings_file.write_json(&settings).await?;
    println!("Settings saved to: {:?}", settings_file.path());

    // Print version info
    let version = version_info();
    println!();
    println!("Agent version: {}", version.version);
    println!("Git hash: {}", version.git_hash);
    println!("Build time: {}", version.build_time);

    Ok(())
}

/// Get the system hostname
fn get_hostname() -> Option<String> {
    sysinfo::System::host_name()
}

/// Detect the device type based on system information
fn detect_device_type() -> Option<String> {
    // Try to detect Raspberry Pi
    if std::path::Path::new("/proc/device-tree/model").exists() {
        if let Ok(model) = std::fs::read_to_string("/proc/device-tree/model") {
            let model = model.trim_matches('\0').to_lowercase();
            if model.contains("raspberry pi") {
                return Some("raspberry_pi".to_string());
            }
            if model.contains("jetson") {
                return Some("jetson".to_string());
            }
        }
    }

    // Check for Jetson via tegra
    if std::path::Path::new("/etc/nv_tegra_release").exists() {
        return Some("jetson".to_string());
    }

    // Default to generic linux
    #[cfg(target_os = "linux")]
    return Some("linux".to_string());

    #[cfg(not(target_os = "linux"))]
    None
}
