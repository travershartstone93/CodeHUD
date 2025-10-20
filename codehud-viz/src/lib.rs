//! CodeHUD Visualization - Data visualization and export capabilities
//!
//! This crate provides visualization capabilities matching the Python implementation
//! with support for various output formats and interactive visualizations.

#![warn(clippy::all, clippy::pedantic)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use codehud_core::{
    models::{AnalysisResult, ViewType},
    extractors::FileMetrics,
};
use codehud_utils::logging::get_logger;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, Gauge, List, ListItem,
        Paragraph, BarChart, Clear, Table, Row, Cell, Wrap
    },
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

mod call_graph;
pub use call_graph::{CallGraph, CallGraphNode, CallGraphEdge};

// Graphviz DOT export and rendering
pub mod graph_dot;
pub mod graphviz;
pub mod graph_analysis;

pub use graph_dot::DotExporter;
pub use graphviz::{OutputFormat, LayoutEngine, check_graphviz_installed, render_dot_to_file, render_dot_to_string};
pub use graph_analysis::{
    GraphAnalysis, Module, StronglyConnectedComponent, ImportanceScore,
    analyze_graph, detect_cycles, cluster_by_module, build_module_graph,
    extract_module_subgraph, extract_cycle_subgraph
};
use syntect::{
    easy::HighlightLines,
    highlighting::{ThemeSet, Style as SyntectStyle},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

/// Main visualization system
pub struct VisualizationEngine {
    config: VizConfig,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Clone for VisualizationEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
}

/// Configuration for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizConfig {
    /// Terminal width (characters)
    pub terminal_width: u16,
    /// Terminal height (characters)
    pub terminal_height: u16,
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Maximum items to show in lists
    pub max_list_items: usize,
    /// Chart data point limit
    pub max_chart_points: usize,
}

impl Default for VizConfig {
    fn default() -> Self {
        Self {
            terminal_width: 120,
            terminal_height: 40,
            color_scheme: ColorScheme::Dark,
            syntax_highlighting: true,
            max_list_items: 20,
            max_chart_points: 100,
        }
    }
}

/// Color schemes for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorScheme {
    Dark,
    Light,
    HighContrast,
}

/// Renderable view that can be displayed in terminal
#[derive(Debug, Clone, Serialize)]
pub struct RenderableView {
    pub view_type: ViewType,
    pub title: String,
    pub content: ViewContent,
    pub timestamp: DateTime<Utc>,
}

/// Content types for different visualizations
#[derive(Debug, Clone, Serialize)]
pub enum ViewContent {
    /// Summary dashboard with key metrics
    Summary {
        health_score: f64,
        files_analyzed: usize,
        critical_issues: usize,
        recommendations: Vec<String>,
        metrics: HashMap<String, f64>,
    },
    /// Code topology visualization
    Topology {
        file_tree: FileTree,
        language_distribution: HashMap<String, usize>,
        complexity_distribution: Vec<(String, f64)>,
        coupling_metrics: Vec<(String, f64)>,
    },
    /// Quality analysis visualization
    Quality {
        health_score: f64,
        issues_by_severity: HashMap<String, usize>,
        top_problematic_files: Vec<(String, f64)>,
        complexity_trend: Vec<(String, f64)>,
        maintainability_scores: Vec<(String, f64)>,
    },
    /// Security analysis visualization
    Security {
        risk_level: String,
        vulnerabilities_by_severity: HashMap<String, usize>,
        top_security_issues: Vec<SecurityIssue>,
        security_score: f64,
        files_with_issues: Vec<String>,
    },
    /// Dependencies visualization
    Dependencies {
        total_dependencies: usize,
        circular_dependencies: Vec<String>,
        dependency_graph: DependencyGraph,
        coupling_analysis: Vec<(String, f64)>,
        external_dependencies: Vec<String>,
    },
    /// Performance analysis visualization
    Performance {
        hotspots: Vec<PerformanceHotspot>,
        bottlenecks: Vec<String>,
        performance_score: f64,
        slow_functions: Vec<(String, f64)>,
    },
    /// Evolution/history visualization
    Evolution {
        commit_activity: Vec<(String, usize)>,
        author_contributions: Vec<(String, usize)>,
        file_stability: Vec<(String, f64)>,
        churn_metrics: Vec<(String, f64)>,
    },
    /// Issues inspection
    Issues {
        issues_by_type: HashMap<String, usize>,
        recent_issues: Vec<IssueItem>,
        resolution_trends: Vec<(String, usize)>,
    },
    /// Testing analysis
    Testing {
        test_coverage: f64,
        test_files: Vec<String>,
        uncovered_files: Vec<String>,
        test_trends: Vec<(String, f64)>,
    },
    /// Code flow visualization
    Flow {
        data_flows: Vec<FlowItem>,
        control_flows: Vec<FlowItem>,
        flow_complexity: f64,
        bottlenecks: Vec<String>,
    },
    /// Tree-sitter enhanced semantic analysis
    TreeSitterAnalysis {
        imports_summary: TreeSitterImportSummary,
        symbols_by_type: HashMap<String, Vec<TreeSitterSymbol>>,
        highlights_summary: TreeSitterHighlightSummary,
        semantic_complexity: f64,
        language_features: Vec<LanguageFeature>,
    },
}

/// File tree structure for topology visualization
#[derive(Debug, Clone, Serialize)]
pub struct FileTree {
    pub root: FileNode,
    pub total_files: usize,
    pub total_directories: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub children: Vec<FileNode>,
    pub metrics: Option<FileMetrics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityIssue {
    pub severity: String,
    pub description: String,
    pub file: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub circular_cycles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceHotspot {
    pub function: String,
    pub file: String,
    pub complexity: f64,
    pub estimated_time: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueItem {
    pub issue_type: String,
    pub severity: String,
    pub message: String,
    pub file: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowItem {
    pub from: String,
    pub to: String,
    pub flow_type: String,
    pub complexity: f64,
}

/// Tree-sitter import analysis summary
#[derive(Debug, Clone, Serialize)]
pub struct TreeSitterImportSummary {
    pub total_imports: usize,
    pub unique_modules: usize,
    pub external_dependencies: Vec<String>,
    pub internal_dependencies: Vec<String>,
    pub wildcard_imports: usize,
    pub aliased_imports: usize,
    pub analysis_method: String,
}

/// Tree-sitter symbol information
#[derive(Debug, Clone, Serialize)]
pub struct TreeSitterSymbol {
    pub name: String,
    pub symbol_type: String,
    pub file: String,
    pub line: usize,
    pub scope: Option<String>,
}

/// Tree-sitter semantic highlights summary
#[derive(Debug, Clone, Serialize)]
pub struct TreeSitterHighlightSummary {
    pub total_highlights: usize,
    pub semantic_types: HashMap<String, usize>,
    pub functions_found: usize,
    pub types_found: usize,
    pub variables_found: usize,
}

/// Language feature detected by tree-sitter
#[derive(Debug, Clone, Serialize)]
pub struct LanguageFeature {
    pub feature_type: String,
    pub count: usize,
    pub description: String,
    pub files: Vec<String>,
}

impl VisualizationEngine {
    /// Create a new visualization engine
    pub fn new() -> Self {
        Self {
            config: VizConfig::default(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Configure the visualization engine
    pub fn with_config(mut self, config: VizConfig) -> Self {
        self.config = config;
        self
    }

    /// Generate a renderable view from analysis results
    pub fn generate_view(&self, view_type: ViewType, analysis_result: &AnalysisResult) -> Result<RenderableView> {
        let logger = get_logger("codehud.viz");
        logger.info(&format!("Generating {} view", view_type.to_string()));

        let content = match view_type {
            ViewType::Topology => self.generate_topology_view(analysis_result)?,
            ViewType::Quality => self.generate_quality_view(analysis_result)?,
            ViewType::Security => self.generate_security_view(analysis_result)?,
            ViewType::Dependencies => self.generate_dependencies_view(analysis_result)?,
            ViewType::Performance => self.generate_performance_view(analysis_result)?,
            ViewType::Evolution => self.generate_evolution_view(analysis_result)?,
            ViewType::IssuesInspection => self.generate_issues_view(analysis_result)?,
            ViewType::Testing => self.generate_testing_view(analysis_result)?,
            ViewType::Flow => self.generate_flow_view(analysis_result)?,
            ViewType::FixRollbackDevnotes => self.generate_summary_view(analysis_result)?,
            ViewType::TreeSitterAnalysis => self.generate_tree_sitter_view(analysis_result)?,
        };

        Ok(RenderableView {
            view_type,
            title: format!("CodeHUD - {} Analysis", view_type.to_string()),
            content,
            timestamp: Utc::now(),
        })
    }

    /// Render a view to terminal
    pub fn render_to_terminal(&self, frame: &mut Frame, view: &RenderableView) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Footer
            ])
            .split(frame.size());

        // Render header
        self.render_header(frame, main_layout[0], view);

        // Render content based on view type
        match &view.content {
            ViewContent::Summary { .. } => self.render_summary_content(frame, main_layout[1], view),
            ViewContent::Topology { .. } => self.render_topology_content(frame, main_layout[1], view),
            ViewContent::Quality { .. } => self.render_quality_content(frame, main_layout[1], view),
            ViewContent::Security { .. } => self.render_security_content(frame, main_layout[1], view),
            ViewContent::Dependencies { .. } => self.render_dependencies_content(frame, main_layout[1], view),
            ViewContent::Performance { .. } => self.render_performance_content(frame, main_layout[1], view),
            ViewContent::Evolution { .. } => self.render_evolution_content(frame, main_layout[1], view),
            ViewContent::Issues { .. } => self.render_issues_content(frame, main_layout[1], view),
            ViewContent::Testing { .. } => self.render_testing_content(frame, main_layout[1], view),
            ViewContent::Flow { .. } => self.render_flow_content(frame, main_layout[1], view),
            ViewContent::TreeSitterAnalysis { .. } => self.render_tree_sitter_content(frame, main_layout[1], view),
        }

        // Render footer
        self.render_footer(frame, main_layout[2], view);
    }

    /// Generate summary view content
    fn generate_summary_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let mut metrics = HashMap::new();
        let mut enhanced_recommendations = result.focus_recommendations.clone();

        // Core analysis metrics
        metrics.insert("Health Score".to_string(), result.health_score);
        metrics.insert("Files Analyzed".to_string(), result.files_analyzed as f64);
        metrics.insert("Critical Issues".to_string(), result.critical_issues.len() as f64);
        metrics.insert("Analysis Duration (s)".to_string(), result.analysis_duration);

        // Extract additional metrics from various view data sources

        // Quality metrics
        if let Some(quality_data) = result.get_view_data("quality") {
            if let Some(summary) = quality_data.get("summary") {
                if let Some(total_functions) = summary.get("total_functions") {
                    if let Some(func_count) = total_functions.as_f64() {
                        metrics.insert("Total Functions".to_string(), func_count);
                    }
                }
                if let Some(total_classes) = summary.get("total_classes") {
                    if let Some(class_count) = total_classes.as_f64() {
                        metrics.insert("Total Classes".to_string(), class_count);
                    }
                }
                if let Some(code_lines) = summary.get("total_code_lines") {
                    if let Some(lines_count) = code_lines.as_f64() {
                        metrics.insert("Lines of Code".to_string(), lines_count);
                    }
                }
            }
        }

        // Security metrics
        if let Some(security_data) = result.get_view_data("security") {
            if let Some(summary) = security_data.get("summary") {
                if let Some(security_score) = summary.get("security_score") {
                    if let Some(score) = security_score.as_f64() {
                        metrics.insert("Security Score".to_string(), score);
                    }
                }
            }

            // Count security vulnerabilities
            let mut total_vulns = 0;
            if let Some(vulns) = security_data.get("all_vulnerabilities") {
                if let Some(vulns_array) = vulns.as_array() {
                    total_vulns += vulns_array.len();
                }
            }
            if let Some(issues) = security_data.get("all_security_issues") {
                if let Some(issues_array) = issues.as_array() {
                    total_vulns += issues_array.len();
                }
            }
            if total_vulns > 0 {
                metrics.insert("Security Vulnerabilities".to_string(), total_vulns as f64);
                enhanced_recommendations.push(format!("Address {} security vulnerabilities", total_vulns));
            }
        }

        // Testing metrics
        if let Some(testing_data) = result.get_view_data("testing") {
            if let Some(summary) = testing_data.get("summary") {
                if let Some(coverage) = summary.get("coverage_percentage") {
                    if let Some(coverage_val) = coverage.as_f64() {
                        metrics.insert("Test Coverage %".to_string(), coverage_val);
                        if coverage_val < 80.0 {
                            enhanced_recommendations.push("Improve test coverage (target: 80%+)".to_string());
                        }
                    }
                }
                if let Some(test_files) = summary.get("test_files_count") {
                    if let Some(test_count) = test_files.as_f64() {
                        metrics.insert("Test Files".to_string(), test_count);
                    }
                }
            }
        }

        // Dependencies metrics
        if let Some(deps_data) = result.get_view_data("dependencies") {
            if let Some(summary) = deps_data.get("summary") {
                if let Some(total_deps) = summary.get("total_dependencies") {
                    if let Some(deps_count) = total_deps.as_f64() {
                        metrics.insert("Dependencies".to_string(), deps_count);
                    }
                }
                if let Some(circular_deps) = summary.get("circular_dependencies_count") {
                    if let Some(circular_count) = circular_deps.as_f64() {
                        if circular_count > 0.0 {
                            metrics.insert("Circular Dependencies".to_string(), circular_count);
                            enhanced_recommendations.push(format!("Resolve {} circular dependencies", circular_count as usize));
                        }
                    }
                }
            }
        }

        // Performance metrics
        if let Some(perf_data) = result.get_view_data("performance") {
            if let Some(summary) = perf_data.get("summary") {
                if let Some(hotspots) = summary.get("hotspots_count") {
                    if let Some(hotspots_count) = hotspots.as_f64() {
                        if hotspots_count > 0.0 {
                            metrics.insert("Performance Hotspots".to_string(), hotspots_count);
                            enhanced_recommendations.push("Optimize performance hotspots".to_string());
                        }
                    }
                }
            }
        }

        // Add general recommendations based on analysis
        if result.health_score < 70.0 {
            enhanced_recommendations.push("Overall code health is below target (70%)".to_string());
        }
        if result.critical_issues.len() > 0 {
            enhanced_recommendations.push(format!("Address {} critical issues immediately", result.critical_issues.len()));
        }

        Ok(ViewContent::Summary {
            health_score: result.health_score,
            files_analyzed: result.files_analyzed,
            critical_issues: result.critical_issues.len(),
            recommendations: enhanced_recommendations,
            metrics,
        })
    }

    /// Generate topology view content
    fn generate_topology_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        // Extract topology data from analysis result
        let topology_data = result.get_view_data("topology");

        let mut language_distribution = HashMap::new();
        let mut complexity_distribution = Vec::new();
        let mut coupling_metrics = Vec::new();
        let mut file_tree = FileTree {
            root: FileNode {
                name: "root".to_string(),
                path: PathBuf::from("/"),
                is_directory: true,
                size: None,
                children: Vec::new(),
                metrics: None,
            },
            total_files: result.files_analyzed,
            total_directories: 1,
        };

        if let Some(data) = topology_data {
            // Extract language distribution from summary
            if let Some(summary) = data.get("summary") {
                if let Some(lang_dist) = summary.get("language_distribution") {
                    if let Some(obj) = lang_dist.as_object() {
                        for (lang, count) in obj {
                            if let Some(count_val) = count.as_u64() {
                                language_distribution.insert(lang.clone(), count_val as usize);
                            }
                        }
                    }
                }

                // Extract file and directory counts from structure
                if let Some(total_files) = summary.get("total_files") {
                    if let Some(files_count) = total_files.as_u64() {
                        file_tree.total_files = files_count as usize;
                    }
                }
                if let Some(total_dirs) = summary.get("total_directories") {
                    if let Some(dirs_count) = total_dirs.as_u64() {
                        file_tree.total_directories = dirs_count as usize;
                    }
                }
            }

            // Extract complexity data from files
            if let Some(files) = data.get("files") {
                if let Some(files_array) = files.as_array() {
                    for file in files_array.iter().take(self.config.max_chart_points) {
                        // Extract file path and complexity
                        if let (Some(path), Some(complexity)) = (file.get("path"), file.get("complexity")) {
                            if let (Some(path_str), Some(complexity_val)) = (path.as_str(), complexity.as_i64()) {
                                complexity_distribution.push((
                                    self.extract_filename(path_str),
                                    complexity_val as f64
                                ));
                            }
                        }
                    }
                }
            }

            // Extract coupling metrics from coupling analysis
            if let Some(coupling) = data.get("coupling") {
                if let Some(most_coupled) = coupling.get("most_coupled_files") {
                    if let Some(coupled_array) = most_coupled.as_array() {
                        for item in coupled_array.iter().take(self.config.max_list_items) {
                            if let Some(item_array) = item.as_array() {
                                if item_array.len() >= 2 {
                                    if let (Some(file), Some(score)) = (item_array[0].as_str(), item_array[1].as_f64()) {
                                        coupling_metrics.push((
                                            self.extract_filename(file),
                                            score
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Build real file tree structure from project structure
            if let Some(structure) = data.get("structure") {
                if let Some(directories) = structure.get("directories") {
                    if let Some(dirs_array) = directories.as_array() {
                        file_tree.total_directories = dirs_array.len();
                    }
                }

                // Build hierarchical file tree from files data
                if let Some(files) = data.get("files") {
                    self.build_file_tree_from_files(&mut file_tree.root, files);
                }
            }
        }

        Ok(ViewContent::Topology {
            file_tree,
            language_distribution,
            complexity_distribution,
            coupling_metrics,
        })
    }

    /// Extract filename from full path for display
    fn extract_filename(&self, path: &str) -> String {
        if let Some(filename) = path.split('/').last() {
            filename.to_string()
        } else {
            path.to_string()
        }
    }

    /// Build file tree structure from files data
    fn build_file_tree_from_files(&self, root: &mut FileNode, files_array: &serde_json::Value) {
        if let Some(files) = files_array.as_array() {
            for file in files.iter().take(10) { // Limit for performance
                if let (Some(path), Some(size)) = (file.get("path"), file.get("size")) {
                    if let Some(path_str) = path.as_str() {
                        let filename = self.extract_filename(path_str);
                        let file_size = size.as_u64();

                        root.children.push(FileNode {
                            name: filename,
                            path: PathBuf::from(path_str),
                            is_directory: false,
                            size: file_size,
                            children: Vec::new(),
                            metrics: None,
                        });
                    }
                }
            }
        }
    }

    /// Generate quality view content
    fn generate_quality_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let quality_data = result.get_view_data("quality");

        let mut issues_by_severity = HashMap::new();
        let mut top_problematic_files = Vec::new();
        let mut complexity_trend = Vec::new();
        let mut maintainability_scores = Vec::new();

        if let Some(data) = quality_data {
            // Extract issues by severity from quality_issues
            if let Some(issues) = data.get("quality_issues") {
                if let Some(issues_array) = issues.as_array() {
                    for issue in issues_array {
                        if let Some(severity) = issue.get("severity").and_then(|s| s.as_str()) {
                            *issues_by_severity.entry(severity.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }

            // Extract file metrics for analysis
            if let Some(file_metrics) = data.get("file_metrics") {
                if let Some(files_array) = file_metrics.as_array() {
                    let mut files_with_scores: Vec<_> = files_array.iter()
                        .filter_map(|f| {
                            let file = f.get("file")?.as_str()?;
                            let maintainability = f.get("maintainability_score")?.as_f64()?;
                            Some((self.extract_filename(file), maintainability))
                        })
                        .collect();

                    // Sort by lowest maintainability (most problematic)
                    files_with_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    top_problematic_files = files_with_scores.into_iter().take(self.config.max_list_items).collect();

                    // Extract complexity and maintainability trends with better file handling
                    for file in files_array.iter().take(self.config.max_chart_points) {
                        if let (Some(file_name), Some(complexity)) = (file.get("file"), file.get("complexity_score")) {
                            if let (Some(name_str), Some(complexity_val)) = (file_name.as_str(), complexity.as_f64()) {
                                complexity_trend.push((self.extract_filename(name_str), complexity_val));
                            }
                        }

                        if let (Some(file_name), Some(maintainability)) = (file.get("file"), file.get("maintainability_score")) {
                            if let (Some(name_str), Some(maint_val)) = (file_name.as_str(), maintainability.as_f64()) {
                                maintainability_scores.push((self.extract_filename(name_str), maint_val));
                            }
                        }
                    }
                }
            }

            // Extract additional metrics from summary if available
            if let Some(summary) = data.get("summary") {
                // Add overall statistics to issues if available
                if let Some(total_issues) = summary.get("total_quality_issues") {
                    if let Some(total_val) = total_issues.as_u64() {
                        if issues_by_severity.is_empty() && total_val > 0 {
                            // If no specific severity breakdown, add general category
                            issues_by_severity.insert("general".to_string(), total_val as usize);
                        }
                    }
                }
            }
        }

        // Normalize health score to 0-1 range if needed
        let normalized_health_score = (result.health_score / 100.0).min(1.0).max(0.0);

        Ok(ViewContent::Quality {
            health_score: normalized_health_score,
            issues_by_severity,
            top_problematic_files,
            complexity_trend,
            maintainability_scores,
        })
    }

    /// Generate security view content
    fn generate_security_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let security_data = result.get_view_data("security");

        let mut vulnerabilities_by_severity = HashMap::new();
        let mut top_security_issues = Vec::new();
        let mut files_with_issues = Vec::new();
        let mut risk_level = "low".to_string();
        let mut security_score = 100.0;

        if let Some(data) = security_data {
            // Extract from all_vulnerabilities (comprehensive security issues)
            if let Some(vulns) = data.get("all_vulnerabilities") {
                if let Some(vulns_array) = vulns.as_array() {
                    for vuln in vulns_array.iter().take(self.config.max_list_items) {
                        if let Some(severity) = vuln.get("severity").and_then(|s| s.as_str()) {
                            *vulnerabilities_by_severity.entry(severity.to_string()).or_insert(0) += 1;
                        }

                        // Create security issue object with enhanced data
                        let severity = vuln.get("severity").and_then(|s| s.as_str()).unwrap_or("medium").to_string();
                        let description = vuln.get("description").and_then(|d| d.as_str()).unwrap_or("Security vulnerability detected").to_string();
                        let file_path = vuln.get("file_path").and_then(|f| f.as_str()).unwrap_or("unknown").to_string();
                        let line = vuln.get("line_number").and_then(|l| l.as_u64()).map(|l| l as usize);

                        top_security_issues.push(SecurityIssue {
                            severity,
                            description,
                            file: self.extract_filename(&file_path),
                            line,
                        });

                        files_with_issues.push(self.extract_filename(&file_path));
                    }
                }
            }

            // Extract from all_security_issues (additional security analysis)
            if let Some(issues) = data.get("all_security_issues") {
                if let Some(issues_array) = issues.as_array() {
                    for issue in issues_array.iter().take(self.config.max_list_items) {
                        if let Some(severity) = issue.get("severity").and_then(|s| s.as_str()) {
                            *vulnerabilities_by_severity.entry(severity.to_string()).or_insert(0) += 1;
                        }

                        let severity = issue.get("severity").and_then(|s| s.as_str()).unwrap_or("medium").to_string();
                        let description = issue.get("description").and_then(|d| d.as_str()).unwrap_or("Security issue detected").to_string();
                        let file_path = issue.get("file_path").and_then(|f| f.as_str()).unwrap_or("unknown").to_string();
                        let line = issue.get("line_number").and_then(|l| l.as_u64()).map(|l| l as usize);

                        if top_security_issues.len() < self.config.max_list_items {
                            top_security_issues.push(SecurityIssue {
                                severity,
                                description,
                                file: self.extract_filename(&file_path),
                                line,
                            });
                        }

                        files_with_issues.push(self.extract_filename(&file_path));
                    }
                }
            }

            // Extract from dangerous function usage
            if let Some(dangerous) = data.get("all_dangerous_functions") {
                if let Some(dangerous_array) = dangerous.as_array() {
                    for func in dangerous_array.iter().take(self.config.max_list_items / 2) {
                        let severity = func.get("severity").and_then(|s| s.as_str()).unwrap_or("high").to_string();
                        let function_name = func.get("function_name").and_then(|f| f.as_str()).unwrap_or("unknown").to_string();
                        let file_path = func.get("file_path").and_then(|f| f.as_str()).unwrap_or("unknown").to_string();
                        let line = func.get("line_number").and_then(|l| l.as_u64()).map(|l| l as usize);

                        *vulnerabilities_by_severity.entry(severity.clone()).or_insert(0) += 1;

                        if top_security_issues.len() < self.config.max_list_items {
                            top_security_issues.push(SecurityIssue {
                                severity,
                                description: format!("Dangerous function usage: {}", function_name),
                                file: self.extract_filename(&file_path),
                                line,
                            });
                        }

                        files_with_issues.push(self.extract_filename(&file_path));
                    }
                }
            }

            // Calculate risk level and security score based on vulnerabilities
            let total_issues = vulnerabilities_by_severity.values().sum::<usize>();
            let high_severity = vulnerabilities_by_severity.get("high").unwrap_or(&0);
            let critical_severity = vulnerabilities_by_severity.get("critical").unwrap_or(&0);

            if *critical_severity > 0 || *high_severity > 3 {
                risk_level = "critical".to_string();
                security_score = 20.0;
            } else if *high_severity > 0 || total_issues > 5 {
                risk_level = "high".to_string();
                security_score = 40.0;
            } else if total_issues > 2 {
                risk_level = "medium".to_string();
                security_score = 70.0;
            } else if total_issues > 0 {
                risk_level = "low".to_string();
                security_score = 85.0;
            }

            // Extract from summary if available
            if let Some(summary) = data.get("summary") {
                if let Some(risk) = summary.get("risk_level") {
                    if let Some(risk_str) = risk.as_str() {
                        risk_level = risk_str.to_string();
                    }
                }
                if let Some(score) = summary.get("security_score") {
                    if let Some(score_val) = score.as_f64() {
                        security_score = score_val;
                    }
                }
            }
        }

        // Remove duplicates from files_with_issues
        files_with_issues.sort();
        files_with_issues.dedup();

        Ok(ViewContent::Security {
            risk_level,
            vulnerabilities_by_severity,
            top_security_issues,
            security_score,
            files_with_issues,
        })
    }

    /// Generate dependencies view content
    fn generate_dependencies_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let dependencies_data = result.get_view_data("dependencies");

        let mut total_dependencies = 0;
        let mut circular_dependencies = Vec::new();
        let mut coupling_analysis = Vec::new();
        let mut external_dependencies = Vec::new();
        let mut dependency_graph = DependencyGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            circular_cycles: Vec::new(),
        };

        if let Some(data) = dependencies_data {
            // Extract summary info
            if let Some(summary) = data.get("summary") {
                if let Some(total_imports) = summary.get("total_import_statements") {
                    total_dependencies = total_imports.as_u64().unwrap_or(0) as usize;
                }
            }

            // Extract circular dependencies
            if let Some(circular_deps) = data.get("circular_dependencies") {
                if let Some(cycles_array) = circular_deps.as_array() {
                    for cycle in cycles_array.iter().take(self.config.max_list_items) {
                        if let Some(cycle_data) = cycle.get("cycle") {
                            if let Some(cycle_array) = cycle_data.as_array() {
                                let cycle_files: Vec<String> = cycle_array.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect();
                                if !cycle_files.is_empty() {
                                    circular_dependencies.push(cycle_files.join(" → "));
                                }
                            }
                        }
                    }
                }
            }

            // Extract coupling analysis
            if let Some(coupling) = data.get("coupling_analysis") {
                if let Some(coupling_metrics) = coupling.get("coupling_metrics") {
                    if let Some(metrics_array) = coupling_metrics.as_array() {
                        for metric in metrics_array.iter().take(self.config.max_list_items) {
                            if let (Some(file), Some(score)) = (metric.get("file"), metric.get("coupling_score")) {
                                if let (Some(file_str), Some(score_val)) = (file.as_str(), score.as_f64()) {
                                    coupling_analysis.push((file_str.to_string(), score_val));
                                }
                            }
                        }
                    }
                }
            }

            // Extract external dependencies
            if let Some(external_deps) = data.get("external_dependencies") {
                if let Some(most_used) = external_deps.get("most_used_external") {
                    if let Some(external_array) = most_used.as_array() {
                        for dep in external_array.iter().take(self.config.max_list_items) {
                            if let Some(dep_array) = dep.as_array() {
                                if dep_array.len() >= 2 {
                                    if let (Some(name), Some(count)) = (dep_array[0].as_str(), dep_array[1].as_u64()) {
                                        external_dependencies.push(format!("{} ({})", name, count));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Extract graph structure
            if let Some(graph_analysis) = data.get("graph_analysis") {
                // Extract nodes from file dependencies
                if let Some(file_deps) = data.get("file_dependencies") {
                    if let Some(files_obj) = file_deps.as_object() {
                        dependency_graph.nodes = files_obj.keys().cloned().collect();
                    }
                }

                // Extract edges from coupling analysis
                if let Some(coupling) = data.get("coupling_analysis") {
                    if let Some(strong_couplings) = coupling.get("strong_couplings") {
                        if let Some(couplings_array) = strong_couplings.as_array() {
                            for coupling in couplings_array.iter().take(self.config.max_chart_points) {
                                if let (Some(from), Some(to)) = (coupling.get("from"), coupling.get("to")) {
                                    if let (Some(from_str), Some(to_str)) = (from.as_str(), to.as_str()) {
                                        dependency_graph.edges.push((from_str.to_string(), to_str.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }

                // Extract circular cycles for graph
                dependency_graph.circular_cycles = circular_dependencies.iter()
                    .map(|cycle_str| {
                        cycle_str.split(" → ")
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .collect();
            }
        }

        Ok(ViewContent::Dependencies {
            total_dependencies,
            circular_dependencies,
            dependency_graph,
            coupling_analysis,
            external_dependencies,
        })
    }

    fn generate_performance_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let performance_data = result.get_view_data("performance");

        let mut hotspots = Vec::new();
        let mut bottlenecks = Vec::new();
        let mut performance_score = 0.0;
        let mut slow_functions = Vec::new();

        if let Some(data) = performance_data {
            // Extract average performance score
            if let Some(score) = data.get("average_performance_score") {
                performance_score = score.as_f64().unwrap_or(0.0);
            }

            // Extract performance hotspots
            if let Some(hotspots_data) = data.get("performance_hotspots") {
                if let Some(hotspot_array) = hotspots_data.as_array() {
                    for hotspot in hotspot_array.iter().take(self.config.max_list_items) {
                        if let (Some(function), Some(file), Some(complexity)) = (
                            hotspot.get("function_name").and_then(|v| v.as_str()),
                            hotspot.get("file_path").and_then(|v| v.as_str()),
                            hotspot.get("performance_score").and_then(|v| v.as_f64())
                        ) {
                            hotspots.push(PerformanceHotspot {
                                function: function.to_string(),
                                file: file.to_string(),
                                complexity: complexity,
                                estimated_time: complexity * 100.0, // Rough estimate
                            });
                        }
                    }
                }
            }

            // Extract bottlenecks
            if let Some(bottlenecks_data) = data.get("performance_bottlenecks") {
                if let Some(bottleneck_array) = bottlenecks_data.as_array() {
                    for bottleneck in bottleneck_array.iter().take(self.config.max_list_items) {
                        if let Some(description) = bottleneck.get("description").and_then(|v| v.as_str()) {
                            bottlenecks.push(description.to_string());
                        }
                    }
                }
            }

            // Extract slow functions from optimization opportunities
            if let Some(opportunities) = data.get("optimization_opportunities") {
                if let Some(opp_array) = opportunities.as_array() {
                    for opportunity in opp_array.iter().take(self.config.max_list_items) {
                        if let (Some(function), Some(impact)) = (
                            opportunity.get("function_name").and_then(|v| v.as_str()),
                            opportunity.get("estimated_impact").and_then(|v| v.as_f64())
                        ) {
                            slow_functions.push((function.to_string(), impact));
                        }
                    }
                }
            }
        }

        Ok(ViewContent::Performance {
            hotspots,
            bottlenecks,
            performance_score,
            slow_functions,
        })
    }

    fn generate_evolution_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let evolution_data = result.get_view_data("evolution");

        let mut commit_activity = Vec::new();
        let mut author_contributions = Vec::new();
        let mut file_stability = Vec::new();
        let mut churn_metrics = Vec::new();

        if let Some(data) = evolution_data {
            // Extract commit activity patterns
            if let Some(patterns) = data.get("commit_patterns") {
                if let Some(patterns_array) = patterns.as_array() {
                    for pattern in patterns_array.iter().take(self.config.max_list_items) {
                        if let (Some(pattern_type), Some(frequency)) = (
                            pattern.get("pattern_type").and_then(|v| v.as_str()),
                            pattern.get("frequency").and_then(|v| v.as_u64())
                        ) {
                            commit_activity.push((pattern_type.to_string(), frequency as usize));
                        }
                    }
                }
            }

            // Extract author contributions from author_metrics
            if let Some(authors) = data.get("author_metrics") {
                if let Some(authors_array) = authors.as_array() {
                    for author in authors_array.iter().take(self.config.max_list_items) {
                        if let (Some(name), Some(commits)) = (
                            author.get("author_name").and_then(|v| v.as_str()),
                            author.get("total_commits").and_then(|v| v.as_u64())
                        ) {
                            author_contributions.push((name.to_string(), commits as usize));
                        }
                    }
                }
            }

            // Extract file stability from file_evolutions
            if let Some(files) = data.get("file_evolutions") {
                if let Some(files_array) = files.as_array() {
                    for file in files_array.iter().take(self.config.max_list_items) {
                        if let (Some(path), Some(stability)) = (
                            file.get("file_path").and_then(|v| v.as_str()),
                            file.get("stability_score").and_then(|v| v.as_f64())
                        ) {
                            // Get just the filename for display
                            let filename = path.split('/').last().unwrap_or(path);
                            file_stability.push((filename.to_string(), stability));
                        }
                    }
                }
            }

            // Extract churn metrics (files with high change frequency)
            if let Some(files) = data.get("file_evolutions") {
                if let Some(files_array) = files.as_array() {
                    for file in files_array.iter().take(self.config.max_list_items) {
                        if let (Some(path), Some(frequency)) = (
                            file.get("file_path").and_then(|v| v.as_str()),
                            file.get("commit_frequency").and_then(|v| v.as_f64())
                        ) {
                            // Get just the filename for display
                            let filename = path.split('/').last().unwrap_or(path);
                            churn_metrics.push((filename.to_string(), frequency));
                        }
                    }
                }
            }

            // Sort by value (highest first)
            author_contributions.sort_by(|a, b| b.1.cmp(&a.1));
            file_stability.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            churn_metrics.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            commit_activity.sort_by(|a, b| b.1.cmp(&a.1));
        }

        Ok(ViewContent::Evolution {
            commit_activity,
            author_contributions,
            file_stability,
            churn_metrics,
        })
    }

    fn generate_issues_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let issues_data = result.get_view_data("issues");

        let mut issues_by_type = HashMap::new();
        let mut recent_issues = Vec::new();
        let mut resolution_trends = Vec::new();

        if let Some(data) = issues_data {
            // Extract issues by tool type from issue_summary
            if let Some(summary) = data.get("issue_summary") {
                if let Some(issues_by_tool) = summary.get("issues_by_tool") {
                    if let Some(obj) = issues_by_tool.as_object() {
                        for (tool, count) in obj {
                            issues_by_type.insert(
                                tool.clone(),
                                count.as_u64().unwrap_or(0) as usize
                            );
                        }
                    }
                }
            }

            // Extract recent issues from all tools
            let tool_names = ["pylint_issues", "ruff_issues", "bandit_issues", "mypy_issues"];

            for tool_name in &tool_names {
                if let Some(tool_issues) = data.get(*tool_name) {
                    if let Some(issues_array) = tool_issues.as_array() {
                        for (idx, issue) in issues_array.iter().enumerate().take(self.config.max_list_items / tool_names.len()) {
                            let issue_type = if tool_name.contains("pylint") {
                                "Code Quality"
                            } else if tool_name.contains("ruff") {
                                "Linting"
                            } else if tool_name.contains("bandit") {
                                "Security"
                            } else if tool_name.contains("mypy") {
                                "Type Checking"
                            } else {
                                "General"
                            };

                            let severity = issue.get("severity")
                                .and_then(|v| v.as_str())
                                .unwrap_or("medium")
                                .to_string();

                            let message = issue.get("message")
                                .and_then(|v| v.as_str())
                                .or_else(|| issue.get("text").and_then(|v| v.as_str()))
                                .unwrap_or("Issue detected")
                                .to_string();

                            let file = issue.get("path")
                                .and_then(|v| v.as_str())
                                .or_else(|| issue.get("file").and_then(|v| v.as_str()))
                                .unwrap_or("unknown")
                                .to_string();

                            let line = issue.get("line")
                                .and_then(|v| v.as_u64())
                                .map(|l| l as usize);

                            recent_issues.push(IssueItem {
                                issue_type: issue_type.to_string(),
                                severity,
                                message,
                                file,
                                line,
                            });
                        }
                    }
                }
            }

            // Generate mock resolution trends based on issue types
            for (issue_type, count) in &issues_by_type {
                // Simulate trend data - in production this would come from historical analysis
                resolution_trends.push((issue_type.clone(), *count));
            }

            // Sort by severity (critical, high, medium, low)
            recent_issues.sort_by(|a, b| {
                let severity_order = |s: &str| match s {
                    "critical" => 0,
                    "error" | "high" => 1,
                    "warning" | "medium" => 2,
                    "info" | "low" => 3,
                    _ => 4,
                };
                severity_order(&a.severity).cmp(&severity_order(&b.severity))
            });
        }

        Ok(ViewContent::Issues {
            issues_by_type,
            recent_issues,
            resolution_trends,
        })
    }

    fn generate_testing_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let testing_data = result.get_view_data("testing");

        let mut test_coverage = 0.0;
        let mut test_files = Vec::new();
        let mut uncovered_files = Vec::new();
        let mut test_trends = Vec::new();

        if let Some(data) = testing_data {
            // Extract overall test coverage from metrics
            if let Some(metrics) = data.get("test_metrics") {
                if let Some(coverage_percentage) = metrics.get("test_coverage_percentage") {
                    test_coverage = coverage_percentage.as_f64().unwrap_or(0.0) / 100.0; // Convert to 0-1 range
                }

                // Extract files without tests as uncovered files
                if let Some(files_without_tests) = metrics.get("files_without_tests") {
                    if let Some(files_array) = files_without_tests.as_array() {
                        for file in files_array.iter().take(self.config.max_list_items) {
                            if let Some(file_path) = file.as_str() {
                                let filename = file_path.split('/').last().unwrap_or(file_path);
                                uncovered_files.push(filename.to_string());
                            }
                        }
                    }
                }
            }

            // Extract test file names from test_files array
            if let Some(test_files_data) = data.get("test_files") {
                if let Some(test_files_array) = test_files_data.as_array() {
                    for test_file in test_files_array.iter().take(self.config.max_list_items) {
                        if let Some(file_path) = test_file.get("file_path").and_then(|v| v.as_str()) {
                            let filename = file_path.split('/').last().unwrap_or(file_path);
                            test_files.push(filename.to_string());
                        }
                    }
                }
            }

            // Generate test trends from coverage data
            if let Some(coverage_data) = data.get("test_coverage") {
                if let Some(coverage_array) = coverage_data.as_array() {
                    for coverage in coverage_array.iter().take(self.config.max_list_items) {
                        if let (Some(file_path), Some(coverage_percentage)) = (
                            coverage.get("file_path").and_then(|v| v.as_str()),
                            coverage.get("coverage_percentage").and_then(|v| v.as_f64())
                        ) {
                            let filename = file_path.split('/').last().unwrap_or(file_path);
                            test_trends.push((filename.to_string(), coverage_percentage / 100.0)); // Convert to 0-1 range
                        }
                    }
                }
            }

            // If no coverage data, generate trends from test files and their test counts
            if test_trends.is_empty() {
                if let Some(test_files_data) = data.get("test_files") {
                    if let Some(test_files_array) = test_files_data.as_array() {
                        for test_file in test_files_array.iter().take(self.config.max_list_items) {
                            if let (Some(file_path), Some(test_count)) = (
                                test_file.get("file_path").and_then(|v| v.as_str()),
                                test_file.get("test_count").and_then(|v| v.as_u64())
                            ) {
                                let filename = file_path.split('/').last().unwrap_or(file_path);
                                // Convert test count to a relative score (normalize to 0-1)
                                let score = (test_count as f64 / 20.0).min(1.0); // Assume 20+ tests = 100%
                                test_trends.push((filename.to_string(), score));
                            }
                        }
                    }
                }
            }

            // Sort trends by coverage (highest first)
            test_trends.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        }

        Ok(ViewContent::Testing {
            test_coverage,
            test_files,
            uncovered_files,
            test_trends,
        })
    }

    fn generate_flow_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let flow_data = result.get_view_data("flow");

        let mut data_flows = Vec::new();
        let mut control_flows = Vec::new();
        let mut flow_complexity = 0.0;
        let mut bottlenecks = Vec::new();

        if let Some(data) = flow_data {
            // Extract data flow nodes and edges
            if let Some(nodes) = data.get("data_flow_nodes") {
                if let Some(edges) = data.get("data_flow_edges") {
                    if let (Some(nodes_array), Some(edges_array)) = (nodes.as_array(), edges.as_array()) {
                        // Process data flow edges
                        for edge in edges_array.iter().take(self.config.max_chart_points) {
                            if let (Some(from), Some(to), Some(edge_type)) = (
                                edge.get("from_node"),
                                edge.get("to_node"),
                                edge.get("edge_type")
                            ) {
                                if let (Some(from_str), Some(to_str), Some(type_str)) = (
                                    from.as_str(),
                                    to.as_str(),
                                    edge_type.as_str()
                                ) {
                                    let complexity = self.calculate_edge_complexity(type_str);

                                    if matches!(type_str, "assignment" | "parameter" | "return_value") {
                                        data_flows.push(FlowItem {
                                            from: from_str.to_string(),
                                            to: to_str.to_string(),
                                            flow_type: type_str.to_string(),
                                            complexity,
                                        });
                                    } else if type_str == "function_call" {
                                        control_flows.push(FlowItem {
                                            from: from_str.to_string(),
                                            to: to_str.to_string(),
                                            flow_type: type_str.to_string(),
                                            complexity,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Calculate flow complexity based on patterns
            if let Some(patterns) = data.get("flow_patterns") {
                if let Some(patterns_array) = patterns.as_array() {
                    let mut total_impact = 0.0;
                    let mut pattern_count = 0;

                    for pattern in patterns_array {
                        if let Some(impact) = pattern.get("impact_score") {
                            if let Some(impact_val) = impact.as_f64() {
                                total_impact += impact_val;
                                pattern_count += 1;
                            }
                        }

                        // Extract bottlenecks from patterns
                        if let Some(pattern_type) = pattern.get("pattern_type") {
                            if let Some(type_str) = pattern_type.as_str() {
                                if matches!(type_str, "deep_nesting" | "circular_dependency") {
                                    if let Some(description) = pattern.get("description") {
                                        if let Some(desc_str) = description.as_str() {
                                            bottlenecks.push(desc_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if pattern_count > 0 {
                        flow_complexity = total_impact / pattern_count as f64;
                    }
                }
            }

            // Extract additional bottlenecks from variable lifecycles
            if let Some(lifecycles) = data.get("variable_lifecycles") {
                if let Some(lifecycles_array) = lifecycles.as_array() {
                    for lifecycle in lifecycles_array.iter().take(5) {
                        if let Some(scope_depth) = lifecycle.get("scope_depth") {
                            if let Some(depth_val) = scope_depth.as_u64() {
                                if depth_val > 5 {
                                    if let Some(var_name) = lifecycle.get("variable_name") {
                                        if let Some(name_str) = var_name.as_str() {
                                            bottlenecks.push(format!("Deep scoped variable: {}", name_str));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Normalize complexity to 0-1 range
            flow_complexity = flow_complexity.min(1.0).max(0.0);
        }

        Ok(ViewContent::Flow {
            data_flows,
            control_flows,
            flow_complexity,
            bottlenecks,
        })
    }

    /// Generate tree-sitter analysis view content
    fn generate_tree_sitter_view(&self, result: &AnalysisResult) -> Result<ViewContent> {
        let tree_sitter_data = result.get_view_data("tree_sitter_analysis");

        let mut import_summary = TreeSitterImportSummary {
            total_imports: 0,
            unique_modules: 0,
            external_dependencies: Vec::new(),
            internal_dependencies: Vec::new(),
            wildcard_imports: 0,
            aliased_imports: 0,
            analysis_method: "Enhanced Tree-sitter Queries".to_string(),
        };

        let mut language_features = Vec::new();
        let mut symbols = Vec::new();
        let mut highlight_summary = TreeSitterHighlightSummary {
            total_highlights: 0,
            semantic_types: HashMap::new(),
            functions_found: 0,
            types_found: 0,
            variables_found: 0,
        };

        if let Some(data) = tree_sitter_data {
            // Extract import analysis
            if let Some(imports_data) = data.get("imports") {
                if let Some(summary) = imports_data.get("summary") {
                    if let Some(total) = summary.get("total_imports") {
                        import_summary.total_imports = total.as_u64().unwrap_or(0) as usize;
                    }
                    if let Some(unique) = summary.get("unique_modules") {
                        import_summary.unique_modules = unique.as_u64().unwrap_or(0) as usize;
                    }
                    if let Some(deps) = summary.get("external_dependencies") {
                        if let Some(deps_array) = deps.as_array() {
                            import_summary.external_dependencies = deps_array.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect();
                        }
                    }
                }
            }

            // Extract language features
            if let Some(features_data) = data.get("language_features") {
                if let Some(features_array) = features_data.as_array() {
                    for feature in features_array.iter().take(self.config.max_list_items) {
                        if let (Some(feature_type), Some(count)) = (
                            feature.get("feature_type").and_then(|v| v.as_str()),
                            feature.get("count").and_then(|v| v.as_u64()),
                        ) {
                            language_features.push(LanguageFeature {
                                feature_type: feature_type.to_string(),
                                count: count as usize,
                                description: feature.get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Language feature")
                                    .to_string(),
                                files: Vec::new(), // TODO: extract files list
                            });
                        }
                    }
                }
            }

            // Extract symbols
            if let Some(symbols_data) = data.get("symbols") {
                if let Some(symbols_array) = symbols_data.as_array() {
                    for symbol in symbols_array.iter().take(self.config.max_list_items) {
                        if let (Some(name), Some(symbol_type)) = (
                            symbol.get("name").and_then(|v| v.as_str()),
                            symbol.get("type").and_then(|v| v.as_str()),
                        ) {
                            symbols.push(TreeSitterSymbol {
                                name: name.to_string(),
                                symbol_type: symbol_type.to_string(),
                                scope: symbol.get("scope").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                line: symbol.get("line")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as usize,
                                file: symbol.get("file")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                            });
                        }
                    }
                }
            }

            // Extract highlight summary
            if let Some(highlights_data) = data.get("highlights") {
                if let Some(summary) = highlights_data.get("summary") {
                    if let Some(total) = summary.get("total_highlights") {
                        highlight_summary.total_highlights = total.as_u64().unwrap_or(0) as usize;
                    }
                    if let Some(functions) = summary.get("functions_found") {
                        highlight_summary.functions_found = functions.as_u64().unwrap_or(0) as usize;
                    }
                    if let Some(types) = summary.get("types_found") {
                        highlight_summary.types_found = types.as_u64().unwrap_or(0) as usize;
                    }
                    if let Some(variables) = summary.get("variables_found") {
                        highlight_summary.variables_found = variables.as_u64().unwrap_or(0) as usize;
                    }
                }
            }
        }

        // Group symbols by type
        let mut symbols_by_type = HashMap::new();
        for symbol in symbols {
            symbols_by_type.entry(symbol.symbol_type.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }

        Ok(ViewContent::TreeSitterAnalysis {
            imports_summary: import_summary,
            symbols_by_type,
            highlights_summary: highlight_summary,
            semantic_complexity: 0.5, // TODO: calculate actual complexity
            language_features,
        })
    }

    /// Calculate complexity for a flow edge type
    fn calculate_edge_complexity(&self, edge_type: &str) -> f64 {
        match edge_type {
            "assignment" => 0.2,
            "parameter" => 0.3,
            "return_value" => 0.4,
            "function_call" => 0.6,
            _ => 0.5,
        }
    }

    /// Render header section
    fn render_header(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        let header = Paragraph::new(view.title.clone())
            .block(Block::default().borders(Borders::ALL))
            .style(self.get_header_style());
        frame.render_widget(header, area);
    }

    /// Render footer section
    fn render_footer(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        let footer = Paragraph::new(format!("Updated: {}", view.timestamp.format("%Y-%m-%d %H:%M:%S UTC")))
            .style(self.get_footer_style());
        frame.render_widget(footer, area);
    }

    /// Render summary content
    fn render_summary_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Summary { health_score, files_analyzed, critical_issues, recommendations, metrics } = &view.content {
            // Create 4-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            // Top Left: Health Score Overview
            self.render_health_score_overview(frame, top_chunks[0], *health_score, *files_analyzed, *critical_issues);

            // Top Right: Core Metrics
            self.render_core_metrics(frame, top_chunks[1], metrics);

            // Bottom Left: Quality & Security Metrics
            self.render_quality_security_metrics(frame, bottom_chunks[0], metrics);

            // Bottom Right: Recommendations
            self.render_recommendations_panel(frame, bottom_chunks[1], recommendations);
        }
    }

    /// Render health score overview panel
    fn render_health_score_overview(&self, frame: &mut Frame, area: Rect, health_score: f64, files_analyzed: usize, critical_issues: usize) {
        let percentage = (health_score) as u16;
        let health_color = self.get_health_color(health_score / 100.0);

        let health_icon = match health_score {
            s if s >= 90.0 => "💚",
            s if s >= 70.0 => "💛",
            s if s >= 50.0 => "🧡",
            _ => "❤️",
        };

        let status = match health_score {
            s if s >= 90.0 => "Excellent",
            s if s >= 70.0 => "Good",
            s if s >= 50.0 => "Fair",
            s if s >= 30.0 => "Poor",
            _ => "Critical",
        };

        let overview_text = format!(
            "{} Overall Health\n\n{:.1}% - {}\n\n📁 {} files analyzed\n🚨 {} critical issues",
            health_icon,
            health_score,
            status,
            files_analyzed,
            critical_issues
        );

        let gauge = Gauge::default()
            .block(Block::default().title("📊 Project Overview").borders(Borders::ALL))
            .gauge_style(health_color)
            .percent(percentage);

        frame.render_widget(gauge, area);

        // Render overview text
        let text_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        };

        let text_widget = Paragraph::new(overview_text)
            .style(health_color)
            .alignment(Alignment::Center);

        frame.render_widget(text_widget, text_area);
    }

    /// Render core metrics panel
    fn render_core_metrics(&self, frame: &mut Frame, area: Rect, metrics: &HashMap<String, f64>) {
        let core_metrics = ["Files Analyzed", "Analysis Duration (s)", "Total Functions", "Total Classes", "Lines of Code"];

        let mut core_items = Vec::new();
        for metric in &core_metrics {
            if let Some(value) = metrics.get(*metric) {
                let icon = match *metric {
                    "Files Analyzed" => "📁",
                    "Analysis Duration (s)" => "⏱️",
                    "Total Functions" => "🔧",
                    "Total Classes" => "🏗️",
                    "Lines of Code" => "📝",
                    _ => "📊",
                };
                let formatted_value = if metric.contains("Duration") {
                    format!("{:.2}s", value)
                } else {
                    format!("{:.0}", value)
                };
                core_items.push(format!("{} {}: {}", icon, metric, formatted_value));
            }
        }

        if core_items.is_empty() {
            core_items.push("📊 No core metrics available".to_string());
        }

        let metrics_items: Vec<ListItem> = core_items.iter()
            .map(|item| ListItem::new(item.clone()))
            .collect();

        let metrics_widget = List::new(metrics_items)
            .block(Block::default().title("📈 Core Metrics").borders(Borders::ALL))
            .style(self.get_content_style());
        frame.render_widget(metrics_widget, area);
    }

    /// Render quality and security metrics panel
    fn render_quality_security_metrics(&self, frame: &mut Frame, area: Rect, metrics: &HashMap<String, f64>) {
        let quality_metrics = ["Security Score", "Test Coverage %", "Security Vulnerabilities", "Dependencies", "Circular Dependencies", "Performance Hotspots"];

        let mut quality_items = Vec::new();
        for metric in &quality_metrics {
            if let Some(value) = metrics.get(*metric) {
                let (icon, color) = match *metric {
                    "Security Score" => {
                        let color = if *value >= 80.0 { Color::Green } else if *value >= 60.0 { Color::Yellow } else { Color::Red };
                        ("🔐", color)
                    },
                    "Test Coverage %" => {
                        let color = if *value >= 80.0 { Color::Green } else if *value >= 60.0 { Color::Yellow } else { Color::Red };
                        ("🧪", color)
                    },
                    "Security Vulnerabilities" => ("🚨", if *value > 0.0 { Color::Red } else { Color::Green }),
                    "Dependencies" => ("📦", Color::White),
                    "Circular Dependencies" => ("🔄", if *value > 0.0 { Color::Red } else { Color::Green }),
                    "Performance Hotspots" => ("⚡", if *value > 0.0 { Color::Yellow } else { Color::Green }),
                    _ => ("📊", Color::White),
                };

                let formatted_value = if metric.contains("%") {
                    format!("{:.1}%", value)
                } else {
                    format!("{:.0}", value)
                };
                quality_items.push((format!("{} {}: {}", icon, metric, formatted_value), color));
            }
        }

        if quality_items.is_empty() {
            quality_items.push(("📊 No quality/security metrics available".to_string(), Color::White));
        }

        let mut rendered_items = Vec::new();
        for (item, _color) in quality_items.iter().take(8) {
            rendered_items.push(ListItem::new(item.clone()));
        }

        let quality_widget = List::new(rendered_items)
            .block(Block::default().title("🛡️ Quality & Security").borders(Borders::ALL))
            .style(self.get_content_style());
        frame.render_widget(quality_widget, area);
    }

    /// Render recommendations panel
    fn render_recommendations_panel(&self, frame: &mut Frame, area: Rect, recommendations: &[String]) {
        if recommendations.is_empty() {
            let placeholder = Paragraph::new("✅ No specific recommendations\nYour codebase looks good!")
                .block(Block::default().title("💡 Recommendations").borders(Borders::ALL))
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, area);
        } else {
            let rec_items: Vec<ListItem> = recommendations.iter()
                .take(self.config.max_list_items)
                .enumerate()
                .map(|(i, rec)| {
                    let priority_icon = match i {
                        0..=2 => "🔴", // High priority
                        3..=5 => "🟡", // Medium priority
                        _ => "🔵",     // Low priority
                    };
                    ListItem::new(format!("{} {}", priority_icon, rec))
                })
                .collect();

            let title = format!("💡 Recommendations ({})", recommendations.len());

            let rec_widget = List::new(rec_items)
                .block(Block::default().title(title).borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(rec_widget, area);
        }
    }

    /// Render topology content
    fn render_topology_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Topology { file_tree, language_distribution, complexity_distribution, coupling_metrics } = &view.content {
            // Create 4-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            // Top Left: Language Distribution
            self.render_language_distribution(frame, top_chunks[0], language_distribution);

            // Top Right: File Structure Overview
            self.render_file_structure_overview(frame, top_chunks[1], file_tree);

            // Bottom Left: Complexity Distribution
            self.render_complexity_distribution(frame, bottom_chunks[0], complexity_distribution);

            // Bottom Right: Coupling Metrics
            self.render_coupling_metrics(frame, bottom_chunks[1], coupling_metrics);
        }
    }

    /// Render language distribution panel
    fn render_language_distribution(&self, frame: &mut Frame, area: Rect, language_distribution: &HashMap<String, usize>) {
        if language_distribution.is_empty() {
            let placeholder = Paragraph::new("📄 No language data available")
                .block(Block::default().title("Language Distribution").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(placeholder, area);
        } else {
            let lang_data: Vec<(&str, u64)> = language_distribution.iter()
                .take(8) // Limit for display
                .map(|(lang, count)| (lang.as_str(), *count as u64))
                .collect();

            let lang_chart = BarChart::default()
                .block(Block::default().title("📊 Language Distribution").borders(Borders::ALL))
                .data(&lang_data)
                .bar_width(3)
                .bar_style(self.get_chart_style())
                .value_style(self.get_content_style());
            frame.render_widget(lang_chart, area);
        }
    }

    /// Render file structure overview panel
    fn render_file_structure_overview(&self, frame: &mut Frame, area: Rect, file_tree: &FileTree) {
        let mut tree_info = vec![
            format!("📁 Total Files: {}", file_tree.total_files),
            format!("📂 Total Directories: {}", file_tree.total_directories),
        ];

        // Add sample files if available
        if !file_tree.root.children.is_empty() {
            tree_info.push("".to_string()); // Empty line
            tree_info.push("📋 Sample Files:".to_string());
            for child in file_tree.root.children.iter().take(6) {
                let size_info = if let Some(size) = child.size {
                    format!(" ({} bytes)", size)
                } else {
                    "".to_string()
                };
                tree_info.push(format!("  📄 {}{}", child.name, size_info));
            }
            if file_tree.root.children.len() > 6 {
                tree_info.push(format!("  ... and {} more files", file_tree.root.children.len() - 6));
            }
        }

        let tree_items: Vec<ListItem> = tree_info.iter()
            .map(|info| ListItem::new(info.clone()))
            .collect();

        let tree_widget = List::new(tree_items)
            .block(Block::default().title("🏗️ Project Structure").borders(Borders::ALL))
            .style(self.get_content_style());
        frame.render_widget(tree_widget, area);
    }

    /// Render complexity distribution panel
    fn render_complexity_distribution(&self, frame: &mut Frame, area: Rect, complexity_distribution: &[(String, f64)]) {
        if complexity_distribution.is_empty() {
            let placeholder = Paragraph::new("📊 No complexity data available")
                .block(Block::default().title("Complexity Distribution").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(placeholder, area);
        } else {
            let complexity_items: Vec<ListItem> = complexity_distribution.iter()
                .take(self.config.max_list_items)
                .map(|(file, complexity)| {
                    let complexity_icon = match *complexity {
                        c if c >= 10.0 => "🔴",
                        c if c >= 7.0 => "🟠",
                        c if c >= 4.0 => "🟡",
                        _ => "🟢",
                    };
                    let content = format!("{} {}: {:.0}", complexity_icon, file, complexity);
                    ListItem::new(content)
                })
                .collect();

            let complexity_widget = List::new(complexity_items)
                .block(Block::default().title("🧮 Complexity by File").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(complexity_widget, area);
        }
    }

    /// Render coupling metrics panel
    fn render_coupling_metrics(&self, frame: &mut Frame, area: Rect, coupling_metrics: &[(String, f64)]) {
        if coupling_metrics.is_empty() {
            let placeholder = Paragraph::new("🔗 No coupling data available")
                .block(Block::default().title("Coupling Metrics").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(placeholder, area);
        } else {
            let coupling_items: Vec<ListItem> = coupling_metrics.iter()
                .take(self.config.max_list_items)
                .map(|(file, coupling)| {
                    let coupling_icon = match *coupling {
                        c if c >= 0.8 => "🔴",
                        c if c >= 0.6 => "🟠",
                        c if c >= 0.4 => "🟡",
                        _ => "🟢",
                    };
                    let content = format!("{} {}: {:.2}", coupling_icon, file, coupling);
                    ListItem::new(content)
                })
                .collect();

            let coupling_widget = List::new(coupling_items)
                .block(Block::default().title("🔗 Coupling Metrics").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(coupling_widget, area);
        }
    }

    /// Render quality content
    fn render_quality_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Quality { health_score, issues_by_severity, top_problematic_files, complexity_trend, maintainability_scores } = &view.content {
            // Create 4-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            // Top Left: Health Score Gauge
            self.render_health_score_gauge(frame, top_chunks[0], *health_score);

            // Top Right: Issues by Severity
            self.render_issues_by_severity(frame, top_chunks[1], issues_by_severity);

            // Bottom Left: Complexity Trend
            self.render_complexity_trend(frame, bottom_chunks[0], complexity_trend);

            // Bottom Right: Maintainability Scores
            self.render_maintainability_scores(frame, bottom_chunks[1], maintainability_scores, top_problematic_files);
        }
    }

    /// Render health score gauge
    fn render_health_score_gauge(&self, frame: &mut Frame, area: Rect, health_score: f64) {
        let percentage = (health_score * 100.0) as u16;
        let health_color = self.get_health_color(health_score);

        let health_icon = match health_score {
            s if s >= 0.9 => "💚",
            s if s >= 0.7 => "💛",
            s if s >= 0.5 => "🧡",
            _ => "❤️",
        };

        let gauge_text = format!(
            "{} Health Score\n\n{:.1}%\n\n{}",
            health_icon,
            health_score * 100.0,
            match health_score {
                s if s >= 0.9 => "Excellent",
                s if s >= 0.7 => "Good",
                s if s >= 0.5 => "Fair",
                s if s >= 0.3 => "Poor",
                _ => "Critical",
            }
        );

        let gauge = Gauge::default()
            .block(Block::default().title("📊 Code Health").borders(Borders::ALL))
            .gauge_style(health_color)
            .percent(percentage);

        frame.render_widget(gauge, area);

        // Render text on top
        let text_area = Rect {
            x: area.x + 2,
            y: area.y + area.height / 2,
            width: area.width.saturating_sub(4),
            height: 3,
        };

        let text_widget = Paragraph::new(gauge_text)
            .style(health_color)
            .alignment(Alignment::Center);

        frame.render_widget(text_widget, text_area);
    }

    /// Render issues by severity panel
    fn render_issues_by_severity(&self, frame: &mut Frame, area: Rect, issues_by_severity: &HashMap<String, usize>) {
        if issues_by_severity.is_empty() {
            let placeholder = Paragraph::new("✅ No quality issues detected")
                .block(Block::default().title("⚠️ Issues by Severity").borders(Borders::ALL))
                .style(Style::default().fg(Color::Green));
            frame.render_widget(placeholder, area);
        } else {
            let issues_data: Vec<(&str, u64)> = issues_by_severity.iter()
                .map(|(severity, count)| (severity.as_str(), *count as u64))
                .collect();

            let issues_chart = BarChart::default()
                .block(Block::default().title("⚠️ Issues by Severity").borders(Borders::ALL))
                .data(&issues_data)
                .bar_width(4)
                .bar_style(Style::default().fg(Color::Red))
                .value_style(self.get_content_style());
            frame.render_widget(issues_chart, area);
        }
    }

    /// Render complexity trend panel
    fn render_complexity_trend(&self, frame: &mut Frame, area: Rect, complexity_trend: &[(String, f64)]) {
        if complexity_trend.is_empty() {
            let placeholder = Paragraph::new("📈 No complexity data available")
                .block(Block::default().title("🧮 Complexity Trend").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(placeholder, area);
        } else {
            let complexity_items: Vec<ListItem> = complexity_trend.iter()
                .take(self.config.max_list_items)
                .map(|(file, complexity)| {
                    let complexity_icon = match *complexity {
                        c if c >= 7.0 => "🔴",
                        c if c >= 5.0 => "🟠",
                        c if c >= 3.0 => "🟡",
                        _ => "🟢",
                    };
                    let content = format!("{} {}: {:.1}", complexity_icon, file, complexity);
                    ListItem::new(content)
                })
                .collect();

            let complexity_widget = List::new(complexity_items)
                .block(Block::default().title("🧮 Complexity Trend").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(complexity_widget, area);
        }
    }

    /// Render maintainability scores panel
    fn render_maintainability_scores(&self, frame: &mut Frame, area: Rect, maintainability_scores: &[(String, f64)], top_problematic_files: &[(String, f64)]) {
        // Prefer problematic files if available, otherwise show maintainability scores
        let display_data = if !top_problematic_files.is_empty() {
            top_problematic_files
        } else {
            maintainability_scores
        };

        if display_data.is_empty() {
            let placeholder = Paragraph::new("📋 No maintainability data available")
                .block(Block::default().title("🛠️ Maintainability").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(placeholder, area);
        } else {
            let file_items: Vec<ListItem> = display_data.iter()
                .take(self.config.max_list_items)
                .map(|(file, score)| {
                    let score_icon = match *score {
                        s if s >= 0.8 => "🟢",
                        s if s >= 0.6 => "🟡",
                        s if s >= 0.4 => "🟠",
                        _ => "🔴",
                    };
                    let content = format!("{} {}: {:.1}", score_icon, file, score);
                    ListItem::new(content)
                })
                .collect();

            let title = if !top_problematic_files.is_empty() {
                "🚨 Most Problematic Files"
            } else {
                "🛠️ Maintainability Scores"
            };

            let files_widget = List::new(file_items)
                .block(Block::default().title(title).borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(files_widget, area);
        }
    }

    /// Render security content
    fn render_security_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Security { risk_level, vulnerabilities_by_severity, top_security_issues, security_score, files_with_issues } = &view.content {
            // Create 4-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            // Top Left: Security Score Gauge
            self.render_security_score_gauge(frame, top_chunks[0], *security_score, risk_level);

            // Top Right: Vulnerabilities by Severity
            self.render_vulnerabilities_by_severity(frame, top_chunks[1], vulnerabilities_by_severity);

            // Bottom Left: Top Security Issues
            self.render_top_security_issues(frame, bottom_chunks[0], top_security_issues);

            // Bottom Right: Files with Issues
            self.render_files_with_security_issues(frame, bottom_chunks[1], files_with_issues);
        }
    }

    /// Render security score gauge
    fn render_security_score_gauge(&self, frame: &mut Frame, area: Rect, security_score: f64, risk_level: &str) {
        let percentage = security_score as u16;
        let score_color = self.get_health_color(security_score / 100.0);

        let risk_icon = match risk_level {
            "critical" => "🚨",
            "high" => "⚠️",
            "medium" => "🟡",
            "low" => "🟢",
            _ => "🔍",
        };

        let gauge_text = format!(
            "{} Security Score\n\n{:.1}%\n\nRisk: {}",
            risk_icon,
            security_score,
            risk_level.to_uppercase()
        );

        let gauge = Gauge::default()
            .block(Block::default().title("🔐 Security Assessment").borders(Borders::ALL))
            .gauge_style(score_color)
            .percent(percentage);

        frame.render_widget(gauge, area);

        // Render text on top
        let text_area = Rect {
            x: area.x + 2,
            y: area.y + area.height / 2,
            width: area.width.saturating_sub(4),
            height: 3,
        };

        let text_widget = Paragraph::new(gauge_text)
            .style(score_color)
            .alignment(Alignment::Center);

        frame.render_widget(text_widget, text_area);
    }

    /// Render vulnerabilities by severity panel
    fn render_vulnerabilities_by_severity(&self, frame: &mut Frame, area: Rect, vulnerabilities_by_severity: &HashMap<String, usize>) {
        if vulnerabilities_by_severity.is_empty() {
            let placeholder = Paragraph::new("✅ No security vulnerabilities detected")
                .block(Block::default().title("🛡️ Vulnerabilities").borders(Borders::ALL))
                .style(Style::default().fg(Color::Green));
            frame.render_widget(placeholder, area);
        } else {
            let vuln_data: Vec<(&str, u64)> = vulnerabilities_by_severity.iter()
                .map(|(severity, count)| (severity.as_str(), *count as u64))
                .collect();

            let vuln_chart = BarChart::default()
                .block(Block::default().title("🛡️ Vulnerabilities by Severity").borders(Borders::ALL))
                .data(&vuln_data)
                .bar_width(4)
                .bar_style(Style::default().fg(Color::Red))
                .value_style(self.get_content_style());
            frame.render_widget(vuln_chart, area);
        }
    }

    /// Render top security issues panel
    fn render_top_security_issues(&self, frame: &mut Frame, area: Rect, top_security_issues: &[SecurityIssue]) {
        if top_security_issues.is_empty() {
            let placeholder = Paragraph::new("✅ No security issues detected")
                .block(Block::default().title("🚨 Security Issues").borders(Borders::ALL))
                .style(Style::default().fg(Color::Green));
            frame.render_widget(placeholder, area);
        } else {
            let issue_items: Vec<ListItem> = top_security_issues.iter()
                .take(self.config.max_list_items)
                .map(|issue| {
                    let severity_icon = match issue.severity.as_str() {
                        "critical" => "🚨",
                        "high" => "⚠️",
                        "medium" => "🟡",
                        "low" => "🔵",
                        _ => "🔍",
                    };
                    let line_info = issue.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    let content = format!("{} {} ({}{})", severity_icon, issue.description, issue.file, line_info);
                    ListItem::new(content)
                })
                .collect();

            let issues_widget = List::new(issue_items)
                .block(Block::default().title("🚨 Top Security Issues").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(issues_widget, area);
        }
    }

    /// Render files with security issues panel
    fn render_files_with_security_issues(&self, frame: &mut Frame, area: Rect, files_with_issues: &[String]) {
        if files_with_issues.is_empty() {
            let placeholder = Paragraph::new("✅ No files with security issues")
                .block(Block::default().title("📁 Affected Files").borders(Borders::ALL))
                .style(Style::default().fg(Color::Green));
            frame.render_widget(placeholder, area);
        } else {
            let file_items: Vec<ListItem> = files_with_issues.iter()
                .take(self.config.max_list_items)
                .map(|file| {
                    let content = format!("🔓 {}", file);
                    ListItem::new(content)
                })
                .collect();

            let title = format!("📁 Affected Files ({})", files_with_issues.len());

            let files_widget = List::new(file_items)
                .block(Block::default().title(title).borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(files_widget, area);

            if files_with_issues.len() > self.config.max_list_items {
                // Show count of additional files
                let additional_area = Rect {
                    x: area.x + 2,
                    y: area.y + area.height.saturating_sub(2),
                    width: area.width.saturating_sub(4),
                    height: 1,
                };

                let additional_text = Paragraph::new(format!("... and {} more files",
                    files_with_issues.len() - self.config.max_list_items))
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(additional_text, additional_area);
            }
        }
    }

    /// Render dependencies content
    fn render_dependencies_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Dependencies {
            total_dependencies,
            circular_dependencies,
            dependency_graph,
            coupling_analysis,
            external_dependencies
        } = &view.content {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left side - Summary and circular dependencies
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(6), Constraint::Min(0)])
                .split(layout[0]);

            // Dependencies summary
            let summary_info = vec![
                format!("Total Dependencies: {}", total_dependencies),
                format!("Graph Nodes: {}", dependency_graph.nodes.len()),
                format!("Graph Edges: {}", dependency_graph.edges.len()),
                format!("Circular Dependencies: {}", circular_dependencies.len()),
            ];
            let summary_items: Vec<ListItem> = summary_info.iter()
                .map(|info| ListItem::new(info.clone()))
                .collect();

            let summary_widget = List::new(summary_items)
                .block(Block::default().title("Dependency Summary").borders(Borders::ALL))
                .style(self.get_content_style());
            frame.render_widget(summary_widget, left_layout[0]);

            // Circular dependencies
            if !circular_dependencies.is_empty() {
                let circular_items: Vec<ListItem> = circular_dependencies.iter()
                    .take(self.config.max_list_items)
                    .map(|cycle| {
                        // Truncate long cycle paths for display
                        let display_cycle = if cycle.len() > 60 {
                            format!("{}...", &cycle[..57])
                        } else {
                            cycle.clone()
                        };
                        ListItem::new(display_cycle)
                    })
                    .collect();

                let circular_widget = List::new(circular_items)
                    .block(Block::default().title("Circular Dependencies").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Red));
                frame.render_widget(circular_widget, left_layout[1]);
            } else {
                let no_circular = Paragraph::new("No circular dependencies detected ✓")
                    .block(Block::default().title("Circular Dependencies").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                frame.render_widget(no_circular, left_layout[1]);
            }

            // Right side - Coupling analysis and external dependencies
            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

            // Coupling analysis
            if !coupling_analysis.is_empty() {
                let coupling_items: Vec<ListItem> = coupling_analysis.iter()
                    .take(self.config.max_list_items)
                    .map(|(file, score)| {
                        let color = match *score {
                            s if s >= 0.8 => Color::Red,
                            s if s >= 0.6 => Color::Yellow,
                            s if s >= 0.4 => Color::LightYellow,
                            _ => Color::Green,
                        };
                        let content = format!("{:.2}: {}", score, file);
                        ListItem::new(content).style(Style::default().fg(color))
                    })
                    .collect();

                let coupling_widget = List::new(coupling_items)
                    .block(Block::default().title("Coupling Analysis (Score: File)").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(coupling_widget, right_layout[0]);
            } else {
                let no_coupling = Paragraph::new("No coupling data available")
                    .block(Block::default().title("Coupling Analysis").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(no_coupling, right_layout[0]);
            }

            // External dependencies
            if !external_dependencies.is_empty() {
                let external_items: Vec<ListItem> = external_dependencies.iter()
                    .take(self.config.max_list_items)
                    .map(|dep| ListItem::new(dep.clone()))
                    .collect();

                let external_widget = List::new(external_items)
                    .block(Block::default().title("Top External Dependencies").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(external_widget, right_layout[1]);
            } else {
                let no_external = Paragraph::new("No external dependencies found")
                    .block(Block::default().title("External Dependencies").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(no_external, right_layout[1]);
            }
        }
    }

    fn render_performance_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Performance {
            hotspots,
            bottlenecks,
            performance_score,
            slow_functions
        } = &view.content {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left side - Performance score and hotspots
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(layout[0]);

            // Performance score gauge
            let score_percent = (*performance_score * 100.0).min(100.0) as u16;
            let performance_gauge = Gauge::default()
                .block(Block::default().title("Performance Score").borders(Borders::ALL))
                .gauge_style(self.get_performance_color(*performance_score))
                .percent(score_percent)
                .label(format!("{:.1}%", performance_score * 100.0));
            frame.render_widget(performance_gauge, left_layout[0]);

            // Performance hotspots
            if !hotspots.is_empty() {
                let hotspot_items: Vec<ListItem> = hotspots.iter()
                    .take(self.config.max_list_items)
                    .map(|hotspot| {
                        let content = format!("{} ({}): {:.1}ms",
                            hotspot.function,
                            hotspot.file.split('/').last().unwrap_or(&hotspot.file),
                            hotspot.estimated_time);
                        let style = match hotspot.estimated_time {
                            t if t >= 1000.0 => Style::default().fg(Color::Red),
                            t if t >= 500.0 => Style::default().fg(Color::Yellow),
                            _ => Style::default().fg(Color::White),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let hotspots_widget = List::new(hotspot_items)
                    .block(Block::default().title("Performance Hotspots").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(hotspots_widget, left_layout[1]);
            } else {
                let no_hotspots = Paragraph::new("No performance hotspots detected ✓")
                    .block(Block::default().title("Performance Hotspots").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                frame.render_widget(no_hotspots, left_layout[1]);
            }

            // Right side - Bottlenecks and slow functions
            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

            // Bottlenecks
            if !bottlenecks.is_empty() {
                let bottleneck_items: Vec<ListItem> = bottlenecks.iter()
                    .take(self.config.max_list_items)
                    .map(|bottleneck| ListItem::new(bottleneck.clone()))
                    .collect();

                let bottlenecks_widget = List::new(bottleneck_items)
                    .block(Block::default().title("Performance Bottlenecks").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Red));
                frame.render_widget(bottlenecks_widget, right_layout[0]);
            } else {
                let no_bottlenecks = Paragraph::new("No bottlenecks identified ✓")
                    .block(Block::default().title("Performance Bottlenecks").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                frame.render_widget(no_bottlenecks, right_layout[0]);
            }

            // Slow functions (optimization opportunities)
            if !slow_functions.is_empty() {
                let slow_items: Vec<ListItem> = slow_functions.iter()
                    .take(self.config.max_list_items)
                    .map(|(function, impact)| {
                        let content = format!("{}: {:.2}x improvement potential", function, impact);
                        let style = match *impact {
                            i if i >= 3.0 => Style::default().fg(Color::Red),
                            i if i >= 2.0 => Style::default().fg(Color::Yellow),
                            _ => Style::default().fg(Color::White),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let slow_widget = List::new(slow_items)
                    .block(Block::default().title("Optimization Opportunities").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(slow_widget, right_layout[1]);
            } else {
                let no_optimizations = Paragraph::new("No obvious optimization opportunities")
                    .block(Block::default().title("Optimization Opportunities").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(no_optimizations, right_layout[1]);
            }
        }
    }

    fn render_evolution_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Evolution {
            commit_activity,
            author_contributions,
            file_stability,
            churn_metrics
        } = &view.content {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left side - Commit activity and author contributions
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[0]);

            // Commit activity patterns
            if !commit_activity.is_empty() {
                let activity_items: Vec<ListItem> = commit_activity.iter()
                    .take(self.config.max_list_items)
                    .map(|(pattern, frequency)| {
                        let pattern_display = match pattern.as_str() {
                            "feature_development" => "🚀 Feature Development",
                            "bug_fixes" => "🐛 Bug Fixes",
                            "refactoring" => "🔧 Refactoring",
                            "documentation" => "📚 Documentation",
                            "testing" => "🧪 Testing",
                            "maintenance" => "⚙️ Maintenance",
                            _ => pattern,
                        };
                        let content = format!("{}: {} commits", pattern_display, frequency);
                        let style = match *frequency {
                            f if f >= 50 => Style::default().fg(Color::Green),
                            f if f >= 20 => Style::default().fg(Color::Yellow),
                            f if f >= 10 => Style::default().fg(Color::White),
                            _ => Style::default().fg(Color::Gray),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let activity_widget = List::new(activity_items)
                    .block(Block::default().title("Commit Activity Patterns").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(activity_widget, left_layout[0]);
            } else {
                let no_activity = Paragraph::new("No commit patterns detected\n\nThis may indicate:\n• New repository\n• Limited git history\n• Single contributor")
                    .block(Block::default().title("Commit Activity Patterns").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                frame.render_widget(no_activity, left_layout[0]);
            }

            // Author contributions
            if !author_contributions.is_empty() {
                let author_items: Vec<ListItem> = author_contributions.iter()
                    .take(self.config.max_list_items)
                    .map(|(author, commits)| {
                        let display_name = if author.len() > 20 {
                            format!("{}...", &author[..17])
                        } else {
                            author.clone()
                        };
                        let content = format!("👤 {}: {} commits", display_name, commits);
                        let style = match *commits {
                            c if c >= 100 => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            c if c >= 50 => Style::default().fg(Color::Green),
                            c if c >= 20 => Style::default().fg(Color::Yellow),
                            c if c >= 10 => Style::default().fg(Color::White),
                            _ => Style::default().fg(Color::Gray),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let authors_widget = List::new(author_items)
                    .block(Block::default().title("Author Contributions").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(authors_widget, left_layout[1]);
            } else {
                let no_authors = Paragraph::new("No author data available")
                    .block(Block::default().title("Author Contributions").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                frame.render_widget(no_authors, left_layout[1]);
            }

            // Right side - File stability and churn metrics
            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

            // File stability
            if !file_stability.is_empty() {
                let stability_items: Vec<ListItem> = file_stability.iter()
                    .take(self.config.max_list_items)
                    .map(|(file, stability)| {
                        let stability_icon = match *stability {
                            s if s >= 0.8 => "🟢",
                            s if s >= 0.6 => "🟡",
                            s if s >= 0.4 => "🟠",
                            _ => "🔴",
                        };
                        let content = format!("{} {}: {:.1}%", stability_icon, file, stability * 100.0);
                        let style = match *stability {
                            s if s >= 0.8 => Style::default().fg(Color::Green),
                            s if s >= 0.6 => Style::default().fg(Color::Yellow),
                            s if s >= 0.4 => Style::default().fg(Color::LightRed),
                            _ => Style::default().fg(Color::Red),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let stability_widget = List::new(stability_items)
                    .block(Block::default().title("File Stability").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(stability_widget, right_layout[0]);
            } else {
                let no_stability = Paragraph::new("No file stability data")
                    .block(Block::default().title("File Stability").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                frame.render_widget(no_stability, right_layout[0]);
            }

            // Churn metrics (change frequency)
            if !churn_metrics.is_empty() {
                let churn_items: Vec<ListItem> = churn_metrics.iter()
                    .take(self.config.max_list_items)
                    .map(|(file, frequency)| {
                        let frequency_icon = match *frequency {
                            f if f >= 5.0 => "🔥", // Very high churn
                            f if f >= 2.0 => "⚡", // High churn
                            f if f >= 1.0 => "📈", // Medium churn
                            _ => "📊", // Low churn
                        };
                        let content = format!("{} {}: {:.1} commits/month", frequency_icon, file, frequency);
                        let style = match *frequency {
                            f if f >= 5.0 => Style::default().fg(Color::Red), // Very concerning
                            f if f >= 2.0 => Style::default().fg(Color::Yellow), // Worth monitoring
                            f if f >= 1.0 => Style::default().fg(Color::White), // Normal
                            _ => Style::default().fg(Color::Green), // Stable
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let churn_widget = List::new(churn_items)
                    .block(Block::default().title("Change Frequency (Churn)").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(churn_widget, right_layout[1]);
            } else {
                let no_churn = Paragraph::new("No churn metrics available")
                    .block(Block::default().title("Change Frequency (Churn)").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                frame.render_widget(no_churn, right_layout[1]);
            }
        }
    }

    fn render_issues_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Issues {
            issues_by_type,
            recent_issues,
            resolution_trends
        } = &view.content {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left side - Issues by type and summary
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[0]);

            // Issues by type/tool
            if !issues_by_type.is_empty() {
                let type_items: Vec<ListItem> = issues_by_type.iter()
                    .map(|(tool, count)| {
                        let tool_display = match tool.as_str() {
                            "pylint" => "🐍 PyLint",
                            "ruff" => "⚡ Ruff",
                            "bandit" => "🔒 Bandit",
                            "mypy" => "🔍 MyPy",
                            _ => tool,
                        };
                        let content = format!("{}: {} issues", tool_display, count);
                        let style = match *count {
                            c if c >= 10 => Style::default().fg(Color::Red),
                            c if c >= 5 => Style::default().fg(Color::Yellow),
                            c if c > 0 => Style::default().fg(Color::White),
                            _ => Style::default().fg(Color::Green),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let issues_by_type_widget = List::new(type_items)
                    .block(Block::default().title("Issues by Tool").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(issues_by_type_widget, left_layout[0]);
            } else {
                let no_issues_by_type = Paragraph::new("No issues detected by tools ✓")
                    .block(Block::default().title("Issues by Tool").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                frame.render_widget(no_issues_by_type, left_layout[0]);
            }

            // Resolution trends
            if !resolution_trends.is_empty() {
                let trend_items: Vec<ListItem> = resolution_trends.iter()
                    .take(self.config.max_list_items)
                    .map(|(issue_type, count)| {
                        let content = format!("{}: {} pending", issue_type, count);
                        let style = match *count {
                            c if c >= 10 => Style::default().fg(Color::Red),
                            c if c >= 5 => Style::default().fg(Color::Yellow),
                            _ => Style::default().fg(Color::White),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let trends_widget = List::new(trend_items)
                    .block(Block::default().title("Resolution Status").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(trends_widget, left_layout[1]);
            } else {
                let no_trends = Paragraph::new("No pending issues")
                    .block(Block::default().title("Resolution Status").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                frame.render_widget(no_trends, left_layout[1]);
            }

            // Right side - Recent issues
            if !recent_issues.is_empty() {
                let issue_items: Vec<ListItem> = recent_issues.iter()
                    .take(self.config.max_list_items)
                    .map(|issue| {
                        let severity_icon = match issue.severity.as_str() {
                            "critical" => "🚨",
                            "error" | "high" => "❌",
                            "warning" | "medium" => "⚠️",
                            "info" | "low" => "ℹ️",
                            _ => "•",
                        };

                        let file_name = issue.file.split('/').last().unwrap_or(&issue.file);
                        let line_info = issue.line.map(|l| format!(":{}", l)).unwrap_or_default();

                        let content = format!("{} {} ({}{})\n   {}",
                            severity_icon,
                            issue.issue_type,
                            file_name,
                            line_info,
                            issue.message.chars().take(80).collect::<String>()
                        );

                        let style = match issue.severity.as_str() {
                            "critical" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            "error" | "high" => Style::default().fg(Color::Red),
                            "warning" | "medium" => Style::default().fg(Color::Yellow),
                            "info" | "low" => Style::default().fg(Color::Blue),
                            _ => Style::default().fg(Color::White),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let recent_issues_widget = List::new(issue_items)
                    .block(Block::default().title("Recent Issues").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(recent_issues_widget, layout[1]);
            } else {
                let no_recent_issues = Paragraph::new("No recent issues found ✓\n\nAll static analysis tools\nran successfully with\nno issues detected!")
                    .block(Block::default().title("Recent Issues").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green))
                    .alignment(Alignment::Center);
                frame.render_widget(no_recent_issues, layout[1]);
            }
        }
    }

    fn render_testing_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Testing {
            test_coverage,
            test_files,
            uncovered_files,
            test_trends
        } = &view.content {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left side - Coverage and test files
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(layout[0]);

            // Test coverage gauge
            let coverage_percent = (*test_coverage * 100.0).min(100.0) as u16;
            let coverage_gauge = Gauge::default()
                .block(Block::default().title("Test Coverage").borders(Borders::ALL))
                .gauge_style(self.get_coverage_color(*test_coverage))
                .percent(coverage_percent)
                .label(format!("{:.1}%", test_coverage * 100.0));
            frame.render_widget(coverage_gauge, left_layout[0]);

            // Test files list
            if !test_files.is_empty() {
                let test_file_items: Vec<ListItem> = test_files.iter()
                    .take(self.config.max_list_items)
                    .map(|file| {
                        let content = format!("🧪 {}", file);
                        ListItem::new(content).style(Style::default().fg(Color::Green))
                    })
                    .collect();

                let test_files_widget = List::new(test_file_items)
                    .block(Block::default().title("Test Files").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(test_files_widget, left_layout[1]);
            } else {
                let no_test_files = Paragraph::new("No test files detected ⚠️\n\nConsiderations:\n• Add test files to improve\n  code reliability\n• Use naming conventions like\n  test_*.py or *_test.py\n• Include unit and integration\n  tests")
                    .block(Block::default().title("Test Files").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(no_test_files, left_layout[1]);
            }

            // Right side - Uncovered files and test trends
            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

            // Uncovered files
            if !uncovered_files.is_empty() {
                let uncovered_items: Vec<ListItem> = uncovered_files.iter()
                    .take(self.config.max_list_items)
                    .map(|file| {
                        let content = format!("❌ {}", file);
                        ListItem::new(content).style(Style::default().fg(Color::Red))
                    })
                    .collect();

                let uncovered_widget = List::new(uncovered_items)
                    .block(Block::default().title("Files Without Tests").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(uncovered_widget, right_layout[0]);
            } else {
                let all_covered = Paragraph::new("All files have tests ✅\n\nExcellent test coverage!\nAll source files appear to\nhave corresponding tests.")
                    .block(Block::default().title("Files Without Tests").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                frame.render_widget(all_covered, right_layout[0]);
            }

            // Test trends (coverage by file or test counts)
            if !test_trends.is_empty() {
                let trend_items: Vec<ListItem> = test_trends.iter()
                    .take(self.config.max_list_items)
                    .map(|(file, score)| {
                        let percentage = score * 100.0;
                        let score_icon = match *score {
                            s if s >= 0.9 => "🟢",
                            s if s >= 0.7 => "🟡",
                            s if s >= 0.5 => "🟠",
                            _ => "🔴",
                        };
                        let content = format!("{} {}: {:.1}%", score_icon, file, percentage);
                        let style = match *score {
                            s if s >= 0.9 => Style::default().fg(Color::Green),
                            s if s >= 0.7 => Style::default().fg(Color::Yellow),
                            s if s >= 0.5 => Style::default().fg(Color::LightRed),
                            _ => Style::default().fg(Color::Red),
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let trends_widget = List::new(trend_items)
                    .block(Block::default().title("Coverage by File").borders(Borders::ALL))
                    .style(self.get_content_style());
                frame.render_widget(trends_widget, right_layout[1]);
            } else {
                let no_trends = Paragraph::new("No coverage data available\n\nTo get detailed coverage:\n• Install coverage tools\n• Run tests with coverage\n• Generate coverage reports")
                    .block(Block::default().title("Coverage by File").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                frame.render_widget(no_trends, right_layout[1]);
            }
        }
    }

    fn render_flow_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::Flow { data_flows, control_flows, flow_complexity, bottlenecks } = &view.content {
            // Create 4-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            // Top Left: Flow Complexity Gauge
            self.render_flow_complexity_gauge(frame, top_chunks[0], *flow_complexity);

            // Top Right: Data Flows
            self.render_data_flows_list(frame, top_chunks[1], data_flows);

            // Bottom Left: Control Flows
            self.render_control_flows_list(frame, bottom_chunks[0], control_flows);

            // Bottom Right: Bottlenecks
            self.render_flow_bottlenecks(frame, bottom_chunks[1], bottlenecks);
        }
    }

    /// Render tree-sitter analysis content
    fn render_tree_sitter_content(&self, frame: &mut Frame, area: Rect, view: &RenderableView) {
        if let ViewContent::TreeSitterAnalysis { imports_summary, symbols_by_type, highlights_summary, semantic_complexity, language_features } = &view.content {
            // Create 4-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            // Top Left: Import Summary
            self.render_import_summary(frame, top_chunks[0], imports_summary);

            // Top Right: Highlight Summary
            self.render_highlight_summary(frame, top_chunks[1], highlights_summary);

            // Bottom Left: Language Features
            self.render_language_features(frame, bottom_chunks[0], language_features);

            // Bottom Right: Symbols
            self.render_symbols_list(frame, bottom_chunks[1], symbols_by_type);
        }
    }

    /// Render import summary
    fn render_import_summary(&self, frame: &mut Frame, area: Rect, import_summary: &TreeSitterImportSummary) {
        let summary_text = format!(
            "🔍 Analysis Method: {}\n\n📥 Total Imports: {}\n📦 Unique Modules: {}\n🌐 External Dependencies: {}\n\nTop Dependencies:\n{}",
            import_summary.analysis_method,
            import_summary.total_imports,
            import_summary.unique_modules,
            import_summary.external_dependencies.len(),
            import_summary.external_dependencies.iter()
                .take(5)
                .map(|dep| format!("  • {}", dep))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let summary_widget = Paragraph::new(summary_text)
            .block(Block::default().title("Import Analysis").borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true });

        frame.render_widget(summary_widget, area);
    }

    /// Render highlight summary
    fn render_highlight_summary(&self, frame: &mut Frame, area: Rect, highlight_summary: &TreeSitterHighlightSummary) {
        let summary_text = format!(
            "📊 Total Highlights: {}\n\n🔧 Functions: {}\n📝 Types: {}\n🔄 Variables: {}\n🏷️ Semantic Types: {}\n\nSymbol Breakdown:\n{}",
            highlight_summary.total_highlights,
            highlight_summary.functions_found,
            highlight_summary.types_found,
            highlight_summary.variables_found,
            highlight_summary.semantic_types.len(),
            if highlight_summary.total_highlights > 0 {
                format!(
                    "  Functions: {:.1}%\n  Types: {:.1}%\n  Variables: {:.1}%",
                    (highlight_summary.functions_found as f64 / highlight_summary.total_highlights as f64) * 100.0,
                    (highlight_summary.types_found as f64 / highlight_summary.total_highlights as f64) * 100.0,
                    (highlight_summary.variables_found as f64 / highlight_summary.total_highlights as f64) * 100.0,
                )
            } else {
                "  No highlights detected".to_string()
            }
        );

        let color = if highlight_summary.total_highlights > 0 {
            Color::Green
        } else {
            Color::Yellow
        };

        let summary_widget = Paragraph::new(summary_text)
            .block(Block::default().title("Symbol Highlights").borders(Borders::ALL))
            .style(Style::default().fg(color))
            .wrap(Wrap { trim: true });

        frame.render_widget(summary_widget, area);
    }

    /// Render language features
    fn render_language_features(&self, frame: &mut Frame, area: Rect, language_features: &[LanguageFeature]) {
        let features_text = if language_features.is_empty() {
            "No language features detected".to_string()
        } else {
            let mut text = String::new();
            for (i, feature) in language_features.iter().enumerate().take(8) {
                if i > 0 {
                    text.push('\n');
                }
                let complexity_icon = "📝"; // Simple icon for now
                text.push_str(&format!(
                    "{} {} ({}x)",
                    complexity_icon,
                    self.truncate_name(&feature.feature_type, 15),
                    feature.count
                ));
            }
            if language_features.len() > 8 {
                text.push_str(&format!("\n... and {} more features", language_features.len() - 8));
            }
            text
        };

        let style = if language_features.is_empty() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Magenta)
        };

        let features_widget = Paragraph::new(features_text)
            .block(Block::default().title("Language Features").borders(Borders::ALL))
            .style(style)
            .wrap(Wrap { trim: true });

        frame.render_widget(features_widget, area);
    }

    /// Render symbols list
    fn render_symbols_list(&self, frame: &mut Frame, area: Rect, symbols_by_type: &HashMap<String, Vec<TreeSitterSymbol>>) {
        let symbols_text = if symbols_by_type.is_empty() {
            "No symbols detected".to_string()
        } else {
            let mut text = String::new();
            let mut count = 0;
            let mut total_symbols = 0;

            for (symbol_type, symbols) in symbols_by_type {
                total_symbols += symbols.len();
                if count >= 8 { continue; }

                text.push_str(&format!("{}:\n", symbol_type));
                for symbol in symbols.iter().take(3) {
                    if count >= 8 { break; }
                    let type_icon = match symbol.symbol_type.as_str() {
                        "function" | "method" => "🔧",
                        "struct" | "class" => "📦",
                        "enum" | "type" => "📋",
                        "variable" | "field" => "🔄",
                        "module" | "namespace" => "📁",
                        _ => "📝",
                    };
                    text.push_str(&format!(
                        "  {} {} ({}:{})\n",
                        type_icon,
                        self.truncate_name(&symbol.name, 20),
                        self.extract_filename(&symbol.file),
                        symbol.line
                    ));
                    count += 1;
                }
                if symbols.len() > 3 {
                    text.push_str(&format!("  ... and {} more\n", symbols.len() - 3));
                }
                text.push('\n');
            }
            if total_symbols > count {
                text.push_str(&format!("Total: {} symbols", total_symbols));
            }
            text
        };

        let style = if symbols_by_type.is_empty() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Blue)
        };

        let symbols_widget = Paragraph::new(symbols_text)
            .block(Block::default().title("Symbol Analysis").borders(Borders::ALL))
            .style(style)
            .wrap(Wrap { trim: true });

        frame.render_widget(symbols_widget, area);
    }

    /// Render flow complexity gauge
    fn render_flow_complexity_gauge(&self, frame: &mut Frame, area: Rect, complexity: f64) {
        // Create gauge visualization
        let percentage = (complexity * 100.0) as u16;
        let complexity_color = match complexity {
            c if c >= 0.8 => Color::Red,
            c if c >= 0.6 => Color::LightRed,
            c if c >= 0.4 => Color::Yellow,
            c if c >= 0.2 => Color::LightBlue,
            _ => Color::Green,
        };

        let complexity_icon = match complexity {
            c if c >= 0.8 => "🔴",
            c if c >= 0.6 => "🟠",
            c if c >= 0.4 => "🟡",
            c if c >= 0.2 => "🔵",
            _ => "🟢",
        };

        let gauge_text = format!(
            "{} Flow Complexity\n\n{:.1}%\n\n{}",
            complexity_icon,
            complexity * 100.0,
            match complexity {
                c if c >= 0.8 => "Very Complex",
                c if c >= 0.6 => "Complex",
                c if c >= 0.4 => "Moderate",
                c if c >= 0.2 => "Simple",
                _ => "Very Simple",
            }
        );

        let gauge = Gauge::default()
            .block(Block::default().title("Flow Complexity").borders(Borders::ALL))
            .gauge_style(Style::default().fg(complexity_color))
            .percent(percentage);

        frame.render_widget(gauge, area);

        // Render text on top
        let text_area = Rect {
            x: area.x + 2,
            y: area.y + area.height / 2,
            width: area.width.saturating_sub(4),
            height: 3,
        };

        let text_widget = Paragraph::new(gauge_text)
            .style(Style::default().fg(complexity_color))
            .alignment(Alignment::Center);

        frame.render_widget(text_widget, text_area);
    }

    /// Render data flows list
    fn render_data_flows_list(&self, frame: &mut Frame, area: Rect, data_flows: &[FlowItem]) {
        let flows_text = if data_flows.is_empty() {
            "No data flows detected".to_string()
        } else {
            let mut text = String::new();
            for (i, flow) in data_flows.iter().enumerate().take(10) {
                let flow_icon = match flow.flow_type.as_str() {
                    "assignment" => "📝",
                    "parameter" => "📥",
                    "return_value" => "📤",
                    _ => "🔗",
                };
                text.push_str(&format!(
                    "{} {} → {}\n  Type: {}\n",
                    flow_icon,
                    self.truncate_name(&flow.from, 15),
                    self.truncate_name(&flow.to, 15),
                    flow.flow_type
                ));
                if i < data_flows.len() - 1 {
                    text.push('\n');
                }
            }
            if data_flows.len() > 10 {
                text.push_str(&format!("\n... and {} more", data_flows.len() - 10));
            }
            text
        };

        let data_flows_widget = Paragraph::new(flows_text)
            .block(Block::default().title("Data Flows").borders(Borders::ALL))
            .style(self.get_content_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(data_flows_widget, area);
    }

    /// Render control flows list
    fn render_control_flows_list(&self, frame: &mut Frame, area: Rect, control_flows: &[FlowItem]) {
        let flows_text = if control_flows.is_empty() {
            "No control flows detected".to_string()
        } else {
            let mut text = String::new();
            for (i, flow) in control_flows.iter().enumerate().take(10) {
                let flow_icon = match flow.flow_type.as_str() {
                    "function_call" => "📞",
                    "conditional" => "🔀",
                    "loop" => "🔄",
                    _ => "➡️",
                };
                text.push_str(&format!(
                    "{} {} → {}\n  Type: {}\n",
                    flow_icon,
                    self.truncate_name(&flow.from, 15),
                    self.truncate_name(&flow.to, 15),
                    flow.flow_type
                ));
                if i < control_flows.len() - 1 {
                    text.push('\n');
                }
            }
            if control_flows.len() > 10 {
                text.push_str(&format!("\n... and {} more", control_flows.len() - 10));
            }
            text
        };

        let control_flows_widget = Paragraph::new(flows_text)
            .block(Block::default().title("Control Flows").borders(Borders::ALL))
            .style(self.get_content_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(control_flows_widget, area);
    }

    /// Render flow bottlenecks
    fn render_flow_bottlenecks(&self, frame: &mut Frame, area: Rect, bottlenecks: &[String]) {
        let bottlenecks_text = if bottlenecks.is_empty() {
            "✅ No flow bottlenecks detected".to_string()
        } else {
            let mut text = String::new();
            for (i, bottleneck) in bottlenecks.iter().enumerate().take(8) {
                text.push_str(&format!("⚠️  {}\n", bottleneck));
                if i < bottlenecks.len() - 1 {
                    text.push('\n');
                }
            }
            if bottlenecks.len() > 8 {
                text.push_str(&format!("\n... and {} more issues", bottlenecks.len() - 8));
            }
            text
        };

        let style = if bottlenecks.is_empty() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let bottlenecks_widget = Paragraph::new(bottlenecks_text)
            .block(Block::default().title("Flow Bottlenecks").borders(Borders::ALL))
            .style(style)
            .wrap(Wrap { trim: true });

        frame.render_widget(bottlenecks_widget, area);
    }

    /// Truncate a name to specified length with ellipsis
    fn truncate_name(&self, name: &str, max_len: usize) -> String {
        if name.len() <= max_len {
            name.to_string()
        } else {
            format!("{}...", &name[..max_len.saturating_sub(3)])
        }
    }

    /// Get color for health score visualization
    fn get_health_color(&self, score: f64) -> Style {
        let color = match score {
            s if s >= 0.8 => Color::Green,
            s if s >= 0.6 => Color::Yellow,
            s if s >= 0.4 => Color::LightRed,
            _ => Color::Red,
        };

        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(color),
            ColorScheme::Light => Style::default().fg(color),
            ColorScheme::HighContrast => Style::default().fg(color).add_modifier(Modifier::BOLD),
        }
    }

    /// Get color for risk level
    fn get_risk_color(&self, risk_level: &str) -> Style {
        let color = match risk_level.to_lowercase().as_str() {
            "low" => Color::Green,
            "medium" => Color::Yellow,
            "high" => Color::LightRed,
            "critical" => Color::Red,
            _ => Color::Gray,
        };

        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(color),
            ColorScheme::Light => Style::default().fg(color),
            ColorScheme::HighContrast => Style::default().fg(color).add_modifier(Modifier::BOLD),
        }
    }

    /// Get color for performance score visualization
    fn get_performance_color(&self, score: f64) -> Style {
        let color = match score {
            s if s >= 0.8 => Color::Green,
            s if s >= 0.6 => Color::Yellow,
            s if s >= 0.4 => Color::LightRed,
            _ => Color::Red,
        };

        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(color),
            ColorScheme::Light => Style::default().fg(color),
            ColorScheme::HighContrast => Style::default().fg(color).add_modifier(Modifier::BOLD),
        }
    }

    /// Get color for test coverage visualization
    fn get_coverage_color(&self, coverage: f64) -> Style {
        let color = match coverage {
            c if c >= 0.9 => Color::Green,
            c if c >= 0.7 => Color::Yellow,
            c if c >= 0.5 => Color::LightRed,
            _ => Color::Red,
        };

        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(color),
            ColorScheme::Light => Style::default().fg(color),
            ColorScheme::HighContrast => Style::default().fg(color).add_modifier(Modifier::BOLD),
        }
    }

    /// Get header style
    fn get_header_style(&self) -> Style {
        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ColorScheme::Light => Style::default().fg(Color::Black).add_modifier(Modifier::BOLD),
            ColorScheme::HighContrast => Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD),
        }
    }

    /// Get footer style
    fn get_footer_style(&self) -> Style {
        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(Color::Gray),
            ColorScheme::Light => Style::default().fg(Color::DarkGray),
            ColorScheme::HighContrast => Style::default().fg(Color::White).add_modifier(Modifier::DIM),
        }
    }

    /// Get content style
    fn get_content_style(&self) -> Style {
        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(Color::White),
            ColorScheme::Light => Style::default().fg(Color::Black),
            ColorScheme::HighContrast => Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        }
    }

    /// Get chart style
    fn get_chart_style(&self) -> Style {
        match self.config.color_scheme {
            ColorScheme::Dark => Style::default().fg(Color::Cyan),
            ColorScheme::Light => Style::default().fg(Color::Blue),
            ColorScheme::HighContrast => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        }
    }
}

impl Default for VisualizationEngine {
    fn default() -> Self {
        Self::new()
    }
}