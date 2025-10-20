//! Cargo Audit Integration - Rust security scanner equivalent to bandit
//!
//! Provides integration with cargo audit for Rust security vulnerability scanning

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// Cargo audit integration for Rust security analysis
pub struct CargoAuditIntegration {
    codebase_path: PathBuf,
}

impl CargoAuditIntegration {
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
impl super::ExternalTool for CargoAuditIntegration {
    type Result = CargoAuditResult;

    async fn is_available(&self) -> bool {
        if !self.is_rust_project() {
            return false;
        }

        Command::new("cargo")
            .args(&["audit", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running cargo audit security analysis on {}", self.codebase_path.display());

        let output = Command::new("cargo")
            .args(&[
                "audit",
                "--format=json",
                "--no-fetch" // Don't update advisory database during analysis
            ])
            .current_dir(&self.codebase_path)
            .output()
            .context("Failed to execute cargo audit")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Cargo audit returns non-zero if vulnerabilities are found
        if !output.status.success() && !stdout.is_empty() {
            debug!("Cargo audit found security issues");
        } else if !output.status.success() {
            warn!("Cargo audit execution failed: {}", stderr);
            return Ok(CargoAuditResult {
                total_vulnerabilities: 0,
                high_severity: 0,
                medium_severity: 0,
                low_severity: 0,
                vulnerabilities: Vec::new(),
                analysis_timestamp: chrono::Utc::now().to_rfc3339(),
                scan_successful: false,
                error_message: Some(stderr.to_string()),
            });
        }

        // Parse cargo audit JSON output
        let audit_report: CargoAuditReport = if !stdout.trim().is_empty() {
            serde_json::from_str(&stdout)
                .context("Failed to parse cargo audit JSON output")?
        } else {
            // No vulnerabilities found
            CargoAuditReport {
                vulnerabilities: Vec::new(),
            }
        };

        let mut vulnerabilities = Vec::new();
        let mut high_count = 0;
        let mut medium_count = 0;
        let mut low_count = 0;

        for vuln in audit_report.vulnerabilities {
            let severity = determine_severity(&vuln);

            match severity.as_str() {
                "high" => high_count += 1,
                "medium" => medium_count += 1,
                "low" => low_count += 1,
                _ => {}
            }

            vulnerabilities.push(CargoAuditVulnerability {
                id: vuln.advisory.id,
                title: vuln.advisory.title,
                description: vuln.advisory.description,
                package: vuln.package.name,
                version: vuln.package.version,
                severity,
                url: vuln.advisory.url,
                patched_versions: vuln.advisory.patched_versions,
                date: vuln.advisory.date,
            });
        }

        debug!("Cargo audit found {} vulnerabilities", vulnerabilities.len());

        Ok(CargoAuditResult {
            total_vulnerabilities: vulnerabilities.len(),
            high_severity: high_count,
            medium_severity: medium_count,
            low_severity: low_count,
            vulnerabilities,
            analysis_timestamp: chrono::Utc::now().to_rfc3339(),
            scan_successful: true,
            error_message: None,
        })
    }

    fn tool_name(&self) -> &'static str {
        "cargo audit"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("cargo")
            .args(&["audit", "--version"])
            .output()
            .context("Failed to get cargo audit version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get cargo audit version"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Determine severity based on vulnerability metadata
fn determine_severity(vuln: &CargoAuditVulnRaw) -> String {
    // Simple heuristic based on keywords in title/description
    let text = format!("{} {}", vuln.advisory.title.to_lowercase(),
                      vuln.advisory.description.to_lowercase());

    if text.contains("critical") || text.contains("remote code execution") ||
       text.contains("arbitrary code") || text.contains("privilege escalation") {
        "high".to_string()
    } else if text.contains("denial of service") || text.contains("memory leak") ||
              text.contains("buffer overflow") || text.contains("use after free") {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

/// Cargo audit analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoAuditResult {
    pub total_vulnerabilities: usize,
    pub high_severity: usize,
    pub medium_severity: usize,
    pub low_severity: usize,
    pub vulnerabilities: Vec<CargoAuditVulnerability>,
    pub analysis_timestamp: String,
    pub scan_successful: bool,
    pub error_message: Option<String>,
}

/// Individual vulnerability found by cargo audit
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoAuditVulnerability {
    pub id: String,
    pub title: String,
    pub description: String,
    pub package: String,
    pub version: String,
    pub severity: String,
    pub url: Option<String>,
    pub patched_versions: Vec<String>,
    pub date: String,
}

/// Raw cargo audit report structure
#[derive(Debug, Deserialize)]
struct CargoAuditReport {
    vulnerabilities: Vec<CargoAuditVulnRaw>,
}

#[derive(Debug, Deserialize)]
struct CargoAuditVulnRaw {
    advisory: CargoAuditAdvisory,
    package: CargoAuditPackage,
}

#[derive(Debug, Deserialize)]
struct CargoAuditAdvisory {
    id: String,
    title: String,
    description: String,
    url: Option<String>,
    patched_versions: Vec<String>,
    date: String,
}

#[derive(Debug, Deserialize)]
struct CargoAuditPackage {
    name: String,
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_cargo_audit_integration() {
        let audit_integration = CargoAuditIntegration::new(Path::new("/tmp"));
        assert_eq!(audit_integration.tool_name(), "cargo audit");
    }

    #[tokio::test]
    async fn test_cargo_audit_rust_project_detection() {
        let temp_dir = tempdir().unwrap();

        // Create a Cargo.toml to simulate a Rust project
        fs::write(temp_dir.path().join("Cargo.toml"),
                 "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let audit_integration = CargoAuditIntegration::new(temp_dir.path());
        assert!(audit_integration.is_rust_project());
    }

    #[tokio::test]
    async fn test_cargo_audit_availability() {
        let temp_dir = tempdir().unwrap();

        // Create a Cargo.toml
        fs::write(temp_dir.path().join("Cargo.toml"),
                 "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();

        let audit_integration = CargoAuditIntegration::new(temp_dir.path());

        // This will depend on whether cargo audit is installed
        let is_available = audit_integration.is_available().await;
        println!("Cargo audit available: {}", is_available);
    }

    #[test]
    fn test_severity_determination() {
        let vuln = CargoAuditVulnRaw {
            advisory: CargoAuditAdvisory {
                id: "TEST-001".to_string(),
                title: "Remote Code Execution vulnerability".to_string(),
                description: "Critical security flaw allowing arbitrary code execution".to_string(),
                url: None,
                patched_versions: vec![],
                date: "2023-01-01".to_string(),
            },
            package: CargoAuditPackage {
                name: "test-pkg".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        assert_eq!(determine_severity(&vuln), "high");
    }
}