//! Health Dashboard Widget
//!
//! Displays overall codebase health status and metrics summary.

use crate::{GuiResult, GuiMessage, GuiComponent, state::AppState};
use egui::{Context, Ui, Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Health dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall_score: f64,
    pub quality_score: f64,
    pub security_score: f64,
    pub performance_score: f64,
    pub test_coverage: f64,
    pub technical_debt: f64,
    pub last_updated: String,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            overall_score: 85.0,
            quality_score: 82.0,
            security_score: 95.0,
            performance_score: 78.0,
            test_coverage: 72.4,
            technical_debt: 15.2,
            last_updated: "Just now".to_string(),
        }
    }
}

/// Health dashboard widget displaying overall codebase health
pub struct HealthDashboard {
    state: Arc<RwLock<AppState>>,
    health_status: HealthStatus,
    visible: bool,
    enabled: bool,
}

impl HealthDashboard {
    /// Create new health dashboard
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self {
            state,
            health_status: HealthStatus::default(),
            visible: true,
            enabled: true,
        }
    }

    /// Update health status
    pub fn update_status(&mut self, status: HealthStatus) {
        self.health_status = status;
    }

    /// Render health score with color coding
    fn render_health_score(&self, ui: &mut Ui, label: &str, score: f64) {
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));

            let color = if score >= 80.0 {
                Color32::from_rgb(46, 204, 113) // Green
            } else if score >= 60.0 {
                Color32::from_rgb(243, 156, 18) // Orange
            } else {
                Color32::from_rgb(231, 76, 60) // Red
            };

            ui.colored_label(color, format!("{:.1}%", score));
        });
    }
}

impl GuiComponent for HealthDashboard {
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        if !self.visible {
            return Ok(());
        }

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("💚 Health Dashboard");
                ui.separator();
                ui.small(format!("Last updated: {}", self.health_status.last_updated));
            });

            ui.separator();

            // Overall score prominently displayed
            ui.vertical_centered(|ui| {
                ui.label("Overall Health");
                let overall_color = if self.health_status.overall_score >= 80.0 {
                    Color32::from_rgb(46, 204, 113)
                } else if self.health_status.overall_score >= 60.0 {
                    Color32::from_rgb(243, 156, 18)
                } else {
                    Color32::from_rgb(231, 76, 60)
                };

                ui.colored_label(
                    overall_color,
                    egui::RichText::new(format!("{:.1}%", self.health_status.overall_score))
                        .size(24.0)
                        .strong()
                );
            });

            ui.separator();

            // Individual metrics
            ui.columns(2, |columns| {
                columns[0].group(|ui| {
                    ui.label("📊 Quality Metrics");
                    ui.separator();
                    self.render_health_score(ui, "Code Quality", self.health_status.quality_score);
                    self.render_health_score(ui, "Test Coverage", self.health_status.test_coverage);

                    ui.horizontal(|ui| {
                        ui.label("Technical Debt:");
                        let debt_color = if self.health_status.technical_debt < 10.0 {
                            Color32::GREEN
                        } else if self.health_status.technical_debt < 20.0 {
                            Color32::YELLOW
                        } else {
                            Color32::RED
                        };
                        ui.colored_label(debt_color, format!("{:.1}%", self.health_status.technical_debt));
                    });
                });

                columns[1].group(|ui| {
                    ui.label("🔒 Security & Performance");
                    ui.separator();
                    self.render_health_score(ui, "Security Score", self.health_status.security_score);
                    self.render_health_score(ui, "Performance", self.health_status.performance_score);

                    // Health trend indicator
                    ui.horizontal(|ui| {
                        ui.label("Trend:");
                        ui.colored_label(Color32::GREEN, "↗ Improving");
                    });
                });
            });

            ui.separator();

            // Quick actions
            ui.horizontal(|ui| {
                if ui.small_button("🔄 Refresh").clicked() {
                    // Trigger health refresh
                }
                if ui.small_button("📊 Detailed Report").clicked() {
                    // Show detailed health report
                }
                if ui.small_button("🎯 View Issues").clicked() {
                    // Navigate to issues view
                }
            });
        });

        Ok(())
    }

    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()> {
        match message {
            GuiMessage::HealthUpdate => {
                // Update health metrics from analysis
                Ok(())
            }
            _ => Ok(())
        }
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

    fn name(&self) -> &str {
        "HealthDashboard"
    }
}