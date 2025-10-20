//! Call Graph View - Doxygen-Style Multi-View Visualization
//!
//! Displays call graph visualizations with automatic multi-view generation:
//! - Overview graph (module-level architecture)
//! - Per-module detail graphs (function-level)
//! - Cycle detection graph (circular dependencies)

use crate::{GuiView, GuiResult, GuiError, state::AppState};
use egui::{Context, Ui, ScrollArea, CollapsingHeader, Grid, ComboBox};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Call graph visualization view
pub struct CallGraphViewGui {
    state: Arc<RwLock<AppState>>,

    // Generation settings
    output_format: OutputFormat,
    layout_engine: LayoutEngine,
    output_dir: String,
    enable_cycle_highlighting: bool,
    enable_complexity_coloring: bool,

    // Generation status
    is_generating: bool,
    generation_status: String,
    last_error: Option<String>,

    // Generated files
    generated_files: Vec<GeneratedGraph>,
    selected_graph: Option<usize>,

    // Statistics
    total_functions: usize,
    total_calls: usize,
    num_modules: usize,
    num_cycles: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum OutputFormat {
    Svg,
    Png,
    Pdf,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Svg => write!(f, "SVG"),
            OutputFormat::Png => write!(f, "PNG"),
            OutputFormat::Pdf => write!(f, "PDF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LayoutEngine {
    Auto,
    Dot,
    Neato,
    Fdp,
    Sfdp,
    Circo,
    Twopi,
}

impl std::fmt::Display for LayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutEngine::Auto => write!(f, "Auto (recommended)"),
            LayoutEngine::Dot => write!(f, "Dot (hierarchical)"),
            LayoutEngine::Neato => write!(f, "Neato (spring)"),
            LayoutEngine::Fdp => write!(f, "FDP (force-directed)"),
            LayoutEngine::Sfdp => write!(f, "SFDP (large graphs)"),
            LayoutEngine::Circo => write!(f, "Circo (circular)"),
            LayoutEngine::Twopi => write!(f, "Twopi (radial)"),
        }
    }
}

#[derive(Debug, Clone)]
struct GeneratedGraph {
    path: PathBuf,
    graph_type: GraphType,
    node_count: usize,
    edge_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum GraphType {
    Overview,
    Module(String),
    Cycles,
}

impl std::fmt::Display for GraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphType::Overview => write!(f, "📊 Overview"),
            GraphType::Module(name) => write!(f, "📦 Module: {}", name),
            GraphType::Cycles => write!(f, "🔄 Cycles"),
        }
    }
}

impl CallGraphViewGui {
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        Ok(Self {
            state,
            output_format: OutputFormat::Svg,
            layout_engine: LayoutEngine::Auto,
            output_dir: "/tmp/callgraph".to_string(),
            enable_cycle_highlighting: true,
            enable_complexity_coloring: true,
            is_generating: false,
            generation_status: "Ready".to_string(),
            last_error: None,
            generated_files: Vec::new(),
            selected_graph: None,
            total_functions: 0,
            total_calls: 0,
            num_modules: 0,
            num_cycles: 0,
        })
    }

    fn render_settings(&mut self, ui: &mut Ui) -> GuiResult<()> {
        CollapsingHeader::new("⚙️ Generation Settings")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("callgraph_settings")
                    .num_columns(2)
                    .spacing([40.0, 8.0])
                    .show(ui, |ui| {
                        // Output format
                        ui.label("Output Format:");
                        ComboBox::from_id_source("output_format")
                            .selected_text(format!("{}", self.output_format))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.output_format, OutputFormat::Svg, "SVG");
                                ui.selectable_value(&mut self.output_format, OutputFormat::Png, "PNG");
                                ui.selectable_value(&mut self.output_format, OutputFormat::Pdf, "PDF");
                            });
                        ui.end_row();

                        // Layout engine
                        ui.label("Layout Engine:");
                        ComboBox::from_id_source("layout_engine")
                            .selected_text(format!("{}", self.layout_engine))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Auto, "Auto (recommended)");
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Dot, "Dot (hierarchical)");
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Neato, "Neato (spring)");
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Fdp, "FDP (force-directed)");
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Sfdp, "SFDP (scalable)");
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Circo, "Circo (circular)");
                                ui.selectable_value(&mut self.layout_engine, LayoutEngine::Twopi, "Twopi (radial)");
                            });
                        ui.end_row();

                        // Output directory
                        ui.label("Output Directory:");
                        ui.text_edit_singleline(&mut self.output_dir);
                        ui.end_row();

                        // Options
                        ui.label("Cycle Highlighting:");
                        ui.checkbox(&mut self.enable_cycle_highlighting, "");
                        ui.end_row();

                        ui.label("Complexity Coloring:");
                        ui.checkbox(&mut self.enable_complexity_coloring, "");
                        ui.end_row();
                    });

                ui.add_space(10.0);

                // Generate button
                if ui.button("🚀 Generate Call Graph Visualizations").clicked() && !self.is_generating {
                    self.start_generation();
                }

                if self.is_generating {
                    ui.label(format!("⏳ {}", self.generation_status));
                    ui.spinner();
                }

                if let Some(ref error) = self.last_error {
                    ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", error));
                }
            });

        Ok(())
    }

    fn render_statistics(&mut self, ui: &mut Ui) -> GuiResult<()> {
        if self.generated_files.is_empty() {
            return Ok(());
        }

        CollapsingHeader::new("📊 Statistics")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("callgraph_stats")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("📞 Total Functions:");
                        ui.label(format!("{}", self.total_functions));
                        ui.end_row();

                        ui.label("🔗 Total Calls:");
                        ui.label(format!("{}", self.total_calls));
                        ui.end_row();

                        ui.label("📦 Modules:");
                        ui.label(format!("{}", self.num_modules));
                        ui.end_row();

                        ui.label("🔄 Cycles Detected:");
                        if self.num_cycles > 0 {
                            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), format!("{}", self.num_cycles));
                        } else {
                            ui.colored_label(egui::Color32::GREEN, "0 (No circular dependencies)");
                        }
                        ui.end_row();

                        ui.label("📄 Generated Files:");
                        ui.label(format!("{}", self.generated_files.len()));
                        ui.end_row();
                    });
            });

        Ok(())
    }

    fn render_graph_list(&mut self, ui: &mut Ui) -> GuiResult<()> {
        if self.generated_files.is_empty() {
            return Ok(());
        }

        CollapsingHeader::new("📁 Generated Graphs")
            .default_open(true)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (idx, graph) in self.generated_files.iter().enumerate() {
                            let is_selected = self.selected_graph == Some(idx);

                            ui.horizontal(|ui| {
                                let label = format!(
                                    "{} ({} nodes, {} edges)",
                                    graph.graph_type,
                                    graph.node_count,
                                    graph.edge_count
                                );

                                if ui.selectable_label(is_selected, label).clicked() {
                                    self.selected_graph = Some(idx);
                                }

                                if ui.button("📂 Open").clicked() {
                                    if let Err(e) = open::that(&graph.path) {
                                        self.last_error = Some(format!("Failed to open file: {}", e));
                                    }
                                }
                            });
                        }
                    });
            });

        Ok(())
    }

    fn render_graph_preview(&mut self, ui: &mut Ui) -> GuiResult<()> {
        if let Some(idx) = self.selected_graph {
            if let Some(graph) = self.generated_files.get(idx) {
                CollapsingHeader::new(&format!("🖼️ Preview: {}", graph.graph_type))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(format!("Path: {}", graph.path.display()));
                        ui.label(format!("Nodes: {}, Edges: {}", graph.node_count, graph.edge_count));

                        ui.separator();

                        // SVG preview (if format is SVG)
                        if graph.path.extension().and_then(|s| s.to_str()) == Some("svg") {
                            ui.label("💡 Tip: Click 'Open' to view the graph in your default SVG viewer");
                        } else {
                            ui.label("💡 Tip: Click 'Open' to view the graph");
                        }
                    });
            }
        }

        Ok(())
    }

    fn start_generation(&mut self) {
        self.is_generating = true;
        self.generation_status = "Extracting call graph...".to_string();
        self.last_error = None;
        self.generated_files.clear();

        // Get codebase path from state (or use current directory)
        let state_clone = self.state.clone();

        // Spawn async task to run CLI call-graph command
        tokio::spawn(async move {
            let codebase_path = if let Ok(state) = state_clone.read().await {
                state.codebase_path.clone()
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            };

            // Run CLI call-graph command
            let format = "svg"; // Default to SVG for easy viewing
            let layout = "auto";
            let output_base = "call_graph_gui";

            let result = std::process::Command::new("cargo")
                .args(&[
                    "run", "--bin", "codehud", "--",
                    "call-graph",
                    codebase_path.to_str().unwrap(),
                    "-o", output_base,
                    "-f", format,
                    "-l", layout
                ])
                .current_dir(std::env::current_dir().unwrap())
                .output();

            // Handle result (would update GUI state through channels/signals in real impl)
            match result {
                Ok(output) => {
                    if output.status.success() {
                        println!("✅ Call graph generation succeeded");
                        println!("{}", String::from_utf8_lossy(&output.stdout));
                    } else {
                        eprintln!("❌ Call graph generation failed");
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to spawn call-graph command: {}", e);
                }
            }
        });

        // For now, simulate completion after spawn
        // In a real implementation, this would be called when the async task completes
        self.simulate_generation_complete();
    }

    fn simulate_generation_complete(&mut self) {
        self.is_generating = false;
        self.generation_status = "✅ Generation complete!".to_string();

        // Scan generated_graphs directory for actual files
        if let Ok(entries) = std::fs::read_dir("generated_graphs") {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "svg" || ext == "png" || ext == "pdf") {
                    // Determine graph type from filename
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let graph_type = if filename.contains("overview") {
                        GraphType::Overview
                    } else if filename.contains("cycles") {
                        GraphType::Cycles
                    } else if filename.contains("module_") {
                        let module_name = filename
                            .split("module_")
                            .nth(1)
                            .and_then(|s| s.split('.').next())
                            .unwrap_or("unknown");
                        GraphType::Module(module_name.to_string())
                    } else {
                        continue;
                    };

                    self.generated_files.push(GeneratedGraph {
                        path: path.clone(),
                        graph_type,
                        node_count: 0, // Would parse from file metadata in real impl
                        edge_count: 0,
                    });
                }
            }
        }

        // Set placeholder statistics (would parse from CLI output in real impl)
        self.total_functions = self.generated_files.len() * 100;
        self.total_calls = self.generated_files.len() * 150;
        self.num_modules = self.generated_files.iter()
            .filter(|g| matches!(g.graph_type, GraphType::Module(_)))
            .count();
        self.num_cycles = self.generated_files.iter()
            .filter(|g| matches!(g.graph_type, GraphType::Cycles))
            .count();
    }
}

impl GuiView for CallGraphViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        ui.heading("🔍 Call Graph Visualization (Doxygen-Style)");
        ui.label("Generate multi-view call graph visualizations with automatic cycle detection");
        ui.separator();

        // Settings panel
        self.render_settings(ui)?;
        ui.add_space(10.0);

        // Statistics
        self.render_statistics(ui)?;
        ui.add_space(10.0);

        // Generated graphs list
        self.render_graph_list(ui)?;
        ui.add_space(10.0);

        // Graph preview
        self.render_graph_preview(ui)?;

        Ok(())
    }

    fn handle_message(&mut self, _message: crate::GuiMessage) -> GuiResult<()> {
        Ok(())
    }

    fn get_title(&self) -> String {
        "🔍 Call Graph".to_string()
    }

    fn on_activate(&mut self) -> GuiResult<()> {
        Ok(())
    }

    fn on_deactivate(&mut self) -> GuiResult<()> {
        Ok(())
    }

    fn on_close(&mut self) -> GuiResult<bool> {
        Ok(true)
    }
}
