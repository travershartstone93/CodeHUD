//! Graph Analysis Engine - NetworkX equivalent for CodeHUD
//!
//! Provides zero-degradation compatibility with Python NetworkX for:
//! - Call graph analysis
//! - Dependency graph analysis
//! - Inheritance graph analysis
//! - Centrality calculations
//! - Cycle detection
//! - Strongly connected components
//! - Coupling metrics

use std::collections::{HashMap, HashSet};
use petgraph::{Graph, Directed};
use petgraph::graph::{NodeIndex, EdgeIndex};
use petgraph::algo::{connected_components, has_path_connecting};
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

pub mod analyzer;
pub mod nodes;
pub mod edges;
pub mod algorithms_minimal;
pub mod metrics;

// Re-export minimal algorithms as algorithms for compatibility
pub use algorithms_minimal as algorithms;

pub use analyzer::GraphAnalyzer;
pub use nodes::{CallNode, ModuleNode, ClassNode};
pub use edges::{CallEdge, DependencyEdge, InheritanceEdge};
pub use metrics::{CouplingMetrics, CentralityMetrics, NetworkMetrics};

/// Type aliases for different graph types
pub type CallGraph = Graph<CallNode, CallEdge, Directed>;
pub type DependencyGraph = Graph<ModuleNode, DependencyEdge, Directed>;
pub type InheritanceGraph = Graph<ClassNode, InheritanceEdge, Directed>;

/// Node identifier type - using string for serialization compatibility
pub type GraphNodeId = String;

/// Edge identifier type
pub type GraphEdgeId = EdgeIndex;

/// Graph analysis results combining all metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAnalysisResult {
    /// Call graph centrality metrics
    pub call_centrality: CentralityMetrics,
    /// Dependency graph centrality metrics
    pub dependency_centrality: CentralityMetrics,
    /// Inheritance graph centrality metrics
    pub inheritance_centrality: CentralityMetrics,
    /// Detected cycles in all graphs
    pub cycles: CycleAnalysis,
    /// Strongly connected components
    pub components: ComponentAnalysis,
    /// Coupling metrics for the codebase
    pub coupling: CouplingMetrics,
    /// Graph statistics
    pub statistics: GraphStatistics,
}

/// Cycle detection results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleAnalysis {
    /// Cycles in call graph (node names)
    pub call_cycles: Vec<Vec<String>>,
    /// Cycles in dependency graph (node names)
    pub dependency_cycles: Vec<Vec<String>>,
    /// Cycles in inheritance graph (node names)
    pub inheritance_cycles: Vec<Vec<String>>,
    /// Total cycle count
    pub total_cycles: usize,
}

/// Strongly connected components analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentAnalysis {
    /// Components in call graph (node names)
    pub call_components: Vec<Vec<String>>,
    /// Components in dependency graph (node names)
    pub dependency_components: Vec<Vec<String>>,
    /// Components in inheritance graph (node names)
    pub inheritance_components: Vec<Vec<String>>,
    /// Total component count
    pub total_components: usize,
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    /// Call graph stats
    pub call_graph: GraphStats,
    /// Dependency graph stats
    pub dependency_graph: GraphStats,
    /// Inheritance graph stats
    pub inheritance_graph: GraphStats,
}

/// Individual graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    /// Number of nodes
    pub node_count: usize,
    /// Number of edges
    pub edge_count: usize,
    /// Graph density (edges / max_possible_edges)
    pub density: f64,
    /// Whether the graph is cyclic
    pub is_cyclic: bool,
    /// Average degree
    pub average_degree: f64,
}

/// Graph builder for constructing graphs from extracted data
pub struct GraphBuilder {
    call_graph: CallGraph,
    dependency_graph: DependencyGraph,
    inheritance_graph: InheritanceGraph,
    node_mappings: NodeMappings,
}

/// Mappings between string identifiers and graph node indices
#[derive(Debug, Default)]
struct NodeMappings {
    /// Function name -> call graph node index
    call_nodes: HashMap<String, NodeIndex>,
    /// Module name -> dependency graph node index
    dependency_nodes: HashMap<String, NodeIndex>,
    /// Class name -> inheritance graph node index
    inheritance_nodes: HashMap<String, NodeIndex>,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            call_graph: CallGraph::new(),
            dependency_graph: DependencyGraph::new(),
            inheritance_graph: InheritanceGraph::new(),
            node_mappings: NodeMappings::default(),
        }
    }

    /// Add a function call relationship
    pub fn add_call(&mut self, caller: &str, callee: &str, call_count: usize) -> Result<()> {
        let caller_node = self.get_or_create_call_node(caller);
        let callee_node = self.get_or_create_call_node(callee);

        let edge = CallEdge {
            call_count,
            weight: call_count as f64,
        };

        self.call_graph.add_edge(caller_node, callee_node, edge);
        Ok(())
    }

    /// Add a module dependency relationship
    pub fn add_dependency(&mut self, importer: &str, imported: &str, import_type: &str) -> Result<()> {
        let importer_node = self.get_or_create_dependency_node(importer);
        let imported_node = self.get_or_create_dependency_node(imported);

        let edge = DependencyEdge {
            import_type: import_type.to_string(),
            weight: 1.0,
        };

        self.dependency_graph.add_edge(importer_node, imported_node, edge);
        Ok(())
    }

    /// Add a class inheritance relationship
    pub fn add_inheritance(&mut self, child: &str, parent: &str) -> Result<()> {
        let child_node = self.get_or_create_inheritance_node(child);
        let parent_node = self.get_or_create_inheritance_node(parent);

        let edge = InheritanceEdge {
            inheritance_type: "extends".to_string(),
            weight: 1.0,
        };

        self.inheritance_graph.add_edge(child_node, parent_node, edge);
        Ok(())
    }

    /// Build the final graph analyzer
    pub fn build(self) -> GraphAnalyzer {
        GraphAnalyzer::new(
            self.call_graph,
            self.dependency_graph,
            self.inheritance_graph,
        )
    }

    /// Get or create a call graph node
    fn get_or_create_call_node(&mut self, function_name: &str) -> NodeIndex {
        if let Some(&node_id) = self.node_mappings.call_nodes.get(function_name) {
            node_id
        } else {
            let node = CallNode {
                function_name: function_name.to_string(),
                file_path: String::new(), // Will be populated later
                line_number: 0,
            };
            let node_id = self.call_graph.add_node(node);
            self.node_mappings.call_nodes.insert(function_name.to_string(), node_id);
            node_id
        }
    }

    /// Get or create a dependency graph node
    fn get_or_create_dependency_node(&mut self, module_name: &str) -> NodeIndex {
        if let Some(&node_id) = self.node_mappings.dependency_nodes.get(module_name) {
            node_id
        } else {
            let node = ModuleNode {
                module_name: module_name.to_string(),
                file_path: String::new(),
                is_external: false,
            };
            let node_id = self.dependency_graph.add_node(node);
            self.node_mappings.dependency_nodes.insert(module_name.to_string(), node_id);
            node_id
        }
    }

    /// Get or create an inheritance graph node
    fn get_or_create_inheritance_node(&mut self, class_name: &str) -> NodeIndex {
        if let Some(&node_id) = self.node_mappings.inheritance_nodes.get(class_name) {
            node_id
        } else {
            let node = ClassNode {
                class_name: class_name.to_string(),
                file_path: String::new(),
                line_number: 0,
            };
            let node_id = self.inheritance_graph.add_node(node);
            self.node_mappings.inheritance_nodes.insert(class_name.to_string(), node_id);
            node_id
        }
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_builder_creation() {
        let builder = GraphBuilder::new();
        assert_eq!(builder.call_graph.node_count(), 0);
        assert_eq!(builder.dependency_graph.node_count(), 0);
        assert_eq!(builder.inheritance_graph.node_count(), 0);
    }

    #[test]
    fn test_add_call_relationship() {
        let mut builder = GraphBuilder::new();

        builder.add_call("main", "helper", 5).unwrap();

        assert_eq!(builder.call_graph.node_count(), 2);
        assert_eq!(builder.call_graph.edge_count(), 1);
    }

    #[test]
    fn test_add_dependency_relationship() {
        let mut builder = GraphBuilder::new();

        builder.add_dependency("module_a", "module_b", "import").unwrap();

        assert_eq!(builder.dependency_graph.node_count(), 2);
        assert_eq!(builder.dependency_graph.edge_count(), 1);
    }

    #[test]
    fn test_add_inheritance_relationship() {
        let mut builder = GraphBuilder::new();

        builder.add_inheritance("ChildClass", "ParentClass").unwrap();

        assert_eq!(builder.inheritance_graph.node_count(), 2);
        assert_eq!(builder.inheritance_graph.edge_count(), 1);
    }

    #[test]
    fn test_duplicate_nodes() {
        let mut builder = GraphBuilder::new();

        // Add same call relationship twice
        builder.add_call("main", "helper", 3).unwrap();
        builder.add_call("main", "helper", 2).unwrap();

        // Should have 2 nodes but 2 edges
        assert_eq!(builder.call_graph.node_count(), 2);
        assert_eq!(builder.call_graph.edge_count(), 2);
    }
}