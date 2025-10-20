//! CodeHUD Direct CLI - Direct Analysis Pipeline
//!
//! Enhanced CLI supporting both direct extraction and legacy CLI analysis,
//! matching Python cli_direct.py exactly.

use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use codehud_core::{Result, ViewType, Pipeline, CoreConfig};
use codehud_analysis::{DirectAnalysisPipeline, AnalysisExporter};
use tokio::time::Duration;

#[derive(Parser)]
#[command(name = "codehud-direct")]
#[command(about = "CodeHUD - Visual Mission Control for Codebases")]
#[command(long_about = "Enhanced version supporting both direct extraction and legacy CLI analysis.\n\nLaunch the polymorphic code intelligence interface for real-time\narchitectural awareness and visual code analysis.")]
#[command(version = "1.0.0")]
#[command(author = "CodeHUD Team")]
struct Cli {
    /// Path to the codebase directory to analyze (optional - interactive picker if not provided)
    codebase_path: Option<PathBuf>,
    
    /// Path to CodeHUD configuration file
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,
    
    /// Focus on specific entity (file, function, or class)
    #[arg(short = 'f', long)]
    focus: Option<String>,
    
    /// Initial view to display
    #[arg(short = 'v', long, default_value = "topology")]
    view: ViewType,
    
    /// Analysis pipeline to use
    #[arg(short = 'p', long, default_value = "auto")]
    pipeline: PipelineChoice,
    
    /// Enable debug mode
    #[arg(long)]
    debug: bool,
    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PipelineChoice {
    /// Fast AST-based analysis (recommended)
    Direct,
    /// CLI tool-based analysis (comprehensive but slower)
    Legacy,
    /// Automatically choose best pipeline
    Auto,
}

impl From<PipelineChoice> for Pipeline {
    fn from(choice: PipelineChoice) -> Self {
        match choice {
            PipelineChoice::Direct => Pipeline::Direct,
            PipelineChoice::Legacy => Pipeline::Legacy,
            PipelineChoice::Auto => Pipeline::Hybrid, // Auto maps to Hybrid
        }
    }
}

impl std::fmt::Display for PipelineChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineChoice::Direct => write!(f, "direct"),
            PipelineChoice::Legacy => write!(f, "legacy"),
            PipelineChoice::Auto => write!(f, "auto"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    codehud_utils::logging::basic_config(Some(codehud_utils::logging::LogLevel::Info))?;
    
    let cli = Cli::parse();
    
    
    // Handle missing codebase path - launch interactive picker
    let codebase_path = match &cli.codebase_path {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: Codebase path does not exist: {}", path.display());
                std::process::exit(1);
            }
            if !path.is_dir() {
                eprintln!("Error: Codebase path must be a directory: {}", path.display());
                std::process::exit(1);
            }
            path.clone()
        }
        None => {
            println!("No codebase path provided. Launching interactive directory picker...");
            // TODO: Implement interactive directory picker
            eprintln!("Error: Interactive directory picker not yet implemented");
            eprintln!("Please provide a codebase path as an argument");
            std::process::exit(1);
        }
    };
    
    let pipeline: Pipeline = cli.pipeline.into();
    
    if cli.debug {
        println!("Debug mode enabled");
        println!("Codebase path: {}", codebase_path.display());
        println!("View: {}", cli.view);
        println!("Pipeline: {} -> {}", cli.pipeline, pipeline);
        if let Some(focus) = &cli.focus {
            println!("Focus: {}", focus);
        }
        if let Some(config) = &cli.config {
            println!("Config: {}", config.display());
        }
    }
    
    println!("CodeHUD Direct Analysis Pipeline");
    println!("Analyzing: {}", codebase_path.display());
    println!("View: {}", cli.view);
    println!("Pipeline: {}", pipeline);
    
    match pipeline {
        Pipeline::Direct => {
            println!("Using direct AST-based analysis (fast)");
            run_direct_analysis(&codebase_path, &cli).await?
        }
        Pipeline::Legacy => {
            println!("Using legacy CLI-based analysis (comprehensive)");
            println!("Legacy pipeline not yet implemented");
        }
        Pipeline::Hybrid => {
            println!("Auto-selecting optimal pipeline");
            println!("Selected: Direct pipeline (default for auto mode)");
            run_direct_analysis(&codebase_path, &cli).await?
        }
    }
    
    if let Some(focus) = cli.focus {
        println!("Focusing analysis on: {}", focus);
        // TODO: Implement focus functionality
    }
    
    // TODO: Launch the polymorphic HUD interface
    println!("Launching polymorphic code intelligence interface...");
    println!("(Implementation pending)");
    
    Ok(())
}

/// Run the direct analysis pipeline
async fn run_direct_analysis(codebase_path: &PathBuf, cli: &Cli) -> Result<()> {
    let logger = codehud_utils::logging::get_logger("codehud.direct");
    
    // Create core configuration
    let mut config = CoreConfig::default();
    
    // Configure based on CLI arguments
    if cli.debug {
        config.max_files = 100; // Limit for debug mode
    }
    
    // Configure specific view if requested
    let views_to_run = match cli.view {
        ViewType::Topology => vec![ViewType::Topology],
        view => vec![view], // Single view
    };
    
    logger.info(&format!("Starting direct analysis for {} views", views_to_run.len()));
    
    // Create and configure pipeline
    let pipeline = DirectAnalysisPipeline::new(codebase_path, config)?
        .with_extractors(&views_to_run)
        .with_timeout(Duration::from_secs(300))
        .with_parallel_execution(true);
    
    // Run analysis
    let start_time = std::time::Instant::now();
    let result = pipeline.analyze().await?;
    let elapsed = start_time.elapsed();
    
    // Display results
    println!("\\n✅ Analysis completed in {:.2}s", elapsed.as_secs_f64());
    println!("📊 Coverage: {:.1}%", result.metadata.analysis_coverage);
    println!("🔍 Extractors run: {}", result.extractors_run.len());
    println!("📁 Views generated: {}", result.views.len());
    println!("🏥 Health Score: {:.1}/100", result.health_score.overall_score);
    
    if !result.errors.is_empty() {
        println!("\\n❌ Errors ({}):", result.errors.len());
        for error in &result.errors {
            println!("   - {}", error);
        }
    }
    
    if !result.warnings.is_empty() {
        println!("\\n⚠️ Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("   - {}", warning);
        }
    }
    
    // Export results
    if cli.debug {
        println!("\\n📄 Exporting detailed results...");
        
        // Export to JSON
        let json_output = AnalysisExporter::to_json(&result)?;
        let json_path = codebase_path.join(".codehud_analysis.json");
        std::fs::write(&json_path, json_output)?;
        println!("   JSON: {}", json_path.display());
        
        // Export to Markdown
        let markdown_output = AnalysisExporter::to_markdown(&result)?;
        let md_path = codebase_path.join(".codehud_analysis.md");
        std::fs::write(&md_path, markdown_output)?;
        println!("   Markdown: {}", md_path.display());
    }
    
    // Display view-specific information
    for (view_name, view_data) in &result.views {
        println!("\\n📋 {} View:", view_name);
        
        // Extract key metrics from each view
        if let Some(summary) = view_data.get("summary") {
            println!("   Summary: {}", summary);
        }
        
        if view_name == "topology" {
            if let Some(files) = view_data.get("files_analyzed") {
                println!("   Files analyzed: {}", files);
            }
            if let Some(functions) = view_data.get("total_functions") {
                println!("   Functions found: {}", functions);
            }
        }
        
        if view_name == "issues" {
            if let Some(total) = view_data.get("total_issues") {
                println!("   Total issues: {}", total);
            }
        }
    }
    
    // Show focus information if applicable
    if let Some(focus) = &cli.focus {
        println!("\\n🎯 Focus Analysis on: {}", focus);
        // TODO: Implement focus-specific analysis
        println!("   (Focus analysis implementation pending)");
    }
    
    println!("\\n🚀 Analysis complete! Use the generated reports for detailed insights.");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_conversion() {
        assert_eq!(Pipeline::from(PipelineChoice::Direct), Pipeline::Direct);
        assert_eq!(Pipeline::from(PipelineChoice::Legacy), Pipeline::Legacy);
        assert_eq!(Pipeline::from(PipelineChoice::Auto), Pipeline::Hybrid);
    }

    #[test]
    fn test_pipeline_display() {
        assert_eq!(PipelineChoice::Direct.to_string(), "direct");
        assert_eq!(PipelineChoice::Legacy.to_string(), "legacy");
        assert_eq!(PipelineChoice::Auto.to_string(), "auto");
    }
}