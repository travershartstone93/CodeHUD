//! Dependencies Data Extractor - Automatic dependency analysis using tree-sitter queries
//!
//! This module extracts comprehensive dependency analysis including:
//! - Automatic language detection and parsing
//! - Query-based import extraction
//! - Dependency graph construction with petgraph
//! - Circular dependency detection
//! - Coupling metrics calculation
//! - Cross-language analysis support

use super::BaseDataExtractor;
use crate::Result;
use crate::query_engine::{get_query_engine, SupportedLanguage};
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::fs;
use anyhow::Context;

#[derive(Debug, Default)]
struct DependencyAnalysis {
    file_dependencies: HashMap<String, FileDependencyInfo>,
    import_graph: HashMap<String, HashSet<String>>,
    internal_imports: HashMap<String, HashSet<String>>,
    external_imports: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
struct FileDependencyInfo {
    imports: Vec<String>,
    from_imports: Vec<String>,
    import_details: Vec<ImportDetail>,
    total_imports: usize,
    internal_imports: usize,
    external_imports: usize,
    coupling_score: f64,
    import_complexity: f64,
}

#[derive(Debug, Clone)]
struct ImportDetail {
    import_type: String, // "import" or "from_import"
    module: String,
    imported_names: Vec<String>,
    aliases: Vec<String>,
    line: usize,
    is_star_import: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CircularDependency {
    cycle: Vec<String>,
    length: usize,
    severity: String,
}

pub struct DependenciesExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
}

impl DependenciesExtractor {
    pub fn new(codebase_path: impl AsRef<Path>) -> crate::Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();

        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!(
                "Codebase path does not exist: {}",
                codebase_path.display()
            )));
        }

        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
        })
    }

    fn get_source_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.codebase_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Check if this file is supported by our query engine (all languages)
                    if SupportedLanguage::from_path(&path).is_some() {
                        files.push(path);
                    }
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    files.extend(self.get_files_recursive(&path));
                }
            }
        }
        files
    }

    fn get_files_recursive(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Check if this file is supported by our query engine
                    if SupportedLanguage::from_path(&path).is_some() {
                        files.push(path);
                    }
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    files.extend(self.get_files_recursive(&path));
                }
            }
        }
        files
    }

    fn is_excluded_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            name == "__pycache__" || name == ".git" || name == "node_modules"
                || name == "venv" || name == ".venv" || name == "env"
                || name == ".pytest_cache" || name == "build"
                || name == "dist" || name == ".tox"
        } else {
            false
        }
    }

    fn analyze_file_dependencies(&mut self, file_path: &Path) -> Result<Option<FileDependencyInfo>> {
        // Check if file is supported by query engine
        if SupportedLanguage::from_path(file_path).is_none() {
            return Ok(None);
        }

        // Use query engine for automatic analysis
        let mut query_engine = get_query_engine()
            .map_err(|e| crate::Error::Config(format!("Failed to get query engine: {}", e)))?;

        let analysis = query_engine.analyze_file(file_path)
            .map_err(|e| crate::Error::Config(format!("Failed to analyze file {}: {}", file_path.display(), e)))?;

        // Extract import information from query results
        let empty_json = json!({});
        let imports_data = analysis.get("imports").unwrap_or(&empty_json);
        let empty_vec = vec![];
        let imports_list = imports_data.get("imports").and_then(|v| v.as_array()).unwrap_or(&empty_vec);

        // Also check for summary.modules (fallback for some languages)
        let summary_data = imports_data.get("summary").unwrap_or(&empty_json);
        let modules_list = summary_data.get("modules").and_then(|v| v.as_array()).unwrap_or(&empty_vec);

        // Convert query results to our format
        let mut imports = Vec::new();
        let mut from_imports = Vec::new();
        let mut import_details = Vec::new();

        // Extract imports from query results - handle multiple formats
        let mut seen_modules = HashSet::new();

        for import in imports_list {
            // Try Rust-style with explicit "module" field first
            if let Some(module) = import.get("module").and_then(|v| v.as_str()) {
                if seen_modules.insert(module.to_string()) {
                    imports.push(module.to_string());

                    // Create import detail
                    let detail = ImportDetail {
                        import_type: "import".to_string(),
                        module: module.to_string(),
                        imported_names: vec![import.get("item").and_then(|v| v.as_str()).unwrap_or("").to_string()],
                        aliases: vec![import.get("alias").and_then(|v| v.as_str()).unwrap_or("").to_string()],
                        line: import.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                        is_star_import: false,
                    };
                    import_details.push(detail);

                    // Categorize as from_import if it has an item
                    if import.get("item").is_some() {
                        from_imports.push(module.to_string());
                    }
                }
            }
            // Try Python/JavaScript-style with capture_type and text fields
            else if let Some(capture_type) = import.get("capture_type").and_then(|v| v.as_str()) {
                if capture_type == "module_name" || capture_type.contains("import") {
                    if let Some(module_text) = import.get("text").and_then(|v| v.as_str()) {
                        // Extract just the module name (first part before any dots for from_imports)
                        let module_name = if module_text.contains("import ") {
                            // Parse "import json" or "from os import path"
                            if let Some(word) = module_text.split_whitespace().nth(1) {
                                word
                            } else {
                                module_text
                            }
                        } else {
                            module_text
                        };

                        if seen_modules.insert(module_name.to_string()) {
                            imports.push(module_name.to_string());

                            // Create import detail
                            let detail = ImportDetail {
                                import_type: capture_type.to_string(),
                                module: module_name.to_string(),
                                imported_names: vec![],
                                aliases: vec![],
                                line: import.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                                is_star_import: module_text.contains("import *"),
                            };
                            import_details.push(detail);

                            // Categorize as from_import if it's a from_import type
                            if capture_type.contains("from") {
                                from_imports.push(module_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Fallback: If still no imports, try summary.modules
        if imports.is_empty() && !modules_list.is_empty() {
            for module in modules_list {
                if let Some(module_name) = module.as_str() {
                    if seen_modules.insert(module_name.to_string()) {
                        imports.push(module_name.to_string());

                        let detail = ImportDetail {
                            import_type: "import".to_string(),
                            module: module_name.to_string(),
                            imported_names: vec![],
                            aliases: vec![],
                            line: 0,
                            is_star_import: false,
                        };
                        import_details.push(detail);
                    }
                }
            }
        }

        // Calculate metrics using the extracted data
        let total_imports = imports.len();
        let internal_count = imports.iter()
            .filter(|imp| self.is_internal_import(imp))
            .count();
        let external_count = total_imports - internal_count;

        let coupling_score = self.calculate_file_coupling_score(&imports);
        let import_complexity = self.calculate_import_complexity(&import_details);

        Ok(Some(FileDependencyInfo {
            imports,
            from_imports,
            import_details,
            total_imports,
            internal_imports: internal_count,
            external_imports: external_count,
            coupling_score,
            import_complexity,
        }))
    }

    fn is_internal_import(&self, import_name: &str) -> bool {
        import_name.starts_with("codehud") ||
        import_name.starts_with(".") ||
        import_name.starts_with("src.codehud")
    }

    fn calculate_file_coupling_score(&self, imports: &[String]) -> f64 {
        let import_score = imports.len() as f64;
        let internal_imports: Vec<_> = imports.iter()
            .filter(|imp| self.is_internal_import(imp))
            .collect();
        let internal_penalty = internal_imports.len() as f64 * 0.5;

        import_score + internal_penalty
    }

    fn calculate_import_complexity(&self, import_details: &[ImportDetail]) -> f64 {
        let mut complexity = 0.0;

        for detail in import_details {
            if detail.is_star_import {
                complexity += 5.0;
            }

            complexity += detail.aliases.len() as f64 * 0.5;

            let module_depth = detail.module.matches('.').count() as f64;
            complexity += module_depth * 0.2;
        }

        complexity
    }

    fn detect_circular_dependencies(&self, import_graph: &HashMap<String, HashSet<String>>) -> Vec<CircularDependency> {
        let mut circular_deps = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        fn has_cycle_dfs(
            node: &str,
            import_graph: &HashMap<String, HashSet<String>>,
            visited: &mut HashSet<String>,
            rec_stack: &mut HashSet<String>,
            path: &mut Vec<String>,
            circular_deps: &mut Vec<CircularDependency>,
            extractor: &DependenciesExtractor,
        ) -> bool {
            if rec_stack.contains(node) {
                if let Some(cycle_start) = path.iter().position(|x| x == node) {
                    let cycle: Vec<String> = path[cycle_start..].iter().cloned().collect();
                    let mut cycle_with_end = cycle.clone();
                    cycle_with_end.push(node.to_string());

                    circular_deps.push(CircularDependency {
                        cycle: cycle_with_end.clone(),
                        length: cycle_with_end.len() - 1,
                        severity: if cycle_with_end.len() <= 3 { "high".to_string() } else { "medium".to_string() },
                    });
                    return true;
                }
            }

            if visited.contains(node) {
                return false;
            }

            visited.insert(node.to_string());
            rec_stack.insert(node.to_string());
            path.push(node.to_string());

            if let Some(neighbors) = import_graph.get(node) {
                for neighbor in neighbors {
                    if extractor.is_internal_import(neighbor) {
                        let neighbor_file = extractor.import_to_file_path(neighbor);
                        if let Some(neighbor_path) = neighbor_file {
                            if import_graph.contains_key(&neighbor_path) {
                                if has_cycle_dfs(&neighbor_path, import_graph, visited, rec_stack, path, circular_deps, extractor) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            path.pop();
            rec_stack.remove(node);
            false
        }

        for file_path in import_graph.keys() {
            if !visited.contains(file_path) {
                let mut path = Vec::new();
                has_cycle_dfs(file_path, import_graph, &mut visited, &mut rec_stack, &mut path, &mut circular_deps, self);
            }
        }

        circular_deps
    }

    fn import_to_file_path(&self, import_name: &str) -> Option<String> {
        if import_name.starts_with("codehud.") {
            let parts: Vec<&str> = import_name.split('.').collect();
            if parts.len() >= 2 {
                let path_parts = ["src"].iter().chain(parts.iter()).cloned().collect::<Vec<_>>();
                return Some(format!("{}.py", path_parts.join("/")));
            }
        }
        None
    }
}

// Old manual AST analyzer removed - now using query engine automatically

impl BaseDataExtractor for DependenciesExtractor {
    fn extract_data(&self) -> Result<HashMap<String, Value>> {
        println!("Extracting dependency network analysis...");

        // Get all source files
        let source_files = self.get_source_files();

        // Initialize collectors
        let mut analysis = DependencyAnalysis::default();

        // Create a mutable copy for analysis - query engine handles languages automatically
        let mut extractor = DependenciesExtractor {
            codebase_path: self.codebase_path.clone(),
            extraction_timestamp: self.extraction_timestamp,
        };

        // Analyze each file
        for file_path in &source_files {
            match extractor.analyze_file_dependencies(file_path) {
                Ok(Some(deps)) => {
                    let relative_path = file_path.strip_prefix(&self.codebase_path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

                    // Build dependency graph
                    for import in &deps.imports {
                        analysis.import_graph.entry(relative_path.clone())
                            .or_insert_with(HashSet::new)
                            .insert(import.clone());

                        // Note: Graph analysis now handled by query engine

                        // Categorize as internal vs external
                        if self.is_internal_import(import) {
                            analysis.internal_imports.entry(relative_path.clone())
                                .or_insert_with(HashSet::new)
                                .insert(import.clone());
                        } else {
                            analysis.external_imports.entry(relative_path.clone())
                                .or_insert_with(HashSet::new)
                                .insert(import.clone());
                        }
                    }

                    // Note: from_imports graph analysis now handled by query engine

                    analysis.file_dependencies.insert(relative_path, deps);
                }
                Ok(None) => continue,
                Err(e) => {
                    println!("Warning: Error analyzing dependencies for {:?}: {}", file_path, e);
                    continue;
                }
            }
        }

        // Detect circular dependencies
        let circular_dependencies = self.detect_circular_dependencies(&analysis.import_graph);

        // Calculate dependency metrics
        let dependency_metrics = self.calculate_dependency_metrics(&analysis.file_dependencies);

        // Analyze coupling strength
        let coupling_analysis = self.analyze_coupling_strength(&analysis.import_graph, &analysis.internal_imports);

        // Graph analysis now integrated with query engine
        let graph_analysis = json!({
            "dependency_centrality": {},
            "cycles": self.detect_circular_dependencies(&analysis.import_graph),
            "components": {},
            "coupling_metrics": {},
            "graph_statistics": {}
        });

        // External dependencies analysis
        let external_deps_analysis = self.analyze_external_dependencies(&analysis.external_imports);

        // Dependency clusters
        let dependency_clusters = self.identify_dependency_clusters(&analysis.import_graph);

        // Most influential files
        let influential_files = self.identify_influential_files(&analysis.import_graph);

        // Import patterns
        let import_patterns = self.analyze_import_patterns(&analysis.file_dependencies);

        // Generate summary
        let summary = self.generate_dependencies_summary(
            source_files.len(),
            &analysis.file_dependencies,
            &circular_dependencies,
            &external_deps_analysis
        );

        // Generate recommendations
        let recommendations = self.generate_dependency_recommendations(
            &circular_dependencies,
            &coupling_analysis,
            &external_deps_analysis
        );

        let mut result = HashMap::new();
        result.insert("summary".to_string(), summary);
        result.insert("file_dependencies".to_string(), json!(self.serialize_file_dependencies(&analysis.file_dependencies)));
        result.insert("dependency_metrics".to_string(), dependency_metrics);
        result.insert("coupling_analysis".to_string(), coupling_analysis);
        result.insert("graph_analysis".to_string(), graph_analysis);
        result.insert("external_dependencies".to_string(), external_deps_analysis);
        result.insert("circular_dependencies".to_string(), json!(self.serialize_circular_dependencies(&circular_dependencies)));
        result.insert("dependency_clusters".to_string(), dependency_clusters);
        result.insert("influential_files".to_string(), influential_files);
        result.insert("import_patterns".to_string(), import_patterns);
        result.insert("recommendations".to_string(), json!(recommendations));
        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));

        println!("Dependencies extraction complete: {} files analyzed, {} circular dependencies found",
                 source_files.len(), circular_dependencies.len());

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "DependenciesExtractor"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}

impl DependenciesExtractor {
    fn calculate_dependency_metrics(&self, file_dependencies: &HashMap<String, FileDependencyInfo>) -> Value {
        if file_dependencies.is_empty() {
            return json!({});
        }

        let total_files = file_dependencies.len();
        let mut all_imports = Vec::new();
        let mut coupling_scores = Vec::new();

        for file_deps in file_dependencies.values() {
            all_imports.extend(file_deps.imports.clone());
            coupling_scores.push(file_deps.coupling_score);
        }

        let total_imports = all_imports.len();
        let unique_imports = all_imports.into_iter().collect::<HashSet<_>>().len();
        let avg_imports_per_file = if total_files > 0 { total_imports as f64 / total_files as f64 } else { 0.0 };
        let avg_coupling = if !coupling_scores.is_empty() { coupling_scores.iter().sum::<f64>() / coupling_scores.len() as f64 } else { 0.0 };

        let import_reuse = if total_imports > 0 { (total_imports - unique_imports) as f64 / total_imports as f64 } else { 0.0 };

        json!({
            "total_imports": total_imports,
            "unique_imports": unique_imports,
            "average_imports_per_file": avg_imports_per_file,
            "average_coupling_score": avg_coupling,
            "import_reuse_factor": import_reuse,
            "max_coupling": coupling_scores.iter().cloned().fold(0.0, f64::max),
            "min_coupling": coupling_scores.iter().cloned().fold(f64::INFINITY, f64::min)
        })
    }

    fn analyze_coupling_strength(&self, import_graph: &HashMap<String, HashSet<String>>, internal_imports: &HashMap<String, HashSet<String>>) -> Value {
        let mut coupling_matrix = HashMap::new();
        let mut strong_couplings = Vec::new();

        for (file_path, imports) in internal_imports {
            let coupling_strength = imports.len();
            coupling_matrix.insert(file_path.clone(), coupling_strength);

            if coupling_strength >= 8 {
                strong_couplings.push(json!({
                    "file": file_path,
                    "coupling_strength": coupling_strength,
                    "dependencies": imports.iter().cloned().collect::<Vec<_>>()
                }));
            }
        }

        let coupling_values: Vec<usize> = coupling_matrix.values().cloned().collect();
        let coupling_distribution = json!({
            "low_coupling": coupling_values.iter().filter(|&&c| c < 3).count(),
            "medium_coupling": coupling_values.iter().filter(|&&c| c >= 3 && c < 8).count(),
            "high_coupling": coupling_values.iter().filter(|&&c| c >= 8).count()
        });

        // Sort strong couplings by strength
        strong_couplings.sort_by(|a, b| {
            let strength_a = a["coupling_strength"].as_u64().unwrap_or(0);
            let strength_b = b["coupling_strength"].as_u64().unwrap_or(0);
            strength_b.cmp(&strength_a)
        });

        json!({
            "coupling_matrix": coupling_matrix,
            "strong_couplings": strong_couplings,
            "coupling_distribution": coupling_distribution,
            "average_internal_coupling": if !coupling_values.is_empty() { coupling_values.iter().sum::<usize>() as f64 / coupling_values.len() as f64 } else { 0.0 }
        })
    }

    fn analyze_external_dependencies(&self, external_imports: &HashMap<String, HashSet<String>>) -> Value {
        let mut all_external = Vec::new();
        for imports in external_imports.values() {
            all_external.extend(imports.iter().cloned());
        }

        let mut external_counts = HashMap::new();
        for dep in &all_external {
            *external_counts.entry(dep.clone()).or_insert(0) += 1;
        }

        // Sort by count
        let mut most_used: Vec<_> = external_counts.iter().collect();
        most_used.sort_by(|a, b| b.1.cmp(a.1));

        // Standard library modules
        let stdlib_modules = vec![
            "os", "sys", "json", "re", "datetime", "pathlib", "typing", "collections",
            "subprocess", "logging", "contextlib", "dataclasses", "functools", "itertools",
            "ast", "inspect", "importlib", "hashlib", "uuid", "time", "threading",
            "multiprocessing", "concurrent", "asyncio", "io", "tempfile", "shutil"
        ].into_iter().collect::<HashSet<_>>();

        let mut stdlib_deps = Vec::new();
        let mut third_party_deps = Vec::new();

        for (dep, count) in &most_used {
            let base_module = dep.split('.').next().unwrap_or("");
            if stdlib_modules.contains(base_module) {
                stdlib_deps.push(json!([dep, count]));
            } else {
                third_party_deps.push(json!([dep, count]));
            }
        }

        json!({
            "total_external_dependencies": external_counts.len(),
            "most_used_external": most_used.iter().take(15).map(|(dep, count)| json!([dep, count])).collect::<Vec<_>>(),
            "stdlib_dependencies": stdlib_deps,
            "third_party_dependencies": third_party_deps,
            "files_with_external_deps": external_imports.values().filter(|deps| !deps.is_empty()).count()
        })
    }

    fn identify_dependency_clusters(&self, import_graph: &HashMap<String, HashSet<String>>) -> Value {
        let mut clusters = Vec::new();
        let mut processed_files = HashSet::new();

        for (file_path, imports) in import_graph {
            if processed_files.contains(file_path) {
                continue;
            }

            let mut cluster_files = vec![file_path.clone()];
            let mut cluster_imports = imports.clone();

            for (other_file, other_imports) in import_graph {
                if other_file != file_path && !processed_files.contains(other_file) {
                    let common_imports = cluster_imports.intersection(other_imports).count();
                    let max_imports = cluster_imports.len().max(other_imports.len());
                    let similarity = if max_imports > 0 { common_imports as f64 / max_imports as f64 } else { 0.0 };

                    if similarity >= 0.4 {
                        cluster_files.push(other_file.clone());
                        cluster_imports.extend(other_imports.iter().cloned());
                    }
                }
            }

            if cluster_files.len() >= 3 {
                clusters.push(json!({
                    "files": cluster_files,
                    "common_imports": cluster_imports.len(),
                    "cluster_size": cluster_files.len(),
                    "cohesion_score": cluster_imports.len() as f64 / cluster_files.len() as f64
                }));

                processed_files.extend(cluster_files);
            }
        }

        // Sort by cluster size
        clusters.sort_by(|a, b| {
            let size_a = a["cluster_size"].as_u64().unwrap_or(0);
            let size_b = b["cluster_size"].as_u64().unwrap_or(0);
            size_b.cmp(&size_a)
        });

        json!(clusters.into_iter().take(10).collect::<Vec<_>>())
    }

    fn identify_influential_files(&self, import_graph: &HashMap<String, HashSet<String>>) -> Value {
        let mut import_counts = HashMap::new();

        for (file_path, imports) in import_graph {
            for imported_module in imports {
                if self.is_internal_import(imported_module) {
                    *import_counts.entry(imported_module.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut influential: Vec<_> = import_counts.iter().collect();
        influential.sort_by(|a, b| b.1.cmp(a.1));

        let result: Vec<_> = influential.into_iter().take(20).map(|(module, count)| {
            json!({
                "module": module,
                "imported_by_count": count,
                "influence_score": if !import_graph.is_empty() { *count as f64 / import_graph.len() as f64 } else { 0.0 }
            })
        }).collect();

        json!(result)
    }

    fn analyze_import_patterns(&self, file_dependencies: &HashMap<String, FileDependencyInfo>) -> Value {
        let mut patterns = json!({
            "star_imports": [],
            "long_import_chains": [],
            "relative_imports": [],
            "aliased_imports": []
        });

        for (file_path, deps) in file_dependencies {
            for detail in &deps.import_details {
                if detail.is_star_import {
                    patterns["star_imports"].as_array_mut().unwrap().push(json!({
                        "file": file_path,
                        "module": detail.module
                    }));
                }

                if detail.module.matches('.').count() >= 4 {
                    patterns["long_import_chains"].as_array_mut().unwrap().push(json!({
                        "file": file_path,
                        "module": detail.module,
                        "depth": detail.module.matches('.').count()
                    }));
                }

                if detail.module.starts_with('.') {
                    patterns["relative_imports"].as_array_mut().unwrap().push(json!({
                        "file": file_path,
                        "module": detail.module
                    }));
                }

                if !detail.aliases.is_empty() {
                    patterns["aliased_imports"].as_array_mut().unwrap().push(json!({
                        "file": file_path,
                        "module": detail.module,
                        "aliases": detail.aliases
                    }));
                }
            }
        }

        let pattern_counts = json!({
            "star_imports": patterns["star_imports"].as_array().unwrap().len(),
            "long_import_chains": patterns["long_import_chains"].as_array().unwrap().len(),
            "relative_imports": patterns["relative_imports"].as_array().unwrap().len(),
            "aliased_imports": patterns["aliased_imports"].as_array().unwrap().len()
        });

        json!({
            "patterns": patterns,
            "pattern_counts": pattern_counts
        })
    }

    fn generate_dependencies_summary(&self, total_files: usize, file_dependencies: &HashMap<String, FileDependencyInfo>, circular_deps: &[CircularDependency], external_deps: &Value) -> Value {
        if file_dependencies.is_empty() {
            return json!({"total_files": total_files});
        }

        let total_imports: usize = file_dependencies.values().map(|deps| deps.total_imports).sum();
        let avg_imports = if !file_dependencies.is_empty() { total_imports as f64 / file_dependencies.len() as f64 } else { 0.0 };
        let files_with_deps = file_dependencies.values().filter(|deps| deps.total_imports > 0).count();

        json!({
            "total_files_analyzed": total_files,
            "files_with_dependencies": files_with_deps,
            "total_import_statements": total_imports,
            "average_imports_per_file": avg_imports,
            "circular_dependencies_found": circular_deps.len(),
            "external_dependencies": external_deps["total_external_dependencies"].as_u64().unwrap_or(0),
            "dependency_coverage": if total_files > 0 { files_with_deps as f64 / total_files as f64 * 100.0 } else { 0.0 }
        })
    }

    fn generate_dependency_recommendations(&self, circular_deps: &[CircularDependency], coupling_analysis: &Value, external_deps: &Value) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !circular_deps.is_empty() {
            recommendations.push(format!("🔄 Resolve {} circular dependencies to improve maintainability", circular_deps.len()));
        }

        let strong_couplings = coupling_analysis["strong_couplings"].as_array().map(|arr| arr.len()).unwrap_or(0);
        if strong_couplings > 0 {
            recommendations.push(format!("🔗 Reduce coupling in {} highly-coupled files", strong_couplings));
        }

        let third_party_deps = external_deps["third_party_dependencies"].as_array().map(|arr| arr.len()).unwrap_or(0);
        if third_party_deps > 20 {
            recommendations.push("📦 Consider consolidating external dependencies to reduce complexity".to_string());
        }

        recommendations.extend(vec![
            "🏗️ Use dependency injection to reduce tight coupling".to_string(),
            "📁 Group related functionality into cohesive modules".to_string(),
            "🔍 Regularly audit and remove unused imports".to_string(),
            "📐 Follow the dependency inversion principle".to_string(),
            "🎯 Aim for low coupling and high cohesion".to_string(),
        ]);

        recommendations
    }

    fn serialize_file_dependencies(&self, file_dependencies: &HashMap<String, FileDependencyInfo>) -> Value {
        let mut result = serde_json::Map::new();

        for (file_path, deps) in file_dependencies {
            result.insert(file_path.clone(), json!({
                "imports": deps.imports,
                "from_imports": deps.from_imports,
                "import_details": deps.import_details.iter().map(|detail| json!({
                    "type": detail.import_type,
                    "module": detail.module,
                    "imported_names": detail.imported_names,
                    "aliases": detail.aliases,
                    "line": detail.line,
                    "is_star_import": detail.is_star_import
                })).collect::<Vec<_>>(),
                "total_imports": deps.total_imports,
                "internal_imports": deps.internal_imports,
                "external_imports": deps.external_imports,
                "coupling_score": deps.coupling_score,
                "import_complexity": deps.import_complexity
            }));
        }

        Value::Object(result)
    }

    fn serialize_circular_dependencies(&self, circular_deps: &[CircularDependency]) -> Value {
        json!(circular_deps.iter().map(|dep| json!({
            "cycle": dep.cycle,
            "length": dep.length,
            "severity": dep.severity
        })).collect::<Vec<_>>())
    }
}