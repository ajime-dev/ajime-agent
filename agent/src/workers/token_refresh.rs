//! Token refresh worker

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tracing::{debug, error, info};

use crate::authn::token_mngr::TokenManagerExt;

/// Token refresh worker options
#[derive(Debug, Clone)]
pub struct Options {
    /// Check interval
    pub check_interval: Duration,

    /// Refresh when token expires within this duration
    pub refresh_threshold: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(3600), // 1 hour
            refresh_threshold: Duration::from_secs(86400), // 24 hours
        }
    }
}

/// Run the token refresh worker
pub async fn run<T, S, F>(
    options: &Options,
    token_mngr: &T,
    sleep_fn: S,
    mut shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send>>,
) where
    T: TokenManagerExt,
    S: Fn(Duration) -> F,
    F: Future<Output = ()>,
{
    info!("Token refresh worker starting...");

    loop {
        // Check for shutdown
        tokio::select! {
            _ = &mut shutdown_signal => {
                info!("Token refresh worker shutting down...");
                return;
            }
            _ = sleep_fn(options.check_interval) => {
                // Continue with check
            }
        }

        debug!("Checking token expiration...");

        // Get current token
        let token = match token_mngr.get_token().await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to get token: {}", e);
                continue;
            }
        };

        // Check if token needs refresh
        let threshold_secs = options.refresh_threshold.as_secs() as i64;
        if token.expires_within(threshold_secs) {
            info!(
                "Token expires within {} hours, refreshing...",
                threshold_secs / 3600
            );

            match token_mngr.refresh_token().await {
                Ok(new_token) => {
                    info!(
                        "Token refreshed successfully, new expiration: {}",
                        new_token.expires_at()
                    );
                }
                Err(e) => {
                    error!("Failed to refresh token: {}", e);
                    // Will retry on next interval
                }
            }
        } else {
            debug!(
                "Token still valid, expires in {} hours",
                token.time_until_expiry() / 3600
            );
        }
    }
}
