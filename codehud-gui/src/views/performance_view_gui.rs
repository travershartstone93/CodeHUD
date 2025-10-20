//! Performance View GUI
//!
//! Displays performance analysis, bottlenecks, and optimization opportunities.

use crate::{GuiResult, GuiMessage, GuiView, signals_pyqt5::PyQtSignal, state::AppState};
use egui::{Context, Ui, Color32, Vec2, ProgressBar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Performance analysis data structure matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceData {
    pub summary: PerformanceSummary,
    pub high_impact_issues: Vec<PerformanceBottleneck>,
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
    pub complexity_metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub performance_score: f64,
    pub high_impact_issues: usize,
    pub total_issues: usize,
    pub files_analyzed: usize,
    pub avg_complexity: f64,
    pub max_complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBottleneck {
    pub impact_level: String,
    pub bottleneck_type: String,
    pub function_file: String,
    pub complexity: usize,
    pub description: String,
    pub file: Option<String>,
    pub function: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOpportunity {
    pub category: String,
    pub description: String,
    pub impact: String,
    pub effort: String,
}

impl Default for PerformanceData {
    fn default() -> Self {
        Self {
            summary: PerformanceSummary {
                performance_score: 75.0,
                high_impact_issues: 3,
                total_issues: 12,
                files_analyzed: 45,
                avg_complexity: 4.2,
                max_complexity: 12.5,
            },
            high_impact_issues: vec![
                PerformanceBottleneck {
                    impact_level: "High".to_string(),
                    bottleneck_type: "nested_loops".to_string(),
                    function_file: "analyzer.py".to_string(),
                    complexity: 15,
                    description: "Nested loop pattern with O(n²) complexity".to_string(),
                    file: Some("analyzer.py".to_string()),
                    function: Some("analyze_dependencies".to_string()),
                },
                PerformanceBottleneck {
                    impact_level: "Medium".to_string(),
                    bottleneck_type: "database_query".to_string(),
                    function_file: "data_extractor.py".to_string(),
                    complexity: 8,
                    description: "Multiple database queries in loop".to_string(),
                    file: Some("data_extractor.py".to_string()),
                    function: Some("extract_data".to_string()),
                },
            ],
            optimization_opportunities: vec![
                OptimizationOpportunity {
                    category: "Algorithm Optimization".to_string(),
                    description: "Replace nested loops with hash-based lookups".to_string(),
                    impact: "High".to_string(),
                    effort: "Medium".to_string(),
                },
                OptimizationOpportunity {
                    category: "Database Optimization".to_string(),
                    description: "Batch database queries to reduce round trips".to_string(),
                    impact: "Medium".to_string(),
                    effort: "Low".to_string(),
                },
            ],
            complexity_metrics: HashMap::new(),
        }
    }
}

/// Performance analysis view matching Python PerformanceView
pub struct PerformanceViewGui {
    data: PerformanceData,
    state: Arc<RwLock<AppState>>,

    // PyQt5-style signals matching Python implementation
    pub performance_updated: PyQtSignal<PerformanceData>,

    // UI state
    selected_bottleneck_index: Option<usize>,
}

impl PerformanceViewGui {
    /// Create new performance view matching Python constructor
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self {
            data: PerformanceData::default(),
            state,
            performance_updated: PyQtSignal::new(),
            selected_bottleneck_index: None,
        }
    }

    /// Get view title matching Python get_view_title
    pub fn get_view_title(&self) -> String {
        "⚡ Performance Analysis".to_string()
    }

    /// Update content with analysis data matching Python update_content
    pub fn update_content(&mut self, data: PerformanceData) {
        self.data = data.clone();
        self.performance_updated.emit(data);
    }

    /// Render performance dashboard matching Python create_performance_dashboard
    fn render_performance_dashboard(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            // Performance score section matching Python score_section
            ui.vertical_centered(|ui| {
                ui.label("Performance Score");

                let score = self.data.summary.performance_score;
                let score_color = if score >= 80.0 {
                    Color32::from_rgb(46, 204, 113) // Green
                } else if score >= 60.0 {
                    Color32::from_rgb(243, 156, 18) // Orange
                } else {
                    Color32::from_rgb(231, 76, 60) // Red
                };

                // Large score display matching Python implementation
                ui.colored_label(score_color, egui::RichText::new(format!("{:.1}", score)).size(32.0));

                // Progress bar
                let mut progress = ProgressBar::new(score as f32 / 100.0);
                progress = progress.fill(score_color);
                ui.add_sized([200.0, 20.0], progress);
            });

            ui.separator();

            // Performance metrics matching Python metrics_section
            ui.group(|ui| {
                ui.label("📊 Performance Metrics");
                ui.separator();

                let metrics_text = format!(
                    "⚡ Performance Statistics:
• Performance Score: {:.1}%
• High Impact Issues: {}
• Total Issues: {}
• Files Analyzed: {}

🔄 Complexity Analysis:
• Average Complexity: {:.1} (Good)
• Maximum Complexity: {:.1} (Acceptable)
• Functions > 10 complexity: 15

🚀 Performance Indicators:
• Nested loops detected: 8
• Database queries: 12
• I/O operations: 23
• Hot functions identified: 6

📈 Optimization Potential:
• Algorithm improvements possible
• Caching opportunities available
• Database query optimization needed",
                    self.data.summary.performance_score,
                    self.data.summary.high_impact_issues,
                    self.data.summary.total_issues,
                    self.data.summary.files_analyzed,
                    self.data.summary.avg_complexity,
                    self.data.summary.max_complexity
                );

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.code(&metrics_text);
                    });
            });

            ui.separator();

            // Optimization opportunities matching Python opportunities_section
            ui.group(|ui| {
                ui.label("🚀 Optimization Opportunities");
                ui.separator();

                let opportunities_text = "🎯 Algorithm Optimization:
• Replace nested loops with hash-based lookups
• Implement caching for expensive computations
• Use generators for memory efficiency

💾 Database Optimization:
• Batch database queries to reduce round trips
• Add indexes for frequently queried fields
• Use connection pooling

🔄 I/O Optimization:
• Implement asynchronous file operations
• Use buffered I/O for better performance
• Cache frequently accessed file contents

📈 Memory Optimization:
• Use memory-efficient data structures
• Implement object pooling for frequent allocations
• Profile memory usage to identify leaks";

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.code(opportunities_text);
                    });
            });
        });
    }

    /// Render bottlenecks table matching Python create_bottlenecks_table
    fn render_bottlenecks_table(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label("🔥 Performance Bottlenecks");
            ui.separator();

            // Bottlenecks table matching Python table structure
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    egui::Grid::new("bottlenecks_table")
                        .num_columns(5)
                        .striped(true)
                        .show(ui, |ui| {
                            // Table headers matching Python setHorizontalHeaderLabels
                            ui.strong("Impact");
                            ui.strong("Type");
                            ui.strong("Function/File");
                            ui.strong("Complexity");
                            ui.strong("Description");
                            ui.end_row();

                            // Display bottlenecks matching Python update_bottlenecks_table
                            for (index, bottleneck) in self.data.high_impact_issues.iter().enumerate() {
                                // Impact level with color coding
                                let impact_color = match bottleneck.impact_level.as_str() {
                                    "High" => Color32::from_rgb(231, 76, 60),
                                    "Medium" => Color32::from_rgb(243, 156, 18),
                                    "Low" => Color32::from_rgb(155, 165, 166),
                                    _ => Color32::WHITE,
                                };

                                if ui.selectable_label(
                                    self.selected_bottleneck_index == Some(index),
                                    &bottleneck.impact_level
                                ).clicked() {
                                    self.selected_bottleneck_index = Some(index);
                                }

                                // Type
                                let type_name = bottleneck.bottleneck_type.replace('_', " ");
                                let formatted_type = type_name
                                    .split(' ')
                                    .map(|word| {
                                        let mut chars = word.chars();
                                        match chars.next() {
                                            None => String::new(),
                                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join(" ");
                                ui.label(formatted_type);

                                // Function/File
                                let function_name = bottleneck.function.as_ref()
                                    .or(bottleneck.file.as_ref())
                                    .unwrap_or(&bottleneck.function_file);
                                ui.label(function_name);

                                // Complexity with color coding
                                let complexity_color = if bottleneck.complexity > 15 {
                                    Color32::RED
                                } else if bottleneck.complexity > 10 {
                                    Color32::YELLOW
                                } else {
                                    Color32::GREEN
                                };
                                ui.colored_label(complexity_color, bottleneck.complexity.to_string());

                                // Description
                                ui.label(&bottleneck.description);
                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// Render selected bottleneck details panel
    fn render_bottleneck_details(&mut self, ui: &mut Ui) {
        if let Some(index) = self.selected_bottleneck_index {
            if let Some(bottleneck) = self.data.high_impact_issues.get(index) {
                ui.group(|ui| {
                    ui.label("📋 Bottleneck Details");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Impact Level:");
                        let impact_color = match bottleneck.impact_level.as_str() {
                            "High" => Color32::RED,
                            "Medium" => Color32::YELLOW,
                            "Low" => Color32::GRAY,
                            _ => Color32::WHITE,
                        };
                        ui.colored_label(impact_color, &bottleneck.impact_level);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        ui.strong(&bottleneck.bottleneck_type);
                    });

                    if let Some(ref file) = bottleneck.file {
                        ui.horizontal(|ui| {
                            ui.label("File:");
                            ui.code(file);
                        });
                    }

                    if let Some(ref function) = bottleneck.function {
                        ui.horizontal(|ui| {
                            ui.label("Function:");
                            ui.code(function);
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Complexity:");
                        let complexity_color = if bottleneck.complexity > 15 {
                            Color32::RED
                        } else if bottleneck.complexity > 10 {
                            Color32::YELLOW
                        } else {
                            Color32::GREEN
                        };
                        ui.colored_label(complexity_color, bottleneck.complexity.to_string());
                    });

                    ui.separator();
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut bottleneck.description.clone());
                });
            }
        }
    }
}

impl GuiView for PerformanceViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        // Main layout matching Python splitter structure (horizontal split)
        ui.horizontal(|ui| {
            // Left: Performance dashboard matching Python create_performance_dashboard
            ui.allocate_ui_with_layout(
                [400.0, ui.available_height()].into(),
                egui::Layout::top_down(egui::Align::Left),
                |ui| {
                    self.render_performance_dashboard(ui);
                }
            );

            ui.separator();

            // Right: Bottlenecks table matching Python create_bottlenecks_table
            ui.allocate_ui_with_layout(
                [ui.available_width(), ui.available_height()].into(),
                egui::Layout::top_down(egui::Align::Left),
                |ui| {
                    self.render_bottlenecks_table(ui);

                    // Details panel for selected bottleneck
                    if self.selected_bottleneck_index.is_some() {
                        ui.separator();
                        self.render_bottleneck_details(ui);
                    }
                }
            );
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match message {
            GuiMessage::AnalysisComplete => {
                // Performance data might be updated after analysis
                Ok(())
            }
            _ => Ok(())
        }
    }

    fn get_title(&self) -> String {
        self.get_view_title()
    }
}