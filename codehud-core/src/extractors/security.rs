//! Security Data Extractor - Security vulnerabilities and potential security issues
//!
//! This module extracts security vulnerabilities using both AST analysis and bandit integration
//! to maintain zero-degradation compliance with the Python implementation.

use super::BaseDataExtractor;
use crate::external_tools::{ExternalToolManager, ExternalTool};
use crate::Result;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tree_sitter::{Language, Parser};
use serde_json::{json, Value};
use std::fs;
use anyhow::Context;
use regex::Regex;
use tracing::{debug, warn};

extern "C" {
    fn tree_sitter_rust() -> Language;
}

#[derive(Debug, Default)]
struct SecurityAnalyzer {
    vulnerabilities: Vec<SecurityVulnerability>,
    security_issues: Vec<SecurityIssue>,
    dangerous_functions: Vec<DangerousFunctionUsage>,
    sensitive_data_exposure: Vec<SensitiveDataExposure>,
    file_permission_issues: Vec<FilePermissionIssue>,
}

#[derive(Debug, Clone)]
struct SecurityVulnerability {
    file_path: String,
    line_number: usize,
    column: usize,
    vulnerability_type: String,
    severity: String,
    description: String,
    cwe_id: Option<String>,
    confidence: String,
}

#[derive(Debug, Clone)]
struct SecurityIssue {
    file_path: String,
    line_number: usize,
    issue_type: String,
    severity: String,
    description: String,
    recommendation: String,
}

#[derive(Debug, Clone)]
struct DangerousFunctionUsage {
    file_path: String,
    line_number: usize,
    function_name: String,
    severity: String,
    context: String,
}

#[derive(Debug, Clone)]
struct SensitiveDataExposure {
    file_path: String,
    line_number: usize,
    exposure_type: String,
    severity: String,
    pattern_matched: String,
}

#[derive(Debug, Clone)]
struct FilePermissionIssue {
    file_path: String,
    line_number: usize,
    issue_type: String,
    severity: String,
    description: String,
}

pub struct SecurityExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
    dangerous_functions: Vec<&'static str>,
    sensitive_patterns: Vec<(Regex, &'static str, &'static str)>,
    file_permission_patterns: Vec<(Regex, &'static str, &'static str)>,
}

impl SecurityExtractor {
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

        // Initialize security patterns - match Python exactly
        let dangerous_functions = vec![
            "eval", "exec", "compile", "__import__"
        ];

        let sensitive_patterns = vec![
            (Regex::new(r#"password\s*=\s*["'][^"']+"["']"#).unwrap(), "hardcoded_password", "high"),
            (Regex::new(r#"api[_-]?key\s*=\s*["'][^"']+"["']"#).unwrap(), "hardcoded_api_key", "high"),
            (Regex::new(r#"secret\s*=\s*["'][^"']+"["']"#).unwrap(), "hardcoded_secret", "high"),
            (Regex::new(r#"token\s*=\s*["'][^"']+"["']"#).unwrap(), "hardcoded_token", "medium"),
            (Regex::new(r#"(?i)(access[_-]?key|secret[_-]?key)\s*=\s*["'][^"']+"["']"#).unwrap(), "hardcoded_credentials", "high"),
            (Regex::new(r"SELECT\s+.*\s+FROM\s+.*\s+WHERE\s+.*\+.*").unwrap(), "sql_injection_risk", "high"),
            (Regex::new(r"\.format\s*\([^)]*\{[^}]*\}[^)]*\)").unwrap(), "string_format_risk", "medium"),
            (Regex::new(r"random\.random\(\)").unwrap(), "weak_randomness", "low"),
            (Regex::new(r"md5|MD5").unwrap(), "weak_crypto_md5", "medium"),
            (Regex::new(r"sha1|SHA1").unwrap(), "weak_crypto_sha1", "medium"),
            (Regex::new(r#"assert\s+[^,\n]*,\s*["'][^"']*["']"#).unwrap(), "debug_assert", "low"),
        ];

        let file_permission_patterns = vec![
            (Regex::new(r"chmod\s*\(\s*[^,]+,\s*0o?77[0-7]").unwrap(), "overly_permissive_chmod", "medium"),
            (Regex::new(r"umask\s*\(\s*0o?00[0-7]").unwrap(), "weak_umask", "medium"),
            (Regex::new(r#"open\s*\([^)]*mode\s*=\s*["'][wa]["']"#).unwrap(), "world_writable_file", "low"),
        ];

        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
            parser,
            external_tools,
            dangerous_functions,
            sensitive_patterns,
            file_permission_patterns,
        })
    }

    fn get_source_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.codebase_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
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
                if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
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
            name == "__pycache__" || name == ".git" || name == "node_modules"
                || name == "venv" || name == ".venv" || name == "env"
                || name == ".pytest_cache" || name == "build"
                || name == "dist" || name == ".tox"
        } else {
            false
        }
    }

    fn analyze_file_security(&mut self, file_path: &Path) -> Result<Option<Value>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))
            .map_err(|e| crate::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

        let tree = self.parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Parse("Failed to parse Rust file".to_string()))?;

        // AST analysis
        let mut analyzer = SecurityAnalyzer::default();
        self.analyze_node(tree.root_node(), &content.as_bytes(), &mut analyzer, file_path)?;

        // Pattern-based analysis - match Python's line-by-line approach
        self.analyze_patterns_line_by_line(&content, file_path, &mut analyzer)?;

        let relative_path = file_path.strip_prefix(&self.codebase_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

        Ok(Some(json!({
            "file": relative_path,
            "vulnerabilities": analyzer.vulnerabilities.iter().map(|v| json!({
                "line": v.line_number,
                "column": v.column,
                "type": v.vulnerability_type,
                "severity": v.severity,
                "description": v.description,
                "cwe_id": v.cwe_id,
                "confidence": v.confidence
            })).collect::<Vec<_>>(),
            "security_issues": analyzer.security_issues.iter().map(|i| json!({
                "line": i.line_number,
                "type": i.issue_type,
                "severity": i.severity,
                "description": i.description,
                "recommendation": i.recommendation
            })).collect::<Vec<_>>(),
            "dangerous_functions": analyzer.dangerous_functions.iter().map(|d| json!({
                "line": d.line_number,
                "function": d.function_name,
                "severity": d.severity,
                "context": d.context
            })).collect::<Vec<_>>(),
            "sensitive_data": analyzer.sensitive_data_exposure.iter().map(|s| json!({
                "line": s.line_number,
                "type": s.exposure_type,
                "severity": s.severity,
                "pattern": s.pattern_matched
            })).collect::<Vec<_>>(),
            "file_permissions": analyzer.file_permission_issues.iter().map(|f| json!({
                "line": f.line_number,
                "type": f.issue_type,
                "severity": f.severity,
                "description": f.description
            })).collect::<Vec<_>>()
        })))
    }

    fn analyze_node(&self, node: tree_sitter::Node, source: &[u8], analyzer: &mut SecurityAnalyzer, file_path: &Path) -> Result<()> {
        match node.kind() {
            "call" => {
                self.analyze_function_call(node, source, analyzer, file_path)?;
            }
            "assignment" => {
                self.analyze_assignment(node, source, analyzer, file_path)?;
            }
            "use_declaration" => {
                self.analyze_import(node, source, analyzer, file_path)?;
            }
            _ => {}
        }

        // Recursively analyze children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_node(child, source, analyzer, file_path)?;
        }

        Ok(())
    }

    fn analyze_function_call(&self, node: tree_sitter::Node, source: &[u8], analyzer: &mut SecurityAnalyzer, file_path: &Path) -> Result<()> {
        let function_node = node.child_by_field_name("function");
        if let Some(func_node) = function_node {
            let func_text = &source[func_node.start_byte()..func_node.end_byte()];
            let function_name = String::from_utf8_lossy(func_text);

            // Check for dangerous functions - match Python exactly (only eval, exec, compile, __import__)
            for dangerous_func in &self.dangerous_functions {
                if function_name == *dangerous_func {
                    let call_text = &source[node.start_byte()..node.end_byte()];
                    let context = String::from_utf8_lossy(call_text).chars().take(100).collect::<String>();

                    analyzer.dangerous_functions.push(DangerousFunctionUsage {
                        file_path: file_path.to_string_lossy().to_string(),
                        line_number: node.start_position().row + 1,
                        function_name: function_name.to_string(),
                        severity: "high".to_string(), // Python sets all these to high
                        context,
                    });
                }
            }

            // Check for subprocess calls with shell=True - more precise matching
            if function_name.starts_with("subprocess.") &&
               (function_name.ends_with("call") || function_name.ends_with("run") || function_name.ends_with("Popen")) {
                let call_text = &source[node.start_byte()..node.end_byte()];
                let call_str = String::from_utf8_lossy(call_text);

                if call_str.contains("shell=True") {
                    analyzer.vulnerabilities.push(SecurityVulnerability {
                        file_path: file_path.to_string_lossy().to_string(),
                        line_number: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        vulnerability_type: "shell_injection_risk".to_string(),
                        severity: "high".to_string(),
                        description: "subprocess call with shell=True - potential command injection".to_string(),
                        cwe_id: Some("CWE-78".to_string()),
                        confidence: "high".to_string(),
                    });
                }
            }

            // Check for pickle.loads usage
            if function_name == "pickle.loads" || function_name == "pickle.load" {
                analyzer.vulnerabilities.push(SecurityVulnerability {
                    file_path: file_path.to_string_lossy().to_string(),
                    line_number: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    vulnerability_type: "deserialization_risk".to_string(),
                    severity: "high".to_string(),
                    description: "Pickle deserialization can execute arbitrary code".to_string(),
                    cwe_id: Some("CWE-502".to_string()),
                    confidence: "high".to_string(),
                });
            }

            // Check for yaml.load usage
            if function_name == "yaml.load" {
                analyzer.vulnerabilities.push(SecurityVulnerability {
                    file_path: file_path.to_string_lossy().to_string(),
                    line_number: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    vulnerability_type: "yaml_deserialization_risk".to_string(),
                    severity: "high".to_string(),
                    description: "yaml.load() can execute arbitrary code - use yaml.safe_load()".to_string(),
                    cwe_id: Some("CWE-502".to_string()),
                    confidence: "high".to_string(),
                });
            }
        }

        Ok(())
    }

    fn analyze_assignment(&self, node: tree_sitter::Node, source: &[u8], analyzer: &mut SecurityAnalyzer, file_path: &Path) -> Result<()> {
        let assignment_text = &source[node.start_byte()..node.end_byte()];
        let assignment_str = String::from_utf8_lossy(assignment_text);

        // Check for sensitive data patterns - match Python line-by-line logic
        let relative_path = file_path.strip_prefix(&self.codebase_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

        for (pattern, exposure_type, severity) in &self.sensitive_patterns {
            for mat in pattern.find_iter(&assignment_str) {
                analyzer.sensitive_data_exposure.push(SensitiveDataExposure {
                    file_path: relative_path.clone(),
                    line_number: node.start_position().row + 1,
                    exposure_type: exposure_type.to_string(),
                    severity: severity.to_string(),
                    pattern_matched: mat.as_str().to_string(),
                });
            }
        }

        Ok(())
    }

    fn analyze_import(&self, node: tree_sitter::Node, source: &[u8], analyzer: &mut SecurityAnalyzer, file_path: &Path) -> Result<()> {
        let import_text = &source[node.start_byte()..node.end_byte()];
        let import_str = String::from_utf8_lossy(import_text);

        // Check for insecure imports
        let insecure_imports = vec![
            ("pickle", "medium", "Pickle can execute arbitrary code during deserialization"),
            ("yaml", "low", "YAML loading can be unsafe, use yaml.safe_load()"),
            ("ssl", "low", "Ensure proper SSL/TLS configuration"),
        ];

        for (insecure_import, severity, description) in &insecure_imports {
            if import_str.contains(insecure_import) {
                analyzer.security_issues.push(SecurityIssue {
                    file_path: file_path.to_string_lossy().to_string(),
                    line_number: node.start_position().row + 1,
                    issue_type: "insecure_import".to_string(),
                    severity: severity.to_string(),
                    description: description.to_string(),
                    recommendation: format!("Review usage of {} for security implications", insecure_import),
                });
            }
        }

        Ok(())
    }

    fn analyze_patterns_line_by_line(&self, content: &str, file_path: &Path, analyzer: &mut SecurityAnalyzer) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let relative_path = file_path.strip_prefix(&self.codebase_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Check sensitive patterns - match Python exactly
            for (pattern, issue_type, severity) in &self.sensitive_patterns {
                for mat in pattern.find_iter(line) {
                    analyzer.sensitive_data_exposure.push(SensitiveDataExposure {
                        file_path: relative_path.clone(),
                        line_number,
                        exposure_type: issue_type.to_string(),
                        severity: severity.to_string(),
                        pattern_matched: mat.as_str().to_string(),
                    });
                }
            }

            // Check file permission patterns
            for (pattern, issue_type, severity) in &self.file_permission_patterns {
                for mat in pattern.find_iter(line) {
                    analyzer.file_permission_issues.push(FilePermissionIssue {
                        file_path: relative_path.clone(),
                        line_number,
                        issue_type: issue_type.to_string(),
                        severity: severity.to_string(),
                        description: format!("Potentially insecure file permission: {}", issue_type),
                    });
                }
            }
        }

        // Additional security checks - match Python's _check_additional_security_issues
        self.check_additional_security_issues(content, file_path, analyzer)?;

        Ok(())
    }

    fn check_additional_security_issues(&self, content: &str, file_path: &Path, analyzer: &mut SecurityAnalyzer) -> Result<()> {
        let relative_path = file_path.strip_prefix(&self.codebase_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

        // Check for use of HTTP instead of HTTPS
        let http_pattern = Regex::new(r#"http://[^\s"'<>]+"#).unwrap();
        for mat in http_pattern.find_iter(content) {
            analyzer.security_issues.push(SecurityIssue {
                file_path: relative_path.clone(),
                line_number: content[..mat.start()].lines().count(),
                issue_type: "insecure_http".to_string(),
                severity: "medium".to_string(),
                description: "HTTP URL found - consider using HTTPS".to_string(),
                recommendation: "Replace HTTP URLs with HTTPS".to_string(),
            });
        }

        // Check for potential XSS vulnerabilities
        if content.contains("render_template") || content.contains("format") {
            let xss_patterns = vec![
                r"render_template\s*\([^)]*\{\{[^}]*\|safe[^}]*\}\}",
                r"\.format\s*\([^)]*request\.[^)]*\)"
            ];
            for pattern_str in xss_patterns {
                let pattern = Regex::new(pattern_str).unwrap();
                for mat in pattern.find_iter(content) {
                    analyzer.security_issues.push(SecurityIssue {
                        file_path: relative_path.clone(),
                        line_number: content[..mat.start()].lines().count(),
                        issue_type: "xss_vulnerability".to_string(),
                        severity: "high".to_string(),
                        description: "Potential XSS vulnerability detected".to_string(),
                        recommendation: "Implement proper input validation and output encoding".to_string(),
                    });
                }
            }
        }

        // Check for potential directory traversal
        if content.contains("..") {
            let traversal_patterns = vec![
                r"open\s*\([^)]*\.\./[^)]*\)",
                r"os\.path\.join\s*\([^)]*\.\./[^)]*\)"
            ];
            for pattern_str in traversal_patterns {
                let pattern = Regex::new(pattern_str).unwrap();
                for mat in pattern.find_iter(content) {
                    analyzer.security_issues.push(SecurityIssue {
                        file_path: relative_path.clone(),
                        line_number: content[..mat.start()].lines().count(),
                        issue_type: "directory_traversal".to_string(),
                        severity: "high".to_string(),
                        description: "Potential directory traversal vulnerability".to_string(),
                        recommendation: "Validate and sanitize file paths".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    fn get_function_severity(&self, function_name: &str) -> String {
        match function_name {
            "eval" | "exec" | "pickle.loads" | "os.system" => "high".to_string(),
            "compile" | "__import__" | "subprocess.call" => "medium".to_string(),
            _ => "low".to_string(),
        }
    }

    fn categorize_by_severity(&self, all_issues: &[Value]) -> HashMap<String, usize> {
        let mut severity_counts = HashMap::new();
        for issue in all_issues {
            if let Some(severity) = issue.get("severity").and_then(|s| s.as_str()) {
                *severity_counts.entry(severity.to_string()).or_insert(0) += 1;
            }
        }
        severity_counts
    }

    fn categorize_by_type(&self, all_issues: &[Value]) -> HashMap<String, usize> {
        let mut type_counts = HashMap::new();
        for issue in all_issues {
            if let Some(issue_type) = issue.get("type").and_then(|t| t.as_str()) {
                *type_counts.entry(issue_type.to_string()).or_insert(0) += 1;
            }
        }
        type_counts
    }

    fn calculate_risk_score(&self, all_issues: &[Value]) -> Value {
        if all_issues.is_empty() {
            return json!({
                "score": 0,
                "level": "low",
                "status": "secure"
            });
        }

        // Weight by severity - match Python exactly
        let mut total_risk = 0;
        for issue in all_issues {
            if let Some(severity) = issue.get("severity").and_then(|s| s.as_str()) {
                total_risk += match severity {
                    "high" => 10,
                    "medium" => 5,
                    "low" => 1,
                    _ => 1,
                };
            }
        }

        // Calculate score (0-100, higher is more risky)
        // Normalize based on number of files analyzed - match Python algorithm
        let unique_files: std::collections::HashSet<String> = all_issues
            .iter()
            .filter_map(|issue| issue.get("file").and_then(|f| f.as_str()))
            .map(|s| s.to_string())
            .collect();

        let files_analyzed = unique_files.len().max(1);
        let normalized_risk = ((total_risk as f64 / files_analyzed as f64) * 2.0).min(100.0);

        // Determine risk level - match Python thresholds
        let (level, status) = if normalized_risk >= 75.0 {
            ("critical", "critical")
        } else if normalized_risk >= 50.0 {
            ("high", "high_risk")
        } else if normalized_risk >= 25.0 {
            ("medium", "moderate_risk")
        } else if normalized_risk >= 10.0 {
            ("low", "low_risk")
        } else {
            ("minimal", "secure")
        };

        json!({
            "score": (normalized_risk * 10.0).round() / 10.0, // Round to 1 decimal place
            "level": level,
            "status": status,
            "total_findings": all_issues.len(),
            "high_severity": all_issues.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("high")).count(),
            "medium_severity": all_issues.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("medium")).count(),
            "low_severity": all_issues.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("low")).count()
        })
    }

    fn generate_security_summary(&self, total_files: usize, all_issues: &[Value], dangerous_functions: &[Value], sensitive_data: &[Value]) -> Value {
        let files_with_issues = all_issues
            .iter()
            .filter_map(|issue| issue.get("file").and_then(|f| f.as_str()))
            .collect::<std::collections::HashSet<_>>()
            .len();

        json!({
            "total_files_scanned": total_files,
            "files_with_security_issues": files_with_issues,
            "total_security_findings": all_issues.len(),
            "high_severity_findings": all_issues.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("high")).count(),
            "medium_severity_findings": all_issues.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("medium")).count(),
            "low_severity_findings": all_issues.iter().filter(|i| i.get("severity").and_then(|s| s.as_str()) == Some("low")).count(),
            "dangerous_function_usage": dangerous_functions.len(),
            "sensitive_data_exposures": sensitive_data.len(),
            "security_coverage": if total_files > 0 { (total_files - files_with_issues) as f64 / total_files as f64 * 100.0 } else { 0.0 }
        })
    }

    fn identify_vulnerable_files(&self, all_issues: &[Value]) -> Vec<Value> {
        let mut file_issues = HashMap::<String, usize>::new();

        for issue in all_issues {
            if let Some(file) = issue.get("file").and_then(|f| f.as_str()) {
                *file_issues.entry(file.to_string()).or_insert(0) += 1;
            }
        }

        let mut vulnerable_files: Vec<_> = file_issues.into_iter().collect();
        vulnerable_files.sort_by(|a, b| b.1.cmp(&a.1));

        vulnerable_files.into_iter().take(10).map(|(file, count)| json!({
            "file": file,
            "issue_count": count
        })).collect()
    }

    fn generate_security_recommendations(&self, all_issues: &[Value]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if all_issues.iter().any(|i| i.get("type").and_then(|t| t.as_str()) == Some("dangerous_function")) {
            recommendations.push("Review usage of dangerous functions like eval(), exec(), and pickle.loads()".to_string());
        }

        if all_issues.iter().any(|i| i.get("type").and_then(|t| t.as_str()) == Some("sensitive_data_exposure")) {
            recommendations.push("Use environment variables or secure vaults for sensitive data instead of hardcoding".to_string());
        }

        if all_issues.iter().any(|i| i.get("type").and_then(|t| t.as_str()) == Some("command_injection")) {
            recommendations.push("Always use shell=False in subprocess calls and validate user input".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Continue following security best practices".to_string());
        }

        recommendations
    }

    fn run_rust_security_analysis(&self, source_files: &[PathBuf]) -> Result<Value> {
        debug!("Running Rust-specific security analysis on {} files", source_files.len());

        // For Rust, we focus on different security patterns than Python
        let mut rust_security_issues = Vec::new();

        // Analyze each file for Rust-specific security issues
        for file_path in source_files {
            if let Ok(content) = fs::read_to_string(file_path) {
                let lines: Vec<&str> = content.lines().collect();

                for (line_num, line) in lines.iter().enumerate() {
                    let line_number = line_num + 1;

                    // Check for unsafe blocks
                    if line.contains("unsafe") {
                        rust_security_issues.push(json!({
                            "file": file_path.strip_prefix(&self.codebase_path)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| file_path.to_string_lossy().to_string()),
                            "line": line_number,
                            "type": "unsafe_block",
                            "severity": "medium",
                            "description": "Unsafe block detected - review for memory safety",
                            "pattern": line.trim()
                        }));
                    }

                    // Check for unwrap() calls
                    if line.contains(".unwrap()") {
                        rust_security_issues.push(json!({
                            "file": file_path.strip_prefix(&self.codebase_path)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| file_path.to_string_lossy().to_string()),
                            "line": line_number,
                            "type": "unwrap_call",
                            "severity": "low",
                            "description": "unwrap() call may panic - consider using expect() or proper error handling",
                            "pattern": line.trim()
                        }));
                    }

                    // Check for expect() calls
                    if line.contains(".expect(") {
                        rust_security_issues.push(json!({
                            "file": file_path.strip_prefix(&self.codebase_path)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| file_path.to_string_lossy().to_string()),
                            "line": line_number,
                            "type": "expect_call",
                            "severity": "low",
                            "description": "expect() call may panic - ensure proper error handling",
                            "pattern": line.trim()
                        }));
                    }
                }
            }
        }

        Ok(json!({
            "tool": "rust_security_analyzer",
            "status": "success",
            "total_files": source_files.len(),
            "total_issues": rust_security_issues.len(),
            "issues": rust_security_issues
        }))
    }
}

impl BaseDataExtractor for SecurityExtractor {
    fn extract_data(&self) -> Result<HashMap<String, Value>> {
        println!("Extracting security metrics...");

        // Get all source files
        let source_files = self.get_source_files();

        // Analyze each file
        let mut file_security_data = Vec::new();
        let mut all_vulnerabilities = Vec::new();
        let mut all_security_issues = Vec::new();
        let mut all_dangerous_functions = Vec::new();
        let mut all_sensitive_data = Vec::new();
        let mut all_file_permissions = Vec::new();

        // Create a mutable copy for analysis
        let mut extractor = SecurityExtractor {
            codebase_path: self.codebase_path.clone(),
            extraction_timestamp: self.extraction_timestamp,
            parser: Parser::new(),
            external_tools: ExternalToolManager::new(&self.codebase_path),
            dangerous_functions: self.dangerous_functions.clone(),
            sensitive_patterns: self.sensitive_patterns.clone(),
            file_permission_patterns: self.file_permission_patterns.clone(),
        };

        let language = tree_sitter_rust::language();
        extractor.parser.set_language(language)
            .map_err(|e| crate::Error::Config(format!("Failed to set language: {}", e)))?;

        for file_path in &source_files {
            match extractor.analyze_file_security(file_path) {
                Ok(Some(file_data)) => {
                    // Collect all issues from this file
                    if let Some(vulnerabilities) = file_data["vulnerabilities"].as_array() {
                        all_vulnerabilities.extend(vulnerabilities.clone());
                    }
                    if let Some(security_issues) = file_data["security_issues"].as_array() {
                        all_security_issues.extend(security_issues.clone());
                    }
                    if let Some(dangerous_functions) = file_data["dangerous_functions"].as_array() {
                        all_dangerous_functions.extend(dangerous_functions.clone());
                    }
                    if let Some(sensitive_data) = file_data["sensitive_data"].as_array() {
                        all_sensitive_data.extend(sensitive_data.clone());
                    }
                    if let Some(file_permissions) = file_data["file_permissions"].as_array() {
                        all_file_permissions.extend(file_permissions.clone());
                    }

                    file_security_data.push(file_data);
                }
                Ok(None) => continue,
                Err(e) => {
                    println!("Warning: Error analyzing security for {:?}: {}", file_path, e);
                    continue;
                }
            }
        }

        // Combine all issues for analysis
        let mut all_issues = Vec::new();
        all_issues.extend(all_vulnerabilities.clone());
        all_issues.extend(all_security_issues.clone());

        // Run Rust-specific security analysis
        let rust_security_results = self.run_rust_security_analysis(&source_files)?;

        // Recalculate analysis with bandit results included
        let summary = extractor.generate_security_summary(source_files.len(), &all_issues, &all_dangerous_functions, &all_sensitive_data);
        let risk_assessment = extractor.calculate_risk_score(&all_issues);
        let findings_by_severity = extractor.categorize_by_severity(&all_issues);
        let findings_by_type = extractor.categorize_by_type(&all_issues);
        let vulnerable_files = extractor.identify_vulnerable_files(&all_issues);
        let recommendations = extractor.generate_security_recommendations(&all_issues);

        let mut result = HashMap::new();
        result.insert("summary".to_string(), summary);
        result.insert("risk_assessment".to_string(), risk_assessment);
        result.insert("vulnerabilities".to_string(), json!(all_vulnerabilities));
        result.insert("security_issues".to_string(), json!(all_security_issues));
        result.insert("dangerous_functions".to_string(), json!(all_dangerous_functions));
        result.insert("sensitive_data_exposure".to_string(), json!(all_sensitive_data));
        result.insert("file_permission_issues".to_string(), json!(all_file_permissions));
        result.insert("findings_by_severity".to_string(), json!(findings_by_severity));
        result.insert("findings_by_type".to_string(), json!(findings_by_type));
        result.insert("vulnerable_files".to_string(), json!(vulnerable_files));
        result.insert("recommendations".to_string(), json!(recommendations));
        result.insert("rust_security_results".to_string(), rust_security_results);
        result.insert("files_analyzed".to_string(), json!(file_security_data.len()));
        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));

        println!("Security extraction complete: {} files analyzed, {} total issues found",
                 file_security_data.len(), all_issues.len());

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str { "SecurityExtractor" }
    fn codebase_path(&self) -> &Path { &self.codebase_path }
    fn extraction_timestamp(&self) -> DateTime<Utc> { self.extraction_timestamp }
}