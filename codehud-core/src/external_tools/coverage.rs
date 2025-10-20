//! Coverage.py Test Coverage Integration
//!
//! Zero-degradation integration with Coverage.py for test coverage analysis

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

pub struct CoverageIntegration {
    codebase_path: PathBuf,
}

impl CoverageIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalTool for CoverageIntegration {
    type Result = CoverageResult;

    async fn is_available(&self) -> bool {
        Command::new("coverage")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running coverage analysis on {}", self.codebase_path.display());

        // Try to get existing coverage data first
        let report_result = self.get_coverage_report().await;

        match report_result {
            Ok(result) => Ok(result),
            Err(_) => {
                // If no existing coverage data, try to run tests with coverage
                debug!("No existing coverage data found, attempting to run tests with coverage");
                self.run_tests_with_coverage().await
            }
        }
    }

    fn tool_name(&self) -> &'static str {
        "coverage"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("coverage")
            .arg("--version")
            .output()
            .await
            .context("Failed to get coverage version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get coverage version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

impl CoverageIntegration {
    async fn get_coverage_report(&self) -> Result<CoverageResult> {
        // Get JSON coverage report
        let output = Command::new("coverage")
            .arg("json")
            .arg("--pretty-print")
            .current_dir(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute coverage json")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Coverage report generation failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(CoverageResult::default());
        }

        // Parse coverage JSON report
        let coverage_data: CoverageJsonReport = serde_json::from_str(&stdout)
            .context("Failed to parse coverage JSON report")?;

        self.process_coverage_data(coverage_data)
    }

    async fn run_tests_with_coverage(&self) -> Result<CoverageResult> {
        // Try to run tests with coverage (common patterns)
        let test_commands = vec![
            vec!["coverage", "run", "-m", "pytest"],
            vec!["coverage", "run", "-m", "unittest", "discover"],
            vec!["coverage", "run", "test.py"],
        ];

        for cmd_args in test_commands {
            if let Ok(_) = Command::new(cmd_args[0])
                .args(&cmd_args[1..])
                .current_dir(&self.codebase_path)
                .output()
                .await
            {
                // Test execution completed, try to get report
                if let Ok(result) = self.get_coverage_report().await {
                    return Ok(result);
                }
            }
        }

        // If all attempts fail, return empty result
        Ok(CoverageResult::default())
    }

    fn process_coverage_data(&self, data: CoverageJsonReport) -> Result<CoverageResult> {
        let mut file_coverage = Vec::new();
        let mut total_lines = 0;
        let mut covered_lines = 0;
        let mut total_branches = 0;
        let mut covered_branches = 0;

        for (file_path, file_data) in data.files {
            let lines_covered = file_data.summary.covered_lines;
            let lines_total = file_data.summary.num_statements;
            let branches_covered = file_data.summary.covered_branches.unwrap_or(0);
            let branches_total = file_data.summary.num_branches.unwrap_or(0);

            let line_coverage_percentage = if lines_total > 0 {
                (lines_covered as f64 / lines_total as f64) * 100.0
            } else {
                100.0
            };

            let branch_coverage_percentage = if branches_total > 0 {
                (branches_covered as f64 / branches_total as f64) * 100.0
            } else {
                100.0
            };

            file_coverage.push(FileCoverage {
                file_path: file_path.clone(),
                lines_covered,
                lines_total,
                line_coverage_percentage,
                branches_covered,
                branches_total,
                branch_coverage_percentage,
                missing_lines: file_data.missing_lines.unwrap_or_default(),
                excluded_lines: file_data.excluded_lines.unwrap_or_default(),
            });

            total_lines += lines_total;
            covered_lines += lines_covered;
            total_branches += branches_total;
            covered_branches += branches_covered;
        }

        let overall_line_percentage = if total_lines > 0 {
            (covered_lines as f64 / total_lines as f64) * 100.0
        } else {
            100.0
        };

        let overall_branch_percentage = if total_branches > 0 {
            (covered_branches as f64 / total_branches as f64) * 100.0
        } else {
            100.0
        };

        // Calculate coverage quality metrics
        let files_with_low_coverage = file_coverage.iter()
            .filter(|f| f.line_coverage_percentage < 70.0)
            .count();

        let files_with_no_coverage = file_coverage.iter()
            .filter(|f| f.line_coverage_percentage == 0.0)
            .count();

        Ok(CoverageResult {
            overall_line_percentage,
            overall_branch_percentage,
            total_lines,
            covered_lines,
            total_branches,
            covered_branches,
            file_coverage,
            files_with_low_coverage,
            files_with_no_coverage,
            timestamp: data.meta.timestamp,
        })
    }
}

// Coverage.py JSON report structures
#[derive(Debug, Deserialize)]
struct CoverageJsonReport {
    pub meta: CoverageMeta,
    pub files: std::collections::HashMap<String, CoverageFileData>,
}

#[derive(Debug, Deserialize)]
struct CoverageMeta {
    pub timestamp: String,
    pub version: String,
    pub branch_coverage: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CoverageFileData {
    pub summary: CoverageFileSummary,
    pub missing_lines: Option<Vec<usize>>,
    pub excluded_lines: Option<Vec<usize>>,
}

#[derive(Debug, Deserialize)]
struct CoverageFileSummary {
    pub covered_lines: usize,
    pub num_statements: usize,
    pub percent_covered: f64,
    pub covered_branches: Option<usize>,
    pub num_branches: Option<usize>,
    pub percent_covered_display: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoverageResult {
    pub overall_line_percentage: f64,
    pub overall_branch_percentage: f64,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub total_branches: usize,
    pub covered_branches: usize,
    pub file_coverage: Vec<FileCoverage>,
    pub files_with_low_coverage: usize,
    pub files_with_no_coverage: usize,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCoverage {
    pub file_path: String,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub line_coverage_percentage: f64,
    pub branches_covered: usize,
    pub branches_total: usize,
    pub branch_coverage_percentage: f64,
    pub missing_lines: Vec<usize>,
    pub excluded_lines: Vec<usize>,
}

impl Default for CoverageResult {
    fn default() -> Self {
        Self {
            overall_line_percentage: 0.0,
            overall_branch_percentage: 0.0,
            total_lines: 0,
            covered_lines: 0,
            total_branches: 0,
            covered_branches: 0,
            file_coverage: Vec::new(),
            files_with_low_coverage: 0,
            files_with_no_coverage: 0,
            timestamp: "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_coverage_integration() {
        let coverage_integration = CoverageIntegration::new(Path::new("/tmp"));

        assert_eq!(coverage_integration.tool_name(), "coverage");

        // Test availability (may or may not be installed)
        let is_available = coverage_integration.is_available().await;
        println!("Coverage available: {}", is_available);
    }

    #[tokio::test]
    async fn test_coverage_version() {
        let coverage_integration = CoverageIntegration::new(Path::new("/tmp"));

        if coverage_integration.is_available().await {
            let version = coverage_integration.get_version().await.unwrap();
            println!("Coverage version: {}", version);
            assert!(!version.is_empty());
        }
    }
}