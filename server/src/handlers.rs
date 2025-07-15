// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
use crate::database;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{Utc, Weekday};
use common::{CreateTaskPayload, Task};
use sqlx::SqlitePool;
use tracing::{debug, error, info};

/// Handler for listing tasks for the current week.
pub async fn list_tasks(
    State(pool): State<SqlitePool>, // State injection (DB pool)
) -> Result<Json<Vec<Task>>, AppError> {
    let tasks = database::get_current_week_tasks_from_db(&pool).await?;
    info!("Successfully retrieved {} tasks.", tasks.len());
    Ok(Json(tasks))
}

/// Handler for creating a new task.
#[allow(clippy::unnecessary_lazy_evaluations)]
#[allow(clippy::uninlined_format_args)]
pub async fn create_task(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateTaskPayload>, // Extracting the request body as JSON
) -> Result<(StatusCode, Json<Task>), AppError> {
    debug!(
        "Received request to create task for client: {}",
        payload.client_name
    );
    // Validate the payload : name, description and date
    if payload.client_name.is_empty() || payload.description.is_empty() {
        error!("Validation failed: Client name or description is empty.");
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Client name and description cannot be empty.",
        ));
    }

    let today = Utc::now().date_naive();
    let current_week_start = today.week(Weekday::Mon).first_day();
    let current_week_end = today.week(Weekday::Mon).last_day();

    // Determine the actual task_date to be used
    let task_date_to_use = payload.task_date.unwrap_or_else(|| today);

    // Validate if the provided or default task_date is within the current week
    if task_date_to_use < current_week_start || task_date_to_use > current_week_end {
        error!(
            "Validation failed: Task date {} is outside the current week ({} to {}).",
            task_date_to_use, current_week_start, current_week_end
        );
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            &format!(
                "Task date must be within the current week (from {} to {}).",
                current_week_start, current_week_end
            ),
        ));
    }

    let new_task = database::create_task_in_db(&pool, payload).await?;

    info!("Task created successfully with ID: {}", new_task.id);

    // Return a 201 Created status with the new task as JSON.
    Ok((StatusCode::CREATED, Json(new_task)))
}

/// Handler for deleting a task by ID.
#[allow(clippy::needless_return)]
#[allow(clippy::uninlined_format_args)]
pub async fn delete_task(
    State(pool): State<SqlitePool>,
    Path(task_id): Path<i64>, // Extract task ID from the URL path
) -> Result<StatusCode, AppError> {
    debug!("Attempting to delete task with ID: {}", task_id);

    //let deleted = database::delete_task_from_db(&pool, task_id).await?;
    let deleted = database::soft_delete_task_in_db(&pool, task_id).await?;

    if deleted {
        info!("Task with ID {} deleted successfully.", task_id);
        Ok(StatusCode::NO_CONTENT) // 204 No Content for successful deletion
    } else {
        error!("Task with ID {} not found for deletion.", task_id);
        return Err(AppError::new(
            StatusCode::NOT_FOUND,
            &format!("Task with ID {} not found for deletion.", task_id),
        ));
    }
}

/// Handler for rollover tasks on the next day.
pub async fn rollover_tasks(
    State(pool): State<SqlitePool>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Return JSON for message/count
    debug!("Received request to roll over tasks.");

    let num_rolled_over = database::rollover_tasks_in_db(&pool).await?;

    info!("Successfully rolled over {} tasks.", num_rolled_over);

    Ok(Json(serde_json::json!({
        "message": format!("Successfully rolled over {} tasks.", num_rolled_over),
        "tasks_rolled_over": num_rolled_over
    })))
}

// --- Custom Error Handling ---
// This is a good practice for transforming our internal errors
// (e.g., from the database) into appropriate HTTP responses.

/// Our custom error type for the application.
pub struct AppError {
    code: StatusCode,
    message: String,
}

impl AppError {
    fn new(code: StatusCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

/// Allows converting an `anyhow::Error` (coming from `database.rs`)
/// into our `AppError`.
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // Log the internal error for debugging.
        tracing::error!("Internal server error: {:?}", err);
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: "An internal error occurred.".to_string(),
        }
    }
}

/// Allows Axum to convert our `AppError` into an HTTP `Response`.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(
            "Responding with error: status_code={}, message={}",
            self.code.as_u16(),
            self.message
        );
        (
            self.code,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use common::CreateTaskPayload;
    use sqlx::SqlitePool;

    // Helper to create a payload for tests
    fn create_test_payload(
        client_name: &str,
        description: &str,
        date: Option<NaiveDate>,
    ) -> Json<CreateTaskPayload> {
        Json(CreateTaskPayload {
            client_name: client_name.to_string(),
            description: description.to_string(),
            task_date: date,
        })
    }

    #[tokio::test]
    async fn test_create_task_validation_empty_name() {
        // Arrange
        // We can use a closed pool because the validation fails before any DB access.
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let payload = create_test_payload("", "A valid description", Some(Utc::now().date_naive()));

        // Act
        let result = create_task(State(pool), payload).await;

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, StatusCode::BAD_REQUEST);
        assert_eq!(err.message, "Client name and description cannot be empty.");
    }

    #[tokio::test]
    async fn test_create_task_validation_date_in_past() {
        // Arrange
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let past_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let payload = create_test_payload("Test Client", "A valid description", Some(past_date));

        // Act
        let result = create_task(State(pool), payload).await;

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, StatusCode::BAD_REQUEST);
        assert!(err
            .message
            .contains("Task date must be within the current week"));
    }
}
