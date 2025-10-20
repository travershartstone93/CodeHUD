//! Clippy Integration - Rust linter equivalent to ruff/pylint
//!
//! Provides integration with cargo clippy for Rust code analysis

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// Clippy integration for Rust code analysis
pub struct ClippyIntegration {
    codebase_path: PathBuf,
}

impl ClippyIntegration {
    pub fn new(codebase_path: impl AsRef<Path>) -> Self {
        Self {
            codebase_path: codebase_path.as_ref().to_path_buf(),
        }
    }

    /// Check if Cargo.toml exists to determine if this is a Rust project
    fn is_rust_project(&self) -> bool {
        self.codebase_path.join("Cargo.toml").exists()
    }
}

#[async_trait::async_trait]
impl super::ExternalTool for ClippyIntegration {
    type Result = ClippyResult;

    async fn is_available(&self) -> bool {
        if !self.is_rust_project() {
            return false;
        }

        Command::new("cargo")
            .args(&["clippy", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running clippy analysis on {}", self.codebase_path.display());

        let output = Command::new("cargo")
            .args(&[
                "clippy",
                "--message-format=json",
                "--all-targets",
                "--all-features"
            ])
            .current_dir(&self.codebase_path)
            .output()
            .context("Failed to execute cargo clippy")?;

        if !output.status.success() && output.status.code() != Some(101) {
            // Exit code 101 means clippy found issues, which is fine
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Clippy execution failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse clippy JSON output
        let mut issues = Vec::new();
        for line in stdout.lines() {
            if let Ok(diagnostic) = serde_json::from_str::<ClippyDiagnostic>(line) {
                if diagnostic.reason == "compiler-message" &&
                   diagnostic.message.level == "warning" {
                    issues.push(ClippyIssue {
                        code: diagnostic.message.code.as_ref()
                            .map(|c| c.code.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                        message: diagnostic.message.message,
                        file_path: diagnostic.message.spans.first()
                            .map(|s| s.file_name.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                        line: diagnostic.message.spans.first()
                            .map(|s| s.line_start)
                            .unwrap_or(0),
                        column: diagnostic.message.spans.first()
                            .map(|s| s.column_start)
                            .unwrap_or(0),
                        severity: "warning".to_string(),
                        suggestion: diagnostic.message.spans.first()
                            .and_then(|s| s.suggested_replacement.clone()),
                    });
                }
            }
        }

        // Count issues by severity
        let warning_count = issues.len();
        let error_count = 0; // Clippy mainly produces warnings

        debug!("Clippy analysis found {} warnings", warning_count);

        Ok(ClippyResult {
            total_issues: issues.len(),
            warning_count,
            error_count,
            issues,
            analysis_timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn tool_name(&self) -> &'static str {
        "cargo clippy"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("cargo")
            .args(&["clippy", "--version"])
            .output()
            .context("Failed to get clippy version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get clippy version"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Clippy analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct ClippyResult {
    pub total_issues: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub analysis_timestamp: String,
    pub issues: Vec<ClippyIssue>,
}

/// Individual clippy issue/warning
#[derive(Debug, Serialize, Deserialize)]
pub struct ClippyIssue {
    pub code: String,
    pub message: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub suggestion: Option<String>,
}

/// Clippy diagnostic message format
#[derive(Debug, Deserialize)]
struct ClippyDiagnostic {
    reason: String,
    message: ClippyMessage,
}

#[derive(Debug, Deserialize)]
struct ClippyMessage {
    message: String,
    level: String,
    code: Option<ClippyCode>,
    spans: Vec<ClippySpan>,
}

#[derive(Debug, Deserialize)]
struct ClippyCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct ClippySpan {
    file_name: String,
    line_start: u32,
    column_start: u32,
    suggested_replacement: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_clippy_integration() {
        let clippy_integration = ClippyIntegration::new(Path::new("/tmp"));
        assert_eq!(clippy_integration.tool_name(), "cargo clippy");
    }

    #[tokio::test]
    async fn test_clippy_rust_project_detection() {
        let temp_dir = tempdir().unwrap();

        // Create a Cargo.toml to simulate a Rust project
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let clippy_integration = ClippyIntegration::new(temp_dir.path());
        assert!(clippy_integration.is_rust_project());
    }

    #[tokio::test]
    async fn test_clippy_availability() {
        let temp_dir = tempdir().unwrap();

        // Create a Cargo.toml
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let clippy_integration = ClippyIntegration::new(temp_dir.path());

        // This will depend on whether clippy is installed in the test environment
        let is_available = clippy_integration.is_available().await;
        println!("Clippy available: {}", is_available);
    }
}