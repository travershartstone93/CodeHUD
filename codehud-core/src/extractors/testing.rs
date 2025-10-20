//! Testing Data Extractor - Analyzes test coverage, patterns, and quality

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

extern "C" {
    fn tree_sitter_rust() -> Language;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestFile {
    file_path: String,
    test_count: usize,
    test_functions: Vec<String>,
    setup_functions: Vec<String>,
    teardown_functions: Vec<String>,
    test_classes: Vec<String>,
    assertion_count: usize,
    mock_usage: bool,
    fixture_usage: bool,
    parametrized_tests: usize,
    skip_count: usize,
    lines_of_code: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestCoverage {
    file_path: String,
    lines_covered: usize,
    lines_total: usize,
    coverage_percentage: f64,
    branches_covered: usize,
    branches_total: usize,
    branch_coverage_percentage: f64,
    missing_lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestPattern {
    pattern_type: String,
    description: String,
    files_with_pattern: Vec<String>,
    severity: String, // "good", "warning", "error"
    recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestMetrics {
    total_test_files: usize,
    total_tests: usize,
    test_to_code_ratio: f64,
    average_tests_per_file: f64,
    test_coverage_percentage: f64,
    files_without_tests: Vec<String>,
    slow_tests: Vec<String>,
    flaky_tests: Vec<String>,
}

pub struct TestingExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
}

impl TestingExtractor {
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

    fn get_all_python_files(&self) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut all_files = Vec::new();
        let mut test_files = Vec::new();

        self.collect_files_recursive(&self.codebase_path, &mut all_files, &mut test_files);
        (all_files, test_files)
    }

    fn collect_files_recursive(&self, dir: &Path, all_files: &mut Vec<PathBuf>, test_files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                    all_files.push(path.clone());

                    if self.is_test_file(&path) {
                        test_files.push(path);
                    }
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    self.collect_files_recursive(&path, all_files, test_files);
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

    fn is_test_file(&self, path: &Path) -> bool {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            file_name.starts_with("test_") ||
            file_name.ends_with("_test.py") ||
            path.display().to_string().contains("/tests/") ||
            path.display().to_string().contains("\\tests\\")
        } else {
            false
        }
    }

    fn analyze_test_file(&self, file_path: &Path) -> crate::Result<TestFile> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io(e))?;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Analysis(format!("Failed to set language: {}", e)))?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Analysis("Failed to parse file".to_string()))?;

        let mut test_functions = Vec::new();
        let mut setup_functions = Vec::new();
        let mut teardown_functions = Vec::new();
        let mut test_classes = Vec::new();
        let mut assertion_count = 0;
        let mut mock_usage = false;
        let mut fixture_usage = false;
        let mut parametrized_tests = 0;
        let mut skip_count = 0;

        self.extract_test_elements(
            tree.root_node(),
            &content,
            &mut test_functions,
            &mut setup_functions,
            &mut teardown_functions,
            &mut test_classes,
            &mut assertion_count,
            &mut mock_usage,
            &mut fixture_usage,
            &mut parametrized_tests,
            &mut skip_count,
        );

        let lines_of_code = content.lines().filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#')).count();

        Ok(TestFile {
            file_path: file_path.display().to_string(),
            test_count: test_functions.len(),
            test_functions,
            setup_functions,
            teardown_functions,
            test_classes,
            assertion_count,
            mock_usage,
            fixture_usage,
            parametrized_tests,
            skip_count,
            lines_of_code,
        })
    }

    fn extract_test_elements(
        &self,
        node: tree_sitter::Node,
        source: &str,
        test_functions: &mut Vec<String>,
        setup_functions: &mut Vec<String>,
        teardown_functions: &mut Vec<String>,
        test_classes: &mut Vec<String>,
        assertion_count: &mut usize,
        mock_usage: &mut bool,
        fixture_usage: &mut bool,
        parametrized_tests: &mut usize,
        skip_count: &mut usize,
    ) {
        match node.kind() {
            "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let func_name = &source[name_node.start_byte()..name_node.end_byte()];

                    if func_name.starts_with("test_") {
                        test_functions.push(func_name.to_string());

                        // Check for decorators
                        let func_source = &source[node.start_byte()..node.end_byte()];
                        if func_source.contains("@pytest.mark.parametrize") {
                            *parametrized_tests += 1;
                        }
                        if func_source.contains("@pytest.mark.skip") || func_source.contains("@unittest.skip") {
                            *skip_count += 1;
                        }
                        if func_source.contains("@pytest.fixture") {
                            *fixture_usage = true;
                        }
                    } else if func_name == "setUp" || func_name == "setup" || func_name.starts_with("setup_") {
                        setup_functions.push(func_name.to_string());
                    } else if func_name == "tearDown" || func_name == "teardown" || func_name.starts_with("teardown_") {
                        teardown_functions.push(func_name.to_string());
                    }
                }
            }
            "struct_item" | "enum_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let class_name = &source[name_node.start_byte()..name_node.end_byte()];
                    if class_name.starts_with("Test") || class_name.ends_with("Test") {
                        test_classes.push(class_name.to_string());
                    }
                }
            }
            "call" => {
                if let Some(function_node) = node.child_by_field_name("function") {
                    let call_text = &source[function_node.start_byte()..function_node.end_byte()];

                    // Count assertions
                    if call_text.contains("assert") || call_text.starts_with("self.assert") {
                        *assertion_count += 1;
                    }

                    // Check for mock usage
                    if call_text.contains("mock") || call_text.contains("Mock") || call_text.contains("patch") {
                        *mock_usage = true;
                    }
                }
            }
            _ => {}
        }

        // Recursively process child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.extract_test_elements(
                    child,
                    source,
                    test_functions,
                    setup_functions,
                    teardown_functions,
                    test_classes,
                    assertion_count,
                    mock_usage,
                    fixture_usage,
                    parametrized_tests,
                    skip_count,
                );
            }
        }
    }

    fn get_test_coverage(&self) -> crate::Result<Vec<TestCoverage>> {
        // Try to get coverage from pytest-cov or coverage.py
        let coverage_output = Command::new("python")
            .args(&["-m", "coverage", "report", "--show-missing", "--format=json"])
            .current_dir(&self.codebase_path)
            .output();

        let mut coverages = Vec::new();

        if let Ok(output) = coverage_output {
            if output.status.success() {
                let coverage_json = String::from_utf8_lossy(&output.stdout);
                if let Ok(coverage_data) = serde_json::from_str::<Value>(&coverage_json) {
                    if let Some(files) = coverage_data["files"].as_object() {
                        for (file_path, file_data) in files {
                            let lines_covered = file_data["summary"]["covered_lines"].as_u64().unwrap_or(0) as usize;
                            let lines_total = file_data["summary"]["num_statements"].as_u64().unwrap_or(0) as usize;
                            let coverage_percentage = if lines_total > 0 {
                                lines_covered as f64 / lines_total as f64 * 100.0
                            } else {
                                0.0
                            };

                            let branches_covered = file_data["summary"]["covered_branches"].as_u64().unwrap_or(0) as usize;
                            let branches_total = file_data["summary"]["num_branches"].as_u64().unwrap_or(0) as usize;
                            let branch_coverage_percentage = if branches_total > 0 {
                                branches_covered as f64 / branches_total as f64 * 100.0
                            } else {
                                100.0 // No branches = 100% coverage
                            };

                            let missing_lines = file_data["missing_lines"]
                                .as_array()
                                .unwrap_or(&Vec::new())
                                .iter()
                                .filter_map(|v| v.as_u64().map(|n| n as usize))
                                .collect();

                            coverages.push(TestCoverage {
                                file_path: file_path.clone(),
                                lines_covered,
                                lines_total,
                                coverage_percentage,
                                branches_covered,
                                branches_total,
                                branch_coverage_percentage,
                                missing_lines,
                            });
                        }
                    }
                }
            }
        }

        Ok(coverages)
    }

    fn identify_test_patterns(&self, test_files: &[TestFile]) -> Vec<TestPattern> {
        let mut patterns = Vec::new();

        // Pattern 1: Tests without assertions
        let tests_without_assertions: Vec<String> = test_files
            .iter()
            .filter(|tf| tf.test_count > 0 && tf.assertion_count == 0)
            .map(|tf| tf.file_path.clone())
            .collect();

        if !tests_without_assertions.is_empty() {
            patterns.push(TestPattern {
                pattern_type: "tests_without_assertions".to_string(),
                description: "Test files with test functions but no assertions".to_string(),
                files_with_pattern: tests_without_assertions,
                severity: "warning".to_string(),
                recommendation: "Add assertions to verify expected behavior".to_string(),
            });
        }

        // Pattern 2: Large test files
        let large_test_files: Vec<String> = test_files
            .iter()
            .filter(|tf| tf.test_count > 20)
            .map(|tf| tf.file_path.clone())
            .collect();

        if !large_test_files.is_empty() {
            patterns.push(TestPattern {
                pattern_type: "large_test_files".to_string(),
                description: "Test files with many test functions".to_string(),
                files_with_pattern: large_test_files,
                severity: "warning".to_string(),
                recommendation: "Consider splitting large test files into smaller, focused ones".to_string(),
            });
        }

        // Pattern 3: Good use of mocks
        let files_with_mocks: Vec<String> = test_files
            .iter()
            .filter(|tf| tf.mock_usage)
            .map(|tf| tf.file_path.clone())
            .collect();

        if !files_with_mocks.is_empty() {
            patterns.push(TestPattern {
                pattern_type: "mock_usage".to_string(),
                description: "Test files using mocks appropriately".to_string(),
                files_with_pattern: files_with_mocks,
                severity: "good".to_string(),
                recommendation: "Good use of mocking for isolated testing".to_string(),
            });
        }

        // Pattern 4: Parametrized tests
        let parametrized_files: Vec<String> = test_files
            .iter()
            .filter(|tf| tf.parametrized_tests > 0)
            .map(|tf| tf.file_path.clone())
            .collect();

        if !parametrized_files.is_empty() {
            patterns.push(TestPattern {
                pattern_type: "parametrized_tests".to_string(),
                description: "Test files using parametrized tests".to_string(),
                files_with_pattern: parametrized_files,
                severity: "good".to_string(),
                recommendation: "Good use of parametrized tests for comprehensive coverage".to_string(),
            });
        }

        patterns
    }

    fn find_files_without_tests(&self, all_files: &[PathBuf], test_files: &[PathBuf]) -> Vec<String> {
        let test_file_names: HashSet<String> = test_files
            .iter()
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
            .map(|s| s.replace("test_", "").replace("_test", ""))
            .collect();

        let mut files_without_tests = Vec::new();

        for file_path in all_files {
            if self.is_test_file(file_path) {
                continue; // Skip test files themselves
            }

            if let Some(file_stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                if !test_file_names.contains(file_stem) {
                    // Check if it's a main module or important file
                    if !file_stem.starts_with('_') && file_stem != "__init__" {
                        files_without_tests.push(file_path.display().to_string());
                    }
                }
            }
        }

        files_without_tests
    }

    fn calculate_test_metrics(&self, all_files: &[PathBuf], test_files: &[TestFile], coverages: &[TestCoverage]) -> TestMetrics {
        let total_test_files = test_files.len();
        let total_tests: usize = test_files.iter().map(|tf| tf.test_count).sum();
        let total_code_files = all_files.len() - total_test_files;

        let test_to_code_ratio = if total_code_files > 0 {
            total_test_files as f64 / total_code_files as f64
        } else {
            0.0
        };

        let average_tests_per_file = if total_test_files > 0 {
            total_tests as f64 / total_test_files as f64
        } else {
            0.0
        };

        let test_coverage_percentage = if !coverages.is_empty() {
            let total_covered: usize = coverages.iter().map(|c| c.lines_covered).sum();
            let total_lines: usize = coverages.iter().map(|c| c.lines_total).sum();
            if total_lines > 0 {
                total_covered as f64 / total_lines as f64 * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        let files_without_tests = self.find_files_without_tests(all_files, &test_files.iter().map(|tf| PathBuf::from(&tf.file_path)).collect::<Vec<_>>());

        // Simplified detection for slow/flaky tests
        let slow_tests = test_files
            .iter()
            .filter(|tf| tf.lines_of_code > 200) // Heuristic for potentially slow tests
            .map(|tf| tf.file_path.clone())
            .collect();

        let flaky_tests = test_files
            .iter()
            .filter(|tf| tf.skip_count > tf.test_count / 4) // Many skipped tests might indicate flakiness
            .map(|tf| tf.file_path.clone())
            .collect();

        TestMetrics {
            total_test_files,
            total_tests,
            test_to_code_ratio,
            average_tests_per_file,
            test_coverage_percentage,
            files_without_tests,
            slow_tests,
            flaky_tests,
        }
    }
}

impl BaseDataExtractor for TestingExtractor {
    fn extract_data(&self) -> crate::Result<HashMap<String, Value>> {
        let mut result = HashMap::new();

        let (all_files, test_file_paths) = self.get_all_python_files();

        if test_file_paths.is_empty() {
            result.insert("test_analysis".to_string(), json!({
                "warning": "No test files found in the codebase"
            }));
            result.insert("files_analyzed".to_string(), json!(0));
            return Ok(result);
        }

        // Analyze test files
        let mut test_files = Vec::new();
        for test_file_path in &test_file_paths {
            if let Ok(test_file) = self.analyze_test_file(test_file_path) {
                test_files.push(test_file);
            }
        }

        // Get test coverage
        let coverages = self.get_test_coverage().unwrap_or_else(|_| Vec::new());

        // Identify patterns
        let patterns = self.identify_test_patterns(&test_files);

        // Calculate metrics
        let metrics = self.calculate_test_metrics(&all_files, &test_files, &coverages);

        // Generate statistics
        let total_files_analyzed = all_files.len();
        let test_files_count = test_files.len();
        let total_tests: usize = test_files.iter().map(|tf| tf.test_count).sum();
        let pattern_count = patterns.len();

        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));
        result.insert("files_analyzed".to_string(), json!(total_files_analyzed));
        result.insert("test_files_count".to_string(), json!(test_files_count));
        result.insert("total_tests".to_string(), json!(total_tests));
        result.insert("pattern_count".to_string(), json!(pattern_count));
        result.insert("test_files".to_string(), json!(test_files));
        result.insert("test_coverage".to_string(), json!(coverages));
        result.insert("test_patterns".to_string(), json!(patterns));
        result.insert("test_metrics".to_string(), json!(metrics));

        // Add recommendations
        let mut recommendations = Vec::new();
        if metrics.test_coverage_percentage < 70.0 {
            recommendations.push(format!("Test coverage is {:.1}% - consider increasing to at least 70%", metrics.test_coverage_percentage));
        }
        if metrics.test_to_code_ratio < 0.5 {
            recommendations.push("Low test-to-code ratio - consider adding more test files".to_string());
        }
        if !metrics.files_without_tests.is_empty() {
            recommendations.push(format!("{} files lack corresponding tests", metrics.files_without_tests.len()));
        }
        if !metrics.flaky_tests.is_empty() {
            recommendations.push(format!("{} test files may be flaky - review skipped tests", metrics.flaky_tests.len()));
        }

        result.insert("recommendations".to_string(), json!(recommendations));

        println!("Testing analysis complete: {} files analyzed, {} test files, {} total tests, {:.1}% coverage",
                 total_files_analyzed, test_files_count, total_tests, metrics.test_coverage_percentage);

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "TestingExtractor"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}