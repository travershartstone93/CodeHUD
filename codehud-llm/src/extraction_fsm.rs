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
use indicatif::{ProgressBar, ProgressStyle};
use crate::progress_monitor::ProgressMonitor;

/// FSM states for the comment extraction process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Whether to use insights-only mode for ultra token-efficient analysis
    insights_only: bool,
    /// Optional Google AI Studio API key for final summary (uses Gemini Flash instead of local 14B)
    gemini_api_key: Option<String>,
}

impl CommentExtractionFSM {
    /// Create a new FSM instance
    pub async fn new(
        ollama_config: OllamaConfig,
        processor_config: ProcessorConfig,
        insights_only: bool,
        gemini_api_key: Option<String>,
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
            insights_only,
            gemini_api_key,
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

        match &event {
            ExtractionEvent::ProjectSelected { project_path } => {
                println!("🔍 FSM DEBUG: ProjectSelected event received for: {}", project_path.display());
                println!("🔍 FSM DEBUG: Current state: {:?}", std::mem::discriminant(&*state));
            }
            _ => {}
        }

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

                // Processing is now handled directly in start_crate_discovery
                // This event handler is no longer needed but kept for compatibility
                println!("🔍 CrateGroupingComplete event received, processing already handled directly");
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
                    self.generate_hierarchical_project_summary(project_path.clone(), vec![crate_summary.clone()]).await?;
                } else {
                    // Process next crate
                    let next_crate = remaining_crates[0].clone();
                    *state = ExtractionState::CrateSummarizing {
                        project_path: project_path.clone(),
                        current_crate: next_crate.clone(),
                        processed_crates,
                        remaining_crates: remaining_crates[1..].to_vec(),
                    };
                    // This is now handled directly in start_crate_discovery
                }
            }

            // All crate summaries complete - generate final summary
            (ExtractionState::CrateSummarizing { project_path, .. },
             ExtractionEvent::AllCrateSummariesComplete { crate_summaries }) => {
                let project_path_clone = project_path.clone();
                let crate_summaries_clone = crate_summaries.clone();
                *state = ExtractionState::GeneratingFinalSummary {
                    project_path: project_path_clone.clone(),
                    crate_summaries: crate_summaries_clone.clone(),
                };
                drop(state); // Release the borrow before async call
                self.generate_hierarchical_project_summary(project_path_clone, crate_summaries_clone).await?;
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
            let output_dir = project_path.join("project_scan_output");
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
                        // Release the lock before calling generate_project_summary
                        drop(state);
                        self.generate_project_summary().await?;
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

        // Keep context manageable (limit to ~8000 characters)
        context.accumulated_knowledge = if new_knowledge.len() > 8000 {
            format!("...(truncated)...\n{}", &new_knowledge[new_knowledge.len()-7000..])
        } else {
            new_knowledge
        };

        Ok(())
    }

    /// Generate final project summary
    async fn generate_project_summary(&self) -> LlmResult<()> {
        let result: LlmResult<ProjectSummary> = async {
            let context = self.project_context.read().await;

            // Create comprehensive prompt for project summary
            let prompt = format!(
                "Based on analysis of {} files with {} total comments, provide a comprehensive project summary:\n\n{}\n\nProvide:\n1. Overall project overview\n2. Key architectural insights\n3. Common patterns found\n4. Recommendations for improvement",
                context.file_summaries.len(),
                context.total_comments,
                context.accumulated_knowledge
            );

            // Generate project summary using LLM
            let summary_text = self.processor.generate_hierarchical_summary(&prompt).await?;

            let project_summary = ProjectSummary {
                overview: summary_text.clone(),
                architectural_insights: vec!["Insight analysis pending".to_string()], // TODO: Parse from LLM response
                common_patterns: vec!["Pattern analysis pending".to_string()], // TODO: Parse from LLM response
                recommendations: vec!["Recommendations pending".to_string()], // TODO: Parse from LLM response
                files_processed: context.file_summaries.len(),
                total_comments: context.total_comments,
            };

            Ok(project_summary)
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

        self.processor.generate_hierarchical_summary(&prompt).await
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

        self.processor.generate_hierarchical_summary(&prompt).await
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
        self.processor.generate_hierarchical_summary(&prompt).await
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
        self.processor.generate_hierarchical_summary(&prompt).await
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

    /// Ultra token-efficient insights-only file analysis for budget-constrained mode
    async fn analyze_extractions_insights_only(&self, all_extractions: &[FileCommentExtraction]) -> LlmResult<Vec<(String, String)>> {
        let mut file_summaries = Vec::new();
        let total_files = all_extractions.len();

        println!("⚡ Analyzing {} files with insights-only mode (ultra token-efficient)...", total_files);

        for (file_index, extraction) in all_extractions.iter().enumerate() {
            let progress = ((file_index + 1) * 100) / total_files;
            let progress_chars = (progress * 50) / 100;
            let bar = "█".repeat(progress_chars) + &"░".repeat(50 - progress_chars);

            print!("\r⚡ Analyzing: [{bar}] {progress:3}% ({current}/{total}) - {file}...",
                bar = bar,
                progress = progress,
                current = file_index + 1,
                total = total_files,
                file = std::path::Path::new(&extraction.file).file_name()
                    .unwrap_or_default().to_string_lossy()
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            // Generate ultra-compact summary using only structural insights
            match self.generate_insights_only_file_summary(extraction).await {
                Ok(file_summary) => {
                    file_summaries.push((extraction.file.clone(), file_summary));
                }
                Err(e) => {
                    println!("\n❌ Insights-only analysis failed for {}: {}", extraction.file, e);
                    // Continue with other files
                }
            }
        }

        // Clear progress bar and show completion
        print!("\r⚡ Analyzing: [██████████████████████████████████████████████████] 100% ({total}/{total}) - Complete!\n",
            total = total_files);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        println!("✅ Phase 2 complete: {} files analyzed with insights-only mode", file_summaries.len());

        Ok(file_summaries)
    }

    /// Generate ultra-compact file summary using only structural insights (token-efficient)
    async fn generate_insights_only_file_summary(&self, extraction: &FileCommentExtraction) -> LlmResult<String> {
        // Build ultra-compact prompt using only structural insights
        let mut prompt = String::new();
        prompt.push_str("CONCISE FILE SUMMARY\n\n");
        prompt.push_str(&format!("File: {}\n", self.make_relative_path(&extraction.file)));
        prompt.push_str(&format!("Language: {}\n", extraction.language));

        // Add structural insights if available
        if let Some(insights) = &extraction.structural_insights {
            if !insights.sections.is_empty() {
                prompt.push_str("STRUCTURAL CONTEXT:\n");
                for (section, items) in &insights.sections {
                    if !items.is_empty() {
                        prompt.push_str(&format!("{}: {}\n",
                            section.replace('_', " "),
                            items.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                        ));
                    }
                }
                prompt.push('\n');
            }
        }

        // Add only essential comments (limit to ~10 most important)
        let essential_comments: Vec<_> = extraction.comments.iter()
            .filter(|c| c.text.len() > 20) // Filter out very short comments
            .take(10) // Limit to 10 comments max
            .collect();

        if !essential_comments.is_empty() {
            prompt.push_str("COMMENTS TO ANALYZE:\n");
            for comment in essential_comments {
                prompt.push_str(&format!("Line {}: {}\n", comment.start_position.line, comment.text));
            }
            prompt.push('\n');
        }

        prompt.push_str("Technical Summary: In 2-3 direct sentences, explain what this file DOES for users/callers. List specific functions, features, or behaviors it implements. Use concrete examples (e.g., \"detects SQL injection\", \"parses JSON configs\", \"calculates hash values\"). Avoid generic phrases like \"serves as\", \"provides functionality\", \"manages\", or \"designed to\". Maximum 100 words.\n");

        // Debug: Show prompt length to verify it's compact
        println!("📏 Prompt length: {} characters", prompt.len());

        // Generate summary using the LLM
        self.processor.generate_hierarchical_summary(&prompt).await
    }

    /// Convert full file path to relative path for token efficiency
    fn make_relative_path(&self, full_path: &str) -> String {
        // For now, just return the file name - this can be enhanced later with project-relative paths
        std::path::Path::new(full_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(full_path)
            .to_string()
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
        self.processor.generate_hierarchical_summary(&prompt).await
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
            "{}CONCISE FILE SUMMARY\n\nFile: {}\nLanguage: {}\nComments found: {}\n\n",
            context,
            extraction.file,
            extraction.language,
            extraction.comments.len()
        );

        let mut detailed_prompt = prompt;

        // Add structural insights if available
        if let Some(ref insights) = extraction.structural_insights {
            if !insights.sections.is_empty() {
                detailed_prompt.push_str("STRUCTURAL CONTEXT:\n");
                for (section_name, items) in &insights.sections {
                    detailed_prompt.push_str(&format!("{}: {}\n", section_name, items.join(", ")));
                }
                detailed_prompt.push_str("\n");
            }
        }

        // If no structural insights, extract basic file context from file path and language
        if extraction.structural_insights.is_none() ||
           extraction.structural_insights.as_ref().map_or(true, |i| i.sections.is_empty()) {
            detailed_prompt.push_str("BASIC FILE CONTEXT:\n");
            detailed_prompt.push_str(&format!("File: {}\n", extraction.file));
            detailed_prompt.push_str(&format!("Language: {}\n", extraction.language));

            // Extract filename which often indicates purpose
            if let Some(filename) = std::path::Path::new(&extraction.file).file_stem() {
                let filename_str = filename.to_string_lossy();
                detailed_prompt.push_str(&format!("Module name: {}\n", filename_str));
            }
            detailed_prompt.push_str("\n");
        }

        // Filter out garbage comments (regex patterns, single words, etc.)
        let meaningful_comments: Vec<_> = extraction.comments.iter()
            .filter(|comment| {
                let text = comment.text.trim();
                // Skip single words, regex patterns, and other garbage
                if text.len() < 10 { return false; }
                if text.contains("\\b") || text.contains("\\d") || text.contains("\\w") { return false; }
                if text.split_whitespace().count() < 3 { return false; }
                // Skip common regex comment patterns
                if text.contains("SSN") || text.contains("Credit card") || text.contains("Email") || text.contains("Phone") { return false; }
                true
            })
            .collect();

        if !meaningful_comments.is_empty() {
            detailed_prompt.push_str("COMMENTS TO ANALYZE:\n");
            for comment in meaningful_comments {
                detailed_prompt.push_str(&format!("Line {}: {}\n", comment.start_position.line, comment.text));
            }
        } else {
            detailed_prompt.push_str("NO MEANINGFUL COMMENTS FOUND - ANALYZING FILE STRUCTURE ONLY:\n");
        }

        detailed_prompt.push_str("\nTechnical Summary: In 2-3 direct sentences, explain what this file DOES for users/callers. List specific functions, features, or behaviors it implements. Use concrete examples (e.g., \"detects SQL injection\", \"parses JSON configs\", \"calculates hash values\"). Avoid generic phrases like \"serves as\", \"provides functionality\", \"manages\", or \"designed to\". Maximum 100 words.");

        // Debug: Log the actual prompt being sent to LLM
        println!("🔍 DEBUG PROMPT for {}:", std::path::Path::new(&extraction.file).file_name().unwrap_or_default().to_string_lossy());
        println!("=====================================");
        println!("{}", detailed_prompt);
        println!("=====================================");
        println!("📏 Prompt length: {} characters", detailed_prompt.len());

        self.processor.generate_hierarchical_summary(&detailed_prompt).await
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
        println!("🔍 🔍 🔍 CRATE DISCOVERY STARTING: {}", project_path.display());
        eprintln!("🔍 🔍 🔍 CRATE DISCOVERY STARTING: {}", project_path.display());

        // Initialize crate grouper
        let mut grouper = CrateGrouper::new(project_path.clone());

        // Discover all crates in the project
        let mut discovered_crates = grouper.discover_crates()?;

        // If no crates found, create a virtual crate
        if discovered_crates.is_empty() {
            println!("📦 No Cargo.toml found - creating virtual crate for hierarchical processing");
            let virtual_crate = CrateInfo {
                name: project_path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project")
                    .to_string(),
                path: project_path.clone(),
                description: Some("Virtual crate for project-level analysis".to_string()),
                version: "0.1.0".to_string(),
                files: Vec::new(), // Will be populated during processing
            };
            discovered_crates.push(virtual_crate);
        }

        println!("📦 Found {} crates for hierarchical analysis", discovered_crates.len());
        for crate_info in &discovered_crates {
            println!("  - {} ({})", crate_info.name, crate_info.description.as_ref().unwrap_or(&"No description".to_string()));
        }

        // Start hierarchical crate processing directly
        println!("🧠 Starting hierarchical crate processing...");

        // Clear output files from previous runs to prevent accumulation
        let files_to_clear = [
            "extracted_comments.json",
            "file_summaries.json",
            "crate_summaries.json",
            "analysis_metadata.json",
            "hierarchical_summary.md"
        ];

        // Clear from multiple possible locations since working directory can vary
        let output_dir_name = if self.insights_only { "project_scan_output_insights_only" } else { "project_scan_output" };
        let possible_output_dirs = [
            PathBuf::from(output_dir_name),
            PathBuf::from(&format!("codehud-cli/{}", output_dir_name)),
            project_path.join(output_dir_name),
            project_path.join(&format!("codehud-cli/{}", output_dir_name)),
        ];

        for output_dir in &possible_output_dirs {
            std::fs::create_dir_all(output_dir).ok(); // Create if doesn't exist, ignore errors

            for file_name in &files_to_clear {
                let file_path = output_dir.join(file_name);
                if file_path.exists() {
                    std::fs::remove_file(&file_path)?;
                    println!("🗑️ Cleared previous {} from {}", file_name, output_dir.display());
                }
            }
        }

        // Initialize progress monitor with DYNAMIC tracking
        // Initial estimate: 3 baseline steps per crate + 1 final summary step
        // We'll add file-level steps dynamically as files are discovered
        let initial_estimate = (discovered_crates.len() * 3) as u64 + 1;
        let monitor = ProgressMonitor::new();
        if let Err(e) = monitor.init(initial_estimate, "Hierarchical Project Analysis") {
            println!("⚠️  Failed to initialize progress monitor: {}", e);
        }
        println!("📊 Progress initialized with {} baseline steps (will grow as files are discovered)", initial_estimate);

        // ===== PHASE 1: Extract comments and generate file summaries for ALL crates =====
        println!("\n📝 PHASE 1: Extracting comments and generating file summaries for all crates...\n");
        let pb1 = ProgressBar::new(discovered_crates.len() as u64);
        pb1.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [Phase 1/3] [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        let mut global_step = 0u64;
        for (index, crate_info) in discovered_crates.iter().enumerate() {
            let step_message = format!("Files: {}", crate_info.name);
            pb1.set_message(step_message.clone());
            global_step += 1;
            if let Err(e) = monitor.update(global_step, &format!("Phase 1/3 - Files: {}", crate_info.name)) {
                println!("⚠️  Failed to update progress monitor: {}", e);
            }

            if let Err(e) = self.process_crate_phase1_files(&project_path, crate_info).await {
                println!("❌ Failed Phase 1 for crate {}: {}", crate_info.name, e);
                let _ = monitor.fail(&format!("Failed Phase 1 for crate {}: {}", crate_info.name, e));
                return Err(e);
            }
            pb1.inc(1);
        }
        pb1.finish_with_message("✅ Phase 1 complete: All file summaries generated");

        // ===== PHASE 2: Detect subcrates and generate subcrate summaries for ALL crates =====
        println!("\n📝 PHASE 2: Detecting subcrates and generating subcrate summaries for all crates...\n");
        let pb2 = ProgressBar::new(discovered_crates.len() as u64);
        pb2.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [Phase 2/3] [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        // Accumulate all subcrates from all crates into one combined structure
        let mut all_subcrates: std::collections::HashMap<String, Option<std::collections::HashMap<String, crate::crate_summarizer::SubcrateSummary>>> = std::collections::HashMap::new();

        for (index, crate_info) in discovered_crates.iter().enumerate() {
            let step_message = format!("Subcrates: {}", crate_info.name);
            pb2.set_message(step_message.clone());
            global_step += 1;
            if let Err(e) = monitor.update(global_step, &format!("Phase 2/3 - Subcrates: {}", crate_info.name)) {
                println!("⚠️  Failed to update progress monitor: {}", e);
            }

            // Process crate and get subcrates back instead of saving per-crate
            let subcrates = self.process_crate_phase2_subcrates_return(&project_path, crate_info).await?;
            all_subcrates.insert(crate_info.name.clone(), subcrates);

            pb2.inc(1);
        }

        // Save ONE combined subcrate file for all crates
        let output_dir = project_path.join("project_scan_output");
        let combined_file = output_dir.join("subcrate_summaries.json");
        let combined_json = serde_json::to_string_pretty(&all_subcrates)?;
        std::fs::write(&combined_file, &combined_json)?;
        let total_subcrates: usize = all_subcrates.values().filter_map(|s| s.as_ref()).map(|s| s.len()).sum();
        println!("💾 Saved combined subcrate summaries: {} subcrates across {} crates", total_subcrates, all_subcrates.len());

        pb2.finish_with_message("✅ Phase 2 complete: All subcrate summaries generated");

        // ===== PHASE 3: Generate crate summaries for ALL crates =====
        println!("\n📝 PHASE 3: Generating crate summaries for all crates...\n");
        let pb3 = ProgressBar::new(discovered_crates.len() as u64);
        pb3.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [Phase 3/3] [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        for (index, crate_info) in discovered_crates.iter().enumerate() {
            let step_message = format!("Summary: {}", crate_info.name);
            pb3.set_message(step_message.clone());
            global_step += 1;
            if let Err(e) = monitor.update(global_step, &format!("Phase 3/3 - Summary: {}", crate_info.name)) {
                println!("⚠️  Failed to update progress monitor: {}", e);
            }

            if let Err(e) = self.process_crate_phase3_summary(&project_path, crate_info).await {
                println!("❌ Failed Phase 3 for crate {}: {}", crate_info.name, e);
                let _ = monitor.fail(&format!("Failed Phase 3 for crate {}: {}", crate_info.name, e));
                return Err(e);
            }
            pb3.inc(1);
        }
        pb3.finish_with_message("✅ Phase 3 complete: All crate summaries generated");

        // Generate final hierarchical summary with single-pass synthesis
        println!("📝 Generating final hierarchical summary from crate summaries...");

        // Update progress during summary generation
        global_step = self.generate_final_summary_with_progress(&project_path, &discovered_crates, &monitor, global_step).await?;

        // Mark as completed
        let completion_message = format!("Successfully analyzed {} crates", discovered_crates.len());
        if let Err(e) = monitor.complete(&completion_message) {
            println!("⚠️  Failed to mark progress as complete: {}", e);
        }

        // Note: generate_final_summary() already sets the FSM state to ScanComplete

        Ok(())
    }


    /// Generate hierarchical project summary from crate summaries
    async fn generate_hierarchical_project_summary(&self, project_path: PathBuf, crate_summaries: Vec<CrateSummary>) -> LlmResult<()> {
        println!("🔗 Generating hierarchical project summary from {} crate summaries", crate_summaries.len());

        // Apply dual-stage denoising to crate summaries for 14K budget
        let denoised_summaries = self.apply_dual_stage_denoising(&crate_summaries).await?;

        // Build hierarchical summary prompt with 14K token budget (max for 14B model: 16K context - 2K output)
        let mut prompt = String::new();
        prompt.push_str("HIERARCHICAL PROJECT ANALYSIS (14K TOKEN BUDGET)\n");
        prompt.push_str("=============================================\n\n");
        prompt.push_str("Based on the following crate-level summaries, provide a comprehensive project analysis:\n\n");

        let mut total_files = 0;
        let mut total_tokens = 0;

        for summary in &denoised_summaries {
            total_files += summary.files_analyzed.len();
            total_tokens += summary.token_count;

            println!("🔍 DEBUG: Crate '{}' summary content (first 200 chars): {}",
                summary.crate_name,
                &summary.summary_text[..summary.summary_text.len().min(200)]);

            prompt.push_str(&format!("=== CRATE: {} ===\n", summary.crate_name.to_uppercase()));
            prompt.push_str(&format!("Files analyzed: {}\n", summary.files_analyzed.len()));
            prompt.push_str(&format!("Summary ({} tokens):\n", summary.token_count));
            prompt.push_str(&summary.summary_text);
            prompt.push_str("\n\n");
        }

        prompt.push_str("FINAL PROJECT SYNTHESIS REQUIREMENTS:\n");
        prompt.push_str("IMPORTANT: Do NOT simply list what each crate does. Instead, synthesize what this PROJECT as a unified whole does.\n\n");
        prompt.push_str("1. **Overall Architecture**: What is this project? What problem does it solve? How do the components work together as a system?\n");
        prompt.push_str("   - Describe the data flow and interactions between components\n");
        prompt.push_str("   - Identify the core pipeline or workflow\n");
        prompt.push_str("   - Explain how the different layers (CLI, core, LLM, analysis, etc.) integrate\n\n");
        prompt.push_str("2. **What Does It Actually Do**: What would a user actually use this for? What are the main features and capabilities the entire system provides?\n");
        prompt.push_str("   - Describe the primary use cases\n");
        prompt.push_str("   - Explain the key features from a user perspective\n");
        prompt.push_str("   - Detail the workflow from input to output\n\n");
        prompt.push_str("CRITICAL LANGUAGE REQUIREMENTS:\n");
        prompt.push_str("- BANNED PHRASES: 'serves as', 'provides functionality', 'manages', 'handles', 'responsible for', 'designed to', 'designed for', 'aims to', 'leverages', 'utilizes', 'stands as', 'acts as', 'facilitates', 'encompasses', 'enables'.\n");
        prompt.push_str("- Use direct, concrete language describing actual operations (e.g., 'parses X', 'detects Y', 'calculates Z', 'transforms A into B').\n");
        prompt.push_str("- Focus on specific technical details, not abstract descriptions.\n\n");
        prompt.push_str("Focus on the unified purpose and capabilities, not individual crate descriptions. Provide a comprehensive analysis of 600-1000 words.\n\n");

        // Enforce token budget to fit within 16K context (16K - 1K output buffer = 15K max input)
        let final_prompt = self.enforce_token_budget(&prompt, 15000).await?;
        println!("📊 Final hierarchical prompt: {} characters (~{} tokens)", final_prompt.len(), final_prompt.len() / 4);

        // Generate final summary via LLM with extended output tokens (1024 for 600-1000 words)
        let summary_text = self.processor.generate_hierarchical_summary(&final_prompt).await?;

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
        let output_dir = project_path.join("project_scan_output");
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
                    crate_summaries.len(), total_files),
                errors: vec![],
            }
        };

        Ok(())
    }

    /// Get project analysis memory for context-aware processing
    async fn get_project_analysis_memory(&self) -> crate::conversation::ProjectAnalysisMemory {
        self.project_memory.read().await.clone()
    }

    /// Apply intelligent denoising that preserves technical content
    async fn apply_dual_stage_denoising(&self, crate_summaries: &[CrateSummary]) -> LlmResult<Vec<CrateSummary>> {
        println!("🧠 Applying intelligent denoising to {} crate summaries", crate_summaries.len());

        // Skip denoising for single crate scenarios to preserve content
        if crate_summaries.len() == 1 {
            println!("⚠️ Single crate detected - skipping denoising to preserve content");
            return Ok(crate_summaries.to_vec());
        }

        let mut denoised_summaries = Vec::new();

        for summary in crate_summaries {
            // Apply single-stage intelligent denoising (40% reduction, keeping 60%)
            let denoised_text = self.apply_intelligent_denoising(&summary.summary_text).await?;

            let mut denoised_summary = summary.clone();
            denoised_summary.summary_text = denoised_text;
            denoised_summary.token_count = denoised_summary.summary_text.len() / 4; // Rough token estimate

            denoised_summaries.push(denoised_summary);
        }

        println!("✅ Intelligent denoising complete: reduced from {} to {} total tokens",
            crate_summaries.iter().map(|s| s.token_count).sum::<usize>(),
            denoised_summaries.iter().map(|s| s.token_count).sum::<usize>()
        );

        Ok(denoised_summaries)
    }

    /// Apply intelligent denoising that preserves technical content (40% reduction)
    async fn apply_intelligent_denoising(&self, text: &str) -> LlmResult<String> {
        println!("🔍 DEBUG: Denoising input: {} chars, {} words", text.len(), text.split_whitespace().count());

        if text.len() < 200 {
            // Don't denoise very short summaries
            println!("🔍 DEBUG: Skipping denoising - text too short");
            return Ok(text.to_string());
        }

        // Split into sentences and rank by importance
        let sentences: Vec<&str> = text.split('.').filter(|s| !s.trim().is_empty()).collect();
        println!("🔍 DEBUG: Found {} sentences", sentences.len());

        if sentences.len() <= 2 {
            // Keep very short summaries intact
            println!("🔍 DEBUG: Skipping denoising - too few sentences");
            return Ok(text.to_string());
        }

        let mut scored_sentences: Vec<(usize, &str, f64)> = sentences.iter().enumerate()
            .map(|(i, sentence)| {
                let importance_score = self.calculate_sentence_importance(sentence);
                println!("🔍 DEBUG: Sentence {}: score={:.1} - {}", i, importance_score, &sentence[..sentence.len().min(80)]);
                (i, *sentence, importance_score)
            })
            .collect();

        // Sort by importance score (descending)
        scored_sentences.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Keep top 60% of sentences (40% reduction)
        let keep_count = (sentences.len() as f64 * 0.6).ceil() as usize;
        println!("🔍 DEBUG: Keeping {} out of {} sentences", keep_count, sentences.len());

        let mut kept_sentences: Vec<(usize, &str)> = scored_sentences.into_iter()
            .take(keep_count)
            .map(|(i, s, _)| (i, s))
            .collect();

        // Sort by original order to maintain flow
        kept_sentences.sort_by(|a, b| a.0.cmp(&b.0));

        let denoised = kept_sentences.into_iter()
            .map(|(_, s)| s.trim())
            .collect::<Vec<_>>()
            .join(". ")
            + ".";

        Ok(denoised)
    }

    /// Calculate importance score for a sentence (higher = more important)
    fn calculate_sentence_importance(&self, sentence: &str) -> f64 {
        let text = sentence.to_lowercase();
        let mut score = 0.0;

        // Technical keywords boost
        let technical_keywords = [
            "provides", "implements", "manages", "serves", "handles", "functionality",
            "module", "crate", "system", "engine", "pipeline", "interface", "bridge",
            "llm", "model", "gpu", "acceleration", "constraint", "validation", "analysis",
            "processing", "generation", "detection", "monitoring", "tracking", "configuration"
        ];

        for keyword in &technical_keywords {
            if text.contains(keyword) {
                score += 2.0;
            }
        }

        // Function/purpose indicators
        let purpose_indicators = [
            "purpose", "function", "role", "responsibility", "designed", "used for",
            "enables", "allows", "facilitates", "supports", "includes", "features"
        ];

        for indicator in &purpose_indicators {
            if text.contains(indicator) {
                score += 1.5;
            }
        }

        // Penalize filler content
        let filler_phrases = [
            "however", "additionally", "furthermore", "moreover", "in conclusion",
            "as mentioned", "it should be noted", "it is important", "please note"
        ];

        for filler in &filler_phrases {
            if text.contains(filler) {
                score -= 1.0;
            }
        }

        // Boost sentences with concrete details
        if text.contains("ffi") || text.contains("rust") || text.contains("python") {
            score += 1.0;
        }

        // Length consideration (medium-length sentences often more informative)
        let word_count = text.split_whitespace().count();
        if word_count >= 8 && word_count <= 25 {
            score += 0.5;
        } else if word_count < 4 || word_count > 40 {
            score -= 0.5;
        }

        score
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

    /// PHASE 1: Extract comments and generate file summaries for a single crate
    async fn process_crate_phase1_files(&self, project_path: &PathBuf, crate_info: &CrateInfo) -> LlmResult<()> {
        println!("🔍 DEBUG: Crate path for {}: {}", crate_info.name, crate_info.path.display());

        // Only discover files within this specific crate directory
        let file_extensions = vec!["rs".to_string()].into_iter().collect();
        let crate_files = CommentExtractionFSM::discover_project_files_static(&crate_info.path, &file_extensions).await?;
        let mut crate_extractions = Vec::new();

        println!("🔍 Processing {} files from crate {}", crate_files.len(), crate_info.name);
        println!("🔍 DEBUG: First 5 files discovered for crate {}:", crate_info.name);
        for (i, file) in crate_files.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, file.display());
        }

        // Extract comments only from files in this crate
        for file_path in &crate_files {
            match self.extractor.extract_from_file(file_path) {
                Ok(extraction) => {
                    if !extraction.comments.is_empty() {
                        crate_extractions.push(extraction);
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to extract from {}: {}", file_path.display(), e);
                }
            }
        }

        if crate_extractions.is_empty() {
            println!("⚠️ No files with comments found for crate: {}", crate_info.name);
            return Ok(());
        }

        // ===== BATCH NARRATOR PROCESSING =====
        println!("🧠 Running narrator on {} files in batch...", crate_extractions.len());
        self.extractor.add_narrator_insights_batch(&mut crate_extractions, &crate_files)?;
        println!("✅ Added structural insights to all files");

        // ===== SAVE STRUCTURED JSON DATA =====
        let output_dir = if self.insights_only {
            project_path.join("project_scan_output_insights_only")
        } else {
            project_path.join("project_scan_output")
        };
        std::fs::create_dir_all(&output_dir)?;

        // Append extracted comments to cumulative JSON file
        let extractions_file = output_dir.join("extracted_comments.json");
        let mut all_extractions = if extractions_file.exists() {
            let existing_content = std::fs::read_to_string(&extractions_file)?;
            serde_json::from_str::<Vec<FileCommentExtraction>>(&existing_content).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        // Add current crate's extractions
        all_extractions.extend(crate_extractions.clone());

        // Save updated cumulative file
        let extractions_json = serde_json::to_string_pretty(&all_extractions)?;
        std::fs::write(&extractions_file, &extractions_json)?;
        println!("💾 Comments JSON updated: {} total files ({} from {})", all_extractions.len(), crate_extractions.len(), crate_info.name);

        // ===========================================
        // PHASE 2: BATCH LLM ANALYSIS WITH ENHANCED CONTEXT
        // ===========================================
        println!("🤖 Phase 2: LLM analysis with batch processing for crate {}...", crate_info.name);
        let file_summaries = if self.insights_only {
            // Use ultra token-efficient insights-only file analysis
            self.analyze_extractions_insights_only(&crate_extractions).await?
        } else {
            // Use full enhanced context analysis
            self.analyze_extractions_with_enhanced_context(&crate_extractions).await?
        };

        // Append file summaries to cumulative JSON file
        let summaries_file = output_dir.join("file_summaries.json");
        let mut all_summaries = if summaries_file.exists() {
            let existing_content = std::fs::read_to_string(&summaries_file)?;
            serde_json::from_str::<Vec<(String, String)>>(&existing_content).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        // Add current crate's summaries
        all_summaries.extend(file_summaries.clone());

        // Save updated cumulative file
        let summaries_json = serde_json::to_string_pretty(&all_summaries)?;
        std::fs::write(&summaries_file, &summaries_json)?;
        println!("📄 File summaries updated: {} total files ({} from {})", all_summaries.len(), file_summaries.len(), crate_info.name);

        // Update cumulative analysis metadata
        let metadata_file = output_dir.join("analysis_metadata.json");
        let mut cumulative_metadata = if metadata_file.exists() {
            let existing_content = std::fs::read_to_string(&metadata_file)?;
            serde_json::from_str::<serde_json::Value>(&existing_content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({
                "analysis_method": "hierarchical-crate-analysis",
                "crates_processed": [],
                "total_files_processed": 0,
                "total_files_with_comments": 0,
                "started_timestamp": chrono::Utc::now().to_rfc3339()
            })
        };

        // Add current crate info
        let crate_metadata = serde_json::json!({
            "name": crate_info.name,
            "files_processed": crate_extractions.len(),
            "files_with_comments": file_summaries.len(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        cumulative_metadata["crates_processed"].as_array_mut().unwrap().push(crate_metadata);
        cumulative_metadata["total_files_processed"] = serde_json::json!(
            cumulative_metadata["total_files_processed"].as_u64().unwrap_or(0) + crate_extractions.len() as u64
        );
        cumulative_metadata["total_files_with_comments"] = serde_json::json!(
            cumulative_metadata["total_files_with_comments"].as_u64().unwrap_or(0) + file_summaries.len() as u64
        );
        cumulative_metadata["last_updated"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

        std::fs::write(&metadata_file, serde_json::to_string_pretty(&cumulative_metadata)?)?;
        println!("📄 Analysis metadata updated: {} crates processed", cumulative_metadata["crates_processed"].as_array().unwrap().len());

        Ok(())
    }

    /// PHASE 2: Detect subcrates and generate subcrate summaries for a single crate (returns subcrates)
    async fn process_crate_phase2_subcrates_return(&self, project_path: &PathBuf, crate_info: &CrateInfo) -> LlmResult<Option<std::collections::HashMap<String, crate::crate_summarizer::SubcrateSummary>>> {
        // Skip if insights-only mode
        if self.insights_only {
            println!("⏭️  Skipping subcrate detection for {} (insights-only mode)", crate_info.name);
            return Ok(None);
        }

        println!("🔍 Detecting subcrates for crate: {}", crate_info.name);

        // Initialize crate summarizer
        let config = CrateSummarizerConfig::default();
        let mut summarizer = CrateSummarizer::new(self.processor.clone(), config, project_path.clone());

        // Load file summaries for this crate
        let output_dir = project_path.join("project_scan_output");

        // Need to load comment extractions to pass to load_file_summaries_for_crate
        let extractions_file = output_dir.join("extracted_comments.json");
        if !extractions_file.exists() {
            println!("⚠️  No extracted comments file found for crate: {}", crate_info.name);
            return Ok(None);
        }

        let all_extractions: Vec<FileCommentExtraction> = serde_json::from_str(
            &std::fs::read_to_string(&extractions_file)?
        )?;

        // Filter to just this crate's extractions (canonicalize both paths for comparison)
        let canonical_crate_path = crate_info.path.canonicalize()
            .unwrap_or_else(|_| crate_info.path.clone())
            .to_string_lossy()
            .to_string();

        let crate_extractions: Vec<FileCommentExtraction> = all_extractions.into_iter()
            .filter(|e| {
                // Convert file path to absolute by canonicalizing relative to project root
                let file_path = PathBuf::from(&e.file);
                let canonical_file = if file_path.is_relative() {
                    project_path.join(&file_path).canonicalize().ok()
                } else {
                    file_path.canonicalize().ok()
                };

                if let Some(abs_file) = canonical_file {
                    abs_file.to_string_lossy().starts_with(&canonical_crate_path)
                } else {
                    false
                }
            })
            .collect();

        println!("🔍 DEBUG: Filtered {} files for crate {} (canonical path: {})",
            crate_extractions.len(), crate_info.name, canonical_crate_path);

        let cleaned_files = summarizer.load_file_summaries_for_crate(&crate_extractions, &output_dir)?;

        // Generate subcrate summaries (bottom-up hierarchical summarization)
        let subcrate_summaries = summarizer.generate_subcrate_summaries(
            crate_info,
            &cleaned_files
        ).await?;

        // Return subcrates for accumulation
        if let Some(ref summaries) = subcrate_summaries {
            println!("✅ Generated {} subcrate summaries for crate: {}", summaries.len(), crate_info.name);
        }

        println!("✅ Phase 2 complete for crate: {}", crate_info.name);
        Ok(subcrate_summaries)
    }

    /// PHASE 2: Detect subcrates and generate subcrate summaries for a single crate (OLD - saves to disk)
    async fn process_crate_phase2_subcrates(&self, project_path: &PathBuf, crate_info: &CrateInfo) -> LlmResult<()> {
        // Skip if insights-only mode
        if self.insights_only {
            println!("⏭️  Skipping subcrate detection for {} (insights-only mode)", crate_info.name);
            return Ok(());
        }

        println!("🔍 Detecting subcrates for crate: {}", crate_info.name);

        // Initialize crate summarizer
        let config = CrateSummarizerConfig::default();
        let mut summarizer = CrateSummarizer::new(self.processor.clone(), config, project_path.clone());

        // Load file summaries for this crate
        let output_dir = project_path.join("project_scan_output");

        // Need to load comment extractions to pass to load_file_summaries_for_crate
        let extractions_file = output_dir.join("extracted_comments.json");
        if !extractions_file.exists() {
            println!("⚠️  No extracted comments file found for crate: {}", crate_info.name);
            return Ok(());
        }

        let all_extractions: Vec<FileCommentExtraction> = serde_json::from_str(
            &std::fs::read_to_string(&extractions_file)?
        )?;

        // Filter to just this crate's extractions (canonicalize both paths for comparison)
        let canonical_crate_path = crate_info.path.canonicalize()
            .unwrap_or_else(|_| crate_info.path.clone())
            .to_string_lossy()
            .to_string();

        let crate_extractions: Vec<FileCommentExtraction> = all_extractions.into_iter()
            .filter(|e| {
                // Convert file path to absolute by canonicalizing relative to project root
                let file_path = PathBuf::from(&e.file);
                let canonical_file = if file_path.is_relative() {
                    project_path.join(&file_path).canonicalize().ok()
                } else {
                    file_path.canonicalize().ok()
                };

                if let Some(abs_file) = canonical_file {
                    abs_file.to_string_lossy().starts_with(&canonical_crate_path)
                } else {
                    false
                }
            })
            .collect();

        println!("🔍 DEBUG: Filtered {} files for crate {} (canonical path: {})",
            crate_extractions.len(), crate_info.name, canonical_crate_path);

        let cleaned_files = summarizer.load_file_summaries_for_crate(&crate_extractions, &output_dir)?;

        // Generate subcrate summaries (bottom-up hierarchical summarization)
        let subcrate_summaries = summarizer.generate_subcrate_summaries(
            crate_info,
            &cleaned_files
        ).await?;

        // Save subcrate summaries to a JSON file for Phase 3 to load
        let subcrate_count = subcrate_summaries.as_ref().map(|s| s.len()).unwrap_or(0);
        if subcrate_count > 0 {
            let subcrate_file = output_dir.join(format!("subcrate_summaries_{}.json", crate_info.name));
            let subcrate_json = serde_json::to_string_pretty(&subcrate_summaries)?;
            std::fs::write(&subcrate_file, &subcrate_json)?;
            println!("💾 Saved {} subcrate summaries for crate: {}", subcrate_count, crate_info.name);
        }

        println!("✅ Phase 2 complete for crate: {}", crate_info.name);
        Ok(())
    }

    /// PHASE 3: Generate crate summary for a single crate
    async fn process_crate_phase3_summary(&self, project_path: &PathBuf, crate_info: &CrateInfo) -> LlmResult<()> {
        // Initialize crate summarizer
        let config = CrateSummarizerConfig::default();
        let mut summarizer = CrateSummarizer::new(self.processor.clone(), config, project_path.clone());

        // Get project memory for context-aware analysis
        let project_memory = self.get_project_analysis_memory().await;

        // Determine output directory
        let output_dir = if self.insights_only {
            project_path.join("project_scan_output_insights_only")
        } else {
            project_path.join("project_scan_output")
        };

        // Load comment extractions
        let extractions_file = output_dir.join("extracted_comments.json");
        if !extractions_file.exists() {
            println!("⚠️  No extracted comments file found for crate: {}", crate_info.name);
            return Ok(());
        }

        let all_extractions: Vec<FileCommentExtraction> = serde_json::from_str(
            &std::fs::read_to_string(&extractions_file)?
        )?;

        // Filter to just this crate's extractions (canonicalize both paths for comparison)
        let canonical_crate_path = crate_info.path.canonicalize()
            .unwrap_or_else(|_| crate_info.path.clone())
            .to_string_lossy()
            .to_string();

        let crate_extractions: Vec<FileCommentExtraction> = all_extractions.into_iter()
            .filter(|e| {
                // Convert file path to absolute by canonicalizing relative to project root
                let file_path = PathBuf::from(&e.file);
                let canonical_file = if file_path.is_relative() {
                    project_path.join(&file_path).canonicalize().ok()
                } else {
                    file_path.canonicalize().ok()
                };

                if let Some(abs_file) = canonical_file {
                    abs_file.to_string_lossy().starts_with(&canonical_crate_path)
                } else {
                    false
                }
            })
            .collect();

        println!("🔍 DEBUG: Filtered {} files for crate {} (canonical path: {})",
            crate_extractions.len(), crate_info.name, canonical_crate_path);

        // Generate crate summary using insights-only or full method based on configuration
        let crate_summary = if self.insights_only {
            // Use ultra token-efficient insights-only analysis
            summarizer.generate_structural_insights_only_summary(
                crate_info,
                &crate_extractions,
            ).await?
        } else {
            // Load file summaries for crate summary generation
            let _cleaned_files = summarizer.load_file_summaries_for_crate(&crate_extractions, &output_dir)?;

            // Load pre-existing subcrate summaries from combined file (already generated in Phase 2)
            let subcrate_file = output_dir.join("subcrate_summaries.json");
            let subcrate_summaries = if subcrate_file.exists() {
                let subcrate_json = std::fs::read_to_string(&subcrate_file)?;
                let all_subcrates: std::collections::HashMap<String, Option<std::collections::HashMap<String, crate::crate_summarizer::SubcrateSummary>>> =
                    serde_json::from_str(&subcrate_json)?;
                // Extract this crate's subcrates from the combined file
                let crate_subcrates = all_subcrates.get(&crate_info.name).cloned().flatten();
                if let Some(ref summaries) = crate_subcrates {
                    println!("📂 Loaded {} subcrate summaries for crate: {}", summaries.len(), crate_info.name);
                } else {
                    println!("⚠️  No subcrate summaries found in combined file for crate: {}", crate_info.name);
                }
                crate_subcrates
            } else {
                println!("⚠️  No combined subcrate summaries file found");
                None
            };

            // Use full context-aware analysis with subcrate summaries
            summarizer.generate_crate_summary_with_context(
                crate_info,
                &crate_extractions,
                &project_memory,
                &output_dir,
                subcrate_summaries
            ).await?
        };

        println!("✅ Generated {} summary for crate '{}': {} tokens",
            if self.insights_only { "insights-only" } else { "context-aware" },
            crate_info.name, crate_summary.token_count);

        // Save crate summary to cumulative JSON file
        let output_dir = if self.insights_only {
            project_path.join("project_scan_output_insights_only")
        } else {
            project_path.join("project_scan_output")
        };
        std::fs::create_dir_all(&output_dir)?;
        let crate_summaries_file = output_dir.join("crate_summaries.json");
        let mut all_crate_summaries = if crate_summaries_file.exists() {
            let existing_content = std::fs::read_to_string(&crate_summaries_file)?;
            serde_json::from_str::<Vec<CrateSummary>>(&existing_content).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        // Add current crate summary
        all_crate_summaries.push(crate_summary.clone());

        // Save updated crate summaries
        let crate_summaries_json = serde_json::to_string_pretty(&all_crate_summaries)?;
        std::fs::write(&crate_summaries_file, &crate_summaries_json)?;
        println!("📄 Crate summaries updated: {} total crates", all_crate_summaries.len());

        // Update project memory with this crate's insights
        {
            let mut memory = self.project_memory.write().await;
            memory.accumulate_crate_insights(&crate_summary);
        }

        Ok(())
    }

    /// Helper to generate summary using Gemini if API key is present, otherwise use local LLM
    async fn generate_summary(&self, prompt: &str) -> LlmResult<String> {
        if let Some(ref api_key) = self.gemini_api_key {
            // Use Gemini Flash API
            let client = crate::gemini::GeminiClient::new(api_key.clone());
            client.generate(prompt).await
        } else {
            // Use local 14B model
            self.processor.generate_project_hierarchical_summary(prompt).await
        }
    }

    /// Generate final hierarchical summary from all processed crates
    async fn generate_final_summary_with_progress(&self, project_path: &PathBuf, discovered_crates: &[CrateInfo], monitor: &ProgressMonitor, mut global_step: u64) -> LlmResult<u64> {
        println!("📝 Synthesizing project summary (1500 token output, 16K context)...");

        // Load real crate summaries from the generated file (use correct directory based on insights_only mode)
        let output_dir = if self.insights_only {
            project_path.join("project_scan_output_insights_only")
        } else {
            project_path.join("project_scan_output")
        };
        let crate_summaries_file = output_dir.join("crate_summaries.json");

        let crate_summaries = if crate_summaries_file.exists() {
            let content = std::fs::read_to_string(&crate_summaries_file)?;
            serde_json::from_str::<Vec<CrateSummary>>(&content)
                .map_err(|e| LlmError::Config(format!("Failed to parse crate summaries: {}", e)))?
        } else {
            return Err(LlmError::Config("No crate summaries found - ensure crates were processed first".to_string()));
        };

        println!("📊 Loaded {} real crate summaries for hierarchical analysis", crate_summaries.len());

        // Build crate summaries string once (reused across passes) - using FULL summaries
        let mut crate_summaries_text = String::new();
        let mut total_files = 0;
        let mut total_tokens = 0;

        for summary in &crate_summaries {
            total_files += summary.files_analyzed.len();
            total_tokens += summary.token_count;

            crate_summaries_text.push_str(&format!("=== CRATE: {} ===\n", summary.crate_name.to_uppercase()));
            crate_summaries_text.push_str(&format!("Summary ({} tokens):\n", summary.token_count));
            crate_summaries_text.push_str(&summary.summary_text);
            crate_summaries_text.push_str("\n\n");
        }

        // Check if using Gemini - if so, skip multi-pass and use direct prompt
        let summary_text = if self.gemini_api_key.is_some() {
            println!("🌟 Using Gemini - generating direct summary from crate summaries (skipping multi-pass)...");

            global_step += 1;
            let gemini_msg = "Generating final summary with Gemini";
            println!("📝 {}...", gemini_msg);
            if let Err(e) = monitor.update(global_step, gemini_msg) {
                println!("⚠️  Failed to update progress: {}", e);
            }

            // Create direct prompt for Gemini with all crate summaries
            let mut gemini_prompt = String::new();
            gemini_prompt.push_str("Generate a comprehensive technical project summary with TWO sections based on the crate summaries below:\n\n");
            gemini_prompt.push_str("## Overall Architecture\n");
            gemini_prompt.push_str("[Describe the system design, architectural patterns, and how components work together. ");
            gemini_prompt.push_str("List SPECIFIC external libraries/frameworks BY NAME. Explain the layered architecture.]\n\n");
            gemini_prompt.push_str("## What Does It Actually Do\n");
            gemini_prompt.push_str("[PRIMARY PURPOSE: Start with the MAIN problem this project solves and its CORE functionality. ");
            gemini_prompt.push_str("State the #1 most important feature FIRST. Then describe user workflows, use cases, and secondary features. ");
            gemini_prompt.push_str("What problem does this solve? How do users interact with it?]\n\n");
            gemini_prompt.push_str("CRITICAL: Identify and state the PRIMARY purpose first - the main reason this project exists.\n");
            gemini_prompt.push_str("Use concrete details from the summaries. Maximum 500 words total.\n\n");
            gemini_prompt.push_str("=============================================\n\n");
            gemini_prompt.push_str("CRATE SUMMARIES:\n\n");
            gemini_prompt.push_str(&crate_summaries_text);

            let result = self.generate_summary(&gemini_prompt).await?;
            println!("✅ Gemini summary complete");
            result
        } else {
            // SINGLE-PASS DIRECT SUMMARY (for local model)
            println!("🚀 Generating single-pass summary from crate summaries...");

            global_step += 1;
            let single_msg = "Generating final summary";
            println!("📝 {}...", single_msg);
            if let Err(e) = monitor.update(global_step, single_msg) {
                println!("⚠️  Failed to update progress: {}", e);
            }

            // Create direct prompt with all crate summaries
            let mut single_prompt = String::new();
            single_prompt.push_str("Based on the following crate-level summaries, provide a comprehensive project analysis:\n\n");
            single_prompt.push_str(&crate_summaries_text);
            single_prompt.push_str("\nGenerate a comprehensive technical project summary with TWO sections:\n\n");
            single_prompt.push_str("## Overall Architecture\n");
            single_prompt.push_str("[Describe the system design, architectural patterns, and how components work together as a unified system. ");
            single_prompt.push_str("List SPECIFIC external libraries/frameworks BY NAME. ");
            single_prompt.push_str("Explain the layered architecture and data flow between components.]\n\n");
            single_prompt.push_str("## What Does It Actually Do\n");
            single_prompt.push_str("[PRIMARY PURPOSE: Start with the MAIN problem this project solves and its CORE user-facing functionality. ");
            single_prompt.push_str("What is the #1 thing users can DO with this tool? What OUTPUT does it produce? ");
            single_prompt.push_str("State the most important capability FIRST, then describe user workflows, use cases, and secondary features. ");
            single_prompt.push_str("What problem does this solve? How do users interact with it?]\n\n");
            single_prompt.push_str("CRITICAL INSTRUCTIONS:\n");
            single_prompt.push_str("- Identify the PRIMARY purpose first - the main reason this project exists and what users actually DO with it\n");
            single_prompt.push_str("- Focus on USER-FACING capabilities, not just internal operations\n");
            single_prompt.push_str("- If this generates summaries/reports/analysis, STATE THAT PROMINENTLY\n");
            single_prompt.push_str("- Use concrete details from the summaries\n");
            single_prompt.push_str("- Maximum 500 words total\n\n");
            single_prompt.push_str("Focus on the unified purpose and user-facing capabilities, not individual crate descriptions.\n");

            let summary_text = self.generate_summary(&single_prompt).await?;
            println!("✅ Single-pass summary complete!");

            summary_text
        }; // End of if/else for Gemini vs multi-pass

        // Save the hierarchical summary to output (use correct directory based on insights_only mode)
        let save_output_dir = if self.insights_only {
            project_path.join("project_scan_output_insights_only")
        } else {
            project_path.join("project_scan_output")
        };
        std::fs::create_dir_all(&save_output_dir)?;
        let summary_file = save_output_dir.join("hierarchical_summary.md");
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
                    crate_summaries.len(), total_files),
                errors: vec![],
            }
        };

        // Return the updated step count
        Ok(global_step)
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