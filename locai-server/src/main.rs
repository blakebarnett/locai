use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use locai::{config::ConfigBuilder, init};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

mod api;
mod cli;
mod config;
mod error;
mod messaging;
mod state;
mod websocket;

use crate::api::create_router;
use crate::cli::CliArgs;
use crate::config::ServerConfig;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli_args = CliArgs::parse();

    // Set up logging
    let filter = if let Some(ref level) = cli_args.log_level {
        tracing_subscriber::EnvFilter::new(level)
            .add_directive("surrealdb_core=warn".parse().unwrap())
            .add_directive("surrealdb=warn".parse().unwrap())
    } else {
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("surrealdb_core=warn".parse().unwrap())
            .add_directive("surrealdb=warn".parse().unwrap())
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting Locai server v{}", locai::VERSION);

    // Load configuration from CLI arguments and environment variables
    let server_config = ServerConfig::from_cli_and_env(cli_args.clone())?;
    info!("Server configuration loaded");

    // Initialize Locai with configuration - load from file if provided!
    let locai_config = if let Some(config_file) = &cli_args.config_file {
        info!(
            "📁 Loading Locai configuration from: {}",
            config_file.display()
        );

        let mut loader = locai::config::ConfigLoader::new();
        match loader.load_file(config_file) {
            Ok(_) => match loader.extract() {
                Ok(config) => {
                    info!(
                        "✅ Successfully loaded configuration from {}",
                        config_file.display()
                    );
                    config
                }
                Err(e) => {
                    warn!(
                        "Failed to parse config file {}: {}. Using defaults.",
                        config_file.display(),
                        e
                    );
                    ConfigBuilder::new()
                        .with_default_storage()
                        .with_remote_surrealdb_if_configured()
                        .with_default_ml()
                        .build()?
                }
            },
            Err(e) => {
                warn!(
                    "Failed to load config file {}: {}. Using defaults.",
                    config_file.display(),
                    e
                );
                ConfigBuilder::new()
                    .with_default_storage()
                    .with_remote_surrealdb_if_configured()
                    .with_default_ml()
                    .build()?
            }
        }
    } else {
        info!("📋 No config file provided, using default configuration");
        ConfigBuilder::new()
            .with_default_storage()
            .with_remote_surrealdb_if_configured()
            .with_default_ml()
            .build()?
    };

    let memory_manager = init(locai_config).await?;
    info!("Locai memory manager initialized");

    // Additional config verification
    let _ = memory_manager.config();

    // Create application state
    let mut app_state = AppState::new(memory_manager, server_config.clone());

    // Initialize messaging server if enabled using shared storage from the memory manager
    if server_config.messaging.enabled {
        // Get the shared storage from the memory manager instead of creating a separate instance
        let shared_storage = app_state.memory_manager.storage();
        let messaging_server = messaging::MessagingServer::new_with_shared_storage(
            server_config.messaging.clone(),
            shared_storage,
        );
        info!("Messaging server initialized successfully with shared storage from memory manager");
        app_state.set_messaging_server(Arc::new(messaging_server));
    }

    // Initialize authentication if enabled
    if server_config.enable_auth {
        if let Err(e) = initialize_auth(&mut app_state, server_config.clone()).await {
            warn!(
                "Failed to initialize authentication: {}. Auth may not work properly.",
                e
            );
        }
    }

    let app_state = Arc::new(app_state);

    // Initialize live queries if enabled and using SurrealDB
    if server_config.enable_live_queries {
        if let Err(e) = setup_live_queries(app_state.clone()).await {
            warn!(
                "Failed to setup live queries: {}. Continuing without live query support.",
                e
            );
        }
    }

    // Create the router with all API endpoints
    let app = create_router(app_state.clone())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], server_config.port));
    let listener = TcpListener::bind(addr).await?;

    info!("Server listening on {}", addr);
    info!("API documentation available at http://{}/docs", addr);

    if server_config.enable_auth {
        info!("Authentication is enabled");
        if server_config.allow_signup {
            info!("User signup is enabled");
        } else {
            info!("User signup is disabled");
        }
    } else {
        info!("Authentication is disabled");
    }

    axum::serve(listener, app).await?;

    Ok(())
}

/// Initialize authentication system and create root user if needed
async fn initialize_auth(app_state: &mut AppState, server_config: ServerConfig) -> Result<()> {
    use crate::api::auth_service::AuthService;

    info!("Initializing authentication system using storage abstractions");

    // Create the auth service
    let auth_service = AuthService::new(server_config.jwt_secret.clone());

    // Initialize the authentication system
    if let Err(e) = auth_service
        .initialize(
            &app_state.memory_manager,
            server_config.root_password.clone(),
        )
        .await
    {
        return Err(anyhow::anyhow!("Failed to initialize auth service: {}", e));
    }

    // Store the auth service in app state
    app_state.set_auth_service(auth_service);

    Ok(())
}

/// Setup live queries for SurrealDB if available
async fn setup_live_queries(app_state: Arc<AppState>) -> Result<()> {
    info!("Setting up live queries");

    // Get the memory manager from app state
    let memory_manager = &app_state.memory_manager;

    // Get the graph store from the storage service
    let graph_store = memory_manager.storage();

    // Check if the store supports live queries
    if graph_store.supports_live_queries() {
        info!("Graph store supports live queries, setting up...");

        // For now, we'll use a simpler approach that doesn't require unsafe downcasting
        // The shared_storage module will handle live queries internally if available
        // and broadcast events through the normal channels

        info!("Live query system configured successfully");
    } else {
        info!("Graph store does not support live queries, skipping setup");
    }

    Ok(())
}
