// Copyright (c) 2025 sbksba
//
// This software is licensed under the terms of the MIT License.
// See the LICENSE file in the project root for the full license text.
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

// Define the directory where you want to store the data
const DATA_DIR: &str = "database";
const CLIENT_COLORS_FILE_NAME: &str = "client_colors.json";

// Struct to hold the client color map
#[derive(Serialize, Deserialize)]
pub struct ClientColorMap {
    colors: HashMap<String, String>,
    #[serde(skip)] // Don't serialize the palette
    palette: Arc<Vec<String>>,
    next_color_index: usize,
}

impl Default for ClientColorMap {
    fn default() -> Self {
        Self {
            colors: HashMap::new(),
            // A palette of 20 distinct, aesthetically pleasing colors.
            // These colors are chosen to be relatively distinguishable and work well together.
            palette: Arc::new(vec![
                "#1f77b4".to_string(), // Muted blue
                "#ff7f0e".to_string(), // Orange
                "#2ca02c".to_string(), // Green
                "#d62728".to_string(), // Red
                "#9467bd".to_string(), // Purple
                "#8c564b".to_string(), // Brown
                "#e377c2".to_string(), // Pink
                "#7f7f7f".to_string(), // Grey
                "#bcbd22".to_string(), // Olive
                "#17becf".to_string(), // Cyan
                "#aec7e8".to_string(), // Light blue
                "#ffbb78".to_string(), // Light orange
                "#98df8a".to_string(), // Light green
                "#ff9896".to_string(), // Light red
                "#c5b0d5".to_string(), // Light purple
                "#c49c94".to_string(), // Light brown
                "#f7b6d2".to_string(), // Light pink
                "#c7c7c7".to_string(), // Light grey
                "#dbdb8d".to_string(), // Light olive
                "#9edae5".to_string(), // Light cyan
            ]),
            next_color_index: 0,
        }
    }
}

lazy_static! {
    // This is the global, lazily initialized, thread-safe client color map.
    static ref CLIENT_COLORS: Arc<RwLock<ClientColorMap>> = {
        let colors_map = load_client_colors().unwrap_or_else(|e| {
            eprintln!("Warning: Could not load client colors file: {}. Creating new map. Error: {}", get_client_colors_path().display(), e);
            ClientColorMap::default()
        });
        Arc::new(RwLock::new(colors_map))
    };
}

// Helper function to get the full path to the client_colors.json file
fn get_client_colors_path() -> PathBuf {
    let mut path = PathBuf::new();
    path.push(DATA_DIR);
    path.push(CLIENT_COLORS_FILE_NAME);
    path
}

// Function to load client colors from a JSON file
fn load_client_colors() -> Result<ClientColorMap, Box<dyn std::error::Error>> {
    let path = get_client_colors_path();
    // Check if the file exists before trying to read it
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()).into());
    }

    let data = fs::read_to_string(&path)?;
    let mut map: ClientColorMap = serde_json::from_str(&data)?;

    // Re-initialize the palette as it's skipped during serialization
    map.palette = Arc::new(ClientColorMap::default().palette.as_ref().clone());
    // Ensure next_color_index is within bounds after loading
    map.next_color_index %= map.palette.len();

    Ok(map)
}

// Function to save client colors to a JSON file
fn save_client_colors(colors_map: &ClientColorMap) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_client_colors_path();

    // Ensure the directory exists before saving the file
    let parent_dir = path.parent().ok_or("Invalid path for client colors file")?;
    fs::create_dir_all(parent_dir)?; // Recursively creates directories if they don't exist

    let data = serde_json::to_string_pretty(colors_map)?;
    fs::write(&path, data)?;
    Ok(())
}

/// Function to get or assign a unique color to a client name.
/// It persists the assignment to a file.
#[allow(clippy::uninlined_format_args)]
pub fn get_or_assign_client_color(client_name: &str) -> String {
    let mut client_colors = CLIENT_COLORS.write(); // Acquire a write lock

    // Check if the client already has an assigned color
    if let Some(color) = client_colors.colors.get(client_name) {
        return color.clone();
    }

    // If not, assign a new color from the palette
    let color_to_assign = client_colors.palette[client_colors.next_color_index].clone();
    client_colors
        .colors
        .insert(client_name.to_string(), color_to_assign.clone());

    // Move to the next color in the palette, wrapping around if necessary
    client_colors.next_color_index =
        (client_colors.next_color_index + 1) % client_colors.palette.len();

    // Save the updated map to the file (error handling inside)
    if let Err(e) = save_client_colors(&client_colors) {
        eprintln!("Error saving client colors: {}", e);
    }

    color_to_assign
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Helper function to set up a temporary directory for testing to avoid
    // interfering with the real `database/client_colors.json` file.
    #[allow(dead_code)]
    fn setup_test_env() -> PathBuf {
        let dir = tempdir().unwrap();
        let db_dir = dir.path().join(DATA_DIR);
        fs::create_dir(&db_dir).unwrap();
        db_dir.join(CLIENT_COLORS_FILE_NAME)
    }

    // Helper to get a clean ClientColorMap for isolated tests.
    fn get_clean_map() -> ClientColorMap {
        ClientColorMap::default()
    }

    #[test]
    fn test_assign_first_color() {
        let mut map = get_clean_map();
        let client_name = "Client A";

        // Act: Assign a color to a new client
        let color = assign_color_to_client(&mut map, client_name);

        // Assert: Check if the assigned color is the first one from the palette
        assert_eq!(color, "#1f77b4");
        assert_eq!(map.colors.get(client_name), Some(&color));
        assert_eq!(map.next_color_index, 1);
    }

    #[test]
    fn test_assign_same_color_for_existing_client() {
        let mut map = get_clean_map();
        let client_name = "Client A";

        // Act: Assign color twice
        let color1 = assign_color_to_client(&mut map, client_name);
        let color2 = assign_color_to_client(&mut map, client_name);

        // Assert: The color should be the same and the index should not advance the second time
        assert_eq!(color1, color2);
        assert_eq!(map.next_color_index, 1);
    }

    #[test]
    fn test_assign_different_colors_for_different_clients() {
        let mut map = get_clean_map();

        // Act
        let color1 = assign_color_to_client(&mut map, "Client A");
        let color2 = assign_color_to_client(&mut map, "Client B");

        // Assert
        assert_ne!(color1, color2);
        assert_eq!(color1, "#1f77b4"); // First color
        assert_eq!(color2, "#ff7f0e"); // Second color
        assert_eq!(map.next_color_index, 2);
    }

    #[test]
    fn test_palette_wraps_around() {
        let mut map = get_clean_map();
        let palette_len = map.palette.len();

        // Act: Assign colors to exhaust the palette
        for i in 0..palette_len {
            let client_name = format!("Client {}", i);
            assign_color_to_client(&mut map, &client_name);
        }

        // Assert: next_color_index should wrap around to 0
        assert_eq!(map.next_color_index, 0);

        // Act: Assign one more color
        let next_color = assign_color_to_client(&mut map, "New Client After Wrap");

        // Assert: The color should be the first one from the palette again
        assert_eq!(next_color, map.palette[0]);
        assert_eq!(map.next_color_index, 1);
    }

    /// This is a test-only helper function that mirrors the logic of
    /// `get_or_assign_client_color` but operates on a mutable map instance
    /// instead of the global `lazy_static`, making it suitable for isolated unit tests.
    fn assign_color_to_client(map: &mut ClientColorMap, client_name: &str) -> String {
        if let Some(color) = map.colors.get(client_name) {
            return color.clone();
        }

        let color_to_assign = map.palette[map.next_color_index].clone();
        map.colors
            .insert(client_name.to_string(), color_to_assign.clone());
        map.next_color_index = (map.next_color_index + 1) % map.palette.len();

        color_to_assign
    }
}
