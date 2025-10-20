//! Settings View GUI
//!
//! Displays application settings and configuration options.

use crate::{GuiResult, GuiMessage, GuiView, state::AppState};
use egui::{Context, Ui, Color32};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application settings and configuration
pub struct SettingsViewGui {
    state: Arc<RwLock<AppState>>,
    dark_mode: bool,
    auto_analysis: bool,
    notification_level: String,
    max_file_size: f32,
}

impl SettingsViewGui {
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        Ok(Self {
            state,
            dark_mode: false,
            auto_analysis: true,
            notification_level: "Normal".to_string(),
            max_file_size: 10.0,
        })
    }

    pub fn get_view_title(&self) -> String {
        "⚙️ Settings".to_string()
    }
}

impl GuiView for SettingsViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        ui.heading("⚙️ Application Settings");
        ui.separator();

        // Theme settings
        ui.group(|ui| {
            ui.label("🎨 Appearance:");
            ui.checkbox(&mut self.dark_mode, "Dark Mode");

            if self.dark_mode {
                ui.colored_label(Color32::LIGHT_GRAY, "Dark theme enabled");
            } else {
                ui.colored_label(Color32::BLACK, "Light theme enabled");
            }
        });

        ui.separator();

        // Analysis settings
        ui.group(|ui| {
            ui.label("🔍 Analysis:");
            ui.checkbox(&mut self.auto_analysis, "Auto-analyze on file changes");

            ui.horizontal(|ui| {
                ui.label("Max file size (MB):");
                ui.add(egui::Slider::new(&mut self.max_file_size, 1.0..=100.0));
            });

            ui.horizontal(|ui| {
                ui.label("Notification Level:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.notification_level)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.notification_level, "Silent".to_string(), "Silent");
                        ui.selectable_value(&mut self.notification_level, "Normal".to_string(), "Normal");
                        ui.selectable_value(&mut self.notification_level, "Verbose".to_string(), "Verbose");
                    });
            });
        });

        ui.separator();

        // Performance settings
        ui.group(|ui| {
            ui.label("⚡ Performance:");
            ui.label("• Thread pool size: 4");
            ui.label("• Cache size: 100 MB");
            ui.label("• Analysis timeout: 30s");
        });

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("💾 Save Settings").clicked() {
                let settings = serde_json::json!({
                    "dark_mode": self.dark_mode,
                    "auto_analysis": self.auto_analysis,
                    "notification_level": self.notification_level,
                    "max_file_size": self.max_file_size
                });

                if let Ok(home_dir) = std::env::var("HOME") {
                    let config_path = format!("{}/.codehud/config.json", home_dir);
                    if let Ok(parent) = std::path::Path::new(&config_path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&config_path, serde_json::to_string_pretty(&settings).unwrap());
                    println!("✅ Settings saved to {}", config_path);
                }
            }
            if ui.button("🔄 Reset to Defaults").clicked() {
                self.dark_mode = false;
                self.auto_analysis = true;
                self.notification_level = "Normal".to_string();
                self.max_file_size = 10.0;
            }
            if ui.button("📤 Export Config").clicked() {
                let settings = serde_json::json!({
                    "dark_mode": self.dark_mode,
                    "auto_analysis": self.auto_analysis,
                    "notification_level": self.notification_level,
                    "max_file_size": self.max_file_size
                });
                println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            }
        });

        Ok(())
    }

    fn handle_message(&mut self, _message: GuiMessage) -> GuiResult<()> {
        Ok(())
    }

    fn get_title(&self) -> String {
        self.get_view_title()
    }
}