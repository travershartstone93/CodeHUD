//! Graph algorithms for CodeHUD analysis
//!
//! NetworkX-equivalent implementations of graph algorithms:
//! - Centrality calculations (betweenness, closeness, degree, eigenvector, PageRank)
//! - Cycle detection
//! - Strongly connected components
//! - Shortest paths
//! - Clustering coefficients

use std::collections::{HashMap, HashSet, VecDeque};
use petgraph::{Graph, Directed};
use petgraph::graph::{NodeIndex, EdgeIndex};
use petgraph::algo::{connected_components, has_path_connecting, dijkstra};
use petgraph::visit::{EdgeRef, IntoNodeReferences, IntoEdgeReferences};
use super::{GraphNodeId, CentralityMetrics, NetworkMetrics};

/// Centrality algorithm implementations
pub struct CentralityAlgorithms;

impl CentralityAlgorithms {
    /// Calculate degree centrality for all nodes
    /// Degree centrality = degree(node) / (n - 1) where n is total nodes
    pub fn degree_centrality<N, E>(graph: &Graph<N, E, Directed>) -> HashMap<GraphNodeId, f64> {
        let mut centrality = HashMap::new();
        let node_count = graph.node_count();

        if node_count <= 1 {
            return centrality;
        }

        let max_degree = (node_count - 1) as f64;

        for node_index in graph.node_indices() {
            let in_degree = graph.edges_directed(node_index, petgraph::Incoming).count();
            let out_degree = graph.edges_directed(node_index, petgraph::Outgoing).count();
            let total_degree = (in_degree + out_degree) as f64;

            centrality.insert(node_index, total_degree / max_degree);
        }

        centrality
    }

    /// Calculate betweenness centrality using Brandes' algorithm
    /// Measures how often a node appears on shortest paths between other nodes
    pub fn betweenness_centrality<N, E>(graph: &Graph<N, E, Directed>) -> HashMap<GraphNodeId, f64> {
        let mut centrality = HashMap::new();
        let nodes: Vec<_> = graph.node_indices().collect();

        // Initialize centrality to 0 for all nodes
        for &node in &nodes {
            centrality.insert(node, 0.0);
        }

        // For each node, calculate shortest paths to all other nodes
        for &source in &nodes {
            let mut stack = Vec::new();
            let mut paths = HashMap::new();
            let mut sigma = HashMap::new();
            let mut distance = HashMap::new();
            let mut delta = HashMap::new();

            // Initialize
            for &node in &nodes {
                paths.insert(node, Vec::new());
                sigma.insert(node, 0.0);
                distance.insert(node, -1.0);
                delta.insert(node, 0.0);
            }

            sigma.insert(source, 1.0);
            distance.insert(source, 0.0);

            let mut queue = VecDeque::new();
            queue.push_back(source);

            // BFS to find shortest paths
            while let Some(current) = queue.pop_front() {
                stack.push(current);

                for edge in graph.edges(current) {
                    let neighbor = edge.target();

                    // First time we see this node
                    if distance[&neighbor] < 0.0 {
                        queue.push_back(neighbor);
                        distance.insert(neighbor, distance[&current] + 1.0);
                    }

                    // Shortest path to neighbor via current
                    if distance[&neighbor] == distance[&current] + 1.0 {
                        *sigma.get_mut(&neighbor).unwrap() += sigma[&current];
                        paths.get_mut(&neighbor).unwrap().push(current);
                    }
                }
            }

            // Accumulation - back-propagate dependencies
            while let Some(node) = stack.pop() {
                for &predecessor in &paths[&node] {
                    let contribution = (sigma[&predecessor] / sigma[&node]) * (1.0 + delta[&node]);
                    *delta.get_mut(&predecessor).unwrap() += contribution;
                }

                if node != source {
                    *centrality.get_mut(&node).unwrap() += delta[&node];
                }
            }
        }

        // Normalize for directed graphs
        let n = nodes.len() as f64;
        let normalization = if n > 2.0 { (n - 1.0) * (n - 2.0) } else { 1.0 };

        for centrality_value in centrality.values_mut() {
            *centrality_value /= normalization;
        }

        centrality
    }

    /// Calculate closeness centrality
    /// Closeness centrality = (n-1) / sum(shortest_path_lengths)
    pub fn closeness_centrality<N, E>(graph: &Graph<N, E, Directed>) -> HashMap<GraphNodeId, f64> {
        let mut centrality = HashMap::new();
        let nodes: Vec<_> = graph.node_indices().collect();

        for &source in &nodes {
            let distances = Self::single_source_shortest_path_length(graph, source);
            let reachable_nodes = distances.len() as f64;

            if reachable_nodes > 1.0 {
                let total_distance: f64 = distances.values().sum();
                let closeness = (reachable_nodes - 1.0) / total_distance;
                centrality.insert(source, closeness);
            } else {
                centrality.insert(source, 0.0);
            }
        }

        centrality
    }

    /// Calculate PageRank centrality using power iteration
    pub fn pagerank_centrality<N, E>(
        graph: &Graph<N, E, Directed>,
        damping: f64,
        max_iterations: usize,
        tolerance: f64
    ) -> HashMap<GraphNodeId, f64> {
        let mut pagerank = HashMap::new();
        let nodes: Vec<_> = graph.node_indices().collect();
        let node_count = nodes.len() as f64;

        if nodes.is_empty() {
            return pagerank;
        }

        // Initialize PageRank values
        let initial_value = 1.0 / node_count;
        for &node in &nodes {
            pagerank.insert(node, initial_value);
        }

        // Power iteration
        for _ in 0..max_iterations {
            let mut new_pagerank = HashMap::new();

            for &node in &nodes {
                let mut rank = (1.0 - damping) / node_count;

                // Sum contributions from incoming edges
                for edge in graph.edges_directed(node, petgraph::Incoming) {
                    let source = edge.source();
                    let out_degree = graph.edges_directed(source, petgraph::Outgoing).count() as f64;

                    if out_degree > 0.0 {
                        rank += damping * pagerank[&source] / out_degree;
                    }
                }

                new_pagerank.insert(node, rank);
            }

            // Check for convergence
            let mut converged = true;
            for &node in &nodes {
                if (new_pagerank[&node] - pagerank[&node]).abs() > tolerance {
                    converged = false;
                    break;
                }
            }

            pagerank = new_pagerank;

            if converged {
                break;
            }
        }

        pagerank
    }

    /// Calculate single-source shortest path lengths using BFS
    fn single_source_shortest_path_length<N, E>(
        graph: &Graph<N, E, Directed>,
        source: GraphNodeId
    ) -> HashMap<GraphNodeId, f64> {
        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();

        distances.insert(source, 0.0);
        queue.push_back(source);

        while let Some(current) = queue.pop_front() {
            let current_distance = distances[&current];

            for edge in graph.edges(current) {
                let neighbor = edge.target();

                if !distances.contains_key(&neighbor) {
                    distances.insert(neighbor, current_distance + 1.0);
                    queue.push_back(neighbor);
                }
            }
        }

        distances
    }
}

/// Cycle detection algorithms
pub struct CycleDetection;

impl CycleDetection {
    /// Find all simple cycles in the graph using Johnson's algorithm
    pub fn find_all_cycles<N, E>(graph: &Graph<N, E, Directed>) -> Vec<Vec<GraphNodeId>> {
        let mut cycles = Vec::new();
        let nodes: Vec<_> = graph.node_indices().collect();

        // For simplicity, we'll use a basic DFS-based approach
        // In a production system, Johnson's algorithm would be more efficient
        for &start in &nodes {
            let mut visited = HashSet::new();
            let mut path = Vec::new();
            Self::dfs_cycles(graph, start, start, &mut visited, &mut path, &mut cycles);
        }

        cycles
    }

    /// DFS-based cycle detection helper
    fn dfs_cycles<N, E>(
        graph: &Graph<N, E, Directed>,
        start: GraphNodeId,
        current: GraphNodeId,
        visited: &mut HashSet<GraphNodeId>,
        path: &mut Vec<GraphNodeId>,
        cycles: &mut Vec<Vec<GraphNodeId>>
    ) {
        if visited.contains(&current) && current == start && path.len() > 1 {
            // Found a cycle back to start
            cycles.push(path.clone());
            return;
        }

        if visited.contains(&current) {
            return;
        }

        visited.insert(current);
        path.push(current);

        for edge in graph.edges(current) {
            let neighbor = edge.target();
            Self::dfs_cycles(graph, start, neighbor, visited, path, cycles);
        }

        visited.remove(&current);
        path.pop();
    }
}

/// Network analysis algorithms
pub struct NetworkAnalysis;

impl NetworkAnalysis {
    /// Calculate clustering coefficient for a node
    pub fn clustering_coefficient<N, E>(graph: &Graph<N, E, Directed>, node: GraphNodeId) -> f64 {
        let neighbors: HashSet<_> = graph.edges(node)
            .map(|edge| edge.target())
            .chain(graph.edges_directed(node, petgraph::Incoming).map(|edge| edge.source()))
            .collect();

        let neighbor_count = neighbors.len();
        if neighbor_count < 2 {
            return 0.0;
        }

        let mut connections = 0;
        for &neighbor1 in &neighbors {
            for &neighbor2 in &neighbors {
                if neighbor1 != neighbor2 && graph.find_edge(neighbor1, neighbor2).is_some() {
                    connections += 1;
                }
            }
        }

        connections as f64 / (neighbor_count * (neighbor_count - 1)) as f64
    }

    /// Calculate average clustering coefficient for the entire graph
    pub fn average_clustering_coefficient<N, E>(graph: &Graph<N, E, Directed>) -> f64 {
        let nodes: Vec<_> = graph.node_indices().collect();

        if nodes.is_empty() {
            return 0.0;
        }

        let total_clustering: f64 = nodes.iter()
            .map(|&node| Self::clustering_coefficient(graph, node))
            .sum();

        total_clustering / nodes.len() as f64
    }

    /// Calculate graph density
    pub fn graph_density<N, E>(graph: &Graph<N, E, Directed>) -> f64 {
        let node_count = graph.node_count() as f64;
        let edge_count = graph.edge_count() as f64;

        if node_count <= 1.0 {
            return 0.0;
        }

        // For directed graphs: max edges = n * (n - 1)
        let max_edges = node_count * (node_count - 1.0);
        edge_count / max_edges
    }

    /// Calculate average path length using BFS from all nodes
    pub fn average_path_length<N, E>(graph: &Graph<N, E, Directed>) -> f64 {
        let nodes: Vec<_> = graph.node_indices().collect();

        if nodes.len() < 2 {
            return 0.0;
        }

        let mut total_length = 0.0;
        let mut path_count = 0;

        for &source in &nodes {
            let distances = CentralityAlgorithms::single_source_shortest_path_length(graph, source);

            for &target in &nodes {
                if source != target {
                    if let Some(&distance) = distances.get(&target) {
                        total_length += distance;
                        path_count += 1;
                    }
                }
            }
        }

        if path_count > 0 {
            total_length / path_count as f64
        } else {
            0.0
        }
    }

    /// Calculate graph diameter (longest shortest path)
    pub fn graph_diameter<N, E>(graph: &Graph<N, E, Directed>) -> usize {
        let nodes: Vec<_> = graph.node_indices().collect();
        let mut max_distance = 0;

        for &source in &nodes {
            let distances = CentralityAlgorithms::single_source_shortest_path_length(graph, source);

            for distance in distances.values() {
                max_distance = max_distance.max(*distance as usize);
            }
        }

        max_distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::Graph;

    #[test]
    fn test_degree_centrality() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());

        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n3, ());

        let centrality = CentralityAlgorithms::degree_centrality(&graph);

        // Node 2 should have highest centrality (connected to both others)
        assert!(centrality[&n2] > centrality[&n1]);
        assert!(centrality[&n2] > centrality[&n3]);
    }

    #[test]
    fn test_pagerank_centrality() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());

        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n3, ());
        graph.add_edge(n3, n1, ());

        let pagerank = CentralityAlgorithms::pagerank_centrality(&graph, 0.85, 100, 1e-6);

        // All nodes should have similar PageRank in this symmetric case
        let values: Vec<_> = pagerank.values().collect();
        let avg = values.iter().copied().sum::<f64>() / values.len() as f64;

        for &value in &values {
            assert!((value - avg).abs() < 0.1);
        }
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());

        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n3, ());
        graph.add_edge(n3, n1, ());

        let cycles = CycleDetection::find_all_cycles(&graph);
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_clustering_coefficient() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());

        graph.add_edge(n1, n2, ());
        graph.add_edge(n1, n3, ());
        graph.add_edge(n2, n3, ()); // Creates triangle

        let clustering = NetworkAnalysis::clustering_coefficient(&graph, n1);
        assert!(clustering > 0.0);
    }

    #[test]
    fn test_graph_density() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());

        // No edges - density should be 0
        assert_eq!(NetworkAnalysis::graph_density(&graph), 0.0);

        graph.add_edge(n1, n2, ());

        // One edge between two nodes - density should be 0.5
        let density = NetworkAnalysis::graph_density(&graph);
        assert!((density - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_empty_graph() {
        let graph: Graph<(), (), Directed> = Graph::new();
        let centrality = CentralityAlgorithms::degree_centrality(&graph);
        assert!(centrality.is_empty());

        let density = NetworkAnalysis::graph_density(&graph);
        assert_eq!(density, 0.0);
    }
}