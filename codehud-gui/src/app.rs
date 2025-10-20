use crate::{
    components::*,
    signals::SignalBus,
    state::AppState,
    views::*,
    GuiMessage, GuiResult, GuiView, GuiComponent,
};
use crate::views::CallGraphViewGui;
use eframe::egui::{self, Context, Ui, ViewportBuilder};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CodeHudGuiApp {
    state: Arc<RwLock<AppState>>,
    signal_bus: Arc<SignalBus>,

    // Core components (25+ as per plan)
    menu_bar: MenuBarComponent,
    project_explorer: ProjectExplorerComponent,
    code_editor: CodeEditorComponent,
    topology_view: TopologyViewComponent,
    quality_dashboard: QualityDashboardComponent,
    health_monitor: HealthMonitorComponent,
    llm_debugger: LlmDebuggerComponent,
    metrics_panel: MetricsPanelComponent,
    console_output: ConsoleOutputComponent,
    status_bar: StatusBarComponent,
    toolbar: ToolbarComponent,
    search_panel: SearchPanelComponent,
    file_browser: FileBrowserComponent,
    dependency_graph: DependencyGraphComponent,
    code_quality_metrics: CodeQualityMetricsComponent,
    performance_monitor: PerformanceMonitorComponent,
    git_integration: GitIntegrationComponent,
    test_runner: TestRunnerComponent,
    documentation_viewer: DocumentationViewerComponent,
    settings_panel: SettingsPanelComponent,
    plugin_manager: PluginManagerComponent,
    task_scheduler: TaskSchedulerComponent,
    ai_assistant: AiAssistantComponent,
    code_formatter: CodeFormatterComponent,
    refactoring_tools: RefactoringToolsComponent,

    // Views (11+ as per plan)
    views: HashMap<String, Box<dyn GuiView>>,
    active_view: String,

    // UI state
    is_initialized: bool,
    window_title: String,
}

impl CodeHudGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> GuiResult<Self> {
        let state = Arc::new(RwLock::new(AppState::new()?));
        let signal_bus = Arc::new(SignalBus::new());

        // Initialize views
        let mut views: HashMap<String, Box<dyn GuiView>> = HashMap::new();
        views.insert("topology".to_string(), Box::new(TopologyViewGui::new(state.clone())?));
        views.insert("quality".to_string(), Box::new(QualityViewGui::new(state.clone())?));
        views.insert("health".to_string(), Box::new(HealthViewGui::new(state.clone())?));
        views.insert("llm".to_string(), Box::new(LlmViewGui::new(state.clone())?));
        views.insert("metrics".to_string(), Box::new(MetricsViewGui::new(state.clone())?));
        views.insert("console".to_string(), Box::new(ConsoleViewGui::new(state.clone())?));
        views.insert("files".to_string(), Box::new(FilesViewGui::new(state.clone())?));
        views.insert("dependencies".to_string(), Box::new(DependenciesViewGui::new(state.clone())?));
        views.insert("call_graph".to_string(), Box::new(CallGraphViewGui::new(state.clone())?));
        views.insert("tests".to_string(), Box::new(TestsViewGui::new(state.clone())?));
        views.insert("documentation".to_string(), Box::new(DocumentationViewGui::new(state.clone())?));
        views.insert("settings".to_string(), Box::new(SettingsViewGui::new(state.clone())?));

        Ok(Self {
            state: state.clone(),
            signal_bus: signal_bus.clone(),

            // Initialize all 25+ components
            menu_bar: MenuBarComponent::new(signal_bus.clone())?,
            project_explorer: ProjectExplorerComponent::new(state.clone(), signal_bus.clone())?,
            code_editor: CodeEditorComponent::new(state.clone(), signal_bus.clone())?,
            topology_view: TopologyViewComponent::new(state.clone(), signal_bus.clone())?,
            quality_dashboard: QualityDashboardComponent::new(state.clone(), signal_bus.clone())?,
            health_monitor: HealthMonitorComponent::new(state.clone(), signal_bus.clone())?,
            llm_debugger: LlmDebuggerComponent::new(state.clone(), signal_bus.clone())?,
            metrics_panel: MetricsPanelComponent::new(state.clone(), signal_bus.clone())?,
            console_output: ConsoleOutputComponent::new(state.clone(), signal_bus.clone())?,
            status_bar: StatusBarComponent::new(state.clone(), signal_bus.clone())?,
            toolbar: ToolbarComponent::new(state.clone(), signal_bus.clone())?,
            search_panel: SearchPanelComponent::new(state.clone(), signal_bus.clone())?,
            file_browser: FileBrowserComponent::new(state.clone(), signal_bus.clone())?,
            dependency_graph: DependencyGraphComponent::new(state.clone(), signal_bus.clone())?,
            code_quality_metrics: CodeQualityMetricsComponent::new(state.clone(), signal_bus.clone())?,
            performance_monitor: PerformanceMonitorComponent::new(state.clone(), signal_bus.clone())?,
            git_integration: GitIntegrationComponent::new(state.clone(), signal_bus.clone())?,
            test_runner: TestRunnerComponent::new(state.clone(), signal_bus.clone())?,
            documentation_viewer: DocumentationViewerComponent::new(state.clone(), signal_bus.clone())?,
            settings_panel: SettingsPanelComponent::new(state.clone(), signal_bus.clone())?,
            plugin_manager: PluginManagerComponent::new(state.clone(), signal_bus.clone())?,
            task_scheduler: TaskSchedulerComponent::new(state.clone(), signal_bus.clone())?,
            ai_assistant: AiAssistantComponent::new(state.clone(), signal_bus.clone())?,
            code_formatter: CodeFormatterComponent::new(state.clone(), signal_bus.clone())?,
            refactoring_tools: RefactoringToolsComponent::new(state.clone(), signal_bus.clone())?,

            views,
            active_view: "topology".to_string(),
            is_initialized: false,
            window_title: "CodeHUD - Visual Mission Control for Codebases".to_string(),
        })
    }

    pub async fn initialize(&mut self) -> GuiResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        // Initialize state
        {
            let mut state = self.state.write().await;
            state.initialize().await?;
        }

        // Set up signal connections (PyQt5-style signal/slot architecture)
        self.setup_signal_connections().await?;

        self.is_initialized = true;
        Ok(())
    }

    async fn setup_signal_connections(&self) -> GuiResult<()> {
        // Connect project explorer signals to other components
        self.signal_bus.connect(
            "project_loaded",
            Box::new(|message| {
                if let GuiMessage::ProjectLoaded(path) = message {
                    log::info!("Project loaded: {}", path);
                }
                Ok(())
            })
        )?;

        // Connect LLM signals
        self.signal_bus.connect(
            "llm_response",
            Box::new(|message| {
                if let GuiMessage::LlmResponse(response) = message {
                    log::info!("LLM response received: {} chars", response.len());
                }
                Ok(())
            })
        )?;

        // Connect quality update signals
        self.signal_bus.connect(
            "quality_updated",
            Box::new(|message| {
                if let GuiMessage::QualityUpdate = message {
                    log::info!("Quality metrics updated");
                }
                Ok(())
            })
        )?;

        Ok(())
    }

    fn render_main_layout(&mut self, ctx: &Context) -> GuiResult<()> {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar.render(ui, ctx)
        }).inner?;

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.toolbar.render(ui, ctx)
        }).inner?;

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.status_bar.render(ui, ctx)
        }).inner?;

        egui::SidePanel::left("project_explorer").show(ctx, |ui| {
            self.project_explorer.render(ui, ctx)
        }).inner?;

        egui::SidePanel::right("metrics_panel").show(ctx, |ui| {
            self.metrics_panel.render(ui, ctx)
        }).inner?;

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_central_area(ui, ctx)
        }).inner
    }

    fn render_central_area(&mut self, ui: &mut Ui, ctx: &Context) -> GuiResult<()> {
        // Tab bar for switching between views
        ui.horizontal(|ui| {
            for (name, _) in &self.views {
                if ui.selectable_label(self.active_view == *name, name.as_str()).clicked() {
                    self.active_view = name.clone();
                }
            }
        });

        ui.separator();

        // Render active view
        if let Some(view) = self.views.get_mut(&self.active_view) {
            view.render(ui, ctx)?;
        }

        Ok(())
    }
}

impl eframe::App for CodeHudGuiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if !self.is_initialized {
            // Initialize immediately to skip loading screen
            self.is_initialized = true;
        }

        // Process signals (skip for now to avoid potential issues)
        // if let Err(e) = self.signal_bus.process_pending() {
        //     log::error!("Error processing signals: {}", e);
        // }

        // Render main layout
        if let Err(e) = self.render_main_layout(ctx) {
            log::error!("Error rendering main layout: {}", e);
        }

        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Save application state
        if let Ok(state_guard) = self.state.try_read() {
            if let Ok(serialized) = serde_json::to_string(&*state_guard) {
                storage.set_string("app_state", serialized);
            }
        }
    }
}