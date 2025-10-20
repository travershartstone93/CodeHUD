use crate::{LlmError, LlmResult};
use crate::ffi::PythonLlmBridge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use jsonschema::JSONSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Syntax,
    Semantic,
    Schema,
    Business,
    Security,
    Performance,
    Consistency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub validation_type: ValidationType,
    pub severity: ValidationSeverity,
    pub enabled: bool,
    pub pattern: Option<String>,
    pub schema: Option<serde_json::Value>,
    pub custom_validator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub rule_id: String,
    pub passed: bool,
    pub severity: ValidationSeverity,
    pub message: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
    pub location: Option<ValidationLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationLocation {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub offset: Option<usize>,
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub content_id: String,
    pub timestamp: DateTime<Utc>,
    pub results: Vec<ValidationResult>,
    pub summary: ValidationSummary,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_rules: usize,
    pub passed_rules: usize,
    pub failed_rules: usize,
    pub warnings: usize,
    pub errors: usize,
    pub critical_issues: usize,
    pub overall_score: f32,
    pub validation_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub rules: Vec<ValidationRule>,
    pub fail_on_error: bool,
    pub fail_on_critical: bool,
    pub max_validation_time_ms: u64,
    pub enable_auto_fix: bool,
    pub custom_validators: HashMap<String, String>,
}

pub struct ValidationEngine {
    config: ValidationConfig,
    schema_cache: HashMap<String, JSONSchema>,
    python_bridge: Option<PythonLlmBridge>,
}

impl ValidationEngine {
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            schema_cache: HashMap::new(),
            python_bridge: None,
        }
    }

    pub fn with_python_bridge(mut self, bridge: PythonLlmBridge) -> Self {
        self.python_bridge = Some(bridge);
        self
    }

    pub async fn validate_content(&mut self, content: &str, content_id: &str) -> LlmResult<ValidationReport> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.validate_content(content, content_id, &self.config).await;
        }
        self.validate_content_native(content, content_id).await
    }

    async fn validate_content_native(&mut self, content: &str, content_id: &str) -> LlmResult<ValidationReport> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();

        let rules = self.config.rules.clone();
        for rule in &rules {
            if !rule.enabled {
                continue;
            }

            let rule_start = std::time::Instant::now();
            let rule_result = self.apply_validation_rule(content, rule).await?;
            let rule_duration = rule_start.elapsed();

            if rule_duration.as_millis() > self.config.max_validation_time_ms as u128 {
                results.push(ValidationResult {
                    rule_id: rule.id.clone(),
                    passed: false,
                    severity: ValidationSeverity::Warning,
                    message: "Validation rule timeout".to_string(),
                    details: Some(format!("Rule took {}ms to execute", rule_duration.as_millis())),
                    suggestion: Some("Consider optimizing this validation rule".to_string()),
                    location: None,
                });
                continue;
            }

            results.extend(rule_result);
        }

        let validation_time = start_time.elapsed();
        let summary = self.generate_validation_summary(&results, validation_time.as_millis() as u64);
        let suggestions = self.generate_suggestions(&results);

        Ok(ValidationReport {
            content_id: content_id.to_string(),
            timestamp: Utc::now(),
            results,
            summary,
            suggestions,
        })
    }

    async fn apply_validation_rule(&mut self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        match rule.validation_type {
            ValidationType::Syntax => self.validate_syntax(content, rule).await,
            ValidationType::Semantic => self.validate_semantic(content, rule).await,
            ValidationType::Schema => self.validate_schema(content, rule).await,
            ValidationType::Business => self.validate_business_logic(content, rule).await,
            ValidationType::Security => self.validate_security(content, rule).await,
            ValidationType::Performance => self.validate_performance(content, rule).await,
            ValidationType::Consistency => self.validate_consistency(content, rule).await,
        }
    }

    async fn validate_syntax(&self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        if let Some(ref pattern) = rule.pattern {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| LlmError::ValidationError(format!("Invalid regex pattern: {}", e)))?;

            if !regex.is_match(content) {
                results.push(ValidationResult {
                    rule_id: rule.id.clone(),
                    passed: false,
                    severity: rule.severity.clone(),
                    message: format!("Syntax validation failed: {}", rule.name),
                    details: Some("Content does not match expected syntax pattern".to_string()),
                    suggestion: Some("Check syntax and formatting".to_string()),
                    location: None,
                });
            } else {
                results.push(ValidationResult {
                    rule_id: rule.id.clone(),
                    passed: true,
                    severity: rule.severity.clone(),
                    message: format!("Syntax validation passed: {}", rule.name),
                    details: None,
                    suggestion: None,
                    location: None,
                });
            }
        }

        if content.trim().is_empty() {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: false,
                severity: ValidationSeverity::Warning,
                message: "Empty content detected".to_string(),
                details: Some("Content appears to be empty or whitespace only".to_string()),
                suggestion: Some("Provide meaningful content".to_string()),
                location: None,
            });
        }

        Ok(results)
    }

    async fn validate_semantic(&self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        let word_count = content.split_whitespace().count();
        let sentence_count = content.matches(&['.', '!', '?'][..]).count();

        if word_count < 5 {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: false,
                severity: ValidationSeverity::Warning,
                message: "Content too short for semantic validation".to_string(),
                details: Some(format!("Only {} words found", word_count)),
                suggestion: Some("Provide more detailed content".to_string()),
                location: None,
            });
        }

        if sentence_count == 0 && word_count > 0 {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: false,
                severity: ValidationSeverity::Info,
                message: "No sentence terminators found".to_string(),
                details: Some("Content may be incomplete".to_string()),
                suggestion: Some("Consider adding proper punctuation".to_string()),
                location: None,
            });
        }

        if results.is_empty() {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: format!("Semantic validation passed: {}", rule.name),
                details: None,
                suggestion: None,
                location: None,
            });
        }

        Ok(results)
    }

    async fn validate_schema(&mut self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        if let Some(ref schema_value) = rule.schema {
            let schema = if let Some(cached_schema) = self.schema_cache.get(&rule.id) {
                cached_schema
            } else {
                let compiled_schema = JSONSchema::compile(schema_value)
                    .map_err(|e| LlmError::ValidationError(format!("Invalid JSON schema: {}", e)))?;
                self.schema_cache.insert(rule.id.clone(), compiled_schema);
                self.schema_cache.get(&rule.id).unwrap()
            };

            match serde_json::from_str::<serde_json::Value>(content) {
                Ok(json_content) => {
                    let validation_result = schema.validate(&json_content);
                    if let Err(errors) = validation_result {
                        for error in errors {
                            results.push(ValidationResult {
                                rule_id: rule.id.clone(),
                                passed: false,
                                severity: rule.severity.clone(),
                                message: format!("Schema validation failed: {}", error),
                                details: Some(format!("At path: {}", error.instance_path)),
                                suggestion: Some("Fix schema validation errors".to_string()),
                                location: None,
                            });
                        }
                    } else {
                        results.push(ValidationResult {
                            rule_id: rule.id.clone(),
                            passed: true,
                            severity: rule.severity.clone(),
                            message: format!("Schema validation passed: {}", rule.name),
                            details: None,
                            suggestion: None,
                            location: None,
                        });
                    }
                }
                Err(e) => {
                    results.push(ValidationResult {
                        rule_id: rule.id.clone(),
                        passed: false,
                        severity: ValidationSeverity::Error,
                        message: "Invalid JSON format".to_string(),
                        details: Some(format!("JSON parsing error: {}", e)),
                        suggestion: Some("Fix JSON formatting".to_string()),
                        location: None,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn validate_business_logic(&self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        let business_patterns = vec![
            (r"(?i)\b(TODO|FIXME|XXX|HACK)\b", "Unresolved development markers"),
            (r"(?i)\b(placeholder|temporary|temp)\b", "Placeholder content detected"),
            (r"\b\d{4}-\d{2}-\d{2}\b", "Date format validation"),
        ];

        for (pattern, description) in business_patterns {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| LlmError::ValidationError(format!("Invalid regex: {}", e)))?;

            if regex.is_match(content) {
                results.push(ValidationResult {
                    rule_id: rule.id.clone(),
                    passed: false,
                    severity: ValidationSeverity::Warning,
                    message: format!("Business logic issue: {}", description),
                    details: Some("Content contains business logic concerns".to_string()),
                    suggestion: Some("Review and address business logic issues".to_string()),
                    location: None,
                });
            }
        }

        if results.is_empty() {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: format!("Business validation passed: {}", rule.name),
                details: None,
                suggestion: None,
                location: None,
            });
        }

        Ok(results)
    }

    async fn validate_security(&self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        let security_patterns = vec![
            (r"(?i)\b(password|secret|key|token)\s*[:=]\s*['\u{0022}]?[^\s'\u{0022}]+", "Potential credential exposure"),
            (r"(?i)\b(api[_-]?key|access[_-]?token)\b", "API key pattern detected"),
            (r"(?i)<script[^>]*>.*?</script>", "Script injection risk"),
            (r"(?i)\b(exec|eval|system)\s*\(", "Code execution risk"),
        ];

        for (pattern, description) in security_patterns {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| LlmError::ValidationError(format!("Invalid regex: {}", e)))?;

            if regex.is_match(content) {
                results.push(ValidationResult {
                    rule_id: rule.id.clone(),
                    passed: false,
                    severity: ValidationSeverity::Critical,
                    message: format!("Security vulnerability: {}", description),
                    details: Some("Potential security issue detected".to_string()),
                    suggestion: Some("Remove or sanitize sensitive content".to_string()),
                    location: None,
                });
            }
        }

        if results.is_empty() {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: format!("Security validation passed: {}", rule.name),
                details: None,
                suggestion: None,
                location: None,
            });
        }

        Ok(results)
    }

    async fn validate_performance(&self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();
        let content_length = content.len();
        let line_count = content.lines().count();

        if content_length > 100000 {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: false,
                severity: ValidationSeverity::Warning,
                message: "Content size may impact performance".to_string(),
                details: Some(format!("Content is {} bytes", content_length)),
                suggestion: Some("Consider breaking into smaller chunks".to_string()),
                location: None,
            });
        }

        if line_count > 1000 {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: false,
                severity: ValidationSeverity::Info,
                message: "High line count detected".to_string(),
                details: Some(format!("Content has {} lines", line_count)),
                suggestion: Some("Consider content optimization".to_string()),
                location: None,
            });
        }

        if results.is_empty() {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: format!("Performance validation passed: {}", rule.name),
                details: None,
                suggestion: None,
                location: None,
            });
        }

        Ok(results)
    }

    async fn validate_consistency(&self, content: &str, rule: &ValidationRule) -> LlmResult<Vec<ValidationResult>> {
        let mut results = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut indentation_levels = Vec::new();

        for line in &lines {
            if !line.trim().is_empty() {
                let leading_spaces = line.len() - line.trim_start().len();
                indentation_levels.push(leading_spaces);
            }
        }

        if indentation_levels.len() > 1 {
            let first_indent = indentation_levels[0];
            let inconsistent = indentation_levels.iter().any(|&indent| {
                indent != first_indent && indent % 2 != first_indent % 2
            });

            if inconsistent {
                results.push(ValidationResult {
                    rule_id: rule.id.clone(),
                    passed: false,
                    severity: ValidationSeverity::Warning,
                    message: "Inconsistent indentation detected".to_string(),
                    details: Some("Mixed indentation styles found".to_string()),
                    suggestion: Some("Use consistent indentation throughout".to_string()),
                    location: None,
                });
            }
        }

        if results.is_empty() {
            results.push(ValidationResult {
                rule_id: rule.id.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: format!("Consistency validation passed: {}", rule.name),
                details: None,
                suggestion: None,
                location: None,
            });
        }

        Ok(results)
    }

    fn generate_validation_summary(&self, results: &[ValidationResult], validation_time_ms: u64) -> ValidationSummary {
        let total_rules = results.len();
        let passed_rules = results.iter().filter(|r| r.passed).count();
        let failed_rules = total_rules - passed_rules;

        let warnings = results.iter()
            .filter(|r| matches!(r.severity, ValidationSeverity::Warning))
            .count();
        let errors = results.iter()
            .filter(|r| matches!(r.severity, ValidationSeverity::Error))
            .count();
        let critical_issues = results.iter()
            .filter(|r| matches!(r.severity, ValidationSeverity::Critical))
            .count();

        let overall_score = if total_rules > 0 {
            passed_rules as f32 / total_rules as f32
        } else {
            1.0
        };

        ValidationSummary {
            total_rules,
            passed_rules,
            failed_rules,
            warnings,
            errors,
            critical_issues,
            overall_score,
            validation_time_ms,
        }
    }

    fn generate_suggestions(&self, results: &[ValidationResult]) -> Vec<String> {
        results.iter()
            .filter_map(|r| r.suggestion.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn add_validation_rule(&mut self, rule: ValidationRule) -> LlmResult<()> {
        self.config.rules.push(rule);
        Ok(())
    }

    pub fn remove_validation_rule(&mut self, rule_id: &str) -> LlmResult<()> {
        self.config.rules.retain(|r| r.id != rule_id);
        self.schema_cache.remove(rule_id);
        Ok(())
    }

    pub fn update_validation_rule(&mut self, rule: ValidationRule) -> LlmResult<()> {
        if let Some(pos) = self.config.rules.iter().position(|r| r.id == rule.id) {
            self.config.rules[pos] = rule.clone();
            self.schema_cache.remove(&rule.id);
            Ok(())
        } else {
            Err(LlmError::ValidationError(
                format!("Rule '{}' not found", rule.id)
            ))
        }
    }

    pub fn get_validation_rule(&self, rule_id: &str) -> Option<&ValidationRule> {
        self.config.rules.iter().find(|r| r.id == rule_id)
    }

    pub fn list_validation_rules(&self) -> Vec<&ValidationRule> {
        self.config.rules.iter().collect()
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                ValidationRule {
                    id: "syntax_basic".to_string(),
                    name: "Basic Syntax Check".to_string(),
                    description: "Validates basic syntax requirements".to_string(),
                    validation_type: ValidationType::Syntax,
                    severity: ValidationSeverity::Error,
                    enabled: true,
                    pattern: None,
                    schema: None,
                    custom_validator: None,
                },
                ValidationRule {
                    id: "security_basic".to_string(),
                    name: "Basic Security Check".to_string(),
                    description: "Scans for common security issues".to_string(),
                    validation_type: ValidationType::Security,
                    severity: ValidationSeverity::Critical,
                    enabled: true,
                    pattern: None,
                    schema: None,
                    custom_validator: None,
                },
            ],
            fail_on_error: false,
            fail_on_critical: true,
            max_validation_time_ms: 5000,
            enable_auto_fix: false,
            custom_validators: HashMap::new(),
        }
    }
}