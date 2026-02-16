use std::path::Path;

use crate::beatmap::{ChartFile, SongMetadata};

/// Save a chart file in RON format.
pub fn save_chart_ron(chart: &ChartFile, path: &Path) -> Result<(), String> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .separate_tuple_members(true);
    let data = ron::ser::to_string_pretty(chart, config)
        .map_err(|e| format!("RON serialize error: {e}"))?;
    std::fs::write(path, data).map_err(|e| format!("Write error: {e}"))
}

/// Save metadata in RON format.
pub fn save_metadata_ron(metadata: &SongMetadata, path: &Path) -> Result<(), String> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(3)
        .separate_tuple_members(true);
    let data = ron::ser::to_string_pretty(metadata, config)
        .map_err(|e| format!("RON serialize error: {e}"))?;
    std::fs::write(path, data).map_err(|e| format!("Write error: {e}"))
}

/// Export a chart file as JSON.
pub fn export_chart_json(chart: &ChartFile, path: &Path) -> Result<(), String> {
    let data = serde_json::to_string_pretty(chart)
        .map_err(|e| format!("JSON serialize error: {e}"))?;
    std::fs::write(path, data).map_err(|e| format!("Write error: {e}"))
}

/// Import a chart file from JSON.
pub fn import_chart_json(path: &Path) -> Result<ChartFile, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("JSON parse error: {e}"))
}
