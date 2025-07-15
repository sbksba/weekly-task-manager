# Weekly Task Manager Backend

![License](https://img.shields.io/github/license/sbksba/weekly-task-manager)
![Project Status](https://github.com/sbksba/weekly-task-manager/actions/workflows/rust-ci.yml/badge.svg?branch=main)
[![Release CI/CD](https://github.com/sbksba/weekly-task-manager/actions/workflows/release.yml/badge.svg)](https://github.com/sbksba/weekly-task-manager/actions/workflows/release.yml)

A robust and intuitive backend for a weekly task management application, built with Rust. This project emphasizes clean architecture, efficient database interactions, and a clear API for a companion frontend.

---

## Table of Contents

* [Features](#features)

* [Technologies Used](#technologies-used)

* [Project Structure](#project-structure)

* [API Endpoints](#api-endpoints)

* [Getting Started](#getting-started)

  * [Prerequisites](#prerequisites)

  * [Cloning the Repository](#cloning-the-repository)

  * [Backend Setup](#backend-setup)

  * [Running with Podman Compose](#running-with-podman-compose)

* [Database Management](#database-management)

* [Future Enhancements (Roadmap)](#future-enhancements-roadmap)

* [Contributing](#contributing)

* [License](#license)

* [Contact](#contact)

---

## Features

This application provides the core backend services for managing weekly tasks, including:

* **Task Creation & Retrieval:** Create new tasks and fetch all tasks for the current week.

* **Client Management:** Automatically assign unique IDs and colors to clients based on their name.

* **Delete Task:** Permanently delete any task from the system.

* **Automatic Rollover:** Uncompleted non-recurrent tasks from the current day are automatically rolled over to the next day.

* **Database Persistence:** All data is stored in a SQLite database.

## Technologies Used

### Backend

* **Rust:** The core programming language, chosen for its performance, safety, and concurrency.

* **Axum:** A fast, ergonomic, and modular web framework for Rust, built on Tokio.

* **SQLx:** An asynchronous, compile-time checked ORM for Rust, used for interacting with the database.

* **SQLite:** A lightweight, file-based relational database, ideal for this project's scale.

* **Chrono:** A powerful date and time library for Rust.

* **UUID:** For generating unique identifiers for clients.

* **Anyhow:** For simplified error handling.

### Containerization

* **Podman / Podman Compose:** For building and orchestrating the backend container.

### Frontend (Companion Project - *Not part of this repository*)

* This backend is designed to serve a simple HTML/CSS/JavaScript frontend.

## Project Structure

The repository is organized into the following key directories:

```
.
├── common/             # Shared data structures (e.g., Task, CreateTaskPayload)
│   └── src/
│       └── lib.rs
├── Container           # The container file use by Podman Compose orchestration
│   ├── Containerfile.backend
│   └── Containerfile.frontend
├── javascript-client   # The Rust frontend application
│   ├── client-app.html
│   └── dashboard-app.html
├── server/             # The Rust backend application
│   ├── src/
│   │   ├── main.rs     # Application entry point, router setup
│   │   ├── handlers.rs # API endpoint handlers (create, get, done, delete, rollover)
│   │   ├── database.rs # Database connection and query logic
│   │   ├── colors.rs   # Client ID and color generation logic
│   │   └── error.rs    # Custom error types
│   └── Cargo.toml      # Backend Rust dependencies
├── podman-compose.yml  # Podman Compose file for container orchestration
├── README.md           # This file
└── .gitignore          # Files/directories to ignore in Git
```

## API Endpoints

The backend exposes the following RESTful API endpoints:

| Method | Endpoint | Description | Payload (Request) | Response (Success) |
 | ----- | ----- | ----- | ----- | ----- |
| `GET` | `/tasks` | Retrieve all active tasks for the current week. | None | `List<Task>` |
| `POST` | `/tasks` | Create a new task. | `CreateTaskPayload` | `Task` (created) |
| `DELETE` | `/tasks/:id` | Permanently delete a task from the system. | None | `204 No Content` |
| `POST` | `/tasks/rollover` | Manually trigger rollover of tasks. | None | `200 OK` (rows affected) |

**Note on `Task` and `CreateTaskPayload` structure:**
(See `common/src/lib.rs` for full details)

* **`Task`**: `id`, `client_id`, `client_name`, `description`, `task_date`, `client_color`, `deleted_at`, `created_at`.

* **`CreateTaskPayload`**: `client_name`, `description`, `task_date` (optional).

## Getting Started

Follow these steps to set up and run the backend locally.

### Prerequisites

Make sure you have the following installed:

* **Rust:** [Install Rust](https://www.rust-lang.org/tools/install) (using `rustup`)

* **Podman:** [Install Podman](https://podman.io/docs/installation)

* **Podman Compose:** [Install Podman Compose](https://github.com/containers/podman-compose) (often installed via `pip install podman-compose`)

### Cloning the Repository

```
git clone https://github.com/sbksba/weekly-task-manager-backend.git
cd weekly-task-manager-backend
```

### Backend Setup (Without Podman) - For Development/Testing

If you want to run the Rust server directly (outside of Podman Compose for development):

Navigate to the server directory

```
cd server
```

Build the project (first time might take a while)

```
cargo build
```

Run the server (it will use an SQLite database file in the current directory)

```
cargo run
```

The server will typically run on `http://127.0.0.1:3000`.

### Running with Podman Compose

This is the recommended way to run the application as it handles the database setup and server execution in containers.

1. **Create a `.env` file:** In the root of your project, create a file named `.env` and add your database URL.

DATABASE_URL=sqlite://data/tasks.db


*Note: The `data` directory will be created as a volume inside the container to persist your SQLite database.*

2. **Build and run the containers:**

```
podman-compose up --build
```

* `--build`: Ensures your Rust application image is built. On subsequent runs, you can omit this if your code hasn't changed.

* This command will:

  * Build the `weekly-task-manager-backend` Docker image from your `Dockerfile`.

  * Start the database (SQLite file persistence) and the Rust backend service.

  * The backend will be accessible via `http://127.0.0.1:3000` (or `http://localhost:3000`).

3. **To run in detached mode (in the background):**

```
podman-compose up -d --build
```

4. **To stop and remove containers:**

```
podman-compose down
```

5. **Viewing Backend Logs:**
To see the real-time logs from your running backend service (named server in docker-compose.yaml), use:

```
podman-compose logs -f server
# Or, if you prefer the full container name (e.g., after `podman-compose ps`):
# podman logs weekly-task-manager-backend_server_1
```

6. **Accessing the Frontend Example:**
The `index.html` file (and its associated `script.js`) provided in the Frontend Example section is a local file. To access the client and dashboard:

  * Ensure your backend is running using podman-compose up.

  * Open the `index.html` file directly in your web browser. You can usually do this by navigating to the file in your file explorer and double-clicking it, or by dragging it into your browser window.

  * The JavaScript in `script.js` will automatically connect to your running backend at http://localhost:3000. If your backend is running on a different address or port, remember to update the `BACKEND_URL` constant in your `script.js` file.


## Frontend Example

This backend is designed to be consumed by a web frontend. Here's a basic example using plain HTML and JavaScript to demonstrate how to interact with the API.

`index.html` (Minimal Structure)

```
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Frontend for Rust Backend</title>
    <style>
        body { font-family: sans-serif; margin: 20px; }
        #taskForm input, #taskForm button { margin-bottom: 10px; padding: 8px; }
        #taskList li {
            background-color: #f0f0f0;
            padding: 10px;
            margin-bottom: 5px;
            border-radius: 4px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        #taskList button {
            background-color: #dc3545;
            color: white;
            border: none;
            padding: 5px 10px;
            border-radius: 3px;
            cursor: pointer;
        }
        #dashboard {
            border: 1px solid #ccc;
            padding: 15px;
            margin-bottom: 20px;
            border-radius: 8px;
            background-color: #e9ecef;
        }
        #clientStatsList li {
            display: inline-block;
            margin-right: 8px;
            padding: 4px 10px;
            border-radius: 15px;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <h1>Weekly Task Manager</h1>

    <div id="dashboard">
        <h2>Dashboard</h2>
        <p>Total Active Tasks: <span id="totalActiveTasks">0</span></p>
        <p>Total Soft-Deleted Tasks: <span id="totalSoftDeletedTasks">0</span></p>
        <h3>Tasks by Client:</h3>
        <ul id="clientStatsList"></ul>
    </div>

    <h2>Manage Tasks</h2>
    <div id="taskForm">
        <input type="text" id="clientName" placeholder="Client Name" required>
        <input type="text" id="description" placeholder="Description" required>
        <input type="date" id="taskDate">
        <button onclick="handleCreateTask()">Add Task</button>
    </div>
    <ul id="taskList">
        <!-- Tasks will be rendered here -->
    </ul>
    <button onclick="handleRolloverTasks()">Rollover Incomplete Tasks</button>

    <script src="script.js"></script>
</body>
</html>
```

`script.js` (API Interactions).
Create a file named script.js in the same directory as index.html.

```
// script.js

const BACKEND_URL = 'http://localhost:3000'; // Ensure this matches your backend's address

/**
 * Fetches all active tasks for the current week and updates the UI.
 */
async function fetchTasks() {
    try {
        const response = await fetch(`${BACKEND_URL}/tasks`);
        if (!response.ok) {
            const errorData = await response.json();
            throw new Error(`HTTP error! status: ${response.status}, message: ${errorData.message}`);
        }
        const tasks = await response.json();
        console.log('Fetched tasks:', tasks);
        renderTasks(tasks);
        renderDashboard(tasks); // Update dashboard too
    } catch (error) {
        console.error('Error fetching tasks:', error);
        alert(`Failed to load tasks: ${error.message}`);
    }
}

/**
 * Renders the given array of tasks into the HTML list.
 */
function renderTasks(tasks) {
    const taskList = document.getElementById('taskList');
    taskList.innerHTML = ''; // Clear previous tasks

    if (tasks.length === 0) {
        taskList.innerHTML = '<li>No tasks for this week.</li>';
        return;
    }

    tasks.forEach(task => {
        const li = document.createElement('li');
        li.textContent = `${task.client_name}: ${task.description} on ${task.task_date}`;

        // Apply the client_color received from the backend
        li.style.backgroundColor = task.client_color;
        li.style.color = getContrastColor(task.client_color); // Helper for text readability
        li.style.padding = '5px';
        li.style.margin = '5px 0';
        li.style.borderRadius = '3px';

        // Add a delete button for permanent removal
        const deleteBtn = document.createElement('button');
        deleteBtn.textContent = 'Delete';
        deleteBtn.style.marginLeft = '10px';
        deleteBtn.onclick = () => handleHardDeleteTask(task.id);
        li.appendChild(deleteBtn);

        taskList.appendChild(li);
    });
}

/**
 * Handles creating a new task.
 */
async function handleCreateTask() {
    const clientNameInput = document.getElementById('clientName');
    const descriptionInput = document.getElementById('description');
    const taskDateInput = document.getElementById('taskDate');

    const clientName = clientNameInput.value.trim();
    const description = descriptionInput.value.trim();
    const taskDate = taskDateInput.value;

    if (!clientName || !description) {
        alert('Please enter client name and description.');
        return;
    }

    const payload = { client_name: clientName, description: description };
    if (taskDate) { payload.task_date = taskDate; }

    try {
        const response = await fetch(`${BACKEND_URL}/tasks`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload),
        });

        if (response.status !== 201) {
            const errorData = await response.json();
            throw new Error(`Failed to create task: ${errorData.message || response.statusText}`);
        }

        const newTask = await response.json();
        console.log('Task created:', newTask);

        clientNameInput.value = ''; // Clear form
        descriptionInput.value = '';
        taskDateInput.value = '';

        fetchTasks(); // Refresh UI
    } catch (error) {
        console.error('Error creating task:', error);
        alert(`Failed to create task: ${error.message}`);
    }
}

/**
 * Handles permanently deleting a task.
 */
async function handleHardDeleteTask(taskId) {
    if (!confirm(`Are you sure you want to PERMANENTLY delete task ${taskId}? This cannot be undone.`)) {
        return;
    }
    try {
        const response = await fetch(`${BACKEND_URL}/tasks/${taskId}`, {
            method: 'DELETE',
        });

        if (response.status === 204) { // 204 No Content for successful DELETE
            console.log(`Task ${taskId} permanently deleted.`);
            fetchTasks(); // Refresh UI
        } else if (response.status === 404) {
            alert(`Task ${taskId} not found.`);
        } else {
            const errorData = await response.json();
            throw new Error(`Failed to delete task: ${errorData.message || response.statusText}`);
        }
    } catch (error) {
        console.error('Error deleting task:', error);
        alert(`Error: ${error.message}`);
    }
}

/**
 * Handles triggering the rollover of tasks.
 */
async function handleRolloverTasks() {
    if (!confirm("Are you sure you want to roll over today's incomplete tasks to tomorrow?")) {
        return;
    }

    try {
        const response = await fetch(`${BACKEND_URL}/tasks/rollover`, {
            method: 'POST',
        });

        if (!response.ok) {
            const errorData = await response.json();
            throw new Error(`HTTP error! status: ${response.status}, message: ${errorData.message}`);
        }

        const result = await response.json();
        console.log('Rollover result:', result);
        alert(result.message);

        fetchTasks(); // Refresh UI
    } catch (error) {
        console.error('Error rolling over tasks:', error);
        alert(`Failed to roll over tasks: ${error.message}`);
    }
}

/**
 * Aggregates statistics from the fetched tasks for the dashboard.
 */
function getDashboardStats(tasks) {
    let totalActiveTasks = 0;
    let totalSoftDeletedTasks = 0; // Backend returns all tasks, some might be soft-deleted
    const tasksByClient = {};

    tasks.forEach(task => {
        if (task.deleted_at === null) { // Task is not permanently deleted
            totalActiveTasks++;
            if (!tasksByClient[task.client_name]) {
                tasksByClient[task.client_name] = { count: 0, color: task.client_color };
            }
            tasksByClient[task.client_name].count++;
        } else {
            totalSoftDeletedTasks++;
        }
    });

    return { totalActiveTasks, totalSoftDeletedTasks, tasksByClient };
}

/**
 * Renders the dashboard statistics into the HTML.
 */
function renderDashboard(tasks) {
    const stats = getDashboardStats(tasks);

    document.getElementById('totalActiveTasks').textContent = stats.totalActiveTasks;
    document.getElementById('totalSoftDeletedTasks').textContent = stats.totalSoftDeletedTasks;

    const clientStatsList = document.getElementById('clientStatsList');
    clientStatsList.innerHTML = '';

    for (const clientName in stats.tasksByClient) {
        const clientData = stats.tasksByClient[clientName];
        const li = document.createElement('li');
        li.textContent = `${clientName}: ${clientData.count} tasks`;
        li.style.backgroundColor = clientData.color;
        li.style.color = getContrastColor(clientData.color);
        li.style.padding = '3px 8px';
        li.style.margin = '3px 0';
        li.style.borderRadius = '15px';
        li.style.display = 'inline-block';
        li.style.marginRight = '5px';
        clientStatsList.appendChild(li);
    }

    if (Object.keys(stats.tasksByClient).length === 0 && stats.totalActiveTasks === 0 && stats.totalSoftDeletedTasks === 0) {
        clientStatsList.innerHTML = '<li>No client statistics available.</li>';
    }
}

// Helper to determine good text color for contrast (simple version)
function getContrastColor(hexcolor) {
    const r = parseInt(hexcolor.slice(1, 3), 16);
    const g = parseInt(hexcolor.slice(3, 5), 16);
    const b = parseInt(hexcolor.slice(5, 7), 16);
    const y = (r * 299 + g * 587 + b * 114) / 1000;
    return (y >= 128) ? 'black' : 'white';
}

// Initial fetch when the page loads
fetchTasks();
```

## Database Management

The SQLite database file (`tasks.db`) will be persisted in a `data/` directory (created by Podman compose) relative to where your `docker-compose.yaml` is located.

* To clear the database (for a fresh start), simply delete the `data/` directory and restart `podman-compose up --build`.

* In a real production scenario, you would use database migration tools (like `sqlx-cli` which `SQLx` supports) to manage schema changes without data loss. For this project, the `CREATE TABLE IF NOT EXISTS` statement handles initial setup.

## Future Enhancements (Roadmap)

* **Task Prioritization:** Assign a numerical priority to tasks to help organize workload.

* **Recurrent Tasks:**  Define tasks that repeat at a specified interval (e.g., daily, weekly).

* **"Done" vs. "Delete" Semantics:** Implement distinct "done" and "permanent delete" functionalities, especially for recurrent tasks.

* **User Authentication:** Implement user registration and login.

* **Task Editing:** Allow updating existing task details.

* **More Complex Recurrence:** Support for monthly, yearly, or specific days of the month/year.

* **Notifications:** Integrate with notification systems (e.g., email, push).

* **Frontend Integration:** Develop a full-fledged frontend application (e.g., using React, Vue, Svelte) to consume this API.

* **Advanced Analytics:** Dashboard features to visualize task completion rates, priorities, etc.

## Contributing

Contributions are welcome! If you find a bug or have a feature request, please open an issue. If you'd like to contribute code, please fork the repository and open a pull request.

## License

This project is licensed under the MIT License - see the [LICENSE](https://www.google.com/search?q=LICENSE) file for details.
