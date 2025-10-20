//! External Tool Integration System
//!
//! Manages integration with external code analysis tools (ruff, pylint, mypy, bandit, etc.)
//! providing zero-degradation compatibility with Python implementation.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::process::Command as TokioCommand;
use anyhow::{Result, Context};
use tracing::{info, warn, debug};

pub mod ruff;
pub mod pylint;
pub mod mypy;
pub mod bandit;
pub mod radon;
pub mod vulture;
pub mod coverage;
pub mod git;
pub mod ripgrep;

// Rust-specific tools
pub mod clippy;
pub mod cargo_audit;
pub mod cargo_test;
pub mod rustfmt;

/// External tool manager coordinating all static analysis tools
pub struct ExternalToolManager {
    pub ruff_integration: ruff::RuffIntegration,
    pub pylint_integration: pylint::PylintIntegration,
    pub mypy_integration: mypy::MypyIntegration,
    pub bandit_integration: bandit::BanditIntegration,
    pub radon_integration: radon::RadonIntegration,
    pub vulture_integration: vulture::VultureIntegration,
    pub coverage_integration: coverage::CoverageIntegration,
    pub git_integration: git::GitIntegration,
    pub ripgrep_integration: ripgrep::RipgrepTool,
    tool_availability: HashMap<String, bool>,
    codebase_path: PathBuf,
}

impl ExternalToolManager {
    /// Create new external tool manager
    pub fn new(codebase_path: impl AsRef<Path>) -> Self {
        let codebase_path = codebase_path.as_ref().to_path_buf();

        Self {
            ruff_integration: ruff::RuffIntegration::new(&codebase_path),
            pylint_integration: pylint::PylintIntegration::new(&codebase_path),
            mypy_integration: mypy::MypyIntegration::new(&codebase_path),
            bandit_integration: bandit::BanditIntegration::new(&codebase_path),
            radon_integration: radon::RadonIntegration::new(&codebase_path),
            vulture_integration: vulture::VultureIntegration::new(&codebase_path),
            coverage_integration: coverage::CoverageIntegration::new(&codebase_path),
            git_integration: git::GitIntegration::new(&codebase_path),
            ripgrep_integration: ripgrep::RipgrepTool::new(&codebase_path),
            tool_availability: HashMap::new(),
            codebase_path,
        }
    }

    /// Check availability of all external tools
    pub async fn check_tool_availability(&mut self) -> Result<()> {
        info!("Checking external tool availability...");

        let tools = vec![
            ("ruff", &["--version"]),
            ("pylint", &["--version"]),
            ("mypy", &["--version"]),
            ("bandit", &["--version"]),
            ("radon", &["--version"]),
            ("vulture", &["--version"]),
            ("coverage", &["--version"]),
            ("git", &["--version"]),
            ("rg", &["--version"]),
        ];

        for (tool_name, args) in tools {
            let available = self.is_tool_available(tool_name, args).await;
            self.tool_availability.insert(tool_name.to_string(), available);

            if available {
                debug!("✅ {} is available", tool_name);
            } else {
                warn!("⚠️ {} is not available", tool_name);
            }
        }

        Ok(())
    }

    /// Check if a specific tool is available
    async fn is_tool_available(&self, tool_name: &str, args: &[&str]) -> bool {
        match TokioCommand::new(tool_name)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// Get available tools
    pub fn get_available_tools(&self) -> Vec<String> {
        self.tool_availability
            .iter()
            .filter_map(|(tool, &available)| {
                if available { Some(tool.clone()) } else { None }
            })
            .collect()
    }

    /// Run all available quality analysis tools
    pub async fn run_quality_analysis(&self) -> Result<QualityAnalysisResult> {
        info!("Running quality analysis with available tools...");

        let mut results = QualityAnalysisResult::default();

        // Run ruff if available
        if self.tool_availability.get("ruff").copied().unwrap_or(false) {
            match self.ruff_integration.analyze().await {
                Ok(ruff_result) => {
                    results.ruff_results = Some(ruff_result);
                    debug!("✅ Ruff analysis completed");
                }
                Err(e) => warn!("❌ Ruff analysis failed: {}", e),
            }
        }

        // Run pylint if available
        if self.tool_availability.get("pylint").copied().unwrap_or(false) {
            match self.pylint_integration.analyze().await {
                Ok(pylint_result) => {
                    results.pylint_results = Some(pylint_result);
                    debug!("✅ Pylint analysis completed");
                }
                Err(e) => warn!("❌ Pylint analysis failed: {}", e),
            }
        }

        // Run mypy if available
        if self.tool_availability.get("mypy").copied().unwrap_or(false) {
            match self.mypy_integration.analyze().await {
                Ok(mypy_result) => {
                    results.mypy_results = Some(mypy_result);
                    debug!("✅ Mypy analysis completed");
                }
                Err(e) => warn!("❌ Mypy analysis failed: {}", e),
            }
        }

        // Run bandit if available
        if self.tool_availability.get("bandit").copied().unwrap_or(false) {
            match self.bandit_integration.analyze().await {
                Ok(bandit_result) => {
                    results.bandit_results = Some(bandit_result);
                    debug!("✅ Bandit analysis completed");
                }
                Err(e) => warn!("❌ Bandit analysis failed: {}", e),
            }
        }

        // Run radon if available
        if self.tool_availability.get("radon").copied().unwrap_or(false) {
            match self.radon_integration.analyze().await {
                Ok(radon_result) => {
                    results.radon_results = Some(radon_result);
                    debug!("✅ Radon analysis completed");
                }
                Err(e) => warn!("❌ Radon analysis failed: {}", e),
            }
        }

        // Run vulture if available
        if self.tool_availability.get("vulture").copied().unwrap_or(false) {
            match self.vulture_integration.analyze().await {
                Ok(vulture_result) => {
                    results.vulture_results = Some(vulture_result);
                    debug!("✅ Vulture analysis completed");
                }
                Err(e) => warn!("❌ Vulture analysis failed: {}", e),
            }
        }

        // Run coverage if available
        if self.tool_availability.get("coverage").copied().unwrap_or(false) {
            match self.coverage_integration.analyze().await {
                Ok(coverage_result) => {
                    results.coverage_results = Some(coverage_result);
                    debug!("✅ Coverage analysis completed");
                }
                Err(e) => warn!("❌ Coverage analysis failed: {}", e),
            }
        }

        // Run git analysis if available
        if self.tool_availability.get("git").copied().unwrap_or(false) {
            match self.git_integration.analyze().await {
                Ok(git_result) => {
                    results.git_results = Some(git_result);
                    debug!("✅ Git analysis completed");
                }
                Err(e) => warn!("❌ Git analysis failed: {}", e),
            }
        }

        // Run ripgrep if available
        if self.tool_availability.get("rg").copied().unwrap_or(false) {
            match self.ripgrep_integration.analyze().await {
                Ok(ripgrep_result) => {
                    results.ripgrep_results = Some(ripgrep_result);
                    debug!("✅ Ripgrep analysis completed");
                }
                Err(e) => warn!("❌ Ripgrep analysis failed: {}", e),
            }
        }

        Ok(results)
    }

    /// Run security analysis with available tools
    pub async fn run_security_analysis(&self) -> Result<SecurityAnalysisResult> {
        info!("Running security analysis...");

        let mut results = SecurityAnalysisResult::default();

        // Bandit is the primary security tool
        if self.tool_availability.get("bandit").copied().unwrap_or(false) {
            match self.bandit_integration.analyze().await {
                Ok(bandit_result) => {
                    results.bandit_results = Some(bandit_result);
                    debug!("✅ Security analysis completed");
                }
                Err(e) => warn!("❌ Security analysis failed: {}", e),
            }
        }

        Ok(results)
    }

    /// Get bandit integration if available
    pub fn get_bandit(&self) -> Option<&bandit::BanditIntegration> {
        if self.tool_availability.get("bandit").copied().unwrap_or(false) {
            Some(&self.bandit_integration)
        } else {
            None
        }
    }
}

/// Combined results from all quality analysis tools
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct QualityAnalysisResult {
    pub ruff_results: Option<ruff::RuffResult>,
    pub pylint_results: Option<pylint::PylintResult>,
    pub mypy_results: Option<mypy::MypyResult>,
    pub bandit_results: Option<bandit::BanditResult>,
    pub radon_results: Option<radon::RadonResult>,
    pub vulture_results: Option<vulture::VultureResult>,
    pub coverage_results: Option<coverage::CoverageResult>,
    pub git_results: Option<git::GitResult>,
    pub ripgrep_results: Option<ripgrep::RipgrepResult>,
}

/// Security analysis results
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SecurityAnalysisResult {
    pub bandit_results: Option<bandit::BanditResult>,
}

/// Rust-specific tool manager coordinating Rust static analysis tools
pub struct RustToolManager {
    pub clippy_integration: clippy::ClippyIntegration,
    pub cargo_audit_integration: cargo_audit::CargoAuditIntegration,
    pub cargo_test_integration: cargo_test::CargoTestIntegration,
    pub rustfmt_integration: rustfmt::RustfmtIntegration,
    pub git_integration: git::GitIntegration,
    pub ripgrep_integration: ripgrep::RipgrepTool,
    tool_availability: HashMap<String, bool>,
    codebase_path: PathBuf,
}

impl RustToolManager {
    /// Create new Rust tool manager
    pub fn new(codebase_path: impl AsRef<Path>) -> Self {
        let codebase_path = codebase_path.as_ref().to_path_buf();

        Self {
            clippy_integration: clippy::ClippyIntegration::new(&codebase_path),
            cargo_audit_integration: cargo_audit::CargoAuditIntegration::new(&codebase_path),
            cargo_test_integration: cargo_test::CargoTestIntegration::new(&codebase_path),
            rustfmt_integration: rustfmt::RustfmtIntegration::new(&codebase_path),
            git_integration: git::GitIntegration::new(&codebase_path),
            ripgrep_integration: ripgrep::RipgrepTool::new(&codebase_path),
            tool_availability: HashMap::new(),
            codebase_path,
        }
    }

    /// Check availability of all Rust tools
    pub async fn check_tool_availability(&mut self) -> Result<()> {
        info!("Checking Rust tool availability...");

        let tools: Vec<(&str, &[&str])> = vec![
            ("cargo clippy", &["clippy", "--version"]),
            ("cargo audit", &["audit", "--version"]),
            ("cargo test", &["--version"]),
            ("rustfmt", &["--version"]),
            ("git", &["--version"]),
            ("rg", &["--version"]),
        ];

        for (tool_name, args) in tools {
            let available = if tool_name.starts_with("cargo") {
                self.is_cargo_tool_available(&args[0], &args[1..]).await
            } else {
                self.is_tool_available(tool_name, args).await
            };

            self.tool_availability.insert(tool_name.to_string(), available);

            if available {
                debug!("✅ {} is available", tool_name);
            } else {
                warn!("⚠️ {} is not available", tool_name);
            }
        }

        Ok(())
    }

    /// Check if a cargo subcommand is available
    async fn is_cargo_tool_available(&self, subcommand: &str, args: &[&str]) -> bool {
        let mut cmd_args = vec![subcommand];
        cmd_args.extend_from_slice(args);

        match TokioCommand::new("cargo")
            .args(&cmd_args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// Check if a specific tool is available
    async fn is_tool_available(&self, tool_name: &str, args: &[&str]) -> bool {
        match TokioCommand::new(tool_name)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// Get available tools
    pub fn get_available_tools(&self) -> Vec<String> {
        self.tool_availability
            .iter()
            .filter_map(|(tool, &available)| {
                if available { Some(tool.clone()) } else { None }
            })
            .collect()
    }

    /// Run all available Rust quality analysis tools
    pub async fn run_quality_analysis(&self) -> Result<RustQualityAnalysisResult> {
        info!("Running Rust quality analysis with available tools...");

        let mut results = RustQualityAnalysisResult::default();

        // Run clippy if available
        if self.tool_availability.get("cargo clippy").copied().unwrap_or(false) {
            match self.clippy_integration.analyze().await {
                Ok(clippy_result) => {
                    results.clippy_results = Some(clippy_result);
                    debug!("✅ Clippy analysis completed");
                }
                Err(e) => warn!("❌ Clippy analysis failed: {}", e),
            }
        }

        // Run rustfmt if available
        if self.tool_availability.get("rustfmt").copied().unwrap_or(false) {
            match self.rustfmt_integration.analyze().await {
                Ok(rustfmt_result) => {
                    results.rustfmt_results = Some(rustfmt_result);
                    debug!("✅ Rustfmt analysis completed");
                }
                Err(e) => warn!("❌ Rustfmt analysis failed: {}", e),
            }
        }

        // Run cargo test if available
        if self.tool_availability.get("cargo test").copied().unwrap_or(false) {
            match self.cargo_test_integration.analyze().await {
                Ok(test_result) => {
                    results.cargo_test_results = Some(test_result);
                    debug!("✅ Cargo test analysis completed");
                }
                Err(e) => warn!("❌ Cargo test analysis failed: {}", e),
            }
        }

        // Run git analysis if available
        if self.tool_availability.get("git").copied().unwrap_or(false) {
            match self.git_integration.analyze().await {
                Ok(git_result) => {
                    results.git_results = Some(git_result);
                    debug!("✅ Git analysis completed");
                }
                Err(e) => warn!("❌ Git analysis failed: {}", e),
            }
        }

        // Run ripgrep if available
        if self.tool_availability.get("rg").copied().unwrap_or(false) {
            match self.ripgrep_integration.analyze().await {
                Ok(ripgrep_result) => {
                    results.ripgrep_results = Some(ripgrep_result);
                    debug!("✅ Ripgrep analysis completed");
                }
                Err(e) => warn!("❌ Ripgrep analysis failed: {}", e),
            }
        }

        Ok(results)
    }

    /// Run security analysis with available Rust tools
    pub async fn run_security_analysis(&self) -> Result<RustSecurityAnalysisResult> {
        info!("Running Rust security analysis...");

        let mut results = RustSecurityAnalysisResult::default();

        // Cargo audit is the primary security tool for Rust
        if self.tool_availability.get("cargo audit").copied().unwrap_or(false) {
            match self.cargo_audit_integration.analyze().await {
                Ok(audit_result) => {
                    results.cargo_audit_results = Some(audit_result);
                    debug!("✅ Cargo audit security analysis completed");
                }
                Err(e) => warn!("❌ Cargo audit security analysis failed: {}", e),
            }
        }

        Ok(results)
    }
}

/// Combined results from all Rust quality analysis tools
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RustQualityAnalysisResult {
    pub clippy_results: Option<clippy::ClippyResult>,
    pub rustfmt_results: Option<rustfmt::RustfmtResult>,
    pub cargo_test_results: Option<cargo_test::CargoTestResult>,
    pub git_results: Option<git::GitResult>,
    pub ripgrep_results: Option<ripgrep::RipgrepResult>,
}

/// Rust security analysis results
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RustSecurityAnalysisResult {
    pub cargo_audit_results: Option<cargo_audit::CargoAuditResult>,
}

/// Base trait for all external tool integrations
#[async_trait::async_trait]
pub trait ExternalTool {
    type Result: Serialize + for<'de> Deserialize<'de>;

    /// Check if the tool is available
    async fn is_available(&self) -> bool;

    /// Run the analysis
    async fn analyze(&self) -> Result<Self::Result>;

    /// Get the tool name
    fn tool_name(&self) -> &'static str;

    /// Get the tool version
    async fn get_version(&self) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_external_tool_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let mut manager = ExternalToolManager::new(temp_dir.path());

        // Check tool availability
        manager.check_tool_availability().await.unwrap();

        // Should not crash even if no tools are available
        let available_tools = manager.get_available_tools();
        println!("Available tools: {:?}", available_tools);
    }
}