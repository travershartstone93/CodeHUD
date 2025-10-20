//! Health Score Calculator
//!
//! Calculates overall codebase health score based on multiple metrics

use crate::models::analysis_result::CodeMetrics;
use crate::Result;
use serde_json::Value;

pub struct HealthCalculator;

impl HealthCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate overall health score (0-100) based on all analysis data
    pub fn calculate_health_score(&self,
                                  metrics: &CodeMetrics,
                                  quality_data: &Value,
                                  security_data: &Value,
                                  dependencies_data: &Value) -> Result<f64> {

        let quality_score = self.calculate_quality_score(metrics, quality_data);
        let security_score = self.calculate_security_score(security_data);
        let dependencies_score = self.calculate_dependencies_score(dependencies_data);
        let complexity_score = self.calculate_complexity_score(metrics);

        // Weighted average of all scores
        let health_score = (quality_score * 0.3) +
                          (security_score * 0.25) +
                          (dependencies_score * 0.25) +
                          (complexity_score * 0.2);

        Ok(health_score.max(0.0).min(100.0))
    }

    fn calculate_quality_score(&self, _metrics: &CodeMetrics, quality_data: &Value) -> f64 {
        // Extract health score from quality data if available
        if let Some(summary) = quality_data.get("summary") {
            if let Some(health_score) = summary.get("health_score").and_then(|v| v.as_f64()) {
                return health_score;
            }
        }

        // Fallback calculation based on issues
        let mut score = 100.0;

        if let Some(issues) = quality_data.get("issues").and_then(|v| v.as_array()) {
            let high_issues = issues.iter().filter(|i| {
                i.get("severity").and_then(|s| s.as_str()) == Some("high")
            }).count();

            let medium_issues = issues.iter().filter(|i| {
                i.get("severity").and_then(|s| s.as_str()) == Some("medium")
            }).count();

            score -= (high_issues as f64 * 10.0) + (medium_issues as f64 * 5.0);
        }

        score.max(0.0)
    }

    fn calculate_security_score(&self, security_data: &Value) -> f64 {
        let mut score = 100.0;

        if let Some(summary) = security_data.get("summary") {
            let high_findings = summary.get("high_severity_findings")
                .and_then(|v| v.as_u64()).unwrap_or(0) as f64;
            let medium_findings = summary.get("medium_severity_findings")
                .and_then(|v| v.as_u64()).unwrap_or(0) as f64;
            let low_findings = summary.get("low_severity_findings")
                .and_then(|v| v.as_u64()).unwrap_or(0) as f64;

            // Penalize based on security findings
            score -= (high_findings * 15.0) + (medium_findings * 8.0) + (low_findings * 2.0);
        }

        score.max(0.0)
    }

    fn calculate_dependencies_score(&self, dependencies_data: &Value) -> f64 {
        let mut score = 100.0;

        if let Some(summary) = dependencies_data.get("summary") {
            let circular_deps = summary.get("circular_dependencies_found")
                .and_then(|v| v.as_u64()).unwrap_or(0) as f64;

            // Penalize circular dependencies heavily
            score -= circular_deps * 20.0;
        }

        if let Some(coupling) = dependencies_data.get("coupling_analysis") {
            let high_coupling = coupling.get("coupling_distribution")
                .and_then(|d| d.get("high_coupling"))
                .and_then(|v| v.as_u64()).unwrap_or(0) as f64;

            // Penalize high coupling
            score -= high_coupling * 10.0;
        }

        score.max(0.0)
    }

    fn calculate_complexity_score(&self, metrics: &CodeMetrics) -> f64 {
        let mut score = 100.0;

        // Penalize high average complexity
        if metrics.avg_complexity > 10.0 {
            score -= (metrics.avg_complexity - 10.0) * 5.0;
        }

        // Penalize very high maximum complexity
        if metrics.max_complexity > 20.0 {
            score -= (metrics.max_complexity - 20.0) * 2.0;
        }

        // Penalize high technical debt
        score -= metrics.technical_debt_ratio * 30.0;

        score.max(0.0)
    }
}

impl Default for HealthCalculator {
    fn default() -> Self {
        Self::new()
    }
}