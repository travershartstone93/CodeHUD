//! Graph analysis utilities for call graph visualization
//!
//! Provides:
//! - Strongly Connected Component (SCC) detection for cycles
//! - Module extraction and clustering
//! - Importance scoring for filtering large graphs

use petgraph::graph::{Graph, NodeIndex};
use petgraph::Direction;
use petgraph::algo::tarjan_scc;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use codehud_core::graph::{CallNode, CallEdge};

/// Represents a module grouping of functions
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub functions: Vec<NodeIndex>,
    pub call_count: usize,
    pub is_public: bool,
}

/// Importance metrics for a function node
#[derive(Debug, Clone)]
pub struct ImportanceScore {
    pub node: NodeIndex,
    pub call_frequency: usize,
    pub betweenness: f64,
    pub is_entry_point: bool,
    pub is_public: bool,
    pub crosses_module_boundary: bool,
    pub total_score: f64,
}

/// Strongly Connected Component (cycle)
#[derive(Debug, Clone)]
pub struct StronglyConnectedComponent {
    pub nodes: Vec<NodeIndex>,
    pub size: usize,
}

/// Graph analysis results
pub struct GraphAnalysis {
    pub modules: Vec<Module>,
    pub sccs: Vec<StronglyConnectedComponent>,
    pub importance_scores: HashMap<NodeIndex, ImportanceScore>,
}

/// Extract module name from a fully qualified function name
///
/// Examples:
/// - `codehud_core::graph::mod::function` → `codehud_core::graph`
/// - `my_module::submodule::func` → `my_module::submodule`
/// - `standalone_func` → `<root>`
pub fn extract_module_name(function_name: &str) -> String {
    // Split on :: for Rust-style paths
    let parts: Vec<&str> = function_name.split("::").collect();

    if parts.len() <= 1 {
        // No module structure, use file-based or root
        if function_name.contains('/') || function_name.contains('\\') {
            // File path format (Python, JS, etc.)
            let path_parts: Vec<&str> = function_name.split(&['/', '\\'][..]).collect();
            if path_parts.len() > 1 {
                return path_parts[..path_parts.len() - 1].join("/");
            }
        }
        return "<root>".to_string();
    }

    // Take all but the last part (the function name)
    parts[..parts.len() - 1].join("::")
}

/// Detect all strongly connected components (cycles) in the graph
pub fn detect_cycles(graph: &Graph<CallNode, CallEdge, petgraph::Directed>) -> Vec<StronglyConnectedComponent> {
    let sccs = tarjan_scc(graph);

    sccs.into_iter()
        .filter(|scc| scc.len() > 1) // Only keep actual cycles (size > 1)
        .map(|nodes| StronglyConnectedComponent {
            size: nodes.len(),
            nodes,
        })
        .collect()
}

/// Group functions by their module
pub fn cluster_by_module(
    graph: &Graph<CallNode, CallEdge, petgraph::Directed>
) -> HashMap<String, Vec<NodeIndex>> {
    let mut modules: HashMap<String, Vec<NodeIndex>> = HashMap::new();

    for node_idx in graph.node_indices() {
        if let Some(node) = graph.node_weight(node_idx) {
            let module_name = extract_module_name(&node.function_name);
            modules.entry(module_name).or_insert_with(Vec::new).push(node_idx);
        }
    }

    modules
}

/// Calculate importance scores for all nodes
pub fn calculate_importance_scores(
    graph: &Graph<CallNode, CallEdge, petgraph::Directed>,
    modules: &HashMap<String, Vec<NodeIndex>>,
) -> HashMap<NodeIndex, ImportanceScore> {
    let mut scores = HashMap::new();

    // Build reverse lookup: node -> module
    let mut node_to_module: HashMap<NodeIndex, String> = HashMap::new();
    for (module_name, nodes) in modules {
        for &node in nodes {
            node_to_module.insert(node, module_name.clone());
        }
    }

    for node_idx in graph.node_indices() {
        let node = graph.node_weight(node_idx).unwrap();

        // 1. Call frequency - how many incoming edges
        let call_frequency = graph.neighbors_directed(node_idx, Direction::Incoming).count();

        // 2. Check if it's an entry point (no incoming calls)
        let is_entry_point = call_frequency == 0;

        // 3. Check if public (heuristic: exported, not starting with underscore)
        let is_public = !node.function_name.contains("::_") && !node.function_name.starts_with('_');

        // 4. Check if it crosses module boundaries
        let node_module = node_to_module.get(&node_idx);
        let crosses_boundary = graph
            .neighbors_directed(node_idx, Direction::Outgoing)
            .any(|neighbor| {
                node_to_module.get(&neighbor) != node_module
            });

        // 5. Betweenness centrality approximation
        // (simplified: nodes with both incoming and outgoing edges are connectors)
        let in_degree = graph.neighbors_directed(node_idx, Direction::Incoming).count();
        let out_degree = graph.neighbors_directed(node_idx, Direction::Outgoing).count();
        let betweenness = if in_degree > 0 && out_degree > 0 {
            (in_degree * out_degree) as f64
        } else {
            0.0
        };

        // Calculate total score
        let mut total_score = 0.0;
        total_score += call_frequency as f64 * 2.0;  // Weight: 2x
        total_score += betweenness * 1.5;             // Weight: 1.5x
        if is_entry_point { total_score += 10.0; }   // Entry points are important
        if is_public { total_score += 5.0; }         // Public APIs matter
        if crosses_boundary { total_score += 8.0; }  // Module boundaries critical

        scores.insert(node_idx, ImportanceScore {
            node: node_idx,
            call_frequency,
            betweenness,
            is_entry_point,
            is_public,
            crosses_module_boundary: crosses_boundary,
            total_score,
        });
    }

    scores
}

/// Build a module-level graph (nodes are modules, not functions)
pub fn build_module_graph(
    graph: &Graph<CallNode, CallEdge, petgraph::Directed>,
    modules: &HashMap<String, Vec<NodeIndex>>,
) -> (Graph<String, usize, petgraph::Directed>, HashMap<String, NodeIndex>) {
    let mut module_graph = Graph::new();
    let mut module_to_node = HashMap::new();

    // Create module nodes
    for module_name in modules.keys() {
        let node_idx = module_graph.add_node(module_name.clone());
        module_to_node.insert(module_name.clone(), node_idx);
    }

    // Build reverse lookup
    let mut node_to_module: HashMap<NodeIndex, String> = HashMap::new();
    for (module_name, nodes) in modules {
        for &node in nodes {
            node_to_module.insert(node, module_name.clone());
        }
    }

    // Track inter-module edges and their weights
    let mut edge_weights: HashMap<(String, String), usize> = HashMap::new();

    for edge in graph.edge_references() {
        let source = edge.source();
        let target = edge.target();

        if let (Some(src_module), Some(tgt_module)) =
            (node_to_module.get(&source), node_to_module.get(&target)) {

            // Only count edges between different modules
            if src_module != tgt_module {
                let key = (src_module.clone(), tgt_module.clone());
                *edge_weights.entry(key).or_insert(0) += 1;
            }
        }
    }

    // Add weighted edges to module graph
    for ((src_module, tgt_module), weight) in edge_weights {
        if let (Some(&src_idx), Some(&tgt_idx)) =
            (module_to_node.get(&src_module), module_to_node.get(&tgt_module)) {
            module_graph.add_edge(src_idx, tgt_idx, weight);
        }
    }

    (module_graph, module_to_node)
}

/// Extract subgraph containing only nodes in a specific module
pub fn extract_module_subgraph(
    graph: &Graph<CallNode, CallEdge, petgraph::Directed>,
    module_nodes: &[NodeIndex],
) -> Graph<CallNode, CallEdge, petgraph::Directed> {
    let node_set: HashSet<NodeIndex> = module_nodes.iter().copied().collect();
    let mut subgraph = Graph::new();
    let mut old_to_new: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    // Add nodes
    for &old_idx in module_nodes {
        if let Some(node) = graph.node_weight(old_idx) {
            let new_idx = subgraph.add_node(node.clone());
            old_to_new.insert(old_idx, new_idx);
        }
    }

    // Add edges (only within this module)
    for &old_src in module_nodes {
        for edge in graph.edges(old_src) {
            let old_tgt = edge.target();
            if node_set.contains(&old_tgt) {
                if let (Some(&new_src), Some(&new_tgt)) =
                    (old_to_new.get(&old_src), old_to_new.get(&old_tgt)) {
                    subgraph.add_edge(new_src, new_tgt, edge.weight().clone());
                }
            }
        }
    }

    subgraph
}

/// Extract subgraph containing only nodes involved in cycles
pub fn extract_cycle_subgraph(
    graph: &Graph<CallNode, CallEdge, petgraph::Directed>,
    sccs: &[StronglyConnectedComponent],
) -> Graph<CallNode, CallEdge, petgraph::Directed> {
    let mut cycle_nodes = HashSet::new();
    for scc in sccs {
        for &node in &scc.nodes {
            cycle_nodes.insert(node);
        }
    }

    let cycle_vec: Vec<NodeIndex> = cycle_nodes.into_iter().collect();
    extract_module_subgraph(graph, &cycle_vec)
}

/// Perform complete graph analysis
pub fn analyze_graph(
    graph: &Graph<CallNode, CallEdge, petgraph::Directed>
) -> GraphAnalysis {
    let sccs = detect_cycles(graph);
    let module_map = cluster_by_module(graph);
    let importance_scores = calculate_importance_scores(graph, &module_map);

    // Convert module map to Module structs
    let modules = module_map
        .into_iter()
        .map(|(name, functions)| {
            let call_count = functions.iter()
                .map(|&node| graph.neighbors_directed(node, Direction::Incoming).count())
                .sum();

            let is_public = !name.starts_with('_') && name != "<root>";

            Module {
                name,
                functions,
                call_count,
                is_public,
            }
        })
        .collect();

    GraphAnalysis {
        modules,
        sccs,
        importance_scores,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_name() {
        assert_eq!(extract_module_name("codehud_core::graph::mod::func"), "codehud_core::graph::mod");
        assert_eq!(extract_module_name("std::collections::HashMap::new"), "std::collections::HashMap");
        assert_eq!(extract_module_name("standalone"), "<root>");
        assert_eq!(extract_module_name("src/lib/utils.py"), "src/lib");
    }
}
