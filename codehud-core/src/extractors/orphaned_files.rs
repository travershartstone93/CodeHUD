use super::BaseDataExtractor;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use tree_sitter::{Language, Parser};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::fs;

extern "C" {
    fn tree_sitter_rust() -> Language;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrphanedFile {
    file_path: String,
    confidence_score: f64,
    reasons: Vec<String>,
    file_size: u64,
    last_modified: String,
    is_test_file: bool,
    is_config_file: bool,
    is_documentation: bool,
}

#[derive(Debug, Clone)]
struct FileUsage {
    file_path: String,
    imported_by: HashSet<String>,
    imports: HashSet<String>,
    is_executable: bool,
    has_main_function: bool,
    is_entry_point: bool,
}

pub struct OrphanedFilesExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
}

impl OrphanedFilesExtractor {
    pub fn new(codebase_path: impl AsRef<Path>) -> crate::Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();
        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!("Codebase path does not exist: {}", codebase_path.display())));
        }

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Config(format!("Failed to set language: {}", e)))?;

        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
            parser,
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

    fn analyze_file_usage(&self, files: &[PathBuf]) -> HashMap<String, FileUsage> {
        let mut usage_map = HashMap::new();

        // Initialize all files
        for file_path in files {
            let path_str = file_path.display().to_string();
            usage_map.insert(path_str.clone(), FileUsage {
                file_path: path_str,
                imported_by: HashSet::new(),
                imports: HashSet::new(),
                is_executable: self.is_executable_file(file_path),
                has_main_function: false,
                is_entry_point: false,
            });
        }

        // Analyze imports for each file
        for file_path in files {
            if let Ok(imports) = self.extract_imports(file_path) {
                let file_str = file_path.display().to_string();

                // Check if file has main function
                let has_main = self.has_main_function(file_path);
                if let Some(usage) = usage_map.get_mut(&file_str) {
                    usage.has_main_function = has_main;
                    usage.is_entry_point = has_main || self.is_script_entry_point(file_path);
                }

                // Map imports to actual files
                for import in imports {
                    if let Some(target_file) = self.resolve_import_to_file(&import, files) {
                        let target_str = target_file.display().to_string();

                        // Add to usage tracking
                        if let Some(usage) = usage_map.get_mut(&file_str) {
                            usage.imports.insert(target_str.clone());
                        }

                        if let Some(target_usage) = usage_map.get_mut(&target_str) {
                            target_usage.imported_by.insert(file_str.clone());
                        }
                    }
                }
            }
        }

        usage_map
    }

    fn extract_imports(&self, file_path: &Path) -> crate::Result<Vec<String>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io(e))?;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(language)
            .map_err(|e| crate::Error::Analysis(format!("Failed to set language: {}", e)))?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| crate::Error::Analysis("Failed to parse file".to_string()))?;

        let mut imports = Vec::new();
        self.extract_imports_from_node(tree.root_node(), &content, &mut imports);

        Ok(imports)
    }

    fn extract_imports_from_node(&self, node: tree_sitter::Node, source: &str, imports: &mut Vec<String>) {
        match node.kind() {
            "use_declaration" => {
                // Extract the module path from use declaration
                let use_text = &source[node.start_byte()..node.end_byte()];
                if let Some(module_part) = use_text.strip_prefix("use ") {
                    let module_part = module_part.trim_end_matches(';').trim();
                    if let Some(module_name) = module_part.split("::").next() {
                        imports.push(module_name.to_string());
                    }
                }
            }
            _ => {}
        }

        // Visit child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.extract_imports_from_node(child, source, imports);
            }
        }
    }

    fn resolve_import_to_file(&self, import: &str, files: &[PathBuf]) -> Option<PathBuf> {
        // Try to match import to actual file
        for file_path in files {
            if let Some(file_name) = file_path.file_stem().and_then(|n| n.to_str()) {
                if import.contains(file_name) {
                    return Some(file_path.clone());
                }
            }

            // Check if import matches directory structure
            if file_path.display().to_string().contains(import) {
                return Some(file_path.clone());
            }
        }

        None
    }

    fn has_main_function(&self, file_path: &Path) -> bool {
        if let Ok(content) = fs::read_to_string(file_path) {
            content.contains("def main(") || content.contains("if __name__ == \"__main__\"") || content.contains("if __name__ == '__main__'")
        } else {
            false
        }
    }

    fn is_script_entry_point(&self, file_path: &Path) -> bool {
        // Check if file is likely an entry point based on naming conventions
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            matches!(file_name,
                "main.py" | "cli.py" | "app.py" | "run.py" |
                "start.py" | "launcher.py" | "entry.py" |
                "__main__.py" | "manage.py"
            )
        } else {
            false
        }
    }

    fn is_executable_file(&self, file_path: &Path) -> bool {
        if let Ok(metadata) = fs::metadata(file_path) {
            // Check if file has executable permissions (Unix-like systems)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                // On non-Unix systems, check for shebang or entry point patterns
                self.has_main_function(file_path) || self.is_script_entry_point(file_path)
            }
        } else {
            false
        }
    }

    fn identify_orphaned_files(&self, usage_map: &HashMap<String, FileUsage>, files: &[PathBuf]) -> Vec<OrphanedFile> {
        let mut orphaned_files = Vec::new();

        for file_path in files {
            let file_str = file_path.display().to_string();
            if let Some(usage) = usage_map.get(&file_str) {
                let confidence_score = self.calculate_orphan_confidence(usage, file_path);

                if confidence_score > 0.3 {  // Threshold for considering a file orphaned
                    let reasons = self.get_orphan_reasons(usage, file_path);
                    let file_size = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
                    let last_modified = fs::metadata(file_path)
                        .and_then(|m| m.modified())
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|_| "Unknown".to_string());

                    orphaned_files.push(OrphanedFile {
                        file_path: file_str,
                        confidence_score,
                        reasons,
                        file_size,
                        last_modified,
                        is_test_file: self.is_test_file(file_path),
                        is_config_file: self.is_config_file(file_path),
                        is_documentation: self.is_documentation_file(file_path),
                    });
                }
            }
        }

        // Sort by confidence score (highest first)
        orphaned_files.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());

        orphaned_files
    }

    fn calculate_orphan_confidence(&self, usage: &FileUsage, file_path: &Path) -> f64 {
        let mut confidence: f64 = 0.0;

        // No imports from other files
        if usage.imported_by.is_empty() {
            confidence += 0.6;
        }

        // Not an entry point
        if !usage.is_entry_point && !usage.has_main_function {
            confidence += 0.2;
        }

        // Not executable
        if !usage.is_executable {
            confidence += 0.1;
        }

        // Reduce confidence for special file types
        if self.is_test_file(file_path) {
            confidence -= 0.3; // Tests might not be imported but are still needed
        }

        if self.is_config_file(file_path) {
            confidence -= 0.4; // Config files often aren't imported
        }

        if self.is_documentation_file(file_path) {
            confidence -= 0.5; // Documentation files aren't imported
        }

        // Small files might be utilities
        if let Ok(metadata) = fs::metadata(file_path) {
            if metadata.len() < 1000 {  // Files smaller than 1KB
                confidence -= 0.1;
            }
        }

        confidence.max(0.0).min(1.0)
    }

    fn get_orphan_reasons(&self, usage: &FileUsage, file_path: &Path) -> Vec<String> {
        let mut reasons = Vec::new();

        if usage.imported_by.is_empty() {
            reasons.push("Not imported by any other files".to_string());
        }

        if !usage.is_entry_point && !usage.has_main_function {
            reasons.push("Not an entry point or executable script".to_string());
        }

        if usage.imports.is_empty() {
            reasons.push("Does not import any local modules".to_string());
        }

        if self.is_old_file(file_path) {
            reasons.push("File has not been modified recently".to_string());
        }

        if self.is_empty_or_minimal(file_path) {
            reasons.push("File contains minimal code".to_string());
        }

        reasons
    }

    fn is_test_file(&self, file_path: &Path) -> bool {
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            file_name.starts_with("test_") ||
            file_name.ends_with("_test.py") ||
            file_path.display().to_string().contains("/tests/") ||
            file_path.display().to_string().contains("\\tests\\")
        } else {
            false
        }
    }

    fn is_config_file(&self, file_path: &Path) -> bool {
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            matches!(file_name,
                "config.py" | "settings.py" | "configuration.py" |
                "setup.py" | "__init__.py" | "conftest.py"
            )
        } else {
            false
        }
    }

    fn is_documentation_file(&self, file_path: &Path) -> bool {
        file_path.display().to_string().contains("/docs/") ||
        file_path.display().to_string().contains("\\docs\\") ||
        file_path.display().to_string().contains("/doc/") ||
        file_path.display().to_string().contains("\\doc\\")
    }

    fn is_old_file(&self, file_path: &Path) -> bool {
        if let Ok(metadata) = fs::metadata(file_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = std::time::SystemTime::now().duration_since(modified) {
                    return duration.as_secs() > 86400 * 90; // 90 days
                }
            }
        }
        false
    }

    fn is_empty_or_minimal(&self, file_path: &Path) -> bool {
        if let Ok(content) = fs::read_to_string(file_path) {
            let non_empty_lines: Vec<&str> = content
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .collect();

            non_empty_lines.len() < 5  // Less than 5 meaningful lines
        } else {
            true
        }
    }
}

impl BaseDataExtractor for OrphanedFilesExtractor {
    fn extract_data(&self) -> crate::Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        let files = self.get_all_python_files();

        if files.is_empty() {
            result.insert("orphaned_files".to_string(), json!([]));
            result.insert("files_analyzed".to_string(), json!(0));
            return Ok(result);
        }

        // Analyze file usage patterns
        let usage_map = self.analyze_file_usage(&files);

        // Identify orphaned files
        let orphaned_files = self.identify_orphaned_files(&usage_map, &files);

        // Generate statistics
        let total_files = files.len();
        let orphaned_count = orphaned_files.len();
        let high_confidence_orphans = orphaned_files.iter()
            .filter(|f| f.confidence_score > 0.7)
            .count();

        // Calculate file usage statistics
        let mut never_imported = 0;
        let mut no_imports = 0;
        let mut entry_points = 0;

        for usage in usage_map.values() {
            if usage.imported_by.is_empty() {
                never_imported += 1;
            }
            if usage.imports.is_empty() {
                no_imports += 1;
            }
            if usage.is_entry_point {
                entry_points += 1;
            }
        }

        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));
        result.insert("files_analyzed".to_string(), json!(total_files));
        result.insert("orphaned_files".to_string(), json!(orphaned_files));
        result.insert("orphaned_count".to_string(), json!(orphaned_count));
        result.insert("high_confidence_orphans".to_string(), json!(high_confidence_orphans));
        result.insert("orphan_percentage".to_string(), json!(orphaned_count as f64 / total_files as f64 * 100.0));
        result.insert("never_imported_count".to_string(), json!(never_imported));
        result.insert("no_imports_count".to_string(), json!(no_imports));
        result.insert("entry_points_count".to_string(), json!(entry_points));

        // Add recommendations
        let mut recommendations = Vec::new();
        if high_confidence_orphans > 0 {
            recommendations.push(format!("Consider reviewing {} high-confidence orphaned files", high_confidence_orphans));
        }
        if orphaned_count as f64 / total_files as f64 > 0.2 {
            recommendations.push("High percentage of orphaned files detected - consider refactoring".to_string());
        }

        result.insert("recommendations".to_string(), json!(recommendations));

        println!("Orphaned files extraction complete: {} files analyzed, {} orphaned files found ({:.1}% orphaned)",
                 total_files, orphaned_count, orphaned_count as f64 / total_files as f64 * 100.0);

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "OrphanedFilesExtractor"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}