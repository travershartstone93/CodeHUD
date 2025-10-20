use crate::{GuiResult, GuiMessage};
use egui::{Context, Ui};
use std::path::PathBuf;

/// Base trait for all GUI views matching PyQt5 widget pattern
pub trait GuiView {
    /// Render the view content
    fn render(&mut self, ui: &mut Ui, ctx: &Context) -> GuiResult<()>;

    /// Handle incoming messages
    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()>;

    /// Get the view title
    fn get_title(&self) -> String;

    /// Called when view becomes active (optional)
    fn on_activate(&mut self) -> GuiResult<()> {
        Ok(())
    }

    /// Called when view becomes inactive (optional)
    fn on_deactivate(&mut self) -> GuiResult<()> {
        Ok(())
    }

    /// Called when view is being closed (optional)
    fn on_close(&mut self) -> GuiResult<bool> {
        Ok(true) // true = allow close
    }
}

/// Base trait for GUI components matching PyQt5 widget pattern
pub trait GuiComponent {
    /// Get component name
    fn name(&self) -> &str;

    /// Render the component
    fn render(&mut self, ui: &mut Ui, ctx: &Context) -> GuiResult<()>;

    /// Handle incoming messages
    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()>;

    /// Get component visibility
    fn is_visible(&self) -> bool;

    /// Set component visibility
    fn set_visible(&mut self, visible: bool);

    /// Get component enabled state
    fn is_enabled(&self) -> bool;

    /// Set component enabled state
    fn set_enabled(&mut self, enabled: bool);
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

pub fn format_duration(duration_ms: u64) -> String {
    if duration_ms < 1000 {
        format!("{}ms", duration_ms)
    } else if duration_ms < 60_000 {
        format!("{:.1}s", duration_ms as f64 / 1000.0)
    } else if duration_ms < 3_600_000 {
        format!("{}m {}s", duration_ms / 60_000, (duration_ms % 60_000) / 1000)
    } else {
        format!(
            "{}h {}m",
            duration_ms / 3_600_000,
            (duration_ms % 3_600_000) / 60_000
        )
    }
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

pub fn get_file_extension(path: &PathBuf) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

pub fn is_source_file(path: &PathBuf) -> bool {
    if let Some(ext) = get_file_extension(path) {
        matches!(
            ext.as_str(),
            "rs" | "py" | "js" | "ts" | "java" | "cpp" | "c" | "h" | "go" | "rb" | "php" | "cs" | "kt" | "swift"
        )
    } else {
        false
    }
}

pub fn get_language_from_extension(extension: &str) -> &'static str {
    match extension {
        "rs" => "Rust",
        "py" => "Python",
        "js" => "JavaScript",
        "ts" => "TypeScript",
        "java" => "Java",
        "cpp" | "cc" | "cxx" => "C++",
        "c" => "C",
        "h" | "hpp" => "C/C++ Header",
        "go" => "Go",
        "rb" => "Ruby",
        "php" => "PHP",
        "cs" => "C#",
        "kt" => "Kotlin",
        "swift" => "Swift",
        "html" => "HTML",
        "css" => "CSS",
        "json" => "JSON",
        "xml" => "XML",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "md" => "Markdown",
        "txt" => "Text",
        _ => "Unknown",
    }
}

pub fn calculate_health_score(
    performance: f32,
    quality: f32,
    security: f32,
    maintainability: f32,
) -> f32 {
    // Weighted average of different health aspects
    let weights = [0.3, 0.3, 0.2, 0.2]; // performance, quality, security, maintainability
    let scores = [performance, quality, security, maintainability];

    scores
        .iter()
        .zip(weights.iter())
        .map(|(score, weight)| score * weight)
        .sum()
}

pub fn color_for_score(score: f32) -> egui::Color32 {
    if score >= 80.0 {
        egui::Color32::GREEN
    } else if score >= 60.0 {
        egui::Color32::YELLOW
    } else if score >= 40.0 {
        egui::Color32::from_rgb(255, 165, 0) // Orange
    } else {
        egui::Color32::RED
    }
}

pub fn validate_project_path(path: &PathBuf) -> GuiResult<()> {
    if !path.exists() {
        return Err(crate::GuiError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path does not exist: {}", path.display()),
        )));
    }

    if !path.is_dir() {
        return Err(crate::GuiError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path is not a directory: {}", path.display()),
        )));
    }

    Ok(())
}