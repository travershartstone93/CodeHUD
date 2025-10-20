//! Dependencies View GUI
//!
//! Displays module dependencies and coupling analysis with proper GUI components.

use crate::{GuiResult, GuiMessage, GuiView, signals_pyqt5::PyQtSignal, state::AppState};
use egui::{Context, Ui, Color32, Vec2, ProgressBar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Dependencies analysis data structure matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependenciesData {
    pub file_dependencies: HashMap<String, FileDependency>,
    pub summary: DependencySummary,
    pub coupling_distribution: CouplingDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDependency {
    pub total_imports: usize,
    pub coupling_score: f64,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySummary {
    pub total_import_statements: usize,
    pub average_coupling: f64,
    pub high_coupling_modules: usize,
    pub total_modules: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingDistribution {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
}

impl Default for DependenciesData {
    fn default() -> Self {
        Self {
            file_dependencies: HashMap::new(),
            summary: DependencySummary {
                total_import_statements: 0,
                average_coupling: 0.0,
                high_coupling_modules: 0,
                total_modules: 0,
            },
            coupling_distribution: CouplingDistribution {
                low: 0,
                medium: 0,
                high: 0,
            },
        }
    }
}

/// Dependencies and coupling analysis view matching Python DependenciesView
pub struct DependenciesViewGui {
    data: DependenciesData,
    state: Arc<RwLock<AppState>>,

    // PyQt5-style signals matching Python implementation
    pub dependencies_updated: PyQtSignal<DependenciesData>,

    // UI state
    selected_module: Option<String>,
    sort_by_coupling: bool,
}

impl DependenciesViewGui {
    /// Create new dependencies view matching Python constructor
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        Ok(Self {
            data: DependenciesData::default(),
            state,
            dependencies_updated: PyQtSignal::new(),
            selected_module: None,
            sort_by_coupling: true,
        })
    }

    /// Fetch dependencies data from backend
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
                    "--view", "dependencies",
                    "--output", "/tmp/dependencies_analysis.json"
                ])
                .current_dir(std::env::current_dir().unwrap())
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        println!("✅ Dependencies analysis succeeded");
                    } else {
                        eprintln!("❌ Dependencies analysis failed");
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to spawn dependencies analysis: {}", e);
                }
            }
        });

        // Try to load existing data
        if let Ok(json_str) = std::fs::read_to_string("/tmp/dependencies_analysis.json") {
            if let Ok(data) = serde_json::from_str::<DependenciesData>(&json_str) {
                self.update_content(data);
            }
        }

        Ok(())
    }

    /// Get view title matching Python get_view_title
    pub fn get_view_title(&self) -> String {
        "🔗 Module Dependencies".to_string()
    }

    /// Update content with analysis data matching Python update_content
    pub fn update_content(&mut self, data: DependenciesData) {
        self.data = data.clone();
        self.dependencies_updated.emit(data);
    }

    /// Render overview metrics section matching Python create_overview_section
    fn render_overview_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label("📊 Dependencies Overview");
            ui.separator();

            ui.horizontal(|ui| {
                // Left: Summary metrics matching Python metrics_widget
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Total Dependencies:");
                        ui.strong(self.data.summary.total_import_statements.to_string());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Average Coupling:");
                        let avg_coupling = self.data.summary.average_coupling;
                        let color = if avg_coupling > 0.7 {
                            Color32::RED
                        } else if avg_coupling > 0.3 {
                            Color32::YELLOW
                        } else {
                            Color32::GREEN
                        };
                        ui.colored_label(color, format!("{:.2}", avg_coupling));
                    });

                    ui.horizontal(|ui| {
                        ui.label("High Coupling Modules:");
                        ui.colored_label(
                            Color32::RED,
                            self.data.summary.high_coupling_modules.to_string()
                        );
                    });
                });

                ui.separator();

                // Right: Coupling distribution matching Python dist_widget
                ui.vertical(|ui| {
                    ui.label("Coupling Distribution:");

                    let total = self.data.coupling_distribution.low +
                              self.data.coupling_distribution.medium +
                              self.data.coupling_distribution.high;

                    if total > 0 {
                        // Low coupling bar
                        ui.horizontal(|ui| {
                            ui.label("Low:");
                            let low_pct = self.data.coupling_distribution.low as f32 / total as f32;
                            let mut progress = ProgressBar::new(low_pct);
                            progress = progress.fill(Color32::GREEN);
                            ui.add_sized([150.0, 20.0], progress);
                            ui.label(format!("{:.0}%", low_pct * 100.0));
                        });

                        // Medium coupling bar
                        ui.horizontal(|ui| {
                            ui.label("Medium:");
                            let med_pct = self.data.coupling_distribution.medium as f32 / total as f32;
                            let mut progress = ProgressBar::new(med_pct);
                            progress = progress.fill(Color32::YELLOW);
                            ui.add_sized([150.0, 20.0], progress);
                            ui.label(format!("{:.0}%", med_pct * 100.0));
                        });

                        // High coupling bar
                        ui.horizontal(|ui| {
                            ui.label("High:");
                            let high_pct = self.data.coupling_distribution.high as f32 / total as f32;
                            let mut progress = ProgressBar::new(high_pct);
                            progress = progress.fill(Color32::RED);
                            ui.add_sized([150.0, 20.0], progress);
                            ui.label(format!("{:.0}%", high_pct * 100.0));
                        });
                    } else {
                        ui.label("No data available");
                    }
                });
            });
        });
    }

    /// Render dependencies table matching Python create_dependencies_table
    fn render_dependencies_table(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("📋 Module Dependencies");

                ui.separator();

                // Sort toggle
                if ui.selectable_label(self.sort_by_coupling, "Sort by Coupling").clicked() {
                    self.sort_by_coupling = !self.sort_by_coupling;
                }
            });

            ui.separator();

            // Dependencies table matching Python table structure
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    egui::Grid::new("dependencies_table")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            // Table headers matching Python setHorizontalHeaderLabels
                            ui.strong("Module");
                            ui.strong("Dependencies");
                            ui.strong("Coupling Score");
                            ui.strong("Risk Level");
                            ui.end_row();

                            // Sort dependencies by coupling score if enabled
                            let mut deps: Vec<(String, FileDependency)> = self.data.file_dependencies
                                .clone()
                                .into_iter()
                                .collect();

                            if self.sort_by_coupling {
                                deps.sort_by(|a, b| b.1.coupling_score.partial_cmp(&a.1.coupling_score).unwrap());
                            } else {
                                deps.sort_by(|a, b| a.0.cmp(&b.0));
                            }

                            // Display table rows matching Python update_dependencies_table
                            for (module_path, dep) in deps {
                                // Module name (clickable)
                                if ui.selectable_label(
                                    self.selected_module.as_ref() == Some(&module_path),
                                    &module_path
                                ).clicked() {
                                    self.selected_module = Some(module_path.clone());
                                }

                                // Dependencies count
                                ui.label(dep.total_imports.to_string());

                                // Coupling score with color coding
                                let coupling_color = if dep.coupling_score > 0.7 {
                                    Color32::RED
                                } else if dep.coupling_score > 0.3 {
                                    Color32::YELLOW
                                } else {
                                    Color32::GREEN
                                };
                                ui.colored_label(coupling_color, format!("{:.2}", dep.coupling_score));

                                // Risk level with background color matching Python implementation
                                let (risk_color, risk_text) = match dep.risk_level.as_str() {
                                    "High" => (Color32::from_rgb(255, 200, 200), "High"),
                                    "Medium" => (Color32::from_rgb(255, 255, 200), "Medium"),
                                    "Low" => (Color32::from_rgb(200, 255, 200), "Low"),
                                    _ => (Color32::WHITE, dep.risk_level.as_str()),
                                };

                                // Create colored background label
                                ui.scope(|ui| {
                                    ui.style_mut().visuals.extreme_bg_color = risk_color;
                                    ui.label(risk_text);
                                });

                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// Render selected module details panel
    fn render_details_panel(&mut self, ui: &mut Ui) {
        if let Some(ref selected) = self.selected_module.clone() {
            if let Some(dep) = self.data.file_dependencies.get(selected) {
                ui.group(|ui| {
                    ui.label(format!("📋 Module Details: {}", selected));
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Total Imports:");
                        ui.strong(dep.total_imports.to_string());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Coupling Score:");
                        let coupling_color = if dep.coupling_score > 0.7 {
                            Color32::RED
                        } else if dep.coupling_score > 0.3 {
                            Color32::YELLOW
                        } else {
                            Color32::GREEN
                        };
                        ui.colored_label(coupling_color, format!("{:.2}", dep.coupling_score));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Risk Level:");
                        let risk_color = match dep.risk_level.as_str() {
                            "High" => Color32::RED,
                            "Medium" => Color32::YELLOW,
                            "Low" => Color32::GREEN,
                            _ => Color32::WHITE,
                        };
                        ui.colored_label(risk_color, &dep.risk_level);
                    });

                    if !dep.dependencies.is_empty() {
                        ui.separator();
                        ui.label("Dependencies:");
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                for dependency in &dep.dependencies {
                                    ui.label(format!("• {}", dependency));
                                }
                            });
                    }

                    if !dep.dependents.is_empty() {
                        ui.separator();
                        ui.label("Dependents:");
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                for dependent in &dep.dependents {
                                    ui.label(format!("• {}", dependent));
                                }
                            });
                    }
                });
            }
        }
    }
}

impl GuiView for DependenciesViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        // Refresh button
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Analysis").clicked() {
                let _ = self.fetch_data();
            }
        });

        ui.separator();

        // Main layout matching Python splitter structure (vertical split)
        ui.vertical(|ui| {
            // Top: Overview metrics matching Python create_overview_section
            self.render_overview_section(ui);

            ui.separator();

            // Bottom: Dependencies table matching Python create_dependencies_table
            self.render_dependencies_table(ui);

            // Details panel for selected module
            if self.selected_module.is_some() {
                ui.separator();
                self.render_details_panel(ui);
            }
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match message {
            GuiMessage::TopologyUpdate => {
                // Dependencies data might be updated with topology
                Ok(())
            },
            _ => Ok(())
        }
    }

    fn get_title(&self) -> String {
        self.get_view_title()
    }
}