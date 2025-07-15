use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use common::Task;
use http_body_util::BodyExt; // For `collect`
use serde_json::json;
use server::routes::create_router;
use sqlx::SqlitePool;
use std::fs;
use std::path::PathBuf;
use tower::ServiceExt; // For `oneshot` // Add these imports for path manipulation

const TEST_TARGET_DIR_PATH: &str = "database";

/// Returns the absolute path to the test data directory.
fn get_test_data_dir() -> PathBuf {
    // Path::new(".").canonicalize() gets the absolute path of the current working directory.
    // This makes the cleanup more robust regardless of where `cargo test` is called from.
    let mut path =
        std::env::current_dir().expect("Failed to get current working directory for test cleanup");

    // Append the relative path to your database directory
    path.push(TEST_TARGET_DIR_PATH);
    path
}

/// Cleans up the test environment.
fn teardown_test_env_for_file_cleanup(db_dir: &PathBuf) {
    if db_dir.exists() {
        if let Err(e) = fs::remove_dir_all(db_dir) {
            eprintln!(
                "Error: Failed to remove test database directory {:?}: {}",
                db_dir, e
            );
        }
    }
}

/// Helper function to set up a fresh, in-memory database for each test.
async fn setup_test_db_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    // The schema here MUST match the one in `database.rs` exactly.
    // The `deleted_at` column was missing and has been added.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_name TEXT NOT NULL,
            description TEXT NOT NULL,
            task_date DATE NOT NULL,
            client_color TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            deleted_at TIMESTAMP WITH TIME ZONE NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create tasks table in test DB");

    pool
}

#[tokio::test]
async fn test_create_and_list_tasks() {
    let pool = setup_test_db_pool().await;
    let app = create_router(pool);
    let today_str = Utc::now().date_naive().to_string(); // Use a dynamic date

    // Act: Create a new task via POST request
    let create_payload = json!({
        "client_name": "Test Client",
        "description": "Test Task Description",
        "task_date": today_str
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Assert: Check that the task was created successfully
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let created_task: Task = serde_json::from_slice(&body).unwrap();
    assert_eq!(created_task.client_name, "Test Client");

    // Act: List tasks via GET request
    let list_request = Request::builder()
        .method("GET")
        .uri("/api/tasks")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();

    // Assert: Check that the list contains the new task
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<Task> = serde_json::from_slice(&body).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, created_task.id);

    // Call this last to remove the created directory and its contents
    teardown_test_env_for_file_cleanup(&get_test_data_dir());
}

#[tokio::test]
async fn test_delete_task() {
    // Arrange: Create a task to be deleted
    let pool = setup_test_db_pool().await;
    let app = create_router(pool);
    let today_str = Utc::now().date_naive().to_string();
    let create_payload = json!({
        "client_name": "Client to Delete",
        "description": "A task to be deleted",
        "task_date": today_str
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let created_task: Task =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();

    // Act: Send a DELETE request for the created task
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/tasks/{}", created_task.id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();

    // Assert: The delete was successful (204 NO_CONTENT)
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Assert: The task list is now empty
    let list_request = Request::builder()
        .method("GET")
        .uri("/api/tasks")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(list_request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<Task> = serde_json::from_slice(&body).unwrap();
    assert!(tasks.is_empty());

    // Call this last to remove the created directory and its contents
    teardown_test_env_for_file_cleanup(&get_test_data_dir());
}

#[tokio::test]
async fn test_rollover_tasks() {
    // Arrange: Create a task for today
    let pool = setup_test_db_pool().await;
    let app = create_router(pool.clone()); // Clone pool for direct DB checks
    let today = Utc::now().date_naive();
    let tomorrow = today + Duration::days(1);
    let create_payload = json!({
        "client_name": "Rollover Client",
        "description": "This task should roll over",
        "task_date": today.to_string()
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();
    app.clone().oneshot(request).await.unwrap();

    // Act: Send a PATCH request to the rollover endpoint
    let rollover_request = Request::builder()
        .method("PATCH")
        .uri("/api/tasks/rollover")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(rollover_request).await.unwrap();

    // Assert: The rollover was successful
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let rollover_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rollover_response["tasks_rolled_over"], 1);

    // Assert: Verify directly in the DB that the task's date is now tomorrow
    let rolled_over_task: Task =
        sqlx::query_as("SELECT * FROM tasks WHERE client_name = 'Rollover Client'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rolled_over_task.task_date, tomorrow);

    // Call this last to remove the created directory and its contents
    teardown_test_env_for_file_cleanup(&get_test_data_dir());
}
/*
#[tokio::test]
async fn test_rollover_sunday_to_monday() {
    // Arrange
    let pool = setup_test_db_pool().await;
    let app = create_router(pool.clone()); // Clone pool for direct DB checks

    // Define a specific Sunday date (e.g., July 6, 2025, which is a Sunday)
    // You can pick any recent or future Sunday for consistency.
    let sunday =
        chrono::NaiveDate::from_ymd_opt(2025, 7, 6).expect("Failed to create NaiveDate for Sunday");
    let monday = sunday + Duration::days(1); // This will be the Monday after our chosen Sunday

    let create_payload = json!({
        "client_name": "Sunday Rollover Client",
        "description": "This task should roll over from Sunday to Monday",
        "task_date": sunday.to_string() // Set the task date to our specific Sunday
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    // Send the request to create the task for Sunday
    app.clone().oneshot(request).await.unwrap();

    // Act: Send a PATCH request to the rollover endpoint
    let rollover_request = Request::builder()
        .method("PATCH")
        .uri("/api/tasks/rollover")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(rollover_request).await.unwrap();

    // Assert: The rollover was successful
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let rollover_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rollover_response["tasks_rolled_over"], 1);

    // Assert: Verify directly in the DB that the task's date is now Monday
    let rolled_over_task: Task =
        sqlx::query_as("SELECT * FROM tasks WHERE client_name = 'Sunday Rollover Client'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rolled_over_task.task_date, monday);

    // Call this last to remove the created directory and its contents
    teardown_test_env_for_file_cleanup(&get_test_data_dir());
}
*/
#[tokio::test]
async fn test_create_task_empty_payload() {
    // Arrange
    let pool = setup_test_db_pool().await;
    let app = create_router(pool);
    let payload = json!({ "client_name": "", "description": "Some description" });

    // Act
    let request = Request::builder()
        .method("POST")
        .uri("/api/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        error_response["error"],
        "Client name and description cannot be empty."
    );

    // Call this last to remove the created directory and its contents
    teardown_test_env_for_file_cleanup(&get_test_data_dir());
}
