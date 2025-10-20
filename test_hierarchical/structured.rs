//! Structured Code Generation with Constraints
//!
//! This module provides structured code generation with JSON schema validation,
//! grammar-based constraints, and constitutional AI guardrails, preserving the
//! Python implementation's exact behavior during Phase 5a.

#[cfg(feature = "candle")]
use crate::{LlmConfig, LlmResult, LlmError, ffi::PythonLlmBridge, native::NativeLlmEngine};

#[cfg(not(feature = "candle"))]
use crate::{LlmConfig, LlmResult, LlmError, ffi::PythonLlmBridge, native_stub::NativeLlmEngine};
use jsonschema::{JSONSchema, Draft};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Instant;

/// Generation constraints for structured output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConstraints {
    /// JSON schema for output validation
    pub json_schema: Option<Value>,
    /// Grammar rules for structured generation
    pub grammar_rules: Option<String>,
    /// Maximum output length
    pub max_length: Option<usize>,
    /// Expected output format
    pub output_format: OutputFormat,
    /// Additional validation rules
    pub validation_rules: Vec<String>,
}

/// Output format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Plain text output
    PlainText,
    /// Text output (alias for PlainText)
    Text,
    /// Python source code
    PythonCode,
    /// Rust source code
    RustCode,
    /// JavaScript source code
    JavaScriptCode,
    /// JSON object
    JsonObject,
    /// JSON output (alias for JsonObject)
    Json,
    /// Markdown formatted text
    Markdown,
    /// HTML content
    Html,
    /// Custom format with specification
    Custom(String),
}

/// Structured code generator with constraint enforcement
///
/// Phase 5b uses native LLM engine as primary implementation with FFI bridge fallback.
pub struct StructuredCodeGenerator {
    /// Configuration
    config: GeneratorConfig,
    /// JSON schema validator
    schema_validator: Option<JSONSchema>,
    /// Native LLM engine (Phase 5b primary)
    native_engine: Option<NativeLlmEngine>,
    /// Python FFI bridge (Phase 5a fallback)
    python_bridge: Option<PythonLlmBridge>,
    /// Generation statistics
    stats: GenerationStats,
}

/// Configuration for structured generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Maximum generation attempts
    pub max_attempts: usize,
    /// Timeout per generation attempt
    pub timeout_seconds: u64,
    /// Whether to enable self-correction
    pub enable_self_correction: bool,
    /// Whether to validate syntax
    pub validate_syntax: bool,
    /// Whether to enforce style guidelines
    pub enforce_style: bool,
    /// Custom validation rules
    pub custom_validators: Vec<CustomValidator>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            timeout_seconds: 30,
            enable_self_correction: true,
            validate_syntax: true,
            enforce_style: true,
            custom_validators: vec![],
        }
    }
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomValidator {
    /// Validator name
    pub name: String,
    /// Description of what this validates
    pub description: String,
    /// Regex pattern to match (if applicable)
    pub pattern: Option<String>,
    /// Python code to execute for validation
    pub python_code: Option<String>,
    /// Severity level
    pub severity: ValidationSeverity,
}

/// Validation severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Information only
    Info,
    /// Warning but continue
    Warning,
    /// Error - retry generation
    Error,
    /// Critical - fail completely
    Critical,
}

/// Generation statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationStats {
    /// Total generation attempts
    pub total_attempts: u32,
    /// Successful generations
    pub successful_generations: u32,
    /// Failed generations
    pub failed_generations: u32,
    /// Average generation time (milliseconds)
    pub average_time_ms: f64,
    /// Total tokens generated
    pub total_tokens: u32,
    /// Validation failure breakdown
    pub validation_failures: HashMap<String, u32>,
}

/// Generation result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    /// Generated code
    pub code: String,
    /// Whether generation was successful
    pub success: bool,
    /// Validation results
    pub validation_results: Vec<ValidationResult>,
    /// Generation metadata
    pub metadata: GenerationMetadata,
    /// Correction attempts made
    pub correction_attempts: u32,
    /// Final confidence score
    pub confidence: f64,
}

/// Individual validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validator name
    pub validator: String,
    /// Whether validation passed
    pub passed: bool,
    /// Validation message
    pub message: String,
    /// Severity level
    pub severity: ValidationSeverity,
    /// Location of issue (if applicable)
    pub location: Option<ErrorLocation>,
}

/// Error location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLocation {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Length of the issue
    pub length: Option<usize>,
    /// Context around the error
    pub context: Option<String>,
}

/// Generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Time taken for generation (milliseconds)
    pub generation_time_ms: u64,
    /// Model used for generation
    pub model_used: String,
    /// Token count
    pub token_count: Option<u32>,
    /// Temperature used
    pub temperature: f32,
    /// Whether self-correction was applied
    pub self_corrected: bool,
    /// Constraint satisfaction score
    pub constraint_satisfaction: f64,
}

/// Syntax validation result
#[derive(Debug, Clone)]
pub struct SyntaxValidation {
    /// Whether syntax is valid
    pub is_valid: bool,
    /// Syntax errors found
    pub errors: Vec<SyntaxError>,
    /// Warnings
    pub warnings: Vec<SyntaxWarning>,
}

/// Syntax error information
#[derive(Debug, Clone)]
pub struct SyntaxError {
    /// Error message
    pub message: String,
    /// Line number
    pub line: usize,
    /// Column number
    pub column: usize,
    /// Error type
    pub error_type: SyntaxErrorType,
}

/// Syntax warning information
#[derive(Debug, Clone)]
pub struct SyntaxWarning {
    /// Warning message
    pub message: String,
    /// Line number
    pub line: usize,
    /// Warning type
    pub warning_type: SyntaxWarningType,
}

/// Types of syntax errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxErrorType {
    /// Parse error
    ParseError,
    /// Indentation error
    IndentationError,
    /// Name error
    NameError,
    /// Type error
    TypeError,
    /// Import error
    ImportError,
}

/// Types of syntax warnings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxWarningType {
    /// Unused variable
    UnusedVariable,
    /// Deprecated usage
    Deprecated,
    /// Style issue
    Style,
    /// Performance issue
    Performance,
}

impl StructuredCodeGenerator {
    /// Create a new structured code generator
    pub fn new(config: GeneratorConfig) -> LlmResult<Self> {
        Ok(Self {
            config,
            schema_validator: None,
            native_engine: None,
            python_bridge: None,
            stats: GenerationStats::default(),
        })
    }

    /// Create with Python FFI bridge for Phase 5a compatibility
    pub fn with_python_bridge(
        config: GeneratorConfig,
        python_bridge: PythonLlmBridge,
    ) -> LlmResult<Self> {
        let mut generator = Self::new(config)?;
        generator.python_bridge = Some(python_bridge);
        Ok(generator)
    }

    /// Set JSON schema for validation
    pub fn set_json_schema(&mut self, schema: &Value) -> LlmResult<()> {
        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(schema)
            .map_err(|e| LlmError::Schema(format!("Schema compilation error: {}", e)))?;

        self.schema_validator = Some(compiled);
        Ok(())
    }

    /// Generate structured code with constraints
    pub async fn generate_structured_code(
        &mut self,
        prompt: &str,
        constraints: &GenerationConstraints,
    ) -> LlmResult<GenerationResult> {
        let start_time = Instant::now();
        self.stats.total_attempts += 1;

        // Phase 5a: Use Python bridge for guaranteed compatibility
        if let Some(ref python_bridge) = self.python_bridge {
            return self.generate_via_python_bridge(prompt, constraints, start_time).await;
        }

        // Native Rust implementation (Phase 5b)
        self.generate_native(prompt, constraints, start_time).await
    }

    /// Generate via Python bridge (Phase 5a)
    async fn generate_via_python_bridge(
        &mut self,
        prompt: &str,
        constraints: &GenerationConstraints,
        start_time: Instant,
    ) -> LlmResult<GenerationResult> {
        // Call Python structured generator directly
        let ffi_constraints = crate::ffi::GenerationConstraints {
            json_schema: constraints.json_schema.clone(),
            grammar_rules: constraints.grammar_rules.clone(),
            max_length: constraints.max_length,
            output_format: match constraints.output_format {
                OutputFormat::PlainText => crate::ffi::OutputFormat::Text,
                OutputFormat::Text => crate::ffi::OutputFormat::Text,
                OutputFormat::JsonObject => crate::ffi::OutputFormat::Json,
                OutputFormat::Json => crate::ffi::OutputFormat::Json,
                OutputFormat::PythonCode => crate::ffi::OutputFormat::PythonCode,
                OutputFormat::RustCode => crate::ffi::OutputFormat::RustCode,
                OutputFormat::Markdown => crate::ffi::OutputFormat::Markdown,
                OutputFormat::JavaScriptCode => crate::ffi::OutputFormat::Text,
                OutputFormat::Html => crate::ffi::OutputFormat::Text,
                OutputFormat::Custom(_) => crate::ffi::OutputFormat::Text,
            },
            validation_rules: constraints.validation_rules.clone(),
        };
        let generated_code = self.python_bridge
            .as_ref()
            .unwrap()
            .generate_structured_code(prompt, &ffi_constraints)?;

        let generation_time = start_time.elapsed().as_millis() as u64;

        // Validate the generated code using Rust validators
        let validation_results = self.validate_generated_code(&generated_code, constraints).await?;

        let success = validation_results.iter().all(|v|
            v.passed || v.severity == ValidationSeverity::Info || v.severity == ValidationSeverity::Warning
        );

        if success {
            self.stats.successful_generations += 1;
        } else {
            self.stats.failed_generations += 1;
        }

        // Update statistics
        self.update_stats(generation_time, &validation_results);

        Ok(GenerationResult {
            code: generated_code,
            success,
            validation_results: validation_results.clone(),
            metadata: GenerationMetadata {
                generation_time_ms: generation_time,
                model_used: "python_bridge".to_string(),
                token_count: None, // Not available from Python bridge
                temperature: 0.1, // Default from Python
                self_corrected: false, // Handled by Python
                constraint_satisfaction: self.calculate_constraint_satisfaction(&validation_results),
            },
            correction_attempts: 0,
            confidence: if success { 0.95 } else { 0.5 },
        })
    }

    /// Native generation implementation (Phase 5b)
    async fn generate_native(
        &mut self,
        prompt: &str,
        constraints: &GenerationConstraints,
        start_time: Instant,
    ) -> LlmResult<GenerationResult> {
        let mut attempts = 0;
        let mut last_error: Option<LlmError> = None;

        while attempts < self.config.max_attempts {
            attempts += 1;

            match self.attempt_generation(prompt, constraints).await {
                Ok(mut result) => {
                    result.correction_attempts = (attempts - 1) as u32;
                    result.metadata.generation_time_ms = start_time.elapsed().as_millis() as u64;

                    if result.success {
                        self.stats.successful_generations += 1;
                        return Ok(result);
                    } else if !self.config.enable_self_correction {
                        self.stats.failed_generations += 1;
                        return Ok(result);
                    }

                    // Continue to next attempt for self-correction
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        self.stats.failed_generations += 1;

        // All attempts failed
        Err(last_error.unwrap_or_else(||
            LlmError::Inference("All generation attempts failed".to_string())
        ))
    }

    /// Attempt a single generation
    async fn attempt_generation(
        &self,
        prompt: &str,
        constraints: &GenerationConstraints,
    ) -> LlmResult<GenerationResult> {
        // Enhanced prompt with constraints
        let enhanced_prompt = self.build_enhanced_prompt(prompt, constraints);

        // TODO: Integrate with Ollama or other LLM backend for native generation
        // For now, return a placeholder
        let generated_code = format!(
            "# Generated code for prompt: {}\n# Constraints: {:?}\n\npass",
            prompt.lines().next().unwrap_or(""),
            constraints.output_format
        );

        let validation_results = self.validate_generated_code(&generated_code, constraints).await?;

        let success = validation_results.iter().all(|v|
            v.passed || v.severity == ValidationSeverity::Info || v.severity == ValidationSeverity::Warning
        );

        Ok(GenerationResult {
            code: generated_code,
            success,
            validation_results: validation_results.clone(),
            metadata: GenerationMetadata {
                generation_time_ms: 0, // Will be set by caller
                model_used: "native_rust".to_string(),
                token_count: None,
                temperature: 0.1,
                self_corrected: false,
                constraint_satisfaction: self.calculate_constraint_satisfaction(&validation_results),
            },
            correction_attempts: 0,
            confidence: if success { 0.8 } else { 0.3 },
        })
    }

    /// Build enhanced prompt with constraints
    fn build_enhanced_prompt(&self, prompt: &str, constraints: &GenerationConstraints) -> String {
        let mut enhanced = String::from(prompt);

        // Add output format instructions
        match constraints.output_format {
            OutputFormat::Json | OutputFormat::JsonObject => {
                enhanced.push_str("\n\nGenerate valid JSON only.");
                if let Some(ref schema) = constraints.json_schema {
                    enhanced.push_str(&format!(" Follow this schema: {}", schema));
                }
            }
            OutputFormat::PythonCode => {
                enhanced.push_str("\n\nGenerate valid Python code only. Include proper imports and follow PEP 8 style guidelines.");
            }
            OutputFormat::RustCode => {
                enhanced.push_str("\n\nGenerate valid Rust code only. Include proper use statements and follow Rust conventions.");
            }
            OutputFormat::JavaScriptCode => {
                enhanced.push_str("\n\nGenerate valid JavaScript code only. Include proper imports and follow JavaScript conventions.");
            }
            OutputFormat::Markdown => {
                enhanced.push_str("\n\nGenerate properly formatted Markdown with appropriate headings and structure.");
            }
            OutputFormat::Html => {
                enhanced.push_str("\n\nGenerate valid HTML with proper structure and semantic tags.");
            }
            OutputFormat::Text | OutputFormat::PlainText => {
                enhanced.push_str("\n\nGenerate clear, well-structured text.");
            }
            OutputFormat::Custom(ref spec) => {
                enhanced.push_str(&format!("\n\nGenerate output in the following custom format: {}", spec));
            }
        }

        // Add validation rules as constraints
        if !constraints.validation_rules.is_empty() {
            enhanced.push_str("\n\nImportant constraints:");
            for rule in &constraints.validation_rules {
                enhanced.push_str(&format!("\n- {}", rule));
            }
        }

        // Add length constraint
        if let Some(max_length) = constraints.max_length {
            enhanced.push_str(&format!("\n\nKeep response under {} characters.", max_length));
        }

        enhanced
    }

    /// Validate generated code against constraints
    async fn validate_generated_code(
        &self,
        code: &str,
        constraints: &GenerationConstraints,
    ) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        // Length validation
        if let Some(max_length) = constraints.max_length {
            let passed = code.len() <= max_length;
            results.push(ValidationResult {
                validator: "length".to_string(),
                passed,
                message: if passed {
                    format!("Code length {} within limit {}", code.len(), max_length)
                } else {
                    format!("Code length {} exceeds limit {}", code.len(), max_length)
                },
                severity: if passed { ValidationSeverity::Info } else { ValidationSeverity::Error },
                location: None,
            });
        }

        // JSON schema validation
        if let Some(ref validator) = self.schema_validator {
            match constraints.output_format {
                OutputFormat::Json => {
                    if let Ok(json_value) = serde_json::from_str::<Value>(code) {
                        let validation_result = validator.validate(&json_value);
                        let passed = validation_result.is_ok();

                        results.push(ValidationResult {
                            validator: "json_schema".to_string(),
                            passed,
                            message: if passed {
                                "JSON schema validation passed".to_string()
                            } else {
                                "JSON schema validation failed".to_string()
                            },
                            severity: if passed { ValidationSeverity::Info } else { ValidationSeverity::Error },
                            location: None,
                        });
                    } else {
                        results.push(ValidationResult {
                            validator: "json_syntax".to_string(),
                            passed: false,
                            message: "Invalid JSON syntax".to_string(),
                            severity: ValidationSeverity::Error,
                            location: None,
                        });
                    }
                }
                _ => {}
            }
        }

        // Syntax validation
        if self.config.validate_syntax {
            let syntax_result = self.validate_syntax(code, &constraints.output_format).await;
            results.push(ValidationResult {
                validator: "syntax".to_string(),
                passed: syntax_result.is_valid,
                message: if syntax_result.is_valid {
                    "Syntax validation passed".to_string()
                } else {
                    format!("Syntax errors found: {}", syntax_result.errors.len())
                },
                severity: if syntax_result.is_valid {
                    ValidationSeverity::Info
                } else {
                    ValidationSeverity::Error
                },
                location: None,
            });
        }

        // Custom validation rules
        for rule in &constraints.validation_rules {
            let passed = self.apply_validation_rule(code, rule);
            results.push(ValidationResult {
                validator: format!("custom_{}", rule),
                passed,
                message: if passed {
                    format!("Custom rule '{}' satisfied", rule)
                } else {
                    format!("Custom rule '{}' violated", rule)
                },
                severity: ValidationSeverity::Warning,
                location: None,
            });
        }

        Ok(results)
    }

    /// Validate syntax for specific output format
    async fn validate_syntax(&self, code: &str, format: &OutputFormat) -> SyntaxValidation {
        match format {
            OutputFormat::PythonCode => {
                self.validate_python_syntax(code).await
            }
            OutputFormat::RustCode => {
                self.validate_rust_syntax(code).await
            }
            OutputFormat::Json => {
                self.validate_json_syntax(code).await
            }
            OutputFormat::Markdown => {
                self.validate_markdown_syntax(code).await
            }
            OutputFormat::Text => {
                // Text format doesn't have strict syntax rules
                SyntaxValidation {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                }
            }
            OutputFormat::PlainText => {
                // Plain text format doesn't have strict syntax rules
                SyntaxValidation {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                }
            }
            OutputFormat::JavaScriptCode => {
                self.validate_javascript_syntax(code).await
            }
            OutputFormat::JsonObject => {
                self.validate_json_syntax(code).await
            }
            OutputFormat::Html => {
                self.validate_html_syntax(code).await
            }
            OutputFormat::Custom(_) => {
                // Custom format validation not implemented
                SyntaxValidation {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![SyntaxWarning {
                        message: "Custom format syntax validation not implemented".to_string(),
                        line: 1,
                        warning_type: SyntaxWarningType::Style,
                    }],
                }
            }
        }
    }

    /// Validate HTML syntax
    async fn validate_html_syntax(&self, _code: &str) -> SyntaxValidation {
        // HTML syntax validation would require external parsers
        // For now, provide a placeholder implementation
        SyntaxValidation {
            is_valid: true,
            errors: vec![],
            warnings: vec![SyntaxWarning {
                message: "HTML syntax validation not implemented".to_string(),
                line: 1,
                warning_type: SyntaxWarningType::Style,
            }],
        }
    }

    /// Validate JavaScript syntax
    async fn validate_javascript_syntax(&self, _code: &str) -> SyntaxValidation {
        // JavaScript syntax validation would require external tools or parsers
        // For now, provide a placeholder implementation
        SyntaxValidation {
            is_valid: true,
            errors: vec![],
            warnings: vec![SyntaxWarning {
                message: "JavaScript syntax validation not implemented".to_string(),
                line: 1,
                warning_type: SyntaxWarningType::Style,
            }],
        }
    }

    /// Validate Python syntax
    async fn validate_python_syntax(&self, code: &str) -> SyntaxValidation {
        // Use Python AST parsing via FFI bridge if available
        if let Some(ref python_bridge) = self.python_bridge {
            // Call Python ast.parse() for accurate syntax validation
            // TODO: Implement Python AST validation via FFI
        }

        // Basic validation for now
        let mut errors = vec![];
        let mut warnings = vec![];

        // Check for basic Python syntax issues
        for (line_num, line) in code.lines().enumerate() {
            let line = line.trim();

            // Check indentation (very basic)
            if line.starts_with(' ') && !code.lines().nth(line_num.saturating_sub(1))
                .map_or(false, |prev| prev.trim().ends_with(':')) {
                // Potential indentation issue
                warnings.push(SyntaxWarning {
                    message: "Unexpected indentation".to_string(),
                    line: line_num + 1,
                    warning_type: SyntaxWarningType::Style,
                });
            }

            // Check for common syntax errors
            if line.contains("print ") && !line.contains("print(") {
                errors.push(SyntaxError {
                    message: "print statement syntax (use print() function)".to_string(),
                    line: line_num + 1,
                    column: line.find("print ").unwrap_or(0) + 1,
                    error_type: SyntaxErrorType::ParseError,
                });
            }
        }

        SyntaxValidation {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate Rust syntax
    async fn validate_rust_syntax(&self, code: &str) -> SyntaxValidation {
        // Try to parse with syn crate
        match syn::parse_file(code) {
            Ok(_) => SyntaxValidation {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
            },
            Err(e) => {
                let error = SyntaxError {
                    message: e.to_string(),
                    line: 1, // proc_macro2::Span doesn't have start() method
                    column: 1,
                    error_type: SyntaxErrorType::ParseError,
                };

                SyntaxValidation {
                    is_valid: false,
                    errors: vec![error],
                    warnings: vec![],
                }
            }
        }
    }

    /// Validate JSON syntax
    async fn validate_json_syntax(&self, code: &str) -> SyntaxValidation {
        match serde_json::from_str::<Value>(code) {
            Ok(_) => SyntaxValidation {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
            },
            Err(e) => {
                let error = SyntaxError {
                    message: e.to_string(),
                    line: e.line(),
                    column: e.column(),
                    error_type: SyntaxErrorType::ParseError,
                };

                SyntaxValidation {
                    is_valid: false,
                    errors: vec![error],
                    warnings: vec![],
                }
            }
        }
    }

    /// Validate Markdown syntax
    async fn validate_markdown_syntax(&self, code: &str) -> SyntaxValidation {
        // Basic Markdown validation
        let mut warnings = vec![];

        // Check for common Markdown issues
        for (line_num, line) in code.lines().enumerate() {
            // Check for headers without space after #
            if line.starts_with('#') && !line.starts_with("# ") && !line.starts_with("##") {
                warnings.push(SyntaxWarning {
                    message: "Header should have space after #".to_string(),
                    line: line_num + 1,
                    warning_type: SyntaxWarningType::Style,
                });
            }
        }

        SyntaxValidation {
            is_valid: true, // Markdown is very forgiving
            errors: vec![],
            warnings,
        }
    }

    /// Apply custom validation rule
    fn apply_validation_rule(&self, code: &str, rule: &str) -> bool {
        match rule {
            "no_dangerous_imports" => {
                !code.contains("import os") &&
                !code.contains("import subprocess") &&
                !code.contains("import shutil")
            }
            "no_dangerous_code" => {
                !code.contains("eval(") &&
                !code.contains("exec(") &&
                !code.contains("__import__")
            }
            "require_docstrings" => {
                code.contains("\"\"\"") || code.contains("///")
            }
            "no_hardcoded_paths" => {
                !code.contains("C:\\") &&
                !code.contains("/home/") &&
                !code.contains("/usr/")
            }
            _ => true, // Unknown rule passes by default
        }
    }

    /// Calculate constraint satisfaction score
    fn calculate_constraint_satisfaction(&self, validation_results: &[ValidationResult]) -> f64 {
        if validation_results.is_empty() {
            return 1.0;
        }

        let total_weight: f64 = validation_results.iter().map(|v| match v.severity {
            ValidationSeverity::Critical => 4.0,
            ValidationSeverity::Error => 3.0,
            ValidationSeverity::Warning => 2.0,
            ValidationSeverity::Info => 1.0,
        }).sum();

        let satisfied_weight: f64 = validation_results.iter()
            .filter(|v| v.passed)
            .map(|v| match v.severity {
                ValidationSeverity::Critical => 4.0,
                ValidationSeverity::Error => 3.0,
                ValidationSeverity::Warning => 2.0,
                ValidationSeverity::Info => 1.0,
            }).sum();

        if total_weight > 0.0 {
            satisfied_weight / total_weight
        } else {
            1.0
        }
    }

    /// Update generation statistics
    fn update_stats(&mut self, generation_time_ms: u64, validation_results: &[ValidationResult]) {
        // Update average time
        let total_time = self.stats.average_time_ms * f64::from(self.stats.total_attempts - 1)
            + generation_time_ms as f64;
        self.stats.average_time_ms = total_time / f64::from(self.stats.total_attempts);

        // Update validation failure counts
        for result in validation_results {
            if !result.passed {
                *self.stats.validation_failures
                    .entry(result.validator.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    /// Get current generation statistics
    pub fn get_stats(&self) -> &GenerationStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = GenerationStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_config_default() {
        let config = GeneratorConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert!(config.enable_self_correction);
        assert!(config.validate_syntax);
    }

    #[tokio::test]
    async fn test_json_syntax_validation() {
        let config = GeneratorConfig::default();
        let generator = StructuredCodeGenerator::new(config).unwrap();

        let valid_json = r#"{"key": "value", "number": 42}"#;
        let validation = generator.validate_json_syntax(valid_json).await;
        assert!(validation.is_valid);

        let invalid_json = r#"{"key": "value", "number": 42"#; // Missing closing brace
        let validation = generator.validate_json_syntax(invalid_json).await;
        assert!(!validation.is_valid);
    }

    #[tokio::test]
    async fn test_rust_syntax_validation() {
        let config = GeneratorConfig::default();
        let generator = StructuredCodeGenerator::new(config).unwrap();

        let valid_rust = "fn main() { println!(\"Hello, world!\"); }";
        let validation = generator.validate_rust_syntax(valid_rust).await;
        assert!(validation.is_valid);

        let invalid_rust = "fn main() { println!(\"Hello, world!\" }"; // Missing closing paren
        let validation = generator.validate_rust_syntax(invalid_rust).await;
        assert!(!validation.is_valid);
    }

    #[test]
    fn test_custom_validation_rules() {
        let config = GeneratorConfig::default();
        let generator = StructuredCodeGenerator::new(config).unwrap();

        let dangerous_code = "import os\nos.system('rm -rf /')";
        assert!(!generator.apply_validation_rule(dangerous_code, "no_dangerous_imports"));
        assert!(!generator.apply_validation_rule(dangerous_code, "no_dangerous_code"));

        let safe_code = "print('Hello, world!')";
        assert!(generator.apply_validation_rule(safe_code, "no_dangerous_imports"));
        assert!(generator.apply_validation_rule(safe_code, "no_dangerous_code"));
    }

    #[test]
    fn test_constraint_satisfaction_calculation() {
        let config = GeneratorConfig::default();
        let generator = StructuredCodeGenerator::new(config).unwrap();

        let validation_results = vec![
            ValidationResult {
                validator: "test1".to_string(),
                passed: true,
                message: "OK".to_string(),
                severity: ValidationSeverity::Error,
                location: None,
            },
            ValidationResult {
                validator: "test2".to_string(),
                passed: false,
                message: "Failed".to_string(),
                severity: ValidationSeverity::Warning,
                location: None,
            },
        ];

        let score = generator.calculate_constraint_satisfaction(&validation_results);
        assert!(score > 0.0 && score < 1.0); // Should be partial satisfaction
    }
}