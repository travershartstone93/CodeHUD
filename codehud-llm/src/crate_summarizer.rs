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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Subcrate summaries (for large crates with subdirectories)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcrates: Option<HashMap<String, SubcrateSummary>>,
}

/// Summary of a subcrate (subdirectory within a crate) with recursive nesting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubcrateSummary {
    /// Name/path of the subcrate (e.g., "narrator/detectors")
    pub name: String,
    /// Number of direct files in this subcrate (not including nested subcrates)
    pub file_count: usize,
    /// Direct files in this subcrate
    pub files: Vec<String>,
    /// LLM-generated summary for this subcrate
    pub summary: String,
    /// Token count of the summary
    pub token_count: usize,
    /// Total size in kilobytes (for prioritization)
    pub total_size_kb: f64,
    /// Nested subcrates (recursive structure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcrates: Option<HashMap<String, SubcrateSummary>>,
}

/// Cleaned file data for crate summarization input (using file summaries, not raw comments)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanedFileData {
    /// Relative file path
    pub file_path: String,
    /// LLM-generated file summary (from file_summaries.json)
    pub file_summary: String,
    /// Preserved structural insights for technical details
    pub structural_insights: Option<StructuralInsights>,
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
    /// Project root path for relative path conversion
    project_path: PathBuf,
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
            max_tokens_per_crate: 8000,
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

        println!("🔍 DEBUG: Searching for Cargo.toml files in: {}", self.project_path.display());

        // Find all Cargo.toml files
        for entry in WalkDir::new(&self.project_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "Cargo.toml" {
                println!("🔍 DEBUG: Found Cargo.toml at: {}", entry.path().display());
                let cargo_path = entry.path();
                let crate_dir = cargo_path.parent().unwrap();

                // Skip tree-sitter grammars and ALL test directories
                if crate_dir.to_string_lossy().contains("tree-sitter-grammars") ||
                   crate_dir.to_string_lossy().contains("test_project_hierarchical") ||
                   crate_dir.to_string_lossy().contains("test_multi_crate") ||
                   crate_dir.to_string_lossy().contains("test_hierarchical") {
                    continue;
                }

                match self.parse_cargo_toml(cargo_path) {
                    Ok(crate_info) => {
                        // Check for duplicates before adding
                        if !crates.iter().any(|existing: &CrateInfo| existing.name == crate_info.name) {
                            println!("📦 Discovered crate: {} at {}", crate_info.name, crate_info.path.display());
                            crates.push(crate_info);
                        } else {
                            println!("⚠️  Skipping duplicate crate: {} at {}", crate_info.name, crate_info.path.display());
                        }
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

        println!("🔍 DEBUG: Grouping {} files into crates", file_extractions.len());

        for extraction in file_extractions {
            let file_path = Path::new(&extraction.file);
            let crate_name = self.find_crate_for_file(file_path)?;

            println!("🔍 DEBUG: File {} -> crate {}", extraction.file, crate_name);

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

        // Debug output for final grouping
        println!("🔍 DEBUG: Final grouping results:");
        for (crate_name, extractions) in &grouped {
            println!("🔍 DEBUG:   {} -> {} files", crate_name, extractions.len());
        }

        Ok(grouped)
    }

    /// Find which crate a file belongs to
    fn find_crate_for_file(&self, file_path: &Path) -> LlmResult<String> {
        // Convert to absolute path and canonicalize to resolve symlinks
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_path.join(file_path)
        };

        // Canonicalize paths to resolve any symlinks or '..' components
        let canonical_file_path = abs_path.canonicalize()
            .unwrap_or_else(|_| abs_path.clone());

        // Find the crate that contains this file
        let mut best_match: Option<&CrateInfo> = None;
        let mut best_depth = usize::MAX;

        for crate_info in &self.crates {
            // Make crate path absolute and canonical for comparison
            let abs_crate_path = if crate_info.path.is_absolute() {
                crate_info.path.clone()
            } else {
                self.project_path.join(&crate_info.path)
            };

            let canonical_crate_path = abs_crate_path.canonicalize()
                .unwrap_or_else(|_| abs_crate_path.clone());

            println!("🔍 DEBUG: Checking file {} against crate {} path {}",
                canonical_file_path.display(), crate_info.name, canonical_crate_path.display());

            if let Ok(relative) = canonical_file_path.strip_prefix(&canonical_crate_path) {
                println!("🔍 DEBUG: ✅ MATCH! File {} matches crate {} (relative: {})",
                    canonical_file_path.display(), crate_info.name, relative.display());
                let depth = relative.components().count();
                if depth < best_depth {
                    best_match = Some(crate_info);
                    best_depth = depth;
                }
            } else {
                println!("🔍 DEBUG: ❌ No match: {} not under {}",
                    canonical_file_path.display(), canonical_crate_path.display());
            }
        }

        match best_match {
            Some(crate_info) => {
                println!("🔍 DEBUG: ✅ Final match: {} -> crate {}", canonical_file_path.display(), crate_info.name);
                Ok(crate_info.name.clone())
            }
            None => {
                println!("🔍 DEBUG: ❌ No match found for file {}, defaulting to workspace", canonical_file_path.display());
                println!("🔍 DEBUG: Available crates:");
                for crate_info in &self.crates {
                    let abs_crate_path = if crate_info.path.is_absolute() {
                        crate_info.path.clone()
                    } else {
                        self.project_path.join(&crate_info.path)
                    };
                    let canonical_crate_path = abs_crate_path.canonicalize()
                        .unwrap_or_else(|_| abs_crate_path.clone());
                    println!("🔍 DEBUG:   - {} at {}", crate_info.name, canonical_crate_path.display());
                }
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
    pub fn new(processor: std::sync::Arc<FileProcessor>, config: CrateSummarizerConfig, project_path: PathBuf) -> Self {
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
            project_path,
        }
    }

    /// Convert full path to project-relative path for token efficiency
    fn make_relative_path(&self, full_path: &str) -> String {
        let path = Path::new(full_path);
        if let Ok(relative) = path.strip_prefix(&self.project_path) {
            relative.to_string_lossy().to_string()
        } else {
            // Fallback: just use filename if strip_prefix fails
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(full_path)
                .to_string()
        }
    }

    /// Convert crate path to project-relative path
    fn make_relative_crate_path(&self, crate_path: &PathBuf) -> PathBuf {
        if let Ok(relative) = crate_path.strip_prefix(&self.project_path) {
            relative.to_path_buf()
        } else {
            crate_path.clone()
        }
    }

    /// Estimate token count for a prompt (1 token ≈ 4 characters)
    fn estimate_tokens(&self, prompt: &str) -> usize {
        prompt.len() / 4
    }

    /// HARDCODED: Always use 14B model for better semantic understanding
    /// Only fallback to Gemini for prompts > 28K tokens
    async fn generate_with_routing(&self, prompt: &str) -> LlmResult<String> {
        let estimated_tokens = self.estimate_tokens(prompt);

        if estimated_tokens > 28000 {
            // Use Gemini for very large prompts (>28K tokens)
            println!("🌟 Prompt {} tokens > 28K, routing to Gemini Flash", estimated_tokens);

            // Check if Gemini is available (via environment variable)
            if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
                return self.generate_with_gemini(prompt, &api_key).await;
            } else {
                println!("⚠️  Gemini not available (no GEMINI_API_KEY), falling back to 14B model");
                return self.generate_with_14b(prompt).await;
            }
        } else {
            // HARDCODED: Always use 14B model for all prompts <= 28K tokens
            println!("🚀 Using 14B model ({} tokens, 16K context)", estimated_tokens);
            return self.generate_with_14b(prompt).await;
        }
    }

    /// Generate summary using 14B model (32K context window)
    async fn generate_with_14b(&self, prompt: &str) -> LlmResult<String> {
        let system_prompt = "You are an expert software architect. Analyze the complete system architecture, component interactions, and unified capabilities. Provide comprehensive, detailed analysis.";

        let client = reqwest::Client::new();
        let request = serde_json::json!({
            "model": "qwen2.5-coder:14b-instruct-q4_K_M",
            "prompt": prompt,
            "system": system_prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "top_p": 0.9,
                "top_k": 40,
                "num_predict": 2048,  // 2K tokens for complete, detailed crate summaries
                "num_ctx": 16384  // 16K context for crate summaries
            }
        });

        let response = client
            .post("http://localhost:11434/api/generate")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Http(e))?;

        if !response.status().is_success() {
            return Err(LlmError::Inference(format!(
                "Ollama 14B generation failed: {}",
                response.status()
            )));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| LlmError::Http(e))?;

        let generated_text = response_json["response"]
            .as_str()
            .ok_or_else(|| LlmError::Inference("No response field in Ollama response".to_string()))?
            .to_string();

        Ok(generated_text)
    }

    /// Generate summary using Gemini Flash (1M context window)
    async fn generate_with_gemini(&self, prompt: &str, api_key: &str) -> LlmResult<String> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": 0.7,
                "topP": 0.9,
                "topK": 40,
                "maxOutputTokens": 2048,  // 2K tokens for complete, detailed crate summaries
            }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            api_key
        );

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| LlmError::Http(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::Inference(format!(
                "Gemini API failed: {}",
                error_text
            )));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| LlmError::Http(e))?;

        // Extract text from Gemini response
        let generated_text = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| LlmError::Inference("No text in Gemini response".to_string()))?
            .to_string();

        Ok(generated_text)
    }

    /// Generate summary for a single crate
    pub async fn generate_crate_summary(
        &mut self,
        crate_info: &CrateInfo,
        crate_files: &[FileCommentExtraction],
        output_dir: &Path,
    ) -> LlmResult<CrateSummary> {
        println!("🤖 Generating summary for crate: {} ({} files)", crate_info.name, crate_files.len());

        // Load file summaries (hierarchical: file summaries → crate summary)
        let cleaned_files = self.load_file_summaries_for_crate(crate_files, output_dir)?;

        // Build crate summary prompt
        let prompt = self.build_crate_summary_prompt(crate_info, &cleaned_files);

        // Check token budget (8K limit per crate)
        let prompt_tokens = prompt.len() / 4; // Rough estimate
        if prompt_tokens > self.config.max_tokens_per_crate {
            println!("⚠️  Prompt too large ({} tokens), applying aggressive denoising", prompt_tokens);
            // Apply more aggressive denoising if needed
            let aggressive_cleaned = self.apply_aggressive_denoising(&cleaned_files)?;
            let reduced_prompt = self.build_crate_summary_prompt(crate_info, &aggressive_cleaned);

            // Generate summary via LLM with routing based on prompt size
            let summary_text = self.generate_with_routing(&reduced_prompt).await?;
            let token_count = summary_text.len() / 4;

            println!("✅ Crate summary generated: {} tokens (from {} token prompt)", token_count, reduced_prompt.len() / 4);

            return Ok(CrateSummary {
                crate_name: crate_info.name.clone(),
                crate_path: self.make_relative_crate_path(&crate_info.path),
                files_analyzed: crate_files.iter().map(|f| self.make_relative_path(&f.file)).collect(),
                summary_text,
                structural_insights: self.aggregate_structural_insights(crate_files),
                token_count,
                timestamp: chrono::Utc::now(),
                subcrates: None,
            });
        }

        // Generate summary via LLM with routing based on prompt size
        let summary_text = self.generate_with_routing(&prompt).await?;

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
            subcrates: None,
        })
    }

    /// Generate context-aware crate summary using project memory
    pub async fn generate_crate_summary_with_context(
        &mut self,
        crate_info: &CrateInfo,
        crate_files: &[FileCommentExtraction],
        project_memory: &crate::conversation::ProjectAnalysisMemory,
        output_dir: &Path,
        subcrate_summaries: Option<HashMap<String, SubcrateSummary>>,
    ) -> LlmResult<CrateSummary> {
        println!("🧠 Generating context-aware summary for crate: {} (with {} previous insights)",
            crate_info.name, project_memory.processed_crates.len());

        // Load file summaries (hierarchical: file summaries → crate summary)
        let all_cleaned_files = self.load_file_summaries_for_crate(crate_files, output_dir)?;

        // Filter files based on whether they belong to subcrates
        let (subcrate_files, individual_files) = if let Some(ref subcrates) = subcrate_summaries {
            let subcrate_file_set: std::collections::HashSet<String> = subcrates
                .values()
                .flat_map(|s| self.collect_all_files_recursive(s))
                .collect();

            let individual: Vec<CleanedFileData> = all_cleaned_files
                .iter()
                .filter(|f| !subcrate_file_set.contains(&self.make_relative_path(&f.file_path)))
                .cloned()
                .collect();

            println!("📦 Using subcrate summaries: {} subcrates, {} individual files, {} files in subcrates",
                subcrates.len(), individual.len(), subcrate_file_set.len());

            (subcrate_file_set.len(), individual)
        } else {
            println!("📄 No subcrates, using all {} files individually", all_cleaned_files.len());
            (0, all_cleaned_files)
        };

        // Build context-aware prompt with subcrates
        let prompt = self.build_context_aware_prompt_with_subcrates(
            crate_info,
            &individual_files,
            &subcrate_summaries,
            project_memory
        );

        // Check token budget (8K limit per crate)
        let prompt_tokens = prompt.len() / 4;
        if prompt_tokens > self.config.max_tokens_per_crate {
            println!("⚠️  Context-aware prompt too large ({} tokens), reducing context", prompt_tokens);
            let reduced_prompt = self.build_reduced_context_prompt_with_subcrates(
                crate_info,
                &individual_files,
                &subcrate_summaries,
                project_memory
            );

            let summary_text = self.generate_with_routing(&reduced_prompt).await?;
            let token_count = summary_text.len() / 4;

            println!("✅ Context-aware crate summary generated: {} tokens", token_count);

            return Ok(CrateSummary {
                crate_name: crate_info.name.clone(),
                crate_path: self.make_relative_crate_path(&crate_info.path),
                files_analyzed: crate_files.iter().map(|f| self.make_relative_path(&f.file)).collect(),
                summary_text,
                structural_insights: self.aggregate_structural_insights(crate_files),
                token_count,
                timestamp: chrono::Utc::now(),
                subcrates: subcrate_summaries.clone(),
            });
        }

        // Generate summary via LLM with context and routing based on prompt size
        let summary_text = self.generate_with_routing(&prompt).await?;
        let token_count = summary_text.len() / 4;

        println!("✅ Context-aware crate summary generated: {} tokens", token_count);

        Ok(CrateSummary {
            crate_name: crate_info.name.clone(),
            crate_path: self.make_relative_crate_path(&crate_info.path),
            files_analyzed: crate_files.iter().map(|f| self.make_relative_path(&f.file)).collect(),
            summary_text,
            structural_insights: self.aggregate_structural_insights(crate_files),
            token_count,
            timestamp: chrono::Utc::now(),
            subcrates: subcrate_summaries,
        })
    }

    /// Load file summaries for crate summary input (hierarchical: file summaries → crate summary)
    pub fn load_file_summaries_for_crate(&mut self, files: &[FileCommentExtraction], output_dir: &Path) -> LlmResult<Vec<CleanedFileData>> {
        let mut cleaned_files = Vec::new();

        // Load all file summaries from disk
        let summaries_file = output_dir.join("file_summaries.json");
        if !summaries_file.exists() {
            return Err(LlmError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file_summaries.json not found at {:?} - file summaries must be generated first", summaries_file)
            )));
        }

        let content = std::fs::read_to_string(&summaries_file)?;
        let all_file_summaries: Vec<(String, String)> = serde_json::from_str(&content)
            .map_err(|e| LlmError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse file_summaries.json: {}", e)
            )))?;

        // Create a lookup map for file summaries
        let summary_map: HashMap<String, String> = all_file_summaries.into_iter().collect();

        // Build cleaned file data using file summaries + structural insights
        for extraction in files {
            // Get the file summary (required)
            let file_summary = summary_map.get(&extraction.file)
                .ok_or_else(|| LlmError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("No file summary found for: {}", extraction.file)
                )))?
                .clone();

            let cleaned = CleanedFileData {
                file_path: extraction.file.clone(),
                file_summary,
                structural_insights: extraction.structural_insights.clone(),
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
        prompt.push_str("1. **Overall Architecture**: How does this crate fit within the project? What design patterns, dependencies, and structural organization does it use?\n");
        prompt.push_str("2. **What Does It Actually Do**: What is the actual functionality and what the features and overall intention of the program is?\n\n");

        prompt.push_str("FILE SUMMARIES AND TECHNICAL DETAILS:\n\n");

        for cleaned_file in cleaned_files {
            prompt.push_str(&format!("=== {} ===\n", cleaned_file.file_path));

            // Add LLM-generated file summary (includes synthesized structural insights)
            prompt.push_str("File Summary:\n");
            prompt.push_str(&format!("  {}\n\n", cleaned_file.file_summary));

            // Raw structural insights removed - already synthesized in file summaries
            // This prevents redundancy and token waste in crate-level aggregation

            prompt.push_str("\n");
        }

        prompt.push_str(&format!(
            "\nPROVIDE CRATE SUMMARY:\nGenerate a focused analysis of the '{}' crate with exactly TWO sections:\n",
            crate_info.name
        ));
        prompt.push_str("## Overall Architecture\n[How this crate fits in the project, design patterns, dependencies, structure]\n\n");
        prompt.push_str("## What Does It Actually Do\n[Actual functionality and what the features and overall intention of the program is]\n\n");
        prompt.push_str("Keep the summary under 800 words total for efficient processing.\n");

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

    /// Apply aggressive denoising for oversized prompts (limit number of files)
    fn apply_aggressive_denoising(&self, cleaned_files: &[CleanedFileData]) -> LlmResult<Vec<CleanedFileData>> {
        // Simply limit to most important files (first N files)
        // File summaries are already concise, so we just need to limit count
        let max_files = 5;
        Ok(cleaned_files.iter().take(max_files).cloned().collect())
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
        prompt.push_str("1. **Overall Architecture**: How does this crate fit within the project architecture, relate to other crates, use design patterns, and integrate with the technology stack?\n");
        prompt.push_str("2. **What Does It Actually Do**: What is the actual functionality and what the features and overall intention of the program is?\n\n");

        // Limit files to fit within token budget
        let max_files = if project_memory.processed_crates.is_empty() { 8 } else { 6 };

        prompt.push_str("FILE SUMMARIES AND TECHNICAL DETAILS:\n\n");

        for (i, cleaned_file) in cleaned_files.iter().take(max_files).enumerate() {
            prompt.push_str(&format!("=== File {}: {} ===\n", i + 1, cleaned_file.file_path));

            // Add LLM-generated file summary (primary source)
            prompt.push_str("File Summary:\n");
            prompt.push_str(&format!("  {}\n\n", cleaned_file.file_summary));

            // Add structural insights for technical details
            if let Some(ref insights) = cleaned_file.structural_insights {
                prompt.push_str("Technical Details:\n");
                for (section, items) in &insights.sections {
                    if !items.is_empty() {
                        prompt.push_str(&format!("  {}:\n", section));
                        for item in items.iter().take(15) {
                            prompt.push_str(&format!("    {}\n", item));
                        }
                    }
                }
            }

            prompt.push_str("\n");
        }

        prompt.push_str("\n");
        prompt.push_str("GENERATE CONTEXT-AWARE CRATE SUMMARY:\n");
        prompt.push_str(&format!("Analyze the '{}' crate with exactly TWO sections:\n", crate_info.name));
        prompt.push_str("## Overall Architecture\n[Project fit, relationships to other crates, patterns, technology integration]\n\n");
        prompt.push_str("## What Does It Actually Do\n[Actual functionality and what the features and overall intention of the program is]\n\n");
        prompt.push_str("Keep the summary under 800 words total for efficient processing.\n");

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

        prompt.push_str(&format!("Files: {}\n\n", cleaned_files.len()));

        prompt.push_str("ANALYSIS REQUIREMENTS:\n");
        prompt.push_str("1. Purpose and architecture\n");
        prompt.push_str("2. Key components and patterns\n");
        prompt.push_str("3. Integration approach\n\n");

        // Very limited file data (just first line of summary)
        for (i, cleaned_file) in cleaned_files.iter().take(4).enumerate() {
            prompt.push_str(&format!("File {}: ", i + 1));
            // Take first sentence of file summary
            let first_sentence = cleaned_file.file_summary
                .split('.')
                .next()
                .unwrap_or(&cleaned_file.file_summary);
            prompt.push_str(&format!("{}\n", first_sentence.chars().take(100).collect::<String>()));
        }

        prompt.push_str("\n");
        prompt.push_str(&format!("Provide focused analysis of '{}' crate. Limit to 600 words.\n", crate_info.name));

        prompt
    }

    /// Generate summary using only structural insights (no comments) - ultra token efficient
    pub async fn generate_structural_insights_only_summary(
        &mut self,
        crate_info: &CrateInfo,
        crate_files: &[FileCommentExtraction],
    ) -> LlmResult<CrateSummary> {
        println!("🔍 Generating structural-insights-only summary for crate: {} ({} files)", crate_info.name, crate_files.len());

        // Build prompt using only structural insights
        let prompt = self.build_structural_insights_only_prompt(crate_info, crate_files);

        // Generate summary via LLM with routing based on prompt size
        let summary_text = self.generate_with_routing(&prompt).await?;
        let token_count = summary_text.len() / 4;

        println!("✅ Structural-insights-only summary generated: {} tokens", token_count);

        Ok(CrateSummary {
            crate_name: crate_info.name.clone(),
            crate_path: self.make_relative_crate_path(&crate_info.path),
            files_analyzed: crate_files.iter().map(|f| self.make_relative_path(&f.file)).collect(),
            summary_text,
            structural_insights: self.aggregate_structural_insights(crate_files),
            token_count,
            timestamp: chrono::Utc::now(),
            subcrates: None,
        })
    }

    /// Build prompt using only structural insights - extremely token efficient
    fn build_structural_insights_only_prompt(
        &self,
        crate_info: &CrateInfo,
        crate_files: &[FileCommentExtraction],
    ) -> String {
        let mut prompt = format!(
            "CRATE ANALYSIS FROM STRUCTURAL INSIGHTS ONLY\n\nCrate: {}\nFiles: {} files analyzed\n\nSTRUCTURAL INSIGHTS:\n",
            crate_info.name,
            crate_files.len()
        );

        // Aggregate all structural insights
        let aggregated = self.aggregate_structural_insights(crate_files);

        for (section, items) in &aggregated.sections {
            if !items.is_empty() {
                prompt.push_str(&format!("- {}: ", section.replace("_", " ")));
                let summary: Vec<String> = items.iter().take(10).map(|item| {
                    // Shorten long items for token efficiency
                    if item.len() > 100 {
                        format!("{}...", &item[..97])
                    } else {
                        item.clone()
                    }
                }).collect();
                prompt.push_str(&summary.join(", "));
                prompt.push('\n');
            }
        }

        prompt.push_str("\n");
        prompt.push_str(&format!("Provide a technical summary of the '{}' crate with exactly TWO sections:\n", crate_info.name));
        prompt.push_str("## Overall Architecture\n[List SPECIFIC external libraries/tools BY NAME that are mentioned in the summaries above. Then describe how this crate fits in the project]\n\n");
        prompt.push_str("## What Does It Actually Do\n[CRITICAL - Answer this FIRST: What PRIMARY OUTPUT does this crate produce for end users? If a user runs this crate, what do they GET? State the #1 USER-FACING CAPABILITY first (e.g., 'Generates X', 'Produces Y', 'Creates Z'), then list secondary features. Focus on OUTPUTS and DELIVERABLES, not internal operations.]\n\n");
        prompt.push_str("CRITICAL INSTRUCTIONS:\n");
        prompt.push_str("- Start with what users GET/RECEIVE from this crate (the output/deliverable)\n");
        prompt.push_str("- If this crate generates summaries/reports/analysis, STATE THAT FIRST\n");
        prompt.push_str("- If this crate produces visualizations/graphs/files, STATE THAT FIRST\n");
        prompt.push_str("- Only mention internal operations AFTER stating the primary output\n");
        prompt.push_str("Limit response to 250 words total.\n");

        prompt
    }

    /// Calculates total size in KB for a collection of files
    fn calculate_total_size_kb(&self, files: &[CleanedFileData]) -> f64 {
        files.iter()
            .filter_map(|f| std::fs::metadata(&f.file_path).ok())
            .map(|m| m.len() as f64 / 1024.0)
            .sum()
    }

    /// Detect subcrates recursively with unlimited nesting depth
    /// Returns a tree structure of subdirectories that meet the threshold (5+ files)
    fn detect_subcrates_recursive(
        &self,
        files: &[CleanedFileData],
        crate_path: &Path,
        base_dir: &Path,
    ) -> HashMap<String, SubcrateNode> {
        // Group files by immediate parent directory
        let mut dir_groups: HashMap<PathBuf, Vec<CleanedFileData>> = HashMap::new();

        for file in files {
            let file_path = Path::new(&file.file_path);
            if let Ok(relative) = file_path.strip_prefix(base_dir) {
                if let Some(parent) = relative.parent() {
                    if parent.components().count() > 0 {
                        // File is in a subdirectory
                        let immediate_parent = base_dir.join(parent.components().next().unwrap().as_os_str());
                        dir_groups.entry(immediate_parent).or_default().push(file.clone());
                    }
                }
            }
        }

        // Build subcrate nodes recursively
        let mut subcrates = HashMap::new();
        for (dir_path, dir_files) in dir_groups {
            let dir_name = dir_path.strip_prefix(crate_path)
                .unwrap_or(&dir_path)
                .to_string_lossy()
                .to_string();

            // Calculate directory depth relative to crate root
            let depth = Path::new(&dir_name).components().count();

            // Skip depth-1 directories (like src/, lib/, pkg/) - these are standard source directories
            // Files at depth 1 should be direct crate files, only depth 2+ are subcrates
            // This is polyglot: works for Rust (src/), Python (src/ or package/), Java (src/main/), etc.
            if depth == 1 {
                let direct_file_count = dir_files.iter().filter(|f| {
                    let p = Path::new(&f.file_path);
                    p.strip_prefix(&dir_path).ok()
                        .and_then(|rel| rel.parent())
                        .map(|parent| parent.components().count() == 0)
                        .unwrap_or(false)
                }).count();

                println!("📦 Skipping depth-1 directory '{}/' (standard source dir, not a subcrate)", dir_name);
                println!("   → {} total files: {} direct files at depth-1, {} in deeper subdirs",
                    dir_files.len(), direct_file_count, dir_files.len() - direct_file_count);

                // Recursively detect subcrates WITHIN this depth-1 dir (e.g., src/narrator/, pkg/utils/)
                let nested = self.detect_subcrates_recursive(&dir_files, crate_path, &dir_path);

                // Add nested subcrates directly (they're already relative to crate_path from the recursive call)
                for (nested_name, nested_node) in nested {
                    subcrates.insert(nested_name, nested_node);
                }

                continue;
            }

            // Only create subcrate if it has 5+ files (threshold)
            if dir_files.len() >= 5 {
                let dir_name = dir_path.strip_prefix(crate_path)
                    .unwrap_or(&dir_path)
                    .to_string_lossy()
                    .to_string();

                // Find direct files in this directory (not in subdirectories)
                let direct_files: Vec<CleanedFileData> = dir_files.iter()
                    .filter(|f| {
                        let file_path = Path::new(&f.file_path);
                        if let Ok(relative) = file_path.strip_prefix(&dir_path) {
                            relative.parent().map(|p| p.components().count() == 0).unwrap_or(true)
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                // Recursively find nested subcrates
                let nested_subcrates = self.detect_subcrates_recursive(&dir_files, crate_path, &dir_path);

                // Calculate total size in KB
                let total_size_kb = self.calculate_total_size_kb(&dir_files);

                subcrates.insert(dir_name.clone(), SubcrateNode {
                    name: dir_name,
                    direct_files,
                    all_files: dir_files,
                    nested_subcrates,
                    total_size_kb,
                });
            }
        }

        subcrates
    }

    /// Count all subcrates including nested ones
    fn count_all_subcrates(&self, nodes: &HashMap<String, SubcrateNode>) -> usize {
        let mut count = nodes.len();
        for node in nodes.values() {
            count += self.count_all_subcrates(&node.nested_subcrates);
        }
        count
    }

    /// Flatten all subcrates into a prioritized list for truncation
    /// Returns (full_path, size_kb, is_nested)
    fn flatten_subcrates_by_priority(
        &self,
        nodes: &HashMap<String, SubcrateNode>,
        parent_path: &str,
        is_nested: bool
    ) -> Vec<(String, f64, bool)> {
        let mut result = Vec::new();
        for (name, node) in nodes {
            let full_path = if parent_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", parent_path, name)
            };

            result.push((full_path.clone(), node.total_size_kb, is_nested));

            // Recursively add nested subcrates (marked as nested=true)
            let nested = self.flatten_subcrates_by_priority(
                &node.nested_subcrates,
                &full_path,
                true  // nested subcrates are marked as nested
            );
            result.extend(nested);
        }
        result
    }

    /// Generate subcrate summaries recursively (bottom-up traversal)
    pub async fn generate_subcrate_summaries(
        &mut self,
        crate_info: &CrateInfo,
        files: &[CleanedFileData],
    ) -> LlmResult<Option<HashMap<String, SubcrateSummary>>> {
        // Detect subcrates from the crate root, not hardcoded src/ directory
        let subcrate_nodes = self.detect_subcrates_recursive(files, &crate_info.path, &crate_info.path);

        if subcrate_nodes.is_empty() {
            println!("📦 No subcrates detected for {} (files: {})", crate_info.name, files.len());
            return Ok(None);
        }

        // Count all subcrates including nested
        let total_subcrates = self.count_all_subcrates(&subcrate_nodes);

        println!("📦 Detected {} subcrates for {} (including nested)", total_subcrates, crate_info.name);
        for (name, node) in &subcrate_nodes {
            println!("   - {} ({} direct files, {} nested, {:.1} KB)",
                name, node.direct_files.len(), node.nested_subcrates.len(), node.total_size_kb);
        }

        // Determine which subcrates to keep (max 10)
        let skip_paths: std::collections::HashSet<String> = if total_subcrates > 10 {
            let mut all_subcrates = self.flatten_subcrates_by_priority(&subcrate_nodes, "", false);

            // Sort by priority for TRUNCATION (will truncate from start):
            // 1. Nested subcrates first (is_nested=true)
            // 2. Within each group, smallest size first
            all_subcrates.sort_by(|a, b| {
                match (a.2, b.2) {
                    (true, false) => std::cmp::Ordering::Less,    // nested comes first (truncated first)
                    (false, true) => std::cmp::Ordering::Greater, // top-level comes last (kept)
                    _ => a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal),  // sort by size ascending
                }
            });

            let to_drop = total_subcrates - 10;
            let dropped: std::collections::HashSet<String> = all_subcrates.iter()
                .take(to_drop)
                .map(|(path, _, _)| path.clone())
                .collect();

            println!("⚠️  Truncating {} subcrates (keeping 10 largest):", to_drop);
            for (path, size, is_nested) in &all_subcrates[..to_drop] {
                println!("   ✗ {} ({:.1} KB) [{}]",
                    path, size, if *is_nested { "nested" } else { "top-level" });
            }

            dropped
        } else {
            std::collections::HashSet::new()
        };

        // Calculate per-subcrate token budget (increased to 800 max for complete summaries)
        let subcrates_to_generate = total_subcrates.min(10);
        let per_subcrate_budget = (5000 / subcrates_to_generate).min(800);

        println!("📊 Token budget: {} subcrates × {} tokens/subcrate = {} total (max 5000)",
            subcrates_to_generate, per_subcrate_budget, subcrates_to_generate * per_subcrate_budget);

        // Generate summaries (skipping truncated ones)
        let mut summaries = HashMap::new();
        for (name, node) in subcrate_nodes {
            if !skip_paths.contains(&name) {
                println!("📝 Generating summary for subcrate: {} ({} files, {:.1} KB)",
                    name, node.direct_files.len(), node.total_size_kb);
                let summary = self.generate_subcrate_summary_recursive(
                    crate_info,
                    &name,
                    &node,
                    per_subcrate_budget,
                    &skip_paths
                ).await?;
                println!("✅ Generated subcrate summary: {} ({} tokens)", name, summary.token_count);
                summaries.insert(name, summary);
            } else {
                println!("⏭️  Skipped subcrate (truncated): {}", name);
            }
        }

        println!("📦 Total subcrate summaries generated: {}", summaries.len());
        Ok(Some(summaries))
    }

    /// Generate summary for a single subcrate (handles nesting recursively)
    fn generate_subcrate_summary_recursive<'a>(
        &'a mut self,
        crate_info: &'a CrateInfo,
        subcrate_name: &'a str,
        node: &'a SubcrateNode,
        token_budget: usize,
        skip_paths: &'a std::collections::HashSet<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = LlmResult<SubcrateSummary>> + 'a>> {
        Box::pin(async move {
            // First, generate summaries for all nested subcrates (bottom-up)
            let nested_summaries = if !node.nested_subcrates.is_empty() {
                let mut nested = HashMap::new();
                for (nested_name, nested_node) in &node.nested_subcrates {
                    let full_path = format!("{}/{}", subcrate_name, nested_name);

                    // Skip if this nested subcrate was truncated
                    if !skip_paths.contains(&full_path) {
                        let nested_summary = self.generate_subcrate_summary_recursive(
                            crate_info,
                            &full_path,
                            nested_node,
                            token_budget,
                            skip_paths
                        ).await?;
                        nested.insert(nested_name.clone(), nested_summary);
                    }
                }
                if nested.is_empty() { None } else { Some(nested) }
            } else {
                None
            };

        // Build prompt for this subcrate
        let mut prompt = String::new();
        prompt.push_str(&format!("SUBCRATE SUMMARY\n\n"));
        prompt.push_str(&format!("Crate: {}\n", crate_info.name));
        prompt.push_str(&format!("Subcrate: {}\n", subcrate_name));
        prompt.push_str(&format!("Direct Files: {}\n", node.direct_files.len()));

        if let Some(ref nested) = nested_summaries {
            prompt.push_str(&format!("Nested Subcrates: {}\n", nested.len()));
        }

        prompt.push_str("\nFILE SUMMARIES:\n");
        for file in &node.direct_files {
            prompt.push_str(&format!("\n=== {} ===\n",
                Path::new(&file.file_path).file_name().unwrap().to_string_lossy()));
            prompt.push_str(&format!("{}\n", file.file_summary));
        }

        // Add nested subcrate summaries if present
        if let Some(ref nested) = nested_summaries {
            prompt.push_str("\nNESTED SUBCRATES:\n");
            for (nested_name, nested_summary) in nested {
                prompt.push_str(&format!("\n=== {} ({} files) ===\n", nested_name, nested_summary.file_count));
                prompt.push_str(&format!("{}\n", nested_summary.summary));
            }
        }

        prompt.push_str("\nProvide a concise technical summary:\n");
        prompt.push_str("Primary Purpose: State the core functionality in 1-2 sentences.\n");
        prompt.push_str("Key Components: Synthesize the main capabilities (don't enumerate every file - group by theme/function).\n");
        if nested_summaries.is_some() {
            prompt.push_str("Nested Subcrates: Brief overview of what nested components provide.\n");
        }
        prompt.push_str("Integration: How components work together.\n");
        prompt.push_str("Be concise and synthesis-focused.\n");

        // Debug: Log prompt size BEFORE sending to LLM
        let prompt_chars = prompt.len();
        let estimated_tokens = prompt_chars / 4;
        println!("🔍 DEBUG: Subcrate '{}' prompt size: {} chars, ~{} tokens (budget: {})",
            subcrate_name, prompt_chars, estimated_tokens, token_budget);

        if estimated_tokens > 14000 {
            println!("⚠️  WARNING: Prompt exceeds 14K tokens! This will cause CPU offloading.");
            println!("   Prompt breakdown:");
            println!("   - Direct files: {}", node.direct_files.len());
            if let Some(ref nested) = nested_summaries {
                println!("   - Nested subcrates: {}", nested.len());
            }
        }

        // Generate LLM summary with token budget
        let summary_text = self.processor.generate_hierarchical_summary_with_budget(&prompt, token_budget).await?;
        let token_count = summary_text.split_whitespace().count();

        // Collect file paths for this subcrate
        let file_paths: Vec<String> = node.direct_files.iter()
            .map(|f| self.make_relative_path(&f.file_path))
            .collect();

            Ok(SubcrateSummary {
                name: subcrate_name.to_string(),
                file_count: node.direct_files.len(),
                files: file_paths,
                summary: summary_text,
                token_count,
                total_size_kb: node.total_size_kb,
                subcrates: nested_summaries,
            })
        })
    }

    /// Collect all file paths recursively from a subcrate summary
    fn collect_all_files_recursive(&self, subcrate: &SubcrateSummary) -> Vec<String> {
        let mut all_files = subcrate.files.clone();

        if let Some(ref nested) = subcrate.subcrates {
            for nested_subcrate in nested.values() {
                all_files.extend(self.collect_all_files_recursive(nested_subcrate));
            }
        }

        all_files
    }

    /// Build context-aware prompt with subcrate summaries
    fn build_context_aware_prompt_with_subcrates(
        &self,
        crate_info: &CrateInfo,
        individual_files: &[CleanedFileData],
        subcrate_summaries: &Option<HashMap<String, SubcrateSummary>>,
        project_memory: &crate::conversation::ProjectAnalysisMemory,
    ) -> String {
        let mut prompt = String::new();

        // Add context from project memory
        if !project_memory.processed_crates.is_empty() {
            prompt.push_str("PREVIOUSLY ANALYZED CRATES:\n");
            for crate_name in &project_memory.processed_crates {
                prompt.push_str(&format!("- {}\n", crate_name));
            }
            if !project_memory.architectural_insights.is_empty() {
                prompt.push_str("\nARCHITECTURAL INSIGHTS:\n");
                for insight in &project_memory.architectural_insights {
                    prompt.push_str(&format!("- {}\n", insight));
                }
            }
            prompt.push_str("\n");
        }

        prompt.push_str(&format!("CURRENT CRATE: {}\n", crate_info.name));
        if let Some(ref desc) = crate_info.description {
            prompt.push_str(&format!("Description: {}\n", desc));
        }
        prompt.push_str("\n");

        // Add subcrate summaries if present
        if let Some(ref subcrates) = subcrate_summaries {
            prompt.push_str("SUBCRATES:\n");
            for (name, summary) in subcrates {
                prompt.push_str(&format!("\n=== {} ({} files) ===\n", name, summary.file_count));
                prompt.push_str(&format!("{}\n", summary.summary));
            }
            prompt.push_str("\n");
        }

        // Add individual file summaries
        if !individual_files.is_empty() {
            prompt.push_str("INDIVIDUAL FILES:\n");
            for file in individual_files {
                prompt.push_str(&format!("\n=== {} ===\n", self.make_relative_path(&file.file_path)));
                prompt.push_str(&format!("{}\n", file.file_summary));
            }
            prompt.push_str("\n");
        }

        prompt.push_str(&format!("Provide a technical summary of the '{}' crate with exactly TWO sections:\n", crate_info.name));
        prompt.push_str("## Overall Architecture\n[List SPECIFIC external libraries/tools BY NAME that are mentioned in the summaries above. Then describe how this crate fits in the project]\n\n");
        prompt.push_str("## What Does It Actually Do\n[CRITICAL - Answer this FIRST: What PRIMARY OUTPUT does this crate produce for end users? If a user runs this crate, what do they GET? State the #1 USER-FACING CAPABILITY first (e.g., 'Generates X', 'Produces Y', 'Creates Z'), then list secondary features. Focus on OUTPUTS and DELIVERABLES, not internal operations.]\n\n");
        prompt.push_str("CRITICAL INSTRUCTIONS:\n");
        prompt.push_str("- Start with what users GET/RECEIVE from this crate (the output/deliverable)\n");
        prompt.push_str("- If this crate generates summaries/reports/analysis, STATE THAT FIRST\n");
        prompt.push_str("- If this crate produces visualizations/graphs/files, STATE THAT FIRST\n");
        prompt.push_str("- Only mention internal operations AFTER stating the primary output\n");
        prompt.push_str("Limit response to 250 words total.\n");

        prompt
    }

    /// Build reduced context prompt with subcrates (for when token budget is exceeded)
    fn build_reduced_context_prompt_with_subcrates(
        &self,
        crate_info: &CrateInfo,
        individual_files: &[CleanedFileData],
        subcrate_summaries: &Option<HashMap<String, SubcrateSummary>>,
        _project_memory: &crate::conversation::ProjectAnalysisMemory,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!("CRATE: {}\n", crate_info.name));
        if let Some(ref desc) = crate_info.description {
            prompt.push_str(&format!("Description: {}\n", desc));
        }
        prompt.push_str("\n");

        // Add subcrate summaries (these are already compressed)
        if let Some(ref subcrates) = subcrate_summaries {
            prompt.push_str("SUBCRATES:\n");
            for (name, summary) in subcrates {
                prompt.push_str(&format!("- {} ({} files): {}\n", name, summary.file_count,
                    summary.summary.lines().next().unwrap_or(&summary.summary)));
            }
            prompt.push_str("\n");
        }

        // Only include first sentence of each individual file summary
        if !individual_files.is_empty() {
            prompt.push_str("KEY FILES:\n");
            for file in individual_files.iter().take(10) {
                let first_sentence = file.file_summary.split('.').next().unwrap_or(&file.file_summary);
                prompt.push_str(&format!("- {}: {}\n",
                    Path::new(&file.file_path).file_name().unwrap().to_string_lossy(),
                    first_sentence));
            }
            prompt.push_str("\n");
        }

        prompt.push_str(&format!("Provide a technical summary of the '{}' crate with exactly TWO sections:\n", crate_info.name));
        prompt.push_str("## Overall Architecture\n[List SPECIFIC external libraries/tools BY NAME that are mentioned in the summaries above. Then describe how this crate fits in the project]\n\n");
        prompt.push_str("## What Does It Actually Do\n[CRITICAL - Answer this FIRST: What PRIMARY OUTPUT does this crate produce for end users? If a user runs this crate, what do they GET? State the #1 USER-FACING CAPABILITY first (e.g., 'Generates X', 'Produces Y', 'Creates Z'), then list secondary features. Focus on OUTPUTS and DELIVERABLES, not internal operations.]\n\n");
        prompt.push_str("CRITICAL INSTRUCTIONS:\n");
        prompt.push_str("- Start with what users GET/RECEIVE from this crate (the output/deliverable)\n");
        prompt.push_str("- If this crate generates summaries/reports/analysis, STATE THAT FIRST\n");
        prompt.push_str("- If this crate produces visualizations/graphs/files, STATE THAT FIRST\n");
        prompt.push_str("- Only mention internal operations AFTER stating the primary output\n");
        prompt.push_str("Limit response to 250 words total.\n");

        prompt
    }
}

/// Internal node structure for subcrate tree
#[derive(Debug, Clone)]
struct SubcrateNode {
    name: String,
    direct_files: Vec<CleanedFileData>,
    all_files: Vec<CleanedFileData>,
    nested_subcrates: HashMap<String, SubcrateNode>,
    total_size_kb: f64,
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