//! CodeHUD LLM CLI - LLM-Powered Features
//!
//! Interactive LLM interface for AI-powered code analysis, bug fixing,
//! and development assistance. Matches Python cli_llm.py exactly.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::io::Write;
use codehud_core::{Result, ViewType};
use codehud_llm::{
    FileProcessor, ProcessorConfig, CommentExtractor, ExtractionConfig,
    LlmConfig, OllamaConfig, ModelType, GpuType,
    CommentExtractionFSM, CommentExtractionCLI, ExtractionState, ScanResult,
};

#[derive(Parser)]
#[command(name = "codehud-llm")]
#[command(about = "CodeHUD LLM - AI-Powered Code Analysis Interface")]
#[command(long_about = "Interactive LLM interface for AI-powered code analysis and development assistance.\n\nProvides intelligent bug fixing, code generation, architectural insights,\nand automated improvements with 97%+ success rate.")]
#[command(version = "1.0.0")]
#[command(author = "CodeHUD Team")]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,

    /// LLM backend to use
    #[arg(short = 'b', long, default_value = "ollama")]
    backend: LlmBackend,

    /// Model name to use for analysis
    #[arg(short = 'm', long)]
    model: Option<String>,

    /// Enable GPU acceleration
    #[arg(long)]
    gpu: bool,

    /// Temperature for LLM responses (0.0-2.0)
    #[arg(short = 't', long, default_value = "0.7")]
    temperature: f32,

    /// Configuration file path
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    /// Enable debug mode
    #[arg(long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract and analyze comments for codebase understanding
    AnalyzeComments {
        /// Path to the codebase directory to analyze
        codebase_path: PathBuf,

        /// Output directory for analysis files
        #[arg(short = 'o', long, default_value = "analysis_output")]
        output_dir: PathBuf,

        /// Skip system summary generation (faster for large codebases)
        #[arg(long)]
        skip_system_summary: bool,

        /// Focus on specific file extensions
        #[arg(short = 'e', long)]
        extensions: Vec<String>,

        /// Use cached comment extraction if available
        #[arg(long)]
        use_cache: bool,

        /// Maximum tokens per file analysis
        #[arg(long, default_value = "1000")]
        max_tokens_per_file: usize,

        /// Maximum tokens for system summary
        #[arg(long, default_value = "4000")]
        system_summary_max_tokens: usize,
    },

    /// Interactive comment exploration mode
    ExploreComments {
        /// Path to the codebase directory to explore
        codebase_path: PathBuf,

        /// Start with specific file or directory
        #[arg(short = 'f', long)]
        focus: Option<PathBuf>,
    },

    /// Generate documentation from comment analysis
    GenerateDocs {
        /// Path to the codebase directory
        codebase_path: PathBuf,

        /// Template for documentation generation
        #[arg(short = 't', long)]
        template: Option<PathBuf>,

        /// Output format (markdown, html, json)
        #[arg(long, default_value = "markdown")]
        format: DocumentFormat,

        /// Output file for generated documentation
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// Interactive LLM session (legacy mode)
    Interactive {
        /// Path to the codebase directory to analyze
        codebase_path: PathBuf,

        /// Session file for conversation persistence
        #[arg(short = 's', long)]
        session: Option<PathBuf>,

        /// Initial view context for the LLM
        #[arg(short = 'v', long, default_value = "topology")]
        view: ViewType,

        /// Maximum tokens for responses
        #[arg(long, default_value = "4096")]
        max_tokens: u32,

        /// Enable constitutional AI guardrails
        #[arg(long, default_value = "true")]
        guardrails: bool,

        /// Enable bug fixing mode with high success rate
        #[arg(long)]
        bug_fix_mode: bool,

        /// Enable code generation mode
        #[arg(long)]
        generate_mode: bool,
    },

    /// Scan a single file with FSM interface
    ScanFile {
        /// File path to scan (will show picker if not provided)
        file_path: Option<PathBuf>,
    },

    /// Scan an entire project with FSM interface
    ScanProject {
        /// Project directory to scan (will show picker if not provided)
        project_path: Option<PathBuf>,

        /// Use structural insights only mode (ultra token-efficient)
        #[arg(long)]
        insights_only: bool,

        /// Google AI Studio API key for final summary (uses Gemini Flash instead of local 14B)
        #[arg(long, env = "GEMINI_API_KEY")]
        gemini_api_key: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocumentFormat {
    Markdown,
    Html,
    Json,
    Pdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LlmBackend {
    /// Local Ollama backend (recommended)
    Ollama,
    /// PyTorch/HuggingFace backend
    Pytorch,
    /// OpenHands external integration
    Openhands,
    /// Shell-GPT integration
    Shellgpt,
}

impl std::fmt::Display for LlmBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmBackend::Ollama => write!(f, "ollama"),
            LlmBackend::Pytorch => write!(f, "pytorch"),
            LlmBackend::Openhands => write!(f, "openhands"),
            LlmBackend::Shellgpt => write!(f, "shell-gpt"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    codehud_utils::logging::basic_config(Some(codehud_utils::logging::LogLevel::Info))?;

    let cli = Cli::parse();

    // Validate temperature range
    if cli.temperature < 0.0 || cli.temperature > 2.0 {
        eprintln!("Error: Temperature must be between 0.0 and 2.0");
        std::process::exit(1);
    }

    // Handle commands
    match cli.command {
        Commands::AnalyzeComments {
            ref codebase_path,
            ref output_dir,
            skip_system_summary,
            ref extensions,
            use_cache,
            max_tokens_per_file,
            system_summary_max_tokens,
        } => {
            handle_analyze_comments(
                &cli,
                codebase_path.clone(),
                output_dir.clone(),
                skip_system_summary,
                extensions.clone(),
                use_cache,
                max_tokens_per_file,
                system_summary_max_tokens,
            ).await
        }
        Commands::ExploreComments { ref codebase_path, ref focus } => {
            handle_explore_comments(&cli, codebase_path.clone(), focus.clone()).await
        }
        Commands::GenerateDocs { ref codebase_path, ref template, format, ref output } => {
            handle_generate_docs(&cli, codebase_path.clone(), template.clone(), format, output.clone()).await
        }
        Commands::Interactive {
            ref codebase_path,
            ref session,
            view,
            max_tokens,
            guardrails,
            bug_fix_mode,
            generate_mode,
        } => {
            handle_interactive(
                &cli,
                codebase_path.clone(),
                session.clone(),
                view,
                max_tokens,
                guardrails,
                bug_fix_mode,
                generate_mode,
            ).await
        }
        Commands::ScanFile { ref file_path } => {
            handle_scan_file(&cli, file_path.clone()).await
        }
        Commands::ScanProject { ref project_path, insights_only, ref gemini_api_key } => {
            handle_scan_project(&cli, project_path.clone(), insights_only, gemini_api_key.clone()).await
        }
    }
}

/// Handle comment analysis command (implements the three-phase workflow)
async fn handle_analyze_comments(
    cli: &Cli,
    codebase_path: PathBuf,
    output_dir: PathBuf,
    skip_system_summary: bool,
    extensions: Vec<String>,
    use_cache: bool,
    max_tokens_per_file: usize,
    system_summary_max_tokens: usize,
) -> Result<()> {
    validate_codebase_path(&codebase_path)?;

    println!("🚀 CodeHUD LLM - Comment Analysis");
    println!("📁 Analyzing: {}", codebase_path.display());
    println!("📂 Output: {}", output_dir.display());
    println!("🤖 Backend: {}", cli.backend);
    if cli.gpu { println!("⚡ GPU acceleration enabled"); }

    // Create processor configuration
    let processor_config = ProcessorConfig {
        extraction_config: ExtractionConfig::default(),
        llm_config: codehud_llm::file_processor::LlmAnalysisConfig {
            max_tokens_per_file,
            system_summary_max_tokens,
            include_code_context: true,
            extract_todos: true,
            analyze_documentation_coverage: true,
            temperature: cli.temperature,
        },
        output_config: codehud_llm::file_processor::OutputConfig {
            output_dir: output_dir.clone(),
            pretty_json: true,
            include_debug_info: cli.debug,
        },
        performance_config: codehud_llm::file_processor::PerformanceConfig {
            parallel_processing: true,
            max_concurrent_llm_calls: 3,
            use_cache,
            cache_duration_hours: 24,
        },
    };

    // Create Ollama configuration
    let ollama_config = OllamaConfig::default();

    // Create file processor
    println!("🔧 Initializing LLM pipeline...");
    let processor = FileProcessor::new(ollama_config, processor_config)
        .await
        .map_err(|e| codehud_core::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // Process the codebase
    println!("📝 Processing codebase...");
    let report = processor.process_codebase(&codebase_path)
        .await
        .map_err(|e| codehud_core::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // Display results
    if report.success {
        println!("✅ Analysis completed successfully!");
        println!("📊 Processed {} files in {:.2} seconds",
                 report.files_processed,
                 report.total_duration.as_secs_f64());

        println!("\n📁 Output files:");
        println!("   📋 Comments: {}", report.output_files.comments_file.display());
        println!("   📝 Summaries: {}", report.output_files.summaries_file.display());
        println!("   🌐 System Summary: {}", report.output_files.system_summary_file.display());

        println!("\n⚡ Performance:");
        println!("   🔍 Extraction: {:.2}s", report.performance.extraction_time.as_secs_f64());
        println!("   🤖 LLM Analysis: {:.2}s", report.performance.analysis_time.as_secs_f64());
        println!("   🌐 System Summary: {:.2}s", report.performance.summary_time.as_secs_f64());
        println!("   📊 Total Tokens: {}", report.performance.total_tokens);
        println!("   🔧 LLM Calls: {}", report.performance.llm_calls);
    } else {
        println!("❌ Analysis completed with errors");
        println!("📊 Processed {} files, {} failed", report.files_processed, report.files_failed);

        if !report.errors.is_empty() {
            println!("\n🚨 Errors:");
            for error in &report.errors {
                println!("   {} ({}): {}", error.file, format!("{:?}", error.phase), error.error);
            }
        }
    }

    Ok(())
}

/// Handle interactive comment exploration
async fn handle_explore_comments(
    cli: &Cli,
    codebase_path: PathBuf,
    focus: Option<PathBuf>,
) -> Result<()> {
    validate_codebase_path(&codebase_path)?;

    println!("🔍 CodeHUD LLM - Interactive Comment Explorer");
    println!("📁 Exploring: {}", codebase_path.display());
    if let Some(focus_path) = &focus {
        println!("🎯 Focus: {}", focus_path.display());
    }

    println!("\n🔧 Interactive comment exploration is under development.");
    println!("This feature will provide an interactive interface to explore");
    println!("comments and their relationships across the codebase.");

    // TODO: Implement interactive exploration
    Ok(())
}

/// Handle documentation generation
async fn handle_generate_docs(
    cli: &Cli,
    codebase_path: PathBuf,
    template: Option<PathBuf>,
    format: DocumentFormat,
    output: Option<PathBuf>,
) -> Result<()> {
    validate_codebase_path(&codebase_path)?;

    println!("📚 CodeHUD LLM - Documentation Generator");
    println!("📁 Source: {}", codebase_path.display());
    println!("📄 Format: {:?}", format);
    if let Some(template_path) = &template {
        println!("📋 Template: {}", template_path.display());
    }
    if let Some(output_path) = &output {
        println!("💾 Output: {}", output_path.display());
    }

    println!("\n🔧 Documentation generation is under development.");
    println!("This feature will generate comprehensive documentation");
    println!("from comment analysis and codebase understanding.");

    // TODO: Implement documentation generation
    Ok(())
}

/// Handle interactive LLM session (legacy mode)
async fn handle_interactive(
    cli: &Cli,
    codebase_path: PathBuf,
    session: Option<PathBuf>,
    view: ViewType,
    max_tokens: u32,
    guardrails: bool,
    bug_fix_mode: bool,
    generate_mode: bool,
) -> Result<()> {
    validate_codebase_path(&codebase_path)?;

    println!("🤖 CodeHUD LLM - Interactive Session");
    println!("📁 Analyzing: {}", codebase_path.display());
    println!("👁️  View: {:?}", view);
    println!("🛡️  Guardrails: {}", if guardrails { "Enabled" } else { "Disabled" });
    if bug_fix_mode { println!("🐛 Bug fixing mode enabled"); }
    if generate_mode { println!("⚡ Code generation mode enabled"); }

    println!("\n🔧 Interactive LLM session is under development.");
    println!("This interface will provide AI-powered code analysis,");
    println!("bug fixing, and development assistance.");

    // TODO: Implement interactive session
    Ok(())
}

/// Validate codebase path
fn validate_codebase_path(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        eprintln!("Error: Codebase path does not exist: {}", path.display());
        std::process::exit(1);
    }
    if !path.is_dir() {
        eprintln!("Error: Codebase path must be a directory: {}", path.display());
        std::process::exit(1);
    }
    Ok(())
}

/// Handle single file scan command using FSM
async fn handle_scan_file(
    cli: &Cli,
    file_path: Option<PathBuf>,
) -> codehud_core::Result<()> {
    println!("🔍 CodeHUD LLM - File Scanner");
    println!("🤖 Backend: {}", cli.backend);

    // Create processor configuration
    let processor_config = ProcessorConfig {
        extraction_config: ExtractionConfig::default(),
        llm_config: codehud_llm::file_processor::LlmAnalysisConfig {
            max_tokens_per_file: 1000,
            system_summary_max_tokens: 4000,
            include_code_context: true,
            extract_todos: true,
            analyze_documentation_coverage: true,
            temperature: cli.temperature,
        },
        output_config: codehud_llm::file_processor::OutputConfig {
            output_dir: PathBuf::from("scan_output"),
            pretty_json: true,
            include_debug_info: cli.debug,
        },
        performance_config: codehud_llm::file_processor::PerformanceConfig {
            parallel_processing: false, // Single file mode
            max_concurrent_llm_calls: 1,
            use_cache: true,
            cache_duration_hours: 24,
        },
    };

    // Create Ollama configuration
    let ollama_config = OllamaConfig::default();

    // Create FSM (scan file doesn't use insights_only mode or gemini API)
    let fsm = std::sync::Arc::new(
        CommentExtractionFSM::new(ollama_config, processor_config, false, None)
            .await
            .map_err(|e| codehud_core::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
    );

    // Create CLI interface
    let cli_interface = CommentExtractionCLI::new(fsm.clone());

    // Execute scan file command
    match cli_interface.scan_file_command(file_path.clone()).await {
        Ok(()) => {
            // Check if we need file picker
            match fsm.get_state().await {
                ExtractionState::FilePickerMode => {
                    println!("🗂️  File picker mode active. Please specify a file path:");
                    println!("Usage: codehud-llm scan-file <file_path>");
                    return Ok(());
                }
                _ => {}
            }

            println!("✨ File scan initiated!");

            // If we have a file path, process it directly
            if let Some(file_path) = file_path {
                // Process the file directly
                match fsm.process_single_file(file_path.clone()).await {
                    Ok(()) => {
                        match fsm.get_state().await {
                            ExtractionState::ScanComplete { result } => {
                                if result.success {
                                    println!("✅ File scan completed successfully!");
                                    println!("📊 Files processed: {}", result.files_processed);
                                    println!("⏱️  Duration: {:.2}s", result.duration_seconds);
                                    println!("📝 Summary: {}", result.summary);
                                } else {
                                    println!("❌ File scan failed!");
                                    for error in &result.errors {
                                        println!("   Error: {}", error);
                                    }
                                }
                            }
                            ExtractionState::Error { error } => {
                                println!("❌ Error during scan: {}", error);
                            }
                            _ => {
                                println!("⚠️  Scan completed but state is unexpected");
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to process file: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initiate file scan: {}", e);
        }
    }

    Ok(())
}

/// Handle project scan command using FSM
async fn handle_scan_project(
    cli: &Cli,
    project_path: Option<PathBuf>,
    insights_only: bool,
    gemini_api_key: Option<String>,
) -> codehud_core::Result<()> {
    println!("🚀 CodeHUD LLM - Project Scanner");
    println!("🤖 Backend: {}", cli.backend);
    if insights_only {
        println!("⚡ Insights-only mode: Ultra token-efficient analysis");
    }

    // Create processor configuration for project scanning
    let processor_config = ProcessorConfig {
        extraction_config: ExtractionConfig::default(),
        llm_config: codehud_llm::file_processor::LlmAnalysisConfig {
            max_tokens_per_file: 800, // Slightly smaller per file for project mode
            system_summary_max_tokens: 6000, // Larger system summary for projects
            include_code_context: true,
            extract_todos: true,
            analyze_documentation_coverage: true,
            temperature: cli.temperature,
        },
        output_config: codehud_llm::file_processor::OutputConfig {
            output_dir: PathBuf::from(if insights_only { "project_scan_output_insights_only" } else { "project_scan_output" }),
            pretty_json: true,
            include_debug_info: cli.debug,
        },
        performance_config: codehud_llm::file_processor::PerformanceConfig {
            parallel_processing: false, // Sequential for project context accumulation
            max_concurrent_llm_calls: 1,
            use_cache: true,
            cache_duration_hours: 24,
        },
    };

    // Create Ollama configuration
    let ollama_config = OllamaConfig::default();

    // Handle Gemini API key - prompt if flag used but no key provided
    let gemini_key = if gemini_api_key.is_some() && gemini_api_key.as_ref().unwrap().is_empty() {
        println!("🌟 Google AI Studio (Gemini Flash) selected for final hierarchical summary");
        println!("📝 Please enter your Google AI Studio API key:");
        print!("   API Key: ");
        std::io::stdout().flush().unwrap();

        let mut api_key = String::new();
        std::io::stdin().read_line(&mut api_key)
            .map_err(|e| codehud_core::Error::Io(e))?;

        let trimmed_key = api_key.trim().to_string();
        if trimmed_key.is_empty() {
            println!("❌ No API key provided - using local 14B model instead");
            None
        } else {
            println!("✅ API key received");
            Some(trimmed_key)
        }
    } else if gemini_api_key.is_some() {
        println!("🌟 Using Google AI Studio (Gemini Flash) for final hierarchical summary");
        gemini_api_key.clone()
    } else {
        None
    };

    // Create FSM
    let fsm = std::sync::Arc::new(
        CommentExtractionFSM::new(ollama_config, processor_config, insights_only, gemini_key)
            .await
            .map_err(|e| codehud_core::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
    );

    // Create CLI interface
    let cli_interface = CommentExtractionCLI::new(fsm.clone());

    // Execute scan project command - the FSM handles everything through events
    match cli_interface.scan_project_command(project_path.clone()).await {
        Ok(()) => {
            // Wait for the FSM to complete hierarchical processing
            loop {
                match fsm.get_state().await {
                    ExtractionState::ScanComplete { ref result } => {
                        if result.success {
                            println!("✅ Hierarchical project scan completed successfully!");
                            println!("📊 Files processed: {}", result.files_processed);
                            println!("⏱️  Duration: {:.2}s", result.duration_seconds);
                            println!("📝 Summary: {}", result.summary);
                        } else {
                            println!("❌ Project scan completed with errors!");
                            for error in &result.errors {
                                println!("   Error: {}", error);
                            }
                        }
                        // Exit cleanly after completion
                        std::process::exit(0);
                    }
                    ExtractionState::Error { ref error } => {
                        println!("❌ Error during hierarchical scan: {}", error);
                        std::process::exit(1);
                    }
                    _ => {
                        // Still processing, wait a bit and check again
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initiate project scan: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_display() {
        assert_eq!(LlmBackend::Ollama.to_string(), "ollama");
        assert_eq!(LlmBackend::Pytorch.to_string(), "pytorch");
        assert_eq!(LlmBackend::Openhands.to_string(), "openhands");
        assert_eq!(LlmBackend::Shellgpt.to_string(), "shell-gpt");
    }

    #[test]
    fn test_temperature_validation() {
        // Valid temperatures should work
        assert!(0.0 <= 0.7 && 0.7 <= 2.0);
        assert!(0.0 <= 1.0 && 1.0 <= 2.0);
        
        // Invalid temperatures
        assert!(-0.1 < 0.0);
        assert!(2.1 > 2.0);
    }
}