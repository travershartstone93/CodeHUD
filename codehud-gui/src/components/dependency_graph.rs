use crate::{GuiComponent, GuiMessage, GuiResult, signals::SignalBus, state::AppState};
use egui::{Context, Ui};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DependencyGraphComponent {
    state: Arc<RwLock<AppState>>,
    signal_bus: Arc<SignalBus>,
    visible: bool,
    enabled: bool,
}

impl DependencyGraphComponent {
    pub fn new(state: Arc<RwLock<AppState>>, signal_bus: Arc<SignalBus>) -> GuiResult<Self> {
        Ok(Self { state, signal_bus, visible: true, enabled: true })
    }
}

impl GuiComponent for DependencyGraphComponent {
    fn name(&self) -> &str { "dependency_graph" }
    fn render(&mut self, ui: &mut Ui, _ctx: &Context) -> GuiResult<()> {
        if self.visible { ui.label("Dependency Graph Component"); }
        Ok(())
    }
    fn handle_message(&mut self, _message: GuiMessage) -> GuiResult<()> { Ok(()) }
    fn is_visible(&self) -> bool { self.visible }
    fn set_visible(&mut self, visible: bool) { self.visible = visible; }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
}
