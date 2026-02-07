//! Telemetry and metrics collection

use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks};

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,

    /// Memory usage in bytes
    pub memory_used: u64,

    /// Total memory in bytes
    pub memory_total: u64,

    /// Memory usage percentage
    pub memory_percent: f32,

    /// Disk usage in bytes
    pub disk_used: u64,

    /// Total disk space in bytes
    pub disk_total: u64,

    /// Disk usage percentage
    pub disk_percent: f32,

    /// System uptime in seconds
    pub uptime_secs: u64,

    /// Number of CPU cores
    pub cpu_count: usize,

    /// Hostname
    pub hostname: String,
}

/// Collect system metrics
pub fn collect_metrics() -> SystemMetrics {
    let mut sys = System::new_all();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();

    // Calculate total disk usage
    let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(used, total), disk| {
        (
            used + (disk.total_space() - disk.available_space()),
            total + disk.total_space(),
        )
    });

    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();

    SystemMetrics {
        cpu_usage: sys.global_cpu_usage(),
        memory_used,
        memory_total,
        memory_percent: if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        },
        disk_used,
        disk_total,
        disk_percent: if disk_total > 0 {
            (disk_used as f32 / disk_total as f32) * 100.0
        } else {
            0.0
        },
        uptime_secs: System::uptime(),
        cpu_count: sys.cpus().len(),
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
    }
}

/// Agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// System metrics
    pub system: SystemMetrics,

    /// Agent version
    pub agent_version: String,

    /// Number of deployed workflows
    pub deployed_workflows: usize,

    /// Number of active workflow executions
    pub active_executions: usize,

    /// Last sync timestamp (Unix epoch seconds)
    pub last_sync_at: Option<u64>,

    /// Last successful sync timestamp
    pub last_successful_sync_at: Option<u64>,

    /// Sync error count
    pub sync_error_count: u32,
}
