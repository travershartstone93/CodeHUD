//! Topology Data Extractor
//!
//! Extracts codebase topology information including file structure,
//! classes, functions, and architectural overview.
//! 
//! This is a zero-degradation Rust translation of topology_extractor.py

use super::{BaseDataExtractor, FileMetrics};
use crate::Result;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tracing::{info, warn};
use regex;

/// Extract codebase topology and architecture data
pub struct TopologyExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
}

impl TopologyExtractor {
    /// Create a new topology extractor
    pub fn new(codebase_path: impl AsRef<Path>) -> crate::Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();
        
        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!(
                "Codebase path does not exist: {}", 
                codebase_path.display()
            )));
        }
        
        if !codebase_path.is_dir() {
            return Err(crate::Error::Config(format!(
                "Codebase path is not a directory: {}", 
                codebase_path.display()
            )));
        }
        
        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
        })
    }
    
    /// Analyze a single file (Python _analyze_file equivalent)
    fn analyze_file(&self, file_path: &Path) -> Result<FileAnalysis> {
        let mut file_data = FileAnalysis {
            metrics: self.calculate_file_metrics(file_path),
            classes: Vec::new(),
            functions: Vec::new(),
            dependencies: Vec::new(),
            complexity: 0,
            is_test: false,
        };
        
        // Check if this is a test file
        file_data.is_test = self.is_test_file(file_path);
        
        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
            match ext {
                "py" => {
                    // Python-specific analysis using tree-sitter
                    if let Ok(py_data) = self.analyze_python_file(file_path) {
                        file_data.classes = py_data.classes;
                        file_data.functions = py_data.functions;
                        file_data.dependencies = py_data.dependencies;
                        file_data.complexity = py_data.complexity;
                    }
                }
                "js" | "ts" => {
                    // JavaScript/TypeScript analysis
                    if let Ok(js_data) = self.analyze_javascript_file(file_path) {
                        file_data.functions = js_data.functions;
                        file_data.dependencies = js_data.dependencies;
                        file_data.complexity = js_data.complexity;
                    }
                }
                "rs" => {
                    // Rust analysis
                    if let Ok(rs_data) = self.analyze_rust_file(file_path) {
                        file_data.functions = rs_data.functions;
                        file_data.dependencies = rs_data.dependencies;
                        file_data.complexity = rs_data.complexity;
                    }
                }
                _ => {
                    // Basic analysis for other file types
                    file_data.complexity = self.calculate_basic_complexity(file_path).unwrap_or(0);
                }
            }
        }
        
        Ok(file_data)
    }
    
    /// Analyze Python file using tree-sitter AST (Python _analyze_python_file equivalent)
    fn analyze_python_file(&self, file_path: &Path) -> Result<LanguageAnalysis> {
        use tree_sitter::Parser;

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io(e))?;

        // Initialize tree-sitter parser for Python
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Analysis(format!("Failed to set Python language: {}", e)))?;

        // Parse the file
        let tree = parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Analysis("Failed to parse Python file".to_string()))?;

        let root_node = tree.root_node();
        let mut analyzer = PythonAstAnalyzer::new(file_path, &self.codebase_path);
        analyzer.visit_node(root_node, &content.as_bytes());

        Ok(LanguageAnalysis {
            functions: analyzer.functions,
            classes: analyzer.classes,
            dependencies: analyzer.dependencies,
            complexity: analyzer.complexity,
        })
    }
    
    /// Analyze JavaScript/TypeScript file using regex-based parsing
    fn analyze_javascript_file(&self, file_path: &Path) -> Result<LanguageAnalysis> {
        let content = std::fs::read_to_string(file_path)?;
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut dependencies = Vec::new();
        let mut complexity = 1;

        // Function detection patterns
        let function_patterns = [
            regex::Regex::new(r"function\s+(\w+)\s*\(").unwrap(),
            regex::Regex::new(r"(\w+)\s*=\s*function\s*\(").unwrap(),
            regex::Regex::new(r"(\w+)\s*:\s*function\s*\(").unwrap(),
            regex::Regex::new(r"(\w+)\s*=\s*\([^)]*\)\s*=>").unwrap(),
            regex::Regex::new(r"const\s+(\w+)\s*=\s*\([^)]*\)\s*=>").unwrap(),
            regex::Regex::new(r"async\s+function\s+(\w+)\s*\(").unwrap(),
        ];

        // Class detection pattern
        let class_pattern = regex::Regex::new(r"class\s+(\w+)").unwrap();

        // Import/require patterns
        let import_patterns = [
            regex::Regex::new(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap(),
            regex::Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap(),
            regex::Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]"#).unwrap(),
        ];

        // Complexity keywords
        let complexity_keywords = ["if", "else if", "for", "while", "switch", "case", "try", "catch"];

        for (line_num, line) in content.lines().enumerate() {
            let line_number = line_num + 1;

            // Detect functions
            for pattern in &function_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(name) = captures.get(1) {
                        functions.push(FunctionInfo {
                            name: name.as_str().to_string(),
                            line_number,
                            file_path: file_path.display().to_string(),
                            complexity: 1,
                            is_async: line.contains("async"),
                            parameters: self.extract_js_parameters(line),
                            return_type: self.extract_js_return_type(line),
                        });
                    }
                }
            }

            // Detect classes
            if let Some(captures) = class_pattern.captures(line) {
                if let Some(name) = captures.get(1) {
                    classes.push(ClassInfo {
                        name: name.as_str().to_string(),
                        line_number,
                        file_path: file_path.display().to_string(),
                        methods: Vec::new(),
                        base_classes: self.extract_js_extends(line),
                    });
                }
            }

            // Detect imports/dependencies
            for pattern in &import_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(module) = captures.get(1) {
                        let module_name = module.as_str();
                        if !module_name.starts_with('.') && !module_name.starts_with('/') {
                            dependencies.push(module_name.to_string());
                        }
                    }
                }
            }

            // Calculate complexity
            for keyword in &complexity_keywords {
                if line.trim().contains(keyword) {
                    complexity += 1;
                }
            }
        }

        Ok(LanguageAnalysis {
            functions,
            classes,
            dependencies,
            complexity,
        })
    }

    /// Extract JavaScript function parameters
    fn extract_js_parameters(&self, line: &str) -> Vec<String> {
        if let Some(start) = line.find('(') {
            if let Some(end) = line[start..].find(')') {
                let params_str = &line[start + 1..start + end];
                return params_str
                    .split(',')
                    .map(|p| p.trim().split('=').next().unwrap_or("").trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
            }
        }
        Vec::new()
    }

    /// Extract JavaScript return type from TypeScript annotations
    fn extract_js_return_type(&self, line: &str) -> Option<String> {
        if let Some(arrow_pos) = line.find("): ") {
            if let Some(brace_pos) = line[arrow_pos + 3..].find('{') {
                let return_type = line[arrow_pos + 3..arrow_pos + 3 + brace_pos].trim();
                if !return_type.is_empty() {
                    return Some(return_type.to_string());
                }
            }
        }
        None
    }

    /// Extract JavaScript extends clause
    fn extract_js_extends(&self, line: &str) -> Vec<String> {
        if let Some(extends_pos) = line.find("extends ") {
            let after_extends = &line[extends_pos + 8..];
            if let Some(brace_pos) = after_extends.find('{') {
                let extends_str = &after_extends[..brace_pos].trim();
                return vec![extends_str.to_string()];
            }
        }
        Vec::new()
    }
    
    /// Analyze Rust file using regex-based parsing
    fn analyze_rust_file(&self, file_path: &Path) -> Result<LanguageAnalysis> {
        let content = std::fs::read_to_string(file_path)?;
        let mut functions = Vec::new();
        let mut classes = Vec::new(); // Structs/enums in Rust
        let mut dependencies = Vec::new();
        let mut complexity = 1;

        // Function detection patterns
        let function_patterns = [
            regex::Regex::new(r"fn\s+(\w+)\s*\(").unwrap(),
            regex::Regex::new(r"async\s+fn\s+(\w+)\s*\(").unwrap(),
            regex::Regex::new(r"pub\s+fn\s+(\w+)\s*\(").unwrap(),
            regex::Regex::new(r"pub\s+async\s+fn\s+(\w+)\s*\(").unwrap(),
        ];

        // Struct/enum detection patterns (treating as classes)
        let struct_patterns = [
            regex::Regex::new(r"struct\s+(\w+)").unwrap(),
            regex::Regex::new(r"pub\s+struct\s+(\w+)").unwrap(),
            regex::Regex::new(r"enum\s+(\w+)").unwrap(),
            regex::Regex::new(r"pub\s+enum\s+(\w+)").unwrap(),
        ];

        // Use/import patterns
        let use_patterns = [
            regex::Regex::new(r"use\s+(\w+)::").unwrap(),
            regex::Regex::new(r"use\s+(\w+);").unwrap(),
            regex::Regex::new(r"extern\s+crate\s+(\w+)").unwrap(),
        ];

        // Complexity keywords
        let complexity_keywords = ["if", "else if", "match", "for", "while", "loop", "Result"];

        for (line_num, line) in content.lines().enumerate() {
            let line_number = line_num + 1;

            // Detect functions
            for pattern in &function_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(name) = captures.get(1) {
                        functions.push(FunctionInfo {
                            name: name.as_str().to_string(),
                            line_number,
                            file_path: file_path.display().to_string(),
                            complexity: 1,
                            is_async: line.contains("async"),
                            parameters: self.extract_rust_parameters(line),
                            return_type: self.extract_rust_return_type(line),
                        });
                    }
                }
            }

            // Detect structs/enums as classes
            for pattern in &struct_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(name) = captures.get(1) {
                        classes.push(ClassInfo {
                            name: name.as_str().to_string(),
                            line_number,
                            file_path: file_path.display().to_string(),
                            methods: Vec::new(),
                            base_classes: self.extract_rust_derives(line),
                        });
                    }
                }
            }

            // Detect use statements/dependencies
            for pattern in &use_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(module) = captures.get(1) {
                        let module_name = module.as_str();
                        if !module_name.starts_with("crate") && !module_name.starts_with("self") && !module_name.starts_with("super") {
                            dependencies.push(module_name.to_string());
                        }
                    }
                }
            }

            // Calculate complexity
            for keyword in &complexity_keywords {
                if line.trim().contains(keyword) {
                    complexity += 1;
                }
            }
        }

        Ok(LanguageAnalysis {
            functions,
            classes,
            dependencies,
            complexity,
        })
    }

    /// Extract Rust function parameters
    fn extract_rust_parameters(&self, line: &str) -> Vec<String> {
        if let Some(start) = line.find('(') {
            if let Some(end) = line[start..].find(')') {
                let params_str = &line[start + 1..start + end];
                return params_str
                    .split(',')
                    .map(|p| {
                        // Extract parameter name before colon
                        if let Some(colon_pos) = p.find(':') {
                            p[..colon_pos].trim().to_string()
                        } else {
                            p.trim().to_string()
                        }
                    })
                    .filter(|p| !p.is_empty() && p != "&self" && p != "self" && p != "&mut self")
                    .collect();
            }
        }
        Vec::new()
    }

    /// Extract Rust return type
    fn extract_rust_return_type(&self, line: &str) -> Option<String> {
        if let Some(arrow_pos) = line.find(" -> ") {
            if let Some(brace_pos) = line[arrow_pos + 4..].find('{') {
                let return_type = line[arrow_pos + 4..arrow_pos + 4 + brace_pos].trim();
                if !return_type.is_empty() {
                    return Some(return_type.to_string());
                }
            } else {
                // Handle single line functions
                let return_type = line[arrow_pos + 4..].trim();
                if !return_type.is_empty() && !return_type.contains('{') {
                    return Some(return_type.to_string());
                }
            }
        }
        None
    }

    /// Extract Rust derive traits as base classes
    fn extract_rust_derives(&self, line: &str) -> Vec<String> {
        if let Some(derive_pos) = line.find("#[derive(") {
            if let Some(end_pos) = line[derive_pos..].find(")]") {
                let derives_str = &line[derive_pos + 9..derive_pos + end_pos];
                return derives_str
                    .split(',')
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty())
                    .collect();
            }
        }
        Vec::new()
    }
    
    /// Calculate basic complexity based on control flow keywords
    fn calculate_basic_complexity(&self, file_path: &Path) -> Result<i32> {
        let content = std::fs::read_to_string(file_path)?;
        let keywords = ["if", "elif", "else", "for", "while", "try", "except", "match", "case"];
        
        let complexity = content.lines()
            .map(|line| {
                keywords.iter()
                    .filter(|&&keyword| line.trim().starts_with(keyword))
                    .count() as i32
            })
            .sum::<i32>()
            .max(1); // Minimum complexity of 1
            
        Ok(complexity)
    }
    
    /// Check if a file is a test file based on naming conventions
    fn is_test_file(&self, file_path: &Path) -> bool {
        if let Some(filename) = file_path.file_name().and_then(|s| s.to_str()) {
            filename.starts_with("test_") || 
            filename.ends_with("_test.py") ||
            filename.contains("test") ||
            file_path.to_string_lossy().contains("test")
        } else {
            false
        }
    }
    
    /// Extract function name from a function definition line
    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple pattern: "def function_name(" or "async def function_name("
        let start = if line.trim().starts_with("async def") {
            line.find("def ")? + 4
        } else {
            line.find("def ")? + 4
        };
        
        let end = line[start..].find('(')?;
        Some(line[start..start + end].trim().to_string())
    }
    
    /// Extract class name from a class definition line
    fn extract_class_name(&self, line: &str) -> Option<String> {
        // Simple pattern: "class ClassName:" or "class ClassName("
        let start = line.find("class ")? + 6;
        let rest = &line[start..];
        
        let end = rest.find(':').or_else(|| rest.find('('))?;
        Some(rest[..end].trim().to_string())
    }
    
    /// Extract dependency from import statement
    fn extract_dependency(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        
        if trimmed.starts_with("import ") {
            // import module
            let module = trimmed.strip_prefix("import ")?.split_whitespace().next()?;
            Some(module.split('.').next()?.to_string())
        } else if trimmed.starts_with("from ") {
            // from module import ...
            let after_from = trimmed.strip_prefix("from ")?;
            let module = after_from.split_whitespace().next()?;
            Some(module.split('.').next()?.to_string())
        } else {
            None
        }
    }
    
    /// Calculate summary statistics (Python _calculate_summary equivalent)
    fn calculate_summary(&self, files_data: &[FileAnalysis]) -> TopologySummary {
        let total_files = files_data.len();
        let total_lines = files_data.iter().map(|f| f.metrics.total_lines).sum();
        let total_code_lines = files_data.iter().map(|f| f.metrics.code_lines).sum();
        let total_classes = files_data.iter().map(|f| f.classes.len()).sum();
        let total_functions = files_data.iter().map(|f| f.functions.len()).sum();
        let test_files = files_data.iter().filter(|f| f.is_test).count();
        
        // Calculate language distribution
        let mut language_distribution = HashMap::new();
        for file_data in files_data {
            if let Some(ext) = &file_data.metrics.extension {
                *language_distribution.entry(ext.clone()).or_insert(0) += 1;
            }
        }
        
        // Calculate complexity distribution
        let complexities: Vec<i32> = files_data.iter().map(|f| f.complexity).collect();
        let avg_complexity = if complexities.is_empty() {
            0.0
        } else {
            complexities.iter().sum::<i32>() as f64 / complexities.len() as f64
        };
        
        TopologySummary {
            total_files,
            total_lines,
            total_code_lines,
            total_classes,
            total_functions,
            test_files,
            language_distribution,
            average_complexity: avg_complexity,
        }
    }
    
    /// Analyze project structure (Python _analyze_project_structure equivalent)
    fn analyze_project_structure(&self, source_files: &[PathBuf]) -> ProjectStructure {
        let mut directories = HashMap::new();
        let mut max_depth = 0;
        
        for file_path in source_files {
            if let Ok(relative_path) = file_path.strip_prefix(&self.codebase_path) {
                let depth = relative_path.components().count();
                max_depth = max_depth.max(depth);
                
                // Count files per directory
                if let Some(parent) = relative_path.parent() {
                    let dir_str = parent.to_string_lossy().to_string();
                    *directories.entry(dir_str).or_insert(0) += 1;
                }
            }
        }
        
        let total_directories = directories.len();
        ProjectStructure {
            max_depth,
            directories,
            total_directories,
        }
    }
    
    /// Calculate coupling metrics (Python _calculate_coupling equivalent)
    fn calculate_coupling(&self, dependencies: &HashMap<String, HashSet<String>>) -> CouplingMetrics {
        let total_files = dependencies.len();
        let total_dependencies: usize = dependencies.values().map(|deps| deps.len()).sum();
        
        let average_dependencies = if total_files > 0 {
            total_dependencies as f64 / total_files as f64
        } else {
            0.0
        };
        
        // Find most coupled files
        let mut coupling_pairs: Vec<(String, usize)> = dependencies
            .iter()
            .map(|(file, deps)| (file.clone(), deps.len()))
            .collect();
        coupling_pairs.sort_by(|a, b| b.1.cmp(&a.1));
        
        let highly_coupled_files = coupling_pairs
            .into_iter()
            .take(10)
            .collect::<Vec<(String, usize)>>();
        
        CouplingMetrics {
            average_dependencies,
            total_dependencies,
            highly_coupled_files,
        }
    }
}

impl BaseDataExtractor for TopologyExtractor {
    fn extract_data(&self) -> Result<HashMap<String, serde_json::Value>> {
        info!("Extracting topology data from {}", self.codebase_path.display());
        
        // Get all source files
        let source_files = self.get_source_files(None)?;
        
        // Analyze each file
        let mut files_data = Vec::new();
        let mut all_classes = Vec::new();
        let mut all_functions = Vec::new();
        let mut dependencies = HashMap::new();
        
        for file_path in &source_files {
            match self.analyze_file(file_path) {
                Ok(file_data) => {
                    // Collect classes and functions
                    all_classes.extend(file_data.classes.iter().cloned());
                    all_functions.extend(file_data.functions.iter().cloned());
                    
                    // Collect dependencies
                    if !file_data.dependencies.is_empty() {
                        let rel_path = file_path.strip_prefix(&self.codebase_path)
                            .unwrap_or(file_path)
                            .to_string_lossy()
                            .to_string();
                        dependencies.insert(rel_path, file_data.dependencies.iter().cloned().collect());
                    }
                    
                    files_data.push(file_data);
                }
                Err(e) => {
                    warn!("Failed to analyze file {:?}: {}", file_path, e);
                }
            }
        }
        
        // Calculate summary statistics
        let summary = self.calculate_summary(&files_data);
        
        // Analyze project structure
        let structure = self.analyze_project_structure(&source_files);
        
        // Calculate coupling metrics
        let coupling = self.calculate_coupling(&dependencies);
        
        // Convert to the expected format
        let mut result = HashMap::new();
        result.insert("summary".to_string(), serde_json::to_value(summary)?);
        result.insert("files".to_string(), serde_json::to_value(files_data)?);
        result.insert("classes".to_string(), serde_json::to_value(all_classes)?);
        result.insert("functions".to_string(), serde_json::to_value(all_functions)?);
        result.insert("dependencies".to_string(), serde_json::to_value(dependencies)?);
        result.insert("structure".to_string(), serde_json::to_value(structure)?);
        result.insert("coupling".to_string(), serde_json::to_value(coupling)?);
        
        Ok(result)
    }
    
    fn extractor_type(&self) -> &'static str {
        "TopologyExtractor"
    }
    
    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }
    
    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}

// Data structures matching Python implementation

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileAnalysis {
    #[serde(flatten)]
    metrics: FileMetrics,
    classes: Vec<ClassInfo>,
    functions: Vec<FunctionInfo>,
    dependencies: Vec<String>,
    complexity: i32,
    is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LanguageAnalysis {
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    dependencies: Vec<String>,
    complexity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line_number: usize,
    pub file_path: String,
    pub methods: Vec<String>,
    pub base_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_number: usize,
    pub file_path: String,
    pub complexity: i32,
    pub is_async: bool,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TopologySummary {
    total_files: usize,
    total_lines: usize,
    total_code_lines: usize,
    total_classes: usize,
    total_functions: usize,
    test_files: usize,
    language_distribution: HashMap<String, usize>,
    average_complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectStructure {
    max_depth: usize,
    directories: HashMap<String, usize>,
    total_directories: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CouplingMetrics {
    average_dependencies: f64,
    total_dependencies: usize,
    highly_coupled_files: Vec<(String, usize)>,
}

/// Tree-sitter AST analyzer for Python files (equivalent to PythonASTAnalyzer in Python)
struct PythonAstAnalyzer {
    file_path: String,
    codebase_path: PathBuf,
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    dependencies: Vec<String>,
    complexity: i32,
}

impl PythonAstAnalyzer {
    fn new(file_path: &Path, codebase_path: &Path) -> Self {
        let relative_path = file_path.strip_prefix(codebase_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        Self {
            file_path: relative_path,
            codebase_path: codebase_path.to_path_buf(),
            functions: Vec::new(),
            classes: Vec::new(),
            dependencies: Vec::new(),
            complexity: 1, // Minimum complexity
        }
    }

    fn visit_node(&mut self, node: tree_sitter::Node, source: &[u8]) {
        match node.kind() {
            "function_item" => self.visit_function_def(node, source),
            "struct_item" | "enum_item" | "trait_item" => self.visit_class_def(node, source),
            "use_declaration" => self.visit_import(node, source),
            // Complexity contributors
            "if_expression" | "while_expression" | "for_expression" |
            "match_expression" | "loop_expression" => {
                self.complexity += 1;
            }
            _ => {}
        }

        // Recursively visit children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child, source);
        }
    }

    fn visit_function_def(&mut self, node: tree_sitter::Node, source: &[u8]) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.node_text(name_node, source);

            // Extract parameters
            let mut parameters = Vec::new();
            if let Some(params_node) = node.child_by_field_name("parameters") {
                parameters = self.extract_parameters(params_node, source);
            }

            let function_info = FunctionInfo {
                name,
                line_number: node.start_position().row + 1,
                file_path: self.file_path.clone(),
                complexity: 1, // Basic complexity, will be updated by complexity analysis
                is_async: false,
                parameters,
                return_type: None, // TODO: Extract return type annotation
            };

            self.functions.push(function_info);
        }
    }

    fn visit_async_function_def(&mut self, node: tree_sitter::Node, source: &[u8]) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.node_text(name_node, source);

            // Extract parameters
            let mut parameters = Vec::new();
            if let Some(params_node) = node.child_by_field_name("parameters") {
                parameters = self.extract_parameters(params_node, source);
            }

            let function_info = FunctionInfo {
                name,
                line_number: node.start_position().row + 1,
                file_path: self.file_path.clone(),
                complexity: 1, // Basic complexity
                is_async: true,
                parameters,
                return_type: None,
            };

            self.functions.push(function_info);
        }
    }

    fn visit_class_def(&mut self, node: tree_sitter::Node, source: &[u8]) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.node_text(name_node, source);

            // Extract base classes
            let mut base_classes = Vec::new();
            if let Some(superclasses_node) = node.child_by_field_name("superclasses") {
                base_classes = self.extract_base_classes(superclasses_node, source);
            }

            // Extract methods (will be updated when we visit function definitions)
            let class_info = ClassInfo {
                name,
                line_number: node.start_position().row + 1,
                file_path: self.file_path.clone(),
                methods: Vec::new(), // Will be populated by function visitor
                base_classes,
            };

            self.classes.push(class_info);
        }
    }

    fn visit_import(&mut self, node: tree_sitter::Node, source: &[u8]) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let import_name = self.node_text(name_node, source);
            // Extract the root module name
            let root_module = import_name.split('.').next().unwrap_or(&import_name);
            self.dependencies.push(root_module.to_string());
        }
    }

    fn visit_import_from(&mut self, node: tree_sitter::Node, source: &[u8]) {
        if let Some(module_node) = node.child_by_field_name("module_name") {
            let module_name = self.node_text(module_node, source);
            // Extract the root module name
            let root_module = module_name.split('.').next().unwrap_or(&module_name);
            self.dependencies.push(root_module.to_string());
        }
    }

    fn extract_parameters(&self, params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();

        for child in params_node.children(&mut cursor) {
            if child.kind() == "identifier" {
                parameters.push(self.node_text(child, source));
            }
        }

        parameters
    }

    fn extract_base_classes(&self, superclasses_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
        let mut base_classes = Vec::new();
        let mut cursor = superclasses_node.walk();

        for child in superclasses_node.children(&mut cursor) {
            if child.kind() == "identifier" {
                base_classes.push(self.node_text(child, source));
            }
        }

        base_classes
    }

    fn node_text(&self, node: tree_sitter::Node, source: &[u8]) -> String {
        String::from_utf8_lossy(&source[node.start_byte()..node.end_byte()]).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_topology_extractor_creation() {
        let temp_dir = tempdir().unwrap();
        let extractor = TopologyExtractor::new(temp_dir.path()).unwrap();
        assert_eq!(extractor.codebase_path(), temp_dir.path());
        assert_eq!(extractor.extractor_type(), "TopologyExtractor");
    }

    #[test]
    fn test_extract_function_name() {
        let temp_dir = tempdir().unwrap();
        let extractor = TopologyExtractor::new(temp_dir.path()).unwrap();
        
        assert_eq!(extractor.extract_function_name("def hello_world():"), Some("hello_world".to_string()));
        assert_eq!(extractor.extract_function_name("async def async_func():"), Some("async_func".to_string()));
        assert_eq!(extractor.extract_function_name("    def indented():"), Some("indented".to_string()));
    }

    #[test]
    fn test_is_test_file() {
        let temp_dir = tempdir().unwrap();
        let extractor = TopologyExtractor::new(temp_dir.path()).unwrap();
        
        assert!(extractor.is_test_file(Path::new("test_example.py")));
        assert!(extractor.is_test_file(Path::new("example_test.py")));
        assert!(extractor.is_test_file(Path::new("tests/test_module.py")));
        assert!(!extractor.is_test_file(Path::new("regular_module.py")));
    }

    #[test]
    fn test_extract_dependency() {
        let temp_dir = tempdir().unwrap();
        let extractor = TopologyExtractor::new(temp_dir.path()).unwrap();
        
        assert_eq!(extractor.extract_dependency("import os"), Some("os".to_string()));
        assert_eq!(extractor.extract_dependency("from pathlib import Path"), Some("pathlib".to_string()));
        assert_eq!(extractor.extract_dependency("import collections.defaultdict"), Some("collections".to_string()));
    }
}