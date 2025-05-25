use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod docker;
mod models;
mod web;
mod auth;

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration first
    let config = Config::from_env().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("See documentation for available environment variables");
        e
    })?;

    // Initialize structured logging with environment-based configuration
    init_tracing(&config.log_level)?;

    info!("🐳 Simple Docker Manager starting up");
    info!("Configuration: {:#?}", config);

    // Build the application with middleware
    let app = web::app_router(&config).layer(TraceLayer::new_for_http());

    // Bind to the configured address
    let bind_addr = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.map_err(|e| {
        error!("Failed to bind to {}: {}", bind_addr, e);
        format!("Cannot bind to {}. Port may be in use or address unavailable", bind_addr)
    })?;

    let local_addr = listener.local_addr()?;
    info!("🚀 Server listening on http://{}", local_addr);
    info!("📊 Dashboard: http://{}/metrics", local_addr);
    info!("🏠 Management: http://{}/", local_addr);

    // Set up graceful shutdown signal handling
    let shutdown_signal = shutdown_signal();

    // Start the server with graceful shutdown
    info!("✅ Server ready! Press Ctrl+C to stop");
    
    // Start the server and wait for shutdown signal
    let server_handle = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal);
    
    // Run the server until it completes (either by shutdown signal or error)
    server_handle.await.map_err(|e| {
        error!("Server error: {}", e);
        e
    })?;

    info!("🛑 Server stopped gracefully");
    Ok(())
}

/// Initialize tracing with environment-based log level configuration
fn init_tracing(log_level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .map_err(|e| format!("Invalid log level '{}': {}", log_level, e))?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    Ok(())
}

/// Set up graceful shutdown signal handling for multiple platforms
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown...");
        }
    }
} 