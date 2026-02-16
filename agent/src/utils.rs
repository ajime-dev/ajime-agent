//! Utility functions

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Version information for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_hash: String,
    pub build_time: String,
}

/// Get version information
pub fn version_info() -> VersionInfo {
    VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_hash: option_env!("GIT_HASH").unwrap_or("unknown").to_string(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
    }
}

/// Cooldown options for exponential backoff
#[derive(Debug, Clone)]
pub struct CooldownOptions {
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for CooldownOptions {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300), // 5 minutes
            multiplier: 2.0,
        }
    }
}

/// Calculate exponential backoff delay
pub fn calc_exp_backoff(options: &CooldownOptions, attempt: u32) -> Duration {
    let delay_secs = options.base_delay.as_secs_f64() * options.multiplier.powi(attempt as i32);
    let capped_delay = delay_secs.min(options.max_delay.as_secs_f64());
    Duration::from_secs_f64(capped_delay)
}

/// Generate a random UUID v4
pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Calculate SHA256 hash of data
pub fn sha256_hash(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Hex encoding utilities
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(data: impl AsRef<[u8]>) -> String {
        let data = data.as_ref();
        let mut result = String::with_capacity(data.len() * 2);
        for byte in data {
            result.push(HEX_CHARS[(byte >> 4) as usize] as char);
            result.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exp_backoff() {
        let options = CooldownOptions::default();
        
        assert_eq!(calc_exp_backoff(&options, 0), Duration::from_secs(1));
        assert_eq!(calc_exp_backoff(&options, 1), Duration::from_secs(2));
        assert_eq!(calc_exp_backoff(&options, 2), Duration::from_secs(4));
        assert_eq!(calc_exp_backoff(&options, 10), Duration::from_secs(300)); // Capped at max
    }

    #[test]
    fn test_sha256_hash() {
        let hash = sha256_hash(b"hello world");
        assert_eq!(hash.len(), 64);
    }
}

/// Run diagnostics on the agent
pub async fn run_diagnostic() {
    use crate::storage::layout::StorageLayout;
    use crate::storage::device::Device;
    use crate::storage::settings::Settings;
    use colored::*;

    println!("{}", "=== Ajime Agent Diagnostic ===".bold().cyan());
    
    let layout = StorageLayout::default();
    let device_file = layout.device_file();
    let settings_file = layout.settings_file();

    // 1. Check device.json
    print!("Checking device credentials (device.json)... ");
    let device = match device_file.read_json::<Device>().await {
        Ok(d) => {
            println!("{}", "OK".green());
            Some(d)
        },
        Err(e) => {
            println!("{} ({})", "FAILED".red(), e);
            None
        }
    };

    // 2. Check settings.json
    print!("Checking agent settings (settings.json)... ");
    let settings = match settings_file.read_json::<Settings>().await {
        Ok(s) => {
            println!("{}", "OK".green());
            Some(s)
        },
        Err(e) => {
            println!("{} ({})", "FAILED".red(), e);
            None
        }
    };

    if let (Some(device), Some(settings)) = (device, settings) {
        println!("\n{}", "--- Connectivity ---".bold());
        
        let backend_url = &settings.backend.base_url;
        println!("Backend URL: {}", backend_url);
        
        // 3. Test basic reachability
        print!("Testing backend reachability... ");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Ajime-Agent-Diagnostic")
            .build()
            .unwrap();

        match client.get(backend_url.trim_end_matches("/api/v1")).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("{}", "OK".green());
                } else {
                    println!("{} (HTTP {})", "WARNING".yellow(), resp.status());
                }
            },
            Err(e) => {
                println!("{} ({})", "FAILED".red(), e);
            }
        }

        // 4. Test authentication
        print!("Testing credential authentication... ");
        let test_url = format!("{}/agent/devices/{}/test-credentials", backend_url, device.id);
        
        let auth_resp = client.post(&test_url)
            .header("X-Device-ID", &device.id)
            .header("Authorization", format!("Bearer {}", device.token))
            .send()
            .await;

        match auth_resp {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    if body["status"] == "success" {
                        println!("{}", "AUTHENTICATED".green().bold());
                    } else {
                        let msg = body["message"].as_str().unwrap_or("Unknown error");
                        println!("{} (Backend: {})", "REFUSED".red().bold(), msg);
                    }
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    println!("{} (HTTP {} - {})", "ERROR".red().bold(), status, body);
                }
            },
            Err(e) => {
                println!("{} ({})", "FAILED".red(), e);
            }
        }
    } else {
        println!("\n{}", "Cannot proceed with connectivity tests due to missing configuration.".yellow());
    }

    println!("\n{}", "==============================".bold().cyan());
}
