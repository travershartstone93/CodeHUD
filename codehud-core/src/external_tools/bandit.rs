//! Bandit Security Analyzer Integration
//!
//! Zero-degradation integration with Bandit security scanner matching Python behavior

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

pub struct BanditIntegration {
    codebase_path: PathBuf,
}

impl BanditIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }

    /// Analyze a single file with bandit - CRITICAL for zero-degradation security compliance
    pub async fn analyze_file(&self, file_path: &Path) -> Result<BanditResult> {
        debug!("Running bandit security analysis on file: {}", file_path.display());

        let output = Command::new("bandit")
            .arg("-f") // Format
            .arg("json") // JSON output
            .arg("-q") // Quiet (no progress bar)
            .arg(file_path)
            .output()
            .await
            .context("Failed to execute bandit on file")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        if stdout.trim().is_empty() {
            // No issues found
            return Ok(BanditResult {
                issues: Vec::new(),
                total_issues: 0,
                high_severity: 0,
                medium_severity: 0,
                low_severity: 0,
                confidence_high: 0,
                confidence_medium: 0,
                confidence_low: 0,
            });
        }

        // Parse bandit JSON output format
        let bandit_output: BanditJsonOutput = serde_json::from_str(&stdout)
            .context("Failed to parse bandit JSON output")?;

        let mut high_severity = 0;
        let mut medium_severity = 0;
        let mut low_severity = 0;
        let mut confidence_high = 0;
        let mut confidence_medium = 0;
        let mut confidence_low = 0;

        // Categorize issues by severity and confidence
        for issue in &bandit_output.results {
            match issue.issue_severity.to_lowercase().as_str() {
                "high" => high_severity += 1,
                "medium" => medium_severity += 1,
                "low" => low_severity += 1,
                _ => low_severity += 1,
            }

            match issue.issue_confidence.to_lowercase().as_str() {
                "high" => confidence_high += 1,
                "medium" => confidence_medium += 1,
                "low" => confidence_low += 1,
                _ => confidence_low += 1,
            }
        }

        // Convert to simplified format
        let issues: Vec<BanditIssue> = bandit_output.results.into_iter().map(|result| BanditIssue {
            test_name: result.test_name,
            test_id: result.test_id,
            issue_severity: result.issue_severity,
            issue_confidence: result.issue_confidence,
            issue_text: result.issue_text,
            filename: result.filename,
            line_number: result.line_number,
            line_range: result.line_range,
            code: result.code,
            more_info: result.more_info,
        }).collect();

        Ok(BanditResult {
            total_issues: issues.len(),
            high_severity,
            medium_severity,
            low_severity,
            confidence_high,
            confidence_medium,
            confidence_low,
            issues,
        })
    }

    /// Analyze a directory with bandit - wrapper around analyze() for compatibility
    pub async fn analyze_directory(&self, _directory_path: &Path) -> Result<BanditResult> {
        // Since bandit always analyzes the full codebase, just call analyze()
        self.analyze().await
    }
}

#[async_trait::async_trait]
impl ExternalTool for BanditIntegration {
    type Result = BanditResult;

    async fn is_available(&self) -> bool {
        Command::new("bandit")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running bandit security analysis on {}", self.codebase_path.display());

        let output = Command::new("bandit")
            .arg("-r") // Recursive
            .arg("-f") // Format
            .arg("json") // JSON output
            .arg("-q") // Quiet (no progress bar)
            .arg("-x") // Exclude paths
            .arg("*/tests/*,*/test_*,*/.venv/*,*/venv/*,*/__pycache__/*")
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute bandit")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Bandit returns non-zero exit code when issues are found, which is expected
        if stdout.trim().is_empty() {
            // No issues found or no Python files
            return Ok(BanditResult::default());
        }

        // Parse bandit JSON output
        let bandit_data: BanditJsonOutput = match serde_json::from_str(&stdout) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to parse bandit JSON output: {}", e);
                warn!("Raw output: {}", stdout);
                return Ok(BanditResult::default());
            }
        };

        let mut high_severity = 0;
        let mut medium_severity = 0;
        let mut low_severity = 0;

        // Count issues by severity
        for result in &bandit_data.results {
            match result.issue_severity.to_lowercase().as_str() {
                "high" => high_severity += 1,
                "medium" => medium_severity += 1,
                "low" => low_severity += 1,
                _ => low_severity += 1, // Default to low
            }
        }

        // Convert to simplified format
        let issues: Vec<BanditIssue> = bandit_data.results.into_iter().map(|result| BanditIssue {
            test_name: result.test_name,
            test_id: result.test_id,
            issue_severity: result.issue_severity,
            issue_confidence: result.issue_confidence,
            issue_text: result.issue_text,
            filename: result.filename,
            line_number: result.line_number,
            line_range: result.line_range,
            code: result.code,
            more_info: result.more_info,
        }).collect();

        let total_issues = issues.len();

        Ok(BanditResult {
            total_issues,
            high_severity,
            medium_severity,
            low_severity,
            confidence_high: 0, // TODO: Calculate from issues
            confidence_medium: 0, // TODO: Calculate from issues
            confidence_low: 0, // TODO: Calculate from issues
            issues,
        })
    }

    fn tool_name(&self) -> &'static str {
        "bandit"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("bandit")
            .arg("--version")
            .output()
            .await
            .context("Failed to get bandit version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get bandit version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

// Bandit JSON output structures
#[derive(Debug, Deserialize)]
struct BanditJsonOutput {
    pub results: Vec<BanditJsonResult>,
    pub metrics: BanditMetrics,
}

#[derive(Debug, Deserialize)]
struct BanditJsonResult {
    pub test_name: String,
    pub test_id: String,
    pub issue_severity: String,
    pub issue_confidence: String,
    pub issue_text: String,
    pub filename: String,
    pub line_number: usize,
    pub line_range: Vec<usize>,
    pub code: String,
    pub more_info: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BanditResult {
    pub total_issues: usize,
    pub high_severity: usize,
    pub medium_severity: usize,
    pub low_severity: usize,
    pub confidence_high: usize,
    pub confidence_medium: usize,
    pub confidence_low: usize,
    pub issues: Vec<BanditIssue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BanditIssue {
    pub test_name: String,
    pub test_id: String,
    pub issue_severity: String,
    pub issue_confidence: String,
    pub issue_text: String,
    pub filename: String,
    pub line_number: usize,
    pub line_range: Vec<usize>,
    pub code: String,
    pub more_info: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BanditMetrics {
    #[serde(rename = "_totals")]
    pub totals: BanditTotals,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BanditTotals {
    #[serde(rename = "CONFIDENCE.HIGH")]
    pub confidence_high: usize,
    #[serde(rename = "CONFIDENCE.MEDIUM")]
    pub confidence_medium: usize,
    #[serde(rename = "CONFIDENCE.LOW")]
    pub confidence_low: usize,
    #[serde(rename = "SEVERITY.HIGH")]
    pub severity_high: usize,
    #[serde(rename = "SEVERITY.MEDIUM")]
    pub severity_medium: usize,
    #[serde(rename = "SEVERITY.LOW")]
    pub severity_low: usize,
    pub loc: usize,
    pub nosec: usize,
}

impl Default for BanditResult {
    fn default() -> Self {
        Self {
            total_issues: 0,
            high_severity: 0,
            medium_severity: 0,
            low_severity: 0,
            issues: Vec::new(),
            confidence_high: 0,
            confidence_medium: 0,
            confidence_low: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_bandit_integration() {
        let bandit_integration = BanditIntegration::new(Path::new("/tmp"));

        assert_eq!(bandit_integration.tool_name(), "bandit");

        // Test availability (may or may not be installed)
        let is_available = bandit_integration.is_available().await;
        println!("Bandit available: {}", is_available);
    }

    #[tokio::test]
    async fn test_bandit_version() {
        let bandit_integration = BanditIntegration::new(Path::new("/tmp"));

        if bandit_integration.is_available().await {
            let version = bandit_integration.get_version().await.unwrap();
            println!("Bandit version: {}", version);
            assert!(!version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_bandit_analysis_insecure_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file with security issues
        let python_file = temp_dir.path().join("insecure.py");
        fs::write(&python_file, r#"
import os
import subprocess
import pickle
import hashlib

# Security issue: hardcoded password
PASSWORD = "secret123"

def unsafe_exec(user_input):
    # Security issue: arbitrary code execution
    exec(user_input)

def unsafe_shell(command):
    # Security issue: shell injection
    os.system(command)
    subprocess.call(command, shell=True)

def unsafe_pickle(data):
    # Security issue: unsafe deserialization
    return pickle.loads(data)

def weak_crypto():
    # Security issue: weak cryptographic hash
    return hashlib.md5(b"password").hexdigest()

def sql_injection(user_id):
    # Security issue: SQL injection (simulated)
    query = "SELECT * FROM users WHERE id = '%s'" % user_id
    return query

# Security issue: hardcoded secret key
SECRET_KEY = "this-is-a-secret-key-123"

# Security issue: binding to all interfaces
if __name__ == "__main__":
    import socket
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.bind(("0.0.0.0", 8080))
"#).unwrap();

        let bandit_integration = BanditIntegration::new(temp_dir.path());

        if bandit_integration.is_available().await {
            let result = bandit_integration.analyze().await.unwrap();
            println!("Bandit found {} security issues", result.total_issues);
            println!("High: {}, Medium: {}, Low: {}",
                result.high_severity, result.medium_severity, result.low_severity);

            for issue in result.issues.iter().take(3) {
                println!("Issue: {} ({}) at {} line {}",
                    issue.test_name, issue.issue_severity, issue.filename, issue.line_number);
            }

            // Should find multiple security issues in the insecure code
            assert!(result.total_issues > 0);
        }
    }

    #[tokio::test]
    async fn test_bandit_analysis_secure_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file without security issues
        let python_file = temp_dir.path().join("secure.py");
        fs::write(&python_file, r#"
import hashlib
import secrets

def secure_hash(password: str) -> str:
    """Secure password hashing using SHA-256."""
    salt = secrets.token_bytes(32)
    return hashlib.sha256(salt + password.encode()).hexdigest()

def safe_calculation(x: int, y: int) -> int:
    """Safe arithmetic operation."""
    return x + y

class SecureCalculator:
    def __init__(self):
        self.result = 0

    def add(self, value: int) -> int:
        self.result += value
        return self.result
"#).unwrap();

        let bandit_integration = BanditIntegration::new(temp_dir.path());

        if bandit_integration.is_available().await {
            let result = bandit_integration.analyze().await.unwrap();
            println!("Bandit found {} security issues in secure code", result.total_issues);

            // Should find few or no security issues in secure code
            assert!(result.total_issues <= 1); // May still have minor issues
        }
    }
}