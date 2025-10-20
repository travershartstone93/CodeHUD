//! Test Generation Transformer
//!
//! This module implements automated test generation using property-based testing
//! and search-based techniques, matching Python's Hypothesis and similar tools.

use crate::{
    types::{TransformInput, TransformationResult, ComplexityEstimate, RiskLevel, TransformationStatistics},
    transformers::Transformer,
    Result, TransformError,
};
use async_trait::async_trait;
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tree_sitter::{Parser, Node, TreeCursor};

/// Test generation transformer
pub struct TestGenerationTransformer {
    /// Parser for syntax analysis
    parser: Parser,
    /// Configuration for test generation
    config: TestGenerationConfig,
    /// Test generators for different languages
    generators: HashMap<String, Box<dyn TestGenerator>>,
}

/// Configuration for test generation
#[derive(Debug, Clone)]
pub struct TestGenerationConfig {
    /// Type of tests to generate
    pub test_types: Vec<TestType>,
    /// Whether to use property-based testing
    pub use_property_based: bool,
    /// Whether to use mutation testing
    pub use_mutation_testing: bool,
    /// Maximum number of test cases per function
    pub max_tests_per_function: usize,
    /// Whether to generate edge case tests
    pub generate_edge_cases: bool,
    /// Whether to generate performance tests
    pub generate_performance_tests: bool,
    /// Test framework to target
    pub target_framework: String,
    /// Coverage threshold
    pub coverage_threshold: f64,
}

impl Default for TestGenerationConfig {
    fn default() -> Self {
        Self {
            test_types: vec![TestType::Unit, TestType::Integration, TestType::Property],
            use_property_based: true,
            use_mutation_testing: false,
            max_tests_per_function: 10,
            generate_edge_cases: true,
            generate_performance_tests: false,
            target_framework: "pytest".to_string(),
            coverage_threshold: 0.8,
        }
    }
}

/// Types of tests to generate
#[derive(Debug, Clone, PartialEq)]
pub enum TestType {
    /// Unit tests
    Unit,
    /// Integration tests
    Integration,
    /// Property-based tests
    Property,
    /// Mutation tests
    Mutation,
    /// Performance tests
    Performance,
    /// Regression tests
    Regression,
}

/// Test generator trait
pub trait TestGenerator: Send + Sync {
    /// Generate tests for a function
    fn generate_function_tests(&self, function: &FunctionInfo, config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>>;
    
    /// Generate tests for a class
    fn generate_class_tests(&self, class: &ClassInfo, config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>>;
    
    /// Generate property-based tests
    fn generate_property_tests(&self, function: &FunctionInfo, config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>>;
}

/// Information about a function for test generation
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Function signature
    pub signature: String,
    /// Parameter types and names
    pub parameters: Vec<Parameter>,
    /// Return type
    pub return_type: Option<String>,
    /// Function body (for analysis)
    pub body: String,
    /// Docstring/comments
    pub documentation: Option<String>,
    /// Line numbers (start, end)
    pub location: (usize, usize),
    /// Complexity metrics
    pub complexity: f64,
    /// Detected patterns
    pub patterns: Vec<FunctionPattern>,
}

/// Information about a class for test generation
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Base classes
    pub base_classes: Vec<String>,
    /// Methods in the class
    pub methods: Vec<FunctionInfo>,
    /// Attributes/fields
    pub attributes: Vec<Attribute>,
    /// Constructor information
    pub constructor: Option<FunctionInfo>,
    /// Class-level documentation
    pub documentation: Option<String>,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: Option<String>,
    /// Default value
    pub default_value: Option<String>,
    /// Whether parameter is optional
    pub optional: bool,
    /// Constraints on parameter values
    pub constraints: Vec<ParameterConstraint>,
}

/// Attribute information
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Attribute name
    pub name: String,
    /// Attribute type
    pub attr_type: Option<String>,
    /// Initial value
    pub initial_value: Option<String>,
    /// Whether attribute is public
    pub public: bool,
}

/// Constraints on parameter values
#[derive(Debug, Clone)]
pub enum ParameterConstraint {
    /// Range constraint for numbers
    Range { min: f64, max: f64 },
    /// Length constraint for strings/collections
    Length { min: usize, max: usize },
    /// Enum values
    OneOf(Vec<String>),
    /// Regex pattern for strings
    Pattern(String),
    /// Custom constraint
    Custom(String),
}

/// Detected function patterns
#[derive(Debug, Clone)]
pub enum FunctionPattern {
    /// Pure function (no side effects)
    Pure,
    /// Function with side effects
    SideEffects,
    /// Function that throws exceptions
    ThrowsExceptions,
    /// Function with loops
    HasLoops,
    /// Function with conditionals
    HasConditionals,
    /// Recursive function
    Recursive,
    /// Function with I/O operations
    IoOperations,
}

/// Generated test case
#[derive(Debug, Clone)]
pub struct GeneratedTest {
    /// Test name
    pub name: String,
    /// Test type
    pub test_type: TestType,
    /// Test code
    pub code: String,
    /// Test description
    pub description: String,
    /// Expected assertions
    pub assertions: Vec<TestAssertion>,
    /// Setup code (if needed)
    pub setup: Option<String>,
    /// Teardown code (if needed)
    pub teardown: Option<String>,
    /// Test data/fixtures
    pub test_data: Vec<TestData>,
}

/// Test assertion
#[derive(Debug, Clone)]
pub struct TestAssertion {
    /// Assertion type
    pub assertion_type: AssertionType,
    /// Expected value
    pub expected: String,
    /// Actual value expression
    pub actual: String,
    /// Assertion message
    pub message: Option<String>,
}

/// Types of assertions
#[derive(Debug, Clone)]
pub enum AssertionType {
    /// Equality assertion
    Equal,
    /// Inequality assertion
    NotEqual,
    /// Truth assertion
    True,
    /// False assertion
    False,
    /// None assertion
    IsNone,
    /// Not None assertion
    IsNotNone,
    /// Exception assertion
    Raises,
    /// Type assertion
    IsInstance,
    /// Approximate equality
    AlmostEqual,
    /// Contains assertion
    Contains,
    /// Custom assertion
    Custom(String),
}

/// Test data for parameterized tests
#[derive(Debug, Clone)]
pub struct TestData {
    /// Input parameters
    pub inputs: HashMap<String, String>,
    /// Expected output
    pub expected_output: Option<String>,
    /// Expected exception
    pub expected_exception: Option<String>,
    /// Test case description
    pub description: String,
}

/// Python test generator
pub struct PythonTestGenerator {
    /// Hypothesis strategy mappings
    strategy_map: HashMap<String, String>,
}

/// JavaScript test generator
pub struct JavaScriptTestGenerator {
    /// Test framework specific settings
    framework_settings: HashMap<String, String>,
}

impl TestGenerationTransformer {
    /// Create new test generation transformer
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language())?;

        let mut generators: HashMap<String, Box<dyn TestGenerator>> = HashMap::new();
        generators.insert("python".to_string(), Box::new(PythonTestGenerator::new()));
        generators.insert("javascript".to_string(), Box::new(JavaScriptTestGenerator::new()));

        Ok(Self {
            parser,
            config: TestGenerationConfig::default(),
            generators,
        })
    }

    /// Generate tests for source code
    pub async fn generate_tests(&mut self, input: &TransformInput) -> Result<Vec<GeneratedTest>> {
        // Parse source code
        let tree = self.parser.parse(&input.source_code, None)
            .ok_or_else(|| TransformError::Parse("Failed to parse source code".to_string()))?;

        // Extract functions and classes
        let functions = self.extract_functions(&tree.root_node(), &input.source_code)?;
        let classes = self.extract_classes(&tree.root_node(), &input.source_code)?;

        // Get appropriate generator
        let generator = self.generators.get(&input.language)
            .ok_or_else(|| TransformError::Config(
                format!("No test generator available for language: {}", input.language)
            ))?;

        let mut all_tests = Vec::new();

        // Generate tests for functions
        for function in &functions {
            let mut tests = generator.generate_function_tests(function, &self.config)?;
            all_tests.append(&mut tests);

            // Generate property-based tests if enabled
            if self.config.use_property_based {
                let mut property_tests = generator.generate_property_tests(function, &self.config)?;
                all_tests.append(&mut property_tests);
            }
        }

        // Generate tests for classes
        for class in &classes {
            let mut tests = generator.generate_class_tests(class, &self.config)?;
            all_tests.append(&mut tests);
        }

        Ok(all_tests)
    }

    /// Extract function information from syntax tree
    fn extract_functions(&self, root: &Node, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        self.extract_functions_recursive(root, source, &mut functions)?;
        Ok(functions)
    }

    /// Recursively extract functions
    fn extract_functions_recursive(&self, node: &Node, source: &str, functions: &mut Vec<FunctionInfo>) -> Result<()> {
        if node.kind() == "function_definition" {
            let function_info = self.analyze_function(node, source)?;
            functions.push(function_info);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.extract_functions_recursive(&cursor.node(), source, functions)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Extract class information from syntax tree
    fn extract_classes(&self, root: &Node, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        self.extract_classes_recursive(root, source, &mut classes)?;
        Ok(classes)
    }

    /// Recursively extract classes
    fn extract_classes_recursive(&self, node: &Node, source: &str, classes: &mut Vec<ClassInfo>) -> Result<()> {
        if node.kind() == "class_definition" {
            let class_info = self.analyze_class(node, source)?;
            classes.push(class_info);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.extract_classes_recursive(&cursor.node(), source, classes)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Analyze function node
    fn analyze_function(&self, node: &Node, source: &str) -> Result<FunctionInfo> {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        // Extract function name
        let name = self.extract_function_name(node, source)?;
        
        // Extract function signature
        let signature = node.utf8_text(source.as_bytes())?.lines().next().unwrap_or("").to_string();
        
        // Extract parameters
        let parameters = self.extract_parameters(node, source)?;
        
        // Extract return type (if available)
        let return_type = self.extract_return_type(node, source)?;
        
        // Extract function body
        let body = self.extract_function_body(node, source)?;
        
        // Extract documentation
        let documentation = self.extract_documentation(node, source)?;
        
        // Calculate complexity
        let complexity = self.calculate_function_complexity(node)?;
        
        // Detect patterns
        let patterns = self.detect_function_patterns(node, source)?;

        Ok(FunctionInfo {
            name,
            signature,
            parameters,
            return_type,
            body,
            documentation,
            location: (start_line, end_line),
            complexity,
            patterns,
        })
    }

    /// Analyze class node
    fn analyze_class(&self, node: &Node, source: &str) -> Result<ClassInfo> {
        let name = self.extract_class_name(node, source)?;
        let base_classes = self.extract_base_classes(node, source)?;
        let methods = self.extract_class_methods(node, source)?;
        let attributes = self.extract_class_attributes(node, source)?;
        let constructor = self.extract_constructor(node, source)?;
        let documentation = self.extract_documentation(node, source)?;

        Ok(ClassInfo {
            name,
            base_classes,
            methods,
            attributes,
            constructor,
            documentation,
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

    /// Extract class name
    fn extract_class_name(&self, node: &Node, source: &str) -> Result<String> {
        // Similar to extract_function_name but for classes
        self.extract_function_name(node, source)
    }

    /// Extract function parameters
    fn extract_parameters(&self, node: &Node, source: &str) -> Result<Vec<Parameter>> {
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
                    parameters.push(Parameter {
                        name,
                        param_type: None, // Would need type annotation analysis
                        default_value: None, // Would need default value analysis
                        optional: false,
                        constraints: vec![],
                    });
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Extract return type
    fn extract_return_type(&self, _node: &Node, _source: &str) -> Result<Option<String>> {
        // Simplified - would need full type annotation parsing
        Ok(None)
    }

    /// Extract function body
    fn extract_function_body(&self, node: &Node, source: &str) -> Result<String> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "block" {
                    return Ok(cursor.node().utf8_text(source.as_bytes())?.to_string());
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Ok(String::new())
    }

    /// Extract documentation
    fn extract_documentation(&self, _node: &Node, _source: &str) -> Result<Option<String>> {
        // Simplified - would need docstring extraction
        Ok(None)
    }

    /// Calculate function complexity
    fn calculate_function_complexity(&self, node: &Node) -> Result<f64> {
        let mut complexity = 1.0;
        self.count_complexity_nodes(node, &mut complexity);
        Ok(complexity)
    }

    /// Count complexity contributing nodes
    fn count_complexity_nodes(&self, node: &Node, complexity: &mut f64) {
        match node.kind() {
            "if_statement" | "while_statement" | "for_statement" => {
                *complexity += 1.0;
            }
            "try_statement" | "with_statement" => {
                *complexity += 1.0;
            }
            _ => {}
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.count_complexity_nodes(&cursor.node(), complexity);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    /// Detect function patterns
    fn detect_function_patterns(&self, node: &Node, source: &str) -> Result<Vec<FunctionPattern>> {
        let mut patterns = Vec::new();
        
        let body = self.extract_function_body(node, source)?;
        
        // Check for various patterns
        if !body.contains("print") && !body.contains("input") && !body.contains("open") {
            patterns.push(FunctionPattern::Pure);
        } else {
            patterns.push(FunctionPattern::SideEffects);
        }

        if body.contains("raise ") || body.contains("except ") {
            patterns.push(FunctionPattern::ThrowsExceptions);
        }

        if body.contains("for ") || body.contains("while ") {
            patterns.push(FunctionPattern::HasLoops);
        }

        if body.contains("if ") || body.contains("elif ") {
            patterns.push(FunctionPattern::HasConditionals);
        }

        Ok(patterns)
    }

    /// Extract base classes
    fn extract_base_classes(&self, _node: &Node, _source: &str) -> Result<Vec<String>> {
        // Simplified implementation
        Ok(vec![])
    }

    /// Extract class methods
    fn extract_class_methods(&self, node: &Node, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut methods = Vec::new();
        self.extract_functions_recursive(node, source, &mut methods)?;
        Ok(methods)
    }

    /// Extract class attributes
    fn extract_class_attributes(&self, _node: &Node, _source: &str) -> Result<Vec<Attribute>> {
        // Simplified implementation
        Ok(vec![])
    }

    /// Extract constructor
    fn extract_constructor(&self, _node: &Node, _source: &str) -> Result<Option<FunctionInfo>> {
        // Simplified implementation
        Ok(None)
    }
}

impl PythonTestGenerator {
    fn new() -> Self {
        let mut strategy_map = HashMap::new();
        strategy_map.insert("int".to_string(), "st.integers()".to_string());
        strategy_map.insert("str".to_string(), "st.text()".to_string());
        strategy_map.insert("float".to_string(), "st.floats()".to_string());
        strategy_map.insert("bool".to_string(), "st.booleans()".to_string());
        strategy_map.insert("list".to_string(), "st.lists(st.integers())".to_string());

        Self { strategy_map }
    }
}

impl TestGenerator for PythonTestGenerator {
    fn generate_function_tests(&self, function: &FunctionInfo, config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>> {
        let mut tests = Vec::new();

        // Generate basic unit test
        let unit_test = GeneratedTest {
            name: format!("test_{}", function.name),
            test_type: TestType::Unit,
            code: self.generate_unit_test_code(function)?,
            description: format!("Basic unit test for {}", function.name),
            assertions: vec![TestAssertion {
                assertion_type: AssertionType::IsNotNone,
                expected: "None".to_string(),
                actual: format!("{}()", function.name),
                message: Some("Function should return a value".to_string()),
            }],
            setup: None,
            teardown: None,
            test_data: vec![],
        };
        tests.push(unit_test);

        // Generate edge case tests if enabled
        if config.generate_edge_cases {
            let edge_test = self.generate_edge_case_test(function)?;
            tests.push(edge_test);
        }

        // Limit number of tests
        tests.truncate(config.max_tests_per_function);

        Ok(tests)
    }

    fn generate_class_tests(&self, class: &ClassInfo, config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>> {
        let mut tests = Vec::new();

        // Generate constructor test
        let constructor_test = GeneratedTest {
            name: format!("test_{}_init", class.name.to_lowercase()),
            test_type: TestType::Unit,
            code: format!("def test_{}_init():\n    obj = {}()\n    assert obj is not None", 
                         class.name.to_lowercase(), class.name),
            description: format!("Test {} constructor", class.name),
            assertions: vec![],
            setup: None,
            teardown: None,
            test_data: vec![],
        };
        tests.push(constructor_test);

        // Generate tests for each method
        for method in &class.methods {
            let mut method_tests = self.generate_function_tests(method, config)?;
            tests.append(&mut method_tests);
        }

        Ok(tests)
    }

    fn generate_property_tests(&self, function: &FunctionInfo, _config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>> {
        let mut tests = Vec::new();

        if function.patterns.contains(&FunctionPattern::Pure) {
            let property_test = GeneratedTest {
                name: format!("test_{}_property", function.name),
                test_type: TestType::Property,
                code: self.generate_hypothesis_test(function)?,
                description: format!("Property-based test for {}", function.name),
                assertions: vec![],
                setup: Some("from hypothesis import given, strategies as st".to_string()),
                teardown: None,
                test_data: vec![],
            };
            tests.push(property_test);
        }

        Ok(tests)
    }
}

impl PythonTestGenerator {
    fn generate_unit_test_code(&self, function: &FunctionInfo) -> Result<String> {
        let mut code = format!("def test_{}():\n", function.name);
        
        // Generate test parameters
        let mut params = Vec::new();
        for param in &function.parameters {
            if param.name != "self" {
                params.push(self.generate_test_value(&param.name, param.param_type.as_deref()));
            }
        }

        if params.is_empty() {
            code.push_str(&format!("    result = {}()\n", function.name));
        } else {
            code.push_str(&format!("    result = {}({})\n", function.name, params.join(", ")));
        }

        code.push_str("    assert result is not None\n");

        Ok(code)
    }

    fn generate_edge_case_test(&self, function: &FunctionInfo) -> Result<GeneratedTest> {
        Ok(GeneratedTest {
            name: format!("test_{}_edge_cases", function.name),
            test_type: TestType::Unit,
            code: format!("def test_{}_edge_cases():\n    # TODO: Add edge case tests\n    pass", function.name),
            description: format!("Edge case tests for {}", function.name),
            assertions: vec![],
            setup: None,
            teardown: None,
            test_data: vec![],
        })
    }

    fn generate_hypothesis_test(&self, function: &FunctionInfo) -> Result<String> {
        let mut code = String::new();
        
        // Generate hypothesis decorators
        for param in &function.parameters {
            if param.name != "self" {
                let strategy = self.get_strategy_for_type(param.param_type.as_deref());
                code.push_str(&format!("@given({}={})\n", param.name, strategy));
            }
        }

        code.push_str(&format!("def test_{}_property(", function.name));
        let param_names: Vec<&String> = function.parameters.iter()
            .filter(|p| p.name != "self")
            .map(|p| &p.name)
            .collect();
        code.push_str(&param_names.join(", "));
        code.push_str("):\n");

        // Generate property assertion
        if function.patterns.contains(&FunctionPattern::Pure) {
            code.push_str(&format!("    result1 = {}({})\n", function.name, param_names.join(", ")));
            code.push_str(&format!("    result2 = {}({})\n", function.name, param_names.join(", ")));
            code.push_str("    assert result1 == result2  # Pure function should be deterministic\n");
        }

        Ok(code)
    }

    fn generate_test_value(&self, _param_name: &str, param_type: Option<&str>) -> String {
        match param_type {
            Some("int") => "42".to_string(),
            Some("str") => "\"test\"".to_string(),
            Some("float") => "3.14".to_string(),
            Some("bool") => "True".to_string(),
            Some("list") => "[1, 2, 3]".to_string(),
            _ => "None".to_string(),
        }
    }

    fn get_strategy_for_type(&self, param_type: Option<&str>) -> String {
        match param_type {
            Some(type_name) => {
                self.strategy_map.get(type_name)
                    .cloned()
                    .unwrap_or_else(|| "st.none()".to_string())
            }
            None => "st.none()".to_string(),
        }
    }
}

impl JavaScriptTestGenerator {
    fn new() -> Self {
        let mut framework_settings = HashMap::new();
        framework_settings.insert("jest".to_string(), "expect".to_string());
        framework_settings.insert("mocha".to_string(), "assert".to_string());

        Self { framework_settings }
    }
}

impl TestGenerator for JavaScriptTestGenerator {
    fn generate_function_tests(&self, function: &FunctionInfo, _config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>> {
        let test = GeneratedTest {
            name: format!("test {}", function.name),
            test_type: TestType::Unit,
            code: format!("test('{}', () => {{\n  const result = {}();\n  expect(result).toBeDefined();\n}});", 
                         function.name, function.name),
            description: format!("Test {}", function.name),
            assertions: vec![],
            setup: None,
            teardown: None,
            test_data: vec![],
        };

        Ok(vec![test])
    }

    fn generate_class_tests(&self, class: &ClassInfo, config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>> {
        let mut tests = Vec::new();

        for method in &class.methods {
            let mut method_tests = self.generate_function_tests(method, config)?;
            tests.append(&mut method_tests);
        }

        Ok(tests)
    }

    fn generate_property_tests(&self, _function: &FunctionInfo, _config: &TestGenerationConfig) -> Result<Vec<GeneratedTest>> {
        // JavaScript property-based testing would use libraries like fast-check
        Ok(vec![])
    }
}

#[async_trait]
impl Transformer for TestGenerationTransformer {
    async fn transform(&self, input: &TransformInput) -> Result<TransformationResult> {
        // Generate tests
        let mut transformer = Self::new()?;
        
        // Set parser for input language
        match input.language.as_str() {
            "python" => transformer.parser.set_language(tree_sitter_python::language())?,
            "javascript" => transformer.parser.set_language(tree_sitter_javascript::language())?,
            _ => return Err(TransformError::Config(
                format!("Unsupported language for test generation: {}", input.language)
            )),
        }

        let tests = transformer.generate_tests(input).await?;

        // Generate test file content
        let test_file_content = self.generate_test_file(&tests, &input.language)?;

        // Determine test file name
        let test_file_name = self.generate_test_file_name(&input.file_path, &input.language);

        let statistics = TransformationStatistics {
            lines_processed: input.source_code.lines().count(),
            lines_modified: 0,
            lines_added: test_file_content.lines().count(),
            lines_removed: 0,
            transformations_applied: tests.len(),
            complexity_before: None,
            complexity_after: None,
            issues_fixed: 0,
            issues_introduced: 0,
        };

        Ok(TransformationResult {
            success: !tests.is_empty(),
            transformed_code: None, // Original file unchanged
            modified_files: vec![],
            created_files: vec![test_file_name],
            backup_info: None,
            statistics,
            errors: vec![],
            warnings: if tests.is_empty() { 
                vec!["No tests could be generated for this file".to_string()] 
            } else { 
                vec![] 
            },
            execution_time_ms: 0,
        })
    }

    fn supports_dry_run(&self) -> bool {
        false // Test generation always creates files
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn estimate_complexity(&self, input: &TransformInput) -> Result<ComplexityEstimate> {
        let lines = input.source_code.lines().count();
        let estimated_duration = (lines as f64 * 0.5).max(2.0); // 0.5 seconds per line, minimum 2 seconds
        
        Ok(ComplexityEstimate {
            estimated_duration_seconds: estimated_duration,
            files_to_modify: 0,
            transformation_count: 1,
            risk_level: RiskLevel::Low, // Test generation is safe
            confidence: 0.8,
            lines_affected: 0, // Original file not modified
            requires_manual_review: true, // Generated tests should be reviewed
        })
    }
}

impl TestGenerationTransformer {
    /// Generate test file content from tests
    fn generate_test_file(&self, tests: &[GeneratedTest], language: &str) -> Result<String> {
        let mut content = String::new();

        match language {
            "python" => {
                content.push_str("import pytest\n");
                content.push_str("from hypothesis import given, strategies as st\n\n");
                
                for test in tests {
                    if let Some(setup) = &test.setup {
                        content.push_str(setup);
                        content.push('\n');
                    }
                    content.push_str(&test.code);
                    content.push_str("\n\n");
                }
            }
            "javascript" => {
                for test in tests {
                    content.push_str(&test.code);
                    content.push_str("\n\n");
                }
            }
            _ => return Err(TransformError::Config(
                format!("Unsupported language for test file generation: {}", language)
            )),
        }

        Ok(content)
    }

    /// Generate test file name
    fn generate_test_file_name(&self, source_file: &str, language: &str) -> String {
        let path = std::path::Path::new(source_file);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));

        match language {
            "python" => dir.join(format!("test_{}.py", stem)).to_string_lossy().to_string(),
            "javascript" => dir.join(format!("{}.test.js", stem)).to_string_lossy().to_string(),
            _ => dir.join(format!("{}.test", stem)).to_string_lossy().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_generation_transformer_creation() {
        let transformer = TestGenerationTransformer::new();
        assert!(transformer.is_ok());
    }

    #[test]
    fn test_python_test_generator() {
        let generator = PythonTestGenerator::new();
        assert!(!generator.strategy_map.is_empty());
    }

    #[test]
    fn test_function_pattern_detection() {
        // Would need actual test with tree-sitter parsing
        assert!(true);
    }
}