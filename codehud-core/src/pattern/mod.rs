//! Pattern detection module for CodeHUD core
//!
//! This module provides pattern detection capabilities for identifying
//! anti-patterns, code smells, architectural patterns, and security patterns.
//!
//! The pattern detection must produce identical results to the Python implementation
//! to ensure zero degradation in analysis accuracy.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use regex::Regex;

/// Types of patterns that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    AntiPattern,
    CodeSmell,
    Architectural,
    Design,
    Security,
    Performance,
}

/// Severity levels for detected patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// A detected pattern with location and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub severity: Severity,
    pub name: String,
    pub description: String,
    pub file_path: String,
    pub line_start: i32,
    pub line_end: i32,
    pub confidence: f64,  // 0.0 to 1.0
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Rule for pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    pub name: String,
    pub pattern_type: PatternType,
    pub severity: Severity,
    pub regex_pattern: String,
    pub description: String,
    pub confidence: f64,
}

/// Pattern detector that identifies various code patterns
#[derive(Debug)]
pub struct PatternDetector {
    anti_patterns: Vec<AntiPatternRule>,
    code_smells: Vec<CodeSmellRule>,
    architectural_patterns: Vec<ArchPatternRule>,
    security_patterns: Vec<SecurityPatternRule>,
}

/// Anti-pattern detection rule
#[derive(Debug, Clone)]
pub struct AntiPatternRule {
    pub base: PatternRule,
    pub compiled_regex: Regex,
}

/// Code smell detection rule
#[derive(Debug, Clone)]
pub struct CodeSmellRule {
    pub base: PatternRule,
    pub compiled_regex: Regex,
}

/// Architectural pattern detection rule
#[derive(Debug, Clone)]
pub struct ArchPatternRule {
    pub base: PatternRule,
    pub compiled_regex: Regex,
}

/// Security pattern detection rule
#[derive(Debug, Clone)]
pub struct SecurityPatternRule {
    pub base: PatternRule,
    pub compiled_regex: Regex,
}

impl PatternDetector {
    /// Create a new pattern detector with default rules
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            anti_patterns: Self::load_anti_pattern_rules()?,
            code_smells: Self::load_code_smell_rules()?,
            architectural_patterns: Self::load_arch_pattern_rules()?,
            security_patterns: Self::load_security_pattern_rules()?,
        })
    }

    /// Detect all patterns in the given code
    pub fn detect_patterns(&self, code: &str, file_path: &str) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        // Detect anti-patterns
        patterns.extend(self.detect_anti_patterns(code, file_path));
        
        // Detect code smells
        patterns.extend(self.detect_code_smells(code, file_path));
        
        // Detect architectural patterns
        patterns.extend(self.detect_architectural_patterns(code, file_path));
        
        // Detect security patterns
        patterns.extend(self.detect_security_patterns(code, file_path));

        patterns
    }

    /// Detect anti-patterns in code
    fn detect_anti_patterns(&self, code: &str, file_path: &str) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();
        
        for rule in &self.anti_patterns {
            for mat in rule.compiled_regex.find_iter(code) {
                // Calculate line numbers (Python-compatible)
                let line_start = Self::get_line_number(code, mat.start()) as i32;
                let line_end = Self::get_line_number(code, mat.end()) as i32;
                
                patterns.push(DetectedPattern {
                    pattern_type: PatternType::AntiPattern,
                    severity: rule.base.severity,
                    name: rule.base.name.clone(),
                    description: rule.base.description.clone(),
                    file_path: file_path.to_string(),
                    line_start,
                    line_end,
                    confidence: rule.base.confidence,
                    metadata: HashMap::new(),
                });
            }
        }
        
        patterns
    }

    /// Detect code smells
    fn detect_code_smells(&self, code: &str, file_path: &str) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        for rule in &self.code_smells {
            for mat in rule.compiled_regex.find_iter(code) {
                let line_start = Self::get_line_number(code, mat.start());
                let line_end = Self::get_line_number(code, mat.end());

                patterns.push(DetectedPattern {
                    pattern_type: PatternType::CodeSmell,
                    severity: rule.base.severity,
                    name: rule.base.name.clone(),
                    description: rule.base.description.clone(),
                    file_path: file_path.to_string(),
                    line_start: line_start as i32,
                    line_end: line_end as i32,
                    confidence: rule.base.confidence,
                    metadata: HashMap::new(),
                });
            }
        }

        patterns
    }

    /// Detect architectural patterns
    fn detect_architectural_patterns(&self, code: &str, file_path: &str) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        for rule in &self.architectural_patterns {
            for mat in rule.compiled_regex.find_iter(code) {
                let line_start = Self::get_line_number(code, mat.start());
                let line_end = Self::get_line_number(code, mat.end());

                patterns.push(DetectedPattern {
                    pattern_type: PatternType::Architectural,
                    severity: rule.base.severity,
                    name: rule.base.name.clone(),
                    description: rule.base.description.clone(),
                    file_path: file_path.to_string(),
                    line_start: line_start as i32,
                    line_end: line_end as i32,
                    confidence: rule.base.confidence,
                    metadata: HashMap::new(),
                });
            }
        }

        patterns
    }

    /// Detect security patterns
    fn detect_security_patterns(&self, code: &str, file_path: &str) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        for rule in &self.security_patterns {
            for mat in rule.compiled_regex.find_iter(code) {
                let line_start = Self::get_line_number(code, mat.start());
                let line_end = Self::get_line_number(code, mat.end());

                patterns.push(DetectedPattern {
                    pattern_type: PatternType::Security,
                    severity: rule.base.severity,
                    name: rule.base.name.clone(),
                    description: rule.base.description.clone(),
                    file_path: file_path.to_string(),
                    line_start: line_start as i32,
                    line_end: line_end as i32,
                    confidence: rule.base.confidence,
                    metadata: HashMap::new(),
                });
            }
        }

        patterns
    }

    /// Get line number for a byte position in the code (1-indexed like Python)
    fn get_line_number(code: &str, byte_pos: usize) -> usize {
        code[..byte_pos].chars().filter(|&c| c == '\n').count() + 1
    }

    /// Load anti-pattern detection rules
    fn load_anti_pattern_rules() -> crate::Result<Vec<AntiPatternRule>> {
        let mut rules = Vec::new();

        // God Object anti-pattern
        rules.push(AntiPatternRule {
            base: PatternRule {
                name: "god_object".to_string(),
                pattern_type: PatternType::AntiPattern,
                severity: Severity::High,
                regex_pattern: r"class\s+\w+\s*\([^)]*\)\s*:\s*(?:\n.*?){100,}".to_string(),
                description: "Large class with too many responsibilities (God Object)".to_string(),
                confidence: 0.8,
            },
            compiled_regex: Regex::new(r"class\s+\w+\s*\([^)]*\)\s*:\s*(?:\n.*?){100,}")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Long parameter list
        rules.push(AntiPatternRule {
            base: PatternRule {
                name: "long_parameter_list".to_string(),
                pattern_type: PatternType::AntiPattern,
                severity: Severity::Medium,
                regex_pattern: r"def\s+\w+\s*\([^)]*,.*?,.*?,.*?,.*?,.*?\)".to_string(),
                description: "Function with too many parameters".to_string(),
                confidence: 0.9,
            },
            compiled_regex: Regex::new(r"def\s+\w+\s*\([^)]*,.*?,.*?,.*?,.*?,.*?\)")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Deep nesting
        rules.push(AntiPatternRule {
            base: PatternRule {
                name: "deep_nesting".to_string(),
                pattern_type: PatternType::AntiPattern,
                severity: Severity::Medium,
                regex_pattern: r"(\s{16,})if\s".to_string(),
                description: "Deeply nested code structure".to_string(),
                confidence: 0.7,
            },
            compiled_regex: Regex::new(r"(\s{16,})if\s")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        Ok(rules)
    }

    /// Load code smell detection rules
    fn load_code_smell_rules() -> crate::Result<Vec<CodeSmellRule>> {
        let mut rules = Vec::new();

        // Dead code (unused imports)
        rules.push(CodeSmellRule {
            base: PatternRule {
                name: "unused_import".to_string(),
                pattern_type: PatternType::CodeSmell,
                severity: Severity::Low,
                regex_pattern: r"import\s+\w+.*(?:\n(?!.*\b\1\b).*)*".to_string(),
                description: "Unused import statement".to_string(),
                confidence: 0.6,
            },
            compiled_regex: Regex::new(r"import\s+(\w+)")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Magic numbers
        rules.push(CodeSmellRule {
            base: PatternRule {
                name: "magic_number".to_string(),
                pattern_type: PatternType::CodeSmell,
                severity: Severity::Medium,
                regex_pattern: r"\b(?:(?!0|1)\d{2,}|\d+\.\d+)\b".to_string(),
                description: "Magic number found - consider using named constants".to_string(),
                confidence: 0.7,
            },
            compiled_regex: Regex::new(r"\b(?:(?!0|1)\d{2,}|\d+\.\d+)\b")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Long method
        rules.push(CodeSmellRule {
            base: PatternRule {
                name: "long_method".to_string(),
                pattern_type: PatternType::CodeSmell,
                severity: Severity::Medium,
                regex_pattern: r"def\s+\w+.*?(?:\n.*?){30,}(?=\n\s*def|\n\s*class|\Z)".to_string(),
                description: "Method is too long - consider breaking it down".to_string(),
                confidence: 0.8,
            },
            compiled_regex: Regex::new(r"def\s+\w+.*?(?:\n.*?){30,}(?=\n\s*def|\n\s*class|\Z)")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        Ok(rules)
    }

    /// Load architectural pattern detection rules
    fn load_arch_pattern_rules() -> crate::Result<Vec<ArchPatternRule>> {
        let mut rules = Vec::new();

        // Singleton pattern
        rules.push(ArchPatternRule {
            base: PatternRule {
                name: "singleton_pattern".to_string(),
                pattern_type: PatternType::Architectural,
                severity: Severity::Low,
                regex_pattern: r"class\s+\w+.*?_instance\s*=\s*None".to_string(),
                description: "Singleton pattern detected".to_string(),
                confidence: 0.8,
            },
            compiled_regex: Regex::new(r"class\s+\w+.*?_instance\s*=\s*None")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Factory pattern
        rules.push(ArchPatternRule {
            base: PatternRule {
                name: "factory_pattern".to_string(),
                pattern_type: PatternType::Architectural,
                severity: Severity::Low,
                regex_pattern: r"def\s+create_\w+\s*\(.*?\)\s*:".to_string(),
                description: "Factory pattern detected".to_string(),
                confidence: 0.7,
            },
            compiled_regex: Regex::new(r"def\s+create_\w+\s*\(.*?\)\s*:")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Observer pattern
        rules.push(ArchPatternRule {
            base: PatternRule {
                name: "observer_pattern".to_string(),
                pattern_type: PatternType::Architectural,
                severity: Severity::Low,
                regex_pattern: r"def\s+notify\s*\(.*?\)\s*:|def\s+subscribe\s*\(.*?\)\s*:".to_string(),
                description: "Observer pattern detected".to_string(),
                confidence: 0.6,
            },
            compiled_regex: Regex::new(r"def\s+(?:notify|subscribe)\s*\(.*?\)\s*:")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        Ok(rules)
    }

    /// Load security pattern detection rules
    fn load_security_pattern_rules() -> crate::Result<Vec<SecurityPatternRule>> {
        let mut rules = Vec::new();

        // SQL injection vulnerability
        rules.push(SecurityPatternRule {
            base: PatternRule {
                name: "sql_injection".to_string(),
                pattern_type: PatternType::Security,
                severity: Severity::Critical,
                regex_pattern: r#"(?:execute|query)\s*\(\s*["'].*?%s.*?["']"#.to_string(),
                description: "Potential SQL injection vulnerability".to_string(),
                confidence: 0.9,
            },
            compiled_regex: Regex::new(r#"(?:execute|query)\s*\(\s*["'].*?%s.*?["']"#)
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Hardcoded password
        rules.push(SecurityPatternRule {
            base: PatternRule {
                name: "hardcoded_password".to_string(),
                pattern_type: PatternType::Security,
                severity: Severity::High,
                regex_pattern: r#"(?:password|passwd|pwd)\s*=\s*["'][^"']{6,}["']"#.to_string(),
                description: "Hardcoded password detected".to_string(),
                confidence: 0.8,
            },
            compiled_regex: Regex::new(r#"(?:password|passwd|pwd)\s*=\s*["'][^"']{6,}["']"#)
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Command injection
        rules.push(SecurityPatternRule {
            base: PatternRule {
                name: "command_injection".to_string(),
                pattern_type: PatternType::Security,
                severity: Severity::High,
                regex_pattern: r"(?:system|exec|eval|subprocess\.call)\s*\(.*?\+.*?\)".to_string(),
                description: "Potential command injection vulnerability".to_string(),
                confidence: 0.7,
            },
            compiled_regex: Regex::new(r"(?:system|exec|eval|subprocess\.call)\s*\(.*?\+.*?\)")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        // Insecure random
        rules.push(SecurityPatternRule {
            base: PatternRule {
                name: "insecure_random".to_string(),
                pattern_type: PatternType::Security,
                severity: Severity::Medium,
                regex_pattern: r"import\s+random|from\s+random\s+import".to_string(),
                description: "Using insecure random for security-sensitive operations".to_string(),
                confidence: 0.6,
            },
            compiled_regex: Regex::new(r"import\s+random|from\s+random\s+import")
                .map_err(|e| crate::Error::Config(format!("Invalid regex: {}", e)))?,
        });

        Ok(rules)
    }

    /// Filter patterns by severity
    pub fn filter_by_severity(patterns: &[DetectedPattern], min_severity: Severity) -> Vec<&DetectedPattern> {
        patterns.iter()
            .filter(|p| p.severity >= min_severity)
            .collect()
    }

    /// Filter patterns by type
    pub fn filter_by_type(patterns: &[DetectedPattern], pattern_type: PatternType) -> Vec<&DetectedPattern> {
        patterns.iter()
            .filter(|p| p.pattern_type == pattern_type)
            .collect()
    }

    /// Get pattern statistics
    pub fn get_statistics(patterns: &[DetectedPattern]) -> PatternStatistics {
        let mut stats = PatternStatistics::default();
        
        for pattern in patterns {
            match pattern.severity {
                Severity::Low => stats.low_severity += 1,
                Severity::Medium => stats.medium_severity += 1,
                Severity::High => stats.high_severity += 1,
                Severity::Critical => stats.critical_severity += 1,
            }
            
            match pattern.pattern_type {
                PatternType::AntiPattern => stats.anti_patterns += 1,
                PatternType::CodeSmell => stats.code_smells += 1,
                PatternType::Architectural => stats.architectural_patterns += 1,
                PatternType::Design => stats.design_patterns += 1,
                PatternType::Security => stats.security_patterns += 1,
                PatternType::Performance => stats.performance_patterns += 1,
            }
        }
        
        stats
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new().expect("Failed to create default PatternDetector")
    }
}

/// Statistics about detected patterns
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PatternStatistics {
    pub low_severity: usize,
    pub medium_severity: usize,
    pub high_severity: usize,
    pub critical_severity: usize,
    pub anti_patterns: usize,
    pub code_smells: usize,
    pub architectural_patterns: usize,
    pub design_patterns: usize,
    pub security_patterns: usize,
    pub performance_patterns: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn test_line_number_calculation() {
        let code = "line 1\nline 2\nline 3\n";
        assert_eq!(PatternDetector::get_line_number(code, 0), 1);
        assert_eq!(PatternDetector::get_line_number(code, 7), 2);
        assert_eq!(PatternDetector::get_line_number(code, 14), 3);
    }

    #[test]
    fn test_pattern_filtering() {
        let patterns = vec![
            DetectedPattern {
                pattern_type: PatternType::AntiPattern,
                severity: Severity::High,
                name: "Test Pattern".to_string(),
                description: "Test".to_string(),
                file_path: "test.py".to_string(),
                line_start: 1,
                line_end: 1,
                confidence: 0.9,
                metadata: HashMap::new(),
            },
        ];

        let high_severity = PatternDetector::filter_by_severity(&patterns, Severity::High);
        assert_eq!(high_severity.len(), 1);

        let critical_severity = PatternDetector::filter_by_severity(&patterns, Severity::Critical);
        assert_eq!(critical_severity.len(), 0);
    }

    #[test]
    fn test_pattern_statistics() {
        let patterns = vec![
            DetectedPattern {
                pattern_type: PatternType::AntiPattern,
                severity: Severity::High,
                name: "Test".to_string(),
                description: "Test".to_string(),
                file_path: "test.py".to_string(),
                line_start: 1,
                line_end: 1,
                confidence: 0.9,
                metadata: HashMap::new(),
            },
        ];

        let stats = PatternDetector::get_statistics(&patterns);
        assert_eq!(stats.high_severity, 1);
        assert_eq!(stats.anti_patterns, 1);
    }
}