//! Radon Complexity Analyzer Integration
//!
//! Zero-degradation integration with Radon complexity analyzer matching Python behavior

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

pub struct RadonIntegration {
    codebase_path: PathBuf,
}

impl RadonIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalTool for RadonIntegration {
    type Result = RadonResult;

    async fn is_available(&self) -> bool {
        Command::new("radon")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running radon complexity analysis on {}", self.codebase_path.display());

        // Run cyclomatic complexity analysis
        let cc_result = self.run_cyclomatic_complexity().await?;

        // Run maintainability index analysis
        let mi_result = self.run_maintainability_index().await?;

        // Run halstead complexity analysis
        let hal_result = self.run_halstead_analysis().await?;

        // Combine results
        let mut total_complexity = 0;
        let mut function_count = 0;

        for function in &cc_result.functions {
            total_complexity += function.complexity;
            function_count += 1;
        }

        let average_complexity = if function_count > 0 {
            total_complexity as f64 / function_count as f64
        } else {
            0.0
        };

        Ok(RadonResult {
            total_complexity,
            average_complexity,
            function_count,
            cyclomatic_complexity: cc_result,
            maintainability_index: mi_result,
            halstead_metrics: hal_result,
        })
    }

    fn tool_name(&self) -> &'static str {
        "radon"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("radon")
            .arg("--version")
            .output()
            .await
            .context("Failed to get radon version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get radon version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

impl RadonIntegration {
    async fn run_cyclomatic_complexity(&self) -> Result<CyclomaticComplexityResult> {
        let output = Command::new("radon")
            .arg("cc")
            .arg("-j") // JSON output
            .arg("-a") // Show all functions
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute radon cc")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(CyclomaticComplexityResult::default());
        }

        // Parse radon CC JSON output
        let cc_data: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to parse radon CC JSON output: {}", e);
                return Ok(CyclomaticComplexityResult::default());
            }
        };

        let mut functions = Vec::new();

        // Parse the nested JSON structure
        if let Some(obj) = cc_data.as_object() {
            for (file_path, file_data) in obj {
                if let Some(functions_array) = file_data.as_array() {
                    for func_data in functions_array {
                        if let Some(func_obj) = func_data.as_object() {
                            let function = FunctionComplexity {
                                name: func_obj.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                complexity: func_obj.get("complexity")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1) as usize,
                                line_number: func_obj.get("lineno")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1) as usize,
                                end_line: func_obj.get("endline")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1) as usize,
                                file_path: file_path.clone(),
                                rank: func_obj.get("rank")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("A")
                                    .to_string(),
                            };
                            functions.push(function);
                        }
                    }
                }
            }
        }

        Ok(CyclomaticComplexityResult { functions })
    }

    async fn run_maintainability_index(&self) -> Result<MaintainabilityResult> {
        let output = Command::new("radon")
            .arg("mi")
            .arg("-j") // JSON output
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute radon mi")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(MaintainabilityResult::default());
        }

        // Parse radon MI JSON output
        let mi_data: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to parse radon MI JSON output: {}", e);
                return Ok(MaintainabilityResult::default());
            }
        };

        let mut files = Vec::new();

        if let Some(obj) = mi_data.as_object() {
            for (file_path, mi_value) in obj {
                if let Some(mi) = mi_value.as_f64() {
                    files.push(FileMaintainability {
                        file_path: file_path.clone(),
                        maintainability_index: mi,
                        rank: self.get_mi_rank(mi),
                    });
                }
            }
        }

        let average_mi = if !files.is_empty() {
            files.iter().map(|f| f.maintainability_index).sum::<f64>() / files.len() as f64
        } else {
            0.0
        };

        Ok(MaintainabilityResult {
            files,
            average_maintainability_index: average_mi,
        })
    }

    async fn run_halstead_analysis(&self) -> Result<HalsteadResult> {
        let output = Command::new("radon")
            .arg("hal")
            .arg("-j") // JSON output
            .arg(&self.codebase_path)
            .output()
            .await
            .context("Failed to execute radon hal")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(HalsteadResult::default());
        }

        // Parse radon Halstead JSON output
        let hal_data: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to parse radon Halstead JSON output: {}", e);
                return Ok(HalsteadResult::default());
            }
        };

        let mut functions = Vec::new();

        if let Some(obj) = hal_data.as_object() {
            for (file_path, file_data) in obj {
                if let Some(functions_array) = file_data.as_array() {
                    for func_data in functions_array {
                        if let Some(func_obj) = func_data.as_object() {
                            let function = FunctionHalstead {
                                name: func_obj.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                file_path: file_path.clone(),
                                line_number: func_obj.get("lineno")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1) as usize,
                                volume: func_obj.get("volume")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                                difficulty: func_obj.get("difficulty")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                                effort: func_obj.get("effort")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                                time: func_obj.get("time")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                                bugs: func_obj.get("bugs")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                            };
                            functions.push(function);
                        }
                    }
                }
            }
        }

        Ok(HalsteadResult { functions })
    }

    fn get_mi_rank(&self, mi: f64) -> String {
        if mi >= 85.0 {
            "A".to_string()
        } else if mi >= 70.0 {
            "B".to_string()
        } else if mi >= 50.0 {
            "C".to_string()
        } else if mi >= 25.0 {
            "D".to_string()
        } else {
            "F".to_string()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RadonResult {
    pub total_complexity: usize,
    pub average_complexity: f64,
    pub function_count: usize,
    pub cyclomatic_complexity: CyclomaticComplexityResult,
    pub maintainability_index: MaintainabilityResult,
    pub halstead_metrics: HalsteadResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CyclomaticComplexityResult {
    pub functions: Vec<FunctionComplexity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub complexity: usize,
    pub line_number: usize,
    pub end_line: usize,
    pub file_path: String,
    pub rank: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaintainabilityResult {
    pub files: Vec<FileMaintainability>,
    pub average_maintainability_index: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMaintainability {
    pub file_path: String,
    pub maintainability_index: f64,
    pub rank: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HalsteadResult {
    pub functions: Vec<FunctionHalstead>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionHalstead {
    pub name: String,
    pub file_path: String,
    pub line_number: usize,
    pub volume: f64,
    pub difficulty: f64,
    pub effort: f64,
    pub time: f64,
    pub bugs: f64,
}

impl Default for RadonResult {
    fn default() -> Self {
        Self {
            total_complexity: 0,
            average_complexity: 0.0,
            function_count: 0,
            cyclomatic_complexity: CyclomaticComplexityResult::default(),
            maintainability_index: MaintainabilityResult::default(),
            halstead_metrics: HalsteadResult::default(),
        }
    }
}

impl Default for CyclomaticComplexityResult {
    fn default() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}

impl Default for MaintainabilityResult {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            average_maintainability_index: 0.0,
        }
    }
}

impl Default for HalsteadResult {
    fn default() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_radon_integration() {
        let radon_integration = RadonIntegration::new(Path::new("/tmp"));

        assert_eq!(radon_integration.tool_name(), "radon");

        // Test availability (may or may not be installed)
        let is_available = radon_integration.is_available().await;
        println!("Radon available: {}", is_available);
    }

    #[tokio::test]
    async fn test_radon_version() {
        let radon_integration = RadonIntegration::new(Path::new("/tmp"));

        if radon_integration.is_available().await {
            let version = radon_integration.get_version().await.unwrap();
            println!("Radon version: {}", version);
            assert!(!version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_radon_analysis_complex_code() {
        let temp_dir = tempdir().unwrap();

        // Create a Python file with varying complexity
        let python_file = temp_dir.path().join("complex.py");
        fs::write(&python_file, r#"
def simple_function(x):
    """Simple function with complexity 1."""
    return x + 1

def complex_function(data):
    """Complex function with high cyclomatic complexity."""
    result = 0

    for item in data:
        if item > 0:
            if item % 2 == 0:
                if item > 100:
                    result += item * 2
                elif item > 50:
                    result += item * 1.5
                else:
                    result += item
            else:
                if item > 10:
                    result -= item
                else:
                    result += item / 2
        elif item < 0:
            if abs(item) > 50:
                result *= 0.9
            else:
                result *= 1.1
        else:
            result = result or 1

    try:
        normalized = result / len(data)
    except ZeroDivisionError:
        normalized = 0
    except Exception:
        normalized = -1

    return normalized

class ComplexClass:
    def __init__(self, value):
        self.value = value

    def process(self, data):
        """Method with moderate complexity."""
        if not data:
            return None

        processed = []
        for item in data:
            if isinstance(item, (int, float)):
                if item > self.value:
                    processed.append(item * 2)
                elif item < 0:
                    processed.append(abs(item))
                else:
                    processed.append(item)
            else:
                try:
                    processed.append(float(item))
                except (ValueError, TypeError):
                    processed.append(0)

        return processed if processed else None
"#).unwrap();

        let radon_integration = RadonIntegration::new(temp_dir.path());

        if radon_integration.is_available().await {
            let result = radon_integration.analyze().await.unwrap();
            println!("Radon analysis results:");
            println!("  Total complexity: {}", result.total_complexity);
            println!("  Average complexity: {:.2}", result.average_complexity);
            println!("  Function count: {}", result.function_count);

            for func in result.cyclomatic_complexity.functions.iter().take(3) {
                println!("  Function '{}': complexity {}, rank {}",
                    func.name, func.complexity, func.rank);
            }

            for file in result.maintainability_index.files.iter().take(3) {
                println!("  File '{}': MI {:.2}, rank {}",
                    file.file_path, file.maintainability_index, file.rank);
            }

            // Should find some functions with varying complexity
            assert!(result.function_count > 0);
            assert!(result.total_complexity > 0);
        }
    }

    #[test]
    fn test_mi_rank_calculation() {
        let radon_integration = RadonIntegration::new(Path::new("/tmp"));

        assert_eq!(radon_integration.get_mi_rank(90.0), "A");
        assert_eq!(radon_integration.get_mi_rank(75.0), "B");
        assert_eq!(radon_integration.get_mi_rank(60.0), "C");
        assert_eq!(radon_integration.get_mi_rank(30.0), "D");
        assert_eq!(radon_integration.get_mi_rank(10.0), "F");
    }
}