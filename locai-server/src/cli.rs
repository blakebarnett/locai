use clap::{Arg, ArgAction, Command, ValueHint};
use std::path::PathBuf;

/// CLI arguments for locai-server
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub port: Option<u16>,
    pub enable_auth: Option<bool>,
    pub allow_signup: Option<bool>,
    pub root_password: Option<String>,
    pub jwt_secret: Option<String>,
    pub jwt_expiration_hours: Option<u64>,
    pub config_file: Option<PathBuf>,
    pub rate_limit_rpm: Option<u32>,
    pub websocket_timeout: Option<u64>,
    pub enable_live_queries: Option<bool>,
    pub messaging_enabled: Option<bool>,
    pub messaging_auth_required: Option<bool>,
    pub max_request_size: Option<usize>,
    pub log_level: Option<String>,
}

impl CliArgs {
    /// Parse command line arguments
    pub fn parse() -> Self {
        let matches = Command::new("locai-server")
            .version(locai::VERSION)
            .author("Locai Team")
            .about("HTTP API server for the Locai memory service")
            .long_about(
                r#"Locai Server provides a REST API and WebSocket interface for the Locai 
memory management system. It supports authentication, real-time messaging, 
live queries, and integrates with SurrealDB for persistent storage.

The server can be configured through command line arguments or environment 
variables. Command line arguments take precedence over environment variables.

Examples:
  locai-server --port 8080 --enable-auth
  locai-server --config config.json --no-auth --allow-signup=false
  locai-server --messaging-disabled --log-level debug"#,
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .value_name("PORT")
                    .help("Port to listen on")
                    .long_help(
                        "Port number for the HTTP server to listen on. 
Environment variable: LOCAI_PORT",
                    )
                    .value_hint(ValueHint::Other)
                    .value_parser(clap::value_parser!(u16)),
            )
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .value_name("FILE")
                    .help("Configuration file path")
                    .long_help(
                        "Path to the configuration file. The file can contain 
JSON configuration that will be merged with environment variables and CLI arguments.
Environment variable: LOCAI_CONFIG_FILE",
                    )
                    .value_hint(ValueHint::FilePath)
                    .value_parser(clap::value_parser!(PathBuf)),
            )
            .arg(
                Arg::new("enable_auth")
                    .long("enable-auth")
                    .help("Enable authentication system")
                    .long_help(
                        "Enable JWT-based authentication system. When enabled, 
most API endpoints will require valid authentication tokens. A root user will be 
created on first run if one doesn't exist.
Environment variable: LOCAI_ENABLE_AUTH",
                    )
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("no_auth")
                    .long("no-auth")
                    .help("Disable authentication system")
                    .long_help(
                        "Disable authentication entirely. All API endpoints 
will be accessible without authentication. WARNING: Only use this in development 
or trusted environments.",
                    )
                    .action(ArgAction::SetTrue)
                    .conflicts_with("enable_auth"),
            )
            .arg(
                Arg::new("allow_signup")
                    .long("allow-signup")
                    .value_name("BOOL")
                    .help("Allow user registration")
                    .long_help(
                        "Allow new users to register accounts via the signup API. 
Set to false in production environments where you want to control user creation.
Environment variable: LOCAI_ALLOW_SIGNUP",
                    )
                    .value_parser(clap::value_parser!(bool)),
            )
            .arg(
                Arg::new("root_password")
                    .long("root-password")
                    .value_name("PASSWORD")
                    .help("Root user password")
                    .long_help(
                        "Set the password for the root user account. If not 
provided, a random password will be generated and displayed on first run.
Environment variable: LOCAI_ROOT_PASSWORD",
                    )
                    .value_hint(ValueHint::Other),
            )
            .arg(
                Arg::new("jwt_secret")
                    .long("jwt-secret")
                    .value_name("SECRET")
                    .help("JWT signing secret")
                    .long_help(
                        "Secret key used for signing JWT tokens. Should be a 
long, random string. If not provided, one will be generated automatically.
Environment variable: LOCAI_JWT_SECRET",
                    )
                    .value_hint(ValueHint::Other),
            )
            .arg(
                Arg::new("jwt_expiration")
                    .long("jwt-expiration")
                    .value_name("HOURS")
                    .help("JWT token expiration time in hours")
                    .long_help(
                        "How long JWT tokens remain valid before expiring. 
Default is 24 hours.
Environment variable: LOCAI_JWT_EXPIRATION_HOURS",
                    )
                    .value_parser(clap::value_parser!(u64)),
            )
            .arg(
                Arg::new("rate_limit")
                    .long("rate-limit")
                    .value_name("RPM")
                    .help("Rate limit in requests per minute")
                    .long_help(
                        "Maximum number of requests per minute allowed from 
a single client IP address.
Environment variable: LOCAI_RATE_LIMIT_RPM",
                    )
                    .value_parser(clap::value_parser!(u32)),
            )
            .arg(
                Arg::new("websocket_timeout")
                    .long("websocket-timeout")
                    .value_name("SECONDS")
                    .help("WebSocket connection timeout")
                    .long_help(
                        "How long to keep WebSocket connections alive without 
activity before closing them.
Environment variable: LOCAI_WEBSOCKET_TIMEOUT",
                    )
                    .value_parser(clap::value_parser!(u64)),
            )
            .arg(
                Arg::new("enable_live_queries")
                    .long("enable-live-queries")
                    .help("Enable live queries for real-time updates")
                    .long_help(
                        "Enable SurrealDB live queries for real-time data 
updates via WebSocket. Requires compatible storage backend.
Environment variable: LOCAI_ENABLE_LIVE_QUERIES",
                    )
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("messaging_enabled")
                    .long("messaging-enabled")
                    .help("Enable messaging features")
                    .long_help(
                        "Enable the built-in messaging system for real-time 
communication between clients.
Environment variable: LOCAI_MESSAGING_ENABLED",
                    )
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("messaging_disabled")
                    .long("messaging-disabled")
                    .help("Disable messaging features")
                    .long_help("Disable the messaging system entirely.")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("messaging_enabled"),
            )
            .arg(
                Arg::new("messaging_no_auth")
                    .long("messaging-no-auth")
                    .help("Disable authentication for messaging")
                    .long_help(
                        "Allow unauthenticated access to the messaging system. 
WARNING: Only use in development or trusted environments.
Environment variable: LOCAI_MESSAGING_AUTH_REQUIRED",
                    )
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("max_request_size")
                    .long("max-request-size")
                    .value_name("BYTES")
                    .help("Maximum request body size in bytes")
                    .long_help(
                        "Maximum size allowed for HTTP request bodies. 
Larger requests will be rejected.
Environment variable: LOCAI_MAX_REQUEST_SIZE",
                    )
                    .value_parser(clap::value_parser!(usize)),
            )
            .arg(
                Arg::new("log_level")
                    .long("log-level")
                    .value_name("LEVEL")
                    .help("Logging level")
                    .long_help(
                        "Set the logging level. Valid values: error, warn, info, debug, trace
Environment variable: RUST_LOG",
                    )
                    .value_parser(["error", "warn", "info", "debug", "trace"]),
            )
            .arg(
                Arg::new("help_env")
                    .long("help-env")
                    .help("Show all environment variables")
                    .long_help(
                        "Display a comprehensive list of all environment variables 
that can be used to configure the server.",
                    )
                    .action(ArgAction::SetTrue),
            )
            .get_matches();

        // Handle special help for environment variables
        if matches.get_flag("help_env") {
            Self::print_env_help();
            std::process::exit(0);
        }

        Self {
            port: matches.get_one::<u16>("port").copied(),
            enable_auth: if matches.get_flag("enable_auth") {
                Some(true)
            } else if matches.get_flag("no_auth") {
                Some(false)
            } else {
                None
            },
            allow_signup: matches.get_one::<bool>("allow_signup").copied(),
            root_password: matches.get_one::<String>("root_password").cloned(),
            jwt_secret: matches.get_one::<String>("jwt_secret").cloned(),
            jwt_expiration_hours: matches.get_one::<u64>("jwt_expiration").copied(),
            config_file: matches.get_one::<PathBuf>("config").cloned(),
            rate_limit_rpm: matches.get_one::<u32>("rate_limit").copied(),
            websocket_timeout: matches.get_one::<u64>("websocket_timeout").copied(),
            enable_live_queries: if matches.get_flag("enable_live_queries") {
                Some(true)
            } else {
                None
            },
            messaging_enabled: if matches.get_flag("messaging_enabled") {
                Some(true)
            } else if matches.get_flag("messaging_disabled") {
                Some(false)
            } else {
                None
            },
            messaging_auth_required: if matches.get_flag("messaging_no_auth") {
                Some(false)
            } else {
                None
            },
            max_request_size: matches.get_one::<usize>("max_request_size").copied(),
            log_level: matches.get_one::<String>("log_level").cloned(),
        }
    }

    /// Print comprehensive environment variable help
    fn print_env_help() {
        println!("Locai Server Environment Variables");
        println!("===================================");
        println!();
        println!("Server Configuration:");
        println!("  LOCAI_PORT                        - Server port (default: 3000)");
        println!(
            "  LOCAI_MAX_REQUEST_SIZE            - Max request body size in bytes (default: 16MB)"
        );
        println!(
            "  LOCAI_CONFIG_FILE                 - Path to config file (default: config.json)"
        );
        println!("  LOCAI_RATE_LIMIT_RPM              - Rate limit per minute (default: 1000)");
        println!(
            "  LOCAI_WEBSOCKET_TIMEOUT           - WebSocket timeout in seconds (default: 300)"
        );
        println!();
        println!("Authentication:");
        println!("  LOCAI_ENABLE_AUTH                 - Enable authentication (default: true)");
        println!(
            "  LOCAI_JWT_SECRET                  - JWT signing secret (auto-generated if not set)"
        );
        println!("  LOCAI_JWT_EXPIRATION_HOURS        - JWT expiration in hours (default: 24)");
        println!("  LOCAI_ALLOW_SIGNUP                - Allow user registration (default: true)");
        println!(
            "  LOCAI_ROOT_PASSWORD               - Root user password (auto-generated if not set)"
        );
        println!();
        println!("Live Queries:");
        println!("  LOCAI_ENABLE_LIVE_QUERIES         - Enable live queries (default: false)");
        println!("  LOCAI_LIVE_QUERY_BUFFER_SIZE      - Event buffer size (default: 100)");
        println!();
        println!("Messaging System:");
        println!("  LOCAI_MESSAGING_ENABLED           - Enable messaging (default: true)");
        println!(
            "  LOCAI_MESSAGING_ENABLE_CROSS_APP  - Enable cross-app messaging (default: true)"
        );
        println!(
            "  LOCAI_MESSAGING_AUTH_REQUIRED     - Require auth for messaging (default: true)"
        );
        println!("  LOCAI_MESSAGING_MAX_MESSAGE_SIZE  - Max message size in bytes (default: 1MB)");
        println!(
            "  LOCAI_MESSAGING_CONNECTION_TIMEOUT - Connection timeout in seconds (default: 60)"
        );
        println!(
            "  LOCAI_MESSAGING_HEARTBEAT_INTERVAL - Heartbeat interval in seconds (default: 30)"
        );
        println!("  LOCAI_MESSAGING_DATA_DIR          - Data directory for embedded storage");
        println!();
        println!("SurrealDB Configuration (shared with main Locai library):");
        println!("  SURREALDB_URL                      - SurrealDB endpoint URL");
        println!("  SURREALDB_NAMESPACE                - SurrealDB namespace (default: locai)");
        println!("  SURREALDB_DATABASE                 - SurrealDB database (default: main)");
        println!("  SURREALDB_USERNAME                 - SurrealDB username");
        println!("  SURREALDB_PASSWORD                 - SurrealDB password");
        println!();
        println!("Logging:");
        println!(
            "  RUST_LOG                           - Logging level (error, warn, info, debug, trace)"
        );
        println!();
        println!("Note: Command line arguments take precedence over environment variables.");
        println!("Use --help for CLI argument documentation.");
    }
}
