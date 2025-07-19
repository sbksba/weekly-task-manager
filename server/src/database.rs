// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
use crate::colors;

use anyhow::{Context, Result};
use chrono::{Utc, Weekday};
use common::{CreateTaskPayload, Task};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool}; // Added MigrateDatabase for database_exists/create_database
use tracing::{debug, info};

/// Establishes the database connection pool.
/// If the database does not exist, it creates it.
/// It also ensures the `tasks` table has the correct schema.
pub async fn establish_connection_pool(database_url: &str) -> Result<SqlitePool> {
    if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
        info!("Creating database {}", database_url);
        Sqlite::create_database(database_url) // Use the passed URL
            .await
            .context("Failed to create database")?;
    } else {
        info!("Database already exists.");
    }

    let pool = SqlitePool::connect(database_url) // Use the passed URL
        .await
        .context("Failed to connect to database")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_name TEXT NOT NULL,
            description TEXT NOT NULL,
            task_date DATE NOT NULL,
            client_color TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            deleted_at TIMESTAMP WITH TIME ZONE NULL,
            priority INTEGER NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .context("Failed to create 'tasks' table")?;

    info!("'tasks' table is ready.");

    Ok(pool)
}

/// Retrieves tasks for the current week (Monday to Sunday), excluding soft-deleted tasks.
pub async fn get_current_week_tasks_from_db(pool: &SqlitePool) -> Result<Vec<Task>> {
    let today = Utc::now().date_naive();
    let week_start = today.week(Weekday::Mon).first_day();
    let week_end = today.week(Weekday::Mon).last_day();

    let tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE task_date BETWEEN ? AND ? AND deleted_at IS NULL ORDER BY task_date ASC, priority ASC NULLS LAST;",
    )
    .bind(week_start)
    .bind(week_end)
    .fetch_all(pool)
    .await
    .context("Failed to retrieve current week's tasks from DB")?;

    Ok(tasks)
}

/// Inserts a new task into the database.
pub async fn create_task_in_db(pool: &SqlitePool, payload: CreateTaskPayload) -> Result<Task> {
    let task_date = payload.task_date.unwrap_or_else(|| Utc::now().date_naive());
    let client_color = colors::get_or_assign_client_color(&payload.client_name);
    let created_at = Utc::now();

    debug!("Insert values: client_name={}, description={}, task_date={}, client_color={}, created_at={}, priority={:?}",
           payload.client_name, payload.description, task_date, client_color, created_at, payload.priority);

    // Make sure to include deleted_at in the column list and provide a value (NULL for new tasks)
    let id = sqlx::query(
        "INSERT INTO tasks (client_name, description, task_date, client_color, created_at, deleted_at, priority) VALUES (?, ?, ?, ?, ?, NULL, ?)"
    )
    .bind(&payload.client_name)
    .bind(&payload.description)
    .bind(task_date)
    .bind(&client_color)
    .bind(created_at)
    .bind(payload.priority)
    .execute(pool)
    .await
    .context("Failed to insert task into DB")?
    .last_insert_rowid();

    let new_task = Task {
        id,
        client_name: payload.client_name,
        description: payload.description,
        task_date,
        client_color,
        created_at,
        deleted_at: None, // Newly created tasks are not deleted
        priority: payload.priority,
    };

    Ok(new_task)
}

/// Soft deletes a task from the database by setting its `deleted_at` timestamp.
/// Returns true if a task was updated, false if no task with the given ID was found.
#[allow(clippy::uninlined_format_args)]
pub async fn soft_delete_task_in_db(pool: &SqlitePool, task_id: i64) -> Result<bool> {
    debug!("Attempting to soft delete task with ID: {}", task_id);
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE tasks SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL", // Only update if not already deleted
    )
    .bind(now)
    .bind(task_id)
    .execute(pool)
    .await
    .context(format!("Failed to soft delete task with ID: {}", task_id))?;

    let rows_affected = result.rows_affected();
    info!(
        "Soft deleted {} rows for task ID: {}",
        rows_affected, task_id
    );

    Ok(rows_affected > 0)
}

/// Rolls over incomplete (not soft-deleted) tasks from today to tomorrow.
pub async fn rollover_tasks_in_db(pool: &SqlitePool) -> Result<usize> {
    let today = Utc::now().date_naive();
    let tomorrow = today.succ_opt().context("Failed to get tomorrow's date")?;

    debug!(
        "Attempting to roll over tasks from {} to {}",
        today, tomorrow
    );

    let result =
        sqlx::query("UPDATE tasks SET task_date = ? WHERE task_date = ? AND deleted_at IS NULL")
            .bind(tomorrow)
            .bind(today)
            .execute(pool)
            .await
            .context("Failed to roll over tasks in DB")?;

    let num_rolled_over = result.rows_affected() as usize;
    info!("Successfully rolled over {} tasks.", num_rolled_over);

    Ok(num_rolled_over)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use common::CreateTaskPayload;
    use std::fs;
    use std::path::PathBuf;

    const TEST_TARGET_DIR_PATH: &str = "database";

    /// Returns the absolute path to the test data directory.
    fn get_test_data_dir() -> PathBuf {
        // Path::new(".").canonicalize() gets the absolute path of the current working directory.
        // This makes the cleanup more robust regardless of where `cargo test` is called from.
        let mut path = std::env::current_dir()
            .expect("Failed to get current working directory for test cleanup");

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

    /// Helper function to set up an in-memory SQLite database for testing.
    /// This creates a fresh, empty database for each test, ensuring they are isolated.
    async fn setup_test_db() -> Result<SqlitePool> {
        // Use :memory: to create an in-memory database
        let pool = SqlitePool::connect("sqlite::memory:").await?;

        // Run the same table creation query as the main application
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                client_name TEXT NOT NULL,
                description TEXT NOT NULL,
                task_date DATE NOT NULL,
                client_color TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                deleted_at TIMESTAMP WITH TIME ZONE NULL,
                priority INTEGER NULL
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let pool = setup_test_db().await.unwrap();
        let today = Utc::now().date_naive();
        let payload = CreateTaskPayload {
            client_name: "Test Client".to_string(),
            description: "Test the database".to_string(),
            task_date: Some(today),
            priority: Some(5),
        };

        // Act: Create a new task in the test database
        let created_task = create_task_in_db(&pool, payload).await.unwrap();

        // Assert: The created task has the correct data
        assert_eq!(created_task.client_name, "Test Client");
        assert_eq!(created_task.description, "Test the database");
        assert_eq!(created_task.task_date, today);
        assert_eq!(created_task.priority, Some(5));
        assert!(created_task.id > 0); // Should have been assigned an ID by the DB

        // Act: Retrieve tasks for the current week
        let week_tasks = get_current_week_tasks_from_db(&pool).await.unwrap();

        // Assert: The newly created task is in the list
        assert_eq!(week_tasks.len(), 1);
        assert_eq!(week_tasks[0].id, created_task.id);
        assert_eq!(week_tasks[0].priority, Some(5));

        // Call this last to remove the created directory and its contents
        teardown_test_env_for_file_cleanup(&get_test_data_dir());
    }

    #[tokio::test]
    async fn test_create_task_without_priority() {
        let pool = setup_test_db().await.unwrap();
        let today = Utc::now().date_naive();
        let payload = CreateTaskPayload {
            client_name: "Client No Prio".to_string(),
            description: "Task without priority".to_string(),
            task_date: Some(today),
            priority: None, // No priority
        };

        let created_task = create_task_in_db(&pool, payload).await.unwrap();
        assert_eq!(created_task.priority, None); // Assert priority is None

        let week_tasks = get_current_week_tasks_from_db(&pool).await.unwrap();
        assert_eq!(week_tasks.len(), 1);
        assert_eq!(week_tasks[0].priority, None); // Assert retrieved priority is None
    }

    #[tokio::test]
    async fn test_soft_delete_task() {
        let pool = setup_test_db().await.unwrap();
        let payload = CreateTaskPayload {
            client_name: "Client to Delete".to_string(),
            description: "This task will be deleted".to_string(),
            task_date: Some(Utc::now().date_naive()),
            priority: Some(1),
        };
        let task_to_delete = create_task_in_db(&pool, payload).await.unwrap();

        // Assert: The task exists before deletion
        let tasks_before_delete = get_current_week_tasks_from_db(&pool).await.unwrap();
        assert_eq!(tasks_before_delete.len(), 1);

        // Act: Soft delete the task
        let was_deleted = soft_delete_task_in_db(&pool, task_to_delete.id)
            .await
            .unwrap();

        // Assert
        assert!(was_deleted); // The function should report success.

        // Assert: The task is no longer retrieved by the standard query
        let tasks_after_delete = get_current_week_tasks_from_db(&pool).await.unwrap();
        assert_eq!(tasks_after_delete.len(), 0);
    }

    #[tokio::test]
    async fn test_rollover_tasks() {
        let pool = setup_test_db().await.unwrap();
        let today = Utc::now().date_naive();
        let tomorrow = today.succ_opt().unwrap();

        // Create a task for today
        let payload_today = CreateTaskPayload {
            client_name: "Rollover Client".to_string(),
            description: "A task for today".to_string(),
            task_date: Some(today),
            priority: Some(10),
        };
        create_task_in_db(&pool, payload_today).await.unwrap();

        // Create a task for a different day (that should not be rolled over)
        let other_date = today - Duration::days(2);
        let payload_other = CreateTaskPayload {
            client_name: "Other Client".to_string(),
            description: "A task from another day".to_string(),
            task_date: Some(other_date),
            priority: Some(20),
        };
        create_task_in_db(&pool, payload_other).await.unwrap();

        // Act: Run the rollover function
        let num_rolled_over = rollover_tasks_in_db(&pool).await.unwrap();

        // Assert: Exactly one task should have been rolled over
        assert_eq!(num_rolled_over, 1);

        // Assert: The task's date is now tomorrow
        let tasks: Vec<Task> =
            sqlx::query_as("SELECT * FROM tasks WHERE client_name = 'Rollover Client'")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_date, tomorrow);
        assert_eq!(tasks[0].priority, Some(10));
    }

    #[tokio::test]
    async fn test_get_tasks_order_by_priority() {
        let pool = setup_test_db().await.unwrap();
        let today = Utc::now().date_naive();

        // Create tasks with different priorities
        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client A".to_string(),
                description: "Task Low Prio".to_string(),
                task_date: Some(today),
                priority: Some(10),
            },
        )
        .await
        .unwrap();

        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client B".to_string(),
                description: "Task High Prio".to_string(),
                task_date: Some(today),
                priority: Some(1),
            },
        )
        .await
        .unwrap();

        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client C".to_string(),
                description: "Task Medium Prio".to_string(),
                task_date: Some(today),
                priority: Some(5),
            },
        )
        .await
        .unwrap();

        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client D".to_string(),
                description: "Task No Prio".to_string(),
                task_date: Some(today),
                priority: None, // No priority
            },
        )
        .await
        .unwrap();

        // Retrieve tasks
        let tasks = get_current_week_tasks_from_db(&pool).await.unwrap();

        // Assert order: Task with priority 1 should be first, then 5, then 10, then None
        // (assuming all are for today and `ORDER BY priority ASC NULLS LAST` works as expected)
        // Note: The `task_date` also plays a role in `get_current_week_tasks_from_db`.
        // Let's create tasks for the same date to test priority ordering specifically.
        // Re-running test with only today's date for simpler priority ordering test.

        // Filter to only tasks for today to test priority ordering directly
        let today_tasks: Vec<Task> = tasks.into_iter().filter(|t| t.task_date == today).collect();

        // Sort order should be: priority 1, priority 5, priority 10, None
        assert_eq!(today_tasks.len(), 4); // Client A, C, D (all for today)
        assert_eq!(today_tasks[0].priority, Some(1)); // Client B, if it was for today.
        assert_eq!(today_tasks[0].description, "Task High Prio".to_string()); // This task has priority 1
        assert_eq!(today_tasks[1].priority, Some(5)); // This task has priority 5
        assert_eq!(today_tasks[2].priority, Some(10)); // This task has priority 10
                                                       // The task with None priority will be last if there are other tasks with None priority.
                                                       // In this case, `NULLS LAST` will put it after 10.
                                                       // Let's refine the test to make sure only tasks for today are considered and their order.

        // To properly test the priority order, we should ensure all tasks are for the same date
        // and then check their relative order.
        // The `get_current_week_tasks_from_db` already sorts by `task_date` first.
        // Let's create a new test that specifically checks the priority order for tasks on the same date.
    }

    #[tokio::test]
    async fn test_priority_ordering_on_same_date() {
        let pool = setup_test_db().await.unwrap();
        let today = Utc::now().date_naive();

        // Create tasks for today with various priorities
        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client C".to_string(),
                description: "Task Medium Prio".to_string(),
                task_date: Some(today),
                priority: Some(5),
            },
        )
        .await
        .unwrap();

        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client A".to_string(),
                description: "Task Low Prio".to_string(),
                task_date: Some(today),
                priority: Some(10),
            },
        )
        .await
        .unwrap();

        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client B".to_string(),
                description: "Task High Prio".to_string(),
                task_date: Some(today),
                priority: Some(1),
            },
        )
        .await
        .unwrap();

        create_task_in_db(
            &pool,
            CreateTaskPayload {
                client_name: "Client D".to_string(),
                description: "Task No Prio".to_string(),
                task_date: Some(today),
                priority: None,
            },
        )
        .await
        .unwrap();

        // Retrieve tasks for the current week (all created tasks are for today)
        let tasks = get_current_week_tasks_from_db(&pool).await.unwrap();

        // Assert the order based on priority (1, 5, 10, None)
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].description, "Task High Prio"); // Priority 1
        assert_eq!(tasks[1].description, "Task Medium Prio"); // Priority 5
        assert_eq!(tasks[2].description, "Task Low Prio"); // Priority 10
        assert_eq!(tasks[3].description, "Task No Prio"); // Priority None (NULLS LAST)
    }
}
