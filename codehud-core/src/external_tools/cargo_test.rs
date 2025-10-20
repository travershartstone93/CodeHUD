//! Cargo Test Integration - Rust test runner equivalent to pytest
//!
//! Provides integration with cargo test for Rust test execution and analysis

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// Cargo test integration for Rust test analysis
pub struct CargoTestIntegration {
    codebase_path: PathBuf,
}

impl CargoTestIntegration {
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
impl super::ExternalTool for CargoTestIntegration {
    type Result = CargoTestResult;

    async fn is_available(&self) -> bool {
        if !self.is_rust_project() {
            return false;
        }

        Command::new("cargo")
            .args(&["--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running cargo test analysis on {}", self.codebase_path.display());

        // First, run tests in quiet mode to get basic results
        let test_output = Command::new("cargo")
            .args(&[
                "test",
                "--message-format=json",
                "--",
                "--format=json",
                "-Z", "unstable-options"
            ])
            .current_dir(&self.codebase_path)
            .output()
            .context("Failed to execute cargo test")?;

        let test_stdout = String::from_utf8_lossy(&test_output.stdout);
        let test_stderr = String::from_utf8_lossy(&test_output.stderr);

        // Parse test results
        let mut passed_tests = 0;
        let mut failed_tests = 0;
        let mut ignored_tests = 0;
        let mut test_cases = Vec::new();

        // Simple parsing - cargo test output format can vary
        for line in test_stdout.lines() {
            if line.contains("test result:") {
                // Extract summary line: "test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if part == &"passed;" && i > 0 {
                        if let Ok(count) = parts[i-1].parse::<usize>() {
                            passed_tests = count;
                        }
                    } else if part == &"failed;" && i > 0 {
                        if let Ok(count) = parts[i-1].parse::<usize>() {
                            failed_tests = count;
                        }
                    } else if part == &"ignored;" && i > 0 {
                        if let Ok(count) = parts[i-1].parse::<usize>() {
                            ignored_tests = count;
                        }
                    }
                }
            } else if line.starts_with("test ") {
                // Individual test result lines
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let test_name = parts[1];
                    let result = parts[3];
                    test_cases.push(CargoTestCase {
                        name: test_name.to_string(),
                        status: result.to_string(),
                        duration: None, // Would need more parsing to extract
                        failure_message: None,
                    });
                }
            }
        }

        // Check for compilation errors that prevent tests from running
        let compilation_successful = test_output.status.success() ||
                                   passed_tests > 0 || failed_tests > 0;

        // Calculate test coverage (simplified - would need actual coverage tools)
        let total_tests = passed_tests + failed_tests + ignored_tests;
        let pass_rate = if total_tests > 0 {
            (passed_tests as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        debug!("Cargo test found {} total tests ({} passed, {} failed, {} ignored)",
               total_tests, passed_tests, failed_tests, ignored_tests);

        Ok(CargoTestResult {
            total_tests,
            passed_tests,
            failed_tests,
            ignored_tests,
            pass_rate,
            compilation_successful,
            test_cases,
            analysis_timestamp: chrono::Utc::now().to_rfc3339(),
            error_message: if !compilation_successful {
                Some(test_stderr.to_string())
            } else {
                None
            },
        })
    }

    fn tool_name(&self) -> &'static str {
        "cargo test"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("cargo")
            .args(&["--version"])
            .output()
            .context("Failed to get cargo version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get cargo version"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Cargo test analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoTestResult {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub ignored_tests: usize,
    pub pass_rate: f64,
    pub compilation_successful: bool,
    pub test_cases: Vec<CargoTestCase>,
    pub analysis_timestamp: String,
    pub error_message: Option<String>,
}

/// Individual test case result
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoTestCase {
    pub name: String,
    pub status: String, // "ok", "FAILED", "ignored"
    pub duration: Option<String>,
    pub failure_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_cargo_test_integration() {
        let test_integration = CargoTestIntegration::new(Path::new("/tmp"));
        assert_eq!(test_integration.tool_name(), "cargo test");
    }

    #[tokio::test]
    async fn test_cargo_test_rust_project_detection() {
        let temp_dir = tempdir().unwrap();

        // Create a Cargo.toml to simulate a Rust project
        fs::write(temp_dir.path().join("Cargo.toml"),
                 "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let test_integration = CargoTestIntegration::new(temp_dir.path());
        assert!(test_integration.is_rust_project());
    }

    #[tokio::test]
    async fn test_cargo_test_availability() {
        let temp_dir = tempdir().unwrap();

        // Create a minimal Rust project
        fs::write(temp_dir.path().join("Cargo.toml"),
                 "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}").unwrap();

        let test_integration = CargoTestIntegration::new(temp_dir.path());

        let is_available = test_integration.is_available().await;
        println!("Cargo test available: {}", is_available);

        if is_available {
            let result = test_integration.analyze().await.unwrap();
            println!("Test results: {:?}", result);
        }
    }
}