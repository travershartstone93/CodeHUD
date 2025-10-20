//! Graph edge definitions for CodeHUD graph analysis
//!
//! Defines the edge types used in different graph representations:
//! - CallEdge: Function call relationships
//! - DependencyEdge: Module dependency relationships
//! - InheritanceEdge: Class inheritance relationships

use serde::{Serialize, Deserialize};

/// Edge representing a function call relationship
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallEdge {
    /// Number of times this call is made
    pub call_count: usize,
    /// Weight of this edge for graph algorithms
    pub weight: f64,
}

impl CallEdge {
    /// Create a new call edge
    pub fn new(call_count: usize) -> Self {
        Self {
            call_count,
            weight: call_count as f64,
        }
    }

    /// Create a call edge with custom weight
    pub fn with_weight(call_count: usize, weight: f64) -> Self {
        Self {
            call_count,
            weight,
        }
    }

    /// Get the call frequency (calls per unit, normalized)
    pub fn frequency(&self) -> f64 {
        self.weight
    }
}

/// Edge representing a module dependency relationship
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Type of import (import, from_import, etc.)
    pub import_type: String,
    /// Weight of this dependency for graph algorithms
    pub weight: f64,
}

impl DependencyEdge {
    /// Create a new dependency edge
    pub fn new(import_type: String) -> Self {
        Self {
            import_type,
            weight: 1.0,
        }
    }

    /// Create a dependency edge with custom weight
    pub fn with_weight(import_type: String, weight: f64) -> Self {
        Self {
            import_type,
            weight,
        }
    }

    /// Check if this is a specific import type
    pub fn is_import_type(&self, import_type: &str) -> bool {
        self.import_type == import_type
    }

    /// Get dependency strength (weight)
    pub fn strength(&self) -> f64 {
        self.weight
    }
}

/// Edge representing a class inheritance relationship
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InheritanceEdge {
    /// Type of inheritance (extends, implements, etc.)
    pub inheritance_type: String,
    /// Weight of this inheritance for graph algorithms
    pub weight: f64,
}

impl InheritanceEdge {
    /// Create a new inheritance edge
    pub fn new(inheritance_type: String) -> Self {
        Self {
            inheritance_type,
            weight: 1.0,
        }
    }

    /// Create an inheritance edge with custom weight
    pub fn with_weight(inheritance_type: String, weight: f64) -> Self {
        Self {
            inheritance_type,
            weight,
        }
    }

    /// Check if this is a specific inheritance type
    pub fn is_inheritance_type(&self, inheritance_type: &str) -> bool {
        self.inheritance_type == inheritance_type
    }

    /// Get inheritance strength (weight)
    pub fn strength(&self) -> f64 {
        self.weight
    }
}

/// Trait for graph edges to provide common functionality
pub trait GraphEdge {
    /// Get the weight of this edge
    fn weight(&self) -> f64;

    /// Get the edge type as string
    fn edge_type(&self) -> &str;

    /// Check if this edge is significant (weight above threshold)
    fn is_significant(&self, threshold: f64) -> bool {
        self.weight() >= threshold
    }
}

impl GraphEdge for CallEdge {
    fn weight(&self) -> f64 {
        self.weight
    }

    fn edge_type(&self) -> &str {
        "call"
    }
}

impl GraphEdge for DependencyEdge {
    fn weight(&self) -> f64 {
        self.weight
    }

    fn edge_type(&self) -> &str {
        &self.import_type
    }
}

impl GraphEdge for InheritanceEdge {
    fn weight(&self) -> f64 {
        self.weight
    }

    fn edge_type(&self) -> &str {
        &self.inheritance_type
    }
}

/// Edge metadata for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMetadata {
    /// Source file of the edge
    pub source_file: String,
    /// Target file of the edge
    pub target_file: String,
    /// Line number where relationship is defined
    pub line_number: Option<usize>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl EdgeMetadata {
    /// Create new edge metadata
    pub fn new(source_file: String, target_file: String) -> Self {
        Self {
            source_file,
            target_file,
            line_number: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add line number information
    pub fn with_line_number(mut self, line_number: usize) -> Self {
        self.line_number = Some(line_number);
        self
    }

    /// Add metadata key-value pair
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if files are in same directory
    pub fn is_same_directory(&self) -> bool {
        if let (Some(source_dir), Some(target_dir)) = (
            std::path::Path::new(&self.source_file).parent(),
            std::path::Path::new(&self.target_file).parent()
        ) {
            source_dir == target_dir
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_edge_creation() {
        let edge = CallEdge::new(5);
        assert_eq!(edge.call_count, 5);
        assert_eq!(edge.weight, 5.0);
        assert_eq!(edge.frequency(), 5.0);
    }

    #[test]
    fn test_call_edge_with_custom_weight() {
        let edge = CallEdge::with_weight(3, 2.5);
        assert_eq!(edge.call_count, 3);
        assert_eq!(edge.weight, 2.5);
        assert_eq!(edge.frequency(), 2.5);
    }

    #[test]
    fn test_dependency_edge_creation() {
        let edge = DependencyEdge::new("import".to_string());
        assert_eq!(edge.import_type, "import");
        assert_eq!(edge.weight, 1.0);
        assert!(edge.is_import_type("import"));
        assert!(!edge.is_import_type("from_import"));
    }

    #[test]
    fn test_inheritance_edge_creation() {
        let edge = InheritanceEdge::new("extends".to_string());
        assert_eq!(edge.inheritance_type, "extends");
        assert_eq!(edge.weight, 1.0);
        assert!(edge.is_inheritance_type("extends"));
        assert!(!edge.is_inheritance_type("implements"));
    }

    #[test]
    fn test_graph_edge_trait() {
        let call_edge = CallEdge::new(10);
        let dep_edge = DependencyEdge::new("import".to_string());
        let inh_edge = InheritanceEdge::new("extends".to_string());

        assert_eq!(call_edge.weight(), 10.0);
        assert_eq!(call_edge.edge_type(), "call");
        assert!(call_edge.is_significant(5.0));
        assert!(!call_edge.is_significant(15.0));

        assert_eq!(dep_edge.weight(), 1.0);
        assert_eq!(dep_edge.edge_type(), "import");

        assert_eq!(inh_edge.weight(), 1.0);
        assert_eq!(inh_edge.edge_type(), "extends");
    }

    #[test]
    fn test_edge_metadata() {
        let metadata = EdgeMetadata::new("src/a.py".to_string(), "src/b.py".to_string())
            .with_line_number(10)
            .with_metadata("context".to_string(), "function_call".to_string());

        assert_eq!(metadata.source_file, "src/a.py");
        assert_eq!(metadata.target_file, "src/b.py");
        assert_eq!(metadata.line_number, Some(10));
        assert!(metadata.is_same_directory());

        let metadata2 = EdgeMetadata::new("src/a.py".to_string(), "lib/b.py".to_string());
        assert!(!metadata2.is_same_directory());
    }
}