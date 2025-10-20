//! Vulture Dead Code Detector Integration
//!
//! Zero-degradation integration with Vulture dead code detector matching Python behavior

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

pub struct VultureIntegration {
    codebase_path: PathBuf,
}

impl VultureIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalTool for VultureIntegration {
    type Result = VultureResult;

    async fn is_available(&self) -> bool {
        Command::new("vulture")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running vulture dead code analysis on {}", self.codebase_path.display());

        let output = Command::new("vulture")
            .arg("--min-confidence")
            .arg("80") // Only report high-confidence dead code
            .arg("--sort-by-size")
            .arg("--exclude")
            .arg("*/tests/*,*/test_*,*/.venv/*,*/venv/*,*/__pycache__/*")
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute vulture")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            // No dead code found
            return Ok(VultureResult::default());
        }

        // Parse vulture output (line-based format)
        let mut dead_code_items = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(item) = self.parse_vulture_line(line) {
                dead_code_items.push(item);
            }
        }

        // Categorize dead code by type
        let mut unused_functions = 0;
        let mut unused_classes = 0;
        let mut unused_variables = 0;
        let mut unused_imports = 0;
        let mut unused_attributes = 0;

        for item in &dead_code_items {
            match item.item_type.as_str() {
                "function" => unused_functions += 1,
                "class" => unused_classes += 1,
                "variable" => unused_variables += 1,
                "import" => unused_imports += 1,
                "attribute" => unused_attributes += 1,
                _ => {}
            }
        }

        Ok(VultureResult {
            total_dead_code: dead_code_items.len(),
            unused_functions,
            unused_classes,
            unused_variables,
            unused_imports,
            unused_attributes,
            dead_code_items,
        })
    }

    fn tool_name(&self) -> &'static str {
        "vulture"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("vulture")
            .arg("--version")
            .output()
            .await
            .context("Failed to get vulture version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get vulture version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

impl VultureIntegration {
    fn parse_vulture_line(&self, line: &str) -> Option<DeadCodeItem> {
        // Parse vulture output format: filename:line: unused item 'name' (confidence%)
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            return None;
        }

        let filename = parts[0].trim();
        let line_num: usize = parts[1].trim().parse().ok()?;
        let message = parts[2].trim();

        // Extract item type and name from message
        let (item_type, name, confidence) = self.parse_vulture_message(message)?;

        Some(DeadCodeItem {
            filename: filename.to_string(),
            line_number: line_num,
            item_type,
            name,
            confidence,
            message: message.to_string(),
        })
    }

    fn parse_vulture_message(&self, message: &str) -> Option<(String, String, usize)> {
        // Parse messages like:
        // "unused function 'old_function' (100% confidence)"
        // "unused variable 'unused_var' (90% confidence)"
        // "unused import 'os' (80% confidence)"

        let message_lower = message.to_lowercase();

        let item_type = if message_lower.contains("function") {
            "function"
        } else if message_lower.contains("class") {
            "class"
        } else if message_lower.contains("variable") {
            "variable"
        } else if message_lower.contains("import") {
            "import"
        } else if message_lower.contains("attribute") {
            "attribute"
        } else if message_lower.contains("property") {
            "property"
        } else {
            "unknown"
        };

        // Extract name between single quotes
        let name = if let Some(start) = message.find('\'') {
            if let Some(end) = message.rfind('\'') {
                if start < end {
                    message[start + 1..end].to_string()
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        };

        // Extract confidence percentage
        let confidence = if let Some(start) = message.find('(') {
            if let Some(end) = message.find("% confidence)") {
                if start < end {
                    let conf_str = &message[start + 1..end];
                    conf_str.parse().unwrap_or(80)
                } else {
                    80
                }
            } else {
                80
            }
        } else {
            80
        };

        Some((item_type.to_string(), name, confidence))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VultureResult {
    pub total_dead_code: usize,
    pub unused_functions: usize,
    pub unused_classes: usize,
    pub unused_variables: usize,
    pub unused_imports: usize,
    pub unused_attributes: usize,
    pub dead_code_items: Vec<DeadCodeItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeItem {
    pub filename: String,
    pub line_number: usize,
    pub item_type: String,
    pub name: String,
    pub confidence: usize,
    pub message: String,
}

impl Default for VultureResult {
    fn default() -> Self {
        Self {
            total_dead_code: 0,
            unused_functions: 0,
            unused_classes: 0,
            unused_variables: 0,
            unused_imports: 0,
            unused_attributes: 0,
            dead_code_items: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_vulture_integration() {
        let vulture_integration = VultureIntegration::new(Path::new("/tmp"));

        assert_eq!(vulture_integration.tool_name(), "vulture");

        // Test availability (may or may not be installed)
        let is_available = vulture_integration.is_available().await;
        println!("Vulture available: {}", is_available);
    }

    #[tokio::test]
    async fn test_vulture_version() {
        let vulture_integration = VultureIntegration::new(Path::new("/tmp"));

        if vulture_integration.is_available().await {
            let version = vulture_integration.get_version().await.unwrap();
            println!("Vulture version: {}", version);
            assert!(!version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_vulture_analysis_dead_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file with dead code
        let python_file = temp_dir.path().join("dead_code.py");
        fs::write(&python_file, r#"
import os
import sys
import unused_module

# Used function
def used_function():
    print("This function is used")
    return 42

# Unused function
def unused_function():
    return "This function is never called"

# Unused variable
unused_variable = "This variable is never used"

# Used variable
used_variable = "This variable is used"

class UsedClass:
    def __init__(self):
        self.value = 1

    def used_method(self):
        return self.value

    # Unused method
    def unused_method(self):
        return "never called"

# Unused class
class UnusedClass:
    def __init__(self):
        self.data = []

    def process(self):
        return len(self.data)

# Main code using some items
if __name__ == "__main__":
    result = used_function()
    print(used_variable)

    obj = UsedClass()
    print(obj.used_method())
"#).unwrap();

        let vulture_integration = VultureIntegration::new(temp_dir.path());

        if vulture_integration.is_available().await {
            let result = vulture_integration.analyze().await.unwrap();
            println!("Vulture found {} dead code items", result.total_dead_code);
            println!("Unused functions: {}, classes: {}, variables: {}, imports: {}",
                result.unused_functions, result.unused_classes,
                result.unused_variables, result.unused_imports);

            for item in result.dead_code_items.iter().take(5) {
                println!("Dead code: {} '{}' at {}:{} ({}% confidence)",
                    item.item_type, item.name, item.filename, item.line_number, item.confidence);
            }

            // Should find some dead code
            assert!(result.total_dead_code > 0);
        }
    }

    #[test]
    fn test_vulture_message_parsing() {
        let vulture_integration = VultureIntegration::new(Path::new("/tmp"));

        let test_cases = vec![
            ("unused function 'old_function' (100% confidence)", Some(("function".to_string(), "old_function".to_string(), 100))),
            ("unused variable 'unused_var' (90% confidence)", Some(("variable".to_string(), "unused_var".to_string(), 90))),
            ("unused import 'os' (80% confidence)", Some(("import".to_string(), "os".to_string(), 80))),
            ("unused class 'UnusedClass' (95% confidence)", Some(("class".to_string(), "UnusedClass".to_string(), 95))),
        ];

        for (message, expected) in test_cases {
            let result = vulture_integration.parse_vulture_message(message);
            assert_eq!(result, expected, "Failed to parse: {}", message);
        }
    }
}