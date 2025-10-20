//! Health Score Calculation - Exact Python Algorithm Implementation
//!
//! This module implements the health score calculation system to match
//! Python algorithms exactly as required by zero-degradation plan.

use codehud_core::{Result, Error};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Health score calculator matching Python HealthScoreCalculator
#[derive(Debug, Clone)]
pub struct HealthScoreCalculator {
    complexity_weight: f64,
    bug_weight: f64,
    vulnerability_weight: f64,
    maintainability_weight: f64,
}

/// Complete health score result matching Python structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthScore {
    pub overall_score: f64,
    pub functionality_score: f64,
    pub maintainability_score: f64,
    pub security_score: f64,
    pub performance_score: f64,
    pub score_breakdown: HashMap<String, f64>,
    pub critical_issues_count: usize,
    pub recommendations: Vec<String>,
}

/// Complexity metrics for health calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: f64,
    pub cognitive_complexity: f64,
    pub lines_of_code: usize,
    pub functions_count: usize,
    pub classes_count: usize,
    pub average_function_length: f64,
    pub max_function_complexity: f64,
}

/// Issue severity levels matching Python classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Security vulnerability types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub severity: String,
    pub category: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub description: String,
}

/// Performance hotspot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    pub file_path: String,
    pub function_name: String,
    pub complexity_score: f64,
    pub execution_frequency: f64,
    pub memory_usage: f64,
}

impl Default for HealthScoreCalculator {
    fn default() -> Self {
        // Default weights matching Python implementation
        Self {
            complexity_weight: 0.25,
            bug_weight: 0.30,
            vulnerability_weight: 0.25,
            maintainability_weight: 0.20,
        }
    }
}

impl HealthScoreCalculator {
    /// Create new health score calculator with custom weights
    pub fn new(
        complexity_weight: f64,
        bug_weight: f64,
        vulnerability_weight: f64,
        maintainability_weight: f64,
    ) -> Self {
        Self {
            complexity_weight,
            bug_weight,
            vulnerability_weight,
            maintainability_weight,
        }
    }
    
    /// Calculate complete health score from analysis data
    pub fn calculate_health_score(
        &self,
        extracted_data: &HashMap<String, serde_json::Value>,
    ) -> Result<HealthScore> {
        // Extract metrics from analysis data
        let complexity_metrics = self.extract_complexity_metrics(extracted_data)?;
        let issues = self.extract_issues(extracted_data)?;
        let vulnerabilities = self.extract_vulnerabilities(extracted_data)?;
        let hotspots = self.extract_hotspots(extracted_data)?;
        
        // Calculate individual scores using exact Python algorithms
        let functionality_score = self.calculate_functionality_score(&issues)?;
        let maintainability_score = self.calculate_maintainability_score(&complexity_metrics)?;
        let security_score = self.calculate_security_score(&vulnerabilities)?;
        let performance_score = self.calculate_performance_score(&hotspots)?;
        
        // Calculate weighted overall score (exact Python formula)
        let overall_score = (
            functionality_score * self.bug_weight +
            maintainability_score * self.maintainability_weight +
            security_score * self.vulnerability_weight +
            performance_score * self.complexity_weight
        ).min(100.0_f64).max(0.0_f64);
        
        // Create score breakdown
        let mut score_breakdown = HashMap::new();
        score_breakdown.insert("functionality".to_string(), functionality_score);
        score_breakdown.insert("maintainability".to_string(), maintainability_score);
        score_breakdown.insert("security".to_string(), security_score);
        score_breakdown.insert("performance".to_string(), performance_score);
        
        // Count critical issues
        let critical_issues_count = issues.iter()
            .filter(|issue| {
                issue.get("severity")
                    .and_then(|s| s.as_str())
                    .map(|s| s == "critical" || s == "error")
                    .unwrap_or(false)
            })
            .count();
        
        // Generate recommendations based on scores
        let recommendations = self.generate_recommendations(
            functionality_score,
            maintainability_score,
            security_score,
            performance_score,
            critical_issues_count,
        );
        
        Ok(HealthScore {
            overall_score,
            functionality_score,
            maintainability_score,
            security_score,
            performance_score,
            score_breakdown,
            critical_issues_count,
            recommendations,
        })
    }
    
    /// Calculate functionality score (exact Python algorithm)
    pub fn calculate_functionality_score(
        &self,
        issues: &[serde_json::Value],
    ) -> Result<f64> {
        if issues.is_empty() {
            return Ok(100.0);
        }
        
        let mut score: f64 = 100.0;
        
        for issue in issues {
            let severity = issue.get("severity")
                .and_then(|s| s.as_str())
                .unwrap_or("info");
                
            let penalty = match severity {
                "critical" | "error" => 25.0,
                "warning" | "high" => 10.0,
                "info" | "medium" => 5.0,
                "low" => 2.0,
                _ => 1.0,
            };
            
            score -= penalty;
        }
        
        Ok(score.max(0.0_f64))
    }
    
    /// Calculate maintainability score (exact Python algorithm)
    pub fn calculate_maintainability_score(
        &self,
        complexity: &ComplexityMetrics,
    ) -> Result<f64> {
        let mut score: f64 = 100.0;
        
        // Cyclomatic complexity penalty (Python thresholds)
        if complexity.cyclomatic_complexity > 15.0 {
            score -= (complexity.cyclomatic_complexity - 15.0) * 2.0;
        } else if complexity.cyclomatic_complexity > 10.0 {
            score -= (complexity.cyclomatic_complexity - 10.0) * 1.0;
        }
        
        // Cognitive complexity penalty
        if complexity.cognitive_complexity > 25.0 {
            score -= (complexity.cognitive_complexity - 25.0) * 1.5;
        }
        
        // Function length penalty
        if complexity.average_function_length > 50.0 {
            score -= (complexity.average_function_length - 50.0) * 0.5;
        }
        
        // Maximum function complexity penalty
        if complexity.max_function_complexity > 20.0 {
            score -= (complexity.max_function_complexity - 20.0) * 3.0;
        }
        
        Ok(score.max(0.0_f64))
    }
    
    /// Calculate security score (exact Python algorithm)
    pub fn calculate_security_score(
        &self,
        vulnerabilities: &[Vulnerability],
    ) -> Result<f64> {
        if vulnerabilities.is_empty() {
            return Ok(100.0);
        }
        
        let mut score: f64 = 100.0;
        
        for vuln in vulnerabilities {
            let penalty = match vuln.severity.as_str() {
                "critical" => 30.0,
                "high" => 20.0,
                "medium" => 10.0,
                "low" => 5.0,
                _ => 2.0,
            };
            
            score -= penalty;
        }
        
        Ok(score.max(0.0_f64))
    }
    
    /// Calculate performance score (exact Python algorithm)
    pub fn calculate_performance_score(
        &self,
        hotspots: &[Hotspot],
    ) -> Result<f64> {
        if hotspots.is_empty() {
            return Ok(100.0);
        }
        
        let mut score: f64 = 100.0;
        
        for hotspot in hotspots {
            // Penalty based on complexity and execution frequency
            let penalty = (hotspot.complexity_score * hotspot.execution_frequency) / 10.0;
            score -= penalty;
            
            // Additional penalty for high memory usage
            if hotspot.memory_usage > 100.0 {
                score -= (hotspot.memory_usage - 100.0) * 0.1;
            }
        }
        
        Ok(score.max(0.0_f64))
    }
    
    /// Extract complexity metrics from analysis data
    fn extract_complexity_metrics(
        &self,
        data: &HashMap<String, serde_json::Value>,
    ) -> Result<ComplexityMetrics> {
        let topology_data = data.get("topology")
            .ok_or_else(|| Error::Analysis("Missing topology data".to_string()))?;
            
        let summary = topology_data.get("summary")
            .ok_or_else(|| Error::Analysis("Missing topology summary".to_string()))?;
        
        Ok(ComplexityMetrics {
            cyclomatic_complexity: summary.get("average_complexity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            cognitive_complexity: summary.get("cognitive_complexity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            lines_of_code: summary.get("total_code_lines")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            functions_count: summary.get("total_functions")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            classes_count: summary.get("total_classes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            average_function_length: summary.get("average_function_length")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            max_function_complexity: summary.get("max_complexity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        })
    }
    
    /// Extract issues from analysis data
    fn extract_issues(
        &self,
        data: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>> {
        let empty_object = serde_json::Value::Object(serde_json::Map::new());
        let issues_data = data.get("issues")
            .or_else(|| data.get("issues_inspection"))
            .unwrap_or(&empty_object);
        
        // Combine all issue types
        let mut all_issues = Vec::new();
        
        if let Some(pylint_issues) = issues_data.get("pylint_issues").and_then(|v| v.as_array()) {
            all_issues.extend(pylint_issues.iter().cloned());
        }
        
        if let Some(ruff_issues) = issues_data.get("ruff_issues").and_then(|v| v.as_array()) {
            all_issues.extend(ruff_issues.iter().cloned());
        }
        
        if let Some(bandit_issues) = issues_data.get("bandit_issues").and_then(|v| v.as_array()) {
            all_issues.extend(bandit_issues.iter().cloned());
        }
        
        if let Some(mypy_issues) = issues_data.get("mypy_issues").and_then(|v| v.as_array()) {
            all_issues.extend(mypy_issues.iter().cloned());
        }
        
        Ok(all_issues)
    }
    
    /// Extract vulnerabilities from analysis data
    fn extract_vulnerabilities(
        &self,
        data: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Vulnerability>> {
        let empty_object = serde_json::Value::Object(serde_json::Map::new());
        let security_data = data.get("security")
            .unwrap_or(&empty_object);
            
        let empty_vec = vec![];
        let vulnerabilities = security_data.get("vulnerabilities")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
            
        let mut result = Vec::new();
        
        for vuln in vulnerabilities {
            if let (Some(severity), Some(category), Some(file_path), Some(description)) = (
                vuln.get("severity").and_then(|v| v.as_str()),
                vuln.get("category").and_then(|v| v.as_str()),
                vuln.get("file_path").and_then(|v| v.as_str()),
                vuln.get("description").and_then(|v| v.as_str()),
            ) {
                result.push(Vulnerability {
                    severity: severity.to_string(),
                    category: category.to_string(),
                    file_path: file_path.to_string(),
                    line_number: vuln.get("line_number").and_then(|v| v.as_u64()).map(|n| n as usize),
                    description: description.to_string(),
                });
            }
        }
        
        Ok(result)
    }
    
    /// Extract performance hotspots from analysis data
    fn extract_hotspots(
        &self,
        data: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Hotspot>> {
        let empty_object = serde_json::Value::Object(serde_json::Map::new());
        let performance_data = data.get("performance")
            .unwrap_or(&empty_object);
            
        let empty_vec = vec![];
        let hotspots = performance_data.get("hotspots")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
            
        let mut result = Vec::new();
        
        for hotspot in hotspots {
            if let (Some(file_path), Some(function_name)) = (
                hotspot.get("file_path").and_then(|v| v.as_str()),
                hotspot.get("function_name").and_then(|v| v.as_str()),
            ) {
                result.push(Hotspot {
                    file_path: file_path.to_string(),
                    function_name: function_name.to_string(),
                    complexity_score: hotspot.get("complexity_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    execution_frequency: hotspot.get("execution_frequency").and_then(|v| v.as_f64()).unwrap_or(1.0),
                    memory_usage: hotspot.get("memory_usage").and_then(|v| v.as_f64()).unwrap_or(0.0),
                });
            }
        }
        
        Ok(result)
    }
    
    /// Generate health recommendations based on scores
    fn generate_recommendations(
        &self,
        functionality_score: f64,
        maintainability_score: f64,
        security_score: f64,
        performance_score: f64,
        critical_issues_count: usize,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if critical_issues_count > 0 {
            recommendations.push(format!(
                "🚨 Address {} critical issues immediately",
                critical_issues_count
            ));
        }
        
        if functionality_score < 70.0 {
            recommendations.push("🐛 Focus on fixing functional issues and bugs".to_string());
        }
        
        if maintainability_score < 70.0 {
            recommendations.push("🔧 Reduce code complexity and improve maintainability".to_string());
        }
        
        if security_score < 70.0 {
            recommendations.push("🔒 Address security vulnerabilities and hardening".to_string());
        }
        
        if performance_score < 70.0 {
            recommendations.push("⚡ Optimize performance hotspots and bottlenecks".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("✅ Codebase health is good - maintain current quality".to_string());
        }
        
        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_functionality_score_calculation() {
        let calculator = HealthScoreCalculator::default();
        
        // No issues should give perfect score
        let score = calculator.calculate_functionality_score(&[]).unwrap();
        assert_eq!(score, 100.0);
        
        // Critical issues should heavily penalize
        let critical_issue = serde_json::json!({
            "severity": "critical",
            "message": "Critical bug"
        });
        let score = calculator.calculate_functionality_score(&[critical_issue]).unwrap();
        assert_eq!(score, 75.0); // 100 - 25
    }
    
    #[test]
    fn test_maintainability_score_calculation() {
        let calculator = HealthScoreCalculator::default();
        
        // Low complexity should give high score
        let low_complexity = ComplexityMetrics {
            cyclomatic_complexity: 5.0,
            cognitive_complexity: 10.0,
            lines_of_code: 1000,
            functions_count: 50,
            classes_count: 10,
            average_function_length: 20.0,
            max_function_complexity: 8.0,
        };
        
        let score = calculator.calculate_maintainability_score(&low_complexity).unwrap();
        assert_eq!(score, 100.0);
        
        // High complexity should penalize
        let high_complexity = ComplexityMetrics {
            cyclomatic_complexity: 20.0,
            cognitive_complexity: 30.0,
            lines_of_code: 1000,
            functions_count: 50,
            classes_count: 10,
            average_function_length: 60.0,
            max_function_complexity: 25.0,
        };
        
        let score = calculator.calculate_maintainability_score(&high_complexity).unwrap();
        assert!(score < 50.0); // Should be heavily penalized
    }
}