//! CodeHUD TUI - Terminal User Interface optimized for Claude Code consumption
//!
//! This crate provides a terminal-based interface using ratatui that presents
//! CodeHUD analysis results in a structured, actionable format optimized for
//! AI agents and command-line integration.

#![warn(clippy::all, clippy::pedantic)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use codehud_core::{
    models::{AnalysisResult, ViewType},
    extractors::BaseDataExtractor,
};
use codehud_analysis::pipeline::{DirectAnalysisPipeline, AnalysisResult as PipelineAnalysisResult};
use codehud_viz::{VisualizationEngine, VizConfig, ColorScheme as VizColorScheme};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        BarChart, Block, Borders, Clear, Gauge, List, ListItem, ListState,
        Paragraph, Table, Row, Cell, Wrap, Tabs,
    },
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, Stdout},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{ThemeSet, Style as SyntectStyle},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

/// Claude Code optimized TUI application
pub struct CodeHudTui {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    state: AppState,
    analysis_data: Option<AnalysisData>,
    config: TuiConfig,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    viz_engine: VisualizationEngine,
}

/// Application state for navigation and display
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current view being displayed
    pub current_view: ViewType,
    /// List of available views
    pub available_views: Vec<ViewType>,
    /// Current selection index in lists
    pub selected_index: usize,
    /// Navigation history
    pub view_history: Vec<ViewType>,
    /// Filter applied to current view
    pub current_filter: Option<String>,
    /// Show only critical items
    pub show_critical_only: bool,
    /// Sort order (ascending/descending)
    pub sort_ascending: bool,
    /// Currently focused panel
    pub focused_panel: FocusedPanel,
}

/// Panels that can receive focus
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    ViewTabs,
    MainContent,
    FilterBox,
    DetailsPane,
}

/// Aggregated analysis data optimized for TUI display
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisData {
    /// Overall health score (0-100)
    pub health_score: f64,
    /// Total files analyzed
    pub files_analyzed: usize,
    /// Critical issues requiring immediate attention
    pub critical_issues: Vec<CriticalIssue>,
    /// Quality metrics summary
    pub quality_summary: QualitySummary,
    /// Security assessment
    pub security_summary: SecuritySummary,
    /// Top problematic files
    pub problematic_files: Vec<ProblematicFile>,
    /// Dependency insights
    pub dependency_insights: DependencyInsights,
    /// Performance bottlenecks
    pub performance_bottlenecks: Vec<PerformanceBottleneck>,
    /// Raw analysis results for detailed views
    pub raw_data: HashMap<ViewType, Value>,
    /// Analysis timestamp
    pub timestamp: DateTime<Utc>,
}

/// Critical issue requiring immediate attention
#[derive(Debug, Clone, Serialize)]
pub struct CriticalIssue {
    pub severity: Severity,
    pub category: String,
    pub description: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub impact_score: f64,
    pub fix_suggestion: Option<String>,
}

/// Quality metrics summary
#[derive(Debug, Clone, Serialize)]
pub struct QualitySummary {
    pub average_maintainability: f64,
    pub total_issues: usize,
    pub issues_by_severity: HashMap<String, usize>,
    pub worst_files: Vec<(String, f64)>,
    pub complexity_distribution: Vec<(String, usize)>,
}

/// Security assessment summary
#[derive(Debug, Clone, Serialize)]
pub struct SecuritySummary {
    pub risk_level: RiskLevel,
    pub total_vulnerabilities: usize,
    pub critical_vulnerabilities: usize,
    pub rust_specific_issues: usize,
    pub top_security_files: Vec<String>,
}

/// Problematic file with actionable metrics
#[derive(Debug, Clone, Serialize)]
pub struct ProblematicFile {
    pub path: String,
    pub maintainability_score: f64,
    pub issues_count: usize,
    pub complexity_score: f64,
    pub priority_rank: usize,
    pub recommended_actions: Vec<String>,
}

/// Dependency analysis insights
#[derive(Debug, Clone, Serialize)]
pub struct DependencyInsights {
    pub total_dependencies: usize,
    pub circular_dependencies: usize,
    pub highly_coupled_files: Vec<(String, usize)>,
    pub dependency_health_score: f64,
    pub external_risk_assessment: String,
}

/// Performance bottleneck identification
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceBottleneck {
    pub file_path: String,
    pub function_name: Option<String>,
    pub bottleneck_type: String,
    pub severity_score: f64,
    pub estimated_impact: String,
}

/// Severity levels for issues
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Risk assessment levels
#[derive(Debug, Clone, Serialize)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Minimal,
}

/// TUI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Maximum items to display in lists
    pub max_list_items: usize,
    /// Show file paths relative or absolute
    pub show_relative_paths: bool,
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Auto-refresh interval in seconds
    pub auto_refresh_interval: Option<u64>,
    /// Default view on startup
    pub default_view: ViewType,
    /// Enable syntax highlighting in code views
    pub syntax_highlighting: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            max_list_items: 50,
            show_relative_paths: true,
            color_scheme: ColorScheme::Dark,
            auto_refresh_interval: None,
            default_view: ViewType::Quality,
            syntax_highlighting: true,
        }
    }
}

/// Color schemes for TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorScheme {
    Dark,
    Light,
    HighContrast,
    Claude, // Optimized for Claude Code consumption
}

impl CodeHudTui {
    /// Create a new TUI instance for interactive use
    pub fn new() -> Result<Self> {
        // Check if we're in a proper terminal environment
        if false && !atty::is(atty::Stream::Stdout) {
            return Err(anyhow::anyhow!("Not running in a terminal environment. Use export mode instead: codehud-tui export <path>"));
        }

        enable_raw_mode().map_err(|e| anyhow::anyhow!("Failed to enable raw mode: {}", e))?;
        let mut stdout = io::stdout();

        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| anyhow::anyhow!("Failed to initialize terminal: {}", e))?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| anyhow::anyhow!("Failed to create terminal: {}", e))?;

        let state = AppState {
            current_view: ViewType::Quality,
            available_views: vec![
                ViewType::Quality,
                ViewType::Quality,
                ViewType::Security,
                ViewType::Topology,
                ViewType::Dependencies,
                ViewType::Performance,
                ViewType::IssuesInspection,
            ],
            selected_index: 0,
            view_history: vec![],
            current_filter: None,
            show_critical_only: false,
            sort_ascending: false,
            focused_panel: FocusedPanel::ViewTabs,
        };

        let config = TuiConfig::default();
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        let viz_engine = VisualizationEngine::new();

        Ok(Self {
            terminal: Some(terminal),
            state,
            analysis_data: None,
            config,
            syntax_set,
            theme_set,
            viz_engine,
        })
    }

    /// Create a new TUI instance for headless data processing (no terminal)
    pub fn new_headless() -> Result<Self> {
        let state = AppState {
            current_view: ViewType::Quality,
            available_views: vec![
                ViewType::Quality,
                ViewType::Security,
                ViewType::Topology,
                ViewType::Dependencies,
                ViewType::Performance,
                ViewType::IssuesInspection,
            ],
            selected_index: 0,
            view_history: vec![],
            current_filter: None,
            show_critical_only: false,
            sort_ascending: false,
            focused_panel: FocusedPanel::ViewTabs,
        };

        let config = TuiConfig::default();
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        let viz_engine = VisualizationEngine::new();

        Ok(Self {
            terminal: None,
            state,
            analysis_data: None,
            config,
            syntax_set,
            theme_set,
            viz_engine,
        })
    }

    /// Load analysis data from a codebase path
    pub async fn load_analysis(&mut self, codebase_path: &Path) -> Result<()> {
        let config = codehud_core::CoreConfig::default();
        let pipeline = DirectAnalysisPipeline::new(codebase_path, config)?;

        // Run comprehensive analysis
        let analysis_result = pipeline.analyze().await?;

        // Process and aggregate data into TUI-optimized format
        self.analysis_data = Some(self.process_analysis_result(&analysis_result)?);

        Ok(())
    }

    /// Process analysis result into TUI-optimized format
    fn process_analysis_result(
        &self,
        analysis_result: &PipelineAnalysisResult,
    ) -> Result<AnalysisData> {
        // Extract data from views
        let quality_data = analysis_result.views.get("quality").unwrap_or(&serde_json::Value::Null);
        let security_data = analysis_result.views.get("security").unwrap_or(&serde_json::Value::Null);
        let topology_data = analysis_result.views.get("topology").unwrap_or(&serde_json::Value::Null);
        let deps_data = analysis_result.views.get("dependencies").unwrap_or(&serde_json::Value::Null);

        // Extract critical issues across all analyses
        let critical_issues = self.extract_critical_issues(quality_data, security_data)?;

        // Use existing health score from analysis
        let health_score = analysis_result.health_score.overall_score;

        // Process quality summary
        let quality_summary = self.process_quality_summary(quality_data)?;

        // Process security summary
        let security_summary = self.process_security_summary(security_data)?;

        // Identify problematic files
        let problematic_files = self.identify_problematic_files(quality_data, security_data)?;

        // Process dependency insights
        let dependency_insights = self.process_dependency_insights(deps_data)?;

        // Extract performance bottlenecks
        let performance_bottlenecks = self.extract_performance_bottlenecks(quality_data)?;

        let files_analyzed = analysis_result.metadata.total_files_analyzed;

        let mut raw_data = HashMap::new();
        for (view_name, view_data) in &analysis_result.views {
            if let Ok(view_type) = view_name.parse::<ViewType>() {
                raw_data.insert(view_type, view_data.clone());
            }
        }

        Ok(AnalysisData {
            health_score,
            files_analyzed,
            critical_issues,
            quality_summary,
            security_summary,
            problematic_files,
            dependency_insights,
            performance_bottlenecks,
            raw_data,
            timestamp: analysis_result.timestamp,
        })
    }

    /// Extract critical issues that need immediate attention
    fn extract_critical_issues(&self, quality_data: &Value, security_data: &Value) -> Result<Vec<CriticalIssue>> {
        let mut issues = Vec::new();

        // Extract critical quality issues
        if let Some(file_metrics) = quality_data.get("file_metrics").and_then(|v| v.as_array()) {
            for file_metric in file_metrics {
                if let Some(file_path) = file_metric.get("file").and_then(|v| v.as_str()) {
                    let maintainability = file_metric.get("maintainability_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100.0);

                    // Critical threshold for maintainability
                    if maintainability < 20.0 {
                        issues.push(CriticalIssue {
                            severity: Severity::Critical,
                            category: "Quality".to_string(),
                            description: format!("Extremely low maintainability score: {:.1}", maintainability),
                            file_path: file_path.to_string(),
                            line_number: None,
                            impact_score: 100.0 - maintainability,
                            fix_suggestion: Some("Consider refactoring this file to improve maintainability".to_string()),
                        });
                    }
                }
            }
        }

        // Extract critical security issues
        if let Some(rust_results) = security_data.get("rust_security_results").and_then(|v| v.get("issues")).and_then(|v| v.as_array()) {
            for issue in rust_results {
                if let (Some(severity), Some(description), Some(file_path)) = (
                    issue.get("severity").and_then(|v| v.as_str()),
                    issue.get("description").and_then(|v| v.as_str()),
                    issue.get("file").and_then(|v| v.as_str()),
                ) {
                    if severity == "medium" || severity == "high" {
                        issues.push(CriticalIssue {
                            severity: if severity == "high" { Severity::High } else { Severity::Medium },
                            category: "Security".to_string(),
                            description: description.to_string(),
                            file_path: file_path.to_string(),
                            line_number: issue.get("line").and_then(|v| v.as_u64()).map(|l| l as usize),
                            impact_score: if severity == "high" { 80.0 } else { 50.0 },
                            fix_suggestion: Some("Review and improve error handling".to_string()),
                        });
                    }
                }
            }
        }

        // Sort by impact score (highest first)
        issues.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(issues)
    }

    /// Calculate overall health score from multiple analyses
    fn calculate_health_score(&self, quality_data: &Value, security_data: &Value) -> Result<f64> {
        let mut total_score = 0.0;
        let mut weight_sum = 0.0;

        // Quality score (40% weight)
        if let Some(file_metrics) = quality_data.get("file_metrics").and_then(|v| v.as_array()) {
            if !file_metrics.is_empty() {
                let avg_maintainability: f64 = file_metrics.iter()
                    .filter_map(|f| f.get("maintainability_score").and_then(|v| v.as_f64()))
                    .sum::<f64>() / file_metrics.len() as f64;
                total_score += avg_maintainability * 0.4;
                weight_sum += 0.4;
            }
        }

        // Security score (35% weight)
        if let Some(risk_assessment) = security_data.get("risk_assessment") {
            let security_score = risk_assessment.get("score").and_then(|v| v.as_f64()).unwrap_or(50.0);
            total_score += security_score * 0.35;
            weight_sum += 0.35;
        }

        // Default to average if we have partial data
        if weight_sum > 0.0 {
            Ok(total_score / weight_sum)
        } else {
            Ok(50.0) // Default neutral score
        }
    }

    /// Process quality data into summary format
    fn process_quality_summary(&self, quality_data: &Value) -> Result<QualitySummary> {
        let mut total_issues = 0;
        let mut issues_by_severity = HashMap::new();
        let mut maintainability_scores = Vec::new();
        let mut worst_files = Vec::new();

        if let Some(file_metrics) = quality_data.get("file_metrics").and_then(|v| v.as_array()) {
            for file_metric in file_metrics {
                if let Some(file_path) = file_metric.get("file").and_then(|v| v.as_str()) {
                    let maintainability = file_metric.get("maintainability_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100.0);

                    maintainability_scores.push(maintainability);

                    let issues_count = file_metric.get("issues_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    total_issues += issues_count;

                    if maintainability < 50.0 {
                        worst_files.push((file_path.to_string(), maintainability));
                    }
                }
            }
        }

        // Sort worst files by maintainability score
        worst_files.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        worst_files.truncate(10); // Top 10 worst files

        let average_maintainability = if !maintainability_scores.is_empty() {
            maintainability_scores.iter().sum::<f64>() / maintainability_scores.len() as f64
        } else {
            100.0
        };

        // Mock complexity distribution for now
        let complexity_distribution = vec![
            ("Low (0-5)".to_string(), 0),
            ("Medium (6-15)".to_string(), 0),
            ("High (16+)".to_string(), 0),
        ];

        Ok(QualitySummary {
            average_maintainability,
            total_issues,
            issues_by_severity,
            worst_files,
            complexity_distribution,
        })
    }

    /// Process security data into summary format
    fn process_security_summary(&self, security_data: &Value) -> Result<SecuritySummary> {
        let risk_level = security_data
            .get("risk_assessment")
            .and_then(|v| v.get("level"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let total_vulnerabilities = security_data
            .get("risk_assessment")
            .and_then(|v| v.get("total_findings"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let critical_vulnerabilities = security_data
            .get("risk_assessment")
            .and_then(|v| v.get("high_severity"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let rust_specific_issues = security_data
            .get("rust_security_results")
            .and_then(|v| v.get("issues"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        let risk_level_enum = match risk_level {
            "critical" => RiskLevel::Critical,
            "high" => RiskLevel::High,
            "medium" => RiskLevel::Medium,
            "low" => RiskLevel::Low,
            _ => RiskLevel::Minimal,
        };

        Ok(SecuritySummary {
            risk_level: risk_level_enum,
            total_vulnerabilities,
            critical_vulnerabilities,
            rust_specific_issues,
            top_security_files: Vec::new(), // TODO: Extract from data
        })
    }

    /// Identify most problematic files across all analyses
    fn identify_problematic_files(&self, quality_data: &Value, security_data: &Value) -> Result<Vec<ProblematicFile>> {
        let mut files = Vec::new();

        if let Some(file_metrics) = quality_data.get("file_metrics").and_then(|v| v.as_array()) {
            for file_metric in file_metrics {
                if let Some(file_path) = file_metric.get("file").and_then(|v| v.as_str()) {
                    let maintainability_score = file_metric.get("maintainability_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100.0);

                    let issues_count = file_metric.get("issues_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    let complexity_score = file_metric.get("complexity_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    // Calculate priority rank based on multiple factors
                    let priority_score = (100.0 - maintainability_score) +
                                       (issues_count as f64 * 5.0) +
                                       complexity_score;

                    let mut recommended_actions = Vec::new();
                    if maintainability_score < 50.0 {
                        recommended_actions.push("Refactor for better maintainability".to_string());
                    }
                    if issues_count > 5 {
                        recommended_actions.push("Address code quality issues".to_string());
                    }
                    if complexity_score > 10.0 {
                        recommended_actions.push("Reduce cyclomatic complexity".to_string());
                    }

                    files.push(ProblematicFile {
                        path: file_path.to_string(),
                        maintainability_score,
                        issues_count,
                        complexity_score,
                        priority_rank: 0, // Will be set after sorting
                        recommended_actions,
                    });
                }
            }
        }

        // Sort by priority score and assign ranks
        files.sort_by(|a, b| {
            let score_a = (100.0 - a.maintainability_score) + (a.issues_count as f64 * 5.0) + a.complexity_score;
            let score_b = (100.0 - b.maintainability_score) + (b.issues_count as f64 * 5.0) + b.complexity_score;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (index, file) in files.iter_mut().enumerate() {
            file.priority_rank = index + 1;
        }

        // Return top 20 most problematic files
        files.truncate(20);

        Ok(files)
    }

    /// Process dependency analysis into insights
    fn process_dependency_insights(&self, deps_data: &Value) -> Result<DependencyInsights> {
        // Mock implementation - would need actual dependency data structure
        Ok(DependencyInsights {
            total_dependencies: 0,
            circular_dependencies: 0,
            highly_coupled_files: Vec::new(),
            dependency_health_score: 80.0,
            external_risk_assessment: "Low".to_string(),
        })
    }

    /// Extract performance bottlenecks from analysis data
    fn extract_performance_bottlenecks(&self, quality_data: &Value) -> Result<Vec<PerformanceBottleneck>> {
        // Mock implementation - would extract from actual performance analysis
        Ok(Vec::new())
    }

    /// Run the main TUI event loop
    pub fn run(&mut self) -> Result<()> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            self.draw()?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Tab => self.next_panel(),
                            KeyCode::BackTab => self.previous_panel(),
                            KeyCode::Left | KeyCode::Char('h') => self.handle_left(),
                            KeyCode::Right | KeyCode::Char('l') => self.handle_right(),
                            KeyCode::Up | KeyCode::Char('k') => self.handle_up(),
                            KeyCode::Down | KeyCode::Char('j') => self.handle_down(),
                            KeyCode::Enter => self.handle_enter(),
                            KeyCode::Char('f') => self.toggle_filter(),
                            KeyCode::Char('c') => self.toggle_critical_only(),
                            KeyCode::Char('s') => self.toggle_sort_order(),
                            KeyCode::Char('r') => self.refresh_data()?,
                            KeyCode::Char(c @ '1'..='7') => {
                                let index = (c as u8 - b'1') as usize;
                                if index < self.state.available_views.len() {
                                    self.state.current_view = self.state.available_views[index].clone();
                                    self.state.selected_index = 0;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    /// Draw the TUI interface
    fn draw(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.terminal {
            let analysis_data = self.analysis_data.clone();
            let state = self.state.clone();
            let config = self.config.clone();

            terminal.draw(move |f| {
                let size = f.size();

                // Main layout: Header + Body + Footer
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Header
                        Constraint::Min(0),    // Body
                        Constraint::Length(3), // Footer
                    ])
                    .split(size);

                // Render components with captured data
                CodeHudTui::render_header_static(f, chunks[0], &state);
                CodeHudTui::render_main_content_static(f, chunks[1], &analysis_data, &state, &config);
                CodeHudTui::render_footer_static(f, chunks[2]);
            })?;
        }
        Ok(())
    }

    /// Render header with view tabs
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let tab_titles: Vec<&str> = self.state.available_views
            .iter()
            .map(|v| match v {
                ViewType::Quality => "Quality",
                ViewType::Security => "Security",
                ViewType::Topology => "Topology",
                ViewType::Dependencies => "Dependencies",
                ViewType::Performance => "Performance",
                ViewType::IssuesInspection => "Issues",
                _ => "Other",
            })
            .collect();

        let current_index = self.state.available_views
            .iter()
            .position(|v| v == &self.state.current_view)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("CodeHUD Analysis Views")
                    .border_style(if self.state.focused_panel == FocusedPanel::ViewTabs {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    })
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(current_index);

        f.render_widget(tabs, area);
    }

    /// Render main content area based on current view
    fn render_main_content(&self, f: &mut Frame, area: Rect) {
        match self.analysis_data {
            Some(ref data) => {
                match self.state.current_view {
                    ViewType::Quality => self.render_quality_view(f, area, data),
                    ViewType::Security => self.render_security_view(f, area, data),
                    ViewType::Topology => self.render_topology_view(f, area, data),
                    ViewType::Dependencies => self.render_dependencies_view(f, area, data),
                    ViewType::Performance => self.render_performance_view(f, area, data),
                    ViewType::IssuesInspection => self.render_issues_view(f, area, data),
                    _ => self.render_placeholder_view(f, area),
                }
            }
            None => self.render_no_data_view(f, area),
        }
    }


    /// Render quality analysis view
    fn render_quality_view(&self, f: &mut Frame, area: Rect, data: &AnalysisData) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // Worst files list
        let worst_files_items: Vec<ListItem> = data.quality_summary.worst_files
            .iter()
            .enumerate()
            .map(|(i, (file_path, score))| {
                let color = if *score < 20.0 { Color::Red }
                           else if *score < 50.0 { Color::Yellow }
                           else { Color::Green };

                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(file_path);

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:>3}: ", i + 1),
                            Style::default().fg(Color::Gray)
                        ),
                        Span::styled(
                            format!("{:>5.1}% ", score),
                            Style::default().fg(color).add_modifier(Modifier::BOLD)
                        ),
                        Span::styled(file_name, Style::default().fg(Color::White)),
                    ])
                ])
            })
            .collect();

        let worst_files_list = List::new(worst_files_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Worst Quality Files")
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut list_state = ListState::default();
        if self.state.focused_panel == FocusedPanel::MainContent {
            list_state.select(Some(self.state.selected_index.min(data.quality_summary.worst_files.len().saturating_sub(1))));
        }
        f.render_stateful_widget(worst_files_list, chunks[0], &mut list_state);

        // Quality metrics summary
        let metrics_text = vec![
            Line::from(format!("Average Maintainability: {:.1}%", data.quality_summary.average_maintainability)),
            Line::from(format!("Total Issues: {}", data.quality_summary.total_issues)),
            Line::from(""),
            Line::from("Issues by Severity:"),
        ];

        let quality_metrics = Paragraph::new(metrics_text)
            .block(Block::default().borders(Borders::ALL).title("Quality Metrics"))
            .wrap(Wrap { trim: true });
        f.render_widget(quality_metrics, chunks[1]);
    }

    /// Render security analysis view
    fn render_security_view(&self, f: &mut Frame, area: Rect, data: &AnalysisData) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Security overview
                Constraint::Min(0),     // Security issues
            ])
            .split(area);

        // Security overview
        let security_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[0]);

        let risk_color = match data.security_summary.risk_level {
            RiskLevel::Critical => Color::Red,
            RiskLevel::High => Color::LightRed,
            RiskLevel::Medium => Color::Yellow,
            RiskLevel::Low => Color::Green,
            RiskLevel::Minimal => Color::Green,
        };

        let security_overview = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Risk Level: "),
                Span::styled(
                    format!("{:?}", data.security_summary.risk_level),
                    Style::default().fg(risk_color).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(format!("Total Vulnerabilities: {}", data.security_summary.total_vulnerabilities)),
            Line::from(format!("Critical Issues: {}", data.security_summary.critical_vulnerabilities)),
            Line::from(format!("Rust-Specific Issues: {}", data.security_summary.rust_specific_issues)),
        ])
        .block(Block::default().borders(Borders::ALL).title("Security Overview"))
        .wrap(Wrap { trim: true });
        f.render_widget(security_overview, security_chunks[0]);

        // Security recommendations
        let recommendations = Paragraph::new(vec![
            Line::from("Recommendations:"),
            Line::from("• Review unwrap() calls"),
            Line::from("• Improve error handling"),
            Line::from("• Add input validation"),
            Line::from("• Use secure defaults"),
        ])
        .block(Block::default().borders(Borders::ALL).title("Recommendations"))
        .wrap(Wrap { trim: true });
        f.render_widget(recommendations, security_chunks[1]);

        // Security issues list (filtered to security issues only)
        let security_issues: Vec<&CriticalIssue> = data.critical_issues
            .iter()
            .filter(|issue| issue.category == "Security")
            .collect();

        let security_items: Vec<ListItem> = security_issues
            .iter()
            .enumerate()
            .map(|(i, issue)| {
                let severity_color = match issue.severity {
                    Severity::Critical => Color::Red,
                    Severity::High => Color::LightRed,
                    Severity::Medium => Color::Yellow,
                    Severity::Low => Color::Green,
                    Severity::Info => Color::Cyan,
                };

                let file_name = Path::new(&issue.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&issue.file_path);

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:?}", issue.severity),
                            Style::default().fg(severity_color).add_modifier(Modifier::BOLD)
                        ),
                        Span::raw(" | "),
                        Span::styled(file_name, Style::default().fg(Color::Cyan)),
                        if let Some(line) = issue.line_number {
                            Span::raw(format!(":{}", line))
                        } else {
                            Span::raw("")
                        },
                    ]),
                    Line::from(format!("  {}", issue.description)),
                ])
            })
            .collect();

        let security_list = List::new(security_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Security Issues")
                    .border_style(if self.state.focused_panel == FocusedPanel::MainContent {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    })
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut list_state = ListState::default();
        if self.state.focused_panel == FocusedPanel::MainContent {
            list_state.select(Some(self.state.selected_index.min(security_issues.len().saturating_sub(1))));
        }
        f.render_stateful_widget(security_list, chunks[1], &mut list_state);
    }

    /// Render topology view
    fn render_topology_view(&self, f: &mut Frame, area: Rect, _data: &AnalysisData) {
        let placeholder = Paragraph::new("Topology visualization - Coming soon")
            .block(Block::default().borders(Borders::ALL).title("Code Topology"))
            .wrap(Wrap { trim: true });
        f.render_widget(placeholder, area);
    }

    /// Render dependencies view
    fn render_dependencies_view(&self, f: &mut Frame, area: Rect, data: &AnalysisData) {
        let deps_info = Paragraph::new(vec![
            Line::from(format!("Total Dependencies: {}", data.dependency_insights.total_dependencies)),
            Line::from(format!("Circular Dependencies: {}", data.dependency_insights.circular_dependencies)),
            Line::from(format!("Health Score: {:.1}%", data.dependency_insights.dependency_health_score)),
            Line::from(format!("External Risk: {}", data.dependency_insights.external_risk_assessment)),
        ])
        .block(Block::default().borders(Borders::ALL).title("Dependency Analysis"))
        .wrap(Wrap { trim: true });
        f.render_widget(deps_info, area);
    }

    /// Render performance view
    fn render_performance_view(&self, f: &mut Frame, area: Rect, data: &AnalysisData) {
        let performance_info = Paragraph::new(vec![
            Line::from(format!("Performance Bottlenecks: {}", data.performance_bottlenecks.len())),
            Line::from("Performance analysis coming soon..."),
        ])
        .block(Block::default().borders(Borders::ALL).title("Performance Analysis"))
        .wrap(Wrap { trim: true });
        f.render_widget(performance_info, area);
    }

    /// Render issues view
    fn render_issues_view(&self, f: &mut Frame, area: Rect, data: &AnalysisData) {
        // Show all critical issues in a detailed list
        let issues_items: Vec<ListItem> = data.critical_issues
            .iter()
            .enumerate()
            .map(|(i, issue)| {
                let severity_color = match issue.severity {
                    Severity::Critical => Color::Red,
                    Severity::High => Color::LightRed,
                    Severity::Medium => Color::Yellow,
                    Severity::Low => Color::Green,
                    Severity::Info => Color::Cyan,
                };

                let file_name = Path::new(&issue.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&issue.file_path);

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:>3}: ", i + 1),
                            Style::default().fg(Color::Gray)
                        ),
                        Span::styled(
                            format!("{:?}", issue.severity),
                            Style::default().fg(severity_color).add_modifier(Modifier::BOLD)
                        ),
                        Span::raw(" | "),
                        Span::styled(&issue.category, Style::default().fg(Color::Magenta)),
                        Span::raw(" | "),
                        Span::styled(file_name, Style::default().fg(Color::Cyan)),
                        if let Some(line) = issue.line_number {
                            Span::raw(format!(":{}", line))
                        } else {
                            Span::raw("")
                        },
                    ]),
                    Line::from(format!("     {}", issue.description)),
                ];

                if let Some(suggestion) = &issue.fix_suggestion {
                    lines.push(Line::from(vec![
                        Span::raw("     Fix: "),
                        Span::styled(suggestion, Style::default().fg(Color::Green)),
                    ]));
                }

                ListItem::new(lines)
            })
            .collect();

        let issues_list = List::new(issues_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("All Issues (Detailed View)")
                    .border_style(if self.state.focused_panel == FocusedPanel::MainContent {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    })
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut list_state = ListState::default();
        if self.state.focused_panel == FocusedPanel::MainContent {
            list_state.select(Some(self.state.selected_index.min(data.critical_issues.len().saturating_sub(1))));
        }
        f.render_stateful_widget(issues_list, area, &mut list_state);
    }

    /// Render placeholder for unimplemented views
    fn render_placeholder_view(&self, f: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("View not yet implemented")
            .block(Block::default().borders(Borders::ALL).title("Placeholder"))
            .wrap(Wrap { trim: true });
        f.render_widget(placeholder, area);
    }

    /// Render view when no analysis data is available
    fn render_no_data_view(&self, f: &mut Frame, area: Rect) {
        let no_data = Paragraph::new(vec![
            Line::from("No analysis data loaded"),
            Line::from(""),
            Line::from("Press 'r' to refresh or load analysis data"),
            Line::from("Use CLI: codehud-tui <codebase_path>"),
        ])
        .block(Block::default().borders(Borders::ALL).title("CodeHUD TUI"))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        f.render_widget(no_data, area);
    }

    /// Render footer with help information
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":quit "),
                Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":panels "),
                Span::styled("↑↓/jk", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":navigate "),
                Span::styled("1-7", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":views "),
                Span::styled("r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":refresh "),
                Span::styled("c", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":critical "),
                Span::styled("s", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":sort"),
            ])
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .wrap(Wrap { trim: true });
        f.render_widget(help, area);
    }

    /// Get color style based on health score
    fn get_health_color(&self, score: f64) -> Style {
        let color = if score >= 80.0 { Color::Green }
                   else if score >= 60.0 { Color::Yellow }
                   else if score >= 40.0 { Color::LightRed }
                   else { Color::Red };
        Style::default().fg(color)
    }

    /// Navigation helpers
    fn next_panel(&mut self) {
        self.state.focused_panel = match self.state.focused_panel {
            FocusedPanel::ViewTabs => FocusedPanel::MainContent,
            FocusedPanel::MainContent => FocusedPanel::FilterBox,
            FocusedPanel::FilterBox => FocusedPanel::DetailsPane,
            FocusedPanel::DetailsPane => FocusedPanel::ViewTabs,
        };
    }

    fn previous_panel(&mut self) {
        self.state.focused_panel = match self.state.focused_panel {
            FocusedPanel::ViewTabs => FocusedPanel::DetailsPane,
            FocusedPanel::MainContent => FocusedPanel::ViewTabs,
            FocusedPanel::FilterBox => FocusedPanel::MainContent,
            FocusedPanel::DetailsPane => FocusedPanel::FilterBox,
        };
    }

    fn handle_left(&mut self) {
        if self.state.focused_panel == FocusedPanel::ViewTabs {
            let current_index = self.state.available_views
                .iter()
                .position(|v| v == &self.state.current_view)
                .unwrap_or(0);

            let new_index = if current_index == 0 {
                self.state.available_views.len() - 1
            } else {
                current_index - 1
            };

            self.state.current_view = self.state.available_views[new_index].clone();
            self.state.selected_index = 0;
        }
    }

    fn handle_right(&mut self) {
        if self.state.focused_panel == FocusedPanel::ViewTabs {
            let current_index = self.state.available_views
                .iter()
                .position(|v| v == &self.state.current_view)
                .unwrap_or(0);

            let new_index = (current_index + 1) % self.state.available_views.len();

            self.state.current_view = self.state.available_views[new_index].clone();
            self.state.selected_index = 0;
        }
    }

    fn handle_up(&mut self) {
        if self.state.focused_panel == FocusedPanel::MainContent {
            self.state.selected_index = self.state.selected_index.saturating_sub(1);
        }
    }

    fn handle_down(&mut self) {
        if self.state.focused_panel == FocusedPanel::MainContent {
            if let Some(data) = &self.analysis_data {
                let max_index = match self.state.current_view {
                    ViewType::Quality => data.quality_summary.worst_files.len().saturating_sub(1),
                    ViewType::Security => data.critical_issues.iter().filter(|i| i.category == "Security").count().saturating_sub(1),
                    ViewType::IssuesInspection => data.critical_issues.len().saturating_sub(1),
                    _ => 0,
                };
                self.state.selected_index = (self.state.selected_index + 1).min(max_index);
            }
        }
    }

    fn handle_enter(&mut self) {
        // TODO: Implement detail view for selected item
    }

    fn toggle_filter(&mut self) {
        // TODO: Implement filtering
    }

    fn toggle_critical_only(&mut self) {
        self.state.show_critical_only = !self.state.show_critical_only;
    }

    fn toggle_sort_order(&mut self) {
        self.state.sort_ascending = !self.state.sort_ascending;
    }

    fn refresh_data(&mut self) -> Result<()> {
        // TODO: Implement data refresh
        Ok(())
    }

    /// Get the current analysis data (for export functionality)
    pub fn get_analysis_data(&self) -> Option<&AnalysisData> {
        self.analysis_data.as_ref()
    }

    /// Static render methods for use in closures
    fn render_header_static(f: &mut Frame, area: Rect, state: &AppState) {
        let tab_titles: Vec<&str> = state.available_views
            .iter()
            .map(|v| match v {
                ViewType::Quality => "Quality",
                ViewType::Security => "Security",
                ViewType::Topology => "Topology",
                ViewType::Dependencies => "Dependencies",
                ViewType::Performance => "Performance",
                ViewType::IssuesInspection => "Issues",
                _ => "Other",
            })
            .collect();

        let current_index = state.available_views
            .iter()
            .position(|v| v == &state.current_view)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("CodeHUD Analysis Views")
                    .border_style(if state.focused_panel == FocusedPanel::ViewTabs {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    })
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(current_index);

        f.render_widget(tabs, area);
    }


    fn render_main_content_static(f: &mut Frame, area: Rect, analysis_data: &Option<AnalysisData>, state: &AppState, _config: &TuiConfig) {
        match analysis_data {
            Some(data) => {
                // Create visualization engine for rendering
                let viz_engine = VisualizationEngine::new();

                // Convert data and use viz engine
                if let Ok(analysis_result) = Self::convert_to_analysis_result(data) {
                    if let Ok(view) = viz_engine.generate_view(state.current_view.clone(), &analysis_result) {
                        // Export visualization to file for inspection
                        let _ = Self::export_visualization(&view, &state.current_view);

                        viz_engine.render_to_terminal(f, &view);
                        return;
                    }
                }

                // Fallback to old static methods if viz engine fails
                match state.current_view {
                    ViewType::Quality => Self::render_quality_view_static(f, area, data, state),
                    ViewType::Security => Self::render_security_view_static(f, area, data, state),
                    ViewType::IssuesInspection => Self::render_issues_view_static(f, area, data, state),
                    _ => Self::render_placeholder_view_static(f, area),
                }
            }
            None => Self::render_no_data_view_static(f, area),
        }
    }

    fn render_footer_static(f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":quit "),
                Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":panels "),
                Span::styled("↑↓/jk", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":navigate "),
                Span::styled("1-7", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":views "),
                Span::styled("r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":refresh "),
                Span::styled("c", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":critical "),
                Span::styled("s", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(":sort"),
            ])
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .wrap(Wrap { trim: true });
        f.render_widget(help, area);
    }

    fn render_quality_view_static(f: &mut Frame, area: Rect, data: &AnalysisData, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // Worst files list
        let worst_files_items: Vec<ListItem> = data.quality_summary.worst_files
            .iter()
            .enumerate()
            .map(|(i, (file_path, score))| {
                let color = if *score < 20.0 { Color::Red }
                           else if *score < 50.0 { Color::Yellow }
                           else { Color::Green };

                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(file_path);

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:>3}: ", i + 1),
                            Style::default().fg(Color::Gray)
                        ),
                        Span::styled(
                            format!("{:>5.1}% ", score),
                            Style::default().fg(color).add_modifier(Modifier::BOLD)
                        ),
                        Span::styled(file_name, Style::default().fg(Color::White)),
                    ])
                ])
            })
            .collect();

        let worst_files_list = List::new(worst_files_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Worst Quality Files")
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(worst_files_list, chunks[0]);

        // Quality metrics summary
        let metrics_text = vec![
            Line::from(format!("Average Maintainability: {:.1}%", data.quality_summary.average_maintainability)),
            Line::from(format!("Total Issues: {}", data.quality_summary.total_issues)),
            Line::from(format!("Files Analyzed: {}", data.files_analyzed)),
            Line::from(format!("Health Score: {:.1}%", data.health_score)),
        ];

        let quality_metrics = Paragraph::new(metrics_text)
            .block(Block::default().borders(Borders::ALL).title("Quality Metrics"))
            .wrap(Wrap { trim: true });
        f.render_widget(quality_metrics, chunks[1]);
    }

    fn render_security_view_static(f: &mut Frame, area: Rect, data: &AnalysisData, _state: &AppState) {
        let security_info = Paragraph::new(vec![
            Line::from(format!("Risk Level: {:?}", data.security_summary.risk_level)),
            Line::from(format!("Total Vulnerabilities: {}", data.security_summary.total_vulnerabilities)),
            Line::from(format!("Critical Issues: {}", data.security_summary.critical_vulnerabilities)),
            Line::from(format!("Rust Issues: {}", data.security_summary.rust_specific_issues)),
        ])
        .block(Block::default().borders(Borders::ALL).title("Security Analysis"))
        .wrap(Wrap { trim: true });
        f.render_widget(security_info, area);
    }

    /// Render visualization charts using the viz engine
    pub fn render_viz_view(&self, f: &mut Frame, area: Rect, view_type: ViewType, analysis_result: &AnalysisResult) -> Result<()> {
        // Use the visualization engine to generate a view
        let viz_view = self.viz_engine.generate_view(view_type, analysis_result)?;

        // Render the visualization view in the TUI
        self.viz_engine.render_to_terminal(f, &viz_view);
        Ok(())
    }

    /// Get visualization engine reference
    pub fn get_viz_engine(&self) -> &VisualizationEngine {
        &self.viz_engine
    }

    /// Convert AnalysisData to AnalysisResult for viz engine
    fn convert_to_analysis_result(data: &AnalysisData) -> Result<AnalysisResult> {
        let mut analysis_result = AnalysisResult::new("current_analysis".to_string());
        analysis_result.health_score = data.health_score;
        analysis_result.files_analyzed = data.files_analyzed as usize;
        analysis_result.analysis_duration = 1.0; // Default duration

        // Convert the analysis data to JSON and store in view data
        let analysis_value = serde_json::to_value(data)?;
        analysis_result.set_view_data("analysis".to_string(), analysis_value);

        Ok(analysis_result)
    }

    /// Fallback rendering when viz engine fails
    fn render_fallback_view_static(f: &mut Frame, area: Rect, view_type: &ViewType) {
        let fallback_text = match view_type {
            ViewType::Quality => "Quality analysis view (visualization engine unavailable)",
            ViewType::Security => "Security analysis view (visualization engine unavailable)",
            ViewType::Topology => "Topology view (visualization engine unavailable)",
            ViewType::Dependencies => "Dependencies view (visualization engine unavailable)",
            ViewType::Performance => "Performance view (visualization engine unavailable)",
            ViewType::IssuesInspection => "Issues view (visualization engine unavailable)",
            _ => "View unavailable",
        };

        let paragraph = Paragraph::new(fallback_text)
            .block(Block::default().borders(Borders::ALL).title("CodeHUD Analysis"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Export visualization to file for inspection
    fn export_visualization(view: &codehud_viz::RenderableView, view_type: &ViewType) -> Result<()> {
        use std::fs;

        // Create output directory
        fs::create_dir_all("tui_visualizations")?;

        // Convert view to displayable format
        let view_name = match view_type {
            ViewType::Quality => "quality",
            ViewType::Security => "security",
            ViewType::Topology => "topology",
            ViewType::Dependencies => "dependencies",
            ViewType::Performance => "performance",
            ViewType::IssuesInspection => "issues",
            _ => "other",
        };

        // Export as JSON for inspection
        let json_output = serde_json::to_string_pretty(view)?;
        let json_file = format!("tui_visualizations/{}_view.json", view_name);
        fs::write(&json_file, json_output)?;

        // Export as text summary
        let text_summary = Self::render_view_as_text(view);
        let text_file = format!("tui_visualizations/{}_view.txt", view_name);
        fs::write(&text_file, text_summary)?;

        println!("📊 Exported {} visualization to: {} and {}", view_name, json_file, text_file);
        Ok(())
    }

    /// Convert visualization view to readable text format
    fn render_view_as_text(view: &codehud_viz::RenderableView) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        writeln!(output, "=== CODEHUD VISUALIZATION ===").unwrap();
        writeln!(output, "Title: {}", view.title).unwrap();
        writeln!(output, "View Type: {:?}", view.view_type).unwrap();
        writeln!(output, "Timestamp: {}", view.timestamp).unwrap();
        writeln!(output, "").unwrap();

        match &view.content {
            codehud_viz::ViewContent::Summary { metrics, recommendations, .. } => {
                writeln!(output, "=== SUMMARY VIEW ===").unwrap();
                writeln!(output, "Key Metrics:").unwrap();
                for (key, value) in metrics {
                    writeln!(output, "  • {}: {:.2}", key, value).unwrap();
                }
                writeln!(output, "\nRecommendations:").unwrap();
                for (i, rec) in recommendations.iter().enumerate() {
                    writeln!(output, "  {}. {}", i + 1, rec).unwrap();
                }
            }
            codehud_viz::ViewContent::Quality { health_score, issues_by_severity, top_problematic_files, .. } => {
                writeln!(output, "=== QUALITY VIEW ===").unwrap();
                writeln!(output, "Health Score: {:.1}%", health_score * 100.0).unwrap();
                writeln!(output, "\nIssues by Severity:").unwrap();
                for (severity, count) in issues_by_severity {
                    writeln!(output, "  • {}: {} issues", severity, count).unwrap();
                }
                writeln!(output, "\nMost Problematic Files:").unwrap();
                for (file, score) in top_problematic_files.iter().take(5) {
                    writeln!(output, "  • {}: {:.2}", file, score).unwrap();
                }
            }
            codehud_viz::ViewContent::Security { risk_level, top_security_issues, security_score, .. } => {
                writeln!(output, "=== SECURITY VIEW ===").unwrap();
                writeln!(output, "Risk Level: {}", risk_level).unwrap();
                writeln!(output, "Security Score: {:.1}%", security_score * 100.0).unwrap();
                writeln!(output, "\nTop Security Issues:").unwrap();
                for issue in top_security_issues.iter().take(10) {
                    writeln!(output, "  • [{}] {}: {}", issue.severity, issue.file, issue.description).unwrap();
                }
            }
            _ => {
                writeln!(output, "=== OTHER VIEW ===").unwrap();
                writeln!(output, "Content: {:?}", view.content).unwrap();
            }
        }

        output
    }

    fn render_issues_view_static(f: &mut Frame, area: Rect, data: &AnalysisData, _state: &AppState) {
        let issues_items: Vec<ListItem> = data.critical_issues
            .iter()
            .take(20) // Limit for static view
            .enumerate()
            .map(|(i, issue)| {
                let severity_color = match issue.severity {
                    Severity::Critical => Color::Red,
                    Severity::High => Color::LightRed,
                    Severity::Medium => Color::Yellow,
                    Severity::Low => Color::Green,
                    Severity::Info => Color::Cyan,
                };

                let file_name = Path::new(&issue.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&issue.file_path);

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:>3}: ", i + 1),
                            Style::default().fg(Color::Gray)
                        ),
                        Span::styled(
                            format!("{:?}", issue.severity),
                            Style::default().fg(severity_color).add_modifier(Modifier::BOLD)
                        ),
                        Span::raw(" | "),
                        Span::styled(file_name, Style::default().fg(Color::Cyan)),
                        Span::raw(" | "),
                        Span::raw(&issue.description),
                    ])
                ])
            })
            .collect();

        let issues_list = List::new(issues_items)
            .block(Block::default().borders(Borders::ALL).title("Critical Issues"))
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(issues_list, area);
    }

    fn render_placeholder_view_static(f: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("View not yet implemented")
            .block(Block::default().borders(Borders::ALL).title("Placeholder"))
            .wrap(Wrap { trim: true });
        f.render_widget(placeholder, area);
    }

    fn render_no_data_view_static(f: &mut Frame, area: Rect) {
        let no_data = Paragraph::new(vec![
            Line::from("No analysis data loaded"),
            Line::from(""),
            Line::from("Press 'r' to refresh or load analysis data"),
            Line::from("Use CLI: codehud-tui <codebase_path>"),
        ])
        .block(Block::default().borders(Borders::ALL).title("CodeHUD TUI"))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        f.render_widget(no_data, area);
    }
}

impl Drop for CodeHudTui {
    fn drop(&mut self) {
        if let Some(terminal) = &mut self.terminal {
            let _ = disable_raw_mode();
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            let _ = terminal.show_cursor();
        }
    }
}

/// Create and run TUI for a given codebase
pub async fn run_tui(codebase_path: &Path) -> Result<()> {
    let mut tui = CodeHudTui::new()?;
    tui.load_analysis(codebase_path).await?;
    tui.run()?;
    Ok(())
}

/// Export analysis data in structured format for programmatic consumption
pub fn export_structured_data(analysis_data: &AnalysisData) -> Result<String> {
    #[derive(Serialize)]
    struct StructuredExport {
        health_score: f64,
        files_analyzed: usize,
        critical_issues: Vec<CriticalIssueExport>,
        quality_summary: QualitySummaryExport,
        security_summary: SecuritySummaryExport,
        problematic_files: Vec<ProblematicFileExport>,
        timestamp: DateTime<Utc>,
    }

    #[derive(Serialize)]
    struct CriticalIssueExport {
        severity: String,
        category: String,
        description: String,
        file_path: String,
        line_number: Option<usize>,
        impact_score: f64,
        fix_suggestion: Option<String>,
    }

    #[derive(Serialize)]
    struct QualitySummaryExport {
        average_maintainability: f64,
        total_issues: usize,
        worst_files: Vec<(String, f64)>,
    }

    #[derive(Serialize)]
    struct SecuritySummaryExport {
        risk_level: String,
        total_vulnerabilities: usize,
        critical_vulnerabilities: usize,
        rust_specific_issues: usize,
    }

    #[derive(Serialize)]
    struct ProblematicFileExport {
        path: String,
        maintainability_score: f64,
        issues_count: usize,
        complexity_score: f64,
        priority_rank: usize,
        recommended_actions: Vec<String>,
    }

    let export = StructuredExport {
        health_score: analysis_data.health_score,
        files_analyzed: analysis_data.files_analyzed,
        critical_issues: analysis_data.critical_issues.iter().map(|issue| CriticalIssueExport {
            severity: format!("{:?}", issue.severity),
            category: issue.category.clone(),
            description: issue.description.clone(),
            file_path: issue.file_path.clone(),
            line_number: issue.line_number,
            impact_score: issue.impact_score,
            fix_suggestion: issue.fix_suggestion.clone(),
        }).collect(),
        quality_summary: QualitySummaryExport {
            average_maintainability: analysis_data.quality_summary.average_maintainability,
            total_issues: analysis_data.quality_summary.total_issues,
            worst_files: analysis_data.quality_summary.worst_files.clone(),
        },
        security_summary: SecuritySummaryExport {
            risk_level: format!("{:?}", analysis_data.security_summary.risk_level),
            total_vulnerabilities: analysis_data.security_summary.total_vulnerabilities,
            critical_vulnerabilities: analysis_data.security_summary.critical_vulnerabilities,
            rust_specific_issues: analysis_data.security_summary.rust_specific_issues,
        },
        problematic_files: analysis_data.problematic_files.iter().map(|file| ProblematicFileExport {
            path: file.path.clone(),
            maintainability_score: file.maintainability_score,
            issues_count: file.issues_count,
            complexity_score: file.complexity_score,
            priority_rank: file.priority_rank,
            recommended_actions: file.recommended_actions.clone(),
        }).collect(),
        timestamp: analysis_data.timestamp,
    };

    Ok(serde_json::to_string_pretty(&export)?)
}