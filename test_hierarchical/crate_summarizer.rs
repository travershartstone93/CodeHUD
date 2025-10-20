//! Crate-Level Summarization for Hierarchical LLM Analysis
//!
//! This module implements hierarchical summarization by grouping files into crates
//! and generating focused summaries for each crate before final project analysis.

use crate::{LlmResult, LlmError, FileProcessor, ProcessorConfig};
use crate::comment_extractor::{FileCommentExtraction, StructuralInsights};
use crate::denoiser::{LlmContextDenoiser, DenoiserConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Summary of a single crate with its analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrateSummary {
    /// Name of the crate
    pub crate_name: String,
    /// Path to the crate directory
    pub crate_path: PathBuf,
    /// Files analyzed in this crate
    pub files_analyzed: Vec<String>,
    /// LLM-generated summary text
    pub summary_text: String,
    /// Aggregated structural insights
    pub structural_insights: StructuralInsights,
    /// Token count of the summary
    pub token_count: usize,
    /// Processing timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Cleaned file data for crate summarization input (Stage 1 denoising)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanedFileData {
    /// Relative file path
    pub file_path: String,
    /// Essential comments (denoised)
    pub key_comments: Vec<String>,
    /// Preserved structural insights
    pub structural_insights: Option<StructuralInsights>,
    /// File purpose/role summary
    pub file_summary: Option<String>,
}

/// Crate grouping and discovery
#[derive(Debug, Clone)]
pub struct CrateGrouper {
    /// Root project path
    project_path: PathBuf,
    /// Discovered crates
    crates: Vec<CrateInfo>,
}

/// Information about a discovered crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    /// Crate name from Cargo.toml
    pub name: String,
    /// Path to crate directory
    pub path: PathBuf,
    /// Crate description from Cargo.toml
    pub description: Option<String>,
    /// Crate version
    pub version: String,
    /// Files belonging to this crate
    pub files: Vec<PathBuf>,
}

/// Main crate summarization engine
pub struct CrateSummarizer {
    /// File processor for LLM calls
    processor: std::sync::Arc<FileProcessor>,
    /// Denoiser for stage 1 cleaning
    denoiser: LlmContextDenoiser,
    /// Configuration
    config: CrateSummarizerConfig,
}

/// Configuration for crate summarization
#[derive(Debug, Clone)]
pub struct CrateSummarizerConfig {
    /// Maximum tokens per crate summary
    pub max_tokens_per_crate: usize,
    /// Whether to include code context in summaries
    pub include_code_context: bool,
    /// Whether to analyze inter-crate dependencies
    pub analyze_dependencies: bool,
    /// Denoising aggressiveness (0.0 to 1.0)
    pub denoising_level: f32,
}

impl Default for CrateSummarizerConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_crate: 4000,
            include_code_context: true,
            analyze_dependencies: true,
            denoising_level: 0.4, // 40% reduction for crate inputs
        }
    }
}

impl CrateGrouper {
    /// Create a new crate grouper for the project
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            crates: Vec::new(),
        }
    }

    /// Discover all crates in the project
    pub fn discover_crates(&mut self) -> LlmResult<Vec<CrateInfo>> {
        let mut crates = Vec::new();

        // Find all Cargo.toml files
        for entry in WalkDir::new(&self.project_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "Cargo.toml" {
                let cargo_path = entry.path();
                let crate_dir = cargo_path.parent().unwrap();

                // Skip tree-sitter grammars and other non-main crates
                if crate_dir.to_string_lossy().contains("tree-sitter-grammars") {
                    continue;
                }

                match self.parse_cargo_toml(cargo_path) {
                    Ok(crate_info) => {
                        println!("📦 Discovered crate: {} at {}", crate_info.name, crate_info.path.display());
                        crates.push(crate_info);
                    }
                    Err(e) => {
                        println!("⚠️  Failed to parse {}: {}", cargo_path.display(), e);
                    }
                }
            }
        }

        // Sort crates by name for consistent processing order
        crates.sort_by(|a, b| a.name.cmp(&b.name));

        self.crates = crates.clone();
        Ok(crates)
    }

    /// Parse Cargo.toml to extract crate information
    fn parse_cargo_toml(&self, cargo_path: &Path) -> LlmResult<CrateInfo> {
        let content = std::fs::read_to_string(cargo_path)
            .map_err(|e| LlmError::Io(e))?;

        let cargo_toml: toml::Value = content.parse()
            .map_err(|e| LlmError::Config(format!("Failed to parse Cargo.toml: {}", e)))?;

        let package = cargo_toml.get("package")
            .ok_or_else(|| LlmError::Config("No [package] section in Cargo.toml".to_string()))?;

        let name = package.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::Config("No package name in Cargo.toml".to_string()))?
            .to_string();

        let description = package.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let version = package.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0")
            .to_string();

        let crate_dir = cargo_path.parent().unwrap().to_path_buf();

        Ok(CrateInfo {
            name,
            path: crate_dir,
            description,
            version,
            files: Vec::new(), // Will be populated later
        })
    }

    /// Group files by their containing crate
    pub fn group_files(&mut self, file_extractions: &[FileCommentExtraction]) -> LlmResult<HashMap<String, Vec<FileCommentExtraction>>> {
        let mut grouped = HashMap::new();

        for extraction in file_extractions {
            let file_path = Path::new(&extraction.file);
            let crate_name = self.find_crate_for_file(file_path)?;

            grouped.entry(crate_name)
                .or_insert_with(Vec::new)
                .push(extraction.clone());
        }

        // Update crate file lists
        for (crate_name, extractions) in &grouped {
            if let Some(crate_info) = self.crates.iter_mut().find(|c| &c.name == crate_name) {
                crate_info.files = extractions.iter()
                    .map(|e| PathBuf::from(&e.file))
                    .collect();
            }
        }

        Ok(grouped)
    }

    /// Find which crate a file belongs to
    fn find_crate_for_file(&self, file_path: &Path) -> LlmResult<String> {
        // Convert to absolute path if needed
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_path.join(file_path)
        };

        // Find the crate that contains this file
        let mut best_match: Option<&CrateInfo> = None;
        let mut best_depth = 0;

        for crate_info in &self.crates {
            if let Ok(relative) = abs_path.strip_prefix(&crate_info.path) {
                let depth = relative.components().count();
                if best_match.is_none() || depth < best_depth {
                    best_match = Some(crate_info);
                    best_depth = depth;
                }
            }
        }

        match best_match {
            Some(crate_info) => Ok(crate_info.name.clone()),
            None => {
                // Default to "workspace" for files not in any specific crate
                Ok("workspace".to_string())
            }
        }
    }

    /// Get discovered crate information
    pub fn get_crates(&self) -> &[CrateInfo] {
        &self.crates
    }
}

impl CrateSummarizer {
    /// Create a new crate summarizer
    pub fn new(processor: std::sync::Arc<FileProcessor>, config: CrateSummarizerConfig) -> Self {
        let denoiser_config = DenoiserConfig {
            target_reduction: config.denoising_level,
            preserve_structural_insights: true,
            preserve_metadata: false, // Strip metadata for crate summaries
            ..Default::default()
        };

        let denoiser = LlmContextDenoiser::new(denoiser_config);

        Self {
            processor,
            denoiser,
            config,
        }
    }

    /// Generate summary for a single crate
    pub async fn generate_crate_summary(
        &mut self,
        crate_info: &CrateInfo,
        crate_files: &[FileCommentExtraction],
    ) -> LlmResult<CrateSummary> {
        println!("🤖 Generating summary for crate: {} ({} files)", crate_info.name, crate_files.len());

        // Stage 1 denoising: Clean files for crate summary input
        let cleaned_files = self.clean_files_for_crate_summary(crate_files)?;

        // Build crate summary prompt
        let prompt = self.build_crate_summary_prompt(crate_info, &cleaned_files);

        // Check token budget (4K limit per crate)
        let prompt_tokens = prompt.len() / 4; // Rough estimate
        if prompt_tokens > self.config.max_tokens_per_crate {
            println!("⚠️  Prompt too large ({} tokens), applying aggressive denoising", prompt_tokens);
            // Apply more aggressive denoising if needed
            let aggressive_cleaned = self.apply_aggressive_denoising(&cleaned_files)?;
            let reduced_prompt = self.build_crate_summary_prompt(crate_info, &aggressive_cleaned);

            // Generate summary via LLM with 4K budget
            let summary_text = self.processor.generate_text_summary(&reduced_prompt).await?;
            let token_count = summary_text.len() / 4;

            println!("✅ Crate summary generated: {} tokens (from {} token prompt)", token_count, reduced_prompt.len() / 4);

            return Ok(CrateSummary {
                crate_name: crate_info.name.clone(),
                crate_path: crate_info.path.clone(),
                files_analyzed: crate_files.iter().map(|f| f.file.clone()).collect(),
                summary_text,
                structural_insights: self.aggregate_structural_insights(crate_files),
                token_count,
                timestamp: chrono::Utc::now(),
            });
        }

        // Generate summary via LLM within budget
        let summary_text = self.processor.generate_text_summary(&prompt).await?;

        // Calculate actual token count
        let token_count = summary_text.len() / 4;

        // Aggregate structural insights
        let structural_insights = self.aggregate_structural_insights(crate_files);

        println!("✅ Crate summary generated: {} tokens", token_count);

        Ok(CrateSummary {
            crate_name: crate_info.name.clone(),
            crate_path: crate_info.path.clone(),
            files_analyzed: crate_files.iter().map(|f| f.file.clone()).collect(),
            summary_text,
            structural_insights,
            token_count,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Generate context-aware crate summary using project memory
    pub async fn generate_crate_summary_with_context(
        &mut self,
        crate_info: &CrateInfo,
        crate_files: &[FileCommentExtraction],
        project_memory: &crate::conversation::ProjectAnalysisMemory,
    ) -> LlmResult<CrateSummary> {
        println!("🧠 Generating context-aware summary for crate: {} (with {} previous insights)",
            crate_info.name, project_memory.processed_crates.len());

        // Stage 1 denoising: Clean files for crate summary input
        let cleaned_files = self.clean_files_for_crate_summary(crate_files)?;

        // Build context-aware prompt
        let prompt = self.build_context_aware_prompt(crate_info, &cleaned_files, project_memory);

        // Check token budget (4K limit per crate)
        let prompt_tokens = prompt.len() / 4;
        if prompt_tokens > self.config.max_tokens_per_crate {
            println!("⚠️  Context-aware prompt too large ({} tokens), reducing context", prompt_tokens);
            let reduced_prompt = self.build_reduced_context_prompt(crate_info, &cleaned_files, project_memory);

            let summary_text = self.processor.generate_text_summary(&reduced_prompt).await?;
            let token_count = summary_text.len() / 4;

            println!("✅ Context-aware crate summary generated: {} tokens", token_count);

            return Ok(CrateSummary {
                crate_name: crate_info.name.clone(),
                crate_path: crate_info.path.clone(),
                files_analyzed: crate_files.iter().map(|f| f.file.clone()).collect(),
                summary_text,
                structural_insights: self.aggregate_structural_insights(crate_files),
                token_count,
                timestamp: chrono::Utc::now(),
            });
        }

        // Generate summary via LLM with context
        let summary_text = self.processor.generate_text_summary(&prompt).await?;
        let token_count = summary_text.len() / 4;

        println!("✅ Context-aware crate summary generated: {} tokens", token_count);

        Ok(CrateSummary {
            crate_name: crate_info.name.clone(),
            crate_path: crate_info.path.clone(),
            files_analyzed: crate_files.iter().map(|f| f.file.clone()).collect(),
            summary_text,
            structural_insights: self.aggregate_structural_insights(crate_files),
            token_count,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Clean files for crate summary input (Stage 1 denoising)
    fn clean_files_for_crate_summary(&mut self, files: &[FileCommentExtraction]) -> LlmResult<Vec<CleanedFileData>> {
        let mut cleaned_files = Vec::new();

        // Apply denoising to reduce token count
        let (denoised_files, _stats) = self.denoiser.denoise_extractions(files);

        for extraction in denoised_files {
            // Extract key comments (non-empty, meaningful)
            let key_comments: Vec<String> = extraction.comments.iter()
                .map(|c| c.text.clone())
                .filter(|text| !text.trim().is_empty() && text.len() > 5)
                .collect();

            let cleaned = CleanedFileData {
                file_path: extraction.file.clone(),
                key_comments,
                structural_insights: extraction.structural_insights,
                file_summary: None, // Could be added later
            };

            cleaned_files.push(cleaned);
        }

        Ok(cleaned_files)
    }

    /// Build prompt for crate summary generation
    fn build_crate_summary_prompt(&self, crate_info: &CrateInfo, cleaned_files: &[CleanedFileData]) -> String {
        let mut prompt = format!(
            "CRATE ANALYSIS: {}\n",
            crate_info.name.to_uppercase()
        );

        if let Some(description) = &crate_info.description {
            prompt.push_str(&format!("Description: {}\n", description));
        }

        prompt.push_str(&format!(
            "Path: {}\nFiles: {}\n\n",
            crate_info.path.display(),
            cleaned_files.len()
        ));

        prompt.push_str("ANALYSIS REQUIREMENTS:\n");
        prompt.push_str("1. **Purpose**: What is this crate's main responsibility?\n");
        prompt.push_str("2. **Architecture**: How is the code organized and structured?\n");
        prompt.push_str("3. **Key Components**: What are the main modules/types/functions?\n");
        prompt.push_str("4. **Dependencies**: What external dependencies does it use?\n");
        prompt.push_str("5. **Design Patterns**: What patterns and approaches are used?\n\n");

        prompt.push_str("FILE ANALYSIS:\n\n");

        for cleaned_file in cleaned_files {
            prompt.push_str(&format!("=== {} ===\n", cleaned_file.file_path));

            // Add structural insights if available
            if let Some(ref insights) = cleaned_file.structural_insights {
                for (section, items) in &insights.sections {
                    if !items.is_empty() {
                        prompt.push_str(&format!("{}:\n", section));
                        for item in items.iter().take(3) { // Limit items
                            prompt.push_str(&format!("  {}\n", item));
                        }
                    }
                }
            }

            // Add key comments (limited)
            if !cleaned_file.key_comments.is_empty() {
                prompt.push_str("Comments:\n");
                for comment in cleaned_file.key_comments.iter().take(3) {
                    prompt.push_str(&format!("  {}\n", comment));
                }
            }

            prompt.push_str("\n");
        }

        prompt.push_str(&format!(
            "\nPROVIDE CRATE SUMMARY:\nGenerate a focused analysis of the '{}' crate covering all requirements above. ",
            crate_info.name
        ));
        prompt.push_str("Be specific about technologies, patterns, and architectural decisions. ");
        prompt.push_str("Keep the summary under 1000 words for efficient processing.\n");

        prompt
    }

    /// Aggregate structural insights from all files in the crate
    fn aggregate_structural_insights(&self, files: &[FileCommentExtraction]) -> StructuralInsights {
        let mut aggregated_sections = HashMap::new();

        for file in files {
            if let Some(ref insights) = file.structural_insights {
                for (section_name, items) in &insights.sections {
                    let section_items = aggregated_sections.entry(section_name.clone())
                        .or_insert_with(Vec::new);

                    for item in items {
                        if !section_items.contains(item) {
                            section_items.push(item.clone());
                        }
                    }
                }
            }
        }

        // Limit items per section to avoid bloat
        for items in aggregated_sections.values_mut() {
            items.truncate(10);
        }

        StructuralInsights {
            source: "crate_aggregation".to_string(),
            generated: true,
            sections: aggregated_sections,
        }
    }

    /// Apply aggressive denoising for oversized prompts
    fn apply_aggressive_denoising(&self, cleaned_files: &[CleanedFileData]) -> LlmResult<Vec<CleanedFileData>> {
        let mut aggressive_cleaned = Vec::new();

        for file in cleaned_files {
            let aggressive_comments: Vec<String> = file.key_comments.iter()
                .filter(|comment| comment.len() > 20) // Only keep longer comments
                .take(2) // Maximum 2 comments per file
                .cloned()
                .collect();

            if !aggressive_comments.is_empty() {
                aggressive_cleaned.push(CleanedFileData {
                    file_path: file.file_path.clone(),
                    key_comments: aggressive_comments,
                    structural_insights: file.structural_insights.clone(),
                    file_summary: file.file_summary.clone(),
                });
            }
        }

        Ok(aggressive_cleaned)
    }

    /// Build context-aware prompt using project memory
    fn build_context_aware_prompt(
        &self,
        crate_info: &CrateInfo,
        cleaned_files: &[CleanedFileData],
        project_memory: &crate::conversation::ProjectAnalysisMemory,
    ) -> String {
        let mut prompt = format!(
            "CONTEXT-AWARE CRATE ANALYSIS: {}\n",
            crate_info.name.to_uppercase()
        );

        // Add project context from memory
        if !project_memory.processed_crates.is_empty() {
            prompt.push_str(&format!(
                "PREVIOUS ANALYSIS CONTEXT:\n- Processed crates: {}\n- Technology stack: {}\n- Patterns discovered: {}\n\n",
                project_memory.processed_crates.join(", "),
                project_memory.technology_stack.join(", "),
                project_memory.discovered_patterns.join(", ")
            ));
        }

        if let Some(description) = &crate_info.description {
            prompt.push_str(&format!("Description: {}\n", description));
        }

        prompt.push_str(&format!(
            "Path: {}\nFiles: {}\n\n",
            crate_info.path.display(),
            cleaned_files.len()
        ));

        prompt.push_str("CONTEXT-AWARE ANALYSIS REQUIREMENTS:\n");
        prompt.push_str("1. **Purpose**: How does this crate fit within the overall project architecture?\n");
        prompt.push_str("2. **Relationships**: How does this crate relate to previously analyzed crates?\n");
        prompt.push_str("3. **Patterns**: What design patterns does this crate implement/extend?\n");
        prompt.push_str("4. **Integration**: How does this crate integrate with the technology stack?\n");
        prompt.push_str("5. **Unique Value**: What unique capabilities does this crate provide?\n\n");

        // Limit files to fit within token budget
        let max_files = if project_memory.processed_crates.is_empty() { 8 } else { 6 };

        prompt.push_str("FILE ANALYSIS:\n\n");

        for (i, cleaned_file) in cleaned_files.iter().take(max_files).enumerate() {
            prompt.push_str(&format!("=== File {} ===\n", i + 1));

            // Add structural insights if available (limited)
            if let Some(ref insights) = cleaned_file.structural_insights {
                for (section, items) in &insights.sections {
                    if !items.is_empty() {
                        prompt.push_str(&format!("{}:\n", section));
                        for item in items.iter().take(2) { // Reduce items per section
                            prompt.push_str(&format!("  {}\\n", item));
                        }
                    }
                }
            }

            // Add key comments (limited)
            if !cleaned_file.key_comments.is_empty() {
                prompt.push_str("Key Comments:\n");
                for comment in cleaned_file.key_comments.iter().take(2) {
                    prompt.push_str(&format!("  {}\\n", comment));
                }
            }

            prompt.push_str("\\n");
        }

        prompt.push_str(&format!(
            "\\nGENERATE CONTEXT-AWARE CRATE SUMMARY:\\nAnalyze the '{}' crate considering the project context and relationships. ",
            crate_info.name
        ));
        prompt.push_str("Focus on integration patterns, architectural relationships, and unique value proposition. ");
        prompt.push_str("Limit response to 800 words for efficient hierarchical processing.\\n");

        prompt
    }

    /// Build reduced context prompt for oversized prompts
    fn build_reduced_context_prompt(
        &self,
        crate_info: &CrateInfo,
        cleaned_files: &[CleanedFileData],
        project_memory: &crate::conversation::ProjectAnalysisMemory,
    ) -> String {
        let mut prompt = format!(
            "CRATE ANALYSIS: {}\n",
            crate_info.name.to_uppercase()
        );

        // Add minimal context
        if !project_memory.processed_crates.is_empty() {
            prompt.push_str(&format!(
                "Context: Part of project with {} other crates\n",
                project_memory.processed_crates.len()
            ));
        }

        prompt.push_str(&format!("Files: {}\\n\\n", cleaned_files.len()));

        prompt.push_str("ANALYSIS REQUIREMENTS:\\n");
        prompt.push_str("1. Purpose and architecture\\n");
        prompt.push_str("2. Key components and patterns\\n");
        prompt.push_str("3. Integration approach\\n\\n");

        // Very limited file data
        for (i, cleaned_file) in cleaned_files.iter().take(4).enumerate() {
            prompt.push_str(&format!("File {}: Key insight only\\n", i + 1));
            if let Some(comment) = cleaned_file.key_comments.first() {
                prompt.push_str(&format!("  {}\\n", comment.chars().take(100).collect::<String>()));
            }
        }

        prompt.push_str(&format!(
            "\\nProvide focused analysis of '{}' crate. Limit to 600 words.\\n",
            crate_info.name
        ));

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_crate_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        // Create a simple Cargo.toml
        let cargo_content = r#"
[package]
name = "test-crate"
version = "0.1.0"
description = "A test crate"
"#;
        std::fs::write(project_path.join("Cargo.toml"), cargo_content).unwrap();

        let mut grouper = CrateGrouper::new(project_path);
        let crates = grouper.discover_crates().unwrap();

        assert_eq!(crates.len(), 1);
        assert_eq!(crates[0].name, "test-crate");
        assert_eq!(crates[0].description, Some("A test crate".to_string()));
    }

    #[test]
    fn test_file_grouping() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        let mut grouper = CrateGrouper::new(project_path.clone());

        // Mock file extractions
        let extractions = vec![
            FileCommentExtraction {
                file: project_path.join("src/main.rs").to_string_lossy().to_string(),
                language: "rust".to_string(),
                extraction_method: "test".to_string(),
                comments: vec![],
                structural_insights: None,
                stats: Default::default(),
            }
        ];

        let grouped = grouper.group_files(&extractions).unwrap();
        assert!(!grouped.is_empty());
    }
}