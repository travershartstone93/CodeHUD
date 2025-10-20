use crate::{GuiComponent, GuiMessage, GuiResult, signals::SignalBus};
use egui::{Context, Ui, menu};
use std::sync::Arc;

pub struct MenuBarComponent {
    signal_bus: Arc<SignalBus>,
    visible: bool,
    enabled: bool,
}

impl MenuBarComponent {
    pub fn new(signal_bus: Arc<SignalBus>) -> GuiResult<Self> {
        Ok(Self {
            signal_bus,
            visible: true,
            enabled: true,
        })
    }
}

impl GuiComponent for MenuBarComponent {
    fn name(&self) -> &str {
        "menu_bar"
    }

    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        if !self.visible {
            return Ok(());
        }

        menu::bar(ui, |ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    let _ = self.signal_bus.emit("new_project", GuiMessage::ProjectLoaded("".to_string()));
                    ui.close_menu();
                }

                if ui.button("Open Project...").clicked() {
                    let _ = self.signal_bus.emit("open_project", GuiMessage::ProjectLoaded("".to_string()));
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Recent Projects").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Save").clicked() {
                    ui.close_menu();
                }

                if ui.button("Save As...").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            // Edit menu
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    ui.close_menu();
                }

                if ui.button("Redo").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Cut").clicked() {
                    ui.close_menu();
                }

                if ui.button("Copy").clicked() {
                    ui.close_menu();
                }

                if ui.button("Paste").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Find...").clicked() {
                    ui.close_menu();
                }

                if ui.button("Replace...").clicked() {
                    ui.close_menu();
                }
            });

            // View menu
            ui.menu_button("View", |ui| {
                if ui.button("Topology View").clicked() {
                    let _ = self.signal_bus.emit("show_topology", GuiMessage::TopologyUpdate);
                    ui.close_menu();
                }

                if ui.button("Quality Dashboard").clicked() {
                    let _ = self.signal_bus.emit("show_quality", GuiMessage::QualityUpdate);
                    ui.close_menu();
                }

                if ui.button("Health Monitor").clicked() {
                    let _ = self.signal_bus.emit("show_health", GuiMessage::HealthUpdate);
                    ui.close_menu();
                }

                if ui.button("LLM Debugger").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Project Explorer").clicked() {
                    ui.close_menu();
                }

                if ui.button("Console Output").clicked() {
                    ui.close_menu();
                }

                if ui.button("Metrics Panel").clicked() {
                    ui.close_menu();
                }
            });

            // Analysis menu
            ui.menu_button("Analysis", |ui| {
                if ui.button("Run Full Analysis").clicked() {
                    let _ = self.signal_bus.emit("run_analysis", GuiMessage::AnalysisComplete);
                    ui.close_menu();
                }

                if ui.button("Quick Analysis").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Code Quality Check").clicked() {
                    ui.close_menu();
                }

                if ui.button("Security Scan").clicked() {
                    ui.close_menu();
                }

                if ui.button("Performance Analysis").clicked() {
                    ui.close_menu();
                }
            });

            // LLM menu
            ui.menu_button("LLM", |ui| {
                if ui.button("Start Conversation").clicked() {
                    let _ = self.signal_bus.emit("start_llm_conversation",
                        GuiMessage::LlmRequest("Start new conversation".to_string()));
                    ui.close_menu();
                }

                if ui.button("Code Review").clicked() {
                    ui.close_menu();
                }

                if ui.button("Generate Documentation").clicked() {
                    ui.close_menu();
                }

                if ui.button("Suggest Improvements").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("LLM Settings").clicked() {
                    ui.close_menu();
                }
            });

            // Tools menu
            ui.menu_button("Tools", |ui| {
                if ui.button("Git Integration").clicked() {
                    ui.close_menu();
                }

                if ui.button("Test Runner").clicked() {
                    ui.close_menu();
                }

                if ui.button("Code Formatter").clicked() {
                    ui.close_menu();
                }

                if ui.button("Refactoring Tools").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Plugin Manager").clicked() {
                    ui.close_menu();
                }

                if ui.button("Task Scheduler").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Settings").clicked() {
                    ui.close_menu();
                }
            });

            // Help menu
            ui.menu_button("Help", |ui| {
                if ui.button("Documentation").clicked() {
                    ui.close_menu();
                }

                if ui.button("Keyboard Shortcuts").clicked() {
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("About CodeHUD").clicked() {
                    ui.close_menu();
                }
            });
        });

        Ok(())
    }

    fn handle_message(&mut self, _message: GuiMessage) -> GuiResult<()> {
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