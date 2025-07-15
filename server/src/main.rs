// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
mod colors;
mod database;
mod handlers;
mod routes;

use axum::http::HeaderName;
use chrono::Utc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tower_http::cors::{Any, CorsLayer};

// Define the DB_URL here for the main application's use.
const MAIN_DB_URL: &str = "sqlite://database/sqlite.db";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting up the server...");

    //let db_pool = match database::establish_connection_pool().await
    let db_pool = match database::establish_connection_pool(MAIN_DB_URL).await {
        Ok(pool) => {
            tracing::info!("Database connection was made successfully.");
            pool
        }
        Err(e) => {
            tracing::error!("Failed to connect with the database: {:?}", e);
            std::process::exit(1);
        }
    };

    let rollover_pool = db_pool.clone(); // Clone the pool for the rollover task
    let last_rollover_date = Arc::new(Mutex::new(Utc::now().date_naive())); // Store last date rollover happened

    tokio::spawn(async move {
        // Set an interval for checking.
        // For testing, you might use `Duration::from_secs(60)` for every minute.
        let mut interval = time::interval(Duration::from_secs(5 * 60)); // Check every 5 minutes

        // The first tick completes immediately. Skip it to wait for the first interval.
        interval.tick().await;

        loop {
            interval.tick().await; // Wait for the next interval tick

            let current_date = Utc::now().date_naive();
            let mut last_date_guard = last_rollover_date.lock().await;

            if *last_date_guard < current_date {
                // If the current date is greater than the last date we rolled over for,
                // it means a new day has started.
                tracing::info!(
                    "New day detected: {}, performing task rollover.",
                    current_date
                );
                match database::rollover_tasks_in_db(&rollover_pool).await {
                    Ok(count) => {
                        tracing::info!(
                            "Successfully rolled over {} tasks for {}.",
                            count,
                            current_date
                        );
                        *last_date_guard = current_date; // Update the last processed date
                    }
                    Err(e) => {
                        tracing::error!("Error during automatic task rollover: {:?}", e);
                    }
                }
            } else {
                tracing::debug!(
                    "No new day yet. Current date: {}. Last rollover date: {}.",
                    current_date,
                    *last_date_guard
                );
            }
        }
    });

    let app_routes = routes::create_router(db_pool);

    // Configure CORS here, applying it globally to the router
    /*
    let cors = CorsLayer::new()
        .allow_methods(Any) // Allow all HTTP methods
        .allow_headers(Any) // Allow all headers
        .allow_origin(Any); // Allow all origins
    */
    let cors = CorsLayer::new()
        .allow_methods(Any) // Autorise toutes les méthodes HTTP
        // Liste explicite des en-têtes que votre frontend pourrait envoyer.
        // Si vous n'utilisez pas d'authentification par token, 'authorization' n'est pas nécessaire.
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("accept"),
            // Si vous prévoyez d'envoyer des tokens d'authentification:
            // HeaderName::from_static("authorization"),
        ])
        .allow_origin(Any); // Autorise toutes les origines
                            // Assurez-vous que .allow_credentials(true) est bien COMMENTÉ ou SUPPRIMÉ
                            // si vous utilisez .allow_origin(Any) ou .allow_headers(Any)

    let app = app_routes.layer(cors); // Apply the CORS layer

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("The server listens on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
