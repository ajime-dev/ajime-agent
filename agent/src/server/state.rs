//! Server state

use std::sync::Arc;

use crate::app::state::{ActivityTracker, Caches};
use crate::authn::token_mngr::TokenManager;
use crate::filesys::file::File;
use crate::http::client::HttpClient;
use crate::sync::syncer::Syncer;

/// Server state shared across handlers
pub struct ServerState {
    pub device_file: Arc<File>,
    pub http_client: Arc<HttpClient>,
    pub syncer: Arc<Syncer>,
    pub caches: Arc<Caches>,
    pub token_mngr: Arc<TokenManager>,
    pub activity_tracker: Arc<ActivityTracker>,
}

impl ServerState {
    pub fn new(
        device_file: Arc<File>,
        http_client: Arc<HttpClient>,
        syncer: Arc<Syncer>,
        caches: Arc<Caches>,
        token_mngr: Arc<TokenManager>,
        activity_tracker: Arc<ActivityTracker>,
    ) -> Self {
        Self {
            device_file,
            http_client,
            syncer,
            caches,
            token_mngr,
            activity_tracker,
        }
    }
}
