use crate::{GuiComponent, GuiMessage, GuiResult, signals::SignalBus, state::AppState};
use egui::{Context, Ui, ScrollArea, CollapsingHeader};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ProjectExplorerComponent {
    state: Arc<RwLock<AppState>>,
    signal_bus: Arc<SignalBus>,
    visible: bool,
    enabled: bool,
    expanded_folders: HashMap<String, bool>,
    selected_file: Option<PathBuf>,
    filter_text: String,
}

impl ProjectExplorerComponent {
    pub fn new(state: Arc<RwLock<AppState>>, signal_bus: Arc<SignalBus>) -> GuiResult<Self> {
        Ok(Self {
            state,
            signal_bus,
            visible: true,
            enabled: true,
            expanded_folders: HashMap::new(),
            selected_file: None,
            filter_text: String::new(),
        })
    }

    fn render_file_tree(&mut self, ui: &mut Ui, files: &[PathBuf], base_path: &PathBuf) -> GuiResult<()> {
        let mut folders: HashMap<String, Vec<PathBuf>> = HashMap::new();
        let mut direct_files = Vec::new();

        // Group files by directory
        for file in files {
            if let Ok(relative) = file.strip_prefix(base_path) {
                if let Some(parent) = relative.parent() {
                    if parent != std::path::Path::new("") {
                        let parent_str = parent.to_string_lossy().to_string();
                        folders.entry(parent_str).or_insert_with(Vec::new).push(file.clone());
                    } else {
                        direct_files.push(file.clone());
                    }
                } else {
                    direct_files.push(file.clone());
                }
            }
        }

        // Render direct files first
        for file in &direct_files {
            if let Some(file_name) = file.file_name().and_then(|n| n.to_str()) {
                if self.filter_text.is_empty() || file_name.to_lowercase().contains(&self.filter_text.to_lowercase()) {
                    let is_selected = self.selected_file.as_ref() == Some(file);

                    if ui.selectable_label(is_selected, file_name).clicked() {
                        self.selected_file = Some(file.clone());
                        let _ = self.signal_bus.emit("file_selected",
                            GuiMessage::ProjectLoaded(file.to_string_lossy().to_string()));
                    }
                }
            }
        }

        // Render folders
        for (folder_name, folder_files) in folders {
            let is_expanded = self.expanded_folders.get(&folder_name).copied().unwrap_or(false);

            CollapsingHeader::new(&folder_name)
                .default_open(false)
                .open(Some(is_expanded))
                .show(ui, |ui| {
                    self.render_file_tree(ui, &folder_files, base_path)?;
                    Ok::<(), crate::GuiError>(())
                });
        }

        Ok(())
    }

    fn render_project_info(&mut self, ui: &mut Ui, project: &crate::state::ProjectState) -> GuiResult<()> {
        ui.heading(&project.name);

        ui.horizontal(|ui| {
            ui.label("Language:");
            ui.label(&project.language);
        });

        ui.horizontal(|ui| {
            ui.label("Files:");
            ui.label(format!("{}", project.files.len()));
        });

        if let Some(last_analysis) = &project.last_analysis {
            ui.horizontal(|ui| {
                ui.label("Last Analysis:");
                ui.label(last_analysis.format("%Y-%m-%d %H:%M").to_string());
            });
        }

        ui.separator();
        Ok(())
    }
}

impl GuiComponent for ProjectExplorerComponent {
    fn name(&self) -> &str {
        "project_explorer"
    }

    fn render(&mut self, ui: &mut Ui, ctx: &Context) -> GuiResult<()> {
        if !self.visible {
            return Ok(());
        }

        ui.vertical(|ui| {
            ui.heading("Project Explorer");

            // Filter input
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.filter_text);
                if ui.button("Clear").clicked() {
                    self.filter_text.clear();
                }
            });

            ui.separator();

            // Project actions
            ui.horizontal(|ui| {
                if ui.button("Open Project").clicked() {
                    let _ = self.signal_bus.emit("open_project_dialog",
                        GuiMessage::ProjectLoaded("".to_string()));
                }

                if ui.button("Refresh").clicked() {
                    let _ = self.signal_bus.emit("refresh_project",
                        GuiMessage::ProjectLoaded("".to_string()));
                }

                if ui.button("Analyze").clicked() {
                    let _ = self.signal_bus.emit("analyze_project",
                        GuiMessage::AnalysisComplete);
                }
            });

            ui.separator();

            // Project content
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Try to read state without blocking
                    let project_opt = if let Ok(state_guard) = self.state.try_read() {
                        state_guard.current_project.clone()
                    } else {
                        None
                    };

                    if let Some(project) = project_opt {
                        // Render project info
                        if let Err(e) = self.render_project_info(ui, &project) {
                            ui.colored_label(egui::Color32::RED, format!("Error rendering project info: {}", e));
                        }

                        // Render file tree
                        if let Err(e) = self.render_file_tree(ui, &project.files, &project.path) {
                            ui.colored_label(egui::Color32::RED, format!("Error rendering file tree: {}", e));
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.label("No project loaded");
                            ui.label("Use 'Open Project' to get started");
                        });
                    }
                });
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match &message {
            GuiMessage::ProjectLoaded(path) => {
                if !path.is_empty() {
                    log::info!("Project loaded: {}", path);
                    self.expanded_folders.clear();
                    self.selected_file = None;
                }
            },
            GuiMessage::AnalysisComplete => {
                log::info!("Analysis complete, refreshing project view");
            },
            _ => {}
        }
        Ok(())
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}