use crate::{GuiComponent, GuiMessage, GuiResult, signals::SignalBus, state::{AppState, ConversationState, MessageRole}};
use egui::{Context, Ui, ScrollArea, Color32, TextEdit};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct LlmDebuggerComponent {
    state: Arc<RwLock<AppState>>,
    signal_bus: Arc<SignalBus>,
    visible: bool,
    enabled: bool,
    current_input: String,
    selected_conversation: Option<String>,
    auto_scroll: bool,
    show_system_messages: bool,
    show_timestamps: bool,
    show_metadata: bool,
}

impl LlmDebuggerComponent {
    pub fn new(state: Arc<RwLock<AppState>>, signal_bus: Arc<SignalBus>) -> GuiResult<Self> {
        Ok(Self {
            state,
            signal_bus,
            visible: false, // Hidden by default
            enabled: true,
            current_input: String::new(),
            selected_conversation: None,
            auto_scroll: true,
            show_system_messages: false,
            show_timestamps: true,
            show_metadata: false,
        })
    }

    fn render_conversation_list(&mut self, ui: &mut Ui, conversations: &std::collections::HashMap<String, ConversationState>) -> GuiResult<()> {
        ui.heading("Conversations");

        if conversations.is_empty() {
            ui.label("No active conversations");
            return Ok(());
        }

        ScrollArea::vertical()
            .max_height(150.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (id, conversation) in conversations {
                    let is_selected = self.selected_conversation.as_ref() == Some(id);

                    if ui.selectable_label(is_selected, &conversation.title).clicked() {
                        self.selected_conversation = Some(id.clone());
                    }

                    if is_selected {
                        ui.indent(id, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("Messages: {}", conversation.messages.len()));
                                ui.label(format!("Updated: {}", conversation.updated_at.format("%H:%M:%S")));
                            });
                        });
                    }
                }
            });

        Ok(())
    }

    fn render_conversation_content(&mut self, ui: &mut Ui, conversation: &ConversationState) -> GuiResult<()> {
        ui.heading(&conversation.title);

        // Context info
        ui.collapsing("Context", |ui| {
            ui.text_edit_multiline(&mut conversation.context.clone());
        });

        ui.separator();

        // Messages
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                for message in &conversation.messages {
                    // Skip system messages if not showing them
                    if matches!(message.role, MessageRole::System) && !self.show_system_messages {
                        continue;
                    }

                    let (role_text, role_color) = match &message.role {
                        MessageRole::User => ("USER", Color32::BLUE),
                        MessageRole::Assistant => ("ASSISTANT", Color32::GREEN),
                        MessageRole::System => ("SYSTEM", Color32::GRAY),
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(role_color, role_text);

                        if self.show_timestamps {
                            ui.label(message.timestamp.format("%H:%M:%S").to_string());
                        }

                        if self.show_metadata && !message.metadata.is_empty() {
                            ui.label(format!("({})", message.metadata.len()));
                        }
                    });

                    // Message content
                    ui.indent("message_content", |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut message.content.clone())
                                .desired_width(f32::INFINITY)
                                .interactive(false)
                        );
                    });

                    // Metadata (if enabled)
                    if self.show_metadata && !message.metadata.is_empty() {
                        ui.collapsing("Metadata", |ui| {
                            for (key, value) in &message.metadata {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", key));
                                    ui.label(value);
                                });
                            }
                        });
                    }

                    ui.separator();
                }
            });

        Ok(())
    }

    fn render_input_controls(&mut self, ui: &mut Ui) -> GuiResult<()> {
        ui.heading("Send Message");

        // Input area
        ui.add(
            TextEdit::multiline(&mut self.current_input)
                .desired_rows(3)
                .hint_text("Type your message here...")
        );

        // Send controls
        ui.horizontal(|ui| {
            if ui.button("Send").clicked() && !self.current_input.trim().is_empty() {
                let message = self.current_input.clone();
                self.current_input.clear();
                let _ = self.signal_bus.emit("llm_request", GuiMessage::LlmRequest(message));
            }

            if ui.button("Clear").clicked() {
                self.current_input.clear();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("New Conversation").clicked() {
                    let _ = self.signal_bus.emit("new_conversation",
                        GuiMessage::LlmRequest("Start new conversation".to_string()));
                }
            });
        });

        Ok(())
    }

    fn render_debug_controls(&mut self, ui: &mut Ui) -> GuiResult<()> {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.auto_scroll, "Auto Scroll");
            ui.checkbox(&mut self.show_system_messages, "System Messages");
            ui.checkbox(&mut self.show_timestamps, "Timestamps");
            ui.checkbox(&mut self.show_metadata, "Metadata");
        });

        ui.horizontal(|ui| {
            if ui.button("Clear History").clicked() {
                let _ = self.signal_bus.emit("clear_llm_history", GuiMessage::LlmRequest("clear".to_string()));
            }

            if ui.button("Export Log").clicked() {
                let _ = self.signal_bus.emit("export_llm_log", GuiMessage::LlmRequest("export".to_string()));
            }

            if ui.button("Import Log").clicked() {
                let _ = self.signal_bus.emit("import_llm_log", GuiMessage::LlmRequest("import".to_string()));
            }
        });

        Ok(())
    }

    fn render_llm_history(&mut self, ui: &mut Ui) -> GuiResult<()> {
        ui.collapsing("LLM Interaction History", |ui| {
            // Try to read state without blocking
            if let Ok(state_guard) = self.state.try_read() {
                if state_guard.llm_history.is_empty() {
                    ui.label("No LLM interactions yet");
                    return;
                }

                ScrollArea::vertical()
                    .max_height(200.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for interaction in &state_guard.llm_history {
                            ui.collapsing(&interaction.id, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Model:");
                                    ui.label(&interaction.model);
                                    ui.label("Duration:");
                                    ui.label(format!("{}ms", interaction.duration_ms));
                                    ui.label("Tokens:");
                                    ui.label(format!("{}", interaction.tokens_used));
                                });

                                ui.label("Request:");
                                ui.indent("request", |ui| {
                                    ui.label(&interaction.request);
                                });

                                ui.label("Response:");
                                ui.indent("response", |ui| {
                                    ui.label(&interaction.response);
                                });
                            });
                        }
                    });
            }
        });

        Ok(())
    }
}

impl GuiComponent for LlmDebuggerComponent {
    fn name(&self) -> &str {
        "llm_debugger"
    }

    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        if !self.visible {
            return Ok(());
        }

        ui.vertical(|ui| {
            // Debug controls
            if let Err(e) = self.render_debug_controls(ui) {
                ui.colored_label(Color32::RED, format!("Error rendering debug controls: {}", e));
            }

            ui.separator();

            // Main content area
            ui.horizontal(|ui| {
                // Left panel - conversation list
                ui.vertical(|ui| {
                    ui.set_min_width(200.0);

                    let conversations = if let Ok(state_guard) = self.state.try_read() {
                        Some(state_guard.active_conversations.clone())
                    } else {
                        None
                    };

                    if let Some(conversations) = conversations {
                        if let Err(e) = self.render_conversation_list(ui, &conversations) {
                            ui.colored_label(Color32::RED, format!("Error rendering conversation list: {}", e));
                        }
                    } else {
                        ui.spinner();
                        ui.label("Loading conversations...");
                    }
                });

                ui.separator();

                // Right panel - conversation content and input
                ui.vertical(|ui| {
                    if let Some(conversation_id) = &self.selected_conversation {
                        let conversation_data = if let Ok(state_guard) = self.state.try_read() {
                            state_guard.active_conversations.get(conversation_id).cloned()
                        } else {
                            None
                        };

                        if let Some(conversation_data) = conversation_data {
                            if let Err(e) = self.render_conversation_content(ui, &conversation_data) {
                                ui.colored_label(Color32::RED, format!("Error rendering conversation: {}", e));
                            }
                        } else {
                            ui.label("Conversation not found");
                        }
                    } else {
                        ui.label("Select a conversation to view");
                    }

                    ui.separator();

                    // Input controls
                    if let Err(e) = self.render_input_controls(ui) {
                        ui.colored_label(Color32::RED, format!("Error rendering input controls: {}", e));
                    }
                });
            });

            ui.separator();

            // LLM History
            if let Err(e) = self.render_llm_history(ui) {
                ui.colored_label(Color32::RED, format!("Error rendering LLM history: {}", e));
            }
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match &message {
            GuiMessage::LlmRequest(request) => {
                log::info!("LLM request: {}", request);
            },
            GuiMessage::LlmResponse(response) => {
                log::info!("LLM response received: {} characters", response.len());
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