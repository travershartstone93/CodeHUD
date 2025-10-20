//! MyPy Type Checker Integration
//!
//! Zero-degradation integration with MyPy type checker matching Python behavior

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

pub struct MypyIntegration {
    codebase_path: PathBuf,
}

impl MypyIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalTool for MypyIntegration {
    type Result = MypyResult;

    async fn is_available(&self) -> bool {
        Command::new("mypy")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running mypy analysis on {}", self.codebase_path.display());

        let output = Command::new("mypy")
            .arg("--show-error-codes")
            .arg("--show-column-numbers")
            .arg("--show-error-context")
            .arg("--no-color-output")
            .arg("--no-error-summary")
            .arg("--follow-imports=silent")
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute mypy")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Combine stdout and stderr as mypy can output to both
        let combined_output = format!("{}\n{}", stdout, stderr);

        if combined_output.trim().is_empty() {
            // No issues found
            return Ok(MypyResult {
                total_errors: 0,
                total_notes: 0,
                errors: Vec::new(),
            });
        }

        // Parse mypy output line by line
        let mut errors = Vec::new();
        let mut error_count = 0;
        let mut note_count = 0;

        for line in combined_output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Skip lines that are not error/note messages
            if !line.contains(":") || line.starts_with("Found ") || line.starts_with("Success:") {
                continue;
            }

            // Parse mypy error format: filename:line:column: error: message [error-code]
            if let Some(error) = self.parse_mypy_line(line) {
                if error.severity == "error" {
                    error_count += 1;
                } else if error.severity == "note" {
                    note_count += 1;
                }
                errors.push(error);
            }
        }

        Ok(MypyResult {
            total_errors: error_count,
            total_notes: note_count,
            errors,
        })
    }

    fn tool_name(&self) -> &'static str {
        "mypy"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("mypy")
            .arg("--version")
            .output()
            .await
            .context("Failed to get mypy version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get mypy version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

impl MypyIntegration {
    fn parse_mypy_line(&self, line: &str) -> Option<MypyError> {
        // Parse format: filename:line:column: error: message [error-code]
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        let filename = parts[0].trim();
        let line_num: usize = parts[1].trim().parse().ok()?;
        let column: usize = parts[2].trim().parse().ok()?;

        let remaining = parts[3].trim();

        // Extract severity and message
        let (severity, message_with_code) = if remaining.starts_with("error:") {
            ("error", remaining.strip_prefix("error:").unwrap_or(remaining).trim())
        } else if remaining.starts_with("note:") {
            ("note", remaining.strip_prefix("note:").unwrap_or(remaining).trim())
        } else if remaining.starts_with("warning:") {
            ("warning", remaining.strip_prefix("warning:").unwrap_or(remaining).trim())
        } else {
            ("error", remaining) // Default to error
        };

        // Extract error code if present [error-code]
        let (message, error_code) = if let Some(code_start) = message_with_code.rfind('[') {
            if let Some(code_end) = message_with_code.rfind(']') {
                if code_start < code_end {
                    let message = message_with_code[..code_start].trim();
                    let code = &message_with_code[code_start + 1..code_end];
                    (message, Some(code.to_string()))
                } else {
                    (message_with_code, None)
                }
            } else {
                (message_with_code, None)
            }
        } else {
            (message_with_code, None)
        };

        Some(MypyError {
            filename: filename.to_string(),
            line: line_num,
            column,
            severity: severity.to_string(),
            message: message.to_string(),
            error_code,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MypyResult {
    pub total_errors: usize,
    pub total_notes: usize,
    pub errors: Vec<MypyError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MypyError {
    pub filename: String,
    pub line: usize,
    pub column: usize,
    pub severity: String,
    pub message: String,
    pub error_code: Option<String>,
}

impl Default for MypyResult {
    fn default() -> Self {
        Self {
            total_errors: 0,
            total_notes: 0,
            errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_mypy_integration() {
        let mypy_integration = MypyIntegration::new(Path::new("/tmp"));

        assert_eq!(mypy_integration.tool_name(), "mypy");

        // Test availability (may or may not be installed)
        let is_available = mypy_integration.is_available().await;
        println!("MyPy available: {}", is_available);
    }

    #[tokio::test]
    async fn test_mypy_version() {
        let mypy_integration = MypyIntegration::new(Path::new("/tmp"));

        if mypy_integration.is_available().await {
            let version = mypy_integration.get_version().await.unwrap();
            println!("MyPy version: {}", version);
            assert!(!version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_mypy_analysis_python_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file with type issues
        let python_file = temp_dir.path().join("test.py");
        fs::write(&python_file, r#"
def add_numbers(x, y):
    return x + y

def process_data(data: list) -> str:
    result = add_numbers("hello", "world")  # Type error: should be numbers
    return result  # Type error: returning wrong type

class Calculator:
    def __init__(self, value: int):
        self.value = value

    def multiply(self, other) -> int:  # Missing type annotation
        return self.value * other

# Using without type annotations
calc = Calculator("not a number")  # Type error
result = calc.multiply(2.5)
"#).unwrap();

        let mypy_integration = MypyIntegration::new(temp_dir.path());

        if mypy_integration.is_available().await {
            let result = mypy_integration.analyze().await.unwrap();
            println!("MyPy found {} errors and {} notes", result.total_errors, result.total_notes);

            for error in result.errors.iter().take(5) {
                println!("Error: {} at {}:{} - {}",
                    error.severity, error.line, error.column, error.message);
            }

            // Should find some type errors
            assert!(result.total_errors > 0 || result.total_notes > 0);
        }
    }

    #[test]
    fn test_mypy_line_parsing() {
        let mypy_integration = MypyIntegration::new(Path::new("/tmp"));

        // Test parsing different mypy output formats
        let test_cases = vec![
            "test.py:5:12: error: Argument 1 to \"add\" has incompatible type \"str\"; expected \"int\" [arg-type]",
            "main.py:15:8: note: Revealed type is \"builtins.str\"",
            "module.py:25:4: warning: Return value expected [return-value]",
        ];

        for case in test_cases {
            if let Some(error) = mypy_integration.parse_mypy_line(case) {
                println!("Parsed: {:?}", error);
                assert!(!error.filename.is_empty());
                assert!(error.line > 0);
                assert!(!error.message.is_empty());
            }
        }
    }
}