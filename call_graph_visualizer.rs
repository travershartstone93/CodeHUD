use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphNode {
    pub name: String,
    pub call_count: usize,
    pub complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphEdge {
    pub from: String,
    pub to: String,
    pub weight: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub nodes: Vec<CallGraphNode>,
    pub edges: Vec<CallGraphEdge>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_call(&mut self, caller: &str, callee: &str) {
        // Add nodes if they don't exist
        if !self.nodes.iter().any(|n| n.name == caller) {
            self.nodes.push(CallGraphNode {
                name: caller.to_string(),
                call_count: 0,
                complexity: 1.0,
            });
        }
        if !self.nodes.iter().any(|n| n.name == callee) {
            self.nodes.push(CallGraphNode {
                name: callee.to_string(),
                call_count: 0,
                complexity: 1.0,
            });
        }

        // Add or update edge
        if let Some(edge) = self.edges.iter_mut().find(|e| e.from == caller && e.to == callee) {
            edge.weight += 1;
        } else {
            self.edges.push(CallGraphEdge {
                from: caller.to_string(),
                to: callee.to_string(),
                weight: 1,
            });
        }

        // Update call counts
        if let Some(node) = self.nodes.iter_mut().find(|n| n.name == caller) {
            node.call_count += 1;
        }
    }

    pub fn visualize_ascii(&self) -> String {
        let mut output = String::new();

        output.push_str("┌─────────────────────────────────────────────────────────────────┐\n");
        output.push_str("│                       CALL GRAPH VISUALIZATION                 │\n");
        output.push_str("├─────────────────────────────────────────────────────────────────┤\n");

        // Show node statistics
        output.push_str("│ 📊 FUNCTION ANALYSIS                                           │\n");
        output.push_str("│                                                                 │\n");

        let mut sorted_nodes = self.nodes.clone();
        sorted_nodes.sort_by(|a, b| b.call_count.cmp(&a.call_count));

        for node in sorted_nodes.iter().take(10) {
            let call_bar = create_bar(node.call_count, 20, "█", "░");
            let name = truncate_string(&node.name, 25);
            output.push_str(&format!("│ {:25} │{:20}│ {:3} calls     │\n",
                name, call_bar, node.call_count));
        }

        output.push_str("│                                                                 │\n");
        output.push_str("│ 🔗 CALL RELATIONSHIPS                                          │\n");
        output.push_str("│                                                                 │\n");

        // Show call relationships
        let mut sorted_edges = self.edges.clone();
        sorted_edges.sort_by(|a, b| b.weight.cmp(&a.weight));

        for edge in sorted_edges.iter().take(15) {
            let arrow = match edge.weight {
                1..=2 => "┄┄┄▶",
                3..=5 => "───▶",
                _ => "━━━▶",
            };
            let from = truncate_string(&edge.from, 20);
            let to = truncate_string(&edge.to, 20);
            output.push_str(&format!("│ {:20} {:4} {:20} ({:2}x)      │\n",
                from, arrow, to, edge.weight));
        }

        output.push_str("│                                                                 │\n");
        output.push_str("│ 📈 GRAPH STATISTICS                                            │\n");
        output.push_str(&format!("│ Total Functions: {:3}  Total Calls: {:4}                     │\n",
            self.nodes.len(), self.edges.iter().map(|e| e.weight).sum::<usize>()));
        output.push_str(&format!("│ Avg Calls/Function: {:4.1}  Max Calls: {:3}                   │\n",
            self.edges.iter().map(|e| e.weight).sum::<usize>() as f64 / self.nodes.len() as f64,
            sorted_edges.first().map(|e| e.weight).unwrap_or(0)));

        output.push_str("└─────────────────────────────────────────────────────────────────┘\n");

        output
    }

    pub fn generate_dot_format(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph CallGraph {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=filled, fillcolor=lightblue];\n");
        dot.push_str("  edge [color=gray, arrowhead=open];\n\n");

        // Add nodes with sizing based on call count
        for node in &self.nodes {
            let size = (node.call_count as f64 * 0.1 + 0.5).min(2.0);
            dot.push_str(&format!("  \"{}\" [width={:.1}, height={:.1}];\n",
                node.name, size, size * 0.7));
        }

        dot.push_str("\n");

        // Add edges with thickness based on weight
        for edge in &self.edges {
            let thickness = edge.weight.min(5);
            dot.push_str(&format!("  \"{}\" -> \"{}\" [penwidth={}];\n",
                edge.from, edge.to, thickness));
        }

        dot.push_str("}\n");
        dot
    }
}

fn create_bar(value: usize, max_width: usize, filled: &str, empty: &str) -> String {
    let max_val = 20; // Normalize to reasonable max
    let normalized = (value as f64 / max_val as f64).min(1.0);
    let filled_count = (normalized * max_width as f64) as usize;
    let empty_count = max_width - filled_count;

    format!("{}{}", filled.repeat(filled_count), empty.repeat(empty_count))
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{}..{:width$}", &s[..max_len-5], &s[s.len()-2..], width = max_len-3)
    }
}

fn main() {
    println!("🔍 CodeHUD Call Graph Visualizer");
    println!("==================================");

    // Create sample call graph
    let mut graph = CallGraph::new();

    // Add sample function calls (in real usage, this would come from analysis)
    let sample_calls = [
        ("main", "analyze_codebase"),
        ("main", "export_visualizations"),
        ("analyze_codebase", "extract_dependencies"),
        ("analyze_codebase", "calculate_metrics"),
        ("analyze_codebase", "parse_files"),
        ("extract_dependencies", "parse_file"),
        ("extract_dependencies", "build_dependency_graph"),
        ("calculate_metrics", "complexity_analysis"),
        ("calculate_metrics", "quality_scoring"),
        ("parse_file", "tokenize"),
        ("parse_file", "ast_analysis"),
        ("build_dependency_graph", "add_edge"),
        ("build_dependency_graph", "detect_cycles"),
        ("complexity_analysis", "cyclomatic_complexity"),
        ("quality_scoring", "maintainability_index"),
        ("export_visualizations", "render_view"),
        ("export_visualizations", "save_to_file"),
        ("render_view", "format_output"),
        ("render_view", "apply_styling"),
        ("main", "export_visualizations"), // duplicate to show weight
        ("analyze_codebase", "parse_files"), // duplicate
    ];

    for (caller, callee) in sample_calls.iter() {
        graph.add_call(caller, callee);
    }

    println!("\n📊 ASCII Call Graph Visualization:");
    println!("{}", graph.visualize_ascii());

    println!("\n💾 Saving DOT format for Graphviz...");
    std::fs::write("call_graph.dot", graph.generate_dot_format())
        .expect("Failed to write DOT file");

    println!("✅ Call graph visualization complete!");
    println!("📁 Generated files:");
    println!("   - ASCII visualization (shown above)");
    println!("   - call_graph.dot (for Graphviz: dot -Tpng call_graph.dot -o call_graph.png)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_graph_creation() {
        let mut graph = CallGraph::new();
        graph.add_call("main", "helper");
        graph.add_call("main", "helper"); // duplicate

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].weight, 2);
    }

    #[test]
    fn test_ascii_visualization() {
        let mut graph = CallGraph::new();
        graph.add_call("main", "helper");

        let ascii = graph.visualize_ascii();
        assert!(ascii.contains("CALL GRAPH VISUALIZATION"));
        assert!(ascii.contains("main"));
        assert!(ascii.contains("helper"));
    }
}