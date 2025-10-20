//! Console View GUI
//!
//! Displays console output and command execution interface.

use crate::{GuiResult, GuiMessage, GuiView, state::AppState};
use egui::{Context, Ui, TextEdit};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Console output and command interface
pub struct ConsoleViewGui {
    state: Arc<RwLock<AppState>>,
    console_output: String,
    command_input: String,
}

impl ConsoleViewGui {
    pub fn new(state: Arc<RwLock<AppState>>) -> GuiResult<Self> {
        Ok(Self {
            state,
            console_output: "CodeHUD Console Ready\n> Welcome to CodeHUD interactive console\n> Type 'help' for available commands\n".to_string(),
            command_input: String::new(),
        })
    }

    pub fn get_view_title(&self) -> String {
        "💻 Console".to_string()
    }
}

impl GuiView for ConsoleViewGui {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        ui.heading("💻 CodeHUD Console");
        ui.separator();

        // Console output
        ui.group(|ui| {
            ui.label("Console Output:");
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(300.0)
                .show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.console_output)
                            .desired_width(f32::INFINITY)
                            .code_editor()
                    );
                });
        });

        ui.separator();

        // Command input
        ui.horizontal(|ui| {
            ui.label("Command:");
            if ui.add(
                TextEdit::singleline(&mut self.command_input)
                    .hint_text("Enter command...")
                    .desired_width(300.0)
            ).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !self.command_input.is_empty() {
                    self.console_output.push_str(&format!("> {}\n", self.command_input));
                    self.console_output.push_str("Command executed (console integration pending)\n");
                    self.command_input.clear();
                }
            }

            if ui.button("Execute").clicked() && !self.command_input.is_empty() {
                let cmd = self.command_input.clone();
                self.console_output.push_str(&format!("> {}\n", cmd));

                // Execute command
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if !parts.is_empty() {
                    match std::process::Command::new(parts[0])
                        .args(&parts[1..])
                        .output()
                    {
                        Ok(output) => {
                            self.console_output.push_str(&String::from_utf8_lossy(&output.stdout));
                            self.console_output.push_str(&String::from_utf8_lossy(&output.stderr));
                        }
                        Err(e) => {
                            self.console_output.push_str(&format!("Error: {}\n", e));
                        }
                    }
                }

                self.command_input.clear();
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