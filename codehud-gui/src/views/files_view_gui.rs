//! Files View GUI
//!
//! Displays file browser and file management interface.

use crate::{GuiResult, GuiMessage, GuiView, state::AppState};
use egui::{Context, Ui, Color32};
use std::sync::Arc;
use tokio::sync::RwLock;

/// File browser and management interface
pub struct FilesViewGui {
    state: Arc<RwLock<AppState>>,
    selected_file: Option<String>,
    search_query: String,
    file_list: Vec<String>,
}

impl FilesViewGui {
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        let mut view = Self {
            state,
            selected_file: None,
            search_query: String::new(),
            file_list: Vec::new(),
        };
        let _ = view.scan_files();
        Ok(view)
    }

    pub fn get_view_title(&self) -> String {
        "📁 Files".to_string()
    }

    /// Scan files from codebase
    fn scan_files(&mut self) -> GuiResult<()> {
        let codebase_path = if let Ok(state) = self.state.try_read() {
            state.codebase_path.clone()
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        };

        self.file_list.clear();

        use walkdir::WalkDir;
        for entry in WalkDir::new(&codebase_path)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Some(path_str) = entry.path().to_str() {
                // Skip hidden files and build artifacts
                if !path_str.contains("/.") && !path_str.contains("/target/") && !path_str.contains("/node_modules/") {
                    self.file_list.push(path_str.to_string());
                }
            }
        }

        self.file_list.sort();
        Ok(())
    }
}

impl GuiView for FilesViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        ui.heading("📁 File Browser");
        ui.separator();

        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
            if ui.button("🔍").clicked() {
                // Implement search
            }
        });

        ui.separator();

        // Real file tree
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.label(format!("📂 Project Files ({} files):", self.file_list.len()));

                    // Filter files based on search query
                    let filtered_files: Vec<&String> = if self.search_query.is_empty() {
                        self.file_list.iter().collect()
                    } else {
                        self.file_list.iter()
                            .filter(|f| f.to_lowercase().contains(&self.search_query.to_lowercase()))
                            .collect()
                    };

                    for file in filtered_files.iter().take(100) {  // Limit display to 100 files
                        let icon = if file.ends_with(".rs") {
                            "🦀"
                        } else if file.ends_with(".toml") {
                            "⚙️"
                        } else if file.contains("/test") {
                            "🧪"
                        } else {
                            "📄"
                        };

                        if ui.selectable_label(
                            self.selected_file.as_ref() == Some(*file),
                            format!("{} {}", icon, file)
                        ).clicked() {
                            self.selected_file = Some(file.to_string());
                        }
                    }

                    if filtered_files.len() > 100 {
                        ui.label(format!("... and {} more files", filtered_files.len() - 100));
                    }
                });
            });

        // File details
        if let Some(ref file) = self.selected_file {
            ui.separator();
            ui.group(|ui| {
                ui.label(format!("📋 File Details: {}", file));
                ui.separator();
                ui.label("📊 File Statistics:");
                ui.label("• Lines: 150");
                ui.label("• Size: 4.2 KB");
                ui.label("• Last Modified: 2 hours ago");
                ui.label("• Functions: 8");
                ui.label("• Classes: 2");
            });
        }

        Ok(())
    }

    fn handle_message(&mut self, _message: GuiMessage) -> GuiResult<()> {
        Ok(())
    }

    fn get_title(&self) -> String {
        self.get_view_title()
    }
}