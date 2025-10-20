//! Quality View GUI
//!
//! Displays code quality metrics, maintainability index, and technical debt analysis.

use crate::{GuiResult, GuiMessage, GuiView, signals_pyqt5::PyQtSignal, state::AppState};
use egui::{Context, Ui, Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Quality metrics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityData {
    pub maintainability_index: f64,
    pub technical_debt: f64,
    pub code_smells: Vec<CodeSmell>,
    pub quality_score: f64,
    pub complexity_metrics: HashMap<String, f64>,
    pub duplication_percentage: f64,
    pub test_coverage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSmell {
    pub smell_type: String,
    pub file_path: String,
    pub line: usize,
    pub severity: String,
    pub description: String,
}

impl Default for QualityData {
    fn default() -> Self {
        Self {
            maintainability_index: 75.0,
            technical_debt: 12.5,
            code_smells: vec![],
            quality_score: 85.0,
            complexity_metrics: HashMap::new(),
            duplication_percentage: 8.2,
            test_coverage: 72.4,
        }
    }
}

/// Quality Analysis GUI View matching Python QualityView implementation
pub struct QualityViewGui {
    data: QualityData,
    state: Arc<RwLock<AppState>>,

    // PyQt5-style signals
    pub quality_updated: PyQtSignal<QualityData>,
    pub smell_selected: PyQtSignal<CodeSmell>,

    // UI state
    selected_smell_index: Option<usize>,
    show_details_panel: bool,
    filter_by_severity: String,
}

impl QualityViewGui {
    /// Create new quality view matching Python constructor
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        Ok(Self {
            data: QualityData::default(),
            state,
            quality_updated: PyQtSignal::new(),
            smell_selected: PyQtSignal::new(),
            selected_smell_index: None,
            show_details_panel: true,
            filter_by_severity: "All".to_string(),
        })
    }

    /// Fetch quality data from backend
    pub fn fetch_data(&mut self) -> GuiResult<()> {
        let state_clone = self.state.clone();

        tokio::spawn(async move {
            let codebase_path = if let Ok(state) = state_clone.read().await {
                state.codebase_path.clone()
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            };

            let result = std::process::Command::new("cargo")
                .args(&[
                    "run", "--bin", "codehud", "--",
                    "analyze",
                    codebase_path.to_str().unwrap(),
                    "--view", "quality",
                    "--output", "/tmp/quality_analysis.json"
                ])
                .current_dir(std::env::current_dir().unwrap())
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        println!("✅ Quality analysis succeeded");
                    } else {
                        eprintln!("❌ Quality analysis failed");
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to spawn quality analysis: {}", e);
                }
            }
        });

        // Try to load existing data
        if let Ok(json_str) = std::fs::read_to_string("/tmp/quality_analysis.json") {
            if let Ok(data) = serde_json::from_str::<QualityData>(&json_str) {
                self.update_content(data);
            }
        }

        Ok(())
    }

    /// Get view title matching Python implementation
    pub fn get_view_title(&self) -> String {
        "📊 Code Quality".to_string()
    }

    /// Update content with analysis data (matching Python update_content)
    pub fn update_content(&mut self, data: QualityData) {
        self.data = data.clone();
        self.quality_updated.emit(data);
    }

    /// Render quality overview section (matching Python UI structure)
    fn render_overview_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label("📊 Quality Overview");
            ui.separator();

            // Quality score display
            ui.horizontal(|ui| {
                ui.label("Overall Quality Score:");
                let score = self.data.quality_score;
                let color = if score >= 80.0 {
                    Color32::GREEN
                } else if score >= 60.0 {
                    Color32::YELLOW
                } else {
                    Color32::RED
                };
                ui.colored_label(color, format!("{:.1}%", score));
            });

            // Maintainability index
            ui.horizontal(|ui| {
                ui.label("Maintainability Index:");
                ui.colored_label(
                    Color32::from_rgb(0, 150, 0),
                    format!("{:.1}", self.data.maintainability_index)
                );
            });

            // Technical debt
            ui.horizontal(|ui| {
                ui.label("Technical Debt:");
                ui.colored_label(
                    Color32::from_rgb(200, 100, 0),
                    format!("{:.1} hours", self.data.technical_debt)
                );
            });

            // Test coverage
            ui.horizontal(|ui| {
                ui.label("Test Coverage:");
                let coverage_color = if self.data.test_coverage >= 80.0 {
                    Color32::GREEN
                } else if self.data.test_coverage >= 60.0 {
                    Color32::YELLOW
                } else {
                    Color32::RED
                };
                ui.colored_label(coverage_color, format!("{:.1}%", self.data.test_coverage));
            });

            // Code duplication
            ui.horizontal(|ui| {
                ui.label("Code Duplication:");
                ui.colored_label(
                    Color32::from_rgb(150, 150, 0),
                    format!("{:.1}%", self.data.duplication_percentage)
                );
            });
        });
    }

    /// Render code smells table (matching Python implementation)
    fn render_code_smells_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("🔍 Code Smells");
                ui.separator();

                // Severity filter
                ui.label("Filter:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.filter_by_severity)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_by_severity, "All".to_string(), "All");
                        ui.selectable_value(&mut self.filter_by_severity, "High".to_string(), "High");
                        ui.selectable_value(&mut self.filter_by_severity, "Medium".to_string(), "Medium");
                        ui.selectable_value(&mut self.filter_by_severity, "Low".to_string(), "Low");
                    });
            });

            ui.separator();

            // Code smells table
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    egui::Grid::new("code_smells_table")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            // Table headers
                            ui.strong("Type");
                            ui.strong("File");
                            ui.strong("Line");
                            ui.strong("Severity");
                            ui.end_row();

                            // Filter and display smells
                            let filtered_smells: Vec<(usize, &CodeSmell)> = self.data.code_smells
                                .iter()
                                .enumerate()
                                .filter(|(_, smell)| {
                                    self.filter_by_severity == "All" ||
                                    smell.severity == self.filter_by_severity
                                })
                                .collect();

                            for (index, smell) in filtered_smells {
                                if ui.selectable_label(
                                    self.selected_smell_index == Some(index),
                                    &smell.smell_type
                                ).clicked() {
                                    self.selected_smell_index = Some(index);
                                    self.smell_selected.emit(smell.clone());
                                }

                                ui.label(&smell.file_path);
                                ui.label(smell.line.to_string());

                                let severity_color = match smell.severity.as_str() {
                                    "High" => Color32::RED,
                                    "Medium" => Color32::YELLOW,
                                    "Low" => Color32::GRAY,
                                    _ => Color32::WHITE,
                                };
                                ui.colored_label(severity_color, &smell.severity);
                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// Render details panel (matching Python implementation)
    fn render_details_panel(&mut self, ui: &mut Ui) {
        if !self.show_details_panel {
            return;
        }

        ui.group(|ui| {
            ui.label("📋 Smell Details");
            ui.separator();

            if let Some(index) = self.selected_smell_index {
                if let Some(smell) = self.data.code_smells.get(index) {
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        ui.strong(&smell.smell_type);
                    });

                    ui.horizontal(|ui| {
                        ui.label("File:");
                        ui.code(&smell.file_path);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Line:");
                        ui.label(smell.line.to_string());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Severity:");
                        let severity_color = match smell.severity.as_str() {
                            "High" => Color32::RED,
                            "Medium" => Color32::YELLOW,
                            "Low" => Color32::GRAY,
                            _ => Color32::WHITE,
                        };
                        ui.colored_label(severity_color, &smell.severity);
                    });

                    ui.separator();
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut smell.description.clone());
                }
            } else {
                ui.label("Select a code smell to view details");
            }
        });
    }
}

impl GuiView for QualityViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        // Refresh button
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Analysis").clicked() {
                let _ = self.fetch_data();
            }
        });

        ui.separator();

        // Main layout matching Python splitter structure
        ui.vertical(|ui| {
            // Top section: Quality overview
            self.render_overview_section(ui);

            ui.separator();

            // Middle section: Code smells table
            self.render_code_smells_section(ui);

            ui.separator();

            // Bottom section: Details panel
            if self.show_details_panel {
                self.render_details_panel(ui);
            }

            // Toggle details panel button
            ui.horizontal(|ui| {
                if ui.button(if self.show_details_panel { "Hide Details" } else { "Show Details" }).clicked() {
                    self.show_details_panel = !self.show_details_panel;
                }
            });
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match message {
            GuiMessage::QualityUpdate => {
                // Refresh quality data from state
                // This would typically fetch new data from the analysis engine
                Ok(())
            }
            _ => Ok(())
        }
    }

    fn get_title(&self) -> String {
        self.get_view_title()
    }
}