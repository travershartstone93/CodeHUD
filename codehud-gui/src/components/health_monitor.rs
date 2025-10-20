use crate::{GuiComponent, GuiMessage, GuiResult, signals::SignalBus, state::{AppState, HealthStatus, AlertLevel}};
use egui::{Context, Ui, ScrollArea, Color32, ProgressBar};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HealthMonitorComponent {
    state: Arc<RwLock<AppState>>,
    signal_bus: Arc<SignalBus>,
    visible: bool,
    auto_refresh: bool,
    refresh_interval: f64,
    last_refresh: f64,
}

impl HealthMonitorComponent {
    pub fn new(state: Arc<RwLock<AppState>>, signal_bus: Arc<SignalBus>) -> GuiResult<Self> {
        Ok(Self {
            state,
            signal_bus,
            visible: true,
            auto_refresh: true,
            refresh_interval: 1.0, // 1 second
            last_refresh: 0.0,
        })
    }

    fn render_health_overview(&mut self, ui: &mut Ui, health: &HealthStatus) -> GuiResult<()> {
        ui.heading("System Health");

        // Overall health score
        ui.horizontal(|ui| {
            ui.label("Overall Health:");
            let progress = health.overall_health / 100.0;
            let color = if progress > 0.8 {
                Color32::GREEN
            } else if progress > 0.5 {
                Color32::YELLOW
            } else {
                Color32::RED
            };

            let progress_bar = ProgressBar::new(progress)
                .fill(color)
                .text(format!("{:.1}%", health.overall_health));
            ui.add(progress_bar);
        });

        // System status
        ui.horizontal(|ui| {
            ui.label("Status:");
            let (status_text, status_color) = match &health.system_status {
                crate::state::SystemStatus::Healthy => ("Healthy", Color32::GREEN),
                crate::state::SystemStatus::Warning => ("Warning", Color32::YELLOW),
                crate::state::SystemStatus::Critical => ("Critical", Color32::RED),
                crate::state::SystemStatus::Unknown => ("Unknown", Color32::GRAY),
            };
            ui.colored_label(status_color, status_text);
        });

        ui.separator();
        Ok(())
    }

    fn render_performance_metrics(&mut self, ui: &mut Ui, health: &HealthStatus) -> GuiResult<()> {
        ui.heading("Performance Metrics");

        let metrics = &health.performance_metrics;

        ui.horizontal(|ui| {
            ui.label("Analysis Time:");
            ui.label(format!("{:.2}s", metrics.analysis_time));
        });

        ui.horizontal(|ui| {
            ui.label("Memory Usage:");
            let progress = (metrics.memory_usage / 100.0).min(1.0);
            let color = if progress > 0.8 { Color32::RED } else if progress > 0.6 { Color32::YELLOW } else { Color32::GREEN };
            let progress_bar = ProgressBar::new(progress)
                .fill(color)
                .text(format!("{:.1}%", metrics.memory_usage));
            ui.add(progress_bar);
        });

        ui.horizontal(|ui| {
            ui.label("CPU Usage:");
            let progress = (metrics.cpu_usage / 100.0).min(1.0);
            let color = if progress > 0.8 { Color32::RED } else if progress > 0.6 { Color32::YELLOW } else { Color32::GREEN };
            let progress_bar = ProgressBar::new(progress)
                .fill(color)
                .text(format!("{:.1}%", metrics.cpu_usage));
            ui.add(progress_bar);
        });

        ui.horizontal(|ui| {
            ui.label("Disk Usage:");
            let progress = (metrics.disk_usage / 100.0).min(1.0);
            let color = if progress > 0.9 { Color32::RED } else if progress > 0.8 { Color32::YELLOW } else { Color32::GREEN };
            let progress_bar = ProgressBar::new(progress)
                .fill(color)
                .text(format!("{:.1}%", metrics.disk_usage));
            ui.add(progress_bar);
        });

        ui.horizontal(|ui| {
            ui.label("Throughput:");
            ui.label(format!("{:.2} ops/s", metrics.throughput));
        });

        ui.separator();
        Ok(())
    }

    fn render_resource_usage(&mut self, ui: &mut Ui, health: &HealthStatus) -> GuiResult<()> {
        ui.heading("Resource Usage");

        let usage = &health.resource_usage;

        ui.horizontal(|ui| {
            ui.label("Memory:");
            ui.label(format!("{:.1} MB", usage.memory_mb));
        });

        ui.horizontal(|ui| {
            ui.label("CPU:");
            ui.label(format!("{:.1}%", usage.cpu_percent));
        });

        ui.horizontal(|ui| {
            ui.label("Disk Space:");
            ui.label(format!("{:.1} GB", usage.disk_space_gb));
        });

        ui.horizontal(|ui| {
            ui.label("Network I/O:");
            ui.label(format!("{:.2} MB/s", usage.network_io));
        });

        if let Some(gpu_usage) = usage.gpu_usage {
            ui.horizontal(|ui| {
                ui.label("GPU Usage:");
                ui.label(format!("{:.1}%", gpu_usage));
            });
        }

        ui.separator();
        Ok(())
    }

    fn render_alerts(&mut self, ui: &mut Ui, health: &HealthStatus) -> GuiResult<()> {
        ui.heading(format!("Alerts ({})", health.alerts.len()));

        if health.alerts.is_empty() {
            ui.colored_label(Color32::GREEN, "No active alerts");
            return Ok(());
        }

        ScrollArea::vertical()
            .max_height(200.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for alert in &health.alerts {
                    let (level_text, level_color) = match &alert.level {
                        AlertLevel::Info => ("INFO", Color32::BLUE),
                        AlertLevel::Warning => ("WARN", Color32::YELLOW),
                        AlertLevel::Error => ("ERROR", Color32::RED),
                        AlertLevel::Critical => ("CRIT", Color32::from_rgb(200, 0, 0)),
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(level_color, level_text);
                        ui.label(format!("[{}]", alert.component));
                        ui.label(&alert.message);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(alert.timestamp.format("%H:%M:%S").to_string());
                        });
                    });

                    ui.separator();
                }
            });

        Ok(())
    }

    fn render_controls(&mut self, ui: &mut Ui) -> GuiResult<()> {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.auto_refresh, "Auto Refresh");

            if ui.button("Refresh Now").clicked() {
                let _ = self.signal_bus.emit("refresh_health", GuiMessage::HealthUpdate);
            }

            if ui.button("Clear Alerts").clicked() {
                let _ = self.signal_bus.emit("clear_alerts", GuiMessage::HealthUpdate);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Refresh: {:.1}s", self.refresh_interval));
                if ui.button("−").clicked() && self.refresh_interval > 0.5 {
                    self.refresh_interval -= 0.5;
                }
                if ui.button("+").clicked() && self.refresh_interval < 10.0 {
                    self.refresh_interval += 0.5;
                }
            });
        });

        Ok(())
    }
}

impl GuiComponent for HealthMonitorComponent {
    fn name(&self) -> &str {
        "health_monitor"
    }

    fn render(&mut self, ui: &mut Ui, ctx: &Context) -> GuiResult<()> {
        if !self.visible {
            return Ok(());
        }

        // Auto-refresh logic
        let current_time = ctx.input(|i| i.time);
        if self.auto_refresh && current_time - self.last_refresh >= self.refresh_interval {
            let _ = self.signal_bus.emit("refresh_health", GuiMessage::HealthUpdate);
            self.last_refresh = current_time;
        }

        ui.vertical(|ui| {
            // Controls
            if let Err(e) = self.render_controls(ui) {
                ui.colored_label(Color32::RED, format!("Error rendering controls: {}", e));
            }

            ui.separator();

            // Health content
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Try to read state without blocking
                    let health_opt = if let Ok(state_guard) = self.state.try_read() {
                        Some(state_guard.health_status.clone())
                    } else {
                        None
                    };

                    if let Some(health) = health_opt {
                        if let Err(e) = self.render_health_overview(ui, &health) {
                            ui.colored_label(Color32::RED, format!("Error rendering health overview: {}", e));
                        }

                        if let Err(e) = self.render_performance_metrics(ui, &health) {
                            ui.colored_label(Color32::RED, format!("Error rendering performance metrics: {}", e));
                        }

                        if let Err(e) = self.render_resource_usage(ui, &health) {
                            ui.colored_label(Color32::RED, format!("Error rendering resource usage: {}", e));
                        }

                        if let Err(e) = self.render_alerts(ui, &health) {
                            ui.colored_label(Color32::RED, format!("Error rendering alerts: {}", e));
                        }
                    } else {
                        ui.spinner();
                        ui.label("Loading health status...");
                    }
                });
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match &message {
            GuiMessage::HealthUpdate => {
                log::info!("Health status updated");
            },
            GuiMessage::Error(err) => {
                log::error!("Health monitor received error: {}", err);
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
        true // Health monitor is always enabled
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // Health monitor cannot be disabled
    }
}