use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

use super::ExternalTool;

/// Ripgrep integration for fast text search and pattern analysis
#[derive(Debug, Clone)]
pub struct RipgrepTool {
    codebase_path: String,
}

/// A match found by ripgrep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipgrepMatch {
    /// The file path where the match was found
    pub file_path: String,
    /// Line number (1-indexed)
    pub line_number: u32,
    /// Column number (1-indexed)
    pub column_number: u32,
    /// The matched text
    pub matched_text: String,
    /// The full line containing the match
    pub line_text: String,
    /// The pattern that was matched
    pub pattern: String,
}

/// Results from ripgrep search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipgrepResult {
    /// List of matches found
    pub matches: Vec<RipgrepMatch>,
    /// Search statistics
    pub stats: RipgrepStats,
    /// Whether the search was successful
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
}

/// Search statistics from ripgrep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipgrepStats {
    /// Total number of matches found
    pub total_matches: usize,
    /// Number of files with matches
    pub files_with_matches: usize,
    /// Number of files searched
    pub files_searched: usize,
    /// Search time in milliseconds
    pub search_time_ms: u64,
}

impl RipgrepTool {
    /// Create a new ripgrep tool instance
    pub fn new<P: AsRef<Path>>(codebase_path: P) -> Self {
        Self {
            codebase_path: codebase_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Search for a pattern in the codebase
    pub async fn search(&self, pattern: &str) -> Result<RipgrepResult> {
        self.search_with_options(pattern, &RipgrepOptions::default()).await
    }

    /// Search with custom options
    pub async fn search_with_options(&self, pattern: &str, options: &RipgrepOptions) -> Result<RipgrepResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new("rg");
        cmd.arg("--json")
           .arg("--line-number")
           .arg("--column")
           .arg("--no-heading")
           .arg("--with-filename");

        // Apply options
        if options.case_insensitive {
            cmd.arg("--ignore-case");
        }
        if options.word_boundary {
            cmd.arg("--word-regexp");
        }
        if options.multiline {
            cmd.arg("--multiline");
        }
        if let Some(ref file_type) = options.file_type {
            cmd.arg("--type").arg(file_type);
        }
        if let Some(ref glob) = options.glob_pattern {
            cmd.arg("--glob").arg(glob);
        }
        if let Some(max_count) = options.max_count {
            cmd.arg("--max-count").arg(max_count.to_string());
        }

        // Add the pattern and path
        cmd.arg(pattern).arg(&self.codebase_path);

        let output = cmd.output().await.context("Failed to execute ripgrep")?;

        let search_time_ms = start_time.elapsed().as_millis() as u64;

        if !output.status.success() {
            return Ok(RipgrepResult {
                matches: Vec::new(),
                stats: RipgrepStats {
                    total_matches: 0,
                    files_with_matches: 0,
                    files_searched: 0,
                    search_time_ms,
                },
                success: false,
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();
        let mut files_with_matches = std::collections::HashSet::new();

        for line in stdout.lines() {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                if json_value["type"] == "match" {
                    if let Ok(rg_match) = self.parse_ripgrep_match(&json_value, pattern) {
                        files_with_matches.insert(rg_match.file_path.clone());
                        matches.push(rg_match);
                    }
                }
            }
        }

        Ok(RipgrepResult {
            stats: RipgrepStats {
                total_matches: matches.len(),
                files_with_matches: files_with_matches.len(),
                files_searched: 0, // ripgrep doesn't provide this in JSON output
                search_time_ms,
            },
            matches,
            success: true,
            error: None,
        })
    }

    /// Parse a ripgrep JSON match entry
    fn parse_ripgrep_match(&self, json: &serde_json::Value, pattern: &str) -> Result<RipgrepMatch> {
        let data = &json["data"];
        let path = data["path"]["text"].as_str()
            .context("Missing path in ripgrep output")?;
        let line_number = data["line_number"].as_u64()
            .context("Missing line_number in ripgrep output")? as u32;
        let lines = &data["lines"];

        if let Some(line_data) = lines["text"].as_str() {
            let submatches = &data["submatches"];
            if let Some(submatch_array) = submatches.as_array() {
                if let Some(first_match) = submatch_array.first() {
                    let start = first_match["start"].as_u64().unwrap_or(0) as u32;
                    let end = first_match["end"].as_u64().unwrap_or(0) as u32;
                    let matched_text = first_match["match"]["text"].as_str()
                        .unwrap_or("").to_string();

                    return Ok(RipgrepMatch {
                        file_path: path.to_string(),
                        line_number,
                        column_number: start + 1, // Convert to 1-indexed
                        matched_text,
                        line_text: line_data.to_string(),
                        pattern: pattern.to_string(),
                    });
                }
            }
        }

        // Fallback if JSON parsing fails
        Ok(RipgrepMatch {
            file_path: path.to_string(),
            line_number,
            column_number: 1,
            matched_text: pattern.to_string(),
            line_text: "".to_string(),
            pattern: pattern.to_string(),
        })
    }

    /// Check if ripgrep is available
    pub async fn is_available() -> bool {
        Command::new("rg")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get ripgrep version
    pub async fn version() -> Result<String> {
        let output = Command::new("rg")
            .arg("--version")
            .output()
            .await
            .context("Failed to get ripgrep version")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("unknown").to_string())
        } else {
            Ok("unknown".to_string())
        }
    }
}

/// Options for ripgrep search
#[derive(Debug, Clone, Default)]
pub struct RipgrepOptions {
    /// Case insensitive search
    pub case_insensitive: bool,
    /// Word boundary matching
    pub word_boundary: bool,
    /// Multiline matching
    pub multiline: bool,
    /// File type filter (e.g., "py", "js", "rs")
    pub file_type: Option<String>,
    /// Glob pattern for file filtering
    pub glob_pattern: Option<String>,
    /// Maximum number of matches per file
    pub max_count: Option<usize>,
}

#[async_trait]
impl ExternalTool for RipgrepTool {
    type Result = RipgrepResult;

    async fn analyze(&self) -> Result<Self::Result> {
        // Default analysis: search for common patterns that might indicate code issues
        let patterns = vec![
            "TODO",
            "FIXME",
            "XXX",
            "HACK",
            "BUG",
            "NOTE",
            "WARNING",
            "DEPRECATED",
        ];

        let mut all_matches = Vec::new();
        let mut total_files_with_matches = std::collections::HashSet::new();
        let start_time = std::time::Instant::now();

        for pattern in &patterns {
            let options = RipgrepOptions {
                case_insensitive: true,
                ..Default::default()
            };

            if let Ok(result) = self.search_with_options(pattern, &options).await {
                for m in result.matches {
                    total_files_with_matches.insert(m.file_path.clone());
                    all_matches.push(m);
                }
            }
        }

        let search_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(RipgrepResult {
            stats: RipgrepStats {
                total_matches: all_matches.len(),
                files_with_matches: total_files_with_matches.len(),
                files_searched: 0,
                search_time_ms,
            },
            matches: all_matches,
            success: true,
            error: None,
        })
    }

    async fn is_available(&self) -> bool {
        Self::is_available().await
    }

    async fn get_version(&self) -> Result<String> {
        Self::version().await
    }

    fn tool_name(&self) -> &'static str {
        "ripgrep"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_file(dir: &TempDir, name: &str, content: &str) -> Result<()> {
        let file_path = dir.path().join(name);
        fs::write(file_path, content).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_ripgrep_search() {
        let temp_dir = TempDir::new().unwrap();

        create_test_file(&temp_dir, "test.py",
            "# TODO: implement this function\ndef hello():\n    pass\n# FIXME: handle edge case").await.unwrap();

        let tool = RipgrepTool::new(temp_dir.path());
        let result = tool.search("TODO").await.unwrap();

        assert!(result.success);
        assert!(!result.matches.is_empty());
        assert_eq!(result.matches[0].pattern, "TODO");
    }

    #[tokio::test]
    async fn test_ripgrep_with_options() {
        let temp_dir = TempDir::new().unwrap();

        create_test_file(&temp_dir, "test.py",
            "def function_name():\n    return 'todo'\n# TODO: implement").await.unwrap();

        let tool = RipgrepTool::new(temp_dir.path());

        // Case sensitive search should not find 'todo'
        let options = RipgrepOptions {
            case_insensitive: false,
            ..Default::default()
        };
        let result = tool.search_with_options("TODO", &options).await.unwrap();
        assert_eq!(result.matches.len(), 1); // Only the comment

        // Case insensitive should find both
        let options = RipgrepOptions {
            case_insensitive: true,
            ..Default::default()
        };
        let result = tool.search_with_options("todo", &options).await.unwrap();
        assert!(result.matches.len() >= 1);
    }

    #[tokio::test]
    async fn test_ripgrep_default_analysis() {
        let temp_dir = TempDir::new().unwrap();

        create_test_file(&temp_dir, "test.py",
            "# TODO: implement\n# FIXME: bug here\n# XXX: this is wrong").await.unwrap();

        let tool = RipgrepTool::new(temp_dir.path());
        let result = tool.analyze().await.unwrap();

        assert!(result.success);
        assert!(result.matches.len() >= 3); // Should find TODO, FIXME, XXX
        assert!(result.stats.files_with_matches >= 1);
    }

    #[tokio::test]
    async fn test_ripgrep_file_type_filter() {
        let temp_dir = TempDir::new().unwrap();

        create_test_file(&temp_dir, "test.py", "# TODO: python").await.unwrap();
        create_test_file(&temp_dir, "test.js", "// TODO: javascript").await.unwrap();

        let tool = RipgrepTool::new(temp_dir.path());

        let options = RipgrepOptions {
            file_type: Some("py".to_string()),
            ..Default::default()
        };

        let result = tool.search_with_options("TODO", &options).await.unwrap();

        // Should only find matches in Python files
        for m in &result.matches {
            assert!(m.file_path.ends_with(".py"));
        }
    }

    #[tokio::test]
    async fn test_ripgrep_availability() {
        // Test if ripgrep is available (may fail in some test environments)
        let available = RipgrepTool::is_available().await;

        if available {
            let version = RipgrepTool::version().await;
            assert!(version.is_ok());
            assert!(!version.unwrap().is_empty());
        }
    }
}