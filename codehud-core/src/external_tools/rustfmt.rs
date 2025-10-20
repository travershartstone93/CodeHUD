//! Rustfmt Integration - Rust code formatter
//!
//! Provides integration with rustfmt for Rust code formatting analysis

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// Rustfmt integration for Rust code formatting analysis
pub struct RustfmtIntegration {
    codebase_path: PathBuf,
}

impl RustfmtIntegration {
    pub fn new(codebase_path: impl AsRef<Path>) -> Self {
        Self {
            codebase_path: codebase_path.as_ref().to_path_buf(),
        }
    }

    /// Check if Cargo.toml exists to determine if this is a Rust project
    fn is_rust_project(&self) -> bool {
        self.codebase_path.join("Cargo.toml").exists()
    }

    /// Get all Rust files in the project
    fn get_rust_files(&self) -> Vec<PathBuf> {
        let mut rust_files = Vec::new();
        self.collect_rust_files(&self.codebase_path, &mut rust_files);
        rust_files
    }

    fn collect_rust_files(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                    files.push(path);
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    self.collect_rust_files(&path, files);
                }
            }
        }
    }

    fn is_excluded_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "target" | ".git" | "node_modules")
        } else {
            false
        }
    }
}

#[async_trait::async_trait]
impl super::ExternalTool for RustfmtIntegration {
    type Result = RustfmtResult;

    async fn is_available(&self) -> bool {
        if !self.is_rust_project() {
            return false;
        }

        Command::new("rustfmt")
            .args(&["--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running rustfmt analysis on {}", self.codebase_path.display());

        let rust_files = self.get_rust_files();

        if rust_files.is_empty() {
            return Ok(RustfmtResult {
                total_files: 0,
                files_needing_formatting: 0,
                properly_formatted_files: 0,
                formatting_issues: Vec::new(),
                analysis_timestamp: chrono::Utc::now().to_rfc3339(),
                scan_successful: true,
                error_message: None,
            });
        }

        let mut files_needing_formatting = 0;
        let mut formatting_issues = Vec::new();

        // Check each Rust file for formatting issues
        for file_path in &rust_files {
            match Command::new("rustfmt")
                .args(&[
                    "--check",
                    file_path.to_str().unwrap()
                ])
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        files_needing_formatting += 1;

                        // Get the diff to show what needs to be formatted
                        let diff_output = Command::new("rustfmt")
                            .args(&[
                                "--emit=stdout",
                                file_path.to_str().unwrap()
                            ])
                            .output();

                        let diff_content = if let Ok(diff) = diff_output {
                            Some(String::from_utf8_lossy(&diff.stdout).to_string())
                        } else {
                            None
                        };

                        formatting_issues.push(RustfmtIssue {
                            file_path: file_path.display().to_string(),
                            issue_type: "formatting".to_string(),
                            description: "File needs formatting".to_string(),
                            suggested_fix: diff_content,
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to check formatting for {}: {}", file_path.display(), e);
                }
            }
        }

        let properly_formatted_files = rust_files.len() - files_needing_formatting;

        debug!("Rustfmt analysis: {}/{} files properly formatted",
               properly_formatted_files, rust_files.len());

        Ok(RustfmtResult {
            total_files: rust_files.len(),
            files_needing_formatting,
            properly_formatted_files,
            formatting_issues,
            analysis_timestamp: chrono::Utc::now().to_rfc3339(),
            scan_successful: true,
            error_message: None,
        })
    }

    fn tool_name(&self) -> &'static str {
        "rustfmt"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("rustfmt")
            .args(&["--version"])
            .output()
            .context("Failed to get rustfmt version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get rustfmt version"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Rustfmt analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct RustfmtResult {
    pub total_files: usize,
    pub files_needing_formatting: usize,
    pub properly_formatted_files: usize,
    pub formatting_issues: Vec<RustfmtIssue>,
    pub analysis_timestamp: String,
    pub scan_successful: bool,
    pub error_message: Option<String>,
}

/// Individual formatting issue found by rustfmt
#[derive(Debug, Serialize, Deserialize)]
pub struct RustfmtIssue {
    pub file_path: String,
    pub issue_type: String,
    pub description: String,
    pub suggested_fix: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_rustfmt_integration() {
        let rustfmt_integration = RustfmtIntegration::new(Path::new("/tmp"));
        assert_eq!(rustfmt_integration.tool_name(), "rustfmt");
    }

    #[tokio::test]
    async fn test_rustfmt_rust_project_detection() {
        let temp_dir = tempdir().unwrap();

        // Create a Cargo.toml to simulate a Rust project
        fs::write(temp_dir.path().join("Cargo.toml"),
                 "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let rustfmt_integration = RustfmtIntegration::new(temp_dir.path());
        assert!(rustfmt_integration.is_rust_project());
    }

    #[tokio::test]
    async fn test_rustfmt_availability() {
        let temp_dir = tempdir().unwrap();

        // Create a Rust project with poorly formatted code
        fs::write(temp_dir.path().join("Cargo.toml"),
                 "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "
fn   main(   )   {
    println!(  \"Hello, world!\"  );
}").unwrap();

        let rustfmt_integration = RustfmtIntegration::new(temp_dir.path());

        let is_available = rustfmt_integration.is_available().await;
        println!("Rustfmt available: {}", is_available);

        if is_available {
            let result = rustfmt_integration.analyze().await.unwrap();
            println!("Formatting results: {:?}", result);
        }
    }

    #[test]
    fn test_rust_file_collection() {
        let temp_dir = tempdir().unwrap();

        // Create some Rust files
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src_dir.join("lib.rs"), "pub fn test() {}").unwrap();

        let rustfmt_integration = RustfmtIntegration::new(temp_dir.path());
        let rust_files = rustfmt_integration.get_rust_files();

        assert_eq!(rust_files.len(), 2);
    }
}