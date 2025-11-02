// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
mod colors;
mod database;
mod handlers;
mod routes;

use axum::http::HeaderName;
use std::net::SocketAddr;
use tokio_cron_scheduler::{Job, JobScheduler};
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

    // The cron job runs at 00:00 server time and calls the same DB function
    // that the manual endpoint uses.
    let rollover_pool = db_pool.clone(); // pool will be moved into the job closure

    // Initialise the scheduler (runs in its own background task)
    let scheduler = JobScheduler::new()
        .await
        .expect("Failed to create cron scheduler");

    // Cron expression "0 0 * * * *" → every day at 00:00 (seconds, minutes, hours, day, month, weekday)
    let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let pool = rollover_pool.clone();
        // The closure must return a **boxed** future.
        Box::pin(async move {
            match database::rollover_tasks_in_db(&pool).await {
                Ok(count) => tracing::info!("Rolled over {} tasks at midnight.", count),
                Err(e) => tracing::error!("Rollover failed: {:?}", e),
            }
        })
    })
    .expect("Failed to create cron job");

    scheduler
        .add(job)
        .await
        .expect("Failed to add cron job to scheduler");
    scheduler
        .start()
        .await
        .expect("Failed to start cron scheduler");

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
