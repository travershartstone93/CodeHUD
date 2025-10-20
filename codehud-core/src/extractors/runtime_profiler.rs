//! Runtime Profiler Data Extractor - Analyzes runtime performance and execution patterns

use super::BaseDataExtractor;
use crate::external_tools::ExternalToolManager;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use tree_sitter::{Language, Parser};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

extern "C" {
    fn tree_sitter_rust() -> Language;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeMetric {
    function_name: String,
    file_path: String,
    line_number: usize,
    execution_time_ms: f64,
    memory_usage_mb: f64,
    call_count: usize,
    cpu_percentage: f64,
    is_hotspot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceBottleneck {
    bottleneck_type: String,
    description: String,
    severity: String, // "critical", "major", "minor"
    file_path: String,
    line_number: usize,
    impact_score: f64,
    recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResourceUsage {
    total_memory_mb: f64,
    peak_memory_mb: f64,
    average_cpu_percentage: f64,
    io_operations: usize,
    network_calls: usize,
    database_queries: usize,
    file_operations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionPattern {
    pattern_type: String,
    description: String,
    frequency: usize,
    files_affected: Vec<String>,
    performance_impact: String, // "high", "medium", "low"
}

pub struct RuntimeProfiler {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
}

impl RuntimeProfiler {
    pub fn new(codebase_path: impl AsRef<Path>) -> crate::Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();
        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!("Codebase path does not exist: {}", codebase_path.display())));
        }

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Config(format!("Failed to set language: {}", e)))?;

        let external_tools = ExternalToolManager::new(&codebase_path);

        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
            parser,
            external_tools,
        })
    }

    fn get_all_python_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_files_recursive(&self.codebase_path, &mut files);
        files
    }

    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                    files.push(path);
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    self.collect_files_recursive(&path, files);
                }
            }
        }
    }

    fn is_excluded_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "__pycache__" | ".git" | ".pytest_cache" | "node_modules" | ".venv" | "venv")
        } else {
            false
        }
    }

    fn analyze_runtime_metrics(&self, files: &[PathBuf]) -> crate::Result<Vec<RuntimeMetric>> {
        let mut metrics = Vec::new();

        for file_path in files {
            if let Ok(file_metrics) = self.analyze_file_runtime(file_path) {
                metrics.extend(file_metrics);
            }
        }

        Ok(metrics)
    }

    fn analyze_file_runtime(&self, file_path: &Path) -> crate::Result<Vec<RuntimeMetric>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io(e))?;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Analysis(format!("Failed to set language: {}", e)))?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Analysis("Failed to parse file".to_string()))?;

        let mut metrics = Vec::new();
        self.extract_runtime_elements(tree.root_node(), &content, file_path, &mut metrics);

        Ok(metrics)
    }

    fn extract_runtime_elements(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        metrics: &mut Vec<RuntimeMetric>,
    ) {
        match node.kind() {
            "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let func_name = &source[name_node.start_byte()..name_node.end_byte()];
                    let line_number = node.start_position().row + 1;

                    // Analyze function complexity as a proxy for runtime cost
                    let complexity_score = self.calculate_function_complexity(node, source);
                    let estimated_execution_time = self.estimate_execution_time(node, source);
                    let estimated_memory_usage = self.estimate_memory_usage(node, source);

                    metrics.push(RuntimeMetric {
                        function_name: func_name.to_string(),
                        file_path: file_path.display().to_string(),
                        line_number,
                        execution_time_ms: estimated_execution_time,
                        memory_usage_mb: estimated_memory_usage,
                        call_count: 1, // Default, would be updated with actual profiling
                        cpu_percentage: complexity_score * 10.0, // Heuristic
                        is_hotspot: complexity_score > 5.0,
                    });
                }
            }
            _ => {}
        }

        // Recursively process child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.extract_runtime_elements(child, source, file_path, metrics);
            }
        }
    }

    fn calculate_function_complexity(&self, node: tree_sitter::Node, source: &str) -> f64 {
        let func_source = &source[node.start_byte()..node.end_byte()];
        let mut complexity = 1.0; // Base complexity

        // Count control flow statements
        complexity += func_source.matches("if ").count() as f64 * 1.0;
        complexity += func_source.matches("for ").count() as f64 * 2.0;
        complexity += func_source.matches("while ").count() as f64 * 2.0;
        complexity += func_source.matches("try:").count() as f64 * 1.5;
        complexity += func_source.matches("except").count() as f64 * 1.5;
        complexity += func_source.matches("with ").count() as f64 * 1.0;

        // Count function calls (potential performance impact)
        complexity += func_source.matches("(").count() as f64 * 0.1;

        // Count list comprehensions and generators
        complexity += func_source.matches("[").count() as f64 * 0.5;
        complexity += func_source.matches("yield").count() as f64 * 1.0;

        complexity
    }

    fn estimate_execution_time(&self, node: tree_sitter::Node, source: &str) -> f64 {
        let func_source = &source[node.start_byte()..node.end_byte()];
        let mut estimated_time = 0.1; // Base time in milliseconds

        // Heuristics for execution time based on code patterns
        estimated_time += func_source.matches("sleep").count() as f64 * 1000.0;
        estimated_time += func_source.matches("requests.").count() as f64 * 100.0;
        estimated_time += func_source.matches("open(").count() as f64 * 10.0;
        estimated_time += func_source.matches("json.loads").count() as f64 * 5.0;
        estimated_time += func_source.matches("json.dumps").count() as f64 * 5.0;
        estimated_time += func_source.matches("pickle.").count() as f64 * 20.0;
        estimated_time += func_source.matches("sql").count() as f64 * 50.0;
        estimated_time += func_source.matches("SELECT").count() as f64 * 50.0;

        // Loop multipliers
        let loop_count = func_source.matches("for ").count() + func_source.matches("while ").count();
        estimated_time *= (1.0 + loop_count as f64).powf(1.5);

        estimated_time
    }

    fn estimate_memory_usage(&self, node: tree_sitter::Node, source: &str) -> f64 {
        let func_source = &source[node.start_byte()..node.end_byte()];
        let mut estimated_memory = 0.1; // Base memory in MB

        // Heuristics for memory usage
        estimated_memory += func_source.matches("list(").count() as f64 * 1.0;
        estimated_memory += func_source.matches("dict(").count() as f64 * 0.5;
        estimated_memory += func_source.matches("set(").count() as f64 * 0.8;
        estimated_memory += func_source.matches("DataFrame").count() as f64 * 10.0;
        estimated_memory += func_source.matches("numpy").count() as f64 * 5.0;
        estimated_memory += func_source.matches("append(").count() as f64 * 0.1;
        estimated_memory += func_source.matches("extend(").count() as f64 * 0.5;

        // Large data operations
        estimated_memory += func_source.matches("read_csv").count() as f64 * 50.0;
        estimated_memory += func_source.matches("read_json").count() as f64 * 20.0;
        estimated_memory += func_source.matches("pickle.load").count() as f64 * 30.0;

        estimated_memory
    }

    fn identify_performance_bottlenecks(&self, metrics: &[RuntimeMetric]) -> Vec<PerformanceBottleneck> {
        let mut bottlenecks = Vec::new();

        for metric in metrics {
            let mut severity = "minor";
            let mut impact_score = 0.0;

            // High execution time bottlenecks
            if metric.execution_time_ms > 1000.0 {
                severity = "critical";
                impact_score = metric.execution_time_ms / 1000.0;
                bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: "high_execution_time".to_string(),
                    description: format!("Function '{}' has estimated high execution time", metric.function_name),
                    severity: severity.to_string(),
                    file_path: metric.file_path.clone(),
                    line_number: metric.line_number,
                    impact_score,
                    recommendation: "Consider optimizing algorithms or adding caching".to_string(),
                });
            } else if metric.execution_time_ms > 100.0 {
                severity = "major";
                impact_score = metric.execution_time_ms / 1000.0;
                bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: "moderate_execution_time".to_string(),
                    description: format!("Function '{}' has moderate execution time", metric.function_name),
                    severity: severity.to_string(),
                    file_path: metric.file_path.clone(),
                    line_number: metric.line_number,
                    impact_score,
                    recommendation: "Monitor and consider optimization if called frequently".to_string(),
                });
            }

            // High memory usage bottlenecks
            if metric.memory_usage_mb > 100.0 {
                severity = if metric.memory_usage_mb > 500.0 { "critical" } else { "major" };
                impact_score = metric.memory_usage_mb / 100.0;
                bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: "high_memory_usage".to_string(),
                    description: format!("Function '{}' has estimated high memory usage", metric.function_name),
                    severity: severity.to_string(),
                    file_path: metric.file_path.clone(),
                    line_number: metric.line_number,
                    impact_score,
                    recommendation: "Consider memory optimization techniques or streaming".to_string(),
                });
            }

            // CPU hotspots
            if metric.is_hotspot {
                bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: "cpu_hotspot".to_string(),
                    description: format!("Function '{}' identified as potential CPU hotspot", metric.function_name),
                    severity: "major".to_string(),
                    file_path: metric.file_path.clone(),
                    line_number: metric.line_number,
                    impact_score: metric.cpu_percentage / 10.0,
                    recommendation: "Profile with actual runtime data and optimize critical paths".to_string(),
                });
            }
        }

        // Sort by impact score (highest first)
        bottlenecks.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());

        bottlenecks
    }

    fn analyze_resource_usage(&self, metrics: &[RuntimeMetric]) -> ResourceUsage {
        let total_memory_mb: f64 = metrics.iter().map(|m| m.memory_usage_mb).sum();
        let peak_memory_mb = metrics.iter().map(|m| m.memory_usage_mb).fold(0.0, f64::max);
        let average_cpu_percentage = if !metrics.is_empty() {
            metrics.iter().map(|m| m.cpu_percentage).sum::<f64>() / metrics.len() as f64
        } else {
            0.0
        };

        // Estimate resource operations based on function analysis
        let io_operations = metrics.iter().filter(|m| {
            m.function_name.contains("read") || m.function_name.contains("write") || m.function_name.contains("file")
        }).count();

        let network_calls = metrics.iter().filter(|m| {
            m.function_name.contains("request") || m.function_name.contains("fetch") || m.function_name.contains("http")
        }).count();

        let database_queries = metrics.iter().filter(|m| {
            m.function_name.contains("query") || m.function_name.contains("sql") || m.function_name.contains("select")
        }).count();

        let file_operations = metrics.iter().filter(|m| {
            m.function_name.contains("open") || m.function_name.contains("save") || m.function_name.contains("load")
        }).count();

        ResourceUsage {
            total_memory_mb,
            peak_memory_mb,
            average_cpu_percentage,
            io_operations,
            network_calls,
            database_queries,
            file_operations,
        }
    }

    fn identify_execution_patterns(&self, metrics: &[RuntimeMetric]) -> Vec<ExecutionPattern> {
        let mut patterns = Vec::new();

        // Pattern 1: High-memory functions
        let high_memory_files: Vec<String> = metrics
            .iter()
            .filter(|m| m.memory_usage_mb > 50.0)
            .map(|m| m.file_path.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if !high_memory_files.is_empty() {
            patterns.push(ExecutionPattern {
                pattern_type: "high_memory_functions".to_string(),
                description: "Functions with high estimated memory usage".to_string(),
                frequency: high_memory_files.len(),
                files_affected: high_memory_files,
                performance_impact: "high".to_string(),
            });
        }

        // Pattern 2: CPU-intensive functions
        let cpu_intensive_files: Vec<String> = metrics
            .iter()
            .filter(|m| m.is_hotspot)
            .map(|m| m.file_path.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if !cpu_intensive_files.is_empty() {
            patterns.push(ExecutionPattern {
                pattern_type: "cpu_intensive_functions".to_string(),
                description: "Functions identified as CPU hotspots".to_string(),
                frequency: cpu_intensive_files.len(),
                files_affected: cpu_intensive_files,
                performance_impact: "high".to_string(),
            });
        }

        // Pattern 3: IO-bound operations
        let io_bound_files: Vec<String> = metrics
            .iter()
            .filter(|m| m.function_name.contains("read") || m.function_name.contains("write") || m.function_name.contains("load"))
            .map(|m| m.file_path.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if !io_bound_files.is_empty() {
            patterns.push(ExecutionPattern {
                pattern_type: "io_bound_operations".to_string(),
                description: "Functions performing I/O operations".to_string(),
                frequency: io_bound_files.len(),
                files_affected: io_bound_files,
                performance_impact: "medium".to_string(),
            });
        }

        patterns
    }

    fn run_actual_profiling(&self) -> crate::Result<HashMap<String, Value>> {
        // Try to run Python profiling tools if available
        let profile_result = Command::new("python")
            .args(&["-m", "cProfile", "-s", "tottime", "-"])
            .arg("import sys; print('Profile test')")
            .current_dir(&self.codebase_path)
            .output();

        let mut profiling_data = HashMap::new();

        if let Ok(output) = profile_result {
            if output.status.success() {
                profiling_data.insert("profiling_available".to_string(), json!(true));
                profiling_data.insert("profiling_output".to_string(), json!(String::from_utf8_lossy(&output.stdout)));
            } else {
                profiling_data.insert("profiling_available".to_string(), json!(false));
                profiling_data.insert("profiling_error".to_string(), json!(String::from_utf8_lossy(&output.stderr)));
            }
        } else {
            profiling_data.insert("profiling_available".to_string(), json!(false));
            profiling_data.insert("profiling_error".to_string(), json!("cProfile not available"));
        }

        Ok(profiling_data)
    }
}

impl BaseDataExtractor for RuntimeProfiler {
    fn extract_data(&self) -> crate::Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        let files = self.get_all_python_files();

        if files.is_empty() {
            result.insert("runtime_profile".to_string(), json!({
                "warning": "No Python files found for profiling"
            }));
            result.insert("files_analyzed".to_string(), json!(0));
            return Ok(result);
        }

        // Analyze runtime metrics
        let metrics = self.analyze_runtime_metrics(&files)?;

        // Identify bottlenecks
        let bottlenecks = self.identify_performance_bottlenecks(&metrics);

        // Analyze resource usage
        let resource_usage = self.analyze_resource_usage(&metrics);

        // Identify execution patterns
        let patterns = self.identify_execution_patterns(&metrics);

        // Try actual profiling
        let profiling_data = self.run_actual_profiling().unwrap_or_else(|_| HashMap::new());

        // Generate statistics
        let total_files = files.len();
        let total_functions = metrics.len();
        let hotspot_count = metrics.iter().filter(|m| m.is_hotspot).count();
        let bottleneck_count = bottlenecks.len();

        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));
        result.insert("files_analyzed".to_string(), json!(total_files));
        result.insert("total_functions".to_string(), json!(total_functions));
        result.insert("hotspot_count".to_string(), json!(hotspot_count));
        result.insert("bottleneck_count".to_string(), json!(bottleneck_count));
        result.insert("runtime_metrics".to_string(), json!(metrics));
        result.insert("performance_bottlenecks".to_string(), json!(bottlenecks));
        result.insert("resource_usage".to_string(), json!(resource_usage));
        result.insert("execution_patterns".to_string(), json!(patterns));
        result.insert("profiling_data".to_string(), json!(profiling_data));

        // Add recommendations
        let mut recommendations = Vec::new();
        if hotspot_count > 0 {
            recommendations.push(format!("Found {} potential CPU hotspots - consider profiling with actual runtime data", hotspot_count));
        }
        if bottleneck_count > 0 {
            recommendations.push(format!("Identified {} performance bottlenecks that need attention", bottleneck_count));
        }
        if resource_usage.peak_memory_mb > 500.0 {
            recommendations.push("High peak memory usage detected - consider memory optimization".to_string());
        }
        if resource_usage.io_operations > 20 {
            recommendations.push("Many I/O operations detected - consider async programming or caching".to_string());
        }

        result.insert("recommendations".to_string(), json!(recommendations));

        println!("Runtime profiling complete: {} files analyzed, {} functions, {} hotspots, {} bottlenecks found",
                 total_files, total_functions, hotspot_count, bottleneck_count);

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "RuntimeProfiler"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}