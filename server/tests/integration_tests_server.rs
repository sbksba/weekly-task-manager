// tests/integration_tests_server.rs
// ------------------------------------------------------------
// Integration tests for the Axum server (backend)
// ------------------------------------------------------------

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use common::Task;
use http_body_util::BodyExt; // for `.collect()`
use serde_json::json;
use server::routes::create_router;
use sqlx::SqlitePool;
use tower::ServiceExt; // for `.oneshot()`

// ---------------------------------------------------------------------
// Helper: spin-up an in-memory SQLite pool with the exact schema used
// ---------------------------------------------------------------------
async fn setup_test_db_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to connect to in-memory SQLite");

    // The schema must match `src/database.rs` (including the `deleted_at` column)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_name TEXT NOT NULL,
            description TEXT NOT NULL,
            task_date DATE NOT NULL,
            client_color TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            deleted_at TIMESTAMP NULL,
            priority INTEGER NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create tasks table");

    pool
}

// ---------------------------------------------------------------------
// Helper: build a router (no auth required)
// ---------------------------------------------------------------------
fn router_without_auth(pool: SqlitePool) -> axum::Router {
    // The router does **not** read any env vars for auth.
    create_router(pool)
}

// ---------------------------------------------------------------------
// Helper: create a request (builder → request).  `body_opt` is
// `Some(json_string)` for POST/PUT, otherwise `None` for GET/DELETE/PATCH.
// ---------------------------------------------------------------------
fn make_request(method: &str, uri: &str, body_opt: Option<String>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);

    // If we have a body we also need a Content-Type header.
    let request = if let Some(body) = body_opt {
        builder = builder.header("Content-Type", "application/json");
        builder.body(Body::from(body)).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };

    request
}

// ---------------------------------------------------------------------
// TEST 1 – Create a task and then list it
// ---------------------------------------------------------------------
#[tokio::test]
async fn test_create_and_list_tasks() {
    let pool = setup_test_db_pool().await;
    let app = router_without_auth(pool.clone());

    // -----------------------------------------------------------------
    // 1️⃣  POST /api/v1/tasks  (create)
    // -----------------------------------------------------------------
    let today = Utc::now().date_naive().to_string();

    let payload = json!({
        "client_name": "Test Client",
        "description": "Test Task Description",
        "task_date": today,
        "priority": 1
    })
    .to_string();

    let request = make_request("POST", "/api/v1/tasks", Some(payload));

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Grab the created task from the body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let created_task: Task = serde_json::from_slice(&body).unwrap();
    assert_eq!(created_task.client_name, "Test Client");

    // -----------------------------------------------------------------
    // 2️⃣  GET /api/v1/tasks  (list)
    // -----------------------------------------------------------------
    let list_req = make_request("GET", "/api/v1/tasks", None);

    let response = app.oneshot(list_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<Task> = serde_json::from_slice(&body).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, created_task.id);
}

// ---------------------------------------------------------------------
// TEST 2 – Soft-delete a task
// ---------------------------------------------------------------------
#[tokio::test]
async fn test_delete_task() {
    let pool = setup_test_db_pool().await;
    let app = router_without_auth(pool.clone());

    // ---- create a task ------------------------------------------------
    let today = Utc::now().date_naive().to_string();
    let payload = json!({
        "client_name": "Client to Delete",
        "description": "A task to be deleted",
        "task_date": today,
        "priority": 1
    })
    .to_string();

    let create_req = make_request("POST", "/api/v1/tasks", Some(payload));

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let created: Task =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();

    // ---- delete it ----------------------------------------------------
    let del_req = make_request("DELETE", &format!("/api/v1/tasks/{}", created.id), None);

    let resp = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // ---- ensure the list is now empty ---------------------------------
    let list_req = make_request("GET", "/api/v1/tasks", None);

    let resp = app.oneshot(list_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<Task> = serde_json::from_slice(&body).unwrap();
    assert!(tasks.is_empty());
}

// ---------------------------------------------------------------------
// TEST 3 – Rollover (move today’s tasks to tomorrow)
// ---------------------------------------------------------------------
#[tokio::test]
async fn test_rollover_tasks() {
    let pool = setup_test_db_pool().await;
    let app = router_without_auth(pool.clone());

    // ---- create a task for *today* ------------------------------------
    let today = Utc::now().date_naive();
    let tomorrow = today + Duration::days(1);

    let payload = json!({
        "client_name": "Rollover Client",
        "description": "Should be rolled over",
        "task_date": today.to_string(),
        "priority": 1
    })
    .to_string();

    let create_req = make_request("POST", "/api/v1/tasks", Some(payload));
    app.clone().oneshot(create_req).await.unwrap();

    // ---- trigger the rollover endpoint --------------------------------
    let roll_req = make_request("PATCH", "/api/v1/tasks/rollover", None);

    let resp = app.clone().oneshot(roll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // ---- verify the DB directly ---------------------------------------
    let rolled: Task = sqlx::query_as("SELECT * FROM tasks WHERE client_name = 'Rollover Client'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(rolled.task_date, tomorrow);
}

// ---------------------------------------------------------------------
// TEST 4 – Payload validation (empty client name / description)
// ---------------------------------------------------------------------
#[tokio::test]
async fn test_create_task_empty_payload() {
    let pool = setup_test_db_pool().await;
    let app = router_without_auth(pool);

    // Empty `client_name` triggers the validator error
    let payload = json!({
        "client_name": "",
        "description": "Some description",
        "task_date": Utc::now().date_naive().to_string(),
        "priority": 1
    })
    .to_string();

    let req = make_request("POST", "/api/v1/tasks", Some(payload));

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // The validator returns a JSON object with an `error` field that contains
    // the concatenated field-error messages.
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let err_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Ensure the error mentions the `client_name` field.
    let err_msg = err_json
        .get("error")
        .and_then(|v| v.as_str())
        .expect("error field missing");
    assert!(
        err_msg.contains("client_name"),
        "unexpected error message: {}",
        err_msg
    );
}
