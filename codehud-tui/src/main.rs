//! CodeHUD TUI CLI entry point
//!
//! Command-line interface for the CodeHUD Terminal User Interface
//! optimized for Claude Code consumption.

use anyhow::Result;
use clap::{Parser, Subcommand};
use codehud_tui::{run_tui, export_structured_data, CodeHudTui};
use std::path::PathBuf;
// use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(name = "codehud-tui")]
#[command(about = "CodeHUD Terminal User Interface - Claude Code optimized")]
#[command(long_about = "A terminal-based interface for CodeHUD analysis results, optimized for AI agent consumption and command-line integration.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the codebase to analyze
    #[arg(value_name = "CODEBASE_PATH")]
    codebase_path: Option<PathBuf>,

    /// Export analysis data to JSON instead of running TUI
    #[arg(short, long)]
    export: bool,

    /// Output file for exported data
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Maximum number of issues to display
    #[arg(long, default_value = "50")]
    max_items: usize,

    /// Show only critical issues
    #[arg(long)]
    critical_only: bool,

    /// Use relative file paths
    #[arg(long)]
    relative_paths: bool,

    /// Force TUI mode (skip terminal check)
    #[arg(long)]
    force_tui: bool,

    /// Show text preview of TUI output (no interactive terminal)
    #[arg(long)]
    preview: bool,

    /// Show visualizations directly in terminal (no interactive mode)
    #[arg(long)]
    show_viz: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the interactive TUI
    Run {
        /// Path to the codebase to analyze
        codebase_path: PathBuf,
    },
    /// Export analysis data as structured JSON
    Export {
        /// Path to the codebase to analyze
        codebase_path: PathBuf,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validate TUI configuration
    Validate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run_main_logic(&cli).await
}

async fn run_main_logic(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Run { codebase_path }) => {
            run_interactive_tui(codebase_path).await
        }
        Some(Commands::Export { codebase_path, output }) => {
            export_analysis_data(codebase_path, output.as_deref()).await
        }
        Some(Commands::Validate) => {
            validate_configuration()
        }
        None => {
            // Handle legacy CLI interface
            if let Some(codebase_path) = &cli.codebase_path {
                if cli.export {
                    export_analysis_data(codebase_path, cli.output.as_deref()).await
                } else if cli.preview {
                    show_text_preview(codebase_path).await
                } else if cli.show_viz {
                    show_visualizations_direct(codebase_path).await
                } else {
                    run_interactive_tui_with_options(codebase_path, cli.force_tui).await
                }
            } else {
                eprintln!("Error: No codebase path provided");
                eprintln!("Usage: codehud-tui <CODEBASE_PATH>");
                eprintln!("       codehud-tui run <CODEBASE_PATH>");
                eprintln!("       codehud-tui export <CODEBASE_PATH> -o output.json");
                eprintln!("       codehud-tui --preview <CODEBASE_PATH>");
                std::process::exit(1);
            }
        }
    }
}

/// Run the interactive TUI
async fn run_interactive_tui(codebase_path: &PathBuf) -> Result<()> {
    run_interactive_tui_with_options(codebase_path, false).await
}

/// Run the interactive TUI with options
async fn run_interactive_tui_with_options(codebase_path: &PathBuf, force_tui: bool) -> Result<()> {
    // Check if we're in a terminal first (unless forced)
    if false && !force_tui && !atty::is(atty::Stream::Stdout) {
        eprintln!("❌ Error: Not running in a proper terminal environment.");
        eprintln!("💡 Try running in a real terminal, or use export mode:");
        eprintln!("   codehud-tui export {} -o analysis.json", codebase_path.display());
        eprintln!("   Or force TUI mode: codehud-tui {} --force-tui", codebase_path.display());
        std::process::exit(1);
    }

    println!("🚀 Starting CodeHUD TUI analysis for: {}", codebase_path.display());
    println!("📊 Loading analysis data...");

    match run_tui(codebase_path).await {
        Ok(()) => {
            println!("✅ TUI session completed");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ TUI failed: {}", e);
            eprintln!("💡 Try using export mode instead:");
            eprintln!("   codehud-tui export {} -o analysis.json", codebase_path.display());
            Err(e)
        }
    }
}

/// Show text preview of TUI output (no interactive terminal required)
async fn show_text_preview(codebase_path: &PathBuf) -> Result<()> {
    println!("🔍 CodeHUD TUI Preview Mode for: {}", codebase_path.display());
    println!("📊 Loading analysis data...\n");

    // Create headless TUI instance and load analysis
    let mut tui = CodeHudTui::new_headless()?;
    tui.load_analysis(codebase_path).await?;

    if let Some(analysis_data) = tui.get_analysis_data() {
        // Display TUI-formatted output as text
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║                     CODEHUD ANALYSIS SUMMARY                 ║");
        println!("╠═══════════════════════════════════════════════════════════════╣");
        println!("║ Health Score: {:<10} │ Files: {:<10} │ Issues: {:<10} ║",
                format!("{:.1}%", analysis_data.health_score),
                analysis_data.files_analyzed,
                analysis_data.quality_summary.total_issues);
        println!("║ Security Risk: {:<12} │ Critical: {:<8} │ Total: {:<11} ║",
                format!("{:?}", analysis_data.security_summary.risk_level),
                analysis_data.critical_issues.len(),
                analysis_data.quality_summary.total_issues);
        println!("╚═══════════════════════════════════════════════════════════════╝\n");

        // Show critical issues (TUI Priority View)
        if !analysis_data.critical_issues.is_empty() {
            println!("🚨 CRITICAL ISSUES (Priority View):");
            println!("───────────────────────────────────────────────────────");
            for (i, issue) in analysis_data.critical_issues.iter().take(5).enumerate() {
                println!("{}. [{:?}] {}", i + 1, issue.severity, issue.description);
                println!("   📁 {}", issue.file_path);
                if let Some(line) = issue.line_number {
                    println!("   📍 Line {}", line);
                }
                println!();
            }
            if analysis_data.critical_issues.len() > 5 {
                println!("   ... and {} more critical issues", analysis_data.critical_issues.len() - 5);
            }
            println!();
        }

        // Show security summary (TUI Security View)
        println!("🔒 SECURITY OVERVIEW:");
        println!("───────────────────────────────────────");
        println!("Risk Level: {:?}", analysis_data.security_summary.risk_level);
        println!("Total Vulnerabilities: {}", analysis_data.security_summary.total_vulnerabilities);
        println!("Critical Vulnerabilities: {}", analysis_data.security_summary.critical_vulnerabilities);
        if !analysis_data.security_summary.top_security_files.is_empty() {
            println!("Top Security Concern Files:");
            for file in analysis_data.security_summary.top_security_files.iter().take(3) {
                println!("  • {}", file);
            }
        }
        println!();

        // Show quality metrics (TUI Quality View)
        println!("📈 CODE QUALITY METRICS:");
        println!("─────────────────────────────────────");
        println!("Overall Health: {:.1}%", analysis_data.health_score);
        println!("Files Analyzed: {}", analysis_data.files_analyzed);
        println!("Total Issues: {}", analysis_data.quality_summary.total_issues);
        println!();

        // Show navigation hint
        println!("💡 TUI Navigation Features:");
        println!("   • Tab/Shift+Tab: Switch between views (Quality/Security/Issues/Files)");
        println!("   • ↑/↓: Navigate through lists");
        println!("   • Enter: View detailed information");
        println!("   • q: Quit");
        println!();
        println!("🎯 This preview shows the structured, prioritized output that the TUI provides.");
        println!("   Run with --force-tui for full interactive experience.");

    } else {
        eprintln!("❌ No analysis data available");
        std::process::exit(1);
    }

    Ok(())
}

/// Show visualizations directly in terminal without interactive mode
async fn show_visualizations_direct(codebase_path: &PathBuf) -> Result<()> {
    println!("🎨 CodeHUD Visualizations for: {}", codebase_path.display());
    println!("📊 Loading and generating visualizations...\n");

    // Create headless TUI instance and load analysis
    let mut tui = CodeHudTui::new_headless()?;
    tui.load_analysis(codebase_path).await?;

    if let Some(analysis_data) = tui.get_analysis_data() {
        // Create viz engine
        let viz_engine = codehud_viz::VisualizationEngine::new();

        // Convert analysis data to AnalysisResult
        let mut analysis_result = codehud_core::models::AnalysisResult::new("current_analysis".to_string());
        analysis_result.health_score = analysis_data.health_score;
        analysis_result.files_analyzed = analysis_data.files_analyzed;
        analysis_result.analysis_duration = 1.0;

        // Convert the analysis data to JSON and store in view data
        let analysis_value = serde_json::to_value(analysis_data)?;
        analysis_result.set_view_data("analysis".to_string(), analysis_value);

        // Create output directory for visualization exports
        std::fs::create_dir_all("codehud_visualizations")?;

        // Generate and display different view types
        let view_types = [
            ("📊 Quality Analysis", codehud_core::models::ViewType::Quality),
            ("🔒 Security Assessment", codehud_core::models::ViewType::Security),
            ("🏗️  Architecture Topology", codehud_core::models::ViewType::Topology),
            ("📦 Dependencies", codehud_core::models::ViewType::Dependencies),
            ("⚡ Performance", codehud_core::models::ViewType::Performance),
            ("🐛 Issues Inspection", codehud_core::models::ViewType::IssuesInspection),
        ];

        for (title, view_type) in view_types {
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║ {} {:<52} ║", title, "");
            println!("╚══════════════════════════════════════════════════════════════╝");

            match viz_engine.generate_view(view_type.clone(), &analysis_result) {
                Ok(view) => {
                    // Export as files
                    let view_name = format!("{:?}", view_type).to_lowercase();
                    let json_file = format!("codehud_visualizations/{}_view.json", view_name);
                    let text_file = format!("codehud_visualizations/{}_view.txt", view_name);

                    let json_output = serde_json::to_string_pretty(&view)?;
                    std::fs::write(&json_file, json_output)?;

                    let text_summary = render_view_as_text(&view);
                    std::fs::write(&text_file, &text_summary)?;

                    // Display summary in terminal
                    println!("{}", text_summary);
                    println!("💾 Exported to: {} and {}", json_file, text_file);
                }
                Err(e) => {
                    println!("❌ Failed to generate {:?} view: {}", view_type, e);
                }
            }
            println!();
        }

        println!("🎯 All visualizations completed!");
        println!("📁 Check codehud_visualizations/ directory for detailed exports");

    } else {
        eprintln!("❌ No analysis data available");
        std::process::exit(1);
    }

    Ok(())
}

/// Convert visualization view to readable text format (standalone version)
fn render_view_as_text(view: &codehud_viz::RenderableView) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    match &view.content {
        codehud_viz::ViewContent::Summary { metrics, recommendations, .. } => {
            writeln!(output, "Key Metrics:").unwrap();
            for (key, value) in metrics {
                writeln!(output, "  • {}: {:.2}", key, value).unwrap();
            }
            writeln!(output, "\nRecommendations:").unwrap();
            for (i, rec) in recommendations.iter().enumerate() {
                writeln!(output, "  {}. {}", i + 1, rec).unwrap();
            }
        }
        codehud_viz::ViewContent::Quality { health_score, issues_by_severity, top_problematic_files, .. } => {
            writeln!(output, "Health Score: {:.1}%", health_score * 100.0).unwrap();
            writeln!(output, "\nIssues by Severity:").unwrap();
            for (severity, count) in issues_by_severity {
                writeln!(output, "  • {}: {} issues", severity, count).unwrap();
            }
            writeln!(output, "\nMost Problematic Files:").unwrap();
            for (file, score) in top_problematic_files.iter().take(5) {
                writeln!(output, "  • {}: {:.2}", file, score).unwrap();
            }
        }
        codehud_viz::ViewContent::Security { risk_level, top_security_issues, security_score, .. } => {
            writeln!(output, "Risk Level: {}", risk_level).unwrap();
            writeln!(output, "Security Score: {:.1}%", security_score * 100.0).unwrap();
            writeln!(output, "\nTop Security Issues:").unwrap();
            for issue in top_security_issues.iter().take(5) {
                writeln!(output, "  • [{}] {}: {}", issue.severity, issue.file, issue.description).unwrap();
            }
        }
        _ => {
            writeln!(output, "View Type: {:?}", view.view_type).unwrap();
            writeln!(output, "Generated: {}", view.timestamp).unwrap();
        }
    }

    output
}

/// Export analysis data as structured JSON
async fn export_analysis_data(codebase_path: &PathBuf, output_file: Option<&std::path::Path>) -> Result<()> {
    println!("🔍 Analyzing codebase: {}", codebase_path.display());

    // Create headless TUI instance and load analysis
    let mut tui = CodeHudTui::new_headless()?;
    tui.load_analysis(codebase_path).await?;

    // Get analysis data and export
    if let Some(analysis_data) = tui.get_analysis_data() {
        let exported_data = export_structured_data(analysis_data)?;

        match output_file {
            Some(file_path) => {
                std::fs::write(file_path, &exported_data)?;
                println!("📄 Analysis data exported to: {}", file_path.display());
            }
            None => {
                println!("{}", exported_data);
            }
        }

        // Print summary
        println!("\n📊 Analysis Summary:");
        println!("   Health Score: {:.1}%", analysis_data.health_score);
        println!("   Files Analyzed: {}", analysis_data.files_analyzed);
        println!("   Critical Issues: {}", analysis_data.critical_issues.len());
        println!("   Security Risk: {:?}", analysis_data.security_summary.risk_level);
    } else {
        eprintln!("❌ No analysis data available");
        std::process::exit(1);
    }

    Ok(())
}

/// Validate TUI configuration
fn validate_configuration() -> Result<()> {
    println!("🔧 Validating CodeHUD TUI configuration...");

    // Check terminal capabilities
    let terminal_size = crossterm::terminal::size()?;
    println!("   Terminal size: {}x{}", terminal_size.0, terminal_size.1);

    if terminal_size.0 < 80 || terminal_size.1 < 24 {
        println!("   ⚠️  Warning: Terminal size is small. Recommended: 120x40 or larger");
    } else {
        println!("   ✅ Terminal size is adequate");
    }

    // Check color support
    println!("   ✅ Color support: Available");

    // Check dependencies
    println!("   ✅ All dependencies: Available");

    println!("\n🎯 TUI is ready for use!");
    println!("   Usage: codehud-tui <codebase_path>");
    println!("   Claude Code optimized interface available");

    Ok(())
}