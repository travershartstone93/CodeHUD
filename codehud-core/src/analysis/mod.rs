//! CodeHUD Analysis Pipeline
//!
//! Main analysis orchestration module that coordinates all extractors
//! and generates comprehensive codebase analysis results.

use crate::extractors::{
    BaseDataExtractor,
    topology::TopologyExtractor,
    quality::QualityExtractor,
    security::SecurityExtractor,
    dependencies::DependenciesExtractor,
    performance::PerformanceExtractor,
    evolution::EvolutionExtractor,
    issues::IssuesExtractor,
    orphaned_files::OrphanedFilesExtractor,
    flow::FlowExtractor,
    testing::TestingExtractor,
};
use crate::models::analysis_result::{AnalysisResult, CodeMetrics};
use crate::query_engine::QueryEngine;
use crate::{Result, ViewType, Pipeline};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde_json::{json, Value};
use chrono::Utc;

pub mod pipeline;
pub mod health_calculator;
pub mod view_generator;

pub use pipeline::AnalysisPipeline;
pub use health_calculator::HealthCalculator;
pub use view_generator::ViewGenerator;

/// Main analysis orchestrator that coordinates all extractors
pub struct AnalysisOrchestrator {
    codebase_path: PathBuf,
    pipeline: Pipeline,
    debug: bool,
}

impl AnalysisOrchestrator {
    pub fn new(codebase_path: impl AsRef<Path>, pipeline: Pipeline) -> Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();

        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!(
                "Codebase path does not exist: {}",
                codebase_path.display()
            )));
        }

        Ok(Self {
            codebase_path,
            pipeline,
            debug: false,
        })
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Run comprehensive analysis using all extractors
    pub async fn analyze(&self) -> Result<AnalysisResult> {
        let start_time = std::time::Instant::now();
        let mut result = AnalysisResult::new(self.codebase_path.to_string_lossy().to_string());

        if self.debug {
            println!("Starting analysis of {}", self.codebase_path.display());
            println!("Using pipeline: {}", self.pipeline);
        }

        // Run all extractors in parallel for better performance
        let (
            topology_data,
            quality_data,
            security_data,
            dependencies_data,
            performance_data,
            evolution_data,
            issues_data,
            testing_data,
            flow_data,
            orphaned_files_data
        ) = tokio::try_join!(
            self.run_topology_analysis(),
            self.run_quality_analysis(),
            self.run_security_analysis(),
            self.run_dependencies_analysis(),
            self.run_performance_analysis(),
            self.run_evolution_analysis(),
            self.run_issues_analysis(),
            self.run_testing_analysis(),
            self.run_flow_analysis(),
            self.run_orphaned_files_analysis()
        )?;

        // Store extracted data
        result.set_view_data("topology".to_string(), topology_data.clone());
        result.set_view_data("quality".to_string(), quality_data.clone());
        result.set_view_data("security".to_string(), security_data.clone());
        result.set_view_data("dependencies".to_string(), dependencies_data.clone());
        result.set_view_data("performance".to_string(), performance_data.clone());
        result.set_view_data("evolution".to_string(), evolution_data.clone());
        result.set_view_data("issues".to_string(), issues_data.clone());
        result.set_view_data("testing".to_string(), testing_data.clone());
        result.set_view_data("flow".to_string(), flow_data.clone());
        result.set_view_data("orphaned_files".to_string(), orphaned_files_data.clone());

        // Calculate aggregate metrics
        result.metrics = self.calculate_code_metrics(&topology_data, &quality_data)?;
        result.files_analyzed = self.count_analyzed_files(&topology_data);

        // Calculate health score
        let health_calculator = HealthCalculator::new();
        result.health_score = health_calculator.calculate_health_score(
            &result.metrics,
            &quality_data,
            &security_data,
            &dependencies_data
        )?;

        // Extract critical issues
        result.critical_issues = self.extract_critical_issues(&quality_data, &security_data)?;

        // Generate focus recommendations
        result.focus_recommendations = self.generate_focus_recommendations(
            &result.metrics,
            &quality_data,
            &security_data,
            &dependencies_data
        )?;

        // Record analysis completion
        let duration = start_time.elapsed();
        result.analysis_duration = duration.as_secs_f64();
        result.analysis_timestamp = Utc::now();

        if self.debug {
            println!("Analysis completed in {:.2}s", result.analysis_duration);
            println!("Health score: {:.1}", result.health_score);
            println!("Files analyzed: {}", result.files_analyzed);
            println!("Critical issues: {}", result.critical_issues.len());
        }

        Ok(result)
    }

    /// Generate specific view data
    pub async fn generate_view(&self, view_type: ViewType) -> Result<Value> {
        match view_type {
            ViewType::Topology => self.run_topology_analysis().await,
            ViewType::Quality => self.run_quality_analysis().await,
            ViewType::Security => self.run_security_analysis().await,
            ViewType::Dependencies => self.run_dependencies_analysis().await,
            ViewType::Performance => self.run_performance_analysis().await,
            ViewType::Evolution => self.run_evolution_analysis().await,
            ViewType::IssuesInspection => self.run_issues_analysis().await,
            ViewType::Testing => self.run_testing_analysis().await,
            ViewType::Flow => self.run_flow_analysis().await,
            ViewType::FixRollbackDevnotes => self.run_devnotes_analysis().await,
            ViewType::TreeSitterAnalysis => self.run_tree_sitter_analysis().await,
        }
    }

    async fn run_topology_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running topology analysis..."); }
        let extractor = TopologyExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_quality_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running quality analysis..."); }
        let extractor = QualityExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_security_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running security analysis..."); }
        let extractor = SecurityExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_dependencies_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running dependencies analysis..."); }
        let extractor = DependenciesExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_performance_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running performance analysis..."); }
        let extractor = PerformanceExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_evolution_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running evolution analysis..."); }
        let extractor = EvolutionExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_issues_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running issues analysis..."); }
        let extractor = IssuesExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_orphaned_files_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running orphaned files analysis..."); }
        let extractor = OrphanedFilesExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_flow_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running flow analysis..."); }
        let extractor = FlowExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_testing_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running testing analysis..."); }
        let extractor = TestingExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_devnotes_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running devnotes analysis..."); }
        let extractor = TestingExtractor::new(&self.codebase_path)?;
        let data = extractor.extract_data()?;
        Ok(json!(data))
    }

    async fn run_tree_sitter_analysis(&self) -> Result<Value> {
        if self.debug { println!("Running tree-sitter analysis..."); }

        // Use the query engine to perform enhanced tree-sitter analysis
        let mut query_engine = QueryEngine::new()
            .map_err(|e| crate::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        // Analyze the entire codebase using the enhanced tree-sitter system
        let result = query_engine.analyze_codebase(&self.codebase_path)
            .map_err(|e| crate::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(result)
    }

    fn calculate_code_metrics(&self, topology_data: &Value, quality_data: &Value) -> Result<CodeMetrics> {
        let mut metrics = CodeMetrics::default();

        // Extract metrics from topology data
        if let Some(summary) = topology_data.get("summary") {
            metrics.file_count = summary.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            metrics.function_count = summary.get("total_functions").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            metrics.class_count = summary.get("total_classes").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            metrics.total_lines = summary.get("total_lines").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        }

        // Extract metrics from quality data
        if let Some(summary) = quality_data.get("summary") {
            metrics.avg_complexity = summary.get("average_complexity").and_then(|v| v.as_f64()).unwrap_or(0.0);
            metrics.max_complexity = summary.get("max_complexity").and_then(|v| v.as_f64()).unwrap_or(0.0);

            // Calculate technical debt ratio based on health score
            let health_score = summary.get("health_score").and_then(|v| v.as_f64()).unwrap_or(100.0);
            metrics.technical_debt_ratio = (100.0 - health_score) / 100.0;
        }

        // Calculate executable lines (estimate: 70% of total lines)
        metrics.executable_lines = (metrics.total_lines as f64 * 0.7) as usize;

        Ok(metrics)
    }

    fn count_analyzed_files(&self, topology_data: &Value) -> usize {
        topology_data
            .get("summary")
            .and_then(|s| s.get("total_files"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize
    }

    fn extract_critical_issues(&self, quality_data: &Value, security_data: &Value) -> Result<Vec<HashMap<String, Value>>> {
        let mut critical_issues = Vec::new();

        // Extract critical quality issues
        if let Some(issues) = quality_data.get("issues").and_then(|v| v.as_array()) {
            for issue in issues {
                if let Some(severity) = issue.get("severity").and_then(|v| v.as_str()) {
                    if severity == "high" || severity == "critical" {
                        let mut issue_map = HashMap::new();
                        issue_map.insert("source".to_string(), json!("quality"));
                        issue_map.insert("severity".to_string(), json!(severity));
                        issue_map.insert("message".to_string(), issue.get("message").cloned().unwrap_or(json!("Quality issue")));
                        issue_map.insert("file".to_string(), issue.get("file").cloned().unwrap_or(json!("unknown")));
                        critical_issues.push(issue_map);
                    }
                }
            }
        }

        // Extract critical security issues
        if let Some(vulnerabilities) = security_data.get("vulnerabilities").and_then(|v| v.as_array()) {
            for vuln in vulnerabilities {
                if let Some(severity) = vuln.get("severity").and_then(|v| v.as_str()) {
                    if severity == "high" || severity == "critical" {
                        let mut issue_map = HashMap::new();
                        issue_map.insert("source".to_string(), json!("security"));
                        issue_map.insert("severity".to_string(), json!(severity));
                        issue_map.insert("message".to_string(), vuln.get("description").cloned().unwrap_or(json!("Security vulnerability")));
                        issue_map.insert("file".to_string(), vuln.get("file").cloned().unwrap_or(json!("unknown")));
                        critical_issues.push(issue_map);
                    }
                }
            }
        }

        Ok(critical_issues)
    }

    fn generate_focus_recommendations(&self,
                                    metrics: &CodeMetrics,
                                    quality_data: &Value,
                                    security_data: &Value,
                                    dependencies_data: &Value) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();

        // Quality-based recommendations
        if metrics.avg_complexity > 10.0 {
            recommendations.push("🔍 Focus on reducing code complexity - average complexity is high".to_string());
        }

        if metrics.technical_debt_ratio > 0.3 {
            recommendations.push("🛠️ Address technical debt - ratio exceeds recommended threshold".to_string());
        }

        // Security-based recommendations
        if let Some(summary) = security_data.get("summary") {
            if let Some(high_severity) = summary.get("high_severity_findings").and_then(|v| v.as_u64()) {
                if high_severity > 0 {
                    recommendations.push(format!("🔒 Address {} high-severity security findings", high_severity));
                }
            }
        }

        // Dependencies-based recommendations
        if let Some(summary) = dependencies_data.get("summary") {
            if let Some(circular_deps) = summary.get("circular_dependencies_found").and_then(|v| v.as_u64()) {
                if circular_deps > 0 {
                    recommendations.push(format!("🔄 Resolve {} circular dependencies", circular_deps));
                }
            }
        }

        // Performance-based recommendations
        if metrics.file_count > 1000 {
            recommendations.push("📊 Consider modularization - large codebase detected".to_string());
        }

        // Default recommendations if none specific
        if recommendations.is_empty() {
            recommendations.push("✨ Codebase is in good shape - continue following best practices".to_string());
        }

        Ok(recommendations)
    }
}