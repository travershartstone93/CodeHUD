use crate::{GuiResult, GuiError};
// use codehud_core::{CoreEngine, ProjectMetrics};  // Temporarily disabled
// use codehud_llm::LlmEngine;  // Temporarily disabled
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    // Project state
    pub current_project: Option<ProjectState>,
    pub recent_projects: Vec<PathBuf>,

    // Engine states
    // pub core_engine: Option<Arc<RwLock<CoreEngine>>>,  // Temporarily disabled
    // pub llm_engine: Option<Arc<RwLock<LlmEngine>>>,  // Temporarily disabled

    // UI state
    pub window_layout: WindowLayout,
    pub theme: Theme,
    pub preferences: UserPreferences,

    // Analysis state
    pub topology_data: Option<TopologyData>,
    pub quality_metrics: Option<QualityMetrics>,
    pub health_status: HealthStatus,

    // LLM state
    pub active_conversations: HashMap<String, ConversationState>,
    pub llm_history: Vec<LlmInteraction>,

    // Component visibility
    pub component_visibility: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub path: PathBuf,
    pub name: String,
    pub language: String,
    pub files: Vec<PathBuf>,
    pub last_analysis: Option<chrono::DateTime<chrono::Utc>>,
    // pub metrics: Option<ProjectMetrics>,  // Temporarily disabled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayout {
    pub main_window_size: (f32, f32),
    pub main_window_pos: (f32, f32),
    pub panel_sizes: HashMap<String, f32>,
    pub panel_positions: HashMap<String, (f32, f32)>,
    pub splitter_positions: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub dark_mode: bool,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
    pub accent_color: [f32; 4],
    pub background_color: [f32; 4],
    pub text_color: [f32; 4],
    pub font_size: f32,
    pub font_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub auto_save: bool,
    pub auto_analysis: bool,
    pub llm_auto_suggestions: bool,
    pub show_tooltips: bool,
    pub animation_speed: f32,
    pub max_recent_projects: usize,
    pub analysis_depth: AnalysisDepth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisDepth {
    Quick,
    Standard,
    Deep,
    Comprehensive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyData {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub clusters: Vec<TopologyCluster>,
    pub metrics: TopologyMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: [f32; 4],
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub weight: f32,
    pub edge_type: String,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyCluster {
    pub id: String,
    pub name: String,
    pub nodes: Vec<String>,
    pub bounds: (f32, f32, f32, f32),
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    pub cluster_count: usize,
    pub density: f32,
    pub modularity: f32,
    pub average_degree: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub overall_score: f32,
    pub maintainability: f32,
    pub readability: f32,
    pub testability: f32,
    pub performance: f32,
    pub security: f32,
    pub complexity_metrics: ComplexityMetrics,
    pub issue_counts: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic: f32,
    pub cognitive: f32,
    pub halstead: f32,
    pub lines_of_code: usize,
    pub technical_debt_hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall_health: f32,
    pub system_status: SystemStatus,
    pub performance_metrics: PerformanceMetrics,
    pub resource_usage: ResourceUsage,
    pub alerts: Vec<HealthAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub analysis_time: f32,
    pub memory_usage: f32,
    pub cpu_usage: f32,
    pub disk_usage: f32,
    pub throughput: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_mb: f32,
    pub cpu_percent: f32,
    pub disk_space_gb: f32,
    pub network_io: f32,
    pub gpu_usage: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub id: String,
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub component: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    pub id: String,
    pub title: String,
    pub messages: Vec<ConversationMessage>,
    pub context: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInteraction {
    pub id: String,
    pub request: String,
    pub response: String,
    pub model: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub tokens_used: u32,
}

impl AppState {
    pub fn new() -> GuiResult<Self> {
        Ok(Self {
            current_project: None,
            recent_projects: Vec::new(),
            // core_engine: None,
            // llm_engine: None,
            window_layout: WindowLayout::default(),
            theme: Theme::default(),
            preferences: UserPreferences::default(),
            topology_data: None,
            quality_metrics: None,
            health_status: HealthStatus::default(),
            active_conversations: HashMap::new(),
            llm_history: Vec::new(),
            component_visibility: Self::default_component_visibility(),
        })
    }

    pub async fn initialize(&mut self) -> GuiResult<()> {
        // Initialize core engine (temporarily disabled)
        // let core_engine = CoreEngine::new().await
        //     .map_err(|e| GuiError::Core(e))?;
        // self.core_engine = Some(Arc::new(RwLock::new(core_engine)));

        // Initialize LLM engine (temporarily disabled)
        // let llm_engine = LlmEngine::new().await
        //     .map_err(|e| GuiError::Llm(e))?;
        // self.llm_engine = Some(Arc::new(RwLock::new(llm_engine)));

        Ok(())
    }

    pub async fn load_project(&mut self, path: PathBuf) -> GuiResult<()> {
        let project_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Project")
            .to_string();

        let project_state = ProjectState {
            path: path.clone(),
            name: project_name,
            language: self.detect_project_language(&path).await?,
            files: self.scan_project_files(&path).await?,
            last_analysis: None,
            // metrics: None,
        };

        // Add to recent projects
        if !self.recent_projects.contains(&path) {
            self.recent_projects.insert(0, path.clone());
            self.recent_projects.truncate(self.preferences.max_recent_projects);
        }

        self.current_project = Some(project_state);
        Ok(())
    }

    async fn detect_project_language(&self, path: &PathBuf) -> GuiResult<String> {
        // Simple language detection based on file extensions
        let files = self.scan_project_files(path).await?;
        let mut language_counts: HashMap<String, usize> = HashMap::new();

        for file in files {
            if let Some(extension) = file.extension().and_then(|e| e.to_str()) {
                let language = match extension {
                    "py" => "Python",
                    "rs" => "Rust",
                    "js" | "ts" => "JavaScript/TypeScript",
                    "java" => "Java",
                    "cpp" | "cc" | "cxx" => "C++",
                    "c" => "C",
                    "go" => "Go",
                    _ => "Other",
                };
                *language_counts.entry(language.to_string()).or_insert(0) += 1;
            }
        }

        Ok(language_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.clone())
            .unwrap_or_else(|| "Unknown".to_string()))
    }

    async fn scan_project_files(&self, path: &PathBuf) -> GuiResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    fn default_component_visibility() -> HashMap<String, bool> {
        let mut visibility = HashMap::new();

        // Default visible components
        visibility.insert("project_explorer".to_string(), true);
        visibility.insert("code_editor".to_string(), true);
        visibility.insert("topology_view".to_string(), true);
        visibility.insert("quality_dashboard".to_string(), true);
        visibility.insert("health_monitor".to_string(), true);
        visibility.insert("metrics_panel".to_string(), true);
        visibility.insert("console_output".to_string(), true);
        visibility.insert("status_bar".to_string(), true);
        visibility.insert("toolbar".to_string(), true);

        // Default hidden components
        visibility.insert("llm_debugger".to_string(), false);
        visibility.insert("search_panel".to_string(), false);
        visibility.insert("settings_panel".to_string(), false);
        visibility.insert("plugin_manager".to_string(), false);

        visibility
    }
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            main_window_size: (1200.0, 800.0),
            main_window_pos: (100.0, 100.0),
            panel_sizes: HashMap::new(),
            panel_positions: HashMap::new(),
            splitter_positions: vec![0.25, 0.75],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            dark_mode: true,
            primary_color: [0.2, 0.4, 0.8, 1.0],
            secondary_color: [0.3, 0.3, 0.3, 1.0],
            accent_color: [0.0, 0.8, 0.4, 1.0],
            background_color: [0.1, 0.1, 0.1, 1.0],
            text_color: [0.9, 0.9, 0.9, 1.0],
            font_size: 14.0,
            font_family: "Consolas".to_string(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            auto_save: true,
            auto_analysis: true,
            llm_auto_suggestions: false,
            show_tooltips: true,
            animation_speed: 1.0,
            max_recent_projects: 10,
            analysis_depth: AnalysisDepth::Standard,
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            overall_health: 100.0,
            system_status: SystemStatus::Healthy,
            performance_metrics: PerformanceMetrics::default(),
            resource_usage: ResourceUsage::default(),
            alerts: Vec::new(),
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            analysis_time: 0.0,
            memory_usage: 0.0,
            cpu_usage: 0.0,
            disk_usage: 0.0,
            throughput: 0.0,
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_mb: 0.0,
            cpu_percent: 0.0,
            disk_space_gb: 0.0,
            network_io: 0.0,
            gpu_usage: None,
        }
    }
}