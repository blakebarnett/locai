//! Server configuration module

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,

    /// Maximum request body size in bytes
    pub max_request_size: usize,

    /// Enable authentication
    pub enable_auth: bool,

    /// JWT secret key for signing tokens
    pub jwt_secret: String,

    /// JWT token expiration time in hours
    pub jwt_expiration_hours: u64,

    /// Allow user signup (set to false in production)
    pub allow_signup: bool,

    /// Root user password (generated on first run if not set)
    pub root_password: Option<String>,

    /// Path to encrypted config file
    pub config_file_path: PathBuf,

    /// Rate limiting: requests per minute
    pub rate_limit_rpm: u32,

    /// WebSocket connection timeout in seconds
    pub websocket_timeout: u64,

    /// Enable SurrealDB live queries for real-time updates
    pub enable_live_queries: bool,

    /// Live query buffer size for event channels
    pub live_query_buffer_size: usize,

    /// Messaging configuration
    pub messaging: MessagingConfig,
}

/// Messaging configuration for locai-server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingConfig {
    /// Enable messaging features
    pub enabled: bool,

    /// Storage backend for messaging
    pub storage_backend: StorageBackend,

    /// Enable cross-app messaging
    pub enable_cross_app: bool,

    /// Require authentication for messaging
    pub auth_required: bool,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

/// Storage backend configuration for messaging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// Phase 1: Embedded SurrealDB (single process)
    Embedded { data_dir: String },
    /// Phase 2: Remote SurrealDB cluster (future)
    Remote {
        endpoint: String,
        namespace: String,
        database: String,
        auth: Option<SurrealDBAuth>,
    },
}

/// SurrealDB authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBAuth {
    pub username: String,
    pub password: String,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_backend: StorageBackend::Embedded {
                data_dir: "./data/messaging".to_string(),
            },
            enable_cross_app: true,
            auth_required: true,
            max_message_size: 1024 * 1024, // 1MB
            connection_timeout: 60,
            heartbeat_interval: 30,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            max_request_size: 16 * 1024 * 1024, // 16MB
            enable_auth: true,                  // Enable by default when using SurrealDB
            jwt_secret: "".to_string(),         // Generated at runtime if not provided
            jwt_expiration_hours: 24,           // 24 hours
            allow_signup: true,                 // Allow signup by default (disable in production)
            root_password: None,
            config_file_path: PathBuf::from("config.json"),
            rate_limit_rpm: 1000,
            websocket_timeout: 300, // 5 minutes
            enable_live_queries: false,
            live_query_buffer_size: 100,
            messaging: MessagingConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Load configuration from CLI arguments and environment variables
    /// CLI arguments take precedence over environment variables
    pub fn from_cli_and_env(cli_args: crate::cli::CliArgs) -> Result<Self> {
        let mut config = Self::default();

        // CLI arguments take precedence over environment variables
        if let Some(port) = cli_args.port {
            config.port = port;
        } else if let Ok(port) = env::var("LOCAI_PORT") {
            config.port = port.parse()?;
        }

        if let Some(max_size) = cli_args.max_request_size {
            config.max_request_size = max_size;
        } else if let Ok(max_size) = env::var("LOCAI_MAX_REQUEST_SIZE") {
            config.max_request_size = max_size.parse()?;
        }

        if let Some(enable_auth) = cli_args.enable_auth {
            config.enable_auth = enable_auth;
        } else if let Ok(enable_auth) = env::var("LOCAI_ENABLE_AUTH") {
            config.enable_auth = enable_auth.parse().unwrap_or(true);
        }

        if let Some(jwt_secret) = cli_args.jwt_secret {
            config.jwt_secret = jwt_secret;
        } else if let Ok(jwt_secret) = env::var("LOCAI_JWT_SECRET") {
            config.jwt_secret = jwt_secret;
        } else if config.jwt_secret.is_empty() {
            // Generate a random JWT secret if not provided
            use rand::Rng;
            use rand::distr::Alphanumeric;
            let secret: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();
            config.jwt_secret = secret;
        }

        if let Some(exp_hours) = cli_args.jwt_expiration_hours {
            config.jwt_expiration_hours = exp_hours;
        } else if let Ok(exp_hours) = env::var("LOCAI_JWT_EXPIRATION_HOURS") {
            config.jwt_expiration_hours = exp_hours.parse()?;
        }

        if let Some(allow_signup) = cli_args.allow_signup {
            config.allow_signup = allow_signup;
        } else if let Ok(allow_signup) = env::var("LOCAI_ALLOW_SIGNUP") {
            config.allow_signup = allow_signup.parse().unwrap_or(true);
        }

        if let Some(root_password) = cli_args.root_password {
            config.root_password = Some(root_password);
        } else if let Ok(root_password) = env::var("LOCAI_ROOT_PASSWORD") {
            config.root_password = Some(root_password);
        }

        if let Some(config_path) = cli_args.config_file {
            config.config_file_path = config_path;
        } else if let Ok(config_path) = env::var("LOCAI_CONFIG_FILE") {
            config.config_file_path = PathBuf::from(config_path);
        }

        if let Some(rate_limit) = cli_args.rate_limit_rpm {
            config.rate_limit_rpm = rate_limit;
        } else if let Ok(rate_limit) = env::var("LOCAI_RATE_LIMIT_RPM") {
            config.rate_limit_rpm = rate_limit.parse()?;
        }

        if let Some(timeout) = cli_args.websocket_timeout {
            config.websocket_timeout = timeout;
        } else if let Ok(timeout) = env::var("LOCAI_WEBSOCKET_TIMEOUT") {
            config.websocket_timeout = timeout.parse()?;
        }

        if let Some(enable_live_queries) = cli_args.enable_live_queries {
            config.enable_live_queries = enable_live_queries;
        } else if let Ok(enable_live_queries) = env::var("LOCAI_ENABLE_LIVE_QUERIES") {
            config.enable_live_queries = enable_live_queries.parse().unwrap_or(false);
        }

        if let Ok(live_query_buffer_size) = env::var("LOCAI_LIVE_QUERY_BUFFER_SIZE") {
            config.live_query_buffer_size = live_query_buffer_size.parse()?;
        }

        // Messaging configuration
        if let Some(messaging_enabled) = cli_args.messaging_enabled {
            config.messaging.enabled = messaging_enabled;
        } else if let Ok(messaging_enabled) = env::var("LOCAI_MESSAGING_ENABLED") {
            config.messaging.enabled = messaging_enabled.parse().unwrap_or(true);
        }

        if let Ok(enable_cross_app) = env::var("LOCAI_MESSAGING_ENABLE_CROSS_APP") {
            config.messaging.enable_cross_app = enable_cross_app.parse().unwrap_or(true);
        }

        if let Some(auth_required) = cli_args.messaging_auth_required {
            config.messaging.auth_required = auth_required;
        } else if let Ok(auth_required) = env::var("LOCAI_MESSAGING_AUTH_REQUIRED") {
            config.messaging.auth_required = auth_required.parse().unwrap_or(true);
        }

        if let Ok(max_message_size) = env::var("LOCAI_MESSAGING_MAX_MESSAGE_SIZE") {
            config.messaging.max_message_size = max_message_size.parse()?;
        }

        if let Ok(connection_timeout) = env::var("LOCAI_MESSAGING_CONNECTION_TIMEOUT") {
            config.messaging.connection_timeout = connection_timeout.parse()?;
        }

        if let Ok(heartbeat_interval) = env::var("LOCAI_MESSAGING_HEARTBEAT_INTERVAL") {
            config.messaging.heartbeat_interval = heartbeat_interval.parse()?;
        }

        if let Ok(data_dir) = env::var("LOCAI_MESSAGING_DATA_DIR") {
            config.messaging.storage_backend = StorageBackend::Embedded { data_dir };
        }

        // Use the same SurrealDB environment variables for messaging as the main storage
        // This ensures consistency between main storage and messaging storage
        if let Ok(endpoint) = env::var("SURREALDB_URL") {
            let namespace = env::var("SURREALDB_NAMESPACE").unwrap_or_else(|_| "locai".to_string());
            let database = env::var("SURREALDB_DATABASE").unwrap_or_else(|_| "main".to_string());

            let auth = if let (Ok(username), Ok(password)) = (
                env::var("SURREALDB_USERNAME"),
                env::var("SURREALDB_PASSWORD"),
            ) {
                Some(SurrealDBAuth { username, password })
            } else {
                None
            };

            config.messaging.storage_backend = StorageBackend::Remote {
                endpoint,
                namespace,
                database,
                auth,
            };
        }

        Ok(config)
    }

    /// Generate a secure random JWT secret
    #[allow(dead_code)]
    pub fn generate_jwt_secret() -> String {
        use rand::Rng;
        use rand::distr::Alphanumeric;
        rand::rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }
}
