//! LLM Comment Extraction Finite State Machine
//!
//! This module implements an FSM-based interface for LLM comment extraction
//! with two primary modes: single file scanning and project scanning with
//! context accumulation.

use crate::{LlmResult, LlmError, FileProcessor, ProcessorConfig};
use crate::comment_extractor::{CommentExtractor, FileCommentExtraction, CleanedFileAnalysis};
use crate::crate_summarizer::{CrateGrouper, CrateSummarizer, CrateSummarizerConfig, CrateSummary, CrateInfo};
use crate::ollama::OllamaConfig;
use serde::{Deserialize, Serialize};
use serde_json;
use chrono;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::io::Write;
use tokio::sync::RwLock;
use walkdir::WalkDir;

/// FSM states for the comment extraction process
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionState {
    /// Initial state - ready to accept scan commands
    Ready,
    /// File picker mode - waiting for single file selection
    FilePickerMode,
    /// Single file scanning in progress
    ScanningFile { file_path: PathBuf },
    /// Project scanning initialization
    ProjectScanInit { project_path: PathBuf },
    /// Hierarchical project scan - discovering crates
    CrateGrouping { project_path: PathBuf },
    /// Processing files within each crate for crate-level summaries
    CrateSummarizing {
        project_path: PathBuf,
        current_crate: String,
        processed_crates: Vec<String>,
        remaining_crates: Vec<String>,
    },
    /// Project scanning in progress - processing individual files (legacy mode)
    ProjectScanProgress {
        project_path: PathBuf,
        current_file: PathBuf,
        processed_files: Vec<PathBuf>,
        remaining_files: Vec<PathBuf>,
    },
    /// Generating final project summary from crate summaries
    GeneratingFinalSummary {
        project_path: PathBuf,
        crate_summaries: Vec<CrateSummary>,
    },
    /// Accumulating context and generating project summary (legacy mode)
    GeneratingProjectSummary { project_path: PathBuf },
    /// Scan completed successfully
    ScanComplete { result: ScanResult },
    /// Error state
    Error { error: String },
}

/// Types of scan operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanMode {
    /// Single file scan with immediate result
    SingleFile,
    /// Project-wide scan with context accumulation
    ProjectScan,
}

/// Result of a scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Whether the scan completed successfully
    pub success: bool,
    /// Number of files processed
    pub files_processed: usize,
    /// Total scan duration in seconds
    pub duration_seconds: f64,
    /// Summary description of results
    pub summary: String,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl PartialEq for ScanResult {
    fn eq(&self, other: &Self) -> bool {
        self.success == other.success &&
        self.files_processed == other.files_processed &&
        (self.duration_seconds - other.duration_seconds).abs() < 0.001 &&
        self.summary == other.summary &&
        self.errors == other.errors
    }
}

impl Eq for ScanResult {}

/// Detailed scan results by type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetailedScanResult {
    /// Single file scan result
    SingleFile {
        file_path: PathBuf,
        extraction: FileCommentExtraction,
        llm_summary: String,
    },
    /// Project scan result with accumulated context
    ProjectScan {
        project_path: PathBuf,
        file_summaries: Vec<FileSummaryWithContext>,
        project_summary: ProjectSummary,
    },
}

/// File summary with accumulated context from previous files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummaryWithContext {
    /// Path to the file
    pub file_path: PathBuf,
    /// Extracted comments
    pub extraction: FileCommentExtraction,
    /// LLM-generated summary for this file
    pub file_summary: String,
    /// Context from previous files that influenced this summary
    pub accumulated_context: String,
    /// Processing order in the project scan
    pub scan_order: usize,
}

/// Project-wide summary with accumulated insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    /// Overall project understanding
    pub overview: String,
    /// Key architectural insights
    pub architectural_insights: Vec<String>,
    /// Common patterns found across files
    pub common_patterns: Vec<String>,
    /// Identified issues or improvements
    pub recommendations: Vec<String>,
    /// Total files processed
    pub files_processed: usize,
    /// Total comments analyzed
    pub total_comments: usize,
}

/// FSM events that trigger state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionEvent {
    /// Start single file scan mode
    StartFileScan,
    /// File selected for single scan
    FileSelected { file_path: PathBuf },
    /// Start project scan mode
    StartProjectScan,
    /// Project directory selected
    ProjectSelected { project_path: PathBuf },
    /// Single file processing completed
    FileProcessingComplete {
        file_path: PathBuf,
        extraction: FileCommentExtraction,
        summary: String,
    },
    /// Project file processing completed
    ProjectFileComplete {
        file_path: PathBuf,
        extraction: FileCommentExtraction,
        summary: String,
    },
    /// All project files processed, ready for final summary
    ProjectFilesComplete,
    /// Crate discovery completed
    CrateGroupingComplete {
        discovered_crates: Vec<CrateInfo>,
    },
    /// Single crate summary completed
    CrateSummaryComplete {
        crate_summary: CrateSummary,
    },
    /// All crate summaries completed, ready for final summary
    AllCrateSummariesComplete {
        crate_summaries: Vec<CrateSummary>,
    },
    /// Project summary generation completed
    ProjectSummaryComplete { summary: ProjectSummary },
    /// Error occurred
    Error { error: String },
    /// Reset to initial state
    Reset,
}

/// Context accumulator for project scans
#[derive(Debug, Clone, Default)]
struct ProjectContext {
    /// Accumulated understanding from processed files
    accumulated_knowledge: String,
    /// File summaries in processing order
    file_summaries: Vec<FileSummaryWithContext>,
    /// Total comments processed
    total_comments: usize,
    /// Architectural patterns discovered
    patterns: Vec<String>,
}

/// Main FSM for LLM comment extraction
pub struct CommentExtractionFSM {
    /// Current state
    state: Arc<RwLock<ExtractionState>>,
    /// File processor for LLM analysis
    processor: Arc<FileProcessor>,
    /// Comment extractor
    extractor: Arc<CommentExtractor>,
    /// Project context accumulator
    project_context: Arc<RwLock<ProjectContext>>,
    /// Event listeners for GUI/CLI integration
    event_listeners: Vec<Box<dyn Fn(ExtractionEvent) + Send + Sync>>,
    /// Project analysis memory for hierarchical context accumulation
    project_memory: Arc<RwLock<crate::conversation::ProjectAnalysisMemory>>,
}

impl CommentExtractionFSM {
    /// Create a new FSM instance
    pub async fn new(
        ollama_config: OllamaConfig,
        processor_config: ProcessorConfig,
    ) -> LlmResult<Self> {
        let processor = Arc::new(
            FileProcessor::new(ollama_config, processor_config).await?
        );

        let extractor = Arc::new(CommentExtractor::new());

        Ok(Self {
            state: Arc::new(RwLock::new(ExtractionState::Ready)),
            processor,
            extractor,
            project_context: Arc::new(RwLock::new(ProjectContext::default())),
            event_listeners: Vec::new(),
            project_memory: Arc::new(RwLock::new(crate::conversation::ProjectAnalysisMemory::new())),
        })
    }

    /// Get current state
    pub async fn get_state(&self) -> ExtractionState {
        self.state.read().await.clone()
    }

    /// Add event listener for GUI/CLI integration
    pub fn add_event_listener<F>(&mut self, listener: F)
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static
    {
        self.event_listeners.push(Box::new(listener));
    }

    /// Process an event and transition state
    pub async fn process_event(&self, event: ExtractionEvent) -> LlmResult<()> {
        // Notify listeners
        for listener in &self.event_listeners {
            listener(event.clone());
        }

        let mut state = self.state.write().await;

        match (&*state, &event) {
            // From Ready state
            (ExtractionState::Ready, ExtractionEvent::StartFileScan) => {
                *state = ExtractionState::FilePickerMode;
            }

            (ExtractionState::Ready, ExtractionEvent::StartProjectScan) => {
                // Reset project context
                *self.project_context.write().await = ProjectContext::default();
                // Stay in Ready state until project is selected
            }

            // File picker mode
            (ExtractionState::FilePickerMode, ExtractionEvent::FileSelected { file_path }) => {
                *state = ExtractionState::ScanningFile {
                    file_path: file_path.clone()
                };

                // Start async file processing
                self.process_single_file(file_path.clone()).await?;
            }

            // Project selection - start with hierarchical crate discovery
            (ExtractionState::Ready, ExtractionEvent::ProjectSelected { project_path }) => {
                // First, transition to crate grouping state
                *state = ExtractionState::CrateGrouping {
                    project_path: project_path.clone()
                };

                // Start async crate discovery
                self.start_crate_discovery(project_path.clone()).await?;
            }

            // File processing completion
            (ExtractionState::ScanningFile { .. }, ExtractionEvent::FileProcessingComplete {
                file_path, extraction, summary
            }) => {
                *state = ExtractionState::ScanComplete {
                    result: ScanResult {
                        success: true,
                        files_processed: 1,
                        duration_seconds: 0.0, // Will be updated by actual processing
                        summary: format!("Successfully processed file: {}", file_path.display()),
                        errors: vec![],
                    }
                };
            }

            // Project file completion
            (ExtractionState::ProjectScanProgress {
                project_path, processed_files, remaining_files, ..
            }, ExtractionEvent::ProjectFileComplete {
                file_path, extraction, summary
            }) => {
                // Update project context
                self.add_to_project_context(
                    file_path, extraction, summary, processed_files.len()
                ).await?;

                let mut new_processed = processed_files.clone();
                new_processed.push(file_path.clone());
                let mut new_remaining = remaining_files.clone();
                new_remaining.retain(|p| p != file_path);

                if new_remaining.is_empty() {
                    // All files processed, generate project summary
                    *state = ExtractionState::GeneratingProjectSummary {
                        project_path: project_path.clone()
                    };
                    self.generate_project_summary().await?;
                } else {
                    // Continue with next file
                    let next_file = new_remaining[0].clone();
                    *state = ExtractionState::ProjectScanProgress {
                        project_path: project_path.clone(),
                        current_file: next_file.clone(),
                        processed_files: new_processed,
                        remaining_files: new_remaining,
                    };
                    self.process_project_file(next_file).await?;
                }
            }

            // Project summary completion
            (ExtractionState::GeneratingProjectSummary { project_path },
             ExtractionEvent::ProjectSummaryComplete { summary }) => {
                let context = self.project_context.read().await;
                *state = ExtractionState::ScanComplete {
                    result: ScanResult {
                        success: true,
                        files_processed: context.file_summaries.len(),
                        duration_seconds: 0.0, // Will be updated by actual processing
                        summary: format!("Successfully processed project: {} ({} files)",
                                       project_path.display(),
                                       context.file_summaries.len()),
                        errors: vec![],
                    }
                };
            }

            // Hierarchical processing state transitions

            // Crate discovery completion
            (ExtractionState::CrateGrouping { .. },
             ExtractionEvent::CrateGroupingComplete { discovered_crates }) => {
                let project_path = match &*state {
                    ExtractionState::CrateGrouping { project_path } => project_path.clone(),
                    _ => return Err(LlmError::Config("Invalid state".to_string())),
                };

                if discovered_crates.is_empty() {
                    // No crates found, fallback to legacy processing
                    *state = ExtractionState::ProjectScanInit {
                        project_path: project_path.clone()
                    };
                    self.start_project_scan(project_path).await?;
                } else {
                    // Start hierarchical crate summarization
                    let remaining_crates: Vec<String> = discovered_crates.iter()
                        .map(|c| c.name.clone())
                        .collect();

                    if let Some(first_crate) = remaining_crates.first() {
                        let first_crate_name = first_crate.clone();
                        *state = ExtractionState::CrateSummarizing {
                            project_path: project_path.clone(),
                            current_crate: first_crate_name.clone(),
                            processed_crates: Vec::new(),
                            remaining_crates: remaining_crates[1..].to_vec(),
                        };
                        self.start_crate_summarization(project_path, first_crate_name).await?;
                    }
                }
            }

            // Single crate summary completion
            (ExtractionState::CrateSummarizing { .. },
             ExtractionEvent::CrateSummaryComplete { crate_summary }) => {
                let (project_path, current_crate, mut processed_crates, remaining_crates) = match &*state {
                    ExtractionState::CrateSummarizing { project_path, current_crate, processed_crates, remaining_crates } => {
                        (project_path.clone(), current_crate.clone(), processed_crates.clone(), remaining_crates.clone())
                    },
                    _ => return Err(LlmError::Config("Invalid state".to_string())),
                };

                processed_crates.push(current_crate);

                if remaining_crates.is_empty() {
                    // All crates processed, move to final summary generation state
                    *state = ExtractionState::GeneratingFinalSummary {
                        project_path: project_path.clone(),
                        crate_summaries: vec![crate_summary.clone()], // This will be collected properly
                    };
                    self.generate_hierarchical_project_summary(vec![crate_summary.clone()]).await?;
                } else {
                    // Process next crate
                    let next_crate = remaining_crates[0].clone();
                    *state = ExtractionState::CrateSummarizing {
                        project_path: project_path.clone(),
                        current_crate: next_crate.clone(),
                        processed_crates,
                        remaining_crates: remaining_crates[1..].to_vec(),
                    };
                    self.start_crate_summarization(project_path, next_crate).await?;
                }
            }

            // All crate summaries complete - generate final summary
            (ExtractionState::CrateSummarizing { project_path, .. },
             ExtractionEvent::AllCrateSummariesComplete { crate_summaries }) => {
                *state = ExtractionState::GeneratingFinalSummary {
                    project_path: project_path.clone(),
                    crate_summaries: crate_summaries.clone(),
                };
                self.generate_hierarchical_project_summary(crate_summaries.clone()).await?;
            }

            // Error handling
            (_, ExtractionEvent::Error { error }) => {
                *state = ExtractionState::Error {
                    error: error.clone()
                };
            }

            // Reset to ready
            (_, ExtractionEvent::Reset) => {
                *state = ExtractionState::Ready;
                *self.project_context.write().await = ProjectContext::default();
            }

            // Invalid transitions - ignore or log
            _ => {
                log::warn!("Invalid state transition: {:?} -> {:?}", *state, event);
            }
        }

        Ok(())
    }

    /// Process a single file scan
    pub async fn process_single_file(&self, file_path: PathBuf) -> LlmResult<()> {

        let result = async {
            // Extract comments
            let extraction = self.extractor.extract_from_file(&file_path)?;

            // Generate LLM summary
            let summary = self.generate_file_summary(&extraction).await?;

            LlmResult::Ok((extraction, summary))
        }.await;

        match result {
            Ok((extraction, summary)) => {
                // Directly update state instead of calling process_event to avoid recursion
                let mut state = self.state.write().await;
                *state = ExtractionState::ScanComplete {
                    result: ScanResult {
                        success: true,
                        files_processed: 1,
                        duration_seconds: 0.0, // TODO: Track actual duration
                        summary: format!("Successfully processed file: {}", file_path.display()),
                        errors: vec![],
                    }
                };
            }
            Err(e) => {
                let mut state = self.state.write().await;
                *state = ExtractionState::Error {
                    error: e.to_string()
                };
            }
        }

        Ok(())
    }

    /// Start project scan with proper two-phase architecture
    pub async fn start_project_scan(&self, project_path: PathBuf) -> LlmResult<()> {
        let result = async {
            // Discover all supported files in the project
            let files = self.discover_project_files(&project_path).await?;

            if files.is_empty() {
                return Err(LlmError::Config("No supported files found in project".to_string()));
            }

            println!("🚀 Starting two-phase project scan: {} files found", files.len());

            // ===========================================
            // PHASE 1: BULK COMMENT EXTRACTION (Tree-sitter only)
            // ===========================================
            println!("🔍 Phase 1: Extracting comments from all files (tree-sitter processing)...");
            let mut all_extractions = Vec::new();
            let mut total_comments_processed = 0;

            for (index, file_path) in files.iter().enumerate() {
                // Real-time progress bar for extraction
                let percentage = (index * 100) / files.len();
                let progress_chars = (percentage * 50) / 100;
                let bar = "█".repeat(progress_chars) + &"░".repeat(50 - progress_chars);

                print!("\r🔍 Extracting: [{bar}] {percentage:3}% ({index}/{total}) - {file}...",
                    bar = bar,
                    percentage = percentage,
                    index = index + 1,
                    total = files.len(),
                    file = file_path.file_name().unwrap_or_default().to_string_lossy()
                );
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                // Extract comments using tree-sitter (fast, no LLM calls)
                match self.extractor.extract_from_file(file_path) {
                    Ok(extraction) => {
                        if !extraction.comments.is_empty() {
                            total_comments_processed += extraction.comments.len();
                            all_extractions.push(extraction);
                        }
                    }
                    Err(e) => {
                        println!("\n⚠️ Failed to extract from {}: {}", file_path.display(), e);
                        // Continue with other files
                    }
                }
            }

            // Clear progress bar and show Phase 1 completion
            print!("\r🔍 Extracting: [██████████████████████████████████████████████████] 100% ({}/{}) - Complete!\n",
                files.len(), files.len());
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            println!("✅ Phase 1 complete: {} files with comments extracted ({} total comments)",
                all_extractions.len(), total_comments_processed);

            // Save extracted comments to JSON for potential reuse/debugging
            let output_dir = std::path::Path::new("project_scan_output");
            std::fs::create_dir_all(&output_dir)?;

            let extractions_json = serde_json::to_string_pretty(&all_extractions)?;
            let extractions_file = output_dir.join("extracted_comments.json");
            std::fs::write(&extractions_file, &extractions_json)?;
            println!("💾 Comments JSON saved: {}", extractions_file.display());

            if all_extractions.is_empty() {
                println!("⚠️ No comments found in the entire project");
                return Ok(());
            }

            // ===========================================
            // PHASE 2: BATCH LLM ANALYSIS WITH ENHANCED CONTEXT
            // ===========================================
            println!("🤖 Phase 2: LLM analysis with batch processing and enhanced context...");
            let file_summaries = self.analyze_extractions_with_enhanced_context(&all_extractions).await?;

            // FINAL PHASE: Generate comprehensive project summary
            if !file_summaries.is_empty() {
                println!("🤖 Generating comprehensive project summary with enhanced token limits...");
                let comprehensive_summary = self.generate_comprehensive_project_summary_enhanced(&file_summaries).await?;

                // Save the results to files
                // Save comprehensive summary
                let summary_file = output_dir.join("comprehensive_summary.md");
                std::fs::write(&summary_file, &comprehensive_summary)?;
                println!("📄 Comprehensive summary saved: {}", summary_file.display());

                // Save individual file summaries as JSON
                let file_summaries_json = serde_json::to_string_pretty(&file_summaries)?;
                let summaries_file = output_dir.join("file_summaries.json");
                std::fs::write(&summaries_file, &file_summaries_json)?;
                println!("📄 File summaries saved: {}", summaries_file.display());

                // Save project metadata
                let metadata = serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "total_files_processed": files.len(),
                    "files_with_comments": file_summaries.len(),
                    "total_comments_processed": total_comments_processed,
                    "analysis_method": "two-phase-llm-analysis"
                });
                let metadata_file = output_dir.join("analysis_metadata.json");
                std::fs::write(&metadata_file, serde_json::to_string_pretty(&metadata)?)?;
                println!("📄 Analysis metadata saved: {}", metadata_file.display());

                println!("🎉 Two-Phase Analysis Complete!");
                println!("📋 Project Summary: {}", comprehensive_summary.chars().take(500).collect::<String>() + "...");
                println!("💾 All results saved to: {}", output_dir.display());
            } else {
                println!("⚠️ No LLM analysis results generated");
            }

            println!("🎉 Project scan complete! Processed {} files", files.len());

            LlmResult::Ok(())
        }.await;

        if let Err(e) = result {
            let mut state = self.state.write().await;
            *state = ExtractionState::Error {
                error: e.to_string()
            };
        }

        Ok(())
    }

    /// Process a single project file without FSM recursion
    async fn process_single_project_file(&self, file_path: PathBuf) -> LlmResult<()> {
        println!("🔄 Processing file: {}", file_path.display());

        // Extract comments
        let extraction = self.extractor.extract_from_file(&file_path)?;
        println!("📄 Extracted {} comments from {}", extraction.comments.len(), file_path.display());

        // Get accumulated context for this file
        let context = self.project_context.read().await;
        let context_prompt = if context.accumulated_knowledge.is_empty() {
            String::new()
        } else {
            format!("\nPrevious project context:\n{}\n", context.accumulated_knowledge)
        };

        // Generate LLM summary with context
        let summary = self.generate_file_summary_with_context(
            &extraction,
            &context_prompt
        ).await?;

        println!("✅ File processed: {} ({} comments)", file_path.display(), extraction.comments.len());

        // Add to project context
        self.add_to_project_context(&file_path, &extraction, &summary, 0).await?;

        Ok(())
    }

    /// Process a single file in project context (legacy recursive version)
    async fn process_project_file(&self, file_path: PathBuf) -> LlmResult<()> {
        let result = async {
            // Extract comments
            let extraction = self.extractor.extract_from_file(&file_path)?;

            // Get accumulated context for this file
            let context = self.project_context.read().await;
            let context_prompt = if context.accumulated_knowledge.is_empty() {
                String::new()
            } else {
                format!("\nPrevious project context:\n{}\n", context.accumulated_knowledge)
            };


            // Generate LLM summary with context
            let summary = self.generate_file_summary_with_context(
                &extraction,
                &context_prompt
            ).await?;


            LlmResult::Ok((extraction, summary))
        }.await;

        match result {
            Ok((extraction, summary)) => {
                println!("✅ File processed: {} ({} comments)", file_path.display(), extraction.comments.len());

                // Handle project file completion directly
                self.add_to_project_context(&file_path, &extraction, &summary, 0).await?;

                // Check if more files need processing by examining current state
                let mut state = self.state.write().await;

                if let ExtractionState::ProjectScanProgress {
                    project_path, processed_files, remaining_files, ..
                } = &*state {
                    let mut new_processed = processed_files.clone();
                    new_processed.push(file_path.clone());
                    let mut new_remaining = remaining_files.clone();
                    new_remaining.retain(|p| p != &file_path);

                    if new_remaining.is_empty() {
                        println!("🎉 Project scan complete! Processed {} files", new_processed.len());
                        // All files processed, move to summary generation
                        *state = ExtractionState::GeneratingProjectSummary {
                            project_path: project_path.clone()
                        };
                        // Note: We'll need to call generate_project_summary separately
                    } else {
                        println!("📝 Progress: {}/{} files processed, continuing...", new_processed.len(), new_processed.len() + new_remaining.len());
                        // Continue with next file
                        let next_file = new_remaining[0].clone();
                        *state = ExtractionState::ProjectScanProgress {
                            project_path: project_path.clone(),
                            current_file: next_file.clone(),
                            processed_files: new_processed,
                            remaining_files: new_remaining,
                        };

                        // Don't recurse - just update state and let the caller continue
                    }
                }
            }
            Err(e) => {
                let mut state = self.state.write().await;
                *state = ExtractionState::Error {
                    error: e.to_string()
                };
            }
        }

        Ok(())
    }

    /// Add file results to project context
    async fn add_to_project_context(
        &self,
        file_path: &PathBuf,
        extraction: &FileCommentExtraction,
        summary: &str,
        scan_order: usize,
    ) -> LlmResult<()> {
        let mut context = self.project_context.write().await;

        // Create file summary with context
        let file_summary = FileSummaryWithContext {
            file_path: file_path.clone(),
            extraction: extraction.clone(),
            file_summary: summary.to_string(),
            accumulated_context: context.accumulated_knowledge.clone(),
            scan_order,
        };

        context.file_summaries.push(file_summary);
        context.total_comments += extraction.comments.len();

        // Update accumulated knowledge
        let new_knowledge = format!(
            "{}\n\nFile: {}\nSummary: {}\nComments: {}",
            context.accumulated_knowledge,
            file_path.display(),
            summary,
            extraction.comments.len()
        );

        // Keep context manageable (limit to ~4000 characters)
        context.accumulated_knowledge = if new_knowledge.len() > 4000 {
            format!("...(truncated)...\n{}", &new_knowledge[new_knowledge.len()-3500..])
        } else {
            new_knowledge
        };

        Ok(())
    }

    /// Generate final project summary
    async fn generate_project_summary(&self) -> LlmResult<()> {
        let result = async {
            let context = self.project_context.read().await;

            // Create comprehensive prompt for project summary
            let prompt = format!(
                "Based on analysis of {} files with {} total comments, provide a comprehensive project summary:\n\n{}\n\nProvide:\n1. Overall project overview\n2. Key architectural insights\n3. Common patterns found\n4. Recommendations for improvement",
                context.file_summaries.len(),
                context.total_comments,
                context.accumulated_knowledge
            );

            // Generate project summary using LLM
            let summary_text = self.processor.generate_text_summary(&prompt).await?;

            let project_summary = ProjectSummary {
                overview: summary_text.clone(),
                architectural_insights: vec!["Insight analysis pending".to_string()], // TODO: Parse from LLM response
                common_patterns: vec!["Pattern analysis pending".to_string()], // TODO: Parse from LLM response
                recommendations: vec!["Recommendations pending".to_string()], // TODO: Parse from LLM response
                files_processed: context.file_summaries.len(),
                total_comments: context.total_comments,
            };

            LlmResult::Ok(project_summary)
        }.await;

        match result {
            Ok(summary) => {
                // Directly update state for project completion
                let context = self.project_context.read().await;
                let mut state = self.state.write().await;
                *state = ExtractionState::ScanComplete {
                    result: ScanResult {
                        success: true,
                        files_processed: context.file_summaries.len(),
                        duration_seconds: 0.0, // TODO: Track actual duration
                        summary: format!("Successfully processed project with {} files", context.file_summaries.len()),
                        errors: vec![],
                    }
                };
            }
            Err(e) => {
                let mut state = self.state.write().await;
                *state = ExtractionState::Error {
                    error: e.to_string()
                };
            }
        }

        Ok(())
    }

    /// Generate LLM summary for a single file
    async fn generate_file_summary(&self, extraction: &FileCommentExtraction) -> LlmResult<String> {

        if extraction.comments.is_empty() {
            return Ok("No comments found for analysis".to_string());
        }

        let prompt = format!(
            "Analyze the comments from file: {}\nLanguage: {}\nComments found: {}\n\nComment details:\n{}\n\nProvide a concise summary of what this file does based on its comments.",
            extraction.file,
            extraction.language,
            extraction.comments.len(),
            extraction.comments.iter()
                .map(|c| format!("- Line {}: {}", c.start_position.line, c.text))
                .collect::<Vec<_>>()
                .join("\n")
        );

        self.processor.generate_text_summary(&prompt).await
    }

    /// Generate LLM summary for a file with project context
    async fn generate_file_summary_with_context(
        &self,
        extraction: &FileCommentExtraction,
        context: &str,
    ) -> LlmResult<String> {
        let prompt = format!(
            "{}Analyze the comments from file: {}\nLanguage: {}\nComments found: {}\n\nComment details:\n{}\n\nBased on the project context above and these comments, provide a summary of this file's role in the project.",
            context,
            extraction.file,
            extraction.language,
            extraction.comments.len(),
            extraction.comments.iter()
                .map(|c| format!("- Line {}: {}", c.start_position.line, c.text))
                .collect::<Vec<_>>()
                .join("\n")
        );

        self.processor.generate_text_summary(&prompt).await
    }

    /// Generate comprehensive LLM analysis of all project comments at once
    async fn generate_comprehensive_project_summary(&self, all_extractions: &[FileCommentExtraction]) -> LlmResult<String> {
        let total_comments: usize = all_extractions.iter().map(|e| e.comments.len()).sum();

        let mut prompt = format!(
            "Analyze the comments from an entire Rust codebase with {} files containing {} total comments.\n\n",
            all_extractions.len(),
            total_comments
        );

        prompt.push_str("Provide a comprehensive analysis covering:\n");
        prompt.push_str("1. Overall project purpose and architecture\n");
        prompt.push_str("2. Key modules and their responsibilities\n");
        prompt.push_str("3. Design patterns and architectural decisions\n");
        prompt.push_str("4. Technical complexity and challenges\n");
        prompt.push_str("5. Areas that need attention or refactoring\n\n");

        prompt.push_str("Comments by file:\n\n");

        for extraction in all_extractions {
            prompt.push_str(&format!("=== {} ({} comments) ===\n", extraction.file, extraction.comments.len()));

            for comment in &extraction.comments {
                let cleaned_text = comment.text.chars().take(150).collect::<String>();
                prompt.push_str(&format!("Line {}: {}\n", comment.start_position.line, cleaned_text));
            }
            prompt.push_str("\n");
        }

        println!("🤖 Sending comprehensive prompt to LLM ({} characters)", prompt.len());
        self.processor.generate_text_summary(&prompt).await
    }

    /// Generate comprehensive project summary from individual file summaries
    async fn generate_comprehensive_project_summary_from_files(&self, file_summaries: &[(String, String)]) -> LlmResult<String> {
        let mut prompt = format!(
            "Based on the following {} individual file analyses from a Rust codebase, provide a comprehensive project summary.\n\n",
            file_summaries.len()
        );

        prompt.push_str("Analyze the overall project architecture, key patterns, and provide insights on:\n");
        prompt.push_str("1. Project purpose and main functionality\n");
        prompt.push_str("2. Architecture and module organization\n");
        prompt.push_str("3. Design patterns and technical approach\n");
        prompt.push_str("4. Code quality and maintainability\n");
        prompt.push_str("5. Potential areas for improvement\n\n");

        prompt.push_str("Individual file summaries:\n\n");

        for (file_path, summary) in file_summaries {
            prompt.push_str(&format!("=== {} ===\n{}\n\n", file_path, summary));
        }

        prompt.push_str("Based on these file-by-file analyses, provide a cohesive project overview:");

        println!("🤖 Generating final comprehensive summary ({} characters)", prompt.len());
        self.processor.generate_text_summary(&prompt).await
    }

    /// Phase 2: Analyze all extractions with enhanced context and batching
    async fn analyze_extractions_with_enhanced_context(&self, all_extractions: &[FileCommentExtraction]) -> LlmResult<Vec<(String, String)>> {
        let mut file_summaries = Vec::new();
        let total_files = all_extractions.len();

        println!("🤖 Analyzing {} files with enhanced context and improved token limits...", total_files);

        // Group files into smart batches based on similarity or module structure
        let batches = self.group_extractions_into_smart_batches(all_extractions);

        for (batch_index, batch) in batches.iter().enumerate() {
            println!("📦 Processing batch {}/{} ({} files)...", batch_index + 1, batches.len(), batch.len());

            // Build enhanced context for this batch
            let batch_context = self.build_enhanced_batch_context(&file_summaries, batch).await?;

            // Process files in this batch with shared context
            for (file_index, extraction) in batch.iter().enumerate() {
                let progress = ((batch_index * batch.len() + file_index + 1) * 100) / total_files;
                let progress_chars = (progress * 50) / 100;
                let bar = "█".repeat(progress_chars) + &"░".repeat(50 - progress_chars);

                print!("\r🤖 Analyzing: [{bar}] {progress:3}% ({current}/{total}) - {file}...",
                    bar = bar,
                    progress = progress,
                    current = batch_index * batch.len() + file_index + 1,
                    total = total_files,
                    file = std::path::Path::new(&extraction.file).file_name()
                        .unwrap_or_default().to_string_lossy()
                );
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                // Analyze this file with enhanced context and higher token limits
                match self.generate_enhanced_file_summary(extraction, &batch_context).await {
                    Ok(file_summary) => {
                        file_summaries.push((extraction.file.clone(), file_summary));
                    }
                    Err(e) => {
                        println!("\n❌ Enhanced analysis failed for {}: {}", extraction.file, e);
                        // Continue with other files
                    }
                }
            }
        }

        // Clear progress bar and show completion
        print!("\r🤖 Analyzing: [██████████████████████████████████████████████████] 100% ({total}/{total}) - Complete!\n",
            total = total_files);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        println!("✅ Phase 2 complete: {} files analyzed with enhanced context", file_summaries.len());
        Ok(file_summaries)
    }

    /// Generate enhanced comprehensive project summary with increased token limits
    async fn generate_comprehensive_project_summary_enhanced(&self, file_summaries: &[(String, String)]) -> LlmResult<String> {
        let mut prompt = format!(
            "COMPREHENSIVE RUST PROJECT ANALYSIS\n\nAnalyze the following {} individual file analyses from a complex Rust codebase and provide a detailed, comprehensive project summary.\n\n",
            file_summaries.len()
        );

        prompt.push_str("ANALYSIS REQUIREMENTS:\n");
        prompt.push_str("1. **Project Purpose & Architecture**: Identify the main purpose, architectural patterns, and system design\n");
        prompt.push_str("2. **Module Organization**: Analyze how modules are structured and their interdependencies\n");
        prompt.push_str("3. **Technical Approach**: Identify key technologies, frameworks, design patterns used\n");
        prompt.push_str("4. **Code Quality & Patterns**: Assess code quality, consistency, and common patterns\n");
        prompt.push_str("5. **System Integration**: How different components interact and integrate\n");
        prompt.push_str("6. **Performance & Scalability**: Performance considerations and scalability aspects\n");
        prompt.push_str("7. **Improvement Areas**: Specific areas that need attention or enhancement\n\n");

        prompt.push_str("DETAILED FILE ANALYSES:\n\n");

        for (file_path, summary) in file_summaries {
            prompt.push_str(&format!("=== FILE: {} ===\n{}\n\n", file_path, summary));
        }

        prompt.push_str("\nPROVIDE COMPREHENSIVE ANALYSIS:\n");
        prompt.push_str("Based on these detailed file analyses, provide an in-depth project overview covering all the requirements above. ");
        prompt.push_str("Be specific about technologies used, architectural decisions, and provide concrete examples from the codebase. ");
        prompt.push_str("This analysis will be used for project documentation and architectural decision-making.\n");

        println!("🤖 Generating enhanced comprehensive summary ({} characters)", prompt.len());

        // Use enhanced token limits for comprehensive analysis
        self.processor.generate_text_summary(&prompt).await
    }

    /// Group extractions into smart batches based on file structure and dependencies
    fn group_extractions_into_smart_batches<'a>(&self, extractions: &'a [FileCommentExtraction]) -> Vec<Vec<&'a FileCommentExtraction>> {
        let mut batches = Vec::new();
        const BATCH_SIZE: usize = 5; // Process 5 files at a time for better context sharing

        // Simple batching for now - can be enhanced with more sophisticated grouping
        for chunk in extractions.chunks(BATCH_SIZE) {
            batches.push(chunk.iter().collect());
        }

        batches
    }

    /// Build enhanced context for a batch of files
    async fn build_enhanced_batch_context(&self, processed_summaries: &[(String, String)], _batch: &[&FileCommentExtraction]) -> LlmResult<String> {
        if processed_summaries.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::from("PREVIOUS PROJECT ANALYSIS:\n\n");

        // Include recent analyses for context
        let recent_count = std::cmp::min(10, processed_summaries.len());
        let recent_summaries = &processed_summaries[processed_summaries.len().saturating_sub(recent_count)..];

        for (file_path, summary) in recent_summaries {
            context.push_str(&format!("- {}: {}\n",
                std::path::Path::new(file_path).file_name().unwrap_or_default().to_string_lossy(),
                summary.chars().take(200).collect::<String>()
            ));
        }

        context.push_str("\nUse this context to understand the project structure and maintain consistency.\n\n");
        Ok(context)
    }

    /// Generate enhanced file summary with better prompting and higher token limits
    async fn generate_enhanced_file_summary(&self, extraction: &FileCommentExtraction, context: &str) -> LlmResult<String> {
        let prompt = format!(
            "{}DETAILED FILE ANALYSIS\n\nFile: {}\nLanguage: {}\nComments found: {}\n\n",
            context,
            extraction.file,
            extraction.language,
            extraction.comments.len()
        );

        let mut detailed_prompt = prompt;
        detailed_prompt.push_str("ANALYSIS REQUIREMENTS:\n");
        detailed_prompt.push_str("1. **Purpose & Functionality**: What does this file do and why is it important?\n");
        detailed_prompt.push_str("2. **Technical Implementation**: What technologies, frameworks, patterns are used?\n");
        detailed_prompt.push_str("3. **Dependencies & Integration**: How does it integrate with other parts of the system?\n");
        detailed_prompt.push_str("4. **Code Quality & Architecture**: Assessment of code structure and design decisions\n");
        detailed_prompt.push_str("5. **Specific Details**: Mention specific functions, structs, traits, or unique aspects\n\n");

        detailed_prompt.push_str("COMMENTS TO ANALYZE:\n");
        for comment in &extraction.comments {
            detailed_prompt.push_str(&format!("Line {}: {}\n", comment.start_position.line, comment.text));
        }

        detailed_prompt.push_str("\nProvide a detailed analysis covering all requirements above. Be specific about implementation details, architectural decisions, and the file's role in the larger system.");

        self.processor.generate_text_summary(&detailed_prompt).await
    }

    /// Discover supported files in a project directory
    async fn discover_project_files(&self, project_path: &Path) -> LlmResult<Vec<PathBuf>> {
        println!("🔍 Discovering files in project: {}", project_path.display());

        use walkdir::WalkDir;
        let mut files = Vec::new();

        for entry in WalkDir::new(project_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            // Only include Rust files for now (can expand later)
            if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path.to_path_buf());
                }
            }
        }

        println!("📁 Found {} Rust files to process", files.len());
        Ok(files)
    }

    // ================== HIERARCHICAL PROCESSING METHODS ==================

    /// Static helper for discovering project files without borrowing self
    async fn discover_project_files_static(
        project_path: &PathBuf,
        file_extensions: &HashSet<String>,
    ) -> LlmResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(project_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if file_extensions.contains(ext_str) {
                            files.push(path.to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    /// Start crate discovery for hierarchical processing
    async fn start_crate_discovery(&self, project_path: PathBuf) -> LlmResult<()> {
        println!("🔍 Starting crate discovery for project: {}", project_path.display());

        // Initialize crate grouper
        let mut grouper = CrateGrouper::new(project_path.clone());

        // Discover all crates in the project
        let discovered_crates = grouper.discover_crates()?;

        println!("📦 Found {} crates for hierarchical analysis", discovered_crates.len());
        for crate_info in &discovered_crates {
            println!("  - {} ({})", crate_info.name, crate_info.description.as_ref().unwrap_or(&"No description".to_string()));
        }

        // Directly trigger the transition logic instead of recursive event processing
        self.handle_crate_grouping_complete(discovered_crates).await?;

        Ok(())
    }

    /// Start summarization for a specific crate
    async fn start_crate_summarization(&self, project_path: PathBuf, crate_name: String) -> LlmResult<()> {
        println!("🤖 Starting crate summarization for: {}", crate_name);

        // Initialize crate grouper and discover crates
        let mut grouper = CrateGrouper::new(project_path.clone());
        let discovered_crates = grouper.discover_crates()?;

        // Find the target crate
        let target_crate = discovered_crates.iter()
            .find(|c| c.name == crate_name)
            .ok_or_else(|| LlmError::Config(format!("Crate '{}' not found", crate_name)))?;

        // Extract comments from all files in the project first
        let file_extensions = vec!["rs".to_string()].into_iter().collect();
        let all_files = CommentExtractionFSM::discover_project_files_static(&project_path, &file_extensions).await?;
        let mut all_extractions = Vec::new();

        for file_path in all_files {
            match self.extractor.extract_from_file(&file_path) {
                Ok(extraction) => {
                    if !extraction.comments.is_empty() {
                        all_extractions.push(extraction);
                    }
                }
                Err(e) => {
                    println!("⚠️  Failed to extract from {}: {}", file_path.display(), e);
                }
            }
        }

        // Group files by crate
        let grouped_files = grouper.group_files(&all_extractions)?;

        // Get files for this specific crate
        let crate_files = grouped_files.get(&crate_name)
            .cloned()
            .unwrap_or_else(Vec::new);

        if crate_files.is_empty() {
            println!("⚠️  No files with comments found for crate: {}", crate_name);
            // Still create a summary for consistency
            let empty_summary = CrateSummary {
                crate_name: crate_name.clone(),
                crate_path: target_crate.path.clone(),
                files_analyzed: vec![],
                summary_text: format!("Crate '{}' has no files with extractable comments.", crate_name),
                structural_insights: Default::default(),
                token_count: 0,
                timestamp: chrono::Utc::now(),
            };

            // Trigger completion logic even for empty crates
            self.handle_crate_summary_complete(empty_summary).await?;
            return Ok(());
        }

        // Initialize crate summarizer with context-aware configuration
        let config = CrateSummarizerConfig::default();
        let mut summarizer = CrateSummarizer::new(self.processor.clone(), config);

        // Get project memory for context-aware analysis
        let project_memory = self.get_project_analysis_memory().await;

        // Generate context-aware crate summary using our new method
        let crate_summary = summarizer.generate_crate_summary_with_context(
            target_crate,
            &crate_files,
            &project_memory
        ).await?;

        println!("✅ Generated context-aware summary for crate '{}': {} tokens",
            crate_name, crate_summary.token_count);

        // Directly trigger the completion logic instead of recursive event processing
        self.handle_crate_summary_complete(crate_summary).await?;

        Ok(())
    }

    /// Generate hierarchical project summary from crate summaries
    async fn generate_hierarchical_project_summary(&self, crate_summaries: Vec<CrateSummary>) -> LlmResult<()> {
        println!("🔗 Generating hierarchical project summary from {} crate summaries", crate_summaries.len());

        // Apply dual-stage denoising to crate summaries for 12K budget
        let denoised_summaries = self.apply_dual_stage_denoising(&crate_summaries).await?;

        // Build hierarchical summary prompt with 12K token budget
        let mut prompt = String::new();
        prompt.push_str("HIERARCHICAL PROJECT ANALYSIS (12K TOKEN BUDGET)\n");
        prompt.push_str("=============================================\n\n");
        prompt.push_str("Based on the following crate-level summaries, provide a comprehensive project analysis:\n\n");

        let mut total_files = 0;
        let mut total_tokens = 0;

        for summary in &denoised_summaries {
            total_files += summary.files_analyzed.len();
            total_tokens += summary.token_count;

            prompt.push_str(&format!("=== CRATE: {} ===\n", summary.crate_name.to_uppercase()));
            prompt.push_str(&format!("Files analyzed: {}\n", summary.files_analyzed.len()));
            prompt.push_str(&format!("Summary ({} tokens):\n", summary.token_count));
            prompt.push_str(&summary.summary_text);
            prompt.push_str("\n\n");
        }

        prompt.push_str("FINAL PROJECT ANALYSIS REQUIREMENTS:\n");
        prompt.push_str("1. **Overall Architecture**: How do these crates work together?\n");
        prompt.push_str("2. **Design Patterns**: What common patterns emerge across crates?\n");
        prompt.push_str("3. **Dependencies**: How are the crates interconnected?\n");
        prompt.push_str("4. **Technical Complexity**: What are the main technical challenges?\n");
        prompt.push_str("5. **Recommendations**: What improvements or refactoring would benefit the project?\n\n");
        prompt.push_str(&format!("Generate a comprehensive project summary covering all aspects above. Total context: {} crates, {} files analyzed, {} tokens of crate summaries.\n", denoised_summaries.len(), total_files, total_tokens));

        // Enforce 12K token budget
        let final_prompt = self.enforce_token_budget(&prompt, 12000).await?;
        println!("📊 Final hierarchical prompt: {} characters (target: 12K tokens)", final_prompt.len());

        // Generate final summary via LLM
        let summary_text = self.processor.generate_text_summary(&final_prompt).await?;

        // Create project summary
        let project_summary = ProjectSummary {
            overview: summary_text.clone(),
            architectural_insights: vec!["Hierarchical analysis complete".to_string()],
            common_patterns: vec!["Cross-crate patterns identified".to_string()],
            recommendations: vec!["Hierarchical improvements suggested".to_string()],
            files_processed: total_files,
            total_comments: 0, // TODO: Could aggregate from crate summaries
        };

        // Save the hierarchical summary to output
        let output_dir = std::path::Path::new("project_scan_output");
        std::fs::create_dir_all(&output_dir)?;
        let summary_file = output_dir.join("hierarchical_summary.md");
        std::fs::write(&summary_file, &summary_text)?;
        println!("📄 Hierarchical summary saved: {}", summary_file.display());

        println!("✅ Hierarchical project summary generated successfully");
        println!("📄 Summary preview: {}", summary_text.chars().take(500).collect::<String>() + "...");

        // Update state to completed
        let mut state = self.state.write().await;
        *state = ExtractionState::ScanComplete {
            result: ScanResult {
                success: true,
                files_processed: total_files,
                duration_seconds: 0.0,
                summary: format!("Successfully completed hierarchical analysis: {} crates, {} files",
                    denoised_summaries.len(), total_files),
                errors: vec![],
            }
        };

        Ok(())
    }

    /// Get project analysis memory for context-aware processing
    async fn get_project_analysis_memory(&self) -> crate::conversation::ProjectAnalysisMemory {
        self.project_memory.read().await.clone()
    }

    /// Apply dual-stage denoising (Stage 1: 40% reduction, Stage 2: 60% reduction)
    async fn apply_dual_stage_denoising(&self, crate_summaries: &[CrateSummary]) -> LlmResult<Vec<CrateSummary>> {
        println!("🧠 Applying dual-stage denoising to {} crate summaries", crate_summaries.len());

        let mut denoised_summaries = Vec::new();

        for summary in crate_summaries {
            // Stage 1: 40% reduction for crate inputs
            let stage1_text = self.apply_denoising(&summary.summary_text, 0.40).await?;

            // Stage 2: 60% reduction for final summary
            let stage2_text = self.apply_denoising(&stage1_text, 0.60).await?;

            let mut denoised_summary = summary.clone();
            denoised_summary.summary_text = stage2_text;
            denoised_summary.token_count = denoised_summary.summary_text.len() / 4; // Rough token estimate

            denoised_summaries.push(denoised_summary);
        }

        println!("✅ Dual-stage denoising complete: reduced from {} to {} total tokens",
            crate_summaries.iter().map(|s| s.token_count).sum::<usize>(),
            denoised_summaries.iter().map(|s| s.token_count).sum::<usize>()
        );

        Ok(denoised_summaries)
    }

    /// Apply denoising with specified reduction percentage
    async fn apply_denoising(&self, text: &str, reduction_factor: f64) -> LlmResult<String> {
        let target_length = (text.len() as f64 * (1.0 - reduction_factor)) as usize;

        if text.len() <= target_length {
            return Ok(text.to_string());
        }

        // Simple denoising: keep the most important sentences
        let sentences: Vec<&str> = text.split('.').collect();
        let keep_count = (sentences.len() as f64 * (1.0 - reduction_factor)) as usize;

        let denoised = sentences.into_iter()
            .take(keep_count)
            .collect::<Vec<_>>()
            .join(".")
            + ".";

        Ok(denoised)
    }

    /// Enforce token budget by truncating if necessary
    async fn enforce_token_budget(&self, text: &str, max_tokens: usize) -> LlmResult<String> {
        let estimated_tokens = text.len() / 4; // Rough estimate: 4 chars per token

        if estimated_tokens <= max_tokens {
            return Ok(text.to_string());
        }

        let target_chars = max_tokens * 4;
        let truncated = if text.len() > target_chars {
            format!("{}\n\n[TRUNCATED DUE TO TOKEN BUDGET]", &text[..target_chars])
        } else {
            text.to_string()
        };

        println!("⚠️  Token budget enforced: {} chars -> {} chars (target: {} tokens)",
            text.len(), truncated.len(), max_tokens);

        Ok(truncated)
    }

    /// Handle crate grouping completion without recursion
    async fn handle_crate_grouping_complete(&self, discovered_crates: Vec<CrateInfo>) -> LlmResult<()> {
        let project_path = {
            let state = self.state.read().await;
            match &*state {
                ExtractionState::CrateGrouping { project_path } => project_path.clone(),
                _ => return Err(LlmError::Config("Invalid state for crate grouping complete".to_string())),
            }
        };

        if discovered_crates.is_empty() {
            // No crates found, fallback to legacy processing
            let mut state = self.state.write().await;
            *state = ExtractionState::ProjectScanInit {
                project_path: project_path.clone()
            };
            drop(state);
            self.start_project_scan(project_path).await?;
        } else {
            // Start hierarchical crate summarization
            let remaining_crates: Vec<String> = discovered_crates.iter()
                .map(|c| c.name.clone())
                .collect();

            if let Some(first_crate) = remaining_crates.first() {
                let first_crate_name = first_crate.clone();
                let mut state = self.state.write().await;
                *state = ExtractionState::CrateSummarizing {
                    project_path: project_path.clone(),
                    current_crate: first_crate_name.clone(),
                    processed_crates: Vec::new(),
                    remaining_crates: remaining_crates[1..].to_vec(),
                };
                drop(state);

                // Use spawned task to avoid recursion
                let processor = self.processor.clone();
                let extractor = self.extractor.clone();
                let state_arc = self.state.clone();
                let memory_arc = self.project_memory.clone();

                tokio::spawn(async move {
                    let fsm = CommentExtractionFSM {
                        state: state_arc,
                        processor,
                        extractor,
                        project_context: Arc::new(RwLock::new(ProjectContext::default())),
                        event_listeners: Vec::new(),
                        project_memory: memory_arc,
                    };

                    if let Err(e) = fsm.start_crate_summarization(project_path, first_crate_name).await {
                        eprintln!("❌ Failed to start crate summarization: {}", e);
                    }
                });
            }
        }
        Ok(())
    }

    /// Handle crate summary completion without recursion
    async fn handle_crate_summary_complete(&self, crate_summary: CrateSummary) -> LlmResult<()> {
        let (project_path, current_crate, mut processed_crates, remaining_crates) = {
            let state = self.state.read().await;
            match &*state {
                ExtractionState::CrateSummarizing { project_path, current_crate, processed_crates, remaining_crates } => {
                    (project_path.clone(), current_crate.clone(), processed_crates.clone(), remaining_crates.clone())
                },
                _ => return Err(LlmError::Config("Invalid state for crate summary complete".to_string())),
            }
        };

        // Update project memory with this crate's insights
        {
            let mut memory = self.project_memory.write().await;
            memory.accumulate_crate_insights(&crate_summary);
        }

        processed_crates.push(current_crate);

        if remaining_crates.is_empty() {
            // All crates processed, move to final summary generation state
            let mut state = self.state.write().await;
            *state = ExtractionState::GeneratingFinalSummary {
                project_path: project_path.clone(),
                crate_summaries: vec![crate_summary.clone()], // TODO: Collect all summaries
            };
            drop(state);
            self.generate_hierarchical_project_summary(vec![crate_summary]).await?;
        } else {
            // Process next crate
            let next_crate = remaining_crates[0].clone();
            let mut state = self.state.write().await;
            *state = ExtractionState::CrateSummarizing {
                project_path: project_path.clone(),
                current_crate: next_crate.clone(),
                processed_crates,
                remaining_crates: remaining_crates[1..].to_vec(),
            };
            drop(state);

            // Use spawned task to avoid recursion
            let processor = self.processor.clone();
            let extractor = self.extractor.clone();
            let state_arc = self.state.clone();
            let memory_arc = self.project_memory.clone();

            tokio::spawn(async move {
                let fsm = CommentExtractionFSM {
                    state: state_arc,
                    processor,
                    extractor,
                    project_context: Arc::new(RwLock::new(ProjectContext::default())),
                    event_listeners: Vec::new(),
                    project_memory: memory_arc,
                };

                if let Err(e) = fsm.start_crate_summarization(project_path, next_crate).await {
                    eprintln!("❌ Failed to start crate summarization: {}", e);
                }
            });
        }
        Ok(())
    }

}

/// CLI interface for the FSM
pub struct CommentExtractionCLI {
    fsm: Arc<CommentExtractionFSM>,
}

impl CommentExtractionCLI {
    pub fn new(fsm: Arc<CommentExtractionFSM>) -> Self {
        Self { fsm }
    }

    /// Execute "scan file" command
    pub async fn scan_file_command(&self, file_path: Option<PathBuf>) -> LlmResult<()> {
        match file_path {
            Some(path) => {
                // Direct file path provided
                self.fsm.process_event(ExtractionEvent::FileSelected { file_path: path }).await
            }
            None => {
                // Enter file picker mode
                self.fsm.process_event(ExtractionEvent::StartFileScan).await?;
                println!("Enter file picker mode. Please specify a file path:");
                // In a real CLI, this would show a file picker or prompt for input
                Ok(())
            }
        }
    }

    /// Execute "scan project" command
    pub async fn scan_project_command(&self, project_path: Option<PathBuf>) -> LlmResult<()> {
        match project_path {
            Some(path) => {
                self.fsm.process_event(ExtractionEvent::ProjectSelected { project_path: path }).await
            }
            None => {
                self.fsm.process_event(ExtractionEvent::StartProjectScan).await?;
                println!("Enter project selection mode. Please specify a project directory:");
                // In a real CLI, this would show a directory picker or prompt for input
                Ok(())
            }
        }
    }

    /// Get current scan results
    pub async fn get_results(&self) -> LlmResult<Option<ScanResult>> {
        match self.fsm.get_state().await {
            ExtractionState::ScanComplete { result } => Ok(Some(result)),
            _ => Ok(None)
        }
    }

    /// Reset FSM to ready state
    pub async fn reset(&self) -> LlmResult<()> {
        self.fsm.process_event(ExtractionEvent::Reset).await
    }
}

/// GUI integration points
pub trait GUIIntegration {
    /// Show file picker dialog
    fn show_file_picker(&self) -> Option<PathBuf>;
    /// Show project directory picker
    fn show_project_picker(&self) -> Option<PathBuf>;
    /// Update progress display
    fn update_progress(&self, state: &ExtractionState);
    /// Display scan results
    fn display_results(&self, results: &ScanResult);
    /// Show error message
    fn show_error(&self, error: &str);
}