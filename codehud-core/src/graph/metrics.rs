//! Graph metrics for CodeHUD analysis
//!
//! Implements NetworkX-equivalent metrics calculations:
//! - Centrality metrics (betweenness, closeness, degree, eigenvector)
//! - Coupling metrics (afferent, efferent, instability)
//! - Graph structural metrics

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use super::GraphNodeId;

/// Centrality metrics for graph nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralityMetrics {
    /// Betweenness centrality for each node
    pub betweenness: HashMap<GraphNodeId, f64>,
    /// Closeness centrality for each node
    pub closeness: HashMap<GraphNodeId, f64>,
    /// Degree centrality for each node
    pub degree: HashMap<GraphNodeId, f64>,
    /// Eigenvector centrality for each node
    pub eigenvector: HashMap<GraphNodeId, f64>,
    /// PageRank scores for each node
    pub pagerank: HashMap<GraphNodeId, f64>,
}

impl CentralityMetrics {
    /// Create empty centrality metrics
    pub fn new() -> Self {
        Self {
            betweenness: HashMap::new(),
            closeness: HashMap::new(),
            degree: HashMap::new(),
            eigenvector: HashMap::new(),
            pagerank: HashMap::new(),
        }
    }

    /// Get the most central node by betweenness centrality
    pub fn most_central_betweenness(&self) -> Option<(GraphNodeId, f64)> {
        self.betweenness
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(node, &centrality)| (node.clone(), centrality))
    }

    /// Get the most central node by closeness centrality
    pub fn most_central_closeness(&self) -> Option<(GraphNodeId, f64)> {
        self.closeness
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(node, &centrality)| (node.clone(), centrality))
    }

    /// Get the highest degree node
    pub fn highest_degree(&self) -> Option<(GraphNodeId, f64)> {
        self.degree
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(node, &centrality)| (node.clone(), centrality))
    }

    /// Get top N nodes by PageRank
    pub fn top_pagerank(&self, n: usize) -> Vec<(GraphNodeId, f64)> {
        let mut pairs: Vec<_> = self.pagerank.iter().map(|(node, &score)| (node.clone(), score)).collect();
        pairs.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        pairs.into_iter().take(n).collect()
    }

    /// Calculate average centrality values
    pub fn average_centralities(&self) -> CentralityAverages {
        CentralityAverages {
            avg_betweenness: average_values(&self.betweenness),
            avg_closeness: average_values(&self.closeness),
            avg_degree: average_values(&self.degree),
            avg_eigenvector: average_values(&self.eigenvector),
            avg_pagerank: average_values(&self.pagerank),
        }
    }
}

impl Default for CentralityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Average centrality values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralityAverages {
    pub avg_betweenness: f64,
    pub avg_closeness: f64,
    pub avg_degree: f64,
    pub avg_eigenvector: f64,
    pub avg_pagerank: f64,
}

/// Coupling metrics for measuring module dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingMetrics {
    /// Afferent coupling (Ca) - number of incoming dependencies
    pub afferent_coupling: HashMap<GraphNodeId, usize>,
    /// Efferent coupling (Ce) - number of outgoing dependencies
    pub efferent_coupling: HashMap<GraphNodeId, usize>,
    /// Instability (I) - Ce / (Ca + Ce)
    pub instability: HashMap<GraphNodeId, f64>,
    /// Abstractness (A) - ratio of abstract classes to total classes
    pub abstractness: HashMap<GraphNodeId, f64>,
    /// Distance from main sequence (D) - |A + I - 1|
    pub distance_from_main: HashMap<GraphNodeId, f64>,
}

impl CouplingMetrics {
    /// Create empty coupling metrics
    pub fn new() -> Self {
        Self {
            afferent_coupling: HashMap::new(),
            efferent_coupling: HashMap::new(),
            instability: HashMap::new(),
            abstractness: HashMap::new(),
            distance_from_main: HashMap::new(),
        }
    }

    /// Calculate instability for a node
    pub fn calculate_instability(&mut self, node: GraphNodeId) {
        let ca = self.afferent_coupling.get(&node).copied().unwrap_or(0) as f64;
        let ce = self.efferent_coupling.get(&node).copied().unwrap_or(0) as f64;

        let instability = if ca + ce == 0.0 {
            0.0
        } else {
            ce / (ca + ce)
        };

        self.instability.insert(node, instability);
    }

    /// Calculate distance from main sequence for a node
    pub fn calculate_distance_from_main(&mut self, node: GraphNodeId) {
        let abstractness = self.abstractness.get(&node).copied().unwrap_or(0.0);
        let instability = self.instability.get(&node).copied().unwrap_or(0.0);

        let distance = (abstractness + instability - 1.0).abs();
        self.distance_from_main.insert(node, distance);
    }

    /// Get most coupled nodes (highest afferent + efferent)
    pub fn most_coupled_nodes(&self, n: usize) -> Vec<(GraphNodeId, usize)> {
        let mut nodes: Vec<_> = self.afferent_coupling
            .keys()
            .map(|node| {
                let ca = self.afferent_coupling.get(node).copied().unwrap_or(0);
                let ce = self.efferent_coupling.get(node).copied().unwrap_or(0);
                (node.clone(), ca + ce)
            })
            .collect();

        nodes.sort_by(|(_, a), (_, b)| b.cmp(a));
        nodes.into_iter().take(n).collect()
    }

    /// Get most unstable nodes
    pub fn most_unstable_nodes(&self, n: usize) -> Vec<(GraphNodeId, f64)> {
        let mut nodes: Vec<_> = self.instability.iter().map(|(node, &instability)| (node.clone(), instability)).collect();
        nodes.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        nodes.into_iter().take(n).collect()
    }

    /// Calculate all coupling metrics for all nodes
    pub fn calculate_all_metrics(&mut self) {
        let nodes: Vec<_> = self.afferent_coupling.keys().cloned().collect();

        for node in nodes {
            self.calculate_instability(node.clone());
            self.calculate_distance_from_main(node);
        }
    }

    /// Get summary statistics
    pub fn summary_stats(&self) -> CouplingStats {
        CouplingStats {
            total_nodes: self.afferent_coupling.len(),
            avg_afferent: average_values_usize(&self.afferent_coupling),
            avg_efferent: average_values_usize(&self.efferent_coupling),
            avg_instability: average_values(&self.instability),
            avg_distance_from_main: average_values(&self.distance_from_main),
            max_coupling: self.most_coupled_nodes(1).first().map(|(_, coupling)| *coupling).unwrap_or(0),
        }
    }
}

impl Default for CouplingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for coupling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingStats {
    pub total_nodes: usize,
    pub avg_afferent: f64,
    pub avg_efferent: f64,
    pub avg_instability: f64,
    pub avg_distance_from_main: f64,
    pub max_coupling: usize,
}

/// Network analysis metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Graph density
    pub density: f64,
    /// Average clustering coefficient
    pub clustering_coefficient: f64,
    /// Average path length
    pub average_path_length: f64,
    /// Diameter of the graph
    pub diameter: usize,
    /// Number of connected components
    pub connected_components: usize,
    /// Largest component size
    pub largest_component_size: usize,
}

impl NetworkMetrics {
    /// Create new network metrics
    pub fn new() -> Self {
        Self {
            density: 0.0,
            clustering_coefficient: 0.0,
            average_path_length: 0.0,
            diameter: 0,
            connected_components: 0,
            largest_component_size: 0,
        }
    }

    /// Check if the network is sparse
    pub fn is_sparse(&self) -> bool {
        self.density < 0.1
    }

    /// Check if the network is dense
    pub fn is_dense(&self) -> bool {
        self.density > 0.5
    }

    /// Get network complexity score (0-1 scale)
    pub fn complexity_score(&self) -> f64 {
        let density_score = self.density;
        let clustering_score = self.clustering_coefficient;
        let path_score = if self.average_path_length > 0.0 {
            1.0 / self.average_path_length
        } else {
            0.0
        };

        (density_score + clustering_score + path_score) / 3.0
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to calculate average of HashMap values
fn average_values(map: &HashMap<GraphNodeId, f64>) -> f64 {
    if map.is_empty() {
        0.0
    } else {
        map.values().sum::<f64>() / map.len() as f64
    }
}

/// Helper function to calculate average of HashMap usize values
fn average_values_usize(map: &HashMap<GraphNodeId, usize>) -> f64 {
    if map.is_empty() {
        0.0
    } else {
        map.values().sum::<usize>() as f64 / map.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::NodeIndex;

    #[test]
    fn test_centrality_metrics_creation() {
        let metrics = CentralityMetrics::new();
        assert!(metrics.betweenness.is_empty());
        assert!(metrics.closeness.is_empty());
        assert!(metrics.degree.is_empty());
        assert!(metrics.eigenvector.is_empty());
        assert!(metrics.pagerank.is_empty());
    }

    #[test]
    fn test_centrality_metrics_most_central() {
        let mut metrics = CentralityMetrics::new();
        let node1 = NodeIndex::new(0);
        let node2 = NodeIndex::new(1);

        metrics.betweenness.insert(node1, 0.5);
        metrics.betweenness.insert(node2, 0.8);

        let (most_central, centrality) = metrics.most_central_betweenness().unwrap();
        assert_eq!(most_central, node2);
        assert_eq!(centrality, 0.8);
    }

    #[test]
    fn test_coupling_metrics_calculation() {
        let mut metrics = CouplingMetrics::new();
        let node = NodeIndex::new(0);

        metrics.afferent_coupling.insert(node, 3);
        metrics.efferent_coupling.insert(node, 2);

        metrics.calculate_instability(node);

        let instability = metrics.instability.get(&node).unwrap();
        assert_eq!(*instability, 2.0 / 5.0); // Ce / (Ca + Ce) = 2 / (3 + 2)
    }

    #[test]
    fn test_coupling_metrics_zero_coupling() {
        let mut metrics = CouplingMetrics::new();
        let node = NodeIndex::new(0);

        metrics.afferent_coupling.insert(node, 0);
        metrics.efferent_coupling.insert(node, 0);

        metrics.calculate_instability(node);

        let instability = metrics.instability.get(&node).unwrap();
        assert_eq!(*instability, 0.0);
    }

    #[test]
    fn test_network_metrics_complexity_score() {
        let mut metrics = NetworkMetrics::new();
        metrics.density = 0.3;
        metrics.clustering_coefficient = 0.4;
        metrics.average_path_length = 2.0;

        let complexity = metrics.complexity_score();
        let expected = (0.3 + 0.4 + 0.5) / 3.0; // 0.5 = 1/2.0
        assert!((complexity - expected).abs() < 0.001);
    }

    #[test]
    fn test_network_metrics_density_classification() {
        let mut sparse = NetworkMetrics::new();
        sparse.density = 0.05;
        assert!(sparse.is_sparse());
        assert!(!sparse.is_dense());

        let mut dense = NetworkMetrics::new();
        dense.density = 0.7;
        assert!(!dense.is_sparse());
        assert!(dense.is_dense());
    }

    #[test]
    fn test_average_values_helper() {
        let mut map = HashMap::new();
        map.insert(NodeIndex::new(0), 1.0);
        map.insert(NodeIndex::new(1), 2.0);
        map.insert(NodeIndex::new(2), 3.0);

        let avg = average_values(&map);
        assert_eq!(avg, 2.0);

        let empty_map = HashMap::new();
        let empty_avg = average_values(&empty_map);
        assert_eq!(empty_avg, 0.0);
    }
}