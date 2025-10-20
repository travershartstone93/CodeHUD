//! Call Graph Visualization Module
//!
//! Provides call graph analysis and visualization capabilities using ratatui widgets.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    pub fn render_to_terminal(&self, frame: &mut Frame, area: Rect) {
        // Split area into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(10),    // Function analysis chart
                Constraint::Min(8),     // Call relationships
                Constraint::Length(4),  // Statistics
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Call Graph Visualization")
            .block(Block::default().borders(Borders::ALL).title("📊 CodeHUD"))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        frame.render_widget(title, chunks[0]);

        // Function analysis as bar chart
        self.render_function_chart(frame, chunks[1]);

        // Call relationships list
        self.render_call_relationships(frame, chunks[2]);

        // Statistics
        self.render_statistics(frame, chunks[3]);
    }

    fn render_function_chart(&self, frame: &mut Frame, area: Rect) {
        let mut sorted_nodes = self.nodes.clone();
        sorted_nodes.sort_by(|a, b| b.call_count.cmp(&a.call_count));

        // Convert to bar chart data
        let data: Vec<(&str, u64)> = sorted_nodes
            .iter()
            .take(10)
            .map(|node| (node.name.as_str(), node.call_count as u64))
            .collect();

        let chart = BarChart::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Function Call Frequency")
            )
            .data(&data)
            .bar_width(3)
            .bar_style(Style::default().fg(Color::Yellow))
            .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

        frame.render_widget(chart, area);
    }

    fn render_call_relationships(&self, frame: &mut Frame, area: Rect) {
        let mut sorted_edges = self.edges.clone();
        sorted_edges.sort_by(|a, b| b.weight.cmp(&a.weight));

        let items: Vec<ListItem> = sorted_edges
            .iter()
            .take(15)
            .map(|edge| {
                let arrow = match edge.weight {
                    1..=2 => "┄┄┄▶",
                    3..=5 => "───▶",
                    _ => "━━━▶",
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:20}", truncate_string(&edge.from, 20)),
                        Style::default().fg(Color::White)
                    ),
                    Span::styled(
                        format!(" {} ", arrow),
                        Style::default().fg(Color::Blue)
                    ),
                    Span::styled(
                        format!("{:20}", truncate_string(&edge.to, 20)),
                        Style::default().fg(Color::White)
                    ),
                    Span::styled(
                        format!(" ({:2}x)", edge.weight),
                        Style::default().fg(Color::Green)
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Call Relationships")
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(list, area);
    }

    fn render_statistics(&self, frame: &mut Frame, area: Rect) {
        let total_calls: usize = self.edges.iter().map(|e| e.weight).sum();
        let avg_calls = if !self.nodes.is_empty() {
            total_calls as f64 / self.nodes.len() as f64
        } else {
            0.0
        };
        let max_calls = self.edges.iter().map(|e| e.weight).max().unwrap_or(0);

        let stats_text = format!(
            "Functions: {}  |  Total Calls: {}  |  Avg: {:.1}  |  Max: {}",
            self.nodes.len(),
            total_calls,
            avg_calls,
            max_calls
        );

        let stats = Paragraph::new(stats_text)
            .block(Block::default().borders(Borders::ALL).title("Statistics"))
            .style(Style::default().fg(Color::Cyan));

        frame.render_widget(stats, area);
    }

    pub fn to_text_visualization(&self) -> String {
        let mut output = String::new();

        output.push_str("┌─────────────────────────────────────────────────────────────────┐\n");
        output.push_str("│                       CALL GRAPH VISUALIZATION                 │\n");
        output.push_str("├─────────────────────────────────────────────────────────────────┤\n");

        // Function analysis
        output.push_str("│ 📊 FUNCTION ANALYSIS                                           │\n");
        output.push_str("│                                                                 │\n");

        let mut sorted_nodes = self.nodes.clone();
        sorted_nodes.sort_by(|a, b| b.call_count.cmp(&a.call_count));

        for node in sorted_nodes.iter().take(10) {
            let call_bar = create_text_bar(node.call_count, 20);
            let name = truncate_string(&node.name, 25);
            output.push_str(&format!(
                "│ {:25} │{:20}│ {:3} calls     │\n",
                name, call_bar, node.call_count
            ));
        }

        output.push_str("│                                                                 │\n");
        output.push_str("│ 🔗 CALL RELATIONSHIPS                                          │\n");
        output.push_str("│                                                                 │\n");

        // Call relationships
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
            output.push_str(&format!(
                "│ {:20} {:4} {:20} ({:2}x)      │\n",
                from, arrow, to, edge.weight
            ));
        }

        output.push_str("│                                                                 │\n");
        output.push_str("│ 📈 GRAPH STATISTICS                                            │\n");

        let total_calls: usize = self.edges.iter().map(|e| e.weight).sum();
        let avg_calls = if !self.nodes.is_empty() {
            total_calls as f64 / self.nodes.len() as f64
        } else {
            0.0
        };
        let max_calls = self.edges.iter().map(|e| e.weight).max().unwrap_or(0);

        output.push_str(&format!(
            "│ Total Functions: {:3}  Total Calls: {:4}                     │\n",
            self.nodes.len(),
            total_calls
        ));
        output.push_str(&format!(
            "│ Avg Calls/Function: {:4.1}  Max Calls: {:3}                   │\n",
            avg_calls, max_calls
        ));

        output.push_str("└─────────────────────────────────────────────────────────────────┘\n");

        output
    }

}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

fn create_text_bar(value: usize, max_width: usize) -> String {
    let max_val = 10; // Normalize to reasonable max
    let normalized = (value as f64 / max_val as f64).min(1.0);
    let filled_count = (normalized * max_width as f64) as usize;
    let empty_count = max_width - filled_count;

    format!("{}{}", "█".repeat(filled_count), "░".repeat(empty_count))
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{}..{}", &s[..max_len-5], &s[s.len()-2..])
    }
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
    fn test_text_visualization() {
        let mut graph = CallGraph::new();
        graph.add_call("main", "analyze_codebase");
        graph.add_call("main", "export_data");
        let text = graph.to_text_visualization();
        assert!(text.contains("CALL GRAPH VISUALIZATION"));
        assert!(text.contains("main"));
    }
}