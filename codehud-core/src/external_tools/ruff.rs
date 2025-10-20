//! Ruff Python Linter Integration
//!
//! Zero-degradation integration with Ruff linter matching Python static_analyzer.py behavior

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// Ruff linter integration
pub struct RuffIntegration {
    codebase_path: PathBuf,
}

impl RuffIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }

    /// Analyze a single file with ruff - CRITICAL for zero-degradation compliance
    pub async fn analyze_file(&self, file_path: &Path) -> Result<RuffResult> {
        debug!("Running ruff analysis on file: {}", file_path.display());

        let output = Command::new("ruff")
            .arg("check")
            .arg("--output-format=json")
            .arg("--no-fix")
            .arg("--target-version=py38")
            .arg(file_path)
            .output()
            .await
            .context("Failed to execute ruff on file")?;

        if !output.status.success() && output.status.code() != Some(1) {
            // Exit code 1 is expected when issues are found
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Ruff execution failed on {}: {}", file_path.display(), stderr);
            return Err(anyhow::anyhow!("Ruff failed on {}: {}", file_path.display(), stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        if stdout.trim().is_empty() {
            // No issues found
            return Ok(RuffResult {
                issues: Vec::new(),
                total_issues: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                fixed_count: 0,
            });
        }

        // Parse ruff JSON output format
        let ruff_issues: Vec<RuffIssue> = serde_json::from_str(&stdout)
            .context("Failed to parse ruff JSON output")?;

        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        // Categorize issues by severity
        for issue in &ruff_issues {
            match issue.severity.as_str() {
                "error" => error_count += 1,
                "warning" => warning_count += 1,
                "info" => info_count += 1,
                _ => info_count += 1, // Default to info
            }
        }

        Ok(RuffResult {
            total_issues: ruff_issues.len(),
            error_count,
            warning_count,
            info_count,
            fixed_count: 0, // We don't run with --fix
            issues: ruff_issues,
        })
    }
}

#[async_trait::async_trait]
impl ExternalTool for RuffIntegration {
    type Result = RuffResult;

    async fn is_available(&self) -> bool {
        Command::new("ruff")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running ruff analysis on {}", self.codebase_path.display());

        let output = Command::new("ruff")
            .arg("check")
            .arg("--output-format=json")
            .arg("--no-fix")
            .arg("--target-version=py38")
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute ruff")?;

        if !output.status.success() && output.status.code() != Some(1) {
            // Exit code 1 is expected when issues are found
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Ruff execution failed: {}", stderr);
            return Err(anyhow::anyhow!("Ruff failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        if stdout.trim().is_empty() {
            // No issues found
            return Ok(RuffResult {
                issues: Vec::new(),
                total_issues: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                fixed_count: 0,
            });
        }

        // Parse ruff JSON output format
        let ruff_issues: Vec<RuffIssue> = serde_json::from_str(&stdout)
            .context("Failed to parse ruff JSON output")?;

        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        // Categorize issues by severity
        for issue in &ruff_issues {
            match issue.severity.as_str() {
                "error" => error_count += 1,
                "warning" => warning_count += 1,
                "info" => info_count += 1,
                _ => info_count += 1, // Default to info
            }
        }

        Ok(RuffResult {
            total_issues: ruff_issues.len(),
            error_count,
            warning_count,
            info_count,
            fixed_count: 0, // We don't run with --fix
            issues: ruff_issues,
        })
    }

    fn tool_name(&self) -> &'static str {
        "ruff"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("ruff")
            .arg("--version")
            .output()
            .await
            .context("Failed to get ruff version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get ruff version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

/// Ruff analysis result matching Python static_analyzer.py format
#[derive(Debug, Serialize, Deserialize)]
pub struct RuffResult {
    pub total_issues: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub fixed_count: usize,
    pub issues: Vec<RuffIssue>,
}

/// Individual ruff issue/violation
#[derive(Debug, Serialize, Deserialize)]
pub struct RuffIssue {
    pub code: String,
    pub message: String,
    pub filename: String,
    pub location: RuffLocation,
    pub end_location: RuffLocation,
    pub severity: String,
    pub rule: String,
    pub url: Option<String>,
}

/// Location information for ruff issues
#[derive(Debug, Serialize, Deserialize)]
pub struct RuffLocation {
    pub row: usize,
    pub column: usize,
}

impl Default for RuffResult {
    fn default() -> Self {
        Self {
            total_issues: 0,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            fixed_count: 0,
            issues: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_ruff_integration() {
        let ruff_integration = RuffIntegration::new(Path::new("/tmp"));

        // Test basic functionality
        assert_eq!(ruff_integration.tool_name(), "ruff");

        // Test availability check (may or may not be installed)
        let is_available = ruff_integration.is_available().await;
        println!("Ruff available: {}", is_available);
    }

    #[tokio::test]
    async fn test_ruff_version() {
        let ruff_integration = RuffIntegration::new(Path::new("/tmp"));

        if ruff_integration.is_available().await {
            let version = ruff_integration.get_version().await.unwrap();
            println!("Ruff version: {}", version);
            assert!(!version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_ruff_analysis_empty_dir() {
        let temp_dir = tempdir().unwrap();
        let ruff_integration = RuffIntegration::new(temp_dir.path());

        if ruff_integration.is_available().await {
            // Should succeed even with empty directory
            let result = ruff_integration.analyze().await.unwrap();
            assert_eq!(result.total_issues, 0);
        }
    }

    #[tokio::test]
    async fn test_ruff_analysis_python_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file with linting issues
        let python_file = temp_dir.path().join("test.py");
        fs::write(&python_file, r#"
import os
import sys
import unused_module

def bad_function():
    x=1+2
    y = x*3
    return y

if True:
    print("This is bad")
"#).unwrap();

        let ruff_integration = RuffIntegration::new(temp_dir.path());

        if ruff_integration.is_available().await {
            let result = ruff_integration.analyze().await.unwrap();
            println!("Ruff found {} issues", result.total_issues);

            for issue in &result.issues {
                println!("Issue: {} - {} at {}:{}",
                    issue.code, issue.message, issue.location.row, issue.location.column);
            }

            // Should find some issues in the poorly written Python code
            assert!(result.total_issues > 0);
        }
    }
}