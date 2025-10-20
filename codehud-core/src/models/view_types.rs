//! Core data models for CodeHUD analysis and visualization.
//!
//! This module provides the fundamental data structures used throughout CodeHUD
//! for representing code analysis results, visualization types, and semantic information.
//!
//! This is a 1:1 translation from Python src/codehud/core/models.py
//! to ensure zero degradation in data model behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Available visualization view types.
///
/// This enum exactly matches the Python ViewType enum to ensure
/// complete compatibility across all visualization systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewType {
    Topology,
    Flow,
    Evolution,
    Quality,
    Dependencies,
    Security,
    Performance,
    Testing,
    IssuesInspection,     // New view type
    FixRollbackDevnotes,  // LLM fix tracking
    TreeSitterAnalysis,   // Enhanced tree-sitter semantic analysis
}

impl ViewType {
    /// Get all available view types
    pub fn all() -> Vec<ViewType> {
        vec![
            ViewType::Topology,
            ViewType::Flow,
            ViewType::Evolution,
            ViewType::Quality,
            ViewType::Dependencies,
            ViewType::Security,
            ViewType::Performance,
            ViewType::Testing,
            ViewType::IssuesInspection,
            ViewType::FixRollbackDevnotes,
            ViewType::TreeSitterAnalysis,
        ]
    }

    /// Get the string representation matching Python behavior
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewType::Topology => "topology",
            ViewType::Flow => "flow",
            ViewType::Evolution => "evolution",
            ViewType::Quality => "quality",
            ViewType::Dependencies => "dependencies",
            ViewType::Security => "security",
            ViewType::Performance => "performance",
            ViewType::Testing => "testing",
            ViewType::IssuesInspection => "issues_inspection",
            ViewType::FixRollbackDevnotes => "fix_rollback_devnotes",
            ViewType::TreeSitterAnalysis => "tree_sitter_analysis",
        }
    }

    /// Parse from string, matching Python behavior exactly
    pub fn from_str(s: &str) -> Option<ViewType> {
        match s.to_lowercase().as_str() {
            "topology" => Some(ViewType::Topology),
            "flow" => Some(ViewType::Flow),
            "evolution" => Some(ViewType::Evolution),
            "quality" => Some(ViewType::Quality),
            "dependencies" => Some(ViewType::Dependencies),
            "security" => Some(ViewType::Security),
            "performance" => Some(ViewType::Performance),
            "testing" => Some(ViewType::Testing),
            "issues_inspection" => Some(ViewType::IssuesInspection),
            "fix_rollback_devnotes" => Some(ViewType::FixRollbackDevnotes),
            "tree_sitter_analysis" => Some(ViewType::TreeSitterAnalysis),
            _ => None,
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ViewType::Topology => "Topology",
            ViewType::Flow => "Data Flow",
            ViewType::Evolution => "Evolution",
            ViewType::Quality => "Code Quality",
            ViewType::Dependencies => "Dependencies",
            ViewType::Security => "Security",
            ViewType::Performance => "Performance",
            ViewType::Testing => "Testing",
            ViewType::IssuesInspection => "Issues & Inspection",
            ViewType::FixRollbackDevnotes => "Fix/Rollback/DevNotes",
            ViewType::TreeSitterAnalysis => "Tree-sitter Analysis",
        }
    }

    /// Check if this view supports focus functionality
    pub fn supports_focus(&self) -> bool {
        match self {
            ViewType::Topology | ViewType::Dependencies | ViewType::Flow => true,
            _ => false,
        }
    }
}

impl fmt::Display for ViewType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ViewType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| format!("Unknown view type: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_type_string_conversion() {
        // Test all view types round-trip correctly
        for view_type in ViewType::all() {
            let as_string = view_type.as_str();
            let parsed = ViewType::from_str(as_string).unwrap();
            assert_eq!(view_type, parsed);
        }
    }

    #[test]
    fn test_python_compatibility() {
        // Ensure exact string matching with Python enum values
        assert_eq!(ViewType::Topology.as_str(), "topology");
        assert_eq!(ViewType::IssuesInspection.as_str(), "issues_inspection");
        assert_eq!(ViewType::FixRollbackDevnotes.as_str(), "fix_rollback_devnotes");
    }

    #[test]
    fn test_case_insensitive_parsing() {
        assert_eq!(ViewType::from_str("TOPOLOGY"), Some(ViewType::Topology));
        assert_eq!(ViewType::from_str("Quality"), Some(ViewType::Quality));
        assert_eq!(ViewType::from_str("issues_inspection"), Some(ViewType::IssuesInspection));
    }

    #[test]
    fn test_invalid_view_type() {
        assert_eq!(ViewType::from_str("invalid"), None);
        assert_eq!(ViewType::from_str(""), None);
    }

    #[test]
    fn test_focus_support() {
        assert!(ViewType::Topology.supports_focus());
        assert!(ViewType::Dependencies.supports_focus());
        assert!(ViewType::Flow.supports_focus());
        assert!(!ViewType::Quality.supports_focus());
        assert!(!ViewType::Security.supports_focus());
    }

    #[test]
    fn test_display_names() {
        assert_eq!(ViewType::IssuesInspection.display_name(), "Issues & Inspection");
        assert_eq!(ViewType::FixRollbackDevnotes.display_name(), "Fix/Rollback/DevNotes");
    }

    #[test]
    fn test_serde_serialization() {
        let view_type = ViewType::Topology;
        let json = serde_json::to_string(&view_type).unwrap();
        let deserialized: ViewType = serde_json::from_str(&json).unwrap();
        assert_eq!(view_type, deserialized);
    }

    #[test]
    fn test_all_view_types_count() {
        // Ensure we have all 10 view types from Python
        assert_eq!(ViewType::all().len(), 10);
    }
}