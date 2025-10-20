use super::BaseDataExtractor;
use crate::external_tools::ExternalToolManager;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tree_sitter::{Language, Parser};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::fs;

extern "C" {
    fn tree_sitter_rust() -> Language;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceHotspot {
    file_path: String,
    function_name: String,
    line_number: usize,
    complexity: usize,
    performance_score: f64,
    issue_type: String,
    severity: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceBottleneck {
    file_path: String,
    bottleneck_type: String,
    severity: String,
    description: String,
    line_number: Option<usize>,
    function_name: Option<String>,
}

pub struct PerformanceExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
}

impl PerformanceExtractor {
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

    fn get_source_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.codebase_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                    files.push(path);
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    files.extend(self.get_files_recursive(&path));
                }
            }
        }
        files
    }

    fn get_files_recursive(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                    files.push(path);
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    files.extend(self.get_files_recursive(&path));
                }
            }
        }
        files
    }

    fn is_excluded_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "__pycache__" | ".git" | ".pytest_cache" | "node_modules" | ".venv" | "venv")
        } else {
            false
        }
    }

    fn analyze_file_performance(&self, file_path: &Path) -> crate::Result<Value> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io(e))?;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Analysis(format!("Failed to set language: {}", e)))?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Analysis("Failed to parse file".to_string()))?;

        let mut analyzer = PerformanceAstAnalyzer::new(
            file_path.display().to_string(),
            self.codebase_path.clone()
        );

        analyzer.visit_node(tree.root_node(), &content);

        let hotspots = self.identify_performance_hotspots(&analyzer);
        let bottlenecks = self.detect_performance_bottlenecks(&analyzer);
        let optimization_opportunities = self.find_optimization_opportunities(&analyzer);
        let performance_score = self.calculate_performance_score(&analyzer);

        Ok(json!({
            "file_path": file_path.display().to_string(),
            "performance_score": performance_score,
            "hotspots": hotspots,
            "bottlenecks": bottlenecks,
            "optimization_opportunities": optimization_opportunities,
            "functions_analyzed": analyzer.functions.len(),
            "loops_found": analyzer.loops.len(),
            "recursive_calls": analyzer.recursive_calls.len(),
            "expensive_operations": analyzer.expensive_operations.len()
        }))
    }

    fn identify_performance_hotspots(&self, analyzer: &PerformanceAstAnalyzer) -> Vec<PerformanceHotspot> {
        let mut hotspots = Vec::new();

        // Complex functions with high performance impact
        for func in &analyzer.functions {
            if func.complexity > 15 || func.lines > 100 {
                hotspots.push(PerformanceHotspot {
                    file_path: analyzer.file_path.clone(),
                    function_name: func.name.clone(),
                    line_number: func.line,
                    complexity: func.complexity,
                    performance_score: self.calculate_function_performance_score(func),
                    issue_type: "complex_function".to_string(),
                    severity: if func.complexity > 25 { "high" } else { "medium" }.to_string(),
                    description: format!("Function '{}' has high complexity ({}) and may impact performance", func.name, func.complexity),
                });
            }
        }

        // Nested loops
        for loop_info in &analyzer.loops {
            if loop_info.nesting_level > 2 {
                hotspots.push(PerformanceHotspot {
                    file_path: analyzer.file_path.clone(),
                    function_name: loop_info.function_name.clone().unwrap_or_else(|| "global".to_string()),
                    line_number: loop_info.line,
                    complexity: loop_info.nesting_level,
                    performance_score: 100.0 - (loop_info.nesting_level as f64 * 20.0),
                    issue_type: "nested_loops".to_string(),
                    severity: if loop_info.nesting_level > 3 { "high" } else { "medium" }.to_string(),
                    description: format!("Nested loop with {} levels of nesting may cause performance issues", loop_info.nesting_level),
                });
            }
        }

        hotspots
    }

    fn detect_performance_bottlenecks(&self, analyzer: &PerformanceAstAnalyzer) -> Vec<PerformanceBottleneck> {
        let mut bottlenecks = Vec::new();

        // File I/O operations without proper handling
        for op in &analyzer.expensive_operations {
            if op.operation_type == "file_io" {
                bottlenecks.push(PerformanceBottleneck {
                    file_path: analyzer.file_path.clone(),
                    bottleneck_type: "file_io".to_string(),
                    severity: "medium".to_string(),
                    description: "File I/O operation detected - consider caching or async operations".to_string(),
                    line_number: Some(op.line),
                    function_name: op.function_name.clone(),
                });
            }
        }

        // Database operations
        for op in &analyzer.expensive_operations {
            if op.operation_type == "database" {
                bottlenecks.push(PerformanceBottleneck {
                    file_path: analyzer.file_path.clone(),
                    bottleneck_type: "database".to_string(),
                    severity: "high".to_string(),
                    description: "Database operation detected - consider connection pooling and query optimization".to_string(),
                    line_number: Some(op.line),
                    function_name: op.function_name.clone(),
                });
            }
        }

        // Network operations
        for op in &analyzer.expensive_operations {
            if op.operation_type == "network" {
                bottlenecks.push(PerformanceBottleneck {
                    file_path: analyzer.file_path.clone(),
                    bottleneck_type: "network".to_string(),
                    severity: "high".to_string(),
                    description: "Network operation detected - consider async requests and connection reuse".to_string(),
                    line_number: Some(op.line),
                    function_name: op.function_name.clone(),
                });
            }
        }

        bottlenecks
    }

    fn find_optimization_opportunities(&self, analyzer: &PerformanceAstAnalyzer) -> Vec<Value> {
        let mut opportunities = Vec::new();

        // Large string concatenations
        for op in &analyzer.expensive_operations {
            if op.operation_type == "string_concat" {
                opportunities.push(json!({
                    "type": "string_optimization",
                    "description": "Use string builder or join() for multiple string concatenations",
                    "line": op.line,
                    "function": op.function_name,
                    "impact": "medium"
                }));
            }
        }

        // List comprehensions vs loops
        for loop_info in &analyzer.loops {
            if loop_info.loop_type == "for" && loop_info.has_append {
                opportunities.push(json!({
                    "type": "list_comprehension",
                    "description": "Consider using list comprehension instead of for loop with append",
                    "line": loop_info.line,
                    "function": loop_info.function_name,
                    "impact": "low"
                }));
            }
        }

        // Caching opportunities
        for func in &analyzer.functions {
            if func.has_repeated_calculations {
                opportunities.push(json!({
                    "type": "caching",
                    "description": "Function may benefit from memoization or caching",
                    "line": func.line,
                    "function": func.name,
                    "impact": "medium"
                }));
            }
        }

        opportunities
    }

    fn calculate_performance_score(&self, analyzer: &PerformanceAstAnalyzer) -> f64 {
        let mut score = 100.0;

        // Penalize complex functions
        for func in &analyzer.functions {
            if func.complexity > 10 {
                score -= (func.complexity as f64 - 10.0) * 2.0;
            }
        }

        // Penalize nested loops
        for loop_info in &analyzer.loops {
            if loop_info.nesting_level > 2 {
                score -= (loop_info.nesting_level as f64 - 2.0) * 5.0;
            }
        }

        // Penalize expensive operations
        score -= analyzer.expensive_operations.len() as f64 * 3.0;

        // Penalize recursive calls without termination checks
        score -= analyzer.recursive_calls.len() as f64 * 2.0;

        score.max(0.0).min(100.0)
    }

    fn calculate_function_performance_score(&self, func: &FunctionInfo) -> f64 {
        let mut score = 100.0;

        // Complexity penalty
        if func.complexity > 10 {
            score -= (func.complexity as f64 - 10.0) * 3.0;
        }

        // Length penalty
        if func.lines > 50 {
            score -= (func.lines as f64 - 50.0) * 0.5;
        }

        score.max(0.0).min(100.0)
    }
}

#[derive(Debug, Default)]
struct PerformanceAstAnalyzer {
    file_path: String,
    codebase_path: PathBuf,
    functions: Vec<FunctionInfo>,
    loops: Vec<LoopInfo>,
    expensive_operations: Vec<ExpensiveOperation>,
    recursive_calls: Vec<RecursiveCall>,
    current_function: Option<String>,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    line: usize,
    complexity: usize,
    lines: usize,
    has_repeated_calculations: bool,
}

#[derive(Debug, Clone)]
struct LoopInfo {
    line: usize,
    loop_type: String,
    nesting_level: usize,
    function_name: Option<String>,
    has_append: bool,
}

#[derive(Debug, Clone)]
struct ExpensiveOperation {
    line: usize,
    operation_type: String,
    function_name: Option<String>,
    description: String,
}

#[derive(Debug, Clone)]
struct RecursiveCall {
    line: usize,
    function_name: String,
    has_termination_check: bool,
}

impl PerformanceAstAnalyzer {
    fn new(file_path: String, codebase_path: PathBuf) -> Self {
        Self {
            file_path,
            codebase_path,
            functions: Vec::new(),
            loops: Vec::new(),
            expensive_operations: Vec::new(),
            recursive_calls: Vec::new(),
            current_function: None,
        }
    }

    fn visit_node(&mut self, node: tree_sitter::Node, source: &str) {
        match node.kind() {
            "function_def" => self.visit_function_def(node, source),
            "for_statement" => self.visit_for_statement(node, source),
            "while_statement" => self.visit_while_statement(node, source),
            "call" => self.visit_call(node, source),
            _ => {}
        }

        // Visit child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit_node(child, source);
            }
        }
    }

    fn visit_function_def(&mut self, node: tree_sitter::Node, source: &str) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = &source[name_node.start_byte()..name_node.end_byte()];
            let line = name_node.start_position().row + 1;

            // Calculate complexity and lines
            let complexity = self.calculate_node_complexity(node);
            let lines = node.end_position().row - node.start_position().row + 1;

            self.functions.push(FunctionInfo {
                name: name.to_string(),
                line,
                complexity,
                lines,
                has_repeated_calculations: self.has_repeated_calculations(node, source),
            });

            self.current_function = Some(name.to_string());
        }
    }

    fn visit_for_statement(&mut self, node: tree_sitter::Node, source: &str) {
        let line = node.start_position().row + 1;
        let nesting_level = self.calculate_nesting_level(node);
        let has_append = self.contains_append_operation(node, source);

        self.loops.push(LoopInfo {
            line,
            loop_type: "for".to_string(),
            nesting_level,
            function_name: self.current_function.clone(),
            has_append,
        });
    }

    fn visit_while_statement(&mut self, node: tree_sitter::Node, _source: &str) {
        let line = node.start_position().row + 1;
        let nesting_level = self.calculate_nesting_level(node);

        self.loops.push(LoopInfo {
            line,
            loop_type: "while".to_string(),
            nesting_level,
            function_name: self.current_function.clone(),
            has_append: false,
        });
    }

    fn visit_call(&mut self, node: tree_sitter::Node, source: &str) {
        if let Some(function_node) = node.child_by_field_name("function") {
            let function_call = &source[function_node.start_byte()..function_node.end_byte()];
            let line = node.start_position().row + 1;

            // Detect expensive operations
            if self.is_file_operation(function_call) {
                self.expensive_operations.push(ExpensiveOperation {
                    line,
                    operation_type: "file_io".to_string(),
                    function_name: self.current_function.clone(),
                    description: format!("File operation: {}", function_call),
                });
            } else if self.is_database_operation(function_call) {
                self.expensive_operations.push(ExpensiveOperation {
                    line,
                    operation_type: "database".to_string(),
                    function_name: self.current_function.clone(),
                    description: format!("Database operation: {}", function_call),
                });
            } else if self.is_network_operation(function_call) {
                self.expensive_operations.push(ExpensiveOperation {
                    line,
                    operation_type: "network".to_string(),
                    function_name: self.current_function.clone(),
                    description: format!("Network operation: {}", function_call),
                });
            }

            // Check for recursive calls
            if let Some(current_func) = &self.current_function {
                if function_call == current_func {
                    self.recursive_calls.push(RecursiveCall {
                        line,
                        function_name: current_func.clone(),
                        has_termination_check: self.has_termination_check_nearby(node),
                    });
                }
            }
        }
    }

    fn calculate_node_complexity(&self, node: tree_sitter::Node) -> usize {
        let mut complexity = 1; // Base complexity

        // Add complexity for control structures
        let mut cursor = node.walk();
        cursor.goto_first_child();

        loop {
            match cursor.node().kind() {
                "if_statement" | "elif_clause" | "else_clause" => complexity += 1,
                "for_statement" | "while_statement" => complexity += 1,
                "try_statement" | "except_clause" => complexity += 1,
                "and" | "or" => complexity += 1,
                _ => {}
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }

        complexity
    }

    fn calculate_nesting_level(&self, node: tree_sitter::Node) -> usize {
        let mut level = 0;
        let mut current = node.parent();

        while let Some(parent) = current {
            match parent.kind() {
                "for_statement" | "while_statement" | "if_statement" => level += 1,
                _ => {}
            }
            current = parent.parent();
        }

        level
    }

    fn contains_append_operation(&self, node: tree_sitter::Node, source: &str) -> bool {
        // Simple check for .append() calls within the loop
        let node_text = &source[node.start_byte()..node.end_byte()];
        node_text.contains(".append(")
    }

    fn has_repeated_calculations(&self, node: tree_sitter::Node, source: &str) -> bool {
        // Simple heuristic: look for repeated expensive function calls
        let node_text = &source[node.start_byte()..node.end_byte()];

        // Count occurrences of potentially expensive operations
        let expensive_patterns = ["len(", "max(", "min(", "sum(", "sorted("];
        for pattern in &expensive_patterns {
            if node_text.matches(pattern).count() > 2 {
                return true;
            }
        }

        false
    }

    fn is_file_operation(&self, function_call: &str) -> bool {
        matches!(function_call, "open" | "read" | "write" | "readlines" | "writelines") ||
        function_call.contains("file") ||
        function_call.contains("Path")
    }

    fn is_database_operation(&self, function_call: &str) -> bool {
        function_call.contains("execute") ||
        function_call.contains("query") ||
        function_call.contains("cursor") ||
        function_call.contains("commit") ||
        function_call.contains("connect")
    }

    fn is_network_operation(&self, function_call: &str) -> bool {
        function_call.contains("request") ||
        function_call.contains("urlopen") ||
        function_call.contains("get") ||
        function_call.contains("post") ||
        function_call.contains("socket")
    }

    fn has_termination_check_nearby(&self, _node: tree_sitter::Node) -> bool {
        // Simplified check - in a real implementation, would analyze surrounding context
        false
    }
}

impl BaseDataExtractor for PerformanceExtractor {
    fn extract_data(&self) -> crate::Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        let source_files = self.get_source_files();

        let mut all_hotspots = Vec::new();
        let mut all_bottlenecks = Vec::new();
        let mut all_opportunities = Vec::new();
        let mut total_performance_score = 0.0;
        let mut files_analyzed = 0;

        for file_path in &source_files {
            match self.analyze_file_performance(file_path) {
                Ok(file_data) => {
                    files_analyzed += 1;

                    if let Some(score) = file_data.get("performance_score").and_then(|v| v.as_f64()) {
                        total_performance_score += score;
                    }

                    if let Some(hotspots) = file_data.get("hotspots").and_then(|v| v.as_array()) {
                        all_hotspots.extend(hotspots.iter().cloned());
                    }

                    if let Some(bottlenecks) = file_data.get("bottlenecks").and_then(|v| v.as_array()) {
                        all_bottlenecks.extend(bottlenecks.iter().cloned());
                    }

                    if let Some(opportunities) = file_data.get("optimization_opportunities").and_then(|v| v.as_array()) {
                        all_opportunities.extend(opportunities.iter().cloned());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to analyze file {}: {}", file_path.display(), e);
                }
            }
        }

        let average_performance_score = if files_analyzed > 0 {
            total_performance_score / files_analyzed as f64
        } else {
            0.0
        };

        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));
        result.insert("files_analyzed".to_string(), json!(files_analyzed));
        result.insert("average_performance_score".to_string(), json!(average_performance_score));
        result.insert("performance_hotspots".to_string(), json!(all_hotspots));
        result.insert("performance_bottlenecks".to_string(), json!(all_bottlenecks));
        result.insert("optimization_opportunities".to_string(), json!(all_opportunities));
        result.insert("total_hotspots".to_string(), json!(all_hotspots.len()));
        result.insert("total_bottlenecks".to_string(), json!(all_bottlenecks.len()));
        result.insert("total_opportunities".to_string(), json!(all_opportunities.len()));

        println!("Performance extraction complete: {} files analyzed, average score {:.1}",
                 files_analyzed, average_performance_score);

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "PerformanceExtractor"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}