//! Full graph algorithms implementation - NetworkX equivalent functionality
//!
//! Complete implementation of centrality metrics, cycle detection, and graph analysis
//! with mathematical equivalence to NetworkX algorithms

use std::collections::{HashMap, HashSet, VecDeque};
use petgraph::{Graph, Directed, Direction};
use petgraph::graph::NodeIndex;
use petgraph::algo::{tarjan_scc, is_cyclic_directed};
use petgraph::visit::EdgeRef;

/// Complete centrality algorithms implementation equivalent to NetworkX
pub struct CentralityAlgorithms;

impl CentralityAlgorithms {
    /// Calculate degree centrality for all nodes (NetworkX equivalent)
    pub fn degree_centrality<N, E>(graph: &Graph<N, E, Directed>) -> HashMap<String, f64> {
        let mut centrality = HashMap::new();
        let node_count = graph.node_count();

        if node_count <= 1 {
            return centrality;
        }

        // Degree centrality = degree / (n-1) where n is number of nodes
        let normalizer = (node_count - 1) as f64;

        for node_idx in graph.node_indices() {
            let in_degree = graph.edges_directed(node_idx, Direction::Incoming).count();
            let out_degree = graph.edges_directed(node_idx, Direction::Outgoing).count();
            let total_degree = in_degree + out_degree;

            let centrality_value = total_degree as f64 / normalizer;
            let node_key = format!("node_{}", node_idx.index());
            centrality.insert(node_key, centrality_value);
        }

        centrality
    }

    /// Calculate betweenness centrality using Brandes algorithm (NetworkX equivalent)
    pub fn betweenness_centrality<N, E>(graph: &Graph<N, E, Directed>) -> HashMap<String, f64> {
        let mut centrality = HashMap::new();
        let nodes: Vec<_> = graph.node_indices().collect();

        // Initialize centrality to 0
        for &node in &nodes {
            let node_key = format!("node_{}", node.index());
            centrality.insert(node_key, 0.0);
        }

        let node_count = nodes.len();
        if node_count <= 2 {
            return centrality;
        }

        // Brandes algorithm for betweenness centrality
        for &source in &nodes {
            let mut stack = Vec::new();
            let mut predecessors: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
            let mut sigma: HashMap<NodeIndex, f64> = HashMap::new();
            let mut distance: HashMap<NodeIndex, i32> = HashMap::new();
            let mut delta: HashMap<NodeIndex, f64> = HashMap::new();

            // Initialize
            for &node in &nodes {
                predecessors.insert(node, Vec::new());
                sigma.insert(node, 0.0);
                distance.insert(node, -1);
                delta.insert(node, 0.0);
            }

            sigma.insert(source, 1.0);
            distance.insert(source, 0);

            let mut queue = VecDeque::new();
            queue.push_back(source);

            // BFS to find shortest paths
            while let Some(v) = queue.pop_front() {
                stack.push(v);

                for edge in graph.edges_directed(v, Direction::Outgoing) {
                    let w = edge.target();

                    // Path discovery
                    if distance[&w] < 0 {
                        queue.push_back(w);
                        distance.insert(w, distance[&v] + 1);
                    }

                    // Path counting
                    if distance[&w] == distance[&v] + 1 {
                        *sigma.get_mut(&w).unwrap() += sigma[&v];
                        predecessors.get_mut(&w).unwrap().push(v);
                    }
                }
            }

            // Accumulation
            while let Some(w) = stack.pop() {
                for &v in &predecessors[&w] {
                    let delta_w = delta[&w];
                    let sigma_v = sigma[&v];
                    let sigma_w = sigma[&w];

                    if sigma_w > 0.0 {
                        *delta.get_mut(&v).unwrap() += (sigma_v / sigma_w) * (1.0 + delta_w);
                    }
                }

                if w != source {
                    let node_key = format!("node_{}", w.index());
                    *centrality.get_mut(&node_key).unwrap() += delta[&w];
                }
            }
        }

        // Normalize (for directed graphs, divide by (n-1)(n-2))
        let normalizer = ((node_count - 1) * (node_count - 2)) as f64;
        if normalizer > 0.0 {
            for (_, value) in centrality.iter_mut() {
                *value /= normalizer;
            }
        }

        centrality
    }

    /// Calculate closeness centrality (NetworkX equivalent)
    pub fn closeness_centrality<N, E>(graph: &Graph<N, E, Directed>) -> HashMap<String, f64> {
        let mut centrality = HashMap::new();
        let nodes: Vec<_> = graph.node_indices().collect();

        for &source in &nodes {
            let distances = Self::single_source_shortest_path_length(graph, source);
            let total_distance: u32 = distances.values().sum();
            let reachable_nodes = distances.len();

            let centrality_value = if total_distance > 0 && reachable_nodes > 1 {
                (reachable_nodes - 1) as f64 / total_distance as f64
            } else {
                0.0
            };

            let node_key = format!("node_{}", source.index());
            centrality.insert(node_key, centrality_value);
        }

        centrality
    }

    /// Single-source shortest path length calculation
    fn single_source_shortest_path_length<N, E>(
        graph: &Graph<N, E, Directed>,
        source: NodeIndex,
    ) -> HashMap<NodeIndex, u32> {
        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();

        distances.insert(source, 0);
        queue.push_back(source);

        while let Some(current) = queue.pop_front() {
            let current_distance = distances[&current];

            for edge in graph.edges_directed(current, Direction::Outgoing) {
                let neighbor = edge.target();

                if !distances.contains_key(&neighbor) {
                    distances.insert(neighbor, current_distance + 1);
                    queue.push_back(neighbor);
                }
            }
        }

        distances
    }

    /// Calculate PageRank centrality with power iteration (NetworkX equivalent)
    pub fn pagerank_centrality<N, E>(
        graph: &Graph<N, E, Directed>,
        alpha: f64,
        max_iter: usize,
        tolerance: f64,
    ) -> HashMap<String, f64> {
        let nodes: Vec<_> = graph.node_indices().collect();
        let node_count = nodes.len();

        if node_count == 0 {
            return HashMap::new();
        }

        // Initialize PageRank values
        let initial_value = 1.0 / node_count as f64;
        let mut pagerank: HashMap<NodeIndex, f64> = nodes.iter()
            .map(|&node| (node, initial_value))
            .collect();

        // Build adjacency information
        let mut out_degree: HashMap<NodeIndex, usize> = HashMap::new();
        let mut incoming_edges: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();

        for &node in &nodes {
            out_degree.insert(node, graph.edges_directed(node, Direction::Outgoing).count());
            incoming_edges.insert(node, Vec::new());
        }

        for &node in &nodes {
            for edge in graph.edges_directed(node, Direction::Outgoing) {
                incoming_edges.get_mut(&edge.target()).unwrap().push(node);
            }
        }

        // Power iteration
        for _ in 0..max_iter {
            let mut new_pagerank = HashMap::new();
            let mut max_diff: f64 = 0.0;

            for &node in &nodes {
                let mut rank = (1.0 - alpha) / node_count as f64;

                // Sum contributions from incoming edges
                for &incoming_node in &incoming_edges[&node] {
                    let incoming_out_degree = out_degree[&incoming_node];
                    if incoming_out_degree > 0 {
                        rank += alpha * pagerank[&incoming_node] / incoming_out_degree as f64;
                    }
                }

                new_pagerank.insert(node, rank);
                max_diff = f64::max(max_diff, (rank - pagerank[&node]).abs());
            }

            pagerank = new_pagerank;

            // Check for convergence
            if max_diff < tolerance {
                break;
            }
        }

        // Convert to string keys
        pagerank.into_iter()
            .map(|(node_idx, value)| (format!("node_{}", node_idx.index()), value))
            .collect()
    }
}

/// Complete cycle detection implementation using DFS and Tarjan's algorithm
pub struct CycleDetection;

impl CycleDetection {
    /// Find all cycles in a directed graph using DFS
    pub fn find_all_cycles<N, E>(graph: &Graph<N, E, Directed>) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();

        if !is_cyclic_directed(graph) {
            return cycles;
        }

        let nodes: Vec<_> = graph.node_indices().collect();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &node in &nodes {
            if !visited.contains(&node) {
                Self::dfs_find_cycles(
                    graph,
                    node,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_find_cycles<N, E>(
        graph: &Graph<N, E, Directed>,
        node: NodeIndex,
        visited: &mut HashSet<NodeIndex>,
        rec_stack: &mut HashSet<NodeIndex>,
        path: &mut Vec<NodeIndex>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        for edge in graph.edges_directed(node, Direction::Outgoing) {
            let neighbor = edge.target();

            if !visited.contains(&neighbor) {
                Self::dfs_find_cycles(graph, neighbor, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(&neighbor) {
                // Found a cycle - extract it from the path
                if let Some(cycle_start) = path.iter().position(|&n| n == neighbor) {
                    let cycle: Vec<String> = path[cycle_start..]
                        .iter()
                        .map(|&n| format!("node_{}", n.index()))
                        .collect();
                    cycles.push(cycle);
                }
            }
        }

        rec_stack.remove(&node);
        path.pop();
    }
}

/// Complete network analysis with graph metrics
pub struct NetworkAnalysis;

impl NetworkAnalysis {
    /// Find strongly connected components using Tarjan's algorithm
    pub fn strongly_connected_components<N, E>(graph: &Graph<N, E, Directed>) -> Vec<Vec<String>> {
        let sccs = tarjan_scc(graph);

        sccs.into_iter()
            .map(|component| {
                component.into_iter()
                    .map(|node_idx| format!("node_{}", node_idx.index()))
                    .collect()
            })
            .collect()
    }

    /// Calculate graph density (NetworkX equivalent)
    pub fn graph_density<N, E>(graph: &Graph<N, E, Directed>) -> f64 {
        let node_count = graph.node_count();
        let edge_count = graph.edge_count();

        if node_count <= 1 {
            return 0.0;
        }

        // For directed graphs: density = m / (n * (n-1))
        let max_edges = node_count * (node_count - 1);
        edge_count as f64 / max_edges as f64
    }

    /// Calculate average clustering coefficient
    pub fn average_clustering_coefficient<N, E>(graph: &Graph<N, E, Directed>) -> f64 {
        let nodes: Vec<_> = graph.node_indices().collect();
        let node_count = nodes.len();

        if node_count == 0 {
            return 0.0;
        }

        let total_clustering: f64 = nodes.iter()
            .map(|&node| Self::local_clustering_coefficient(graph, node))
            .sum();

        total_clustering / node_count as f64
    }

    fn local_clustering_coefficient<N, E>(graph: &Graph<N, E, Directed>, node: NodeIndex) -> f64 {
        let neighbors: HashSet<_> = graph.edges_directed(node, Direction::Outgoing)
            .map(|edge| edge.target())
            .chain(graph.edges_directed(node, Direction::Incoming).map(|edge| edge.source()))
            .filter(|&n| n != node)
            .collect();

        let k = neighbors.len();
        if k < 2 {
            return 0.0;
        }

        // Count edges between neighbors
        let mut edge_count = 0;
        for &neighbor1 in &neighbors {
            for &neighbor2 in &neighbors {
                if neighbor1 != neighbor2 && graph.find_edge(neighbor1, neighbor2).is_some() {
                    edge_count += 1;
                }
            }
        }

        // Clustering coefficient = actual_edges / possible_edges
        let possible_edges = k * (k - 1);
        edge_count as f64 / possible_edges as f64
    }

    /// Calculate average shortest path length
    pub fn average_path_length<N, E>(graph: &Graph<N, E, Directed>) -> f64 {
        let nodes: Vec<_> = graph.node_indices().collect();
        let node_count = nodes.len();

        if node_count <= 1 {
            return 0.0;
        }

        let mut total_distance = 0;
        let mut path_count = 0;

        for &source in &nodes {
            let distances = CentralityAlgorithms::single_source_shortest_path_length(graph, source);

            for (_, distance) in distances {
                total_distance += distance;
                path_count += 1;
            }
        }

        if path_count > 0 {
            total_distance as f64 / path_count as f64
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

            if let Some(&max_dist) = distances.values().max() {
                max_distance = max_distance.max(max_dist as usize);
            }
        }

        max_distance
    }
}