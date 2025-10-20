//! Pylint Code Quality Analyzer Integration
//!
//! Zero-degradation integration with Pylint matching Python static_analyzer.py behavior

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// Pylint integration
pub struct PylintIntegration {
    codebase_path: PathBuf,
}

impl PylintIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }

    /// Analyze a single file with pylint - CRITICAL for zero-degradation compliance
    pub async fn analyze_file(&self, file_path: &Path) -> Result<PylintResult> {
        debug!("Running pylint analysis on file: {}", file_path.display());

        let output = Command::new("pylint")
            .arg("--output-format=json")
            .arg("--disable=C0114,C0115,C0116") // Disable some docstring warnings for cleaner output
            .arg("--max-line-length=120")
            .arg(file_path)
            .output()
            .await
            .context("Failed to execute pylint on file")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output - pylint returns array of messages
        if stdout.trim().is_empty() || stdout.trim() == "[]" {
            // No issues found
            return Ok(PylintResult {
                messages: Vec::new(),
                total_messages: 0,
                error_count: 0,
                warning_count: 0,
                refactor_count: 0,
                convention_count: 0,
            });
        }

        // Parse pylint JSON output format
        let pylint_messages: Vec<PylintMessage> = serde_json::from_str(&stdout)
            .context("Failed to parse pylint JSON output")?;

        let mut error_count = 0;
        let mut warning_count = 0;
        let mut refactor_count = 0;
        let mut convention_count = 0;

        // Categorize messages by type
        for message in &pylint_messages {
            match message.msg_type.as_str() {
                "error" => error_count += 1,
                "warning" => warning_count += 1,
                "refactor" => refactor_count += 1,
                "convention" => convention_count += 1,
                _ => convention_count += 1, // Default to convention
            }
        }

        Ok(PylintResult {
            total_messages: pylint_messages.len(),
            error_count,
            warning_count,
            refactor_count,
            convention_count,
            messages: pylint_messages,
        })
    }
}

#[async_trait::async_trait]
impl ExternalTool for PylintIntegration {
    type Result = PylintResult;

    async fn is_available(&self) -> bool {
        Command::new("pylint")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running pylint analysis on {}", self.codebase_path.display());

        let output = Command::new("pylint")
            .arg("--output-format=json")
            .arg("--recursive=y")
            .arg("--disable=C0114,C0115,C0116") // Disable some docstring warnings for cleaner output
            .arg("--max-line-length=120")
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute pylint")?;

        // Pylint can have various exit codes, we care about the output regardless
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            // No issues found or no Python files
            return Ok(PylintResult::default());
        }

        // Parse pylint JSON output
        let pylint_messages: Vec<PylintMessage> = match serde_json::from_str(&stdout) {
            Ok(messages) => messages,
            Err(e) => {
                warn!("Failed to parse pylint JSON output: {}", e);
                warn!("Raw output: {}", stdout);
                return Ok(PylintResult::default());
            }
        };

        let mut error_count = 0;
        let mut warning_count = 0;
        let mut convention_count = 0;
        let mut refactor_count = 0;

        // Categorize messages by type
        for message in &pylint_messages {
            match message.msg_type.chars().next() {
                Some('E') => error_count += 1,        // Error
                Some('W') => warning_count += 1,      // Warning
                Some('C') => convention_count += 1,   // Convention
                Some('R') => refactor_count += 1,     // Refactor
                _ => warning_count += 1,              // Default to warning
            }
        }

        Ok(PylintResult {
            total_messages: pylint_messages.len(),
            error_count,
            warning_count,
            convention_count,
            refactor_count,
            messages: pylint_messages,
        })
    }

    fn tool_name(&self) -> &'static str {
        "pylint"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("pylint")
            .arg("--version")
            .output()
            .await
            .context("Failed to get pylint version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get pylint version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        // Extract version line from pylint output
        let version_line = version.lines()
            .find(|line| line.contains("pylint"))
            .unwrap_or(version.lines().next().unwrap_or("Unknown"))
            .trim();

        Ok(version_line.to_string())
    }
}

/// Pylint analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct PylintResult {
    pub total_messages: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub convention_count: usize,
    pub refactor_count: usize,
    pub messages: Vec<PylintMessage>,
}

/// Individual pylint message
#[derive(Debug, Serialize, Deserialize)]
pub struct PylintMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub module: String,
    pub obj: String,
    pub line: usize,
    pub column: usize,
    #[serde(rename = "endLine")]
    pub end_line: Option<usize>,
    #[serde(rename = "endColumn")]
    pub end_column: Option<usize>,
    pub path: String,
    pub symbol: String,
    pub message: String,
    #[serde(rename = "message-id")]
    pub message_id: String,
}

impl Default for PylintResult {
    fn default() -> Self {
        Self {
            total_messages: 0,
            error_count: 0,
            warning_count: 0,
            convention_count: 0,
            refactor_count: 0,
            messages: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_pylint_integration() {
        let pylint_integration = PylintIntegration::new(Path::new("/tmp"));

        assert_eq!(pylint_integration.tool_name(), "pylint");

        // Test availability (may or may not be installed)
        let is_available = pylint_integration.is_available().await;
        println!("Pylint available: {}", is_available);
    }

    #[tokio::test]
    async fn test_pylint_version() {
        let pylint_integration = PylintIntegration::new(Path::new("/tmp"));

        if pylint_integration.is_available().await {
            let version = pylint_integration.get_version().await.unwrap();
            println!("Pylint version: {}", version);
            assert!(!version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_pylint_analysis_python_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file with various pylint issues
        let python_file = temp_dir.path().join("test.py");
        fs::write(&python_file, r#"
import os
import sys

def badFunction():  # Bad naming convention
    x = 1
    y = x + 2
    if x > 0:
        if y > 0:  # Too nested
            print("nested")
    return y

class badClass:  # Bad naming convention
    def __init__(self, value):
        self.value = value

    def getValue(self):  # Bad naming convention
        return self.value
"#).unwrap();

        let pylint_integration = PylintIntegration::new(temp_dir.path());

        if pylint_integration.is_available().await {
            let result = pylint_integration.analyze().await.unwrap();
            println!("Pylint found {} messages", result.total_messages);

            for message in result.messages.iter().take(5) {
                println!("Message: {} - {} at {}:{}",
                    message.message_id, message.message, message.line, message.column);
            }

            // Should find some convention issues at minimum
            assert!(result.total_messages > 0);
        }
    }
}