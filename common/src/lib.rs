// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[allow(clippy::doc_overindented_list_items)]
/// Represents a task within the system.
///
/// Derivation attributes (derive):
/// - `Serialize`, `Deserialize`: Allows conversion to/from JSON.
/// - `Debug`: Enables displaying the structure for debugging (e.g., `println!("{:?}", task)`).
/// - `Clone`: Allows creating copies of the object.
/// - `sqlx::FromRow`: Allows `sqlx` to create a `Task` instance directly
///    from a database result row.
#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Task {
    pub id: i64,
    pub client_name: String,
    pub description: String,
    // We use NaiveDate because we are only interested in the day,
    // without a timezone.
    pub task_date: NaiveDate,
    pub client_color: String,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub priority: Option<i32>, // (e.g., 1 = high, lower number = higher priority)
}

/// Structure used to receive task creation data from the API.
/// It's a good practice to separate database models (`Task`)
/// from API models (`CreateTaskPayload`), as they may have different fields.
/// Here, `task_date` is optional.
#[derive(Deserialize, Debug, Validate)]
pub struct CreateTaskPayload {
    #[validate(length(min = 1, max = 100))]
    pub client_name: String,
    #[validate(length(min = 1, max = 500))]
    pub description: String,
    // The day is optional. If not provided,
    // we'll use the current day on the server-side.
    pub task_date: Option<NaiveDate>,
    #[serde(default)]
    pub priority: Option<i32>,
}

/// Represents a client and their associated color.
/// For now, this is a simple structure, but it could be extended.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Client {
    pub name: String,
    pub color: String,
}
