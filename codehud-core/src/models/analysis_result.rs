//! Analysis Result Model
//!
//! Complete analysis result structure matching Python implementation exactly

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Complete analysis result for a codebase - matches Python AnalysisResult exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Path to the analyzed codebase
    pub codebase_path: String,
    /// Number of files that were analyzed
    pub files_analyzed: usize,
    /// Timestamp when analysis was performed
    pub analysis_timestamp: DateTime<Utc>,
    /// Duration of analysis in seconds
    pub analysis_duration: f64,

    // Core analysis data
    /// Overall code metrics
    pub metrics: CodeMetrics,
    /// Overall health score (0.0 to 100.0)
    pub health_score: f64,
    /// Critical issues that need immediate attention
    pub critical_issues: Vec<HashMap<String, serde_json::Value>>,
    /// Recommendations for focus areas
    pub focus_recommendations: Vec<String>,

    // Direct extraction data (new pipeline)
    /// Extracted view data organized by view type
    pub extracted_view_data: HashMap<String, serde_json::Value>,

    // Optional detailed data (for legacy pipeline Phase 2+)
    /// Parsed file data (optional for legacy pipeline)
    pub parsed_files: Option<Vec<serde_json::Value>>,
    /// Graph data (optional for legacy pipeline)
    pub graphs: Option<serde_json::Value>,
    /// Static analysis results (optional for legacy pipeline)
    pub static_analysis: Option<serde_json::Value>,
}

/// Code metrics structure matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Total lines of code
    pub total_lines: usize,
    /// Lines of executable code (excluding comments/whitespace)
    pub executable_lines: usize,
    /// Number of functions
    pub function_count: usize,
    /// Number of classes
    pub class_count: usize,
    /// Number of files
    pub file_count: usize,
    /// Average cyclomatic complexity
    pub avg_complexity: f64,
    /// Maximum cyclomatic complexity found
    pub max_complexity: f64,
    /// Technical debt ratio
    pub technical_debt_ratio: f64,
    /// Code coverage percentage (if available)
    pub coverage_percentage: Option<f64>,
    /// Duplication percentage
    pub duplication_percentage: f64,
}

impl Default for CodeMetrics {
    fn default() -> Self {
        Self {
            total_lines: 0,
            executable_lines: 0,
            function_count: 0,
            class_count: 0,
            file_count: 0,
            avg_complexity: 0.0,
            max_complexity: 0.0,
            technical_debt_ratio: 0.0,
            coverage_percentage: None,
            duplication_percentage: 0.0,
        }
    }
}

impl AnalysisResult {
    /// Create a new analysis result
    pub fn new(codebase_path: String) -> Self {
        Self {
            codebase_path,
            files_analyzed: 0,
            analysis_timestamp: Utc::now(),
            analysis_duration: 0.0,
            metrics: CodeMetrics::default(),
            health_score: 0.0,
            critical_issues: Vec::new(),
            focus_recommendations: Vec::new(),
            extracted_view_data: HashMap::new(),
            parsed_files: None,
            graphs: None,
            static_analysis: None,
        }
    }

    /// Add a critical issue to the analysis result
    pub fn add_critical_issue(&mut self, issue: HashMap<String, serde_json::Value>) {
        self.critical_issues.push(issue);
    }

    /// Add a focus recommendation
    pub fn add_focus_recommendation(&mut self, recommendation: String) {
        self.focus_recommendations.push(recommendation);
    }

    /// Set extracted view data for a specific view type
    pub fn set_view_data(&mut self, view_type: String, data: serde_json::Value) {
        self.extracted_view_data.insert(view_type, data);
    }

    /// Get view data for a specific view type
    pub fn get_view_data(&self, view_type: &str) -> Option<&serde_json::Value> {
        self.extracted_view_data.get(view_type)
    }

    /// Check if analysis has critical issues
    pub fn has_critical_issues(&self) -> bool {
        !self.critical_issues.is_empty()
    }

    /// Get severity distribution of issues
    pub fn get_issue_severity_distribution(&self) -> HashMap<String, usize> {
        let mut distribution = HashMap::new();
        
        for issue in &self.critical_issues {
            if let Some(severity) = issue.get("severity") {
                if let Some(severity_str) = severity.as_str() {
                    *distribution.entry(severity_str.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        distribution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_result_creation() {
        let result = AnalysisResult::new("/test/path".to_string());
        assert_eq!(result.codebase_path, "/test/path");
        assert_eq!(result.files_analyzed, 0);
        assert!(!result.has_critical_issues());
    }

    #[test]
    fn test_critical_issues() {
        let mut result = AnalysisResult::new("/test/path".to_string());
        
        let mut issue = HashMap::new();
        issue.insert("severity".to_string(), serde_json::Value::String("high".to_string()));
        issue.insert("message".to_string(), serde_json::Value::String("Test issue".to_string()));
        
        result.add_critical_issue(issue);
        assert!(result.has_critical_issues());
        assert_eq!(result.critical_issues.len(), 1);
    }

    #[test]
    fn test_view_data() {
        let mut result = AnalysisResult::new("/test/path".to_string());
        let test_data = serde_json::json!({"test": "data"});
        
        result.set_view_data("test_view".to_string(), test_data.clone());
        assert_eq!(result.get_view_data("test_view"), Some(&test_data));
        assert_eq!(result.get_view_data("nonexistent"), None);
    }

    #[test]
    fn test_code_metrics_default() {
        let metrics = CodeMetrics::default();
        assert_eq!(metrics.total_lines, 0);
        assert_eq!(metrics.function_count, 0);
        assert_eq!(metrics.avg_complexity, 0.0);
    }
}