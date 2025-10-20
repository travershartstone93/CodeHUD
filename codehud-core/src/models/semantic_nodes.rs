//! Semantic node models for code analysis and graph construction.
//!
//! This module provides data structures for representing code elements
//! and their relationships in a semantic graph structure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Function signature with comprehensive semantic information.
///
/// This struct exactly matches the Python FunctionSignature dataclass
/// to ensure complete compatibility in function analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub args: Vec<String>,
    pub defaults: Vec<String>,
    pub vararg: Option<String>,
    pub kwarg: Option<String>,
    pub return_annotation: Option<String>,
    pub decorators: Vec<String>,
    pub docstring: Option<String>,
    pub complexity: i32,
    pub line_start: i32,
    pub line_end: i32,
    pub file_path: Option<String>,

    // Extensions for CodeHUD integration
    pub call_frequency: Option<f64>,
    pub performance_class: Option<String>,
    pub side_effects: Vec<String>,
    pub error_conditions: Vec<String>,
}

impl FunctionSignature {
    /// Create a new function signature with required fields
    pub fn new(
        name: String,
        args: Vec<String>,
        complexity: i32,
        line_start: i32,
        line_end: i32,
    ) -> Self {
        Self {
            name,
            args,
            defaults: Vec::new(),
            vararg: None,
            kwarg: None,
            return_annotation: None,
            decorators: Vec::new(),
            docstring: None,
            complexity,
            line_start,
            line_end,
            file_path: None,
            call_frequency: None,
            performance_class: None,
            side_effects: Vec::new(),
            error_conditions: Vec::new(),
        }
    }

    /// Check if function has side effects
    pub fn has_side_effects(&self) -> bool {
        !self.side_effects.is_empty()
    }

    /// Check if function is a property/getter based on decorators
    pub fn is_property(&self) -> bool {
        self.decorators.iter().any(|d| d == "property")
    }

    /// Get function signature as string (Python-like format)
    pub fn signature_string(&self) -> String {
        let mut parts = Vec::new();
        
        // Add regular arguments
        parts.extend(self.args.clone());
        
        // Add vararg
        if let Some(vararg) = &self.vararg {
            parts.push(format!("*{}", vararg));
        }
        
        // Add kwarg
        if let Some(kwarg) = &self.kwarg {
            parts.push(format!("**{}", kwarg));
        }
        
        let args_str = parts.join(", ");
        
        match &self.return_annotation {
            Some(ret_type) => format!("{}({}) -> {}", self.name, args_str, ret_type),
            None => format!("{}({})", self.name, args_str),
        }
    }
}

/// Class definition with inheritance and semantic analysis.
///
/// This struct exactly matches the Python ClassDefinition dataclass
/// to ensure complete compatibility in class analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub name: String,
    pub bases: Vec<String>,
    pub decorators: Vec<String>,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub docstring: Option<String>,
    pub line_start: i32,
    pub line_end: i32,
    pub file_path: Option<String>,

    // Extensions for CodeHUD
    pub design_patterns: Vec<String>,
    pub coupling_score: f64,
    pub cohesion_score: f64,
}

impl ClassDefinition {
    /// Create a new class definition with required fields
    pub fn new(
        name: String,
        line_start: i32,
        line_end: i32,
    ) -> Self {
        Self {
            name,
            bases: Vec::new(),
            decorators: Vec::new(),
            methods: Vec::new(),
            properties: Vec::new(),
            docstring: None,
            line_start,
            line_end,
            file_path: None,
            design_patterns: Vec::new(),
            coupling_score: 0.0,
            cohesion_score: 0.0,
        }
    }

    /// Check if class inherits from a specific base class
    pub fn inherits_from(&self, base_class: &str) -> bool {
        self.bases.iter().any(|base| base == base_class)
    }

    /// Check if class is abstract based on methods or decorators
    pub fn is_abstract(&self) -> bool {
        self.decorators.iter().any(|d| d.contains("abstract")) ||
        self.methods.iter().any(|m| m.contains("NotImplementedError"))
    }

    /// Get total lines of code for the class
    pub fn lines_of_code(&self) -> i32 {
        self.line_end - self.line_start + 1
    }
}

/// Universal semantic code node for graph construction.
///
/// This struct exactly matches the Python SemanticNode dataclass
/// to ensure complete compatibility in semantic analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub node_id: String,
    pub node_type: String,  // function, class, import, variable, call
    pub name: String,
    pub file_path: String,
    pub line_start: i32,
    pub line_end: i32,
    pub scope_path: String,  // full qualified path
    pub relationships: HashMap<String, Vec<String>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SemanticNode {
    /// Create a new semantic node
    pub fn new(
        node_id: String,
        node_type: String,
        name: String,
        file_path: String,
        line_start: i32,
        line_end: i32,
        scope_path: String,
    ) -> Self {
        Self {
            node_id,
            node_type,
            name,
            file_path,
            line_start,
            line_end,
            scope_path,
            relationships: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a relationship to another node
    pub fn add_relationship(&mut self, relationship_type: String, target_node_id: String) {
        self.relationships
            .entry(relationship_type)
            .or_insert_with(Vec::new)
            .push(target_node_id);
    }

    /// Get relationships of a specific type
    pub fn get_relationships(&self, relationship_type: &str) -> Option<&Vec<String>> {
        self.relationships.get(relationship_type)
    }

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Check if node is a function
    pub fn is_function(&self) -> bool {
        self.node_type == "function"
    }

    /// Check if node is a class
    pub fn is_class(&self) -> bool {
        self.node_type == "class"
    }

    /// Get the module name from file path
    pub fn module_name(&self) -> String {
        // Convert file path to module-like name
        self.file_path
            .trim_end_matches(".py")
            .replace('/', ".")
            .replace('\\', ".")
    }
}

/// Collection of analysis graphs.
///
/// This struct represents the graph bundle used throughout CodeHUD
/// for storing various types of analysis graphs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphBundle {
    // Note: In Rust we'll use petgraph::Graph instead of NetworkX
    // but maintain the same logical structure
    pub call_graph_nodes: Vec<SemanticNode>,
    pub dependency_graph_nodes: Vec<SemanticNode>,
    pub inheritance_graph_nodes: Vec<SemanticNode>,
    pub module_graph_nodes: Vec<SemanticNode>,
    pub data_flow_graph_nodes: Vec<SemanticNode>,
    pub test_dependency_graph_nodes: Vec<SemanticNode>,
    
    pub creation_timestamp: DateTime<Utc>,
}

impl GraphBundle {
    /// Create a new empty graph bundle
    pub fn new() -> Self {
        Self {
            call_graph_nodes: Vec::new(),
            dependency_graph_nodes: Vec::new(),
            inheritance_graph_nodes: Vec::new(),
            module_graph_nodes: Vec::new(),
            data_flow_graph_nodes: Vec::new(),
            test_dependency_graph_nodes: Vec::new(),
            creation_timestamp: Utc::now(),
        }
    }

    /// Check if bundle is empty
    pub fn is_empty(&self) -> bool {
        self.call_graph_nodes.is_empty() &&
        self.dependency_graph_nodes.is_empty() &&
        self.inheritance_graph_nodes.is_empty() &&
        self.module_graph_nodes.is_empty() &&
        self.data_flow_graph_nodes.is_empty() &&
        self.test_dependency_graph_nodes.is_empty()
    }

    /// Get total number of nodes across all graphs
    pub fn total_nodes(&self) -> usize {
        self.call_graph_nodes.len() +
        self.dependency_graph_nodes.len() +
        self.inheritance_graph_nodes.len() +
        self.module_graph_nodes.len() +
        self.data_flow_graph_nodes.len() +
        self.test_dependency_graph_nodes.len()
    }
}

impl Default for GraphBundle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_signature_creation() {
        let func = FunctionSignature::new(
            "test_function".to_string(),
            vec!["arg1".to_string(), "arg2".to_string()],
            5,
            10,
            20,
        );
        
        assert_eq!(func.name, "test_function");
        assert_eq!(func.args.len(), 2);
        assert_eq!(func.complexity, 5);
        assert!(!func.has_side_effects());
    }

    #[test]
    fn test_function_signature_string() {
        let mut func = FunctionSignature::new(
            "example".to_string(),
            vec!["a".to_string(), "b".to_string()],
            1,
            1,
            5,
        );
        func.return_annotation = Some("int".to_string());
        
        assert_eq!(func.signature_string(), "example(a, b) -> int");
    }

    #[test]
    fn test_class_definition() {
        let class = ClassDefinition::new("TestClass".to_string(), 1, 50);
        assert_eq!(class.name, "TestClass");
        assert_eq!(class.lines_of_code(), 50);
        assert!(!class.is_abstract());
    }

    #[test]
    fn test_semantic_node() {
        let mut node = SemanticNode::new(
            "node1".to_string(),
            "function".to_string(),
            "test_func".to_string(),
            "src/test.py".to_string(),
            1,
            10,
            "src.test.test_func".to_string(),
        );
        
        node.add_relationship("calls".to_string(), "node2".to_string());
        
        assert!(node.is_function());
        assert!(!node.is_class());
        assert_eq!(node.get_relationships("calls").unwrap().len(), 1);
    }

    #[test]
    fn test_graph_bundle() {
        let bundle = GraphBundle::new();
        assert!(bundle.is_empty());
        assert_eq!(bundle.total_nodes(), 0);
    }

    #[test]
    fn test_serde_compatibility() {
        let func = FunctionSignature::new(
            "test".to_string(),
            vec!["arg".to_string()],
            1,
            1,
            5,
        );
        
        let json = serde_json::to_string(&func).unwrap();
        let deserialized: FunctionSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(func, deserialized);
    }
}