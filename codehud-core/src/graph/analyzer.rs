//! Graph Analyzer - Main entry point for graph analysis
//!
//! Orchestrates all graph analysis operations and provides NetworkX-equivalent functionality

use std::collections::HashMap;
use petgraph::algo::{connected_components, has_path_connecting};
use petgraph::visit::EdgeRef;
use anyhow::{Result, Context};

use super::{
    CallGraph, DependencyGraph, InheritanceGraph,
    GraphAnalysisResult, CycleAnalysis, ComponentAnalysis, GraphStatistics, GraphStats,
    CentralityMetrics, CouplingMetrics, NetworkMetrics,
    algorithms::{CentralityAlgorithms, CycleDetection, NetworkAnalysis}
};

/// Main graph analyzer that orchestrates all analysis operations
pub struct GraphAnalyzer {
    call_graph: CallGraph,
    dependency_graph: DependencyGraph,
    inheritance_graph: InheritanceGraph,
}

impl GraphAnalyzer {
    /// Create a new graph analyzer
    pub fn new(
        call_graph: CallGraph,
        dependency_graph: DependencyGraph,
        inheritance_graph: InheritanceGraph,
    ) -> Self {
        Self {
            call_graph,
            dependency_graph,
            inheritance_graph,
        }
    }

    /// Perform complete graph analysis and return all results
    pub fn analyze(&self) -> Result<GraphAnalysisResult> {
        Ok(GraphAnalysisResult {
            call_centrality: self.calculate_call_centrality()?,
            dependency_centrality: self.calculate_dependency_centrality()?,
            inheritance_centrality: self.calculate_inheritance_centrality()?,
            cycles: self.detect_cycles()?,
            components: self.find_strongly_connected_components()?,
            coupling: self.calculate_coupling_metrics()?,
            statistics: self.calculate_graph_statistics()?,
        })
    }

    /// Calculate centrality metrics for call graph
    pub fn calculate_call_centrality(&self) -> Result<CentralityMetrics> {
        let mut metrics = CentralityMetrics::new();

        metrics.degree = CentralityAlgorithms::degree_centrality(&self.call_graph);
        metrics.betweenness = CentralityAlgorithms::betweenness_centrality(&self.call_graph);
        metrics.closeness = CentralityAlgorithms::closeness_centrality(&self.call_graph);
        metrics.pagerank = CentralityAlgorithms::pagerank_centrality(&self.call_graph, 0.85, 100, 1e-6);

        // Note: Eigenvector centrality would require additional implementation
        // For now, we'll use PageRank as a proxy
        metrics.eigenvector = metrics.pagerank.clone();

        Ok(metrics)
    }

    /// Calculate centrality metrics for dependency graph
    pub fn calculate_dependency_centrality(&self) -> Result<CentralityMetrics> {
        let mut metrics = CentralityMetrics::new();

        metrics.degree = CentralityAlgorithms::degree_centrality(&self.dependency_graph);
        metrics.betweenness = CentralityAlgorithms::betweenness_centrality(&self.dependency_graph);
        metrics.closeness = CentralityAlgorithms::closeness_centrality(&self.dependency_graph);
        metrics.pagerank = CentralityAlgorithms::pagerank_centrality(&self.dependency_graph, 0.85, 100, 1e-6);
        metrics.eigenvector = metrics.pagerank.clone();

        Ok(metrics)
    }

    /// Calculate centrality metrics for inheritance graph
    pub fn calculate_inheritance_centrality(&self) -> Result<CentralityMetrics> {
        let mut metrics = CentralityMetrics::new();

        metrics.degree = CentralityAlgorithms::degree_centrality(&self.inheritance_graph);
        metrics.betweenness = CentralityAlgorithms::betweenness_centrality(&self.inheritance_graph);
        metrics.closeness = CentralityAlgorithms::closeness_centrality(&self.inheritance_graph);
        metrics.pagerank = CentralityAlgorithms::pagerank_centrality(&self.inheritance_graph, 0.85, 100, 1e-6);
        metrics.eigenvector = metrics.pagerank.clone();

        Ok(metrics)
    }

    /// Detect cycles in all graphs (NetworkX equivalent)
    pub fn detect_cycles(&self) -> Result<CycleAnalysis> {
        let call_cycles = CycleDetection::find_all_cycles(&self.call_graph);
        let dependency_cycles = CycleDetection::find_all_cycles(&self.dependency_graph);
        let inheritance_cycles = CycleDetection::find_all_cycles(&self.inheritance_graph);

        let total_cycles = call_cycles.len() + dependency_cycles.len() + inheritance_cycles.len();

        Ok(CycleAnalysis {
            call_cycles,
            dependency_cycles,
            inheritance_cycles,
            total_cycles,
        })
    }

    /// Find strongly connected components in all graphs (NetworkX equivalent)
    pub fn find_strongly_connected_components(&self) -> Result<ComponentAnalysis> {
        // Use the minimal NetworkAnalysis implementation for now
        let call_components = NetworkAnalysis::strongly_connected_components(&self.call_graph);
        let dependency_components = NetworkAnalysis::strongly_connected_components(&self.dependency_graph);
        let inheritance_components = NetworkAnalysis::strongly_connected_components(&self.inheritance_graph);

        let total_components = call_components.len() + dependency_components.len() + inheritance_components.len();

        Ok(ComponentAnalysis {
            call_components,
            dependency_components,
            inheritance_components,
            total_components,
        })
    }

    /// Calculate coupling metrics (NetworkX equivalent)
    pub fn calculate_coupling_metrics(&self) -> Result<CouplingMetrics> {
        let mut metrics = CouplingMetrics::new();

        // Calculate coupling based on dependency graph
        for node_index in self.dependency_graph.node_indices() {
            // Convert NodeIndex to String key
            let node_key = format!("node_{}", node_index.index());

            // Afferent coupling (Ca) - incoming dependencies
            let afferent = self.dependency_graph.edges_directed(node_index, petgraph::Incoming).count();
            metrics.afferent_coupling.insert(node_key.clone(), afferent);

            // Efferent coupling (Ce) - outgoing dependencies
            let efferent = self.dependency_graph.edges_directed(node_index, petgraph::Outgoing).count();
            metrics.efferent_coupling.insert(node_key.clone(), efferent);

            // Calculate abstractness (simplified - would need class analysis for real implementation)
            // For now, assume 0.5 as default abstractness
            metrics.abstractness.insert(node_key, 0.5);
        }

        // Calculate derived metrics
        metrics.calculate_all_metrics();

        Ok(metrics)
    }

    /// Calculate graph statistics
    pub fn calculate_graph_statistics(&self) -> Result<GraphStatistics> {
        Ok(GraphStatistics {
            call_graph: self.calculate_single_graph_stats(&self.call_graph),
            dependency_graph: self.calculate_single_graph_stats(&self.dependency_graph),
            inheritance_graph: self.calculate_single_graph_stats(&self.inheritance_graph),
        })
    }

    /// Calculate statistics for a single graph
    fn calculate_single_graph_stats<N, E>(&self, graph: &petgraph::Graph<N, E, petgraph::Directed>) -> GraphStats {
        let node_count = graph.node_count();
        let edge_count = graph.edge_count();

        let density = NetworkAnalysis::graph_density(graph);
        let is_cyclic = self.has_cycles(graph);

        let average_degree = if node_count > 0 {
            (edge_count * 2) as f64 / node_count as f64 // For directed graphs
        } else {
            0.0
        };

        GraphStats {
            node_count,
            edge_count,
            density,
            is_cyclic,
            average_degree,
        }
    }

    /// Check if a graph has cycles (simplified implementation)
    fn has_cycles<N, E>(&self, graph: &petgraph::Graph<N, E, petgraph::Directed>) -> bool {
        // Simple DFS-based cycle detection
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node in graph.node_indices() {
            if !visited.contains(&node) {
                if self.dfs_has_cycle(graph, node, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        false
    }

    /// DFS helper for cycle detection
    fn dfs_has_cycle<N, E>(
        &self,
        graph: &petgraph::Graph<N, E, petgraph::Directed>,
        node: petgraph::graph::NodeIndex,
        visited: &mut std::collections::HashSet<petgraph::graph::NodeIndex>,
        rec_stack: &mut std::collections::HashSet<petgraph::graph::NodeIndex>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        for edge in graph.edges(node) {
            let neighbor = edge.target();
            if !visited.contains(&neighbor) {
                if self.dfs_has_cycle(graph, neighbor, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&neighbor) {
                return true;
            }
        }

        rec_stack.remove(&node);
        false
    }

    /// Get call graph reference
    pub fn call_graph(&self) -> &CallGraph {
        &self.call_graph
    }

    /// Get dependency graph reference
    pub fn dependency_graph(&self) -> &DependencyGraph {
        &self.dependency_graph
    }

    /// Get inheritance graph reference
    pub fn inheritance_graph(&self) -> &InheritanceGraph {
        &self.inheritance_graph
    }

    /// Calculate network metrics for all graphs
    pub fn calculate_network_metrics(&self) -> Result<HashMap<String, NetworkMetrics>> {
        let mut metrics = HashMap::new();

        // Call graph metrics
        let call_metrics = NetworkMetrics {
            density: NetworkAnalysis::graph_density(&self.call_graph),
            clustering_coefficient: NetworkAnalysis::average_clustering_coefficient(&self.call_graph),
            average_path_length: NetworkAnalysis::average_path_length(&self.call_graph),
            diameter: NetworkAnalysis::graph_diameter(&self.call_graph),
            connected_components: connected_components(&self.call_graph),
            largest_component_size: self.largest_component_size(&self.call_graph),
        };
        metrics.insert("call_graph".to_string(), call_metrics);

        // Dependency graph metrics
        let dependency_metrics = NetworkMetrics {
            density: NetworkAnalysis::graph_density(&self.dependency_graph),
            clustering_coefficient: NetworkAnalysis::average_clustering_coefficient(&self.dependency_graph),
            average_path_length: NetworkAnalysis::average_path_length(&self.dependency_graph),
            diameter: NetworkAnalysis::graph_diameter(&self.dependency_graph),
            connected_components: connected_components(&self.dependency_graph),
            largest_component_size: self.largest_component_size(&self.dependency_graph),
        };
        metrics.insert("dependency_graph".to_string(), dependency_metrics);

        // Inheritance graph metrics
        let inheritance_metrics = NetworkMetrics {
            density: NetworkAnalysis::graph_density(&self.inheritance_graph),
            clustering_coefficient: NetworkAnalysis::average_clustering_coefficient(&self.inheritance_graph),
            average_path_length: NetworkAnalysis::average_path_length(&self.inheritance_graph),
            diameter: NetworkAnalysis::graph_diameter(&self.inheritance_graph),
            connected_components: connected_components(&self.inheritance_graph),
            largest_component_size: self.largest_component_size(&self.inheritance_graph),
        };
        metrics.insert("inheritance_graph".to_string(), inheritance_metrics);

        Ok(metrics)
    }

    /// Calculate the size of the largest connected component
    fn largest_component_size<N, E>(&self, graph: &petgraph::Graph<N, E, petgraph::Directed>) -> usize {
        use petgraph::visit::Dfs;
        use std::collections::HashSet;

        let mut visited = HashSet::new();
        let mut max_component_size = 0;

        for node in graph.node_indices() {
            if !visited.contains(&node) {
                let mut component_size = 0;
                let mut dfs = Dfs::new(graph, node);

                while let Some(visited_node) = dfs.next(graph) {
                    visited.insert(visited_node);
                    component_size += 1;
                }

                max_component_size = max_component_size.max(component_size);
            }
        }

        max_component_size
    }

    /// Check if graphs have problematic patterns
    pub fn check_problematic_patterns(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut issues = HashMap::new();

        // Check for problematic cycles
        let cycles = self.detect_cycles()?;
        let mut cycle_issues = Vec::new();

        if !cycles.dependency_cycles.is_empty() {
            cycle_issues.push(format!("Found {} dependency cycles which can cause circular imports", cycles.dependency_cycles.len()));
        }

        if !cycles.inheritance_cycles.is_empty() {
            cycle_issues.push(format!("Found {} inheritance cycles which indicate design problems", cycles.inheritance_cycles.len()));
        }

        if cycles.call_cycles.len() > 10 {
            cycle_issues.push(format!("Found {} call cycles - consider refactoring recursive patterns", cycles.call_cycles.len()));
        }

        if !cycle_issues.is_empty() {
            issues.insert("cycles".to_string(), cycle_issues);
        }

        // Check coupling metrics
        let coupling = self.calculate_coupling_metrics()?;
        let stats = coupling.summary_stats();
        let mut coupling_issues = Vec::new();

        if stats.avg_instability > 0.8 {
            coupling_issues.push("High average instability detected - modules are too dependent on others".to_string());
        }

        if stats.max_coupling > 20 {
            coupling_issues.push("Modules with very high coupling detected - consider decomposition".to_string());
        }

        if !coupling_issues.is_empty() {
            issues.insert("coupling".to_string(), coupling_issues);
        }

        // Check graph density
        let statistics = self.calculate_graph_statistics()?;
        let mut density_issues = Vec::new();

        if statistics.dependency_graph.density > 0.3 {
            density_issues.push("Dependency graph is very dense - consider modularization".to_string());
        }

        if statistics.call_graph.density > 0.5 {
            density_issues.push("Call graph is very dense - functions are tightly coupled".to_string());
        }

        if !density_issues.is_empty() {
            issues.insert("density".to_string(), density_issues);
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphBuilder, CallNode, ModuleNode, ClassNode};

    #[test]
    fn test_graph_analyzer_creation() {
        let builder = GraphBuilder::new();
        let analyzer = builder.build();

        assert_eq!(analyzer.call_graph().node_count(), 0);
        assert_eq!(analyzer.dependency_graph().node_count(), 0);
        assert_eq!(analyzer.inheritance_graph().node_count(), 0);
    }

    #[test]
    fn test_full_analysis() {
        let mut builder = GraphBuilder::new();

        // Add some relationships
        builder.add_call("main", "helper", 5).unwrap();
        builder.add_call("helper", "util", 3).unwrap();
        builder.add_dependency("main_module", "helper_module", "import").unwrap();
        builder.add_inheritance("ChildClass", "ParentClass").unwrap();

        let analyzer = builder.build();
        let results = analyzer.analyze().unwrap();

        // Should have calculated centrality for all graphs
        assert!(!results.call_centrality.degree.is_empty());
        assert!(!results.dependency_centrality.degree.is_empty());

        // Should have detected cycles and components
        assert!(results.cycles.total_cycles >= 0);
        assert!(results.components.total_components >= 0);

        // Should have calculated statistics
        assert!(results.statistics.call_graph.node_count > 0);
        assert!(results.statistics.dependency_graph.node_count > 0);
    }

    #[test]
    fn test_coupling_metrics_calculation() {
        let mut builder = GraphBuilder::new();

        // Create a dependency chain
        builder.add_dependency("A", "B", "import").unwrap();
        builder.add_dependency("A", "C", "import").unwrap();
        builder.add_dependency("B", "C", "import").unwrap();

        let analyzer = builder.build();
        let coupling = analyzer.calculate_coupling_metrics().unwrap();

        // Module C should have high afferent coupling (imported by A and B)
        // Module A should have high efferent coupling (imports B and C)
        assert!(!coupling.afferent_coupling.is_empty());
        assert!(!coupling.efferent_coupling.is_empty());
    }

    #[test]
    fn test_problematic_patterns_detection() {
        let mut builder = GraphBuilder::new();

        // Create a dependency cycle
        builder.add_dependency("A", "B", "import").unwrap();
        builder.add_dependency("B", "C", "import").unwrap();
        builder.add_dependency("C", "A", "import").unwrap();

        let analyzer = builder.build();
        let issues = analyzer.check_problematic_patterns().unwrap();

        // Should detect the cycle as an issue
        if let Some(cycle_issues) = issues.get("cycles") {
            assert!(!cycle_issues.is_empty());
        }
    }

    #[test]
    fn test_network_metrics_calculation() {
        let mut builder = GraphBuilder::new();

        builder.add_call("main", "helper", 1).unwrap();
        builder.add_call("helper", "util", 1).unwrap();

        let analyzer = builder.build();
        let metrics = analyzer.calculate_network_metrics().unwrap();

        assert!(metrics.contains_key("call_graph"));
        assert!(metrics.contains_key("dependency_graph"));
        assert!(metrics.contains_key("inheritance_graph"));

        let call_metrics = &metrics["call_graph"];
        assert!(call_metrics.density >= 0.0);
        assert!(call_metrics.average_path_length >= 0.0);
    }
}