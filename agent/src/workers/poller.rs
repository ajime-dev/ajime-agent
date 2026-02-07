//! Polling worker for periodic sync

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tracing::{debug, error, info};

use crate::filesys::file::File;
use crate::sync::syncer::Syncer;

/// Poller worker options
#[derive(Debug, Clone)]
pub struct Options {
    /// Polling interval
    pub interval: Duration,

    /// Initial delay before first poll
    pub initial_delay: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            initial_delay: Duration::from_secs(5),
        }
    }
}

/// Run the poller worker
pub async fn run<S, F>(
    options: &Options,
    syncer: &Syncer,
    _device_file: &File,
    sleep_fn: S,
    mut shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send>>,
) where
    S: Fn(Duration) -> F,
    F: Future<Output = ()>,
{
    info!("Poller worker starting...");

    // Initial delay
    sleep_fn(options.initial_delay).await;

    loop {
        // Check for shutdown
        tokio::select! {
            _ = &mut shutdown_signal => {
                info!("Poller worker shutting down...");
                return;
            }
            _ = sleep_fn(options.interval) => {
                // Continue with poll
            }
        }

        debug!("Polling for updates...");

        // Trigger sync
        match syncer.trigger_sync().await {
            Ok(_) => {
                debug!("Sync completed successfully");
            }
            Err(e) => {
                error!("Sync failed: {}", e);
            }
        }
    }
}
