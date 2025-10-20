//! Flow Data Extractor - Analyzes data flow patterns in Python codebases

use super::BaseDataExtractor;
use crate::external_tools::ExternalToolManager;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use tree_sitter::{Language, Parser};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::fs;

extern "C" {
    fn tree_sitter_rust() -> Language;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowNode {
    node_id: String,
    node_type: String, // function, variable, class, module
    file_path: String,
    line_number: usize,
    name: String,
    scope: String,
    data_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowEdge {
    from_node: String,
    to_node: String,
    edge_type: String, // assignment, function_call, parameter, return_value
    file_path: String,
    line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowPattern {
    pattern_type: String,
    description: String,
    files_involved: Vec<String>,
    confidence: f64,
    impact_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VariableLifecycle {
    variable_name: String,
    file_path: String,
    creation_line: usize,
    modifications: Vec<usize>,
    last_usage: usize,
    scope_depth: usize,
    is_global: bool,
}

pub struct FlowExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
}

impl FlowExtractor {
    pub fn new(codebase_path: impl AsRef<Path>) -> crate::Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();
        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!("Codebase path does not exist: {}", codebase_path.display())));
        }

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Config(format!("Failed to set language: {}", e)))?;

        let external_tools = ExternalToolManager::new(&codebase_path);

        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
            parser,
            external_tools,
        })
    }

    fn get_all_python_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_files_recursive(&self.codebase_path, &mut files);
        files
    }

    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                    files.push(path);
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    self.collect_files_recursive(&path, files);
                }
            }
        }
    }

    fn is_excluded_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "__pycache__" | ".git" | ".pytest_cache" | "node_modules" | ".venv" | "venv")
        } else {
            false
        }
    }

    fn analyze_data_flow(&self, files: &[PathBuf]) -> crate::Result<(Vec<DataFlowNode>, Vec<DataFlowEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_counter = 0;

        for file_path in files {
            if let Ok((file_nodes, file_edges)) = self.analyze_file_data_flow(file_path, &mut node_counter) {
                nodes.extend(file_nodes);
                edges.extend(file_edges);
            }
        }

        Ok((nodes, edges))
    }

    fn analyze_file_data_flow(&self, file_path: &Path, node_counter: &mut usize) -> crate::Result<(Vec<DataFlowNode>, Vec<DataFlowEdge>)> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io(e))?;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Analysis(format!("Failed to set language: {}", e)))?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Analysis("Failed to parse file".to_string()))?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut variable_assignments = HashMap::new();
        let mut function_calls = Vec::new();

        self.extract_flow_elements(
            tree.root_node(),
            &content,
            file_path,
            &mut nodes,
            &mut edges,
            &mut variable_assignments,
            &mut function_calls,
            node_counter,
            0 // scope_depth
        );

        Ok((nodes, edges))
    }

    fn extract_flow_elements(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        nodes: &mut Vec<DataFlowNode>,
        edges: &mut Vec<DataFlowEdge>,
        variable_assignments: &mut HashMap<String, String>,
        function_calls: &mut Vec<(String, usize)>,
        node_counter: &mut usize,
        scope_depth: usize,
    ) {
        match node.kind() {
            "assignment" => {
                self.handle_assignment(node, source, file_path, nodes, edges, variable_assignments, node_counter, scope_depth);
            }
            "function_item" => {
                self.handle_function_definition(node, source, file_path, nodes, edges, node_counter, scope_depth);
            }
            "call_expression" => {
                self.handle_function_call(node, source, file_path, nodes, edges, function_calls, node_counter, scope_depth);
            }
            "struct_item" | "enum_item" | "trait_item" => {
                self.handle_class_definition(node, source, file_path, nodes, edges, node_counter, scope_depth);
            }
            _ => {}
        }

        // Recursively process child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.extract_flow_elements(
                    child,
                    source,
                    file_path,
                    nodes,
                    edges,
                    variable_assignments,
                    function_calls,
                    node_counter,
                    scope_depth + 1,
                );
            }
        }
    }

    fn handle_assignment(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        nodes: &mut Vec<DataFlowNode>,
        edges: &mut Vec<DataFlowEdge>,
        variable_assignments: &mut HashMap<String, String>,
        node_counter: &mut usize,
        scope_depth: usize,
    ) {
        if let Some(left) = node.child_by_field_name("left") {
            if let Some(right) = node.child_by_field_name("right") {
                let var_name = &source[left.start_byte()..left.end_byte()];
                let value = &source[right.start_byte()..right.end_byte()];

                let node_id = format!("node_{}", *node_counter);
                *node_counter += 1;

                nodes.push(DataFlowNode {
                    node_id: node_id.clone(),
                    node_type: "variable".to_string(),
                    file_path: file_path.display().to_string(),
                    line_number: node.start_position().row + 1,
                    name: var_name.to_string(),
                    scope: format!("scope_{}", scope_depth),
                    data_types: vec![self.infer_type_from_value(value)],
                });

                variable_assignments.insert(var_name.to_string(), node_id);
            }
        }
    }

    fn handle_function_definition(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        nodes: &mut Vec<DataFlowNode>,
        edges: &mut Vec<DataFlowEdge>,
        node_counter: &mut usize,
        scope_depth: usize,
    ) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let func_name = &source[name_node.start_byte()..name_node.end_byte()];

            let node_id = format!("node_{}", *node_counter);
            *node_counter += 1;

            nodes.push(DataFlowNode {
                node_id,
                node_type: "function".to_string(),
                file_path: file_path.display().to_string(),
                line_number: node.start_position().row + 1,
                name: func_name.to_string(),
                scope: format!("scope_{}", scope_depth),
                data_types: vec!["function".to_string()],
            });
        }
    }

    fn handle_function_call(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        nodes: &mut Vec<DataFlowNode>,
        edges: &mut Vec<DataFlowEdge>,
        function_calls: &mut Vec<(String, usize)>,
        node_counter: &mut usize,
        scope_depth: usize,
    ) {
        if let Some(function_node) = node.child_by_field_name("function") {
            let func_name = &source[function_node.start_byte()..function_node.end_byte()];
            let line_number = node.start_position().row + 1;

            function_calls.push((func_name.to_string(), line_number));

            let node_id = format!("node_{}", *node_counter);
            *node_counter += 1;

            nodes.push(DataFlowNode {
                node_id,
                node_type: "function_call".to_string(),
                file_path: file_path.display().to_string(),
                line_number,
                name: func_name.to_string(),
                scope: format!("scope_{}", scope_depth),
                data_types: vec!["call".to_string()],
            });
        }
    }

    fn handle_class_definition(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        nodes: &mut Vec<DataFlowNode>,
        edges: &mut Vec<DataFlowEdge>,
        node_counter: &mut usize,
        scope_depth: usize,
    ) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let class_name = &source[name_node.start_byte()..name_node.end_byte()];

            let node_id = format!("node_{}", *node_counter);
            *node_counter += 1;

            nodes.push(DataFlowNode {
                node_id,
                node_type: "class".to_string(),
                file_path: file_path.display().to_string(),
                line_number: node.start_position().row + 1,
                name: class_name.to_string(),
                scope: format!("scope_{}", scope_depth),
                data_types: vec!["class".to_string()],
            });
        }
    }

    fn infer_type_from_value(&self, value: &str) -> String {
        if value.starts_with('"') || value.starts_with('\'') {
            "string".to_string()
        } else if value.parse::<i64>().is_ok() {
            "integer".to_string()
        } else if value.parse::<f64>().is_ok() {
            "float".to_string()
        } else if value == "True" || value == "False" {
            "boolean".to_string()
        } else if value.starts_with('[') {
            "list".to_string()
        } else if value.starts_with('{') {
            "dict".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn identify_flow_patterns(&self, nodes: &[DataFlowNode], edges: &[DataFlowEdge]) -> Vec<DataFlowPattern> {
        let mut patterns = Vec::new();

        // Pattern 1: Deep nested function calls
        let deep_nesting = self.find_deep_nesting_pattern(nodes);
        if !deep_nesting.is_empty() {
            patterns.push(DataFlowPattern {
                pattern_type: "deep_nesting".to_string(),
                description: "Deep nested function calls detected".to_string(),
                files_involved: deep_nesting,
                confidence: 0.8,
                impact_score: 0.6,
            });
        }

        // Pattern 2: Circular data dependencies
        let circular_deps = self.find_circular_dependencies(edges);
        if !circular_deps.is_empty() {
            patterns.push(DataFlowPattern {
                pattern_type: "circular_dependency".to_string(),
                description: "Circular data dependencies detected".to_string(),
                files_involved: circular_deps,
                confidence: 0.9,
                impact_score: 0.8,
            });
        }

        // Pattern 3: Unused variables
        let unused_vars = self.find_unused_variables(nodes);
        if !unused_vars.is_empty() {
            patterns.push(DataFlowPattern {
                pattern_type: "unused_variables".to_string(),
                description: "Unused variables detected".to_string(),
                files_involved: unused_vars,
                confidence: 0.7,
                impact_score: 0.3,
            });
        }

        patterns
    }

    fn find_deep_nesting_pattern(&self, nodes: &[DataFlowNode]) -> Vec<String> {
        let mut files_with_deep_nesting = HashSet::new();

        for node in nodes {
            if node.node_type == "function_call" {
                let scope_depth: usize = node.scope.strip_prefix("scope_")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                if scope_depth > 5 {  // Threshold for deep nesting
                    files_with_deep_nesting.insert(node.file_path.clone());
                }
            }
        }

        files_with_deep_nesting.into_iter().collect()
    }

    fn find_circular_dependencies(&self, edges: &[DataFlowEdge]) -> Vec<String> {
        // Simplified circular dependency detection
        let mut files_with_circular_deps = HashSet::new();
        let mut dependency_map: HashMap<String, Vec<String>> = HashMap::new();

        for edge in edges {
            dependency_map
                .entry(edge.from_node.clone())
                .or_default()
                .push(edge.to_node.clone());
        }

        // Simple cycle detection using DFS
        for (node, _) in &dependency_map {
            if self.has_cycle(&dependency_map, node, &mut HashSet::new()) {
                // Get file path from edges involving this node
                for edge in edges {
                    if edge.from_node == *node || edge.to_node == *node {
                        files_with_circular_deps.insert(edge.file_path.clone());
                    }
                }
            }
        }

        files_with_circular_deps.into_iter().collect()
    }

    fn has_cycle(&self, graph: &HashMap<String, Vec<String>>, node: &str, visited: &mut HashSet<String>) -> bool {
        if visited.contains(node) {
            return true;
        }

        visited.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if self.has_cycle(graph, neighbor, visited) {
                    return true;
                }
            }
        }

        visited.remove(node);
        false
    }

    fn find_unused_variables(&self, nodes: &[DataFlowNode]) -> Vec<String> {
        let mut files_with_unused = HashSet::new();
        let mut variable_usage: HashMap<String, usize> = HashMap::new();

        // Count variable usage
        for node in nodes {
            if node.node_type == "variable" {
                *variable_usage.entry(format!("{}:{}", node.file_path, node.name)).or_insert(0) += 1;
            }
        }

        // Find variables used only once (likely unused)
        for (var_key, count) in variable_usage {
            if count == 1 {
                let file_path = var_key.split(':').next().unwrap_or("");
                files_with_unused.insert(file_path.to_string());
            }
        }

        files_with_unused.into_iter().collect()
    }

    fn analyze_variable_lifecycles(&self, nodes: &[DataFlowNode]) -> Vec<VariableLifecycle> {
        let mut lifecycles = Vec::new();
        let mut variable_map: HashMap<String, Vec<&DataFlowNode>> = HashMap::new();

        // Group nodes by variable name and file
        for node in nodes {
            if node.node_type == "variable" {
                let key = format!("{}:{}", node.file_path, node.name);
                variable_map.entry(key).or_default().push(node);
            }
        }

        // Analyze lifecycle for each variable
        for (key, var_nodes) in variable_map {
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() == 2 {
                let file_path = parts[0];
                let var_name = parts[1];

                let mut line_numbers: Vec<usize> = var_nodes.iter().map(|n| n.line_number).collect();
                line_numbers.sort();

                if let (Some(&creation_line), Some(&last_usage)) = (line_numbers.first(), line_numbers.last()) {
                    let modifications = if line_numbers.len() > 2 {
                        line_numbers[1..line_numbers.len()-1].to_vec()
                    } else {
                        Vec::new()
                    };

                    // Determine scope depth (simplified)
                    let scope_depth = var_nodes.first()
                        .and_then(|n| n.scope.strip_prefix("scope_"))
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    lifecycles.push(VariableLifecycle {
                        variable_name: var_name.to_string(),
                        file_path: file_path.to_string(),
                        creation_line,
                        modifications,
                        last_usage,
                        scope_depth,
                        is_global: scope_depth == 0,
                    });
                }
            }
        }

        lifecycles
    }
}

impl BaseDataExtractor for FlowExtractor {
    fn extract_data(&self) -> crate::Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        let files = self.get_all_python_files();

        if files.is_empty() {
            result.insert("data_flow".to_string(), json!({}));
            result.insert("files_analyzed".to_string(), json!(0));
            return Ok(result);
        }

        // Analyze data flow
        let (nodes, edges) = self.analyze_data_flow(&files)?;

        // Identify patterns
        let patterns = self.identify_flow_patterns(&nodes, &edges);

        // Analyze variable lifecycles
        let lifecycles = self.analyze_variable_lifecycles(&nodes);

        // Generate statistics
        let total_files = files.len();
        let total_nodes = nodes.len();
        let total_edges = edges.len();
        let total_patterns = patterns.len();

        let node_type_counts: HashMap<String, usize> = nodes.iter()
            .fold(HashMap::new(), |mut acc, node| {
                *acc.entry(node.node_type.clone()).or_insert(0) += 1;
                acc
            });

        let edge_type_counts: HashMap<String, usize> = edges.iter()
            .fold(HashMap::new(), |mut acc, edge| {
                *acc.entry(edge.edge_type.clone()).or_insert(0) += 1;
                acc
            });

        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));
        result.insert("files_analyzed".to_string(), json!(total_files));
        result.insert("total_nodes".to_string(), json!(total_nodes));
        result.insert("total_edges".to_string(), json!(total_edges));
        result.insert("node_type_counts".to_string(), json!(node_type_counts));
        result.insert("edge_type_counts".to_string(), json!(edge_type_counts));
        result.insert("data_flow_nodes".to_string(), json!(nodes));
        result.insert("data_flow_edges".to_string(), json!(edges));
        result.insert("flow_patterns".to_string(), json!(patterns));
        result.insert("variable_lifecycles".to_string(), json!(lifecycles));
        result.insert("pattern_count".to_string(), json!(total_patterns));

        // Add recommendations
        let mut recommendations = Vec::new();
        if total_patterns > 0 {
            recommendations.push(format!("Found {} data flow patterns that may need attention", total_patterns));
        }
        if total_edges as f64 / total_nodes as f64 > 2.0 {
            recommendations.push("High edge-to-node ratio indicates complex data flow".to_string());
        }
        if lifecycles.iter().any(|l| l.scope_depth > 5) {
            recommendations.push("Deep variable scoping detected - consider refactoring".to_string());
        }

        result.insert("recommendations".to_string(), json!(recommendations));

        println!("Data flow extraction complete: {} files analyzed, {} nodes, {} edges, {} patterns found",
                 total_files, total_nodes, total_edges, total_patterns);

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "FlowExtractor"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}