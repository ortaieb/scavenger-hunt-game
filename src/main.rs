mod auth;
mod config;
mod db;
mod handlers;
mod models;
mod routes;
mod services;
mod utils;

use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use auth::{AuthState, JwtService};
use config::Config;
use db::{create_connection_pool, run_migrations, DatabasePool};
use routes::{create_api_router, AppState};
use services::{AuthService, ImageService, LocationService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Scavenger Hunt Game Server");

    // Load configuration
    let config = Config::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");

    // Create database connection pool
    let pool = create_connection_pool(&config).await.map_err(|e| {
        error!("Failed to create database connection pool: {}", e);
        e
    })?;

    info!("Database connection pool created");

    // Run database migrations
    run_migrations(&pool).await.map_err(|e| {
        error!("Failed to run database migrations: {}", e);
        e
    })?;

    info!("Database migrations completed");

    // Initialize services
    let jwt_service = Arc::new(JwtService::new(&config.jwt_secret));
    let auth_service = Arc::new(AuthService::new(jwt_service.clone(), pool.clone()));
    let location_service = Arc::new(LocationService::new(pool.clone()));
    let image_service = Arc::new(ImageService::new(
        config.image_checker_url.clone(),
        config.image_base_dir.clone(),
    ));

    info!("Services initialized");

    // Create auth state for middleware
    let auth_state = AuthState { jwt_service };

    // Create application state
    let app_state = AppState {
        pool: pool.clone(),
        auth_service,
        location_service,
        image_service,
        auth_state,
    };

    // Create API router
    let app = create_api_router(app_state);

    info!("API router created");

    // Create server
    let listener = tokio::net::TcpListener::bind(&config.server_address())
        .await
        .map_err(|e| {
            error!(
                "Failed to bind to address {}: {}",
                config.server_address(),
                e
            );
            e
        })?;

    info!("Server listening on {}", config.server_address());

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            e
        })?;

    info!("Server shutdown complete");

    Ok(())
}

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
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }

    info!("Starting graceful shutdown...");
}
