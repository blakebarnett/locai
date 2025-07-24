//! # Server Deployment Guide
//! This example demonstrates how to configure and deploy the Locai server
//! in various environments. It's not a runnable example, but provides
//! configuration templates and deployment instructions.
//!
//! Run with: cargo run --example server_deployment
//! Or see server help: 
//! cargo run --bin locai-server -- --help

use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Locai Server Deployment Guide\n");

    // Basic server startup
    println!("🚀 Basic Server Startup");
    println!("   cargo run --bin locai-server");
    println!("   Server will start on http://localhost:3000\n");

    // Custom port
    println!("🔧 Custom Configuration");
    println!("   cargo run --bin locai-server -- --port 8080");
    println!("   Or with config file:");
    println!("   cargo run --bin locai-server -- --config-file locai-config.toml\n");

    // Example configuration file
    let example_config = r#"# Locai Server Configuration Example

[server]
port = 3000
data_dir = "./locai_data"
max_request_size = 16777216  # 16MB

[auth]
enable_auth = true
namespace = "locai"
database = "memories"
allow_signup = true

[features]
enable_live_queries = false
live_query_buffer_size = 100

[rate_limiting]
requests_per_minute = 1000

[websocket]
timeout_seconds = 300

[cors]
allowed_origins = ["http://localhost:3000", "http://localhost:5173"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["content-type", "authorization"]

[storage]
# SurrealDB embedded (default)
type = "surrealdb"
path = "./data/db"

[ml]
# Embedding model configuration
default_model = "BAAI/bge-m3"
model_cache_dir = "./models"

[logging]
level = "info"
format = "pretty"  # or "json" for structured logging
file = "./logs/locai-server.log"
"#;

    println!("📝 Configuration Template");
    println!("Example locai-config.toml:");
    println!("{}", example_config);

    // Environment variables
    println!("\n🌍 Environment Variables");
    println!("   LOCAI_PORT=8080 LOCAI_LOG_LEVEL=debug cargo run --bin locai-server\n");

    // Production setup
    println!("🏭 Production Deployment");
    println!("   cargo run --bin locai-server -- --enable-auth --jwt-secret your-secret-key");
    println!("   Recommended: Use reverse proxy (nginx) for HTTPS\n");

    // Release build
    println!("📦 Release Build");
    println!("   cargo build --release --bin locai-server");

    let systemd_service = r#"[Unit]
Description=Locai Memory Service
After=network.target

[Service]
Type=simple
User=locai
WorkingDirectory=/opt/locai
ExecStart=/opt/locai/locai-server --config-file /etc/locai/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target"#;

    println!("\n🐧 Systemd Service");
    println!("{}", systemd_service);

    let dockerfile = r#"FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin locai-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/locai-server /usr/local/bin/
EXPOSE 3000
CMD ["locai-server"]"#;

    println!("\n🐳 Docker Deployment");
    println!("{}", dockerfile);

    // Best practices
    println!("\n✅ Production Best Practices");
    println!("   - Use environment variables for secrets");
    println!("   - Enable authentication in production");  
    println!("   - Set up reverse proxy with SSL/TLS");
    println!("   - Monitor logs and performance");
    println!("   - Regular database backups");
    println!("   - Store config in /etc/locai/ for production");
    println!("   - Use systemd or Docker for process management");

    Ok(())
} 