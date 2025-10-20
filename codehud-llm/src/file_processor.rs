//! File Processing Pipeline for Comment Analysis
//!
//! This module implements the three-phase workflow described in LLM_vision.txt:
//! 1. Extract comments from files -> comments.json
//! 2. Generate per-file summaries using LLM -> summaries.json
//! 3. Create system-wide summary -> system_summary.json

use crate::{
    LlmResult, LlmError,
    comment_extractor::{CommentExtractor, FileCommentExtraction, ExtractionConfig},
    ollama::{OllamaPipeline, OllamaConfig},
    structured::{StructuredCodeGenerator, GenerationConstraints, OutputFormat},
    constitutional::ConstitutionalAI,
    conversation::ConversationTracker,
    monitoring::LlmMonitor,
};
use codehud_core::query_engine::QueryEngine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;

/// Summary of a single file based on its comments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    /// File path that was analyzed
    pub file: String,
    /// Key themes identified from comments
    pub themes: Vec<String>,
    /// Dependencies mentioned in comments
    pub dependencies: Vec<String>,
    /// Overall purpose of the file
    pub purpose: String,
    /// Confidence level of the analysis
    pub confidence: ConfidenceLevel,
    /// LLM model used for analysis
    pub llm_model: String,
    /// When the analysis was performed
    pub analysis_timestamp: String,
    /// Token usage statistics
    pub token_usage: TokenUsage,
}

/// System-wide summary of the entire codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSummary {
    /// Overall themes across the entire system
    pub system_themes: Vec<String>,
    /// Key dependencies used throughout the system
    pub key_dependencies: Vec<String>,
    /// Overall purpose and description of the system
    pub system_purpose: String,
    /// Confidence level of the system analysis
    pub confidence: ConfidenceLevel,
    /// Metadata about the analysis process
    pub analysis_metadata: AnalysisMetadata,
}

/// Confidence level of LLM analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens sent to LLM
    pub input: usize,
    /// Output tokens received from LLM
    pub output: usize,
}

/// Analysis metadata for system summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    /// Number of files analyzed
    pub files_analyzed: usize,
    /// Total comments extracted
    pub total_comments: usize,
    /// Programming languages detected
    pub languages_detected: Vec<String>,
    /// Total analysis duration in seconds
    pub analysis_duration_seconds: u64,
    /// LLM model used
    pub model_used: String,
}

/// Configuration for file processing pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Comment extraction configuration
    pub extraction_config: ExtractionConfig,
    /// LLM configuration
    pub llm_config: LlmAnalysisConfig,
    /// Output configuration
    pub output_config: OutputConfig,
    /// Performance configuration
    pub performance_config: PerformanceConfig,
}

/// LLM analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAnalysisConfig {
    /// Maximum tokens per file analysis
    pub max_tokens_per_file: usize,
    /// Maximum tokens for system summary
    pub system_summary_max_tokens: usize,
    /// Whether to include code context in analysis
    pub include_code_context: bool,
    /// Whether to extract TODO items
    pub extract_todos: bool,
    /// Whether to analyze documentation coverage
    pub analyze_documentation_coverage: bool,
    /// Temperature for LLM responses
    pub temperature: f32,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output directory for analysis files
    pub output_dir: PathBuf,
    /// Whether to create pretty-formatted JSON
    pub pretty_json: bool,
    /// Whether to include debug information
    pub include_debug_info: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Whether to enable parallel file processing
    pub parallel_processing: bool,
    /// Maximum concurrent LLM calls
    pub max_concurrent_llm_calls: usize,
    /// Whether to use caching
    pub use_cache: bool,
    /// Cache duration in hours
    pub cache_duration_hours: u64,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            extraction_config: ExtractionConfig::default(),
            llm_config: LlmAnalysisConfig {
                max_tokens_per_file: 8000,
                system_summary_max_tokens: 8000,
                include_code_context: true,
                extract_todos: true,
                analyze_documentation_coverage: true,
                temperature: 0.1,
            },
            output_config: OutputConfig {
                output_dir: PathBuf::from("analysis_output"),
                pretty_json: true,
                include_debug_info: false,
            },
            performance_config: PerformanceConfig {
                parallel_processing: true,
                max_concurrent_llm_calls: 3,
                use_cache: true,
                cache_duration_hours: 24,
            },
        }
    }
}

/// Processing report summarizing the entire analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingReport {
    /// Overall success status
    pub success: bool,
    /// Files processed successfully
    pub files_processed: usize,
    /// Files that failed processing
    pub files_failed: usize,
    /// Total processing time
    pub total_duration: std::time::Duration,
    /// Output file paths
    pub output_files: OutputFiles,
    /// Error summary
    pub errors: Vec<ProcessingError>,
    /// Performance metrics
    pub performance: ProcessingPerformance,
}

/// Output file paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFiles {
    /// Path to comments.json
    pub comments_file: PathBuf,
    /// Path to summaries.json
    pub summaries_file: PathBuf,
    /// Path to system_summary.json
    pub system_summary_file: PathBuf,
}

/// Processing error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    /// File that caused the error
    pub file: String,
    /// Error message
    pub error: String,
    /// Processing phase where error occurred
    pub phase: ProcessingPhase,
}

/// Processing phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingPhase {
    CommentExtraction,
    LlmAnalysis,
    SystemSummary,
    OutputGeneration,
}

/// Processing performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingPerformance {
    /// Comment extraction time
    pub extraction_time: std::time::Duration,
    /// LLM analysis time
    pub analysis_time: std::time::Duration,
    /// System summary time
    pub summary_time: std::time::Duration,
    /// Average time per file
    pub avg_time_per_file: std::time::Duration,
    /// Total tokens processed
    pub total_tokens: usize,
    /// LLM calls made
    pub llm_calls: usize,
}

/// Main file processor implementing the three-phase workflow
pub struct FileProcessor {
    /// Comment extractor
    extractor: Arc<CommentExtractor>,
    /// LLM pipeline for analysis
    llm_pipeline: Arc<OllamaPipeline>,
    /// Structured code generator
    structured_generator: Arc<RwLock<StructuredCodeGenerator>>,
    /// Constitutional AI for quality control
    constitutional_ai: Arc<ConstitutionalAI>,
    /// Conversation tracker for context management
    conversation_tracker: Arc<ConversationTracker>,
    /// LLM performance monitor
    llm_monitor: Arc<LlmMonitor>,
    /// Configuration
    config: ProcessorConfig,
}

impl FileProcessor {
    /// Create a new file processor with all required components
    pub async fn new(
        ollama_config: OllamaConfig,
        config: ProcessorConfig,
    ) -> LlmResult<Self> {
        // Create comment extractor
        let extractor = Arc::new(CommentExtractor::with_config(
            config.extraction_config.clone()
        ));

        // Create LLM pipeline
        let llm_pipeline = Arc::new(OllamaPipeline::new(ollama_config)?);

        // Create structured generator
        let structured_generator = Arc::new(RwLock::new(StructuredCodeGenerator::new(
            crate::structured::GeneratorConfig::default(),
        )?));

        // Create constitutional AI with default config
        let constitutional_ai = Arc::new(ConstitutionalAI::new(
            crate::constitutional::ConstitutionalConfig {
                principles: vec![],
                strict_mode: false,
                auto_correction: true,
                violation_threshold: 0.7,
            }
        ));

        // Create conversation tracker with default config
        let conversation_tracker = Arc::new(ConversationTracker::new(
            crate::conversation::ConversationConfig {
                max_context_length: 8000,
                summary_threshold: 50,
                auto_summarize: true,
                track_quality: true,
                preserve_system_messages: true,
                compression_strategy: crate::conversation::CompressionStrategy::SummarizeOldest,
            }
        ));

        // Create LLM monitor with default config
        let llm_monitor = Arc::new(LlmMonitor::new(
            crate::monitoring::MonitoringConfig {
                metrics_interval_seconds: 60,
                health_check_interval_seconds: 30,
                alert_thresholds: crate::monitoring::AlertThresholds::default(),
                retention_days: 1,
                enable_detailed_metrics: true,
                enable_performance_profiling: false,
            }
        ));

        Ok(Self {
            extractor,
            llm_pipeline,
            structured_generator,
            constitutional_ai,
            conversation_tracker,
            llm_monitor,
            config,
        })
    }

    /// Process a single file and return its summary
    pub async fn process_single_file(&self, file_path: &Path) -> LlmResult<FileSummary> {
        // Phase 1: Extract comments
        let extraction = self.extractor.extract_from_file(file_path)?;

        // Phase 2: Generate LLM summary for this file
        let summary = self.generate_file_summary(&extraction).await?;

        Ok(summary)
    }

    /// Generate a text summary using the LLM (for FSM integration)
    // REMOVED: Old generate_text_summary with low token limit
    // Now using generate_hierarchical_summary for all summaries

    /// Generate hierarchical project summary with extended output tokens (for FILE summaries)
    pub async fn generate_hierarchical_summary(&self, prompt: &str) -> LlmResult<String> {

        // Make direct HTTP call to Ollama API with extended token limit
        // TEMPORARY: Using 14B model for better semantic understanding
        let system_prompt = "You are an expert software architect. Analyze the complete system architecture, component interactions, and unified capabilities. Provide comprehensive, detailed analysis WITHOUT cutting yourself short. Complete your full analysis.";

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
                "num_predict": 512,  // 512 tokens for file summaries (small prompts)
                "num_ctx": 8192  // 8K context is enough for FILE summaries (prompts are 2-4K tokens)
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
                "Ollama generation failed: {}",
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

    /// Generate project-level hierarchical summary (SEPARATE from file summaries)
    pub async fn generate_project_hierarchical_summary(&self, prompt: &str) -> LlmResult<String> {

        // Make direct HTTP call to Ollama API for PROJECT-LEVEL summary
        // Use larger model (14B) for better synthesis and abstraction capabilities
        let system_prompt = "You are an expert software architect. Analyze the complete system architecture, component interactions, and unified capabilities. Provide comprehensive, detailed analysis WITHOUT cutting yourself short. Complete your full analysis.";

        let client = reqwest::Client::new();
        let request = serde_json::json!({
            "model": "qwen2.5:14b-instruct-q5_K_M",
            "prompt": prompt,
            "system": system_prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "top_p": 0.9,
                "top_k": 40,
                "num_predict": 1500,  // 1500 tokens for 600-1000 word comprehensive summaries
                "num_ctx": 16384  // 16K context window for 14B model
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
                "Ollama generation failed: {}",
                response.status()
            )));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| LlmError::Http(e))?;

        // DEBUG: Log stop reason and token counts for final project summary
        eprintln!("🔍 DEBUG: Final project summary response metadata:");
        eprintln!("  - Budget (num_predict): 1500");
        if let Some(done_reason) = response_json.get("done_reason") {
            eprintln!("  - Stop reason: {:?}", done_reason);
        }
        if let Some(eval_count) = response_json.get("eval_count") {
            eprintln!("  - Tokens generated: {:?}", eval_count);
        }
        if let Some(prompt_eval_count) = response_json.get("prompt_eval_count") {
            eprintln!("  - Prompt tokens: {:?}", prompt_eval_count);
        }

        let generated_text = response_json["response"]
            .as_str()
            .ok_or_else(|| LlmError::Inference("No response field in Ollama response".to_string()))?
            .to_string();


        Ok(generated_text)
    }

    /// Generate hierarchical summary with custom token budget (for file summaries)
    pub async fn generate_hierarchical_summary_with_budget(&self, prompt: &str, max_tokens: usize) -> LlmResult<String> {

        // Make direct HTTP call to Ollama API with custom token limit
        // TEMPORARY: Using 14B model for better semantic understanding
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
                "num_predict": max_tokens,
                "num_ctx": 16384  // 16K context for subcrate summaries (larger prompts ~10K)
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
                "Ollama generation failed: {}",
                response.status()
            )));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| LlmError::Http(e))?;

        // DEBUG: Log stop reason and token counts to diagnose truncation
        eprintln!("🔍 DEBUG: Ollama response metadata:");
        eprintln!("  - Budget (num_predict): {}", max_tokens);
        if let Some(done_reason) = response_json.get("done_reason") {
            eprintln!("  - Stop reason: {:?}", done_reason);
        }
        if let Some(eval_count) = response_json.get("eval_count") {
            eprintln!("  - Tokens generated: {:?}", eval_count);
        }
        if let Some(prompt_eval_count) = response_json.get("prompt_eval_count") {
            eprintln!("  - Prompt tokens: {:?}", prompt_eval_count);
        }

        let generated_text = response_json["response"]
            .as_str()
            .ok_or_else(|| LlmError::Inference("No response field in Ollama response".to_string()))?
            .to_string();


        Ok(generated_text)
    }

    /// Generate LLM summary for a single file's comments
    pub async fn generate_file_summary(&self, extraction: &crate::comment_extractor::FileCommentExtraction) -> LlmResult<FileSummary> {
        // Debug: Print comment extraction details

        // If no comments found, return early
        if extraction.comments.is_empty() {
            return Ok(FileSummary {
                file: extraction.file.clone(),
                themes: vec!["No comments".to_string()],
                dependencies: vec![],
                purpose: "No comments found for analysis".to_string(),
                confidence: ConfidenceLevel::Low,
                llm_model: "skipped".to_string(),
                analysis_timestamp: chrono::Utc::now().to_rfc3339(),
                token_usage: TokenUsage {
                    input: 0,
                    output: 0,
                },
            });
        }

        // Build prompt with comments and structural insights
        let mut prompt = format!(
            "Analyze the file: {}\nLanguage: {}\nComments found: {}\n\nComment details:\n{}\n",
            extraction.file,
            extraction.language,
            extraction.comments.len(),
            extraction.comments.iter()
                .map(|c| format!("- Line {}: {}", c.start_position.line, c.text))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Add structural insights if available
        if let Some(ref insights) = extraction.structural_insights {
            prompt.push_str("\nTechnical Details:\n");
            for (section, items) in &insights.sections {
                if !items.is_empty() {
                    prompt.push_str(&format!("  {}:\n", section));
                    for item in items.iter().take(10) {
                        prompt.push_str(&format!("    {}\n", item));
                    }
                }
            }
        }

        prompt.push_str("\nProvide a concise analysis including themes, dependencies mentioned, and the overall purpose of this file.");

        let start_time = std::time::Instant::now();

        // Generate LLM analysis using actual API call
        let analysis_response = self.generate_hierarchical_summary(&prompt).await?;

        let duration = start_time.elapsed();

        // Create file summary
        Ok(FileSummary {
            file: extraction.file.clone(),
            themes: vec!["General analysis".to_string()], // TODO: Parse from LLM response
            dependencies: vec![], // TODO: Extract from LLM response
            purpose: analysis_response.clone(),
            confidence: ConfidenceLevel::Medium,
            llm_model: "ollama".to_string(),
            analysis_timestamp: chrono::Utc::now().to_rfc3339(),
            token_usage: TokenUsage {
                input: prompt.len() / 4, // Rough estimate
                output: analysis_response.len() / 4,
            },
        })
    }

    /// Process an entire codebase using the three-phase workflow
    pub async fn process_codebase(&self, codebase_path: &Path) -> LlmResult<ProcessingReport> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut performance = ProcessingPerformance {
            extraction_time: std::time::Duration::ZERO,
            analysis_time: std::time::Duration::ZERO,
            summary_time: std::time::Duration::ZERO,
            avg_time_per_file: std::time::Duration::ZERO,
            total_tokens: 0,
            llm_calls: 0,
        };

        // Create output directory
        fs::create_dir_all(&self.config.output_config.output_dir).await
            .map_err(|e| LlmError::Io(e))?;

        // Phase 1: Extract comments from all files
        log::info!("Phase 1: Extracting comments from codebase");
        let extraction_start = std::time::Instant::now();
        let comment_extractions = match self.extract_comments_phase(codebase_path).await {
            Ok(extractions) => extractions,
            Err(e) => {
                errors.push(ProcessingError {
                    file: codebase_path.to_string_lossy().to_string(),
                    error: e.to_string(),
                    phase: ProcessingPhase::CommentExtraction,
                });
                return Ok(self.create_failed_report(errors, start_time.elapsed()));
            }
        };
        performance.extraction_time = extraction_start.elapsed();

        // Save comments.json
        let comments_file = self.config.output_config.output_dir.join("comments.json");
        self.save_json(&comment_extractions, &comments_file).await?;

        // Phase 2: Generate per-file summaries using LLM
        log::info!("Phase 2: Generating per-file summaries");
        let analysis_start = std::time::Instant::now();
        let (file_summaries, analysis_errors, analysis_stats) = self
            .analyze_files_phase(&comment_extractions).await;
        errors.extend(analysis_errors);
        performance.analysis_time = analysis_start.elapsed();
        performance.total_tokens += analysis_stats.total_tokens;
        performance.llm_calls += analysis_stats.llm_calls;

        // Save summaries.json
        let summaries_file = self.config.output_config.output_dir.join("summaries.json");
        self.save_json(&file_summaries, &summaries_file).await?;

        // Phase 3: Generate system-wide summary
        log::info!("Phase 3: Generating system-wide summary");
        let summary_start = std::time::Instant::now();
        let (system_summary, summary_stats) = match self.generate_system_summary(&file_summaries).await {
            Ok(summary) => summary,
            Err(e) => {
                errors.push(ProcessingError {
                    file: "system_summary".to_string(),
                    error: e.to_string(),
                    phase: ProcessingPhase::SystemSummary,
                });
                return Ok(self.create_failed_report(errors, start_time.elapsed()));
            }
        };
        performance.summary_time = summary_start.elapsed();
        performance.total_tokens += summary_stats.total_tokens;
        performance.llm_calls += summary_stats.llm_calls;

        // Save system_summary.json
        let system_summary_file = self.config.output_config.output_dir.join("system_summary.json");
        self.save_json(&system_summary, &system_summary_file).await?;

        // Calculate final metrics
        let total_duration = start_time.elapsed();
        let files_processed = comment_extractions.len();
        let files_failed = errors.len();

        if files_processed > 0 {
            performance.avg_time_per_file = total_duration / files_processed as u32;
        }

        // Create processing report
        Ok(ProcessingReport {
            success: files_failed == 0,
            files_processed,
            files_failed,
            total_duration,
            output_files: OutputFiles {
                comments_file,
                summaries_file,
                system_summary_file,
            },
            errors,
            performance,
        })
    }

    /// Phase 1: Extract comments from all files in the codebase
    async fn extract_comments_phase(&self, codebase_path: &Path) -> LlmResult<HashMap<String, FileCommentExtraction>> {
        let extractions = if self.config.performance_config.parallel_processing {
            self.extract_comments_parallel(codebase_path).await?
        } else {
            self.extract_comments_sequential(codebase_path).await?
        };

        // Convert to HashMap for JSON output matching vision format
        let mut comments_map = HashMap::new();
        for extraction in extractions {
            comments_map.insert(extraction.file.clone(), extraction);
        }

        Ok(comments_map)
    }

    /// Extract comments sequentially (simpler, more stable)
    async fn extract_comments_sequential(&self, codebase_path: &Path) -> LlmResult<Vec<FileCommentExtraction>> {
        self.extractor.extract_from_directory(codebase_path)
            .map_err(|e| LlmError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    /// Extract comments in parallel (faster for large codebases)
    async fn extract_comments_parallel(&self, codebase_path: &Path) -> LlmResult<Vec<FileCommentExtraction>> {
        // For now, use sequential implementation
        // TODO: Implement true parallel processing with tokio tasks
        self.extract_comments_sequential(codebase_path).await
    }

    /// Phase 2: Analyze each file's comments using LLM
    async fn analyze_files_phase(
        &self,
        comment_extractions: &HashMap<String, FileCommentExtraction>,
    ) -> (HashMap<String, FileSummary>, Vec<ProcessingError>, AnalysisStats) {
        let mut summaries = HashMap::new();
        let mut errors = Vec::new();
        let mut stats = AnalysisStats { total_tokens: 0, llm_calls: 0 };

        for (file_path, extraction) in comment_extractions {
            match self.analyze_single_file(extraction).await {
                Ok((summary, file_stats)) => {
                    summaries.insert(file_path.clone(), summary);
                    stats.total_tokens += file_stats.total_tokens;
                    stats.llm_calls += file_stats.llm_calls;
                }
                Err(e) => {
                    errors.push(ProcessingError {
                        file: file_path.clone(),
                        error: e.to_string(),
                        phase: ProcessingPhase::LlmAnalysis,
                    });
                }
            }
        }

        (summaries, errors, stats)
    }

    /// Analyze a single file's comments using structured LLM generation
    async fn analyze_single_file(&self, extraction: &FileCommentExtraction) -> LlmResult<(FileSummary, AnalysisStats)> {
        if extraction.comments.is_empty() {
            return Ok((self.create_empty_summary(extraction), AnalysisStats { total_tokens: 0, llm_calls: 0 }));
        }

        // Create structured prompt for file analysis
        let prompt = self.build_file_analysis_prompt(extraction);

        // Define constraints for structured generation
        let constraints = GenerationConstraints {
            output_format: OutputFormat::JsonObject,
            json_schema: Some(self.get_file_summary_schema()),
            max_length: Some(self.config.llm_config.max_tokens_per_file),
            validation_rules: vec![
                "coherent_analysis".to_string(),
                "evidence_based".to_string(),
            ],
            grammar_rules: None,
        };

        // Generate structured analysis
        let response = self.structured_generator.write().await.generate_structured_code(&prompt, &constraints).await?;

        // Parse response into FileSummary
        let summary_value: Value = serde_json::from_str(&response.code)?;
        let mut summary: FileSummary = serde_json::from_value(summary_value)?;

        // Add metadata
        summary.file = extraction.file.clone();
        summary.llm_model = self.get_model_name();
        summary.analysis_timestamp = chrono::Utc::now().to_rfc3339();
        summary.token_usage = TokenUsage {
            input: prompt.len() / 4, // Rough token estimation
            output: response.code.len() / 4,
        };

        let stats = AnalysisStats {
            total_tokens: summary.token_usage.input + summary.token_usage.output,
            llm_calls: 1,
        };

        Ok((summary, stats))
    }

    /// Phase 3: Generate system-wide summary from file summaries
    async fn generate_system_summary(&self, file_summaries: &HashMap<String, FileSummary>) -> LlmResult<(SystemSummary, AnalysisStats)> {
        let prompt = self.build_system_summary_prompt(file_summaries);

        let constraints = GenerationConstraints {
            output_format: OutputFormat::JsonObject,
            json_schema: Some(self.get_system_summary_schema()),
            max_length: Some(self.config.llm_config.system_summary_max_tokens),
            validation_rules: vec![
                "comprehensive_analysis".to_string(),
                "system_level_insights".to_string(),
            ],
            grammar_rules: None,
        };

        let response = self.structured_generator.write().await.generate_structured_code(&prompt, &constraints).await?;

        let summary_value: Value = serde_json::from_str(&response.code)?;
        let mut summary: SystemSummary = serde_json::from_value(summary_value)?;

        // Add analysis metadata
        let languages: Vec<String> = file_summaries.values()
            .map(|s| self.extract_language_from_file(&s.file))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        summary.analysis_metadata = AnalysisMetadata {
            files_analyzed: file_summaries.len(),
            total_comments: 0, // TODO: Calculate from extractions
            languages_detected: languages,
            analysis_duration_seconds: 0, // TODO: Calculate actual duration
            model_used: self.get_model_name(),
        };

        let stats = AnalysisStats {
            total_tokens: prompt.len() / 4 + response.code.len() / 4,
            llm_calls: 1,
        };

        Ok((summary, stats))
    }

    /// Build prompt for analyzing a single file's comments
    fn build_file_analysis_prompt(&self, extraction: &FileCommentExtraction) -> String {
        let comments_json = serde_json::to_string_pretty(&extraction.comments)
            .unwrap_or_else(|_| "[]".to_string());

        format!(
            r#"You are a code analyst. Below is a JSON object containing comments extracted from a code file, with their metadata (e.g., position, type). Your task is to deduce the overall function and purpose of the file based solely on these comments.

Input JSON: {}

Steps:
1. Identify key themes or topics in the comments (e.g., "authentication", "data processing").
2. Note any mentioned dependencies, functions, or modules.
3. Summarize the file's purpose in 1-2 sentences.
4. If comments are insufficient, output "Insufficient comments for summary."

Output in JSON:
{{
  "themes": ["theme1", "theme2"],
  "dependencies": ["dep1", "dep2"],
  "purpose": "Brief summary here.",
  "confidence": "High/Medium/Low"
}}

Focus on extracting meaningful insights about the code's purpose and functionality."#,
            comments_json
        )
    }

    /// Build prompt for system-wide summary
    fn build_system_summary_prompt(&self, file_summaries: &HashMap<String, FileSummary>) -> String {
        let summaries_json = serde_json::to_string_pretty(file_summaries)
            .unwrap_or_else(|_| "{}".to_string());

        format!(
            r#"You are a code analyst. Below is a JSON object containing summaries of all files in a codebase, each with their deduced purpose, themes, and dependencies based on comments.

Input JSON: {}

Steps:
1. Identify overarching themes across all files (e.g., "web server", "data pipeline").
2. Note key dependencies or interactions between files (e.g., based on mentioned modules).
3. Summarize the entire codebase's purpose and structure in 2-4 sentences.
4. If summaries are insufficient, output "Insufficient data for system summary."

Output in JSON:
{{
  "system_themes": ["theme1", "theme2"],
  "key_dependencies": ["dep1", "dep2"],
  "system_purpose": "Brief system summary here.",
  "confidence": "High/Medium/Low"
}}

Focus on understanding the overall architecture and purpose of the system."#,
            summaries_json
        )
    }

    /// Get JSON schema for FileSummary
    fn get_file_summary_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "themes": {"type": "array", "items": {"type": "string"}},
                "dependencies": {"type": "array", "items": {"type": "string"}},
                "purpose": {"type": "string"},
                "confidence": {"type": "string", "enum": ["High", "Medium", "Low"]}
            },
            "required": ["themes", "dependencies", "purpose", "confidence"]
        })
    }

    /// Get JSON schema for SystemSummary
    fn get_system_summary_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "system_themes": {"type": "array", "items": {"type": "string"}},
                "key_dependencies": {"type": "array", "items": {"type": "string"}},
                "system_purpose": {"type": "string"},
                "confidence": {"type": "string", "enum": ["High", "Medium", "Low"]}
            },
            "required": ["system_themes", "key_dependencies", "system_purpose", "confidence"]
        })
    }

    /// Create empty summary for files with no comments
    fn create_empty_summary(&self, extraction: &FileCommentExtraction) -> FileSummary {
        FileSummary {
            file: extraction.file.clone(),
            themes: vec![],
            dependencies: vec![],
            purpose: "Insufficient comments for summary.".to_string(),
            confidence: ConfidenceLevel::Low,
            llm_model: self.get_model_name(),
            analysis_timestamp: chrono::Utc::now().to_rfc3339(),
            token_usage: TokenUsage { input: 0, output: 0 },
        }
    }

    /// Save data as JSON file
    async fn save_json<T: Serialize>(&self, data: &T, path: &Path) -> LlmResult<()> {
        let json_string = if self.config.output_config.pretty_json {
            serde_json::to_string_pretty(data)?
        } else {
            serde_json::to_string(data)?
        };

        fs::write(path, json_string).await
            .map_err(|e| LlmError::Io(e))
    }

    /// Create failed processing report
    fn create_failed_report(&self, errors: Vec<ProcessingError>, duration: std::time::Duration) -> ProcessingReport {
        ProcessingReport {
            success: false,
            files_processed: 0,
            files_failed: errors.len(),
            total_duration: duration,
            output_files: OutputFiles {
                comments_file: PathBuf::new(),
                summaries_file: PathBuf::new(),
                system_summary_file: PathBuf::new(),
            },
            errors,
            performance: ProcessingPerformance {
                extraction_time: std::time::Duration::ZERO,
                analysis_time: std::time::Duration::ZERO,
                summary_time: std::time::Duration::ZERO,
                avg_time_per_file: std::time::Duration::ZERO,
                total_tokens: 0,
                llm_calls: 0,
            },
        }
    }

    /// Get current LLM model name
    fn get_model_name(&self) -> String {
        // TODO: Get actual model name from LLM pipeline
        "deepseek-coder:7b".to_string()
    }

    /// Extract language from file path
    fn extract_language_from_file(&self, file_path: &str) -> String {
        Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "rs" => "Rust",
                "py" => "Python",
                "js" => "JavaScript",
                "ts" => "TypeScript",
                "java" => "Java",
                _ => ext,
            })
            .unwrap_or("Unknown")
            .to_string()
    }
}

/// Analysis statistics for tracking token usage
struct AnalysisStats {
    total_tokens: usize,
    llm_calls: usize,
}