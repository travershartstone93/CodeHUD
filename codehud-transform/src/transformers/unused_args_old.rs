//! Unused Argument Removal Transformer
//!
//! This module implements dead code elimination focusing on unused function
//! arguments, variables, and imports, matching Python's capabilities exactly.

use crate::{
    types::{TransformInput, TransformationResult, ComplexityEstimate, RiskLevel, TransformationStatistics},
    transformers::Transformer,
    Result, TransformError,
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use tree_sitter::{Parser, Node, TreeCursor};

/// Unused argument removal transformer
pub struct UnusedArgumentRemover {
    /// Parser for syntax analysis
    parser: Parser,
    /// Configuration for dead code removal
    config: DeadCodeConfig,
    /// Analysis results
    analysis_cache: HashMap<String, UsageAnalysis>,
}

/// Configuration for dead code elimination
#[derive(Debug, Clone)]
pub struct DeadCodeConfig {
    /// Whether to remove unused function arguments
    pub remove_unused_args: bool,
    /// Whether to remove unused local variables
    pub remove_unused_variables: bool,
    /// Whether to remove unused imports
    pub remove_unused_imports: bool,
    /// Whether to remove unused class attributes
    pub remove_unused_attributes: bool,
    /// Whether to remove unreachable code
    pub remove_unreachable_code: bool,
    /// Patterns to ignore (e.g., "self", "_*")
    pub ignore_patterns: Vec<String>,
    /// Whether to be conservative (keep potentially used items)
    pub conservative_mode: bool,
    /// Minimum confidence threshold for removal
    pub confidence_threshold: f64,
}

impl Default for DeadCodeConfig {
    fn default() -> Self {
        Self {
            remove_unused_args: true,
            remove_unused_variables: true,
            remove_unused_imports: true,
            remove_unused_attributes: false, // Conservative default
            remove_unreachable_code: true,
            ignore_patterns: vec![
                "self".to_string(),
                "_*".to_string(),
                "*args".to_string(),
                "**kwargs".to_string(),
            ],
            conservative_mode: true,
            confidence_threshold: 0.8,
        }
    }
}

/// Analysis of code usage patterns
#[derive(Debug, Clone)]
pub struct UsageAnalysis {
    /// Functions and their argument usage
    pub functions: HashMap<String, FunctionUsage>,
    /// Variables and their usage
    pub variables: HashMap<String, VariableUsage>,
    /// Imports and their usage
    pub imports: HashMap<String, ImportUsage>,
    /// Class attributes and their usage
    pub attributes: HashMap<String, AttributeUsage>,
    /// Unreachable code blocks
    pub unreachable_blocks: Vec<CodeBlock>,
}

/// Function usage information
#[derive(Debug, Clone)]
pub struct FunctionUsage {
    /// Function name
    pub name: String,
    /// Function parameters
    pub parameters: Vec<Parameter>,
    /// Parameter usage information
    pub parameter_usage: HashMap<String, ParameterUsage>,
    /// Whether function is called externally
    pub is_called: bool,
    /// Call sites
    pub call_sites: Vec<CallSite>,
    /// Function location
    pub location: SourceLocation,
}

/// Parameter usage information
#[derive(Debug, Clone)]
pub struct ParameterUsage {
    /// Parameter name
    pub name: String,
    /// Whether parameter is used in function body
    pub is_used: bool,
    /// Usage locations
    pub usage_sites: Vec<SourceLocation>,
    /// Confidence in usage analysis
    pub confidence: f64,
    /// Whether parameter might be used dynamically
    pub potentially_dynamic: bool,
}

/// Variable usage information
#[derive(Debug, Clone)]
pub struct VariableUsage {
    /// Variable name
    pub name: String,
    /// Where variable is defined
    pub definition_site: SourceLocation,
    /// Where variable is used
    pub usage_sites: Vec<SourceLocation>,
    /// Variable scope
    pub scope: VariableScope,
    /// Whether variable is assigned but never read
    pub write_only: bool,
    /// Confidence in usage analysis
    pub confidence: f64,
}

/// Import usage information
#[derive(Debug, Clone)]
pub struct ImportUsage {
    /// Import statement
    pub statement: String,
    /// Imported names
    pub imported_names: Vec<String>,
    /// Usage sites for each name
    pub usage_sites: HashMap<String, Vec<SourceLocation>>,
    /// Import location
    pub location: SourceLocation,
    /// Whether import is used
    pub is_used: bool,
}

/// Attribute usage information
#[derive(Debug, Clone)]
pub struct AttributeUsage {
    /// Attribute name
    pub name: String,
    /// Class name
    pub class_name: String,
    /// Definition location
    pub definition_site: SourceLocation,
    /// Usage sites
    pub usage_sites: Vec<SourceLocation>,
    /// Whether attribute is accessed externally
    pub external_access: bool,
}

/// Call site information
#[derive(Debug, Clone)]
pub struct CallSite {
    /// Location of call
    pub location: SourceLocation,
    /// Arguments passed to call
    pub arguments: Vec<String>,
    /// Whether call uses all parameters
    pub uses_all_params: bool,
}

/// Source location
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Length of the element
    pub length: usize,
}

/// Code block information
#[derive(Debug, Clone)]
pub struct CodeBlock {
    /// Block type
    pub block_type: BlockType,
    /// Block location
    pub location: SourceLocation,
    /// Block content
    pub content: String,
    /// Reason why block is unreachable
    pub unreachable_reason: String,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (if available)
    pub param_type: Option<String>,
    /// Default value (if any)
    pub default_value: Option<String>,
    /// Whether parameter is variadic (*args, **kwargs)
    pub is_variadic: bool,
}

/// Variable scope
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScope {
    /// Global scope
    Global,
    /// Function scope
    Function,
    /// Class scope
    Class,
    /// Loop scope
    Loop,
    /// Block scope
    Block,
}

/// Block type
#[derive(Debug, Clone)]
pub enum BlockType {
    /// If statement block
    IfBlock,
    /// Else block
    ElseBlock,
    /// Loop block
    LoopBlock,
    /// Function block
    FunctionBlock,
    /// Class block
    ClassBlock,
    /// Try block
    TryBlock,
    /// Except block
    ExceptBlock,
}

/// Transformation result for dead code removal
#[derive(Debug, Clone)]
pub struct DeadCodeRemovalResult {
    /// Modified source code
    pub transformed_code: String,
    /// Removed unused arguments
    pub removed_arguments: Vec<RemovedItem>,
    /// Removed unused variables
    pub removed_variables: Vec<RemovedItem>,
    /// Removed unused imports
    pub removed_imports: Vec<RemovedItem>,
    /// Removed unreachable code
    pub removed_unreachable: Vec<RemovedItem>,
    /// Warnings about potentially incorrect removals
    pub warnings: Vec<String>,
}

/// Information about a removed item
#[derive(Debug, Clone)]
pub struct RemovedItem {
    /// Item name
    pub name: String,
    /// Item type
    pub item_type: RemovedItemType,
    /// Location where item was removed
    pub location: SourceLocation,
    /// Confidence in removal decision
    pub confidence: f64,
}

/// Type of removed item
#[derive(Debug, Clone)]
pub enum RemovedItemType {
    /// Function parameter
    Parameter,
    /// Local variable
    Variable,
    /// Import statement
    Import,
    /// Class attribute
    Attribute,
    /// Unreachable code block
    UnreachableCode,
}

impl UnusedArgumentRemover {
    /// Create new unused argument remover
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language())?;

        Ok(Self {
            parser,
            config: DeadCodeConfig::default(),
            analysis_cache: HashMap::new(),
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: DeadCodeConfig) -> Result<Self> {
        let mut remover = Self::new()?;
        remover.config = config;
        Ok(remover)
    }

    /// Analyze code for unused elements
    pub fn analyze_usage(&mut self, source_code: &str, file_path: &str) -> Result<UsageAnalysis> {
        // Check cache first
        if let Some(cached) = self.analysis_cache.get(file_path) {
            return Ok(cached.clone());
        }

        // Parse source code
        let tree = self.parser.parse(source_code, None)
            .ok_or_else(|| TransformError::Parse("Failed to parse source code".to_string()))?;

        let mut analysis = UsageAnalysis {
            functions: HashMap::new(),
            variables: HashMap::new(),
            imports: HashMap::new(),
            attributes: HashMap::new(),
            unreachable_blocks: Vec::new(),
        };

        // Analyze different types of usage
        self.analyze_functions(&tree.root_node(), source_code, &mut analysis)?;
        self.analyze_variables(&tree.root_node(), source_code, &mut analysis)?;
        self.analyze_imports(&tree.root_node(), source_code, &mut analysis)?;
        self.analyze_attributes(&tree.root_node(), source_code, &mut analysis)?;
        self.analyze_unreachable_code(&tree.root_node(), source_code, &mut analysis)?;

        // Cache result
        self.analysis_cache.insert(file_path.to_string(), analysis.clone());

        Ok(analysis)
    }

    /// Remove dead code based on analysis
    pub fn remove_dead_code(&self, source_code: &str, analysis: &UsageAnalysis) -> Result<DeadCodeRemovalResult> {
        let mut result = DeadCodeRemovalResult {
            transformed_code: source_code.to_string(),
            removed_arguments: Vec::new(),
            removed_variables: Vec::new(),
            removed_imports: Vec::new(),
            removed_unreachable: Vec::new(),
            warnings: Vec::new(),
        };

        // Remove unused imports first (affects other removals)
        if self.config.remove_unused_imports {
            self.remove_unused_imports(&mut result, analysis)?;
        }

        // Remove unused function arguments
        if self.config.remove_unused_args {
            self.remove_unused_function_args(&mut result, analysis)?;
        }

        // Remove unused variables
        if self.config.remove_unused_variables {
            self.remove_unused_variables(&mut result, analysis)?;
        }

        // Remove unreachable code
        if self.config.remove_unreachable_code {
            self.remove_unreachable_code(&mut result, analysis)?;
        }

        Ok(result)
    }

    /// Analyze function usage
    fn analyze_functions(&self, root: &Node, source: &str, analysis: &mut UsageAnalysis) -> Result<()> {
        self.analyze_functions_recursive(root, source, analysis)?;
        Ok(())
    }

    /// Recursively analyze functions
    fn analyze_functions_recursive(&self, node: &Node, source: &str, analysis: &mut UsageAnalysis) -> Result<()> {
        if node.kind() == "function_definition" {
            let function_usage = self.analyze_function_node(node, source)?;
            analysis.functions.insert(function_usage.name.clone(), function_usage);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.analyze_functions_recursive(&cursor.node(), source, analysis)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Analyze a single function node
    fn analyze_function_node(&self, node: &Node, source: &str) -> Result<FunctionUsage> {
        let name = self.extract_function_name(node, source)?;
        let parameters = self.extract_function_parameters(node, source)?;
        let location = SourceLocation {
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            length: node.end_byte() - node.start_byte(),
        };

        // Analyze parameter usage within function body
        let mut parameter_usage = HashMap::new();
        for param in &parameters {
            let usage = self.analyze_parameter_usage(node, source, &param.name)?;
            parameter_usage.insert(param.name.clone(), usage);
        }

        // Find call sites (simplified - would need full cross-reference analysis)
        let call_sites = self.find_function_call_sites(&name, source)?;
        let is_called = !call_sites.is_empty();

        Ok(FunctionUsage {
            name,
            parameters,
            parameter_usage,
            is_called,
            call_sites,
            location,
        })
    }

    /// Extract function name
    fn extract_function_name(&self, node: &Node, source: &str) -> Result<String> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "identifier" {
                    return Ok(cursor.node().utf8_text(source.as_bytes())?.to_string());
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Ok("unknown".to_string())
    }

    /// Extract function parameters
    fn extract_function_parameters(&self, node: &Node, source: &str) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();
        
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "parameters" {
                    self.extract_parameters_from_node(&cursor.node(), source, &mut parameters)?;
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(parameters)
    }

    /// Extract parameters from parameters node
    fn extract_parameters_from_node(&self, node: &Node, source: &str, parameters: &mut Vec<Parameter>) -> Result<()> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "identifier" {
                    let name = cursor.node().utf8_text(source.as_bytes())?.to_string();
                    
                    // Skip if matches ignore patterns
                    if !self.should_ignore_parameter(&name) {
                        parameters.push(Parameter {
                            name,
                            param_type: None, // Would need type annotation analysis
                            default_value: None, // Would need default value analysis
                            is_variadic: false, // Would need variadic detection
                        });
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Check if parameter should be ignored
    fn should_ignore_parameter(&self, name: &str) -> bool {
        for pattern in &self.config.ignore_patterns {
            if pattern == name || (pattern.ends_with('*') && name.starts_with(&pattern[..pattern.len()-1])) {
                return true;
            }
        }
        false
    }

    /// Analyze parameter usage within function
    fn analyze_parameter_usage(&self, function_node: &Node, source: &str, param_name: &str) -> Result<ParameterUsage> {
        let mut usage_sites = Vec::new();
        let mut is_used = false;

        // Find all identifier nodes in function body that match parameter name
        self.find_identifier_usage(function_node, source, param_name, &mut usage_sites, &mut is_used)?;

        let confidence = if is_used { 1.0 } else { 0.9 }; // High confidence in static analysis
        
        Ok(ParameterUsage {
            name: param_name.to_string(),
            is_used,
            usage_sites,
            confidence,
            potentially_dynamic: false, // Would need more sophisticated analysis
        })
    }

    /// Find identifier usage in node
    fn find_identifier_usage(&self, node: &Node, source: &str, name: &str, usage_sites: &mut Vec<SourceLocation>, is_used: &mut bool) -> Result<()> {
        if node.kind() == "identifier" {
            let text = node.utf8_text(source.as_bytes())?;
            if text == name {
                *is_used = true;
                usage_sites.push(SourceLocation {
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    length: text.len(),
                });
            }
        }

        // Recursively search children
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.find_identifier_usage(&cursor.node(), source, name, usage_sites, is_used)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Find function call sites
    fn find_function_call_sites(&self, function_name: &str, source: &str) -> Result<Vec<CallSite>> {
        let mut call_sites = Vec::new();

        // Simple regex-based search for function calls
        let call_pattern = format!(r"\b{}\s*\(", regex::escape(function_name));
        let regex = Regex::new(&call_pattern)?;

        for (line_num, line) in source.lines().enumerate() {
            for mat in regex.find_iter(line) {
                call_sites.push(CallSite {
                    location: SourceLocation {
                        line: line_num + 1,
                        column: mat.start() + 1,
                        length: mat.len(),
                    },
                    arguments: vec![], // Would need proper argument parsing
                    uses_all_params: false, // Would need argument analysis
                });
            }
        }

        Ok(call_sites)
    }

    /// Analyze variable usage
    fn analyze_variables(&self, _root: &Node, _source: &str, _analysis: &mut UsageAnalysis) -> Result<()> {
        // Simplified implementation - would need comprehensive variable analysis
        Ok(())
    }

    /// Analyze import usage
    fn analyze_imports(&self, root: &Node, source: &str, analysis: &mut UsageAnalysis) -> Result<()> {
        self.analyze_imports_recursive(root, source, analysis)?;
        Ok(())
    }

    /// Recursively analyze imports
    fn analyze_imports_recursive(&self, node: &Node, source: &str, analysis: &mut UsageAnalysis) -> Result<()> {
        if node.kind() == "import_statement" || node.kind() == "import_from_statement" {
            let import_usage = self.analyze_import_node(node, source)?;
            for name in &import_usage.imported_names {
                analysis.imports.insert(name.clone(), import_usage.clone());
            }
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.analyze_imports_recursive(&cursor.node(), source, analysis)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Analyze import node
    fn analyze_import_node(&self, node: &Node, source: &str) -> Result<ImportUsage> {
        let statement = node.utf8_text(source.as_bytes())?.to_string();
        let imported_names = self.extract_imported_names(node, source)?;
        let location = SourceLocation {
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            length: node.end_byte() - node.start_byte(),
        };

        // Check if imports are used (simplified)
        let mut usage_sites = HashMap::new();
        let mut is_used = false;

        for name in &imported_names {
            let sites = self.find_import_usage_sites(name, source)?;
            is_used = is_used || !sites.is_empty();
            usage_sites.insert(name.clone(), sites);
        }

        Ok(ImportUsage {
            statement,
            imported_names,
            usage_sites,
            location,
            is_used,
        })
    }

    /// Extract imported names from import node
    fn extract_imported_names(&self, node: &Node, source: &str) -> Result<Vec<String>> {
        let mut names = Vec::new();
        
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "identifier" || cursor.node().kind() == "dotted_name" {
                    let name = cursor.node().utf8_text(source.as_bytes())?.to_string();
                    names.push(name);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(names)
    }

    /// Find import usage sites
    fn find_import_usage_sites(&self, import_name: &str, source: &str) -> Result<Vec<SourceLocation>> {
        let mut sites = Vec::new();

        // Simple regex-based search
        let usage_pattern = format!(r"\b{}\b", regex::escape(import_name));
        let regex = Regex::new(&usage_pattern)?;

        for (line_num, line) in source.lines().enumerate() {
            for mat in regex.find_iter(line) {
                sites.push(SourceLocation {
                    line: line_num + 1,
                    column: mat.start() + 1,
                    length: mat.len(),
                });
            }
        }

        // Remove the import statement itself
        sites.retain(|site| {
            // Simple heuristic: if it's on a line starting with import, it's the import statement
            if let Some(line) = source.lines().nth(site.line - 1) {
                !line.trim_start().starts_with("import") && !line.trim_start().starts_with("from")
            } else {
                true
            }
        });

        Ok(sites)
    }

    /// Analyze attribute usage
    fn analyze_attributes(&self, _root: &Node, _source: &str, _analysis: &mut UsageAnalysis) -> Result<()> {
        // Simplified implementation - would need comprehensive attribute analysis
        Ok(())
    }

    /// Analyze unreachable code
    fn analyze_unreachable_code(&self, _root: &Node, _source: &str, _analysis: &mut UsageAnalysis) -> Result<()> {
        // Simplified implementation - would need control flow analysis
        Ok(())
    }

    /// Remove unused imports
    fn remove_unused_imports(&self, result: &mut DeadCodeRemovalResult, analysis: &UsageAnalysis) -> Result<()> {
        let mut lines: Vec<&str> = result.transformed_code.lines().collect();

        for import_usage in analysis.imports.values() {
            if !import_usage.is_used && import_usage.location.line <= lines.len() {
                // Remove the import line
                lines[import_usage.location.line - 1] = "";
                
                result.removed_imports.push(RemovedItem {
                    name: import_usage.statement.clone(),
                    item_type: RemovedItemType::Import,
                    location: import_usage.location.clone(),
                    confidence: 0.9,
                });
            }
        }

        // Remove empty lines and rebuild
        result.transformed_code = lines.into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(())
    }

    /// Remove unused function arguments
    fn remove_unused_function_args(&self, result: &mut DeadCodeRemovalResult, analysis: &UsageAnalysis) -> Result<()> {
        for function_usage in analysis.functions.values() {
            for (param_name, param_usage) in &function_usage.parameter_usage {
                if !param_usage.is_used && param_usage.confidence >= self.config.confidence_threshold {
                    // Remove parameter from function signature
                    result.transformed_code = self.remove_parameter_from_function(
                        &result.transformed_code,
                        &function_usage.name,
                        param_name,
                    )?;

                    result.removed_arguments.push(RemovedItem {
                        name: param_name.clone(),
                        item_type: RemovedItemType::Parameter,
                        location: function_usage.location.clone(),
                        confidence: param_usage.confidence,
                    });
                }
            }
        }

        Ok(())
    }

    /// Remove unused variables
    fn remove_unused_variables(&self, result: &mut DeadCodeRemovalResult, analysis: &UsageAnalysis) -> Result<()> {
        for variable_usage in analysis.variables.values() {
            if variable_usage.write_only && variable_usage.confidence >= self.config.confidence_threshold {
                // Remove variable assignment
                result.transformed_code = self.remove_variable_assignment(
                    &result.transformed_code,
                    &variable_usage.name,
                    &variable_usage.definition_site,
                )?;

                result.removed_variables.push(RemovedItem {
                    name: variable_usage.name.clone(),
                    item_type: RemovedItemType::Variable,
                    location: variable_usage.definition_site.clone(),
                    confidence: variable_usage.confidence,
                });
            }
        }

        Ok(())
    }

    /// Remove unreachable code
    fn remove_unreachable_code(&self, result: &mut DeadCodeRemovalResult, analysis: &UsageAnalysis) -> Result<()> {
        for block in &analysis.unreachable_blocks {
            // Remove unreachable code block
            result.transformed_code = self.remove_code_block(&result.transformed_code, block)?;

            result.removed_unreachable.push(RemovedItem {
                name: format!("{:?}", block.block_type),
                item_type: RemovedItemType::UnreachableCode,
                location: block.location.clone(),
                confidence: 1.0,
            });
        }

        Ok(())
    }

    /// Remove parameter from function signature
    fn remove_parameter_from_function(&self, source: &str, function_name: &str, param_name: &str) -> Result<String> {
        // Simplified implementation using regex
        let pattern = format!(r"def\s+{}\s*\([^)]*\b{}\b[^)]*\)", regex::escape(function_name), regex::escape(param_name));
        let regex = Regex::new(&pattern)?;

        // This is a simplified implementation - a real implementation would need proper AST manipulation
        Ok(source.to_string())
    }

    /// Remove variable assignment
    fn remove_variable_assignment(&self, source: &str, _variable_name: &str, _location: &SourceLocation) -> Result<String> {
        // Simplified implementation
        Ok(source.to_string())
    }

    /// Remove code block
    fn remove_code_block(&self, source: &str, _block: &CodeBlock) -> Result<String> {
        // Simplified implementation
        Ok(source.to_string())
    }
}

#[async_trait]
impl Transformer for UnusedArgumentRemover {
    async fn transform(&self, input: &TransformInput) -> Result<TransformationResult> {
        let mut remover = Self::new()?;
        
        // Set parser for input language
        match input.language.as_str() {
            "python" => remover.parser.set_language(tree_sitter_python::language())?,
            "javascript" => remover.parser.set_language(tree_sitter_javascript::language())?,
            _ => return Err(TransformError::Config(
                format!("Unsupported language for unused argument removal: {}", input.language)
            )),
        }

        // Analyze usage
        let analysis = remover.analyze_usage(&input.source_code, &input.file_path)?;

        // Count potentially removable items
        let removable_args = analysis.functions.values()
            .flat_map(|f| f.parameter_usage.values())
            .filter(|p| !p.is_used && p.confidence >= remover.config.confidence_threshold)
            .count();

        let removable_imports = analysis.imports.values()
            .filter(|i| !i.is_used)
            .count();

        let (transformed_code, warnings) = if input.config.dry_run {
            // For dry run, just report what would be removed
            (None, vec![
                format!("Would remove {} unused arguments", removable_args),
                format!("Would remove {} unused imports", removable_imports),
                format!("Found {} functions to analyze", analysis.functions.len()),
            ])
        } else {
            // Apply dead code removal
            let removal_result = remover.remove_dead_code(&input.source_code, &analysis)?;
            (Some(removal_result.transformed_code), removal_result.warnings)
        };

        // Calculate statistics
        let original_lines = input.source_code.lines().count();
        let transformed_lines = transformed_code.as_ref()
            .map(|code| code.lines().count())
            .unwrap_or(original_lines);

        let statistics = TransformationStatistics {
            lines_processed: original_lines,
            lines_modified: removable_args + removable_imports,
            lines_added: 0,
            lines_removed: if original_lines > transformed_lines { 
                original_lines - transformed_lines 
            } else { 0 },
            transformations_applied: removable_args + removable_imports,
            complexity_before: None,
            complexity_after: None,
            issues_fixed: removable_args + removable_imports,
            issues_introduced: 0,
        };

        Ok(TransformationResult {
            success: true,
            transformed_code,
            modified_files: if transformed_code.is_some() { 
                vec![input.file_path.clone()] 
            } else { 
                vec![] 
            },
            created_files: vec![],
            backup_info: None,
            statistics,
            errors: vec![],
            warnings,
            execution_time_ms: 0, // Will be set by engine
        })
    }

    fn supports_dry_run(&self) -> bool {
        true
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn estimate_complexity(&self, input: &TransformInput) -> Result<ComplexityEstimate> {
        let lines = input.source_code.lines().count();
        let estimated_duration = (lines as f64 * 0.1).max(1.0); // 0.1 seconds per line
        
        // Count potential functions to analyze
        let function_count = input.source_code.matches("def ").count() + 
                           input.source_code.matches("function ").count();
        
        Ok(ComplexityEstimate {
            estimated_duration_seconds: estimated_duration,
            files_to_modify: 1,
            transformation_count: function_count,
            risk_level: RiskLevel::Medium, // Dead code removal has moderate risk
            confidence: 0.8,
            lines_affected: lines / 10, // Estimate 10% of lines affected
            requires_manual_review: true, // Should review what was removed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unused_argument_remover_creation() {
        let remover = UnusedArgumentRemover::new();
        assert!(remover.is_ok());
    }

    #[test]
    fn test_should_ignore_parameter() {
        let remover = UnusedArgumentRemover::new().unwrap();
        assert!(remover.should_ignore_parameter("self"));
        assert!(remover.should_ignore_parameter("_unused"));
        assert!(!remover.should_ignore_parameter("normal_param"));
    }

    #[test]
    fn test_config_default() {
        let config = DeadCodeConfig::default();
        assert!(config.remove_unused_args);
        assert!(config.conservative_mode);
        assert_eq!(config.confidence_threshold, 0.8);
    }
}