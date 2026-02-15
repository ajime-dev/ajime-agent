//! Main application run loop

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::app::options::{AppOptions, LifecycleOptions};
use crate::app::state::{ActivityTracker, AppState};
use crate::authn::token_mngr::{TokenManager, TokenManagerExt};
use crate::errors::AgentError;
use crate::http::client::HttpClient;
use crate::server::serve::serve;
use crate::server::state::ServerState;
use crate::workers::{mqtt, poller, token_refresh, deployer, relay};

/// Run the Ajime agent
pub async fn run(
    agent_version: String,
    options: AppOptions,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) -> Result<(), AgentError> {
    info!("Initializing Ajime Agent...");

    // Create shutdown channel
    let (shutdown_tx, _shutdown_rx): (broadcast::Sender<()>, _) = broadcast::channel(1);
    let mut shutdown_manager = ShutdownManager::new(shutdown_tx.clone(), options.lifecycle.clone());

    // Initialize the app state
    let app_state = match init(
        agent_version,
        &options,
        shutdown_tx.clone(),
        &mut shutdown_manager,
    )
    .await
    {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to start agent: {}", e);
            shutdown_manager.shutdown().await?;
            return Err(e);
        }
    };

    // Handle lifecycle based on persistence mode
    if !options.lifecycle.is_persistent {
        tokio::select! {
            _ = shutdown_signal => {
                info!("Shutdown signal received, shutting down...");
            }
            _ = await_idle_timeout(
                app_state.activity_tracker.clone(),
                options.lifecycle.idle_timeout,
                options.lifecycle.idle_timeout_poll_interval,
            ) => {
                info!("Idle timeout ({:?}) reached, shutting down...", options.lifecycle.idle_timeout);
            }
            _ = await_max_runtime(options.lifecycle.max_runtime) => {
                info!("Max runtime ({:?}) reached, shutting down...", options.lifecycle.max_runtime);
            }
        }
    } else {
        tokio::select! {
            _ = shutdown_signal => {
                info!("Shutdown signal received, shutting down...");
            }
        }
    }

    // Shutdown
    drop(shutdown_tx);
    shutdown_manager.shutdown().await
}

async fn await_idle_timeout(
    activity_tracker: Arc<ActivityTracker>,
    idle_timeout: Duration,
    poll_interval: Duration,
) -> Result<(), AgentError> {
    loop {
        tokio::time::sleep(poll_interval).await;
        let last_activity =
            SystemTime::UNIX_EPOCH + Duration::from_secs(activity_tracker.last_touched());
        match SystemTime::now().duration_since(last_activity) {
            Ok(duration) if duration > idle_timeout => {
                info!("Agent idle timeout reached");
                return Ok(());
            }
            Err(_) => {
                error!("Idle timeout checker error, ignoring...");
            }
            _ => {}
        }
    }
}

async fn await_max_runtime(max_runtime: Duration) -> Result<(), AgentError> {
    tokio::time::sleep(max_runtime).await;
    Ok(())
}

// =============================== INITIALIZATION ================================== //

async fn init(
    agent_version: String,
    options: &AppOptions,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_manager: &mut ShutdownManager,
) -> Result<Arc<AppState>, AgentError> {
    let app_state = init_app_state(agent_version, options, shutdown_manager).await?;

    init_token_refresh_worker(
        app_state.token_mngr.clone(),
        options.token_refresh_worker.clone(),
        shutdown_manager,
        shutdown_tx.subscribe(),
    )
    .await?;

    if options.enable_socket_server {
        init_socket_server(
            options,
            app_state.clone(),
            shutdown_manager,
            shutdown_tx.subscribe(),
        )
        .await?;
    }

    if options.enable_poller {
        init_poller_worker(
            options.poller.clone(),
            app_state.clone(),
            shutdown_manager,
            shutdown_tx.subscribe(),
        )
        .await?;
    }

    if options.enable_mqtt_worker {
        init_mqtt_worker(
            options.mqtt_worker.clone(),
            app_state.clone(),
            shutdown_manager,
            shutdown_tx.subscribe(),
        )
        .await?;
    }

    if options.enable_deployer {
        init_deployer_worker(
            options.deployer.clone(),
            app_state.clone(),
            shutdown_manager,
            shutdown_tx.subscribe(),
        )
        .await?;
    }

    if options.enable_relay_worker {
        init_relay_worker(
            options.relay_worker.clone(),
            app_state.clone(),
            options.backend_base_url.clone(),
            shutdown_manager,
            shutdown_tx.subscribe(),
        )
        .await?;
    }

    Ok(app_state)
}

async fn init_app_state(
    agent_version: String,
    options: &AppOptions,
    shutdown_manager: &mut ShutdownManager,
) -> Result<Arc<AppState>, AgentError> {
    let http_client = Arc::new(HttpClient::new(&options.backend_base_url).await?);

    let (app_state, app_state_handle) = AppState::init(
        agent_version,
        &options.storage.layout,
        options.storage.cache_capacities,
        http_client,
        options.fsm_settings.clone(),
    )
    .await?;

    let app_state = Arc::new(app_state);
    shutdown_manager.with_app_state(app_state.clone(), Box::pin(app_state_handle))?;

    Ok(app_state)
}

async fn init_token_refresh_worker(
    token_mngr: Arc<TokenManager>,
    options: token_refresh::Options,
    shutdown_manager: &mut ShutdownManager,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), AgentError> {
    info!("Initializing token refresh worker...");

    // Refresh token if expired
    if let Err(e) = refresh_if_expired(&token_mngr).await {
        error!("Failed to refresh expired token: {}", e);
    }

    let token_refresh_handle = tokio::spawn(async move {
        token_refresh::run(
            &options,
            token_mngr.as_ref(),
            |wait| tokio::time::sleep(wait),
            Box::pin(async move {
                let _ = shutdown_rx.recv().await;
            }),
        )
        .await;
    });

    shutdown_manager.with_token_refresh_worker_handle(token_refresh_handle)?;
    Ok(())
}

async fn refresh_if_expired(token_mngr: &TokenManager) -> Result<(), AgentError> {
    let token = token_mngr.get_token().await?;
    if token.is_expired() {
        token_mngr.refresh_token().await?;
    }
    Ok(())
}

async fn init_poller_worker(
    options: poller::Options,
    app_state: Arc<AppState>,
    shutdown_manager: &mut ShutdownManager,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), AgentError> {
    info!("Initializing poller worker...");

    let syncer = app_state.syncer.clone();
    let device_file = app_state.device_file.clone();

    let poller_handle = tokio::spawn(async move {
        poller::run(
            &options,
            syncer.as_ref(),
            device_file.as_ref(),
            tokio::time::sleep,
            Box::pin(async move {
                let _ = shutdown_rx.recv().await;
            }),
        )
        .await;
    });

    shutdown_manager.with_poller_worker_handle(poller_handle)?;
    Ok(())
}

async fn init_mqtt_worker(
    options: mqtt::Options,
    app_state: Arc<AppState>,
    shutdown_manager: &mut ShutdownManager,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), AgentError> {
    info!("Initializing MQTT worker...");

    let token_mngr = app_state.token_mngr.clone();
    let syncer = app_state.syncer.clone();
    let device_file = app_state.device_file.clone();

    let mqtt_handle = tokio::spawn(async move {
        mqtt::run(
            &options,
            token_mngr.as_ref(),
            syncer.as_ref(),
            device_file.as_ref(),
            tokio::time::sleep,
            Box::pin(async move {
                let _ = shutdown_rx.recv().await;
            }),
        )
        .await;
    });

    shutdown_manager.with_mqtt_worker_handle(mqtt_handle)?;
    Ok(())
}

async fn init_deployer_worker(
    options: deployer::Options,
    app_state: Arc<AppState>,
    shutdown_manager: &mut ShutdownManager,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), AgentError> {
    info!("Initializing deployer worker...");

    let http_client = app_state.http_client.clone();
    let token_mngr = app_state.token_mngr.clone();
    let device_file = app_state.device_file.clone();

    let deployer_handle = tokio::spawn(async move {
        deployer::run(
            &options,
            http_client,
            token_mngr,
            device_file,
            tokio::time::sleep,
            Box::pin(async move {
                let _ = shutdown_rx.recv().await;
            }),
        )
        .await;
    });

    shutdown_manager.with_deployer_worker_handle(deployer_handle)?;
    Ok(())
}

async fn init_relay_worker(
    options: relay::Options,
    app_state: Arc<AppState>,
    backend_url: String,
    shutdown_manager: &mut ShutdownManager,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), AgentError> {
    info!("Initializing relay worker...");

    let token_mngr = app_state.token_mngr.clone();

    let relay_handle = tokio::spawn(async move {
        relay::run(
            &options,
            token_mngr,
            backend_url,
            Box::pin(async move {
                let _ = shutdown_rx.recv().await;
            }),
        )
        .await;
    });

    shutdown_manager.with_relay_worker_handle(relay_handle)?;
    Ok(())
}

async fn init_socket_server(
    options: &AppOptions,
    app_state: Arc<AppState>,
    shutdown_manager: &mut ShutdownManager,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), AgentError> {
    info!("Initializing local HTTP server...");

    let server_state = ServerState::new(
        app_state.device_file.clone(),
        app_state.http_client.clone(),
        app_state.syncer.clone(),
        app_state.caches.clone(),
        app_state.token_mngr.clone(),
        app_state.activity_tracker.clone(),
    );

    let server_handle = serve(&options.server, Arc::new(server_state), async move {
        let _ = shutdown_rx.recv().await;
    })
    .await?;

    shutdown_manager.with_socket_server_handle(server_handle)?;
    Ok(())
}

// ================================= SHUTDOWN ===================================== //

struct AppStateShutdownParams {
    state: Arc<AppState>,
    state_handle: Pin<Box<dyn Future<Output = ()> + Send>>,
}

struct ShutdownManager {
    shutdown_tx: broadcast::Sender<()>,
    lifecycle_options: LifecycleOptions,
    app_state: Option<AppStateShutdownParams>,
    socket_server_handle: Option<JoinHandle<Result<(), AgentError>>>,
    poller_worker_handle: Option<JoinHandle<()>>,
    mqtt_worker_handle: Option<JoinHandle<()>>,
    deployer_worker_handle: Option<JoinHandle<()>>,
    relay_worker_handle: Option<JoinHandle<()>>,
    token_refresh_worker_handle: Option<JoinHandle<()>>,
}

impl ShutdownManager {
    pub fn new(shutdown_tx: broadcast::Sender<()>, lifecycle_options: LifecycleOptions) -> Self {
        Self {
            shutdown_tx,
            lifecycle_options,
            app_state: None,
            socket_server_handle: None,
            poller_worker_handle: None,
            mqtt_worker_handle: None,
            deployer_worker_handle: None,
            relay_worker_handle: None,
            token_refresh_worker_handle: None,
        }
    }

    pub fn with_app_state(
        &mut self,
        state: Arc<AppState>,
        state_handle: Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> Result<(), AgentError> {
        if self.app_state.is_some() {
            return Err(AgentError::ShutdownError("app_state already set".to_string()));
        }
        self.app_state = Some(AppStateShutdownParams { state, state_handle });
        Ok(())
    }

    pub fn with_token_refresh_worker_handle(
        &mut self,
        handle: JoinHandle<()>,
    ) -> Result<(), AgentError> {
        if self.token_refresh_worker_handle.is_some() {
            return Err(AgentError::ShutdownError("token_refresh_handle already set".to_string()));
        }
        self.token_refresh_worker_handle = Some(handle);
        Ok(())
    }

    pub fn with_poller_worker_handle(&mut self, handle: JoinHandle<()>) -> Result<(), AgentError> {
        if self.poller_worker_handle.is_some() {
            return Err(AgentError::ShutdownError("poller_handle already set".to_string()));
        }
        self.poller_worker_handle = Some(handle);
        Ok(())
    }

    pub fn with_mqtt_worker_handle(&mut self, handle: JoinHandle<()>) -> Result<(), AgentError> {
        if self.mqtt_worker_handle.is_some() {
            return Err(AgentError::ShutdownError("mqtt_handle already set".to_string()));
        }
        self.mqtt_worker_handle = Some(handle);
        Ok(())
    }

    pub fn with_deployer_worker_handle(&mut self, handle: JoinHandle<()>) -> Result<(), AgentError> {
        if self.deployer_worker_handle.is_some() {
            return Err(AgentError::ShutdownError("deployer_handle already set".to_string()));
        }
        self.deployer_worker_handle = Some(handle);
        Ok(())
    }

    pub fn with_relay_worker_handle(&mut self, handle: JoinHandle<()>) -> Result<(), AgentError> {
        if self.relay_worker_handle.is_some() {
            return Err(AgentError::ShutdownError("relay_handle already set".to_string()));
        }
        self.relay_worker_handle = Some(handle);
        Ok(())
    }

    pub fn with_socket_server_handle(
        &mut self,
        handle: JoinHandle<Result<(), AgentError>>,
    ) -> Result<(), AgentError> {
        if self.socket_server_handle.is_some() {
            return Err(AgentError::ShutdownError("server_handle already set".to_string()));
        }
        self.socket_server_handle = Some(handle);
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), AgentError> {
        let _ = self.shutdown_tx.send(());

        match tokio::time::timeout(
            self.lifecycle_options.max_shutdown_delay,
            self.shutdown_impl(),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                error!(
                    "Shutdown timed out after {:?}, forcing shutdown...",
                    self.lifecycle_options.max_shutdown_delay
                );
                std::process::exit(1);
            }
        }
    }

    async fn shutdown_impl(&mut self) -> Result<(), AgentError> {
        info!("Shutting down Ajime Agent...");

        // 1. Token refresh worker
        if let Some(handle) = self.token_refresh_worker_handle.take() {
            handle.await.map_err(|e| AgentError::ShutdownError(e.to_string()))?;
        }

        // 2. Poller worker
        if let Some(handle) = self.poller_worker_handle.take() {
            handle.await.map_err(|e| AgentError::ShutdownError(e.to_string()))?;
        }

        // 3. MQTT worker
        if let Some(handle) = self.mqtt_worker_handle.take() {
            handle.await.map_err(|e| AgentError::ShutdownError(e.to_string()))?;
        }

        // 4. Deployer worker
        if let Some(handle) = self.deployer_worker_handle.take() {
            handle.await.map_err(|e| AgentError::ShutdownError(e.to_string()))?;
        }

        // 4.5. Relay worker
        if let Some(handle) = self.relay_worker_handle.take() {
            handle.await.map_err(|e| AgentError::ShutdownError(e.to_string()))?;
        }

        // 5. Socket server
        if let Some(handle) = self.socket_server_handle.take() {
            handle.await.map_err(|e| AgentError::ShutdownError(e.to_string()))??;
        }

        // 5. App state
        if let Some(app_state) = self.app_state.take() {
            app_state.state.shutdown().await?;
            app_state.state_handle.await;
        }

        info!("Shutdown complete");
        Ok(())
    }
}
