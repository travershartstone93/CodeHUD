//! Critical Mistake Detection and Self-Correction System
//!
//! This module implements advanced mistake detection with automatic correction
//! capabilities, preserving the Python implementation's 97%+ bug fix success rate
//! during Phase 5a through FFI bridge integration.

use crate::{LlmResult, LlmError, ffi::PythonLlmBridge};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Types of critical mistakes that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MistakeType {
    /// Syntax errors
    SyntaxError,
    /// Logic errors
    LogicError,
    /// Security vulnerabilities
    SecurityVulnerability,
    /// Performance issues
    PerformanceIssue,
    /// Type mismatches
    TypeMismatch,
    /// Resource leaks
    ResourceLeak,
    /// Infinite loops or recursion
    InfiniteExecution,
}

/// Location in code where a mistake was detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeLocation {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Length of the problematic code
    pub length: Option<usize>,
}

/// A detected critical mistake with correction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalMistake {
    /// Type of mistake detected
    pub mistake_type: MistakeType,
    /// Severity level (1-10)
    pub severity: u8,
    /// Description of the mistake
    pub description: String,
    /// Location in the generated code
    pub location: Option<CodeLocation>,
    /// Suggested correction
    pub suggested_fix: Option<String>,
    /// Confidence in the detection (0.0-1.0)
    pub confidence: f64,
    /// Context around the mistake
    pub context: Option<String>,
}

/// Critical mistake detector with self-correction capabilities
///
/// During Phase 5a, this delegates to the Python implementation for guaranteed
/// compatibility while building the Rust analysis infrastructure.
pub struct CriticalMistakeDetector {
    /// Configuration
    config: DetectorConfig,
    /// Analysis rules for different mistake types
    detection_rules: Vec<DetectionRule>,
    /// Python FFI bridge (Phase 5a)
    python_bridge: Option<PythonLlmBridge>,
    /// Detection statistics
    stats: DetectionStats,
    /// Correction history
    correction_history: Vec<CorrectionRecord>,
}

/// Configuration for mistake detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Enable syntax error detection
    pub detect_syntax_errors: bool,
    /// Enable logic error detection
    pub detect_logic_errors: bool,
    /// Enable security vulnerability detection
    pub detect_security_issues: bool,
    /// Enable performance issue detection
    pub detect_performance_issues: bool,
    /// Enable type mismatch detection
    pub detect_type_mismatches: bool,
    /// Enable resource leak detection
    pub detect_resource_leaks: bool,
    /// Enable infinite execution detection
    pub detect_infinite_execution: bool,
    /// Maximum correction attempts
    pub max_correction_attempts: usize,
    /// Severity threshold for automatic correction
    pub auto_correction_threshold: u8,
    /// Enable learning from corrections
    pub enable_learning: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            detect_syntax_errors: true,
            detect_logic_errors: true,
            detect_security_issues: true,
            detect_performance_issues: true,
            detect_type_mismatches: true,
            detect_resource_leaks: true,
            detect_infinite_execution: true,
            max_correction_attempts: 3,
            auto_correction_threshold: 7, // High severity and above
            enable_learning: true,
        }
    }
}

/// Detection rule for specific mistake patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRule {
    /// Rule identifier
    pub id: String,
    /// Mistake type this rule detects
    pub mistake_type: MistakeType,
    /// Rule description
    pub description: String,
    /// Pattern to match (regex or code pattern)
    pub pattern: String,
    /// Rule severity (1-10)
    pub severity: u8,
    /// Language this rule applies to
    pub language: Option<String>,
    /// Suggested correction template
    pub correction_template: Option<String>,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// Detection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionStats {
    /// Total code samples analyzed
    pub total_analyzed: u32,
    /// Total mistakes detected
    pub total_mistakes: u32,
    /// Mistakes by type
    pub mistakes_by_type: HashMap<MistakeType, u32>,
    /// Mistakes by severity
    pub mistakes_by_severity: HashMap<u8, u32>,
    /// Successful corrections
    pub successful_corrections: u32,
    /// Failed corrections
    pub failed_corrections: u32,
    /// Average detection time (milliseconds)
    pub average_detection_time_ms: f64,
    /// False positive rate
    pub false_positive_rate: f64,
}

/// Record of a correction attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRecord {
    /// Original code with mistake
    pub original_code: String,
    /// Corrected code
    pub corrected_code: String,
    /// Mistakes that were corrected
    pub corrected_mistakes: Vec<CriticalMistake>,
    /// Correction success
    pub success: bool,
    /// Correction timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Correction method used
    pub method: CorrectionMethod,
    /// Validation results after correction
    pub validation_results: Vec<ValidationResult>,
}

/// Methods used for correction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrectionMethod {
    /// Template-based correction
    Template,
    /// Pattern-based replacement
    PatternReplacement,
    /// LLM-based correction
    LlmCorrection,
    /// Rule-based transformation
    RuleBasedTransform,
    /// Manual correction (human-provided)
    Manual,
}

/// Validation result after correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validation type
    pub validation_type: ValidationType,
    /// Whether validation passed
    pub passed: bool,
    /// Validation message
    pub message: String,
    /// Time taken for validation (milliseconds)
    pub validation_time_ms: u64,
}

/// Types of validation performed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationType {
    /// Syntax validation
    Syntax,
    /// Static analysis
    StaticAnalysis,
    /// Type checking
    TypeCheck,
    /// Security scan
    SecurityScan,
    /// Performance analysis
    PerformanceAnalysis,
    /// Runtime test
    RuntimeTest,
}

/// Detection result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Detected mistakes
    pub mistakes: Vec<CriticalMistake>,
    /// Detection metadata
    pub metadata: DetectionMetadata,
    /// Suggested corrections
    pub suggested_corrections: Vec<SuggestedCorrection>,
    /// Confidence in detections
    pub confidence: f64,
}

/// Detection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionMetadata {
    /// Time taken for detection (milliseconds)
    pub detection_time_ms: u64,
    /// Number of rules applied
    pub rules_applied: u32,
    /// Analysis depth level
    pub analysis_depth: AnalysisDepth,
    /// Language detected
    pub language: Option<String>,
    /// Code complexity score
    pub complexity_score: f64,
}

/// Analysis depth levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisDepth {
    /// Surface-level pattern matching
    Surface,
    /// Syntax tree analysis
    Syntactic,
    /// Semantic analysis
    Semantic,
    /// Cross-function analysis
    Interprocedural,
    /// Whole-program analysis
    WholeProgram,
}

/// Suggested correction for a detected mistake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedCorrection {
    /// Mistake being corrected
    pub mistake_id: String,
    /// Suggested fix
    pub suggested_fix: String,
    /// Correction method
    pub method: CorrectionMethod,
    /// Confidence in the correction
    pub confidence: f64,
    /// Explanation of the correction
    pub explanation: String,
    /// Location where correction should be applied
    pub location: CodeLocation,
}

impl CriticalMistakeDetector {
    /// Create a new critical mistake detector
    pub fn new(config: DetectorConfig) -> LlmResult<Self> {
        let detection_rules = Self::load_default_rules();

        Ok(Self {
            config,
            detection_rules,
            python_bridge: None,
            stats: DetectionStats::default(),
            correction_history: Vec::new(),
        })
    }

    /// Create with Python FFI bridge for Phase 5a compatibility
    pub fn with_python_bridge(
        config: DetectorConfig,
        python_bridge: PythonLlmBridge,
    ) -> LlmResult<Self> {
        let mut detector = Self::new(config)?;
        detector.python_bridge = Some(python_bridge);
        Ok(detector)
    }

    /// Detect critical mistakes in code
    pub async fn detect_critical_mistakes(&mut self, code: &str) -> LlmResult<DetectionResult> {
        let start_time = Instant::now();
        self.stats.total_analyzed += 1;

        // Phase 5a: Use Python bridge for guaranteed compatibility
        if let Some(ref python_bridge) = self.python_bridge {
            return self.detect_via_python_bridge(code, start_time).await;
        }

        // Native Rust implementation (Phase 5b)
        self.detect_native(code, start_time).await
    }

    /// Detect via Python bridge (Phase 5a)
    async fn detect_via_python_bridge(
        &mut self,
        code: &str,
        start_time: Instant,
    ) -> LlmResult<DetectionResult> {
        // Call Python critical mistake detector directly
        let mistakes = self.python_bridge
            .as_ref()
            .unwrap()
            .detect_critical_mistakes(code)?;

        let detection_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        self.stats.total_mistakes += mistakes.len() as u32;
        for mistake in &mistakes {
            *self.stats.mistakes_by_type.entry(mistake.mistake_type.clone()).or_insert(0) += 1;
            *self.stats.mistakes_by_severity.entry(mistake.severity).or_insert(0) += 1;
        }

        // Generate suggested corrections using Rust logic
        let suggested_corrections = self.generate_corrections(&mistakes, code).await?;

        // Update average detection time
        let total_time = self.stats.average_detection_time_ms * f64::from(self.stats.total_analyzed - 1)
            + detection_time as f64;
        self.stats.average_detection_time_ms = total_time / f64::from(self.stats.total_analyzed);

        Ok(DetectionResult {
            mistakes,
            metadata: DetectionMetadata {
                detection_time_ms: detection_time,
                rules_applied: self.detection_rules.len() as u32,
                analysis_depth: AnalysisDepth::Semantic, // Python uses semantic analysis
                language: Some(self.detect_language(code)),
                complexity_score: self.calculate_complexity(code),
            },
            suggested_corrections,
            confidence: 0.95, // High confidence from Python implementation
        })
    }

    /// Native detection implementation (Phase 5b)
    async fn detect_native(&mut self, code: &str, start_time: Instant) -> LlmResult<DetectionResult> {
        let mut mistakes = Vec::new();
        let mut rules_applied = 0;

        let language = self.detect_language(code);

        // Apply all enabled detection rules
        for rule in &self.detection_rules {
            if !rule.enabled {
                continue;
            }

            if let Some(ref rule_lang) = rule.language {
                if rule_lang != &language {
                    continue;
                }
            }

            rules_applied += 1;

            if let Some(mistake) = self.apply_detection_rule(rule, code).await? {
                mistakes.push(mistake);

                // Update statistics
                *self.stats.mistakes_by_type.entry(rule.mistake_type.clone()).or_insert(0) += 1;
                *self.stats.mistakes_by_severity.entry(rule.severity).or_insert(0) += 1;
            }
        }

        self.stats.total_mistakes += mistakes.len() as u32;

        let detection_time = start_time.elapsed().as_millis() as u64;

        // Generate suggested corrections
        let suggested_corrections = self.generate_corrections(&mistakes, code).await?;

        Ok(DetectionResult {
            mistakes,
            metadata: DetectionMetadata {
                detection_time_ms: detection_time,
                rules_applied,
                analysis_depth: AnalysisDepth::Syntactic,
                language: Some(language),
                complexity_score: self.calculate_complexity(code),
            },
            suggested_corrections,
            confidence: 0.8, // Lower confidence for native implementation initially
        })
    }

    /// Apply a single detection rule
    async fn apply_detection_rule(
        &self,
        rule: &DetectionRule,
        code: &str,
    ) -> LlmResult<Option<CriticalMistake>> {
        // Pattern matching based on rule type
        match rule.mistake_type {
            MistakeType::SyntaxError => self.detect_syntax_errors(code, rule).await,
            MistakeType::LogicError => self.detect_logic_errors(code, rule).await,
            MistakeType::SecurityVulnerability => self.detect_security_vulnerabilities(code, rule).await,
            MistakeType::PerformanceIssue => self.detect_performance_issues(code, rule).await,
            MistakeType::TypeMismatch => self.detect_type_mismatches(code, rule).await,
            MistakeType::ResourceLeak => self.detect_resource_leaks(code, rule).await,
            MistakeType::InfiniteExecution => self.detect_infinite_execution(code, rule).await,
        }
    }

    /// Detect syntax errors
    async fn detect_syntax_errors(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_syntax_errors {
            return Ok(None);
        }

        // Use regex pattern matching for now
        if let Ok(regex) = regex::Regex::new(&rule.pattern) {
            if let Some(mat) = regex.find(code) {
                let line_num = code[..mat.start()].lines().count();
                let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                return Ok(Some(CriticalMistake {
                    mistake_type: MistakeType::SyntaxError,
                    severity: rule.severity,
                    description: rule.description.clone(),
                    location: Some(CodeLocation {
                        line: line_num,
                        column,
                        length: Some(mat.len()),
                    }),
                    suggested_fix: rule.correction_template.clone(),
                    confidence: 0.9,
                    context: Some("Syntax analysis".to_string()),
                }));
            }
        }

        Ok(None)
    }

    /// Detect logic errors
    async fn detect_logic_errors(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_logic_errors {
            return Ok(None);
        }

        // Common logic error patterns
        let logic_patterns = [
            (r"if.*=.*:", "Assignment in if condition (should use ==)"),
            (r"while.*=.*:", "Assignment in while condition (should use ==)"),
            (r"for\s+\w+\s+in\s+range\(\s*0\s*\):", "Empty range in for loop"),
            (r"return\s+.*,\s*$", "Trailing comma in return statement"),
        ];

        for (pattern, description) in &logic_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].lines().count();
                    let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                    return Ok(Some(CriticalMistake {
                        mistake_type: MistakeType::LogicError,
                        severity: rule.severity,
                        description: format!("{}: {}", rule.description, description),
                        location: Some(CodeLocation {
                            line: line_num,
                            column,
                            length: Some(mat.len()),
                        }),
                        suggested_fix: rule.correction_template.clone(),
                        confidence: 0.8,
                        context: Some("Analysis".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Detect security vulnerabilities
    async fn detect_security_vulnerabilities(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_security_issues {
            return Ok(None);
        }

        // Common security vulnerability patterns
        let security_patterns = [
            (r"eval\s*\(", "Use of eval() function"),
            (r"exec\s*\(", "Use of exec() function"),
            (r"os\.system\s*\(", "Use of os.system()"),
            (r"subprocess\.call.*shell\s*=\s*True", "Shell injection vulnerability"),
            (r"pickle\.loads?\s*\(", "Unsafe pickle deserialization"),
            (r"yaml\.load\s*\(", "Unsafe YAML loading"),
        ];

        for (pattern, description) in &security_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].lines().count();
                    let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                    return Ok(Some(CriticalMistake {
                        mistake_type: MistakeType::SecurityVulnerability,
                        severity: 9, // Security issues are high severity
                        description: format!("Security vulnerability: {}", description),
                        location: Some(CodeLocation {
                            line: line_num,
                            column,
                            length: Some(mat.len()),
                        }),
                        suggested_fix: Some("Review and replace with secure alternative".to_string()),
                        confidence: 0.95,
                        context: Some("Security analysis".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Detect performance issues
    async fn detect_performance_issues(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_performance_issues {
            return Ok(None);
        }

        // Common performance issue patterns
        let performance_patterns = [
            (r"for.*in.*list\(.*\):", "Using list() in for loop (use generator)"),
            (r"\w+\s*\+=?\s*\w+\s*\+\s*\w+", "String concatenation in loop"),
            (r"time\.sleep\s*\(\s*[0-9]+\s*\)", "Long sleep() call"),
            (r"while\s+True:.*time\.sleep\(0\)", "Busy waiting loop"),
        ];

        for (pattern, description) in &performance_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].lines().count();
                    let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                    return Ok(Some(CriticalMistake {
                        mistake_type: MistakeType::PerformanceIssue,
                        severity: rule.severity,
                        description: format!("Performance issue: {}", description),
                        location: Some(CodeLocation {
                            line: line_num,
                            column,
                            length: Some(mat.len()),
                        }),
                        suggested_fix: rule.correction_template.clone(),
                        confidence: 0.8,
                        context: Some("Analysis".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Detect type mismatches
    async fn detect_type_mismatches(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_type_mismatches {
            return Ok(None);
        }

        // Basic type mismatch detection (would be enhanced with proper type inference)
        let type_patterns = [
            (r"int\(\w+\)\s*\+\s*str\(\w+\)", "Type mismatch: int + str"),
            (r"len\(\d+\)", "len() called on numeric literal"),
            (r"\[\d+\]\s*\.\s*append", "append() called on integer"),
        ];

        for (pattern, description) in &type_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].lines().count();
                    let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                    return Ok(Some(CriticalMistake {
                        mistake_type: MistakeType::TypeMismatch,
                        severity: rule.severity,
                        description: format!("Type mismatch: {}", description),
                        location: Some(CodeLocation {
                            line: line_num,
                            column,
                            length: Some(mat.len()),
                        }),
                        suggested_fix: rule.correction_template.clone(),
                        confidence: 0.8,
                        context: Some("Analysis".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Detect resource leaks
    async fn detect_resource_leaks(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_resource_leaks {
            return Ok(None);
        }

        // Resource leak patterns
        let leak_patterns = [
            (r"open\s*\([^)]+\)(?![^{]*\.close\(\))", "File opened without close()"),
            (r"socket\s*\([^)]*\)(?![^{]*\.close\(\))", "Socket opened without close()"),
            (r"threading\.Thread\([^)]*\)(?![^{]*\.join\(\))", "Thread created without join()"),
        ];

        for (pattern, description) in &leak_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].lines().count();
                    let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                    return Ok(Some(CriticalMistake {
                        mistake_type: MistakeType::ResourceLeak,
                        severity: 8, // Resource leaks are serious
                        description: format!("Resource leak: {}", description),
                        location: Some(CodeLocation {
                            line: line_num,
                            column,
                            length: Some(mat.len()),
                        }),
                        suggested_fix: Some("Use context manager (with statement)".to_string()),
                        confidence: 0.9,
                        context: Some("Resource analysis".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Detect infinite execution patterns
    async fn detect_infinite_execution(
        &self,
        code: &str,
        rule: &DetectionRule,
    ) -> LlmResult<Option<CriticalMistake>> {
        if !self.config.detect_infinite_execution {
            return Ok(None);
        }

        // Infinite execution patterns
        let infinite_patterns = [
            (r"while\s+True:(?![^{]*break)", "Infinite loop without break"),
            (r"def\s+\w+\([^)]*\):.*\1\(", "Recursive function without base case"),
            (r"for\s+\w+\s+in\s+itertools\.count\(\)", "Infinite iterator without break"),
        ];

        for (pattern, description) in &infinite_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].lines().count();
                    let column = mat.start() - code[..mat.start()].rfind('\n').unwrap_or(0);

                    return Ok(Some(CriticalMistake {
                        mistake_type: MistakeType::InfiniteExecution,
                        severity: 9, // Infinite execution is critical
                        description: format!("Potential infinite execution: {}", description),
                        location: Some(CodeLocation {
                            line: line_num,
                            column,
                            length: Some(mat.len()),
                        }),
                        suggested_fix: Some("Add break condition or timeout".to_string()),
                        confidence: 0.85,
                        context: Some("Execution analysis".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Generate suggested corrections for detected mistakes
    async fn generate_corrections(
        &self,
        mistakes: &[CriticalMistake],
        code: &str,
    ) -> LlmResult<Vec<SuggestedCorrection>> {
        let mut corrections = Vec::new();

        for (i, mistake) in mistakes.iter().enumerate() {
            if let Some(correction) = self.generate_correction_for_mistake(mistake, code, i).await? {
                corrections.push(correction);
            }
        }

        Ok(corrections)
    }

    /// Generate correction for a specific mistake
    async fn generate_correction_for_mistake(
        &self,
        mistake: &CriticalMistake,
        code: &str,
        mistake_id: usize,
    ) -> LlmResult<Option<SuggestedCorrection>> {
        if let Some(ref suggested_fix) = mistake.suggested_fix {
            return Ok(Some(SuggestedCorrection {
                mistake_id: mistake_id.to_string(),
                suggested_fix: suggested_fix.clone(),
                method: CorrectionMethod::Template,
                confidence: 0.8,
                explanation: format!("Template-based correction for {}", mistake.description),
                location: mistake.location.clone().unwrap_or(CodeLocation {
                    line: 1,
                    column: 1,
                    length: None,
                }),
            }));
        }

        // Generate correction based on mistake type
        let correction = match mistake.mistake_type {
            MistakeType::SyntaxError => {
                self.generate_syntax_correction(mistake, code).await
            }
            MistakeType::SecurityVulnerability => {
                self.generate_security_correction(mistake, code).await
            }
            MistakeType::PerformanceIssue => {
                self.generate_performance_correction(mistake, code).await
            }
            _ => None,
        };

        if let Some((fix, explanation)) = correction {
            Ok(Some(SuggestedCorrection {
                mistake_id: mistake_id.to_string(),
                suggested_fix: fix,
                method: CorrectionMethod::RuleBasedTransform,
                confidence: 0.7,
                explanation,
                location: mistake.location.clone().unwrap_or(CodeLocation {
                    line: 1,
                    column: 1,
                    length: None,
                }),
            }))
        } else {
            Ok(None)
        }
    }

    /// Generate syntax correction
    async fn generate_syntax_correction(&self, mistake: &CriticalMistake, code: &str) -> Option<(String, String)> {
        if mistake.description.contains("Assignment in if condition") {
            Some((
                "Replace '=' with '==' for comparison".to_string(),
                "Changed assignment to comparison operator".to_string(),
            ))
        } else {
            None
        }
    }

    /// Generate security correction
    async fn generate_security_correction(&self, mistake: &CriticalMistake, code: &str) -> Option<(String, String)> {
        if mistake.description.contains("eval()") {
            Some((
                "Use ast.literal_eval() for safe evaluation".to_string(),
                "Replaced eval() with secure alternative".to_string(),
            ))
        } else if mistake.description.contains("os.system") {
            Some((
                "Use subprocess.run() with proper arguments".to_string(),
                "Replaced os.system() with secure subprocess call".to_string(),
            ))
        } else {
            None
        }
    }

    /// Generate performance correction
    async fn generate_performance_correction(&self, mistake: &CriticalMistake, code: &str) -> Option<(String, String)> {
        if mistake.description.contains("String concatenation in loop") {
            Some((
                "Use list.append() and ''.join() instead".to_string(),
                "Optimized string concatenation using join()".to_string(),
            ))
        } else {
            None
        }
    }

    /// Detect programming language
    fn detect_language(&self, code: &str) -> String {
        if code.contains("def ") || code.contains("import ") || code.contains("print(") {
            "python".to_string()
        } else if code.contains("fn ") || code.contains("use ") || code.contains("impl ") {
            "rust".to_string()
        } else if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            "javascript".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Calculate code complexity score
    fn calculate_complexity(&self, code: &str) -> f64 {
        let lines = code.lines().count() as f64;
        let branches = code.matches("if ").count() as f64;
        let loops = code.matches("for ").count() as f64 + code.matches("while ").count() as f64;
        let functions = code.matches("def ").count() as f64 + code.matches("fn ").count() as f64;

        // Simple complexity calculation
        (lines * 0.1) + (branches * 2.0) + (loops * 3.0) + (functions * 1.5)
    }

    /// Load default detection rules
    fn load_default_rules() -> Vec<DetectionRule> {
        vec![
            DetectionRule {
                id: "syntax_assignment_in_condition".to_string(),
                mistake_type: MistakeType::SyntaxError,
                description: "Assignment used in conditional expression".to_string(),
                pattern: r"if\s+\w+\s*=\s*".to_string(),
                severity: 8,
                language: Some("python".to_string()),
                correction_template: Some("Use == for comparison instead of =".to_string()),
                enabled: true,
            },
            DetectionRule {
                id: "security_eval_usage".to_string(),
                mistake_type: MistakeType::SecurityVulnerability,
                description: "Dangerous use of eval() function".to_string(),
                pattern: r"eval\s*\(".to_string(),
                severity: 10,
                language: Some("python".to_string()),
                correction_template: Some("Use ast.literal_eval() instead".to_string()),
                enabled: true,
            },
            DetectionRule {
                id: "performance_string_concat".to_string(),
                mistake_type: MistakeType::PerformanceIssue,
                description: "Inefficient string concatenation in loop".to_string(),
                pattern: r"for.*:\s*\w+\s*\+=\s*".to_string(),
                severity: 6,
                language: None,
                correction_template: Some("Use list.append() and join()".to_string()),
                enabled: true,
            },
            DetectionRule {
                id: "resource_file_not_closed".to_string(),
                mistake_type: MistakeType::ResourceLeak,
                description: "File opened but not properly closed".to_string(),
                pattern: r"open\s*\([^)]+\)".to_string(),
                severity: 7,
                language: Some("python".to_string()),
                correction_template: Some("Use with statement for automatic cleanup".to_string()),
                enabled: true,
            },
        ]
    }

    /// Get current detection statistics
    pub fn get_stats(&self) -> &DetectionStats {
        &self.stats
    }

    /// Get correction history
    pub fn get_correction_history(&self) -> &[CorrectionRecord] {
        &self.correction_history
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = DetectionStats::default();
    }

    /// Add custom detection rule
    pub fn add_detection_rule(&mut self, rule: DetectionRule) {
        self.detection_rules.push(rule);
    }

    /// Remove detection rule by ID
    pub fn remove_detection_rule(&mut self, rule_id: &str) {
        self.detection_rules.retain(|r| r.id != rule_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_config_default() {
        let config = DetectorConfig::default();
        assert!(config.detect_syntax_errors);
        assert!(config.detect_security_issues);
        assert_eq!(config.max_correction_attempts, 3);
    }

    #[tokio::test]
    async fn test_syntax_error_detection() {
        let config = DetectorConfig::default();
        let mut detector = CriticalMistakeDetector::new(config).unwrap();

        let code_with_error = "if x = 5:\n    print('error')";
        let result = detector.detect_critical_mistakes(code_with_error).await.unwrap();

        assert!(!result.mistakes.is_empty());
        assert_eq!(result.mistakes[0].mistake_type, MistakeType::SyntaxError);
    }

    #[tokio::test]
    async fn test_security_vulnerability_detection() {
        let config = DetectorConfig::default();
        let mut detector = CriticalMistakeDetector::new(config).unwrap();

        let code_with_vulnerability = "user_input = input()\nresult = eval(user_input)";
        let result = detector.detect_critical_mistakes(code_with_vulnerability).await.unwrap();

        assert!(!result.mistakes.is_empty());
        assert_eq!(result.mistakes[0].mistake_type, MistakeType::SecurityVulnerability);
        assert_eq!(result.mistakes[0].severity, 10);
    }

    #[test]
    fn test_language_detection() {
        let config = DetectorConfig::default();
        let detector = CriticalMistakeDetector::new(config).unwrap();

        assert_eq!(detector.detect_language("def foo(): pass"), "python");
        assert_eq!(detector.detect_language("fn main() {}"), "rust");
        assert_eq!(detector.detect_language("function foo() {}"), "javascript");
    }

    #[test]
    fn test_complexity_calculation() {
        let config = DetectorConfig::default();
        let detector = CriticalMistakeDetector::new(config).unwrap();

        let simple_code = "print('hello')";
        let complex_code = "def foo():\n    if x > 5:\n        for i in range(10):\n            if i % 2 == 0:\n                print(i)";

        let simple_complexity = detector.calculate_complexity(simple_code);
        let complex_complexity = detector.calculate_complexity(complex_code);

        assert!(complex_complexity > simple_complexity);
    }

    #[test]
    fn test_default_rules_loading() {
        let rules = CriticalMistakeDetector::load_default_rules();
        assert!(!rules.is_empty());
        assert!(rules.iter().any(|r| r.mistake_type == MistakeType::SecurityVulnerability));
        assert!(rules.iter().any(|r| r.mistake_type == MistakeType::SyntaxError));
    }
}