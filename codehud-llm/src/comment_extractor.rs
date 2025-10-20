//! Comment Extraction Engine with Tree-sitter Integration
//!
//! This module implements the core comment extraction functionality described in LLM_vision.txt,
//! leveraging CodeHUD's existing enhanced tree-sitter infrastructure for high-performance
//! multi-language comment analysis.

use crate::{LlmResult, LlmError};
use crate::narrator::{NarratorConfig, DetectorRegistry, FileCst, Node, aggregate_findings, render::render_bullets_compact};
use codehud_core::query_engine::{QueryEngine, get_query_engine, SupportedLanguage};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use regex::Regex;
use walkdir;

/// Position information for comments in source code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

/// Type of comment detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommentType {
    /// Single line comment (// in Rust, # in Python)
    Line,
    /// Block comment (/* */ in Rust)
    Block,
    /// Documentation comment (/// in Rust, """ in Python)
    Doc,
    /// Multi-line documentation block
    DocBlock,
}

/// Code context surrounding a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    /// Name of function the comment is associated with
    pub function_name: Option<String>,
    /// Name of class/struct the comment is associated with
    pub class_name: Option<String>,
    /// Module or namespace path
    pub module_path: String,
    /// 3-5 lines of code adjacent to the comment
    pub adjacent_code: String,
    /// Whether this comment appears to document the following code
    pub documents_following_code: bool,
}

/// Structural insights derived from AST analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StructuralInsights {
    /// Source of these insights
    pub source: String,
    /// Whether these are generated (not human-written)
    pub generated: bool,
    /// Organized insights by category
    pub sections: HashMap<String, Vec<String>>,
}

/// Cleaned format optimized for LLM consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanedFileAnalysis {
    /// File path
    pub file_path: String,
    /// Language detected
    pub language: String,
    /// Cleaned comment text without JSON overhead
    pub comments: Vec<String>,
    /// Generated structural insights
    pub structural_insights: StructuralInsights,
}

/// An extracted comment with rich metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedComment {
    /// The comment text (without comment markers)
    pub text: String,
    /// Type of comment
    pub comment_type: CommentType,
    /// Byte offset in source file where comment starts
    pub start_byte: usize,
    /// Byte offset in source file where comment ends
    pub end_byte: usize,
    /// Line/column position where comment starts
    pub start_position: Position,
    /// Line/column position where comment ends
    pub end_position: Position,
    /// Surrounding code context
    pub context: Option<CodeContext>,
}

/// Configuration for comment extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Include single-line comments
    pub include_line_comments: bool,
    /// Include block comments
    pub include_block_comments: bool,
    /// Include documentation comments
    pub include_doc_comments: bool,
    /// Extract code context around comments
    pub extract_context: bool,
    /// Number of lines of adjacent code to capture
    pub context_lines: usize,
    /// Minimum comment length to include (filters out noise)
    pub min_comment_length: usize,
    /// Maximum comment length to include (prevents huge comments)
    pub max_comment_length: Option<usize>,
    /// Skip comments that are just dividers (e.g., "// ========")
    pub skip_divider_comments: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            include_line_comments: true,
            include_block_comments: true,
            include_doc_comments: true,
            extract_context: true,
            context_lines: 3,
            min_comment_length: 5,
            max_comment_length: Some(2000),
            skip_divider_comments: true,
        }
    }
}

/// Results of extracting comments from a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCommentExtraction {
    /// File path that was analyzed
    pub file: String,
    /// Programming language detected
    pub language: String,
    /// Extraction method used
    pub extraction_method: String,
    /// List of extracted comments
    pub comments: Vec<ExtractedComment>,
    /// Generated structural insights about the code
    pub structural_insights: Option<StructuralInsights>,
    /// Extraction statistics
    pub stats: ExtractionStats,
}

/// Statistics about comment extraction
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractionStats {
    /// Total comments found
    pub total_comments: usize,
    /// Comments by type
    pub comments_by_type: HashMap<String, usize>,
    /// Total lines of code processed
    pub total_lines: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Whether extraction was successful
    pub extraction_successful: bool,
    /// Error message if extraction failed
    pub error_message: Option<String>,
}

/// Main comment extractor leveraging CodeHUD's tree-sitter engine
pub struct CommentExtractor {
    /// Configuration for extraction behavior
    config: ExtractionConfig,
}

impl CommentExtractor {
    /// Create a new comment extractor
    pub fn new() -> Self {
        Self {
            config: ExtractionConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self {
            config,
        }
    }

    /// Extract comments from a single file
    pub fn extract_from_file(&self, file_path: &Path) -> LlmResult<FileCommentExtraction> {
        let start_time = std::time::Instant::now();

        // Read file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| LlmError::Io(e))?;

        // Detect language from file extension
        let language = self.detect_language(file_path)?;

        // Extract comments using tree-sitter
        let comments = self.extract_comments_from_content(&content, &language, file_path)?;

        // Structural insights will be added later in batch processing
        let structural_insights = None;

        // Calculate statistics
        let processing_time = start_time.elapsed().as_millis() as u64;
        let stats = self.calculate_stats(&comments, &content, processing_time, None);

        Ok(FileCommentExtraction {
            file: file_path.to_string_lossy().to_string(),
            language: language.clone(),
            extraction_method: "enhanced_tree_sitter".to_string(),
            comments,
            structural_insights,
            stats,
        })
    }

    /// Extract comments from multiple files in a directory
    pub fn extract_from_directory(&self, dir_path: &Path) -> LlmResult<Vec<FileCommentExtraction>> {
        println!("📁 Phase 1: Extracting comments from all files...");
        let mut results = Vec::new();
        let mut file_paths = Vec::new();

        // Walk directory recursively
        for entry in walkdir::WalkDir::new(dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip directories
            if !path.is_file() {
                continue;
            }

            // Skip build artifacts and common ignore patterns
            if let Some(path_str) = path.to_str() {
                if path_str.contains("/target/") ||
                   path_str.contains("/build/") ||
                   path_str.contains("/.git/") {
                    continue;
                }
            }

            // Check if we support this language
            if let Ok(_language) = self.detect_language(path) {
                match self.extract_from_file(path) {
                    Ok(extraction) => {
                        file_paths.push(path.to_path_buf());
                        results.push(extraction);
                    },
                    Err(e) => {
                        log::warn!("Failed to extract comments from {}: {}", path.display(), e);
                        // Continue with other files
                    }
                }
            }
        }

        println!("✅ Extracted comments from {} files", results.len());
        println!("🧠 Phase 2: Running narrator on all files in batch...");

        // Run narrator in batch and add insights to results
        self.add_narrator_insights_batch(&mut results, &file_paths)?;

        println!("✅ Added structural insights to all files");

        Ok(results)
    }

    /// Add narrator insights to all extractions in batch (runs narrator once for all files)
    pub fn add_narrator_insights_batch(&self, extractions: &mut [FileCommentExtraction], file_paths: &[PathBuf]) -> LlmResult<()> {
        use tree_sitter::Parser;
        use std::fs;

        let config = NarratorConfig::default();
        let registry = DetectorRegistry::new(&config);

        for (extraction, file_path) in extractions.iter_mut().zip(file_paths.iter()) {
            // Read file content
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Failed to read file for narrator: {}", e);
                    continue;
                }
            };

            // Get appropriate language parser
            let supported_lang = self.language_to_supported_language(&extraction.language);
            let tree_sitter_lang = supported_lang.tree_sitter_language();

            // Parse with tree-sitter
            let mut parser = Parser::new();
            if parser.set_language(tree_sitter_lang).is_err() {
                log::warn!("Failed to set language for narrator");
                continue;
            }

            let tree = match parser.parse(&content, None) {
                Some(t) => t,
                None => {
                    log::warn!("Failed to parse file for narrator");
                    continue;
                }
            };

            // Convert to FileCst and run narrator
            match self.convert_to_file_cst(file_path, &tree.root_node(), &content) {
                Ok(file_cst) => {
                    let findings = registry.detect_all(&file_cst);
                    println!("🔍 DEBUG: Found {} findings for {}", findings.len(), file_path.display());
                    let file_doc = aggregate_findings(&file_path.to_string_lossy(), &findings, &config);
                    let bullet_text = render_bullets_compact(&file_doc);
                    println!("🔍 DEBUG: Bullet text length: {} for {}", bullet_text.len(), file_path.display());

                    // Convert to StructuralInsights format
                    let mut sections = HashMap::new();
                    if !bullet_text.is_empty() {
                        let lines: Vec<&str> = bullet_text.lines().collect();
                        let mut current_section = String::new();
                        let mut current_bullets = Vec::new();

                        for line in lines {
                            let trimmed = line.trim();
                            if trimmed.starts_with("- ") {
                                // Bullet point
                                current_bullets.push(trimmed.trim_start_matches("- ").to_string());
                            } else if trimmed.ends_with(':') && !trimmed.is_empty() {
                                // Section header
                                if !current_section.is_empty() && !current_bullets.is_empty() {
                                    sections.insert(current_section.clone(), current_bullets.clone());
                                }
                                current_section = trimmed.trim_end_matches(':').to_string();
                                current_bullets.clear();
                            } else if trimmed.starts_with("Role:") {
                                // Skip role line
                                continue;
                            }
                        }

                        if !current_section.is_empty() && !current_bullets.is_empty() {
                            sections.insert(current_section, current_bullets);
                        }
                    }

                    extraction.structural_insights = Some(StructuralInsights {
                        source: "narrator_batch".to_string(),
                        generated: true,
                        sections,
                    });
                }
                Err(e) => {
                    log::warn!("Failed to convert to FileCst: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Extract comments from file content using tree-sitter
    fn extract_comments_from_content(
        &self,
        content: &str,
        language: &str,
        file_path: &Path,
    ) -> LlmResult<Vec<ExtractedComment>> {
        // Use shared query engine instance for performance
        match get_query_engine() {
            Ok(mut query_engine) => {
                match query_engine.analyze_file(file_path) {
                    Ok(analysis_result) => {

                        let comments = self.extract_comments_from_analysis(content, &analysis_result, file_path)?;
                        if comments.is_empty() {
                        } else {
                        }
                        Ok(comments)
                    }
                    Err(e) => {
                        Err(LlmError::ConfigurationError(format!("Tree-sitter analysis failed: {}", e)))
                    }
                }
            }
            Err(e) => {
                Err(LlmError::ConfigurationError(format!("Failed to create query engine: {}", e)))
            }
        }
    }

    /// Extract comments from tree-sitter analysis result
    fn extract_comments_from_analysis(
        &self,
        content: &str,
        analysis_result: &serde_json::Value,
        file_path: &Path,
    ) -> LlmResult<Vec<ExtractedComment>> {
        let mut comments = Vec::new();

        // Look for dedicated comments section first
        if let Some(comments_section) = analysis_result.get("comments") {
            if let Some(comments_array) = comments_section.get("comments") {
                if let Some(comment_list) = comments_array.as_array() {

                    for comment_data in comment_list {
                        if let Some(text) = comment_data.get("text").and_then(|v| v.as_str()) {
                            if let Some(start) = comment_data.get("start") {
                                if let Some(line) = start.get("row").and_then(|v| v.as_u64()) {
                                    // Get comment type from the analysis
                                    let comment_type = match comment_data.get("comment_type").and_then(|v| v.as_str()) {
                                        Some("doc") => CommentType::Doc,
                                        Some("block") => CommentType::Block,
                                        Some("line") => CommentType::Line,
                                        _ => {
                                            // Fallback: determine from text content
                                            if text.trim_start().starts_with("///") {
                                                CommentType::Doc
                                            } else if text.trim_start().starts_with("/*") {
                                                CommentType::Block
                                            } else {
                                                CommentType::Line
                                            }
                                        }
                                    };

                                    let cleaned_text = self.clean_comment_text(text);

                                    let start_byte = comment_data.get("start_byte").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                    let end_byte = comment_data.get("end_byte").and_then(|v| v.as_u64()).unwrap_or((start_byte + text.len()) as u64) as usize;

                                    let column = start.get("column").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                                    let end_line = comment_data.get("end").and_then(|e| e.get("row")).and_then(|v| v.as_u64()).unwrap_or(line) as usize;
                                    let end_column = comment_data.get("end").and_then(|e| e.get("column")).and_then(|v| v.as_u64()).unwrap_or((column + text.len()) as u64) as usize;


                                    let comment = ExtractedComment {
                                        text: cleaned_text,
                                        comment_type,
                                        start_byte,
                                        end_byte,
                                        start_position: Position { line: line as usize, column },
                                        end_position: Position { line: end_line, column: end_column },
                                        context: if self.config.extract_context {
                                            self.extract_code_context(content, start_byte, end_byte)
                                        } else {
                                            None
                                        },
                                    };

                                    if self.should_include_comment(&comment) {
                                        comments.push(comment);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(comments)
    }

    /// Parse comment from tree-sitter highlight
    fn parse_comment_from_highlight(
        &self,
        highlight: &serde_json::Value,
        content: &str,
    ) -> Option<ExtractedComment> {
        let start_byte = highlight.get("start_byte")?.as_u64()? as usize;
        let end_byte = highlight.get("end_byte")?.as_u64()? as usize;
        let start_line = highlight.get("start_line")?.as_u64()? as usize;
        let start_column = highlight.get("start_column")?.as_u64()? as usize;
        let end_line = highlight.get("end_line")?.as_u64()? as usize;
        let end_column = highlight.get("end_column")?.as_u64()? as usize;

        // Extract comment text
        let raw_text = content.get(start_byte..end_byte)?;
        let cleaned_text = self.clean_comment_text(raw_text);

        // Determine comment type
        let comment_type = self.determine_comment_type(raw_text);

        // Extract context if enabled
        let context = if self.config.extract_context {
            self.extract_code_context(content, start_byte, end_byte)
        } else {
            None
        };

        Some(ExtractedComment {
            text: cleaned_text,
            comment_type,
            start_byte,
            end_byte,
            start_position: Position { line: start_line + 1, column: start_column + 1 },
            end_position: Position { line: end_line + 1, column: end_column + 1 },
            context,
        })
    }

    /// Parse comment from tree-sitter capture
    fn parse_comment_from_capture(
        &self,
        capture: &serde_json::Value,
        content: &str,
    ) -> Option<ExtractedComment> {
        // Similar to parse_comment_from_highlight but for capture format
        // Implementation would be similar, adapting to capture data structure
        self.parse_comment_from_highlight(capture, content)
    }

    /// Fallback comment extraction using regex patterns
    fn fallback_comment_extraction(
        &self,
        content: &str,
        language: &str,
    ) -> LlmResult<Vec<ExtractedComment>> {
        let mut comments = Vec::new();

        // Define regex patterns for different comment types
        let patterns = match language {
            "rust" | "javascript" | "typescript" | "java" => {
                vec![
                    (regex::Regex::new(r"///?(.*)$").unwrap(), CommentType::Line),
                    (regex::Regex::new(r"/\*\*(.*?)\*/").unwrap(), CommentType::DocBlock),
                    (regex::Regex::new(r"/\*(.*?)\*/").unwrap(), CommentType::Block),
                ]
            }
            "python" => {
                vec![
                    (regex::Regex::new(r"#(.*)$").unwrap(), CommentType::Line),
                    (regex::Regex::new(r#""{3}(.*?)"{3}"#).unwrap(), CommentType::DocBlock),
                    (regex::Regex::new(r"'{3}(.*?)'{3}").unwrap(), CommentType::DocBlock),
                ]
            }
            _ => {
                return Err(LlmError::ConfigurationError(
                    format!("Unsupported language for fallback extraction: {}", language)
                ));
            }
        };

        // Apply patterns line by line for better position tracking
        for (line_idx, line) in content.lines().enumerate() {
            for (pattern, comment_type) in &patterns {
                for capture in pattern.captures_iter(line) {
                    if let Some(comment_match) = capture.get(0) {
                        let start_byte = content.lines()
                            .take(line_idx)
                            .map(|l| l.len() + 1) // +1 for newline
                            .sum::<usize>() + comment_match.start();

                        let end_byte = start_byte + comment_match.len();
                        let text = capture.get(1).map(|m| m.as_str()).unwrap_or("").trim().to_string();

                        if !text.is_empty() && text.len() >= self.config.min_comment_length {
                            let comment = ExtractedComment {
                                text,
                                comment_type: comment_type.clone(),
                                start_byte,
                                end_byte,
                                start_position: Position {
                                    line: line_idx + 1,
                                    column: comment_match.start() + 1
                                },
                                end_position: Position {
                                    line: line_idx + 1,
                                    column: comment_match.end() + 1
                                },
                                context: if self.config.extract_context {
                                    self.extract_code_context(content, start_byte, end_byte)
                                } else {
                                    None
                                },
                            };

                            if self.should_include_comment(&comment) {
                                comments.push(comment);
                            }
                        }
                    }
                }
            }
        }

        Ok(comments)
    }

    /// Detect programming language from file extension
    fn detect_language(&self, file_path: &Path) -> LlmResult<String> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let language = match extension.to_lowercase().as_str() {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "java" => "java",
            "cpp" | "cc" | "cxx" | "c++" => "cpp",
            "c" | "h" => "c",
            "go" => "go",
            "rb" => "ruby",
            "php" => "php",
            _ => return Err(LlmError::ConfigurationError(
                format!("Unsupported file extension: {}", extension)
            )),
        };

        Ok(language.to_string())
    }

    /// Clean comment text by removing comment markers and excess whitespace
    fn clean_comment_text(&self, raw_comment: &str) -> String {
        let mut cleaned = raw_comment.to_string();

        // Remove common comment markers
        cleaned = cleaned
            .trim_start_matches("///")
            .trim_start_matches("//")
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .trim_start_matches("**")
            .trim_start_matches("*")
            .trim_start_matches("#")
            .trim()
            .to_string();

        // Remove leading asterisks from multiline comments
        let lines: Vec<&str> = cleaned.lines()
            .map(|line| line.trim_start_matches('*').trim())
            .filter(|line| !line.is_empty())
            .collect();

        lines.join(" ")
    }

    /// Determine the type of comment based on markers
    fn determine_comment_type(&self, raw_comment: &str) -> CommentType {
        let trimmed = raw_comment.trim();

        if trimmed.starts_with("///") || trimmed.starts_with("/**") {
            CommentType::Doc
        } else if trimmed.starts_with("/*") {
            CommentType::Block
        } else if trimmed.starts_with("//") || trimmed.starts_with("#") {
            CommentType::Line
        } else if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            CommentType::DocBlock
        } else {
            CommentType::Line // Default
        }
    }

    /// Extract code context around a comment
    fn extract_code_context(&self, content: &str, start_byte: usize, end_byte: usize) -> Option<CodeContext> {
        let lines: Vec<&str> = content.lines().collect();

        // Find which line the comment is on
        let mut current_byte = 0;
        let mut comment_line = 0;

        for (idx, line) in lines.iter().enumerate() {
            let line_end = current_byte + line.len();
            if start_byte >= current_byte && start_byte <= line_end {
                comment_line = idx;
                break;
            }
            current_byte = line_end + 1; // +1 for newline
        }

        // Extract surrounding lines
        let context_start = comment_line.saturating_sub(self.config.context_lines);
        let context_end = (comment_line + self.config.context_lines + 1).min(lines.len());

        let adjacent_lines = lines[context_start..context_end].join("\n");

        // Try to extract function/class names (basic implementation)
        let function_name = self.extract_function_name(&adjacent_lines);
        let class_name = self.extract_class_name(&adjacent_lines);

        Some(CodeContext {
            function_name,
            class_name,
            module_path: "".to_string(), // TODO: Extract from file structure
            adjacent_code: adjacent_lines,
            documents_following_code: comment_line + 1 < lines.len(),
        })
    }

    /// Basic function name extraction
    fn extract_function_name(&self, code: &str) -> Option<String> {
        // Simple regex patterns for common languages
        let patterns = [
            regex::Regex::new(r"fn\s+(\w+)").ok()?,           // Rust
            regex::Regex::new(r"def\s+(\w+)").ok()?,          // Python
            regex::Regex::new(r"function\s+(\w+)").ok()?,     // JavaScript
            regex::Regex::new(r"\w+\s+(\w+)\s*\(").ok()?,     // Java/C++
        ];

        for pattern in &patterns {
            if let Some(capture) = pattern.captures(code) {
                if let Some(name) = capture.get(1) {
                    return Some(name.as_str().to_string());
                }
            }
        }

        None
    }

    /// Basic class name extraction
    fn extract_class_name(&self, code: &str) -> Option<String> {
        let patterns = [
            regex::Regex::new(r"struct\s+(\w+)").ok()?,       // Rust
            regex::Regex::new(r"class\s+(\w+)").ok()?,        // Python/Java
            regex::Regex::new(r"impl.*?(\w+)").ok()?,         // Rust impl
        ];

        for pattern in &patterns {
            if let Some(capture) = pattern.captures(code) {
                if let Some(name) = capture.get(1) {
                    return Some(name.as_str().to_string());
                }
            }
        }

        None
    }

    /// Check if a comment should be included based on configuration
    fn should_include_comment(&self, comment: &ExtractedComment) -> bool {
        // Check comment type inclusion
        match comment.comment_type {
            CommentType::Line => !self.config.include_line_comments,
            CommentType::Block => !self.config.include_block_comments,
            CommentType::Doc | CommentType::DocBlock => !self.config.include_doc_comments,
        };

        // Check length constraints
        if comment.text.len() < self.config.min_comment_length {
            return false;
        }

        if let Some(max_len) = self.config.max_comment_length {
            if comment.text.len() > max_len {
                return false;
            }
        }

        // Skip divider comments if configured
        if self.config.skip_divider_comments {
            let divider_chars = ['=', '-', '_', '*', '#'];
            let non_divider_chars = comment.text.chars()
                .filter(|c| !c.is_whitespace() && !divider_chars.contains(c))
                .count();

            // If comment is mostly divider characters, skip it
            if non_divider_chars < 3 {
                return false;
            }
        }

        true
    }

    /// Calculate extraction statistics
    fn calculate_stats(
        &self,
        comments: &[ExtractedComment],
        content: &str,
        processing_time_ms: u64,
        error_message: Option<String>,
    ) -> ExtractionStats {
        let mut comments_by_type = HashMap::new();

        for comment in comments {
            let type_key = match comment.comment_type {
                CommentType::Line => "line",
                CommentType::Block => "block",
                CommentType::Doc => "doc",
                CommentType::DocBlock => "doc_block",
            };
            *comments_by_type.entry(type_key.to_string()).or_insert(0) += 1;
        }

        ExtractionStats {
            total_comments: comments.len(),
            comments_by_type,
            total_lines: content.lines().count(),
            processing_time_ms,
            extraction_successful: error_message.is_none(),
            error_message,
        }
    }

    /// Convert FileCommentExtraction to cleaned format optimized for LLM
    pub fn to_cleaned_format(&self, extraction: &FileCommentExtraction) -> CleanedFileAnalysis {
        CleanedFileAnalysis {
            file_path: extraction.file.clone(),
            language: extraction.language.clone(),
            comments: self.clean_comments(&extraction.comments),
            structural_insights: extraction.structural_insights.clone().unwrap_or_else(|| {
                StructuralInsights {
                    source: "ast_analysis".to_string(),
                    generated: true,
                    sections: HashMap::new(),
                }
            }),
        }
    }

    /// Clean comments by removing JSON overhead and comment markers
    fn clean_comments(&self, comments: &[ExtractedComment]) -> Vec<String> {
        comments
            .iter()
            .map(|comment| self.clean_comment_text_for_llm(&comment.text))
            .filter(|text| !text.trim().is_empty())
            .collect()
    }

    /// Clean individual comment text for LLM consumption
    /// Conservative cleaning that preserves content while removing syntax bloat
    fn clean_comment_text_for_llm(&self, text: &str) -> String {
        let cleaned = text
            // Only remove outermost comment markers, preserve inner content
            .trim_start_matches("//!")
            .trim_start_matches("///")
            .trim_start_matches("//")
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .trim_start_matches("#")
            .trim_start_matches("\"\"\"")
            .trim_end_matches("\"\"\"")
            .trim();

        // If cleaning made the text too short or empty, return original (minus just whitespace)
        if cleaned.len() < 3 {
            return text.trim().to_string();
        }

        cleaned.to_string()
    }

    /// Generate structural insights from code content
    pub fn generate_structural_insights(&self, file_path: &Path, content: &str, language: &str) -> StructuralInsights {
        println!("🔍 DEBUG: Generating structural insights for {}", file_path.display());

        // Try to use narrator for bullet point generation
        match self.generate_bullet_points_with_narrator(file_path, content, language) {
            Ok(bullet_points) => {
                println!("✅ DEBUG: Narrator method succeeded for {}", file_path.display());
                return bullet_points;
            }
            Err(e) => {
                println!("❌ DEBUG: Narrator method failed for {}: {}", file_path.display(), e);
            }
        }

        println!("🔧 DEBUG: Using fallback method for {}", file_path.display());
        // Fallback to original method
        let mut sections = HashMap::new();

        // Extract imports
        let imports = self.extract_imports(content, language);
        println!("🔍 DEBUG: Found {} imports for {}", imports.len(), file_path.display());
        if !imports.is_empty() {
            println!("📦 DEBUG: Imports: {:?}", imports);
            sections.insert("imports".to_string(), imports);
        }

        // Extract functions/methods
        let functions = self.extract_functions(content, language);
        println!("🔍 DEBUG: Found {} functions for {}", functions.len(), file_path.display());
        if !functions.is_empty() {
            println!("⚙️ DEBUG: Functions: {:?}", functions);
            sections.insert("functions".to_string(), functions);
        }

        // Extract entry points
        let entry_points = self.extract_entry_points(content, language);
        if !entry_points.is_empty() {
            sections.insert("entry_points".to_string(), entry_points);
        }

        // Extract IO operations
        let io_ops = self.extract_io_operations(content, language);
        if !io_ops.is_empty() {
            sections.insert("io_operations".to_string(), io_ops);
        }

        // Extract test patterns
        let tests = self.extract_test_patterns(content, language);
        if !tests.is_empty() {
            sections.insert("tests".to_string(), tests);
        }

        StructuralInsights {
            source: "ast_analysis".to_string(),
            generated: true,
            sections,
        }
    }

    /// Generate bullet points using the narrator module
    fn generate_bullet_points_with_narrator(&self, file_path: &Path, content: &str, language: &str) -> LlmResult<StructuralInsights> {
        use tree_sitter::Parser;

        // Get the appropriate language parser
        let supported_lang = self.language_to_supported_language(language);
        let tree_sitter_lang = supported_lang.tree_sitter_language();

        // Parse the content directly with tree-sitter
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_lang)
            .map_err(|e| LlmError::Config(format!("Failed to set tree-sitter language: {}", e)))?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| LlmError::Config("Failed to parse content with tree-sitter".to_string()))?;

        // Convert tree-sitter tree to FileCst format
        let file_cst = self.convert_to_file_cst(file_path, &tree.root_node(), content)?;

        // Use narrator to detect findings and generate bullet points
        let config = NarratorConfig::default();
        let registry = DetectorRegistry::new(&config);
        let findings = registry.detect_all(&file_cst);
        let file_doc = aggregate_findings(&file_path.to_string_lossy(), &findings, &config);
        let bullet_text = render_bullets_compact(&file_doc);

        // Convert to StructuralInsights format
        let mut sections = HashMap::new();
        if !bullet_text.is_empty() {
            // Parse the bullet text into sections
            let lines: Vec<&str> = bullet_text.lines().collect();
            let mut current_section = String::new();
            let mut current_bullets = Vec::new();

            for line in lines {
                if line.ends_with(':') && !line.starts_with('-') {
                    // This is a section header
                    if !current_section.is_empty() && !current_bullets.is_empty() {
                        sections.insert(current_section.clone(), current_bullets.clone());
                    }
                    current_section = line.trim_end_matches(':').to_lowercase().replace(' ', "_");
                    current_bullets.clear();
                } else if line.starts_with('-') {
                    // This is a bullet point
                    current_bullets.push(line.trim_start_matches("- ").trim().to_string());
                } else if line.starts_with("Role:") {
                    // Special handling for role line
                    sections.insert("role".to_string(), vec![line.trim_start_matches("Role:").trim().to_string()]);
                }
            }

            // Add the last section
            if !current_section.is_empty() && !current_bullets.is_empty() {
                sections.insert(current_section, current_bullets);
            }
        }

        Ok(StructuralInsights {
            source: "narrator_bullet_generator".to_string(),
            generated: true,
            sections,
        })
    }

    /// Convert language string to SupportedLanguage enum
    fn language_to_supported_language(&self, language: &str) -> SupportedLanguage {
        match language.to_lowercase().as_str() {
            "rust" => SupportedLanguage::Rust,
            "python" => SupportedLanguage::Python,
            "javascript" | "js" => SupportedLanguage::JavaScript,
            "typescript" | "ts" => SupportedLanguage::TypeScript,
            "java" => SupportedLanguage::Java,
            _ => SupportedLanguage::Rust, // Default fallback
        }
    }

    /// Convert tree-sitter tree to narrator FileCst format
    fn convert_to_file_cst(&self, file_path: &Path, root_node: &tree_sitter::Node, content: &str) -> LlmResult<FileCst> {
        let narrator_root = self.convert_ts_node_recursive(root_node, content.as_bytes())?;
        Ok(FileCst {
            path: file_path.to_path_buf(),
            root: narrator_root,
            source_text: Some(content.to_string()),
        })
    }

    /// Recursively convert tree-sitter nodes to narrator Node format
    fn convert_ts_node_recursive(&self, ts_node: &tree_sitter::Node, source: &[u8]) -> LlmResult<Node> {
        let mut children = Vec::new();
        let mut cursor = ts_node.walk();

        if cursor.goto_first_child() {
            loop {
                children.push(self.convert_ts_node_recursive(&cursor.node(), source)?);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        // Include text for all nodes - detectors need text from parent nodes too
        let text = ts_node.utf8_text(source).ok().map(|s| s.to_string());

        Ok(Node {
            kind: ts_node.kind().to_string(),
            children,
            text,
            start_point: Some(crate::narrator::cst::Point {
                row: ts_node.start_position().row,
                column: ts_node.start_position().column,
            }),
        })
    }

    /// Extract import patterns for multiple languages
    fn extract_imports(&self, content: &str, language: &str) -> Vec<String> {
        let mut imports = HashSet::new();

        let patterns = match language {
            "rust" => vec![
                Regex::new(r"use\s+([^;]+);").unwrap(),
                Regex::new(r"extern\s+crate\s+(\w+)").unwrap(),
            ],
            "python" => vec![
                Regex::new(r"import\s+(\w+(?:\.\w+)*)").unwrap(),
                Regex::new(r"from\s+(\w+(?:\.\w+)*)\s+import").unwrap(),
            ],
            "javascript" | "typescript" => vec![
                Regex::new(r#"import\s+.*?from\s+["']([^"']+)["']"#).unwrap(),
                Regex::new(r#"require\(["']([^"']+)["']\)"#).unwrap(),
            ],
            "java" => vec![
                Regex::new(r"import\s+([\w.]+)").unwrap(),
            ],
            "go" => vec![
                Regex::new(r#"import\s+["']([^"']+)["']"#).unwrap(),
            ],
            _ => vec![],
        };

        for pattern in patterns {
            for capture in pattern.captures_iter(content) {
                if let Some(import) = capture.get(1) {
                    let import_name = import.as_str().trim();
                    if !import_name.is_empty() {
                        imports.insert(format!("- Uses {}", import_name));
                    }
                }
            }
        }

        imports.into_iter().take(8).collect() // Limit to avoid noise
    }

    /// Extract function/method patterns
    fn extract_functions(&self, content: &str, language: &str) -> Vec<String> {
        let mut functions = HashSet::new();

        let patterns = match language {
            "rust" => vec![
                Regex::new(r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap(),
                Regex::new(r"impl.*?\{").unwrap(), // impl blocks
            ],
            "python" => vec![
                Regex::new(r"def\s+(\w+)").unwrap(),
                Regex::new(r"async\s+def\s+(\w+)").unwrap(),
                Regex::new(r"class\s+(\w+)").unwrap(),
            ],
            "javascript" | "typescript" => vec![
                Regex::new(r"function\s+(\w+)").unwrap(),
                Regex::new(r"(?:async\s+)?(\w+)\s*(?::\s*\w+)?\s*=>").unwrap(),
                Regex::new(r"class\s+(\w+)").unwrap(),
            ],
            "java" => vec![
                Regex::new(r"(?:public|private|protected)?\s*(?:static)?\s*\w+\s+(\w+)\s*\(").unwrap(),
                Regex::new(r"class\s+(\w+)").unwrap(),
            ],
            "go" => vec![
                Regex::new(r"func\s+(\w+)").unwrap(),
                Regex::new(r"type\s+(\w+)\s+struct").unwrap(),
            ],
            _ => vec![],
        };

        for pattern in patterns {
            for capture in pattern.captures_iter(content) {
                if let Some(func_match) = capture.get(1) {
                    let func_name = func_match.as_str();
                    if func_name == "main" {
                        functions.insert("- main() serves as program entry point".to_string());
                    } else if func_name.starts_with("test") || func_name.contains("Test") {
                        // Skip test functions here, handle in test patterns
                    } else {
                        functions.insert(format!("- Defines {}()", func_name));
                    }
                }
            }
        }

        functions.into_iter().take(6).collect() // Limit to key functions
    }

    /// Extract entry point patterns
    fn extract_entry_points(&self, content: &str, language: &str) -> Vec<String> {
        let mut entry_points = Vec::new();

        match language {
            "rust" => {
                if content.contains("fn main()") || content.contains("async fn main()") {
                    entry_points.push("- Binary entry point: main()".to_string());
                }
                if content.contains("#[tokio::main]") {
                    entry_points.push("- Async runtime: tokio main".to_string());
                }
            },
            "python" => {
                if content.contains("if __name__ == \"__main__\":") {
                    entry_points.push("- Script entry point: __main__".to_string());
                }
            },
            "javascript" | "typescript" => {
                if content.contains("process.argv") {
                    entry_points.push("- CLI script with process.argv".to_string());
                }
            },
            _ => {},
        }

        entry_points
    }

    /// Extract IO operation patterns
    fn extract_io_operations(&self, content: &str, language: &str) -> Vec<String> {
        let mut io_ops = HashSet::new();

        // File IO patterns
        let file_patterns = match language {
            "rust" => vec!["std::fs::", "tokio::fs::", "File::", ".read(", ".write("],
            "python" => vec!["open(", "with open", "os.path", "pathlib"],
            "javascript" | "typescript" => vec!["fs.", "require('fs')", "readFile", "writeFile"],
            "java" => vec!["FileReader", "FileWriter", "Files.", "Path."],
            "go" => vec!["os.Open", "ioutil.", "io/ioutil"],
            _ => vec![],
        };

        for pattern in &file_patterns {
            if content.contains(pattern) {
                io_ops.insert("- Performs file I/O operations".to_string());
                break;
            }
        }

        // Network IO patterns
        let network_patterns = match language {
            "rust" => vec!["reqwest::", "hyper::", "tokio::net::", "TcpStream"],
            "python" => vec!["requests.", "urllib.", "http.", "socket."],
            "javascript" | "typescript" => vec!["fetch(", "axios", "http.", "https."],
            "java" => vec!["HttpClient", "URL(", "Socket"],
            "go" => vec!["http.", "net/http", "net."],
            _ => vec![],
        };

        for pattern in &network_patterns {
            if content.contains(pattern) {
                io_ops.insert("- Makes network requests".to_string());
                break;
            }
        }

        // Database patterns
        let db_patterns = match language {
            "rust" => vec!["sqlx::", "diesel::", "rusqlite::", "mongodb::"],
            "python" => vec!["sqlite3", "psycopg2", "pymongo", "sqlalchemy"],
            "javascript" | "typescript" => vec!["mongodb", "mongoose", "prisma", "sequelize"],
            "java" => vec!["JDBC", "Connection", "Statement", "ResultSet"],
            "go" => vec!["database/sql", "mongo-driver"],
            _ => vec![],
        };

        for pattern in &db_patterns {
            if content.contains(pattern) {
                io_ops.insert("- Accesses database".to_string());
                break;
            }
        }

        io_ops.into_iter().collect()
    }

    /// Extract test patterns
    fn extract_test_patterns(&self, content: &str, language: &str) -> Vec<String> {
        let mut tests = Vec::new();

        match language {
            "rust" => {
                if content.contains("#[test]") || content.contains("#[cfg(test)]") {
                    tests.push("- Contains unit tests".to_string());
                }
                if content.contains("assert!") || content.contains("assert_eq!") {
                    tests.push("- Uses assertion macros".to_string());
                }
            },
            "python" => {
                if content.contains("def test_") || content.contains("class Test") {
                    tests.push("- Contains test functions".to_string());
                }
                if content.contains("import pytest") || content.contains("import unittest") {
                    tests.push("- Uses testing framework".to_string());
                }
            },
            "javascript" | "typescript" => {
                if content.contains("describe(") || content.contains("it(") || content.contains("test(") {
                    tests.push("- Contains test cases".to_string());
                }
                if content.contains("expect(") || content.contains("assert") {
                    tests.push("- Uses test assertions".to_string());
                }
            },
            "java" => {
                if content.contains("@Test") || content.contains("junit") {
                    tests.push("- Contains JUnit tests".to_string());
                }
            },
            "go" => {
                if content.contains("func Test") || content.contains("testing.T") {
                    tests.push("- Contains Go test functions".to_string());
                }
            },
            _ => {},
        }

        tests
    }

    /// Format cleaned analysis as text for LLM consumption
    /// Balanced approach: removes JSON bloat but preserves essential context
    pub fn format_for_llm(&self, analysis: &CleanedFileAnalysis) -> String {
        let mut output = String::new();

        output.push_str(&format!("FILE: {} ({})\n", analysis.file_path, analysis.language));

        // Add comments section with minimal formatting
        if !analysis.comments.is_empty() {
            output.push_str("\nCOMMENTS:\n");
            for comment in analysis.comments.iter().take(20) {  // Limit to avoid excessive length
                if !comment.trim().is_empty() && comment.len() > 3 {
                    output.push_str(&format!("• {}\n", comment));
                }
            }
            if analysis.comments.len() > 20 {
                output.push_str(&format!("• ... and {} more comments\n", analysis.comments.len() - 20));
            }
        }

        // Add structural insights with logical grouping
        if !analysis.structural_insights.sections.is_empty() {
            output.push_str("\nCODE STRUCTURE:\n");

            // Organize sections in a logical order
            let section_order = ["entry_points", "imports", "functions", "io_operations", "tests"];

            for section_name in &section_order {
                if let Some(items) = analysis.structural_insights.sections.get(*section_name) {
                    if !items.is_empty() {
                        for item in items.iter().take(6) {  // Limit per section to avoid noise
                            output.push_str(&format!("{}\n", item));
                        }
                        if items.len() > 6 {
                            output.push_str(&format!("• ... and {} more {}\n", items.len() - 6, section_name));
                        }
                    }
                }
            }

            // Add any remaining sections (limited)
            for (section_name, items) in &analysis.structural_insights.sections {
                if !section_order.contains(&section_name.as_str()) && !items.is_empty() {
                    for item in items.iter().take(3) {  // Even more limited for misc sections
                        output.push_str(&format!("{}\n", item));
                    }
                }
            }
        }

        output.push('\n');
        output
    }
}