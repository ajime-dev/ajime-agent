//! Application state management

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::task::JoinHandle;
use tracing::info;

use crate::app::options::CacheCapacities;
use crate::authn::token_mngr::TokenManager;
use crate::cache::workflow::WorkflowCache;
use crate::deploy::fsm::FsmSettings;
use crate::errors::AgentError;
use crate::filesys::file::File;
use crate::http::client::HttpClient;
use crate::storage::layout::StorageLayout;
use crate::sync::syncer::Syncer;

/// Activity tracker for idle timeout detection
pub struct ActivityTracker {
    last_touched: AtomicU64,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            last_touched: AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        }
    }

    pub fn touch(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_touched.store(now, Ordering::SeqCst);
    }

    pub fn last_touched(&self) -> u64 {
        self.last_touched.load(Ordering::SeqCst)
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Application caches
pub struct Caches {
    pub workflows: Arc<WorkflowCache>,
}

impl Caches {
    pub fn new(capacities: CacheCapacities) -> Self {
        Self {
            workflows: Arc::new(WorkflowCache::new(capacities.workflows)),
        }
    }
}

/// Main application state
pub struct AppState {
    /// Device file reference
    pub device_file: Arc<File>,

    /// HTTP client for backend communication
    pub http_client: Arc<HttpClient>,

    /// Token manager for authentication
    pub token_mngr: Arc<TokenManager>,

    /// Workflow syncer
    pub syncer: Arc<Syncer>,

    /// Application caches
    pub caches: Arc<Caches>,

    /// Activity tracker
    pub activity_tracker: Arc<ActivityTracker>,
}

impl AppState {
    /// Initialize application state
    pub async fn init(
        agent_version: String,
        layout: &StorageLayout,
        cache_capacities: CacheCapacities,
        http_client: Arc<HttpClient>,
        fsm_settings: FsmSettings,
    ) -> Result<(Self, JoinHandle<()>), AgentError> {
        info!("Initializing application state...");

        // Load device file
        let device_file = Arc::new(layout.device_file());

        // Create caches
        let caches = Arc::new(Caches::new(cache_capacities));

        // Create token manager
        let token_mngr = Arc::new(
            TokenManager::new(device_file.clone(), http_client.clone()).await?,
        );

        // Create activity tracker
        let activity_tracker = Arc::new(ActivityTracker::new());

        // Create syncer
        let syncer = Arc::new(Syncer::new(
            device_file.clone(),
            http_client.clone(),
            token_mngr.clone(),
            caches.workflows.clone(),
            layout.deployment_dir(),
            fsm_settings,
            agent_version,
        ));

        // Create background task handle (placeholder for now)
        let handle = tokio::spawn(async {});

        let state = Self {
            device_file,
            http_client,
            token_mngr,
            syncer,
            caches,
            activity_tracker,
        };

        Ok((state, handle))
    }

    /// Shutdown application state
    pub async fn shutdown(&self) -> Result<(), AgentError> {
        info!("Shutting down application state...");
        // Cleanup resources
        Ok(())
    }
}
