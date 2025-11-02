// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
use crate::handlers;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use sqlx::SqlitePool;

/// Creates and configures the application router.
pub fn create_router(pool: SqlitePool) -> Router {
    let api_v1 = Router::new()
        // Associates the `GET /api/tasks` route with the `list_tasks` handler
        .route("/tasks", get(handlers::list_tasks))
        // Associates the `POST /api/tasks` route with the `create_task` handler
        .route("/tasks", post(handlers::create_task))
        // Associates the `DELETE /api/tasks/{id}` route with the `delete_task` handler
        .route("/tasks/{id}", delete(handlers::delete_task))
        // Associates the `PATCH /api/tasks/rollover` route with the `rollover` handler
        .route("/tasks/rollover", patch(handlers::rollover_tasks))
        // Adds the database pool to the application state
        .with_state(pool);

    Router::new().nest("/api/v1", api_v1)
}
