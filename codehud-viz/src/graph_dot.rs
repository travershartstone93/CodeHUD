//! DOT Format Export for Graph Visualization
//!
//! Exports petgraph structures to Graphviz DOT format with Doxygen-style
//! visual attributes including cycle highlighting, complexity coloring,
//! and module clustering.

use petgraph::graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use petgraph::{Direction, Graph, Directed};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use codehud_core::graph::{CallNode, CallEdge};
use crate::graph_analysis::{
    analyze_graph, build_module_graph, extract_cycle_subgraph, extract_module_subgraph,
    GraphAnalysis
};
use crate::graphviz::LayoutEngine;

/// DOT exporter with Doxygen-compatible styling
pub struct DotExporter {
    /// Highlight cycles in red
    pub show_cycles: bool,
    /// Color nodes by complexity/coupling
    pub color_by_complexity: bool,
    /// Show edge weights (call counts)
    pub show_edge_weights: bool,
    /// Group nodes by module/file
    pub cluster_by_module: bool,
}

impl DotExporter {
    /// Create a new DOT exporter with default Doxygen-style settings
    pub fn new() -> Self {
        Self {
            show_cycles: true,
            color_by_complexity: true,
            show_edge_weights: true,
            cluster_by_module: true,
        }
    }

    /// Builder pattern: enable/disable cycle highlighting
    pub fn with_cycle_highlighting(mut self, enabled: bool) -> Self {
        self.show_cycles = enabled;
        self
    }

    /// Builder pattern: enable/disable complexity coloring
    pub fn with_complexity_coloring(mut self, enabled: bool) -> Self {
        self.color_by_complexity = enabled;
        self
    }

    /// Builder pattern: enable/disable edge weights
    pub fn with_edge_weights(mut self, enabled: bool) -> Self {
        self.show_edge_weights = enabled;
        self
    }

    /// Builder pattern: enable/disable module clustering
    pub fn with_module_clustering(mut self, enabled: bool) -> Self {
        self.cluster_by_module = enabled;
        self
    }

    /// Export call graph to Doxygen-compatible DOT format
    pub fn export_call_graph(&self, graph: &Graph<CallNode, CallEdge, Directed>) -> String {
        let mut dot = String::from("digraph CallGraph {\n");

        // Graph-level attributes (Doxygen style)
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica\"];\n");
        dot.push_str("  edge [fontname=\"Helvetica\", fontsize=10];\n");
        dot.push_str("  concentrate=true;\n");
        dot.push_str("  splines=ortho;\n\n");

        // Detect cycles once
        let cycles = if self.show_cycles {
            self.detect_cycles(graph)
        } else {
            vec![]
        };

        // Export nodes with attributes
        for (idx, node) in graph.node_references() {
            let color = if self.color_by_complexity {
                self.get_node_color(graph, idx)
            } else {
                "lightblue"
            };

            // Clean label - just function name
            let label = self.escape_dot_label(&node.function_name);

            dot.push_str(&format!(
                "  n{} [label=\"{}\", fillcolor=\"{}\"];\n",
                idx.index(),
                label,
                color
            ));
        }

        dot.push_str("\n");

        // Export edges with cycle detection
        for edge in graph.edge_references() {
            let source = edge.source().index();
            let target = edge.target().index();
            let weight = edge.weight().call_count;

            // Check if edge is part of cycle
            let in_cycle = self.is_edge_in_cycle(source, target, &cycles);
            let color = if in_cycle { "red" } else { "blue" };
            let penwidth = if in_cycle { "2.0" } else { "1.0" };

            let label = if self.show_edge_weights && weight > 1 {
                format!(" [label=\"{}x\", color=\"{}\", penwidth=\"{}\"]",
                       weight, color, penwidth)
            } else {
                format!(" [color=\"{}\", penwidth=\"{}\"]", color, penwidth)
            };

            dot.push_str(&format!("  n{} -> n{}{}\n", source, target, label));
        }

        // Add module clustering
        if self.cluster_by_module {
            let modules = self.group_by_module(graph);
            if modules.len() > 1 { // Only cluster if multiple modules
                for (module_name, nodes) in modules {
                    if nodes.len() > 1 { // Only create cluster if multiple nodes
                        dot.push_str(&format!("\n  subgraph cluster_{} {{\n",
                            self.sanitize_cluster_name(&module_name)));
                        dot.push_str(&format!("    label=\"{}\";\n",
                            self.escape_dot_label(&module_name)));
                        dot.push_str("    style=dashed;\n");
                        dot.push_str("    color=gray;\n");
                        for idx in nodes {
                            dot.push_str(&format!("    n{};\n", idx));
                        }
                        dot.push_str("  }\n");
                    }
                }
            }
        }

        dot.push_str("}\n");
        dot
    }

    /// Get node color based on in-degree (how often it's called)
    fn get_node_color(&self, graph: &Graph<CallNode, CallEdge, Directed>, idx: NodeIndex) -> &'static str {
        let in_degree = graph.edges_directed(idx, Direction::Incoming).count();
        match in_degree {
            0 => "lightgray",      // Never called (entry point)
            1..=2 => "lightgreen",  // Low coupling
            3..=5 => "yellow",      // Medium coupling
            6..=10 => "orange",     // High coupling
            _ => "red",             // Very high coupling (hotspot)
        }
    }

    /// Detect cycles using DFS
    fn detect_cycles(&self, graph: &Graph<CallNode, CallEdge, Directed>) -> Vec<Vec<usize>> {
        use std::collections::HashSet;

        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node_idx in graph.node_indices() {
            if !visited.contains(&node_idx) {
                self.dfs_find_cycles(
                    graph,
                    node_idx,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_find_cycles(
        &self,
        graph: &Graph<CallNode, CallEdge, Directed>,
        node: NodeIndex,
        visited: &mut std::collections::HashSet<NodeIndex>,
        rec_stack: &mut std::collections::HashSet<NodeIndex>,
        path: &mut Vec<NodeIndex>,
        cycles: &mut Vec<Vec<usize>>,
    ) {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        for edge in graph.edges_directed(node, Direction::Outgoing) {
            let neighbor = edge.target();

            if !visited.contains(&neighbor) {
                self.dfs_find_cycles(graph, neighbor, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(&neighbor) {
                // Found a cycle - extract it from the path
                if let Some(cycle_start) = path.iter().position(|&n| n == neighbor) {
                    let cycle: Vec<usize> = path[cycle_start..]
                        .iter()
                        .map(|&n| n.index())
                        .collect();
                    cycles.push(cycle);
                }
            }
        }

        rec_stack.remove(&node);
        path.pop();
    }

    /// Check if an edge is part of any detected cycle
    fn is_edge_in_cycle(&self, source: usize, target: usize, cycles: &[Vec<usize>]) -> bool {
        for cycle in cycles {
            if let Some(source_pos) = cycle.iter().position(|&n| n == source) {
                let next_pos = (source_pos + 1) % cycle.len();
                if cycle[next_pos] == target {
                    return true;
                }
            }
        }
        false
    }

    /// Group nodes by module (based on file path)
    fn group_by_module(&self, graph: &Graph<CallNode, CallEdge, Directed>) -> HashMap<String, Vec<usize>> {
        let mut modules: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, node) in graph.node_references() {
            // Extract module from file_path
            let module = if !node.file_path.is_empty() {
                std::path::Path::new(&node.file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                // Try to extract from qualified name (e.g., "module::function")
                if node.function_name.contains("::") {
                    node.function_name
                        .split("::")
                        .next()
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    "unknown".to_string()
                }
            };

            modules.entry(module).or_default().push(idx.index());
        }

        modules
    }

    /// Escape special characters for DOT labels
    fn escape_dot_label(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    /// Sanitize cluster name for DOT subgraph identifier
    fn sanitize_cluster_name(&self, s: &str) -> String {
        s.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect()
    }

    /// Auto-detect optimal rendering options based on graph size
    pub fn auto_detect_options(node_count: usize, edge_count: usize) -> (LayoutEngine, &'static str, bool) {
        if node_count > 1000 || edge_count > 2000 {
            // Large graph: use scalable force-directed, straight lines, no edge merging
            (LayoutEngine::Sfdp, "line", false)
        } else if node_count > 500 {
            // Medium graph: use force-directed, curved lines
            (LayoutEngine::Fdp, "spline", false)
        } else {
            // Small graph: use hierarchical Doxygen-style layout
            (LayoutEngine::Dot, "ortho", true)
        }
    }

    /// Export crate-level architecture graph (aggregated from modules)
    pub fn export_crate_level_graph(&self, graph: &Graph<CallNode, CallEdge, Directed>) -> String {
        // Infer crate from module name
        fn infer_crate(module_name: &str) -> &str {
            let name_lower = module_name.to_lowercase();

            // GUI
            if name_lower.contains("_view_gui") || matches!(name_lower.as_str(), "app" | "menu_bar" | "tabs" | "signals") {
                return "codehud-gui";
            }

            // LLM
            if name_lower.contains("llm") || name_lower.contains("extraction_fsm") || name_lower.contains("narrator")
                || name_lower.contains("gemini") || name_lower.contains("ollama") {
                return "codehud-llm";
            }

            // Transform
            if name_lower.contains("test_generation") || name_lower.contains("architectural")
                || name_lower.contains("multi_language") || name_lower.contains("transformers")
                || name_lower.contains("denoiser") || name_lower.contains("equivalence") {
                return "codehud-transform";
            }

            // Viz
            if name_lower.contains("visualizer") || name_lower.contains("graph_dot") {
                return "codehud-viz";
            }

            // CLI
            if name_lower.contains("pipeline") || name_lower.contains("direct") {
                return "codehud-cli";
            }

            // Analysis
            if name_lower.contains("health_") || name_lower.contains("analysis_") || name_lower.contains("monitoring") {
                return "codehud-analysis";
            }

            // TUI
            if name_lower.contains("project_explorer") {
                return "codehud-tui";
            }

            // Utils
            if matches!(name_lower.as_str(), "utils" | "setup" | "graphviz") {
                return "codehud-utils";
            }

            // Core (default)
            "codehud-core"
        }

        let analysis = analyze_graph(graph);

        // Group modules by crate
        let mut crate_modules: HashMap<String, Vec<String>> = HashMap::new();
        let mut crate_function_counts: HashMap<String, usize> = HashMap::new();

        for module in &analysis.modules {
            let crate_name = infer_crate(&module.name).to_string();
            crate_modules.entry(crate_name.clone()).or_default().push(module.name.clone());
            *crate_function_counts.entry(crate_name).or_default() += module.functions.len();
        }

        // Build crate-level dependency graph
        let mut crate_deps: HashMap<String, HashMap<String, usize>> = HashMap::new();

        // For each edge in the module graph, aggregate to crate level
        for module in &analysis.modules {
            let from_crate = infer_crate(&module.name);

            for &from_idx in &module.functions {
                for edge in graph.edges_directed(from_idx, Direction::Outgoing) {
                    let to_idx = edge.target();

                    // Find which module the target belongs to
                    if let Some(to_module) = analysis.modules.iter().find(|m| m.functions.contains(&to_idx)) {
                        let to_crate = infer_crate(&to_module.name);

                        // Only count inter-crate dependencies
                        if from_crate != to_crate {
                            *crate_deps.entry(from_crate.to_string())
                                .or_default()
                                .entry(to_crate.to_string())
                                .or_default() += edge.weight().call_count;
                        }
                    }
                }
            }
        }

        // Generate DOT output with hierarchical layout
        let mut dot = String::from("digraph CrateArchitecture {\n");
        dot.push_str("  graph [rankdir=TB, splines=ortho, nodesep=1.5, ranksep=2.0, compound=true, bgcolor=\"#f8f9fa\"];\n");
        dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica\", fontsize=11];\n");
        dot.push_str("  edge [fontname=\"Helvetica\", fontsize=9, color=\"#555555\"];\n\n");

        // Define crate colors
        let crate_colors: HashMap<&str, &str> = [
            ("codehud-core", "#a7d9ef"),
            ("codehud-cli", "#c8e6c9"),
            ("codehud-gui", "#ffe0b2"),
            ("codehud-llm", "#e1bee7"),
            ("codehud-transform", "#ffcdd2"),
            ("codehud-viz", "#b3e5fc"),
            ("codehud-analysis", "#dcedc8"),
            ("codehud-utils", "#fff9c4"),
            ("codehud-tui", "#f0f4c3"),
        ].iter().cloned().collect();

        // Foundation Layer (subgraph cluster)
        dot.push_str("  subgraph cluster_foundation {\n");
        dot.push_str("    label=\"Core Infrastructure\";\n");
        dot.push_str("    style=filled;\n");
        dot.push_str("    fillcolor=\"#e8f4f8\";\n");
        dot.push_str("    color=\"#4a90a4\";\n");
        dot.push_str("    penwidth=2;\n");

        for crate_name in ["codehud-utils", "codehud-core"] {
            if let Some(&function_count) = crate_function_counts.get(crate_name) {
                let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
                let node_id = crate_name.replace('-', "_");
                let module_count = crate_modules.get(crate_name).map(|v| v.len()).unwrap_or(0);

                dot.push_str(&format!(
                    "    {} [label=\"{}\\\\n{} modules, {} functions\", fillcolor=\"{}\"];\n",
                    node_id, crate_name, module_count, function_count, color
                ));
            }
        }
        dot.push_str("  }\n\n");

        // Analysis Layer (subgraph cluster)
        dot.push_str("  subgraph cluster_analysis {\n");
        dot.push_str("    label=\"Analysis & Transformation\";\n");
        dot.push_str("    style=filled;\n");
        dot.push_str("    fillcolor=\"#fef5e7\";\n");
        dot.push_str("    color=\"#d4a373\";\n");
        dot.push_str("    penwidth=2;\n");

        for crate_name in ["codehud-analysis", "codehud-transform"] {
            if let Some(&function_count) = crate_function_counts.get(crate_name) {
                let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
                let node_id = crate_name.replace('-', "_");
                let module_count = crate_modules.get(crate_name).map(|v| v.len()).unwrap_or(0);

                dot.push_str(&format!(
                    "    {} [label=\"{}\\\\n{} modules, {} functions\", fillcolor=\"{}\"];\n",
                    node_id, crate_name, module_count, function_count, color
                ));
            }
        }
        dot.push_str("  }\n\n");

        // Application Layer (subgraph cluster)
        dot.push_str("  subgraph cluster_interfaces {\n");
        dot.push_str("    label=\"User Interfaces\";\n");
        dot.push_str("    style=filled;\n");
        dot.push_str("    fillcolor=\"#e8f5e9\";\n");
        dot.push_str("    color=\"#66bb6a\";\n");
        dot.push_str("    penwidth=2;\n");

        for crate_name in ["codehud-cli", "codehud-gui", "codehud-tui"] {
            if let Some(&function_count) = crate_function_counts.get(crate_name) {
                let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
                let node_id = crate_name.replace('-', "_");
                let module_count = crate_modules.get(crate_name).map(|v| v.len()).unwrap_or(0);

                dot.push_str(&format!(
                    "    {} [label=\"{}\\\\n{} modules, {} functions\", fillcolor=\"{}\"];\n",
                    node_id, crate_name, module_count, function_count, color
                ));
            }
        }
        dot.push_str("  }\n\n");

        // Extension Layer (subgraph cluster)
        dot.push_str("  subgraph cluster_extensions {\n");
        dot.push_str("    label=\"Extensions & Visualization\";\n");
        dot.push_str("    style=filled;\n");
        dot.push_str("    fillcolor=\"#f3e5f5\";\n");
        dot.push_str("    color=\"#ab47bc\";\n");
        dot.push_str("    penwidth=2;\n");

        for crate_name in ["codehud-llm", "codehud-viz"] {
            if let Some(&function_count) = crate_function_counts.get(crate_name) {
                let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
                let node_id = crate_name.replace('-', "_");
                let module_count = crate_modules.get(crate_name).map(|v| v.len()).unwrap_or(0);

                dot.push_str(&format!(
                    "    {} [label=\"{}\\\\n{} modules, {} functions\", fillcolor=\"{}\"];\n",
                    node_id, crate_name, module_count, function_count, color
                ));
            }
        }
        dot.push_str("  }\n\n");

        // Rank constraints for hierarchical layout
        dot.push_str("  // Layer separation\n");
        dot.push_str("  {rank=same; codehud_utils; codehud_core;}\n");
        dot.push_str("  {rank=same; codehud_analysis; codehud_transform;}\n");
        dot.push_str("  {rank=same; codehud_cli; codehud_gui; codehud_tui;}\n");
        dot.push_str("  {rank=same; codehud_llm; codehud_viz;}\n\n");

        dot.push_str("  // Inter-crate dependencies\n");

        // Add dependency edges
        for (from_crate, targets) in crate_deps.iter() {
            let from_id = from_crate.replace('-', "_");

            for (to_crate, call_count) in targets {
                let to_id = to_crate.replace('-', "_");

                // Calculate edge weight
                let penwidth = 1.0 + ((*call_count as f64).log10() + 1.0).min(4.0);

                // Only show labels for heavy dependencies
                if *call_count >= 10 {
                    dot.push_str(&format!(
                        "  {} -> {} [label=\"{} calls\", penwidth=\"{}\"];\n",
                        from_id, to_id, call_count, penwidth
                    ));
                } else {
                    dot.push_str(&format!(
                        "  {} -> {} [penwidth=\"{}\"];\n",
                        from_id, to_id, penwidth
                    ));
                }
            }
        }

        dot.push_str("}\n");
        dot
    }

    /// Export module-level overview graph (polyglot, uses DependencyExtractor)
    pub fn export_overview_graph(&self, graph: &Graph<CallNode, CallEdge, Directed>, codebase_path: &Path) -> String {
        // Use polyglot dependency extractor for any language
        self.export_polyglot_dependency_graph(graph, codebase_path)
    }

    /// Export polyglot dependency graph using DependencyExtractor (works for 17+ languages)
    pub fn export_polyglot_dependency_graph(&self, graph: &Graph<CallNode, CallEdge, Directed>, codebase_path: &Path) -> String {
        use std::fs;
        use std::collections::HashSet;
        use codehud_core::extractors::dependencies::DependenciesExtractor;
        use codehud_core::extractors::BaseDataExtractor;

        // Check if this is a Rust workspace by looking for Cargo.toml files
        let is_rust_workspace = self.detect_rust_workspace(codebase_path);

        if is_rust_workspace {
            // For Rust workspaces, use Cargo.toml parsing (source of truth for Rust)
            return self.export_cargo_dependency_graph(graph);
        }

        // For non-Rust projects, use DependencyExtractor
        let extractor = match DependenciesExtractor::new(codebase_path) {
            Ok(e) => e,
            Err(_) => {
                // Fallback to Cargo-specific if extractor fails
                return self.export_cargo_dependency_graph(graph);
            }
        };

        let dep_data = match extractor.extract_data() {
            Ok(data) => data,
            Err(_) => {
                // Fallback to Cargo-specific if extraction fails
                return self.export_cargo_dependency_graph(graph);
            }
        };

        // Check if DependencyExtractor returned useful data
        let has_dependencies = dep_data.get("summary")
            .and_then(|s| s.get("total_import_statements"))
            .and_then(|t| t.as_u64())
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_dependencies {
            // DependencyExtractor found no imports - generate error message graph
            eprintln!("⚠️  Warning: DependencyExtractor found 0 import statements in non-Rust project");
            eprintln!("    This may indicate that import detection is not working for this language");

            let mut dot = String::from("digraph DependencyError {\n");
            dot.push_str("  graph [rankdir=TB, bgcolor=\"#f8f9fa\"];\n");
            dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica\"];\n");
            dot.push_str("  error [label=\"⚠️ No dependencies detected\\n\\nDependencyExtractor found 0 import statements.\\nThis project may not be supported yet.\", fillcolor=\"#ffcdd2\", fontsize=14];\n");
            dot.push_str("}\n");
            return dot;
        }

        // Extract the data we need
        let file_deps = dep_data.get("file_dependencies")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().collect::<Vec<_>>())
            .unwrap_or_default();

        let external_deps_data = dep_data.get("external_dependencies")
            .and_then(|v| v.as_object())
            .unwrap_or(&serde_json::Map::new());

        // Group files into modules based on directory structure
        let modules = self.group_files_into_modules(&file_deps, &codebase_path);

        // Extract dependencies between modules
        let (module_deps, module_external_deps) = self.extract_module_dependencies(
            &file_deps,
            &modules,
            &codebase_path
        );

        // Generate DOT using the same layout as the Cargo version
        self.generate_dependency_dot(
            &modules,
            &module_deps,
            &module_external_deps,
            &codebase_path
        )
    }

    /// Detect if this is a Rust workspace by checking for Cargo.toml files
    fn detect_rust_workspace(&self, codebase_path: &Path) -> bool {
        use std::fs;

        // Check for root Cargo.toml
        if codebase_path.join("Cargo.toml").exists() {
            return true;
        }

        // Check for any Cargo.toml in subdirectories (workspace members)
        if let Ok(entries) = fs::read_dir(codebase_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("Cargo.toml").exists() {
                    return true;
                }
            }
        }

        false
    }

    /// Group files into modules based on directory structure (language-agnostic)
    fn group_files_into_modules(
        &self,
        file_deps: &[(&String, &serde_json::Value)],
        codebase_path: &Path,
    ) -> HashMap<String, (Vec<String>, usize, usize)> {
        use std::collections::HashSet;

        let mut modules: HashMap<String, HashSet<String>> = HashMap::new();
        let mut module_functions: HashMap<String, usize> = HashMap::new();

        // Group files by their top-level directory (module/package/crate)
        for (file_path, _) in file_deps {
            let path = Path::new(file_path);

            // Determine the module name from the path
            let module_name = if let Some(first_component) = path.components().next() {
                first_component.as_os_str().to_string_lossy().to_string()
            } else {
                "root".to_string()
            };

            modules.entry(module_name.clone())
                .or_insert_with(HashSet::new)
                .insert(file_path.to_string());

            // Estimate function count from file (rough heuristic)
            *module_functions.entry(module_name).or_insert(0) += 5;
        }

        // Convert to final format: module_name -> (files, file_count, function_count)
        modules.into_iter()
            .map(|(module_name, files)| {
                let file_list: Vec<String> = files.into_iter().collect();
                let file_count = file_list.len();
                let func_count = *module_functions.get(&module_name).unwrap_or(&0);
                (module_name, (file_list, file_count, func_count))
            })
            .collect()
    }

    /// Extract module-level dependencies from file-level dependencies (language-agnostic)
    fn extract_module_dependencies(
        &self,
        file_deps: &[(&String, &serde_json::Value)],
        modules: &HashMap<String, (Vec<String>, usize, usize)>,
        _codebase_path: &Path,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        use std::collections::HashSet;

        // Build reverse lookup: file_path -> module_name
        let mut file_to_module: HashMap<String, String> = HashMap::new();
        for (module_name, (files, _, _)) in modules {
            for file in files {
                file_to_module.insert(file.clone(), module_name.clone());
            }
        }

        // Track internal and external dependencies per module
        let mut module_internal_deps: HashMap<String, HashSet<String>> = HashMap::new();
        let mut module_external_deps: HashMap<String, HashSet<String>> = HashMap::new();

        // Process each file's dependencies
        for (file_path, deps_value) in file_deps {
            let module_name = match file_to_module.get(*file_path) {
                Some(name) => name.clone(),
                None => continue,
            };

            // Extract imports from this file
            let imports = deps_value.get("imports")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            for import in imports {
                // Determine if this import is internal (from our codebase) or external (third-party)
                let is_internal = self.is_import_internal(import, &file_to_module);

                if is_internal {
                    // Try to find which module this import belongs to
                    if let Some(target_module) = self.resolve_import_to_module(import, &file_to_module) {
                        // Only add if it's a different module (not self-reference)
                        if target_module != module_name {
                            module_internal_deps.entry(module_name.clone())
                                .or_insert_with(HashSet::new)
                                .insert(target_module);
                        }
                    }
                } else {
                    // External dependency (third-party library)
                    // Extract the base package name (e.g., "std::fs" -> "std", "tokio::runtime" -> "tokio")
                    let base_dep = import.split("::").next()
                        .or_else(|| import.split('.').next())
                        .or_else(|| import.split('/').next())
                        .unwrap_or(import);

                    module_external_deps.entry(module_name.clone())
                        .or_insert_with(HashSet::new)
                        .insert(base_dep.to_string());
                }
            }
        }

        // Convert HashSets to sorted Vecs
        let module_deps: HashMap<String, Vec<String>> = module_internal_deps.into_iter()
            .map(|(k, v)| {
                let mut sorted: Vec<String> = v.into_iter().collect();
                sorted.sort();
                (k, sorted)
            })
            .collect();

        let module_ext_deps: HashMap<String, Vec<String>> = module_external_deps.into_iter()
            .map(|(k, v)| {
                let mut sorted: Vec<String> = v.into_iter().collect();
                sorted.sort();
                (k, sorted)
            })
            .collect();

        (module_deps, module_ext_deps)
    }

    /// Check if an import is internal to the codebase
    fn is_import_internal(&self, import: &str, file_to_module: &HashMap<String, String>) -> bool {
        // Heuristic: imports starting with project-specific prefixes are internal
        // For Rust: codehud-*, crate::, self::, super::
        // For Python: codehud.*, relative imports (.)
        // For JS/TS: ./ or ../

        if import.starts_with("codehud") {
            return true;
        }

        if import.starts_with("crate::") || import.starts_with("self::") || import.starts_with("super::") {
            return true;
        }

        if import.starts_with('.') || import.starts_with("./") || import.starts_with("../") {
            return true;
        }

        // Check if any file in our codebase matches this import
        file_to_module.keys().any(|file| {
            file.contains(import) || import.contains(file)
        })
    }

    /// Resolve an import to a module name
    fn resolve_import_to_module(&self, import: &str, file_to_module: &HashMap<String, String>) -> Option<String> {
        // Try to match the import to a module
        // For Rust: "codehud_core::extractors" -> "codehud-core"
        // For Python: "codehud.core.extractors" -> "codehud-core"

        // Extract first component
        let first_part = import.split("::").next()
            .or_else(|| import.split('.').next())
            .or_else(|| import.split('/').next())
            .unwrap_or(import);

        // Look for modules matching this component
        for file in file_to_module.keys() {
            if file.contains(first_part) {
                return file_to_module.get(file).cloned();
            }
        }

        // Try to map directly (e.g., "codehud_core" -> "codehud-core")
        let normalized = first_part.replace('_', "-");
        Some(normalized)
    }

    /// Generate DOT output using the perfected layout (external deps at top, invisible edges, etc.)
    fn generate_dependency_dot(
        &self,
        modules: &HashMap<String, (Vec<String>, usize, usize)>,
        module_deps: &HashMap<String, Vec<String>>,
        module_external_deps: &HashMap<String, Vec<String>>,
        _codebase_path: &Path,
    ) -> String {
        let mut dot = String::from("digraph ModuleArchitecture {\n");
        dot.push_str("  graph [rankdir=TB, splines=polyline, nodesep=0.5, ranksep=2.5, newrank=true, bgcolor=\"#f8f9fa\", dpi=96];\n");
        dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica Bold\", fontsize=22, height=1.5, width=5.0];\n");
        dot.push_str("  edge [fontname=\"Helvetica Bold\", fontsize=18, penwidth=2.5, arrowsize=1.2];\n\n");

        // Color schemes
        let module_colors = vec![
            "#3498db", "#c8e6c9", "#ffe0b2", "#e1bee7", "#ffcdd2",
            "#29b6f6", "#dcedc8", "#f9a825", "#f0f4c3", "#4fc3f7",
        ];

        let edge_colors = vec![
            "#1976d2", "#388e3c", "#f57c00", "#7b1fa2", "#c62828",
            "#0097a7", "#689f38", "#fbc02d", "#5d4037", "#0288d1",
        ];

        // Create external dependency boxes (one per module)
        let mut ext_dep_box_ids: Vec<String> = Vec::new();

        dot.push_str("  // External dependency boxes (one per module)\n");
        for (module_name, ext_deps) in module_external_deps {
            if !ext_deps.is_empty() {
                let box_id = format!("ext_deps_{}", module_name.replace('-', "_").replace('.', "_"));
                ext_dep_box_ids.push(box_id.clone());

                dot.push_str(&format!("  {} [shape=box, label=\"{} External Deps\\n\\n",
                    box_id, module_name));

                for ext_dep in ext_deps {
                    dot.push_str(&format!("• {}\\n", ext_dep));
                }

                dot.push_str("\", fillcolor=\"#ffcdd2\", fontsize=10, width=4.0, height=2.0];\n");
            }
        }

        // Group all external dependency boxes at the top (rank=min)
        if !ext_dep_box_ids.is_empty() {
            dot.push_str("  { rank=min; ");
            for box_id in &ext_dep_box_ids {
                dot.push_str(&format!("{}; ", box_id));
            }
            dot.push_str("}\n");
        }
        dot.push_str("\n");

        // Create internal module boxes
        dot.push_str("  // Internal modules\n");
        for (idx, (module_name, (files, file_count, func_count))) in modules.iter().enumerate() {
            let node_id = module_name.replace('-', "_").replace('.', "_");
            let color = module_colors[idx % module_colors.len()];

            // Show top files in the module
            let mut label = format!("{}\\n{} files, {} functions", module_name, file_count, func_count);

            if !files.is_empty() && files.len() <= 8 {
                label.push_str("\\n\\n");
                for file in files.iter().take(8) {
                    // Shorten file paths for display
                    let short_file = file.split('/').last().unwrap_or(file);
                    label.push_str(&format!("{}\\n", short_file));
                }
                if files.len() > 8 {
                    label.push_str(&format!("... {} more", files.len() - 8));
                }
            }

            dot.push_str(&format!("  {} [label=\"{}\", fillcolor=\"{}\"];\n",
                node_id, label, color));
        }
        dot.push_str("\n");

        // Add invisible structural edges from external dep boxes to their modules
        dot.push_str("  // Invisible edges to enforce layout (external deps above internal modules)\n");
        for module_name in module_external_deps.keys() {
            let ext_deps_box_id = format!("ext_deps_{}", module_name.replace('-', "_").replace('.', "_"));
            let module_id = module_name.replace('-', "_").replace('.', "_");
            dot.push_str(&format!("  {} -> {} [style=invis, weight=10];\n",
                ext_deps_box_id, module_id));
        }
        dot.push_str("\n");

        // Add internal dependency edges (module to module)
        dot.push_str("  // Inter-module dependencies\n");
        for (idx, (from_module, to_modules)) in module_deps.iter().enumerate() {
            let from_id = from_module.replace('-', "_").replace('.', "_");
            let edge_color = edge_colors[idx % edge_colors.len()];

            for to_module in to_modules {
                let to_id = to_module.replace('-', "_").replace('.', "_");
                dot.push_str(&format!("  {} -> {} [color=\"{}\", penwidth=2.5, weight=2];\n",
                    from_id, to_id, edge_color));
            }
        }
        dot.push_str("\n");

        // Add visible edges to external dependency boxes
        dot.push_str("  // Edges to external dependency boxes\n");
        for module_name in module_external_deps.keys() {
            let module_id = module_name.replace('-', "_").replace('.', "_");
            let ext_deps_box_id = format!("ext_deps_{}", module_name.replace('-', "_").replace('.', "_"));
            dot.push_str(&format!("  {} -> {} [style=dashed, color=\"black\", penwidth=0.8, constraint=false];\n",
                module_id, ext_deps_box_id));
        }

        dot.push_str("}\n");
        dot
    }

    /// Export clean architecture graph from Cargo.toml dependencies (compile-time)
    pub fn export_cargo_dependency_graph(&self, graph: &Graph<CallNode, CallEdge, Directed>) -> String {
        use std::fs;
        use std::path::Path;
        use std::collections::HashSet;

        // Find all Cargo.toml files
        let crate_names = vec![
            "codehud-core", "codehud-cli", "codehud-gui", "codehud-tui",
            "codehud-llm", "codehud-viz", "codehud-analysis", "codehud-transform",
            "codehud-utils", "codehud-realtime"
        ];

        // Parse dependencies from Cargo.toml files - include BOTH internal and external deps
        let mut cargo_deps: HashMap<String, Vec<String>> = HashMap::new();
        let mut external_deps: HashSet<String> = HashSet::new();

        for crate_name in &crate_names {
            let cargo_path = Path::new(crate_name).join("Cargo.toml");
            if let Ok(content) = fs::read_to_string(&cargo_path) {
                let mut deps = Vec::new();
                let mut in_deps_section = false;
                let mut in_features_block = false;

                for line in content.lines() {
                    let trimmed = line.trim();

                    // Detect [dependencies] section
                    if trimmed == "[dependencies]" {
                        in_deps_section = true;
                        continue;
                    }

                    // Stop at next section
                    if in_deps_section && trimmed.starts_with('[') {
                        break;
                    }

                    // Extract ALL dependencies (not just codehud-*)
                    if in_deps_section && !trimmed.is_empty() && !trimmed.starts_with('#') {
                        // Check if we're entering a features block
                        if trimmed.contains("features") && trimmed.contains('[') {
                            in_features_block = true;
                        }

                        // Check if we're exiting a features block
                        if in_features_block && trimmed.contains(']') {
                            in_features_block = false;
                            // Still process the line if it contains = (actual dep line)
                            if !trimmed.contains('=') {
                                continue;
                            }
                        }

                        // Skip lines inside features blocks that don't have '='
                        if in_features_block && !trimmed.contains('=') {
                            continue;
                        }

                        // Extract dependency name (first word before '=' or whitespace)
                        if let Some(dep_name) = trimmed.split(&['=', ' '][..]).next() {
                            if !dep_name.is_empty() && dep_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                                let dep_clean = dep_name.to_string();
                                deps.push(dep_clean.clone());
                                // Track external dependencies (non-codehud)
                                if !dep_clean.starts_with("codehud-") {
                                    external_deps.insert(dep_clean);
                                }
                            }
                        }
                    }
                }

                if !deps.is_empty() {
                    cargo_deps.insert(crate_name.to_string(), deps);
                }
            }
        }

        // Scan for major modules (src/ subdirectories) for each crate
        let mut crate_modules: HashMap<String, Vec<String>> = HashMap::new();

        for crate_name in &crate_names {
            let src_path = Path::new(crate_name).join("src");
            if let Ok(entries) = fs::read_dir(&src_path) {
                let mut modules = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            modules.push(format!("src/{}", dir_name));
                        }
                    }
                }
                if !modules.is_empty() {
                    crate_modules.insert(crate_name.to_string(), modules);
                }
            }
        }

        // Count modules (files) and functions per crate from filesystem
        let mut crate_metrics: HashMap<String, (usize, usize)> = HashMap::new();

        for crate_name in &crate_names {
            let mut file_count = 0;
            let mut function_count = 0;

            // Count .rs files recursively
            if let Ok(entries) = fs::read_dir(Path::new(crate_name).join("src")) {
                fn count_files_recursive(dir: &Path, file_count: &mut usize, function_count: &mut usize) {
                    if let Ok(entries) = fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                count_files_recursive(&path, file_count, function_count);
                            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                                *file_count += 1;
                                // Count function definitions
                                if let Ok(content) = fs::read_to_string(&path) {
                                    *function_count += content.lines()
                                        .filter(|line| {
                                            let trimmed = line.trim();
                                            trimmed.starts_with("fn ") ||
                                            trimmed.starts_with("pub fn ") ||
                                            trimmed.starts_with("async fn ") ||
                                            trimmed.starts_with("pub async fn ")
                                        })
                                        .count();
                                }
                            }
                        }
                    }
                }
                count_files_recursive(&Path::new(crate_name).join("src"), &mut file_count, &mut function_count);
            }

            crate_metrics.insert(crate_name.to_string(), (file_count, function_count));
        }

        // Generate DOT with hierarchical clustering and nested modules
        let mut dot = String::from("digraph CrateArchitecture {\n");
        dot.push_str("  graph [rankdir=TB, splines=polyline, nodesep=0.5, ranksep=2.5, newrank=true, bgcolor=\"#f8f9fa\", dpi=96];\n");
        dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica Bold\", fontsize=22, height=1.5, width=5.0];\n");
        dot.push_str("  edge [fontname=\"Helvetica Bold\", fontsize=18, penwidth=2.5, arrowsize=1.2];\n\n");

        // Define crate colors (for nodes)
        let crate_colors: HashMap<&str, &str> = [
            ("codehud-core", "#3498db"),       // Darker blue (was light blue #a7d9ef)
            ("codehud-cli", "#c8e6c9"),
            ("codehud-gui", "#ffe0b2"),
            ("codehud-llm", "#e1bee7"),
            ("codehud-transform", "#ffcdd2"),
            ("codehud-viz", "#29b6f6"),        // Darker blue (was light blue #b3e5fc)
            ("codehud-analysis", "#dcedc8"),
            ("codehud-utils", "#f9a825"),      // Darker yellow (was light yellow #fff9c4)
            ("codehud-tui", "#f0f4c3"),
            ("codehud-realtime", "#4fc3f7"),   // Darker blue (was light blue #e1f5fe)
        ].iter().cloned().collect();

        // Define edge colors (distinct, saturated colors for each crate's dependencies)
        let edge_colors: HashMap<&str, &str> = [
            ("codehud-core", "#1976d2"),      // Blue
            ("codehud-cli", "#388e3c"),       // Green
            ("codehud-gui", "#f57c00"),       // Orange
            ("codehud-llm", "#7b1fa2"),       // Purple
            ("codehud-transform", "#c62828"), // Red
            ("codehud-viz", "#0097a7"),       // Cyan
            ("codehud-analysis", "#689f38"),  // Light Green
            ("codehud-utils", "#fbc02d"),     // Yellow
            ("codehud-tui", "#5d4037"),       // Brown
            ("codehud-realtime", "#0288d1"),  // Light Blue
        ].iter().cloned().collect();

        // Helper function to create simple crate nodes (no nested modules to avoid horizontal spread)
        fn add_crate_node(
            dot: &mut String,
            crate_name: &str,
            modules: &[String],
            metrics: &(usize, usize),
            color: &str,
        ) {
            let node_id = crate_name.replace('-', "_");
            let (file_count, func_count) = metrics;

            // List top modules in label (vertical list)
            let mut label = format!("{}\\n{} files, {} functions", crate_name, file_count, func_count);

            if !modules.is_empty() {
                label.push_str("\\n\\n");
                // Show first 8 modules in the label, vertically
                for (i, module) in modules.iter().take(8).enumerate() {
                    let module_display = module.replace("src/", "");
                    label.push_str(&format!("{}\\n", module_display));
                }
                if modules.len() > 8 {
                    label.push_str(&format!("... {} more", modules.len() - 8));
                }
            }

            dot.push_str(&format!(
                "  {} [label=\"{}\", fillcolor=\"{}\"];\n",
                node_id, label, color
            ));
        }

        // External Dependencies - create separate boxes for each crate's external deps
        // Collect all external dep box IDs for rank grouping
        let mut ext_dep_box_ids: Vec<String> = Vec::new();

        dot.push_str("  // External dependency boxes (one per crate)\n");
        for crate_name in &crate_names {
            if let Some(deps) = cargo_deps.get(*crate_name) {
                // Get only external dependencies for this crate
                let mut ext_deps_for_crate: Vec<&String> = deps.iter()
                    .filter(|d| !d.starts_with("codehud-"))
                    .collect();

                if !ext_deps_for_crate.is_empty() {
                    ext_deps_for_crate.sort();

                    let box_id = format!("ext_deps_{}", crate_name.replace('-', "_"));
                    ext_dep_box_ids.push(box_id.clone());

                    // Create compact box with bullet-pointed list
                    dot.push_str(&format!("  {} [shape=box, label=\"{} External Deps\\n\\n",
                        box_id, crate_name));

                    for ext_dep in &ext_deps_for_crate {
                        dot.push_str(&format!("• {}\\n", ext_dep));
                    }

                    dot.push_str("\", fillcolor=\"#ffcdd2\", fontsize=10, width=4.0, height=2.0];\n");
                }
            }
        }

        // Group all external dependency boxes at the top/side (min rank)
        if !ext_dep_box_ids.is_empty() {
            dot.push_str("  { rank=min; ");
            for box_id in &ext_dep_box_ids {
                dot.push_str(&format!("{}; ", box_id));
            }
            dot.push_str("}\n");
        }
        dot.push_str("\n");

        // Internal crates with rank constraints for vertical layout
        dot.push_str("  // Internal crates (stacked vertically)\n");

        // Foundation layer
        for crate_name in ["codehud-utils"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        for crate_name in ["codehud-core"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        // Analysis layer
        for crate_name in ["codehud-analysis"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        for crate_name in ["codehud-transform"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        // Interface layer
        for crate_name in ["codehud-cli"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        for crate_name in ["codehud-gui"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        for crate_name in ["codehud-tui"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        // Extensions layer
        for crate_name in ["codehud-llm"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        for crate_name in ["codehud-viz"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        for crate_name in ["codehud-realtime"] {
            let color = crate_colors.get(crate_name).unwrap_or(&"#e0e0e0");
            let modules = crate_modules.get(crate_name).cloned().unwrap_or_default();
            let metrics = crate_metrics.get(crate_name).unwrap_or(&(0, 0));
            add_crate_node(&mut dot, crate_name, &modules, metrics, color);
        }

        // Add invisible structural edges from external dependency boxes to corresponding internal crates
        // This forces internal crates to be placed below their external dependency boxes
        dot.push_str("  // Invisible edges to enforce layout (external deps above internal crates)\n");
        for crate_name in &crate_names {
            if let Some(deps) = cargo_deps.get(*crate_name) {
                let has_external_deps = deps.iter().any(|d| !d.starts_with("codehud-"));
                if has_external_deps {
                    let ext_deps_box_id = format!("ext_deps_{}", crate_name.replace('-', "_"));
                    let crate_id = crate_name.replace('-', "_");
                    // Invisible edge with high weight to force vertical separation
                    dot.push_str(&format!("  {} -> {} [style=invis, weight=10];\n",
                        ext_deps_box_id, crate_id));
                }
            }
        }
        dot.push_str("\n");

        // Add dependency edges (Cargo.toml dependencies) with color coding
        dot.push_str("  // Inter-crate dependencies from Cargo.toml\n");
        for (from_crate, deps) in cargo_deps.iter() {
            let from_id = from_crate.replace('-', "_");
            let edge_color = edge_colors.get(from_crate.as_str()).unwrap_or(&"#555555");

            for to_dep in deps {
                if to_dep.starts_with("codehud-") {
                    // Internal dependency - solid line with crate-specific color, enforces layout
                    let to_id = to_dep.replace('-', "_");
                    dot.push_str(&format!("  {} -> {} [color=\"{}\", penwidth=2.5, weight=2];\n",
                        from_id, to_id, edge_color));
                }
            }
        }

        // Visible edges to external dependency boxes (drawn last to appear on top)
        dot.push_str("  // Edges to external dependency boxes\n");
        for (from_crate, deps) in cargo_deps.iter() {
            let has_external_deps = deps.iter().any(|d| !d.starts_with("codehud-"));
            if has_external_deps {
                let from_id = from_crate.replace('-', "_");
                let ext_deps_box_id = format!("ext_deps_{}", from_crate.replace('-', "_"));
                dot.push_str(&format!("  {} -> {} [style=dashed, color=\"black\", penwidth=0.8, constraint=false];\n",
                    from_id, ext_deps_box_id));
            }
        }

        dot.push_str("}\n");
        dot
    }

    /// Export per-module detail graph
    pub fn export_module_graph(
        &self,
        graph: &Graph<CallNode, CallEdge, Directed>,
        module_name: &str,
        module_nodes: &[NodeIndex],
    ) -> String {
        let subgraph = extract_module_subgraph(graph, module_nodes);

        let mut dot = String::from(&format!("digraph Module_{} {{\n", self.sanitize_cluster_name(module_name)));
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica\"];\n");
        dot.push_str("  edge [fontname=\"Helvetica\", fontsize=10];\n");

        // Auto-detect splines based on subgraph size
        let (_, splines, concentrate) = Self::auto_detect_options(subgraph.node_count(), subgraph.edge_count());
        dot.push_str(&format!("  splines={};\n", splines));
        if concentrate {
            dot.push_str("  concentrate=true;\n");
        }
        dot.push_str("\n");

        // Detect cycles in subgraph
        let cycles = if self.show_cycles {
            self.detect_cycles(&subgraph)
        } else {
            vec![]
        };

        // Export nodes with importance-based coloring
        for (idx, node) in subgraph.node_references() {
            let color = if self.color_by_complexity {
                self.get_node_color(&subgraph, idx)
            } else {
                "lightblue"
            };

            let label = self.escape_dot_label(&node.function_name);
            dot.push_str(&format!(
                "  n{} [label=\"{}\", fillcolor=\"{}\"];\n",
                idx.index(),
                label,
                color
            ));
        }

        dot.push_str("\n");

        // Export edges
        for edge in subgraph.edge_references() {
            let source = edge.source().index();
            let target = edge.target().index();
            let weight = edge.weight().call_count;

            let in_cycle = self.is_edge_in_cycle(source, target, &cycles);
            let color = if in_cycle { "red" } else { "blue" };
            let penwidth = if in_cycle { "2.0" } else { "1.0" };

            if self.show_edge_weights && weight > 1 {
                dot.push_str(&format!(
                    "  n{} -> n{} [label=\"{}x\", color=\"{}\", penwidth=\"{}\"];\n",
                    source, target, weight, color, penwidth
                ));
            } else {
                dot.push_str(&format!(
                    "  n{} -> n{} [color=\"{}\", penwidth=\"{}\"];\n",
                    source, target, color, penwidth
                ));
            }
        }

        dot.push_str("}\n");
        dot
    }

    /// Export cycle-only graph (strongly connected components)
    pub fn export_cycles_graph(&self, graph: &Graph<CallNode, CallEdge, Directed>) -> String {
        let analysis = analyze_graph(graph);

        if analysis.sccs.is_empty() {
            // No cycles found - return minimal graph
            let mut dot = String::from("digraph Cycles {\n");
            dot.push_str("  label=\"No cycles detected\";\n");
            dot.push_str("  node [shape=box];\n");
            dot.push_str("  success [label=\"✓ No circular dependencies found\", fillcolor=\"lightgreen\", style=\"filled\"];\n");
            dot.push_str("}\n");
            return dot;
        }

        let cycle_graph = extract_cycle_subgraph(graph, &analysis.sccs);

        let mut dot = String::from("digraph Cycles {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=\"filled,rounded\", fontname=\"Helvetica\"];\n");
        dot.push_str("  edge [fontname=\"Helvetica\", fontsize=10, color=\"red\", penwidth=\"2.0\"];\n");
        dot.push_str("  splines=spline;\n\n");

        // Group by SCC
        for (scc_idx, scc) in analysis.sccs.iter().enumerate() {
            dot.push_str(&format!("\n  subgraph cluster_scc_{} {{\n", scc_idx));
            dot.push_str(&format!("    label=\"Cycle {} ({} functions)\";\n", scc_idx + 1, scc.size));
            dot.push_str("    style=dashed;\n");
            dot.push_str("    color=red;\n");
            dot.push_str("    bgcolor=\"#ffe0e0\";\n");

            for &node_idx in &scc.nodes {
                if let Some(original_node) = graph.node_weight(node_idx) {
                    dot.push_str(&format!(
                        "    n{} [label=\"{}\", fillcolor=\"salmon\"];\n",
                        node_idx.index(),
                        self.escape_dot_label(&original_node.function_name)
                    ));
                }
            }

            dot.push_str("  }\n");
        }

        dot.push_str("\n");

        // Export cycle edges
        for edge in cycle_graph.edge_references() {
            let source = edge.source().index();
            let target = edge.target().index();
            dot.push_str(&format!("  n{} -> n{};\n", source, target));
        }

        dot.push_str("}\n");
        dot
    }
}

impl Default for DotExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codehud_core::graph::{CallNode, CallEdge};

    #[test]
    fn test_export_empty_graph() {
        let graph: Graph<CallNode, CallEdge, Directed> = Graph::new();
        let exporter = DotExporter::new();
        let dot = exporter.export_call_graph(&graph);

        assert!(dot.starts_with("digraph CallGraph {"));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn test_export_simple_graph() {
        let mut graph: Graph<CallNode, CallEdge, Directed> = Graph::new();

        let main_node = graph.add_node(CallNode {
            function_name: "main".to_string(),
            file_path: "main.rs".to_string(),
            line_number: 1,
        });

        let helper_node = graph.add_node(CallNode {
            function_name: "helper".to_string(),
            file_path: "main.rs".to_string(),
            line_number: 10,
        });

        graph.add_edge(main_node, helper_node, CallEdge {
            call_count: 3,
            weight: 3.0,
        });

        let exporter = DotExporter::new();
        let dot = exporter.export_call_graph(&graph);

        assert!(dot.contains("main"));
        assert!(dot.contains("helper"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph: Graph<CallNode, CallEdge, Directed> = Graph::new();

        let a = graph.add_node(CallNode::new("a".to_string(), "test.rs".to_string(), 1));
        let b = graph.add_node(CallNode::new("b".to_string(), "test.rs".to_string(), 2));
        let c = graph.add_node(CallNode::new("c".to_string(), "test.rs".to_string(), 3));

        // Create cycle: a -> b -> c -> a
        graph.add_edge(a, b, CallEdge::new(1));
        graph.add_edge(b, c, CallEdge::new(1));
        graph.add_edge(c, a, CallEdge::new(1));

        let exporter = DotExporter::new();
        let cycles = exporter.detect_cycles(&graph);

        assert!(!cycles.is_empty(), "Should detect cycle");
    }

    #[test]
    fn test_label_escaping() {
        let exporter = DotExporter::new();
        let escaped = exporter.escape_dot_label("test\"quote\\slash\nnewline");
        assert!(escaped.contains("\\\""));
        assert!(escaped.contains("\\\\"));
        assert!(escaped.contains("\\n"));
    }
}
