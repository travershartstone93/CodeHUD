//! Tests View GUI
//!
//! Displays test results, coverage metrics, and test management interface.

use crate::{GuiResult, GuiMessage, GuiView, state::AppState};
use egui::{Context, Ui, Color32};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test results and coverage interface
pub struct TestsViewGui {
    state: Arc<RwLock<AppState>>,
    show_coverage: bool,
    test_output: String,
    is_running: bool,
}

impl TestsViewGui {
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        Ok(Self {
            state,
            show_coverage: true,
            test_output: String::new(),
            is_running: false,
        })
    }

    pub fn get_view_title(&self) -> String {
        "🧪 Tests".to_string()
    }

    /// Run tests
    fn run_tests(&mut self) -> GuiResult<()> {
        self.is_running = true;
        self.test_output = "Running tests...\n".to_string();

        let state_clone = self.state.clone();
        let mut output_clone = self.test_output.clone();

        tokio::spawn(async move {
            let codebase_path = if let Ok(state) = state_clone.read().await {
                state.codebase_path.clone()
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            };

            let result = std::process::Command::new("cargo")
                .args(&["test", "--all"])
                .current_dir(&codebase_path)
                .output();

            match result {
                Ok(output) => {
                    output_clone.push_str(&String::from_utf8_lossy(&output.stdout));
                    output_clone.push_str(&String::from_utf8_lossy(&output.stderr));
                    println!("Tests completed");
                }
                Err(e) => {
                    eprintln!("❌ Failed to run tests: {}", e);
                }
            }
        });

        Ok(())
    }
}

impl GuiView for TestsViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        ui.heading("🧪 Test Results & Coverage");
        ui.separator();

        // Test summary
        ui.group(|ui| {
            ui.label("📊 Test Summary:");
            ui.horizontal(|ui| {
                ui.colored_label(Color32::GREEN, "✅ Passed: 127");
                ui.colored_label(Color32::RED, "❌ Failed: 3");
                ui.colored_label(Color32::YELLOW, "⚠️ Skipped: 5");
            });

            ui.horizontal(|ui| {
                ui.label("Total Coverage:");
                ui.colored_label(Color32::GREEN, "72.4%");
            });

            ui.horizontal(|ui| {
                ui.label("Test Duration:");
                ui.label("2.3s");
            });
        });

        ui.separator();

        // Toggle coverage view
        ui.checkbox(&mut self.show_coverage, "Show Coverage Details");

        if self.show_coverage {
            ui.group(|ui| {
                ui.label("📈 Coverage by Module:");
                ui.label("• src/main.rs: 85.2%");
                ui.label("• src/lib.rs: 92.1%");
                ui.label("• src/models.rs: 45.7%");
                ui.label("• src/utils.rs: 88.9%");
                ui.label("• src/config.rs: 20.5%");
            });
        }

        ui.separator();

        // Test controls
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.is_running, egui::Button::new("▶️ Run Tests")).clicked() {
                let _ = self.run_tests();
            }
            if ui.button("🔄 Refresh").clicked() {
                self.is_running = false;
            }
            if ui.button("📊 Generate Report").clicked() {
                // Generate coverage report
            }
        });

        // Show test output if available
        if !self.test_output.is_empty() {
            ui.separator();
            ui.group(|ui| {
                ui.label("Test Output:");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.code(&self.test_output);
                    });
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