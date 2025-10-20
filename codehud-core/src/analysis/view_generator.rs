//! View Generator
//!
//! Generates formatted output for different view types

use crate::{Result, ViewType};
use crate::models::analysis_result::AnalysisResult;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct ViewGenerator;

impl ViewGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate formatted view output for the specified view type
    pub fn generate_view_output(&self, result: &AnalysisResult, view_type: ViewType) -> Result<Value> {
        match view_type {
            ViewType::Topology => self.generate_topology_view(result),
            ViewType::Quality => self.generate_quality_view(result),
            ViewType::Security => self.generate_security_view(result),
            ViewType::Dependencies => self.generate_dependencies_view(result),
            ViewType::Performance => self.generate_performance_view(result),
            ViewType::Evolution => self.generate_evolution_view(result),
            ViewType::IssuesInspection => self.generate_issues_view(result),
            ViewType::Testing => self.generate_orphaned_files_view(result),
            ViewType::Flow => self.generate_flow_view(result),
            ViewType::FixRollbackDevnotes => self.generate_devnotes_view(result),
            ViewType::TreeSitterAnalysis => self.generate_tree_sitter_view(result),
        }
    }

    /// Generate a comprehensive summary view
    pub fn generate_summary_view(&self, result: &AnalysisResult) -> Result<Value> {
        Ok(json!({
            "overview": {
                "codebase_path": result.codebase_path,
                "files_analyzed": result.files_analyzed,
                "analysis_duration": result.analysis_duration,
                "health_score": result.health_score
            },
            "metrics": result.metrics,
            "critical_issues_count": result.critical_issues.len(),
            "has_critical_issues": result.has_critical_issues(),
            "focus_recommendations": result.focus_recommendations,
            "issue_severity_distribution": result.get_issue_severity_distribution()
        }))
    }

    fn generate_topology_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(topology_data) = result.get_view_data("topology") {
            Ok(json!({
                "view_type": "topology",
                "title": "Code Structure Analysis",
                "data": topology_data,
                "summary": self.extract_topology_summary(topology_data)
            }))
        } else {
            Ok(json!({
                "view_type": "topology",
                "title": "Code Structure Analysis",
                "error": "Topology data not available"
            }))
        }
    }

    fn generate_quality_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(quality_data) = result.get_view_data("quality") {
            Ok(json!({
                "view_type": "quality",
                "title": "Code Quality Analysis",
                "data": quality_data,
                "summary": self.extract_quality_summary(quality_data)
            }))
        } else {
            Ok(json!({
                "view_type": "quality",
                "title": "Code Quality Analysis",
                "error": "Quality data not available"
            }))
        }
    }

    fn generate_security_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(security_data) = result.get_view_data("security") {
            Ok(json!({
                "view_type": "security",
                "title": "Security Analysis",
                "data": security_data,
                "summary": self.extract_security_summary(security_data)
            }))
        } else {
            Ok(json!({
                "view_type": "security",
                "title": "Security Analysis",
                "error": "Security data not available"
            }))
        }
    }

    fn generate_dependencies_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(deps_data) = result.get_view_data("dependencies") {
            Ok(json!({
                "view_type": "dependencies",
                "title": "Dependencies Analysis",
                "data": deps_data,
                "summary": self.extract_dependencies_summary(deps_data)
            }))
        } else {
            Ok(json!({
                "view_type": "dependencies",
                "title": "Dependencies Analysis",
                "error": "Dependencies data not available"
            }))
        }
    }

    fn generate_performance_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(performance_data) = result.get_view_data("performance") {
            Ok(json!({
                "view_type": "performance",
                "title": "Performance Analysis",
                "data": performance_data,
                "summary": self.extract_performance_summary(performance_data)
            }))
        } else {
            Ok(json!({
                "view_type": "performance",
                "title": "Performance Analysis",
                "error": "Performance data not available"
            }))
        }
    }

    fn generate_evolution_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(evolution_data) = result.get_view_data("evolution") {
            Ok(json!({
                "view_type": "evolution",
                "title": "Code Evolution Analysis",
                "data": evolution_data,
                "summary": self.extract_evolution_summary(evolution_data)
            }))
        } else {
            Ok(json!({
                "view_type": "evolution",
                "title": "Code Evolution Analysis",
                "error": "Evolution data not available"
            }))
        }
    }

    fn generate_issues_view(&self, result: &AnalysisResult) -> Result<Value> {
        Ok(json!({
            "view_type": "issues",
            "title": "Issues Analysis",
            "critical_issues": result.critical_issues,
            "critical_count": result.critical_issues.len(),
            "severity_distribution": result.get_issue_severity_distribution()
        }))
    }

    fn generate_orphaned_files_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(orphaned_data) = result.get_view_data("testing") {
            Ok(json!({
                "view_type": "orphaned_files",
                "title": "Testing & Orphaned Files Analysis",
                "data": orphaned_data,
                "summary": self.extract_testing_summary(orphaned_data)
            }))
        } else {
            Ok(json!({
                "view_type": "orphaned_files",
                "title": "Testing & Orphaned Files Analysis",
                "error": "Testing data not available"
            }))
        }
    }

    fn generate_flow_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(flow_data) = result.get_view_data("flow") {
            Ok(json!({
                "view_type": "flow",
                "title": "Data Flow Analysis",
                "data": flow_data,
                "summary": self.extract_flow_summary(flow_data)
            }))
        } else {
            Ok(json!({
                "view_type": "flow",
                "title": "Data Flow Analysis",
                "error": "Flow data not available"
            }))
        }
    }

    fn generate_devnotes_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(devnotes_data) = result.get_view_data("devnotes") {
            Ok(json!({
                "view_type": "devnotes",
                "title": "Developer Notes & Fix Tracking",
                "data": devnotes_data,
                "summary": self.extract_devnotes_summary(devnotes_data)
            }))
        } else {
            Ok(json!({
                "view_type": "devnotes",
                "title": "Developer Notes & Fix Tracking",
                "error": "Developer notes data not available"
            }))
        }
    }

    fn generate_tree_sitter_view(&self, result: &AnalysisResult) -> Result<Value> {
        if let Some(tree_sitter_data) = result.get_view_data("tree_sitter_analysis") {
            Ok(json!({
                "view_type": "tree_sitter_analysis",
                "title": "Tree-sitter Enhanced Analysis",
                "data": tree_sitter_data,
                "summary": self.extract_tree_sitter_summary(tree_sitter_data)
            }))
        } else {
            Ok(json!({
                "view_type": "tree_sitter_analysis",
                "title": "Tree-sitter Enhanced Analysis",
                "error": "Tree-sitter analysis data not available"
            }))
        }
    }

    fn extract_topology_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "files": summary.get("total_files").unwrap_or(&json!(0)),
                "functions": summary.get("total_functions").unwrap_or(&json!(0)),
                "classes": summary.get("total_classes").unwrap_or(&json!(0)),
                "imports": summary.get("total_imports").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_quality_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "health_score": summary.get("health_score").unwrap_or(&json!(0.0)),
                "avg_complexity": summary.get("average_complexity").unwrap_or(&json!(0.0)),
                "max_complexity": summary.get("max_complexity").unwrap_or(&json!(0.0)),
                "issues_found": summary.get("total_issues").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_security_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "risk_level": data.get("risk_assessment").and_then(|r| r.get("level")).unwrap_or(&json!("unknown")),
                "high_severity": summary.get("high_severity_findings").unwrap_or(&json!(0)),
                "medium_severity": summary.get("medium_severity_findings").unwrap_or(&json!(0)),
                "low_severity": summary.get("low_severity_findings").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_dependencies_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "total_imports": summary.get("total_import_statements").unwrap_or(&json!(0)),
                "circular_deps": summary.get("circular_dependencies_found").unwrap_or(&json!(0)),
                "external_deps": summary.get("external_dependencies").unwrap_or(&json!(0)),
                "files_with_deps": summary.get("files_with_dependencies").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_performance_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "hotspots_found": summary.get("total_hotspots").unwrap_or(&json!(0)),
                "performance_issues": summary.get("performance_issues").unwrap_or(&json!(0)),
                "bottlenecks": summary.get("bottlenecks_identified").unwrap_or(&json!(0)),
                "average_complexity": summary.get("average_complexity").unwrap_or(&json!(0.0))
            })
        } else {
            json!({})
        }
    }

    fn extract_evolution_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "total_commits": summary.get("total_commits").unwrap_or(&json!(0)),
                "active_authors": summary.get("active_authors").unwrap_or(&json!(0)),
                "files_analyzed": summary.get("files_with_history").unwrap_or(&json!(0)),
                "evolution_patterns": summary.get("patterns_identified").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_testing_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "test_files": summary.get("test_files_found").unwrap_or(&json!(0)),
                "orphaned_files": summary.get("orphaned_files").unwrap_or(&json!(0)),
                "test_coverage": summary.get("estimated_coverage").unwrap_or(&json!(0.0)),
                "testing_patterns": summary.get("testing_patterns_found").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_flow_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "data_flows": summary.get("data_flows_analyzed").unwrap_or(&json!(0)),
                "control_flows": summary.get("control_flows_analyzed").unwrap_or(&json!(0)),
                "flow_complexity": summary.get("average_flow_complexity").unwrap_or(&json!(0.0)),
                "bottlenecks": summary.get("flow_bottlenecks").unwrap_or(&json!(0))
            })
        } else {
            json!({})
        }
    }

    fn extract_devnotes_summary(&self, data: &Value) -> Value {
        if let Some(summary) = data.get("summary") {
            json!({
                "total_notes": summary.get("total_devnotes").unwrap_or(&json!(0)),
                "todo_items": summary.get("todo_items").unwrap_or(&json!(0)),
                "fixme_items": summary.get("fixme_items").unwrap_or(&json!(0)),
                "test_coverage": summary.get("test_coverage").unwrap_or(&json!(0.0))
            })
        } else {
            json!({})
        }
    }

    fn extract_tree_sitter_summary(&self, data: &Value) -> Value {
        if let Some(imports_data) = data.get("imports") {
            if let Some(summary) = imports_data.get("summary") {
                json!({
                    "total_imports": summary.get("total_imports").unwrap_or(&json!(0)),
                    "unique_modules": summary.get("unique_modules").unwrap_or(&json!(0)),
                    "external_dependencies": summary.get("external_dependencies").unwrap_or(&json!([])),
                    "analysis_method": "Enhanced Tree-sitter Queries",
                    "symbols_detected": data.get("highlights")
                        .and_then(|h| h.get("summary"))
                        .and_then(|s| s.get("total_symbols"))
                        .unwrap_or(&json!(0))
                })
            } else {
                json!({
                    "analysis_method": "Enhanced Tree-sitter Queries",
                    "error": "No import summary available"
                })
            }
        } else {
            json!({
                "analysis_method": "Enhanced Tree-sitter Queries",
                "error": "No tree-sitter data available"
            })
        }
    }
}

impl Default for ViewGenerator {
    fn default() -> Self {
        Self::new()
    }
}