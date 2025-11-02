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
