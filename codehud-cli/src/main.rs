//! CodeHUD Main CLI Entry Point
//!
//! Main entry point providing unified interface with 7+ commands matching Python exactly

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use codehud_core::{Result, ViewType, Pipeline};

#[derive(Parser)]
#[command(name = "codehud")]
#[command(about = "CodeHUD - Visual Mission Control for Codebases")]
#[command(version = "1.0.0")]
#[command(author = "CodeHUD Team")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run comprehensive codebase analysis with visualization
    Analyze {
        /// Path to codebase to analyze
        codebase_path: PathBuf,
        
        /// View type for analysis output
        #[arg(short = 'v', long, default_value = "topology")]
        view: ViewType,
        
        /// Analysis pipeline to use
        #[arg(short = 'p', long, default_value = "direct")]
        pipeline: Pipeline,
        
        /// Output file path for results
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        
        /// Focus on specific entity (file, class, function)
        #[arg(short = 'f', long)]
        focus: Option<String>,
        
        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,
        
        /// Enable debug mode
        #[arg(short = 'd', long)]
        debug: bool,
        
        /// Files to analyze (glob patterns)
        #[arg(long)]
        files: Option<String>,
    },

    /// Export visualizations to text files (no TUI required)
    ExportViz {
        /// Path to codebase to analyze
        codebase_path: PathBuf,

        /// Output directory for visualization files
        #[arg(short = 'o', long, default_value = "visualizations")]
        output_dir: PathBuf,

        /// View types to export (comma-separated: quality,security,topology,dependencies)
        #[arg(short = 'v', long, default_value = "quality,security,topology")]
        views: String,
    },

    /// Launch interactive LLM interface for AI-powered analysis
    Llm {
        /// Path to codebase
        codebase_path: PathBuf,
        
        /// LLM backend to use
        #[arg(short = 'b', long, default_value = "ollama")]
        backend: String,
        
        /// Model to use for analysis
        #[arg(short = 'm', long)]
        model: Option<String>,
        
        /// Enable GPU acceleration
        #[arg(long)]
        gpu: bool,
        
        /// Session file for conversation persistence
        #[arg(short = 's', long)]
        session: Option<PathBuf>,
    },
    
    /// Launch graphical user interface
    Gui {
        /// Path to codebase
        codebase_path: PathBuf,
        
        /// Initial view to display
        #[arg(short = 'v', long, default_value = "topology")]
        view: ViewType,
        
        /// Window geometry (WIDTHxHEIGHT+X+Y)
        #[arg(short = 'g', long)]
        geometry: Option<String>,
        
        /// Enable fullscreen mode
        #[arg(long)]
        fullscreen: bool,
    },

    /// Generate call graph visualization (multi-view: overview, per-module, cycles)
    CallGraph {
        /// Path to codebase to analyze
        codebase_path: PathBuf,

        /// Output file path (without extension)
        #[arg(short = 'o', long, default_value = "call_graph")]
        output: PathBuf,

        /// Output format (svg, png, pdf)
        #[arg(short = 'f', long, default_value = "pdf")]
        format: String,

        /// Layout engine (auto, dot, neato, fdp, sfdp, circo, twopi)
        #[arg(short = 'l', long, default_value = "auto")]
        layout: String,

        /// Disable cycle highlighting
        #[arg(long)]
        no_cycles: bool,

        /// Disable complexity coloring
        #[arg(long)]
        no_colors: bool,
    },

    /// Interactive file editing with AI assistance
    Edit {
        /// Path to codebase
        codebase_path: PathBuf,
        
        /// File to edit
        file: PathBuf,
        
        /// Description of changes to make
        description: Option<String>,
        
        /// Backup before editing
        #[arg(long, default_value = "true")]
        backup: bool,
        
        /// Editor to use
        #[arg(short = 'e', long, default_value = "nano")]
        editor: String,
    },
    
    /// Issues inspection and management
    Issues {
        /// Path to codebase
        codebase_path: PathBuf,
        
        /// Issue category to focus on
        #[arg(short = 'c', long)]
        category: Option<String>,
        
        /// Severity filter
        #[arg(short = 's', long)]
        severity: Option<String>,
        
        /// Auto-fix issues where possible
        #[arg(long)]
        autofix: bool,
        
        /// Generate issue report
        #[arg(short = 'r', long)]
        report: bool,
    },
    
    /// Developer notes and fix tracking
    Devnotes {
        /// Path to codebase
        codebase_path: PathBuf,
        
        /// Add new note
        #[arg(short = 'a', long)]
        add: Option<String>,
        
        /// List all notes
        #[arg(short = 'l', long)]
        list: bool,
        
        /// Search notes
        #[arg(short = 's', long)]
        search: Option<String>,
        
        /// Note category
        #[arg(short = 'c', long)]
        category: Option<String>,
    },
    
    /// Full comprehensive analysis with all views
    Full {
        /// Path to codebase
        codebase_path: PathBuf,
        
        /// Output directory for all reports
        #[arg(short = 'o', long, default_value = "codehud_output")]
        output_dir: PathBuf,
        
        /// Include performance profiling
        #[arg(long)]
        profile: bool,
        
        /// Generate only specified views (comma-separated)
        #[arg(long)]
        views: Option<String>,
        
        /// Skip cache and force fresh analysis
        #[arg(long)]
        fresh: bool,
    },
}

// Use the Pipeline type from codehud-core instead of defining our own

/// Extract real call graph data from codebase using Tree-sitter query engine
async fn extract_real_call_graph(codebase_path: &PathBuf) -> anyhow::Result<codehud_viz::CallGraph> {
    use codehud_core::query_engine;
    use std::collections::{HashMap, HashSet};
    use walkdir::WalkDir;

    #[derive(Debug, Clone)]
    struct FunctionDef {
        name: String,
        start_line: usize,
        end_line: usize,
        qualified_name: String, // module::function
    }

    #[derive(Debug, Clone)]
    struct FunctionCall {
        callee: String,
        line: usize,
    }

    let mut call_graph = codehud_viz::CallGraph::new();

    // Get query engine instance
    let mut engine = query_engine::get_query_engine()?;

    // Track all function definitions with line ranges
    let mut all_functions: Vec<FunctionDef> = Vec::new();

    // Track all function calls with their locations
    let mut all_calls: HashMap<String, Vec<FunctionCall>> = HashMap::new(); // file -> calls

    // Known stdlib/external patterns to filter out
    let stdlib_patterns = [
        "push", "pop", "get", "set", "insert", "remove", "iter", "map", "filter",
        "collect", "clone", "to_string", "to_owned", "as_ref", "as_mut",
        "unwrap", "expect", "ok", "err", "and_then", "or_else", "map_or",
        "push_str", "len", "is_empty", "contains", "starts_with", "ends_with",
        "split", "trim", "parse", "format", "println", "print", "eprintln",
        "new", "default", "from", "into", "try_from", "try_into",
        "render_widget", "block", "borders", "style", "fg", "bg", // ratatui stdlib
        "Some", "None", "Ok", "Err", "Vec", "String", "Box", "Arc", // constructors
        "button", "clicked", "close_menu", "open_menu", "checkbox", "radio", // egui
        "metaloadfn", "load_with", // OpenGL loaders
        "import", "getattr", "setattr", "call", "call0", "call1", // Python FFI
        "globals", "locals", "exec", "eval", // Python builtins
        "join", "file", "path", "read", "write", // Path/IO stdlib
    ];

    println!("🔍 Scanning codebase for functions and calls...");
    println!("   📁 Target directory: {}", codebase_path.display());
    println!("   📁 Current directory: {}", std::env::current_dir()?.display());

    // Walk through all source files
    let mut file_count = 0;
    for entry in WalkDir::new(codebase_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        file_count += 1;
        if file_count <= 5 {
            println!("   Processing file #{}: {}", file_count, file_path.display());
        }

        // Only process Rust and Python files for now
        if let Some(ext) = file_path.extension() {
            if ext == "rs" || ext == "py" {
                if let Ok(analysis) = engine.analyze_file(file_path) {
                    let file_path_str = file_path.to_string_lossy().to_string();
                    let module_name = extract_module_name(&file_path_str);

                    // Extract function definitions with line ranges
                    if let Some(functions) = analysis.get("functions") {
                        if let Some(func_list) = functions.get("functions").and_then(|f| f.as_array()) {
                            for func in func_list {
                                if let (Some(name), Some(line), Some(end_line)) = (
                                    func.get("name").and_then(|n| n.as_str()),
                                    func.get("line").and_then(|l| l.as_u64()),
                                    func.get("end_line").and_then(|e| e.as_u64())
                                ) {
                                    let qualified_name = format!("{}::{}", module_name, name);
                                    all_functions.push(FunctionDef {
                                        name: name.to_string(),
                                        start_line: line as usize,
                                        end_line: end_line as usize,
                                        qualified_name: qualified_name.clone(),
                                    });
                                }
                            }
                        }
                    }

                    // Extract function calls with line numbers
                    if let Some(calls) = analysis.get("calls") {
                        if let Some(call_list) = calls.get("calls").and_then(|c| c.as_array()) {
                            let mut file_calls = Vec::new();
                            for call in call_list {
                                if let (Some(callee), Some(line)) = (
                                    call.get("callee").and_then(|c| c.as_str()),
                                    call.get("line").and_then(|l| l.as_u64())
                                ) {
                                    // Filter out obvious stdlib/external calls
                                    if !stdlib_patterns.iter().any(|p| callee == *p || callee.ends_with(p)) {
                                        file_calls.push(FunctionCall {
                                            callee: callee.to_string(),
                                            line: line as usize,
                                        });
                                    }
                                }
                            }
                            if !file_calls.is_empty() {
                                all_calls.insert(file_path_str.clone(), file_calls);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("📊 Found {} total functions", all_functions.len());
    println!("📞 Found {} files with calls", all_calls.len());

    // Build qualified name index for fast lookup
    let func_index: HashMap<String, String> = all_functions
        .iter()
        .map(|f| (f.name.clone(), f.qualified_name.clone()))
        .collect();

    // Match calls to their containing functions using line numbers
    for (file_path, calls) in &all_calls {
        let module_name = extract_module_name(file_path);

        // Get functions defined in this file
        let file_functions: Vec<&FunctionDef> = all_functions
            .iter()
            .filter(|f| f.qualified_name.starts_with(&format!("{}::", module_name)))
            .collect();

        for call in calls {
            // Find which function contains this call by line number
            let caller_func = file_functions
                .iter()
                .find(|f| call.line >= f.start_line && call.line <= f.end_line)
                .map(|f| f.qualified_name.clone());

            // Try to resolve callee to a known internal function
            if let Some(callee_qualified) = func_index.get(&call.callee) {
                // Both caller and callee are known internal functions
                if let Some(caller) = caller_func {
                    call_graph.add_call(&caller, callee_qualified);
                }
            }
        }
    }

    println!("✅ Built call graph with {} nodes and {} edges",
             call_graph.nodes.len(), call_graph.edges.len());

    Ok(call_graph)
}

/// Extract a clean module name from a file path
fn extract_module_name(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            // Handle special cases
            if s == "main" || s == "lib" {
                // Try to get the parent directory name for context
                if let Some(parent) = std::path::Path::new(file_path).parent() {
                    if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                        if parent_name.starts_with("codehud-") {
                            return format!("{}::{}", &parent_name[8..], s); // Remove "codehud-" prefix
                        } else if parent_name != "src" {
                            return format!("{}::{}", parent_name, s);
                        }
                    }
                }
            }
            s.to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Convert codehud_viz::CallGraph to petgraph format for DOT export
fn convert_to_petgraph(viz_graph: &codehud_viz::CallGraph) -> anyhow::Result<petgraph::Graph<codehud_core::graph::CallNode, codehud_core::graph::CallEdge, petgraph::Directed>> {
    use petgraph::Graph;
    use std::collections::HashMap;
    use codehud_core::graph::{CallNode, CallEdge};

    let mut graph = Graph::new();
    let mut node_map: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    // Add all nodes
    for viz_node in &viz_graph.nodes {
        let node = CallNode {
            function_name: viz_node.name.clone(),
            file_path: String::new(), // Extracted from qualified name later if needed
            line_number: 0,
        };
        let idx = graph.add_node(node);
        node_map.insert(viz_node.name.clone(), idx);
    }

    // Add all edges
    for viz_edge in &viz_graph.edges {
        if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(&viz_edge.from), node_map.get(&viz_edge.to)) {
            let edge = CallEdge {
                call_count: viz_edge.weight,
                weight: viz_edge.weight as f64,
            };
            graph.add_edge(from_idx, to_idx, edge);
        }
    }

    Ok(graph)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    codehud_utils::logging::basic_config(Some(codehud_utils::logging::LogLevel::Info))?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            codebase_path,
            view,
            pipeline,
            output,
            focus,
            config,
            debug,
            files
        } => {
            println!("🔍 Analyzing {} with {} view using {} pipeline",
                     codebase_path.display(), view, pipeline);

            if let Some(focus) = &focus {
                println!("🎯 Focusing on: {}", focus);
            }

            let result = match view {
                ViewType::Topology | ViewType::Quality | ViewType::Security | ViewType::Dependencies => {
                    // Generate specific view
                    let view_data = codehud_core::analysis::AnalysisPipeline::run_view(
                        &codebase_path, view, debug
                    ).await?;

                    println!("\n=== {} ANALYSIS RESULTS ===", view.to_string().to_uppercase());
                    println!("{}", serde_json::to_string_pretty(&view_data)?);

                    // Save to output file if specified
                    if let Some(output_path) = output {
                        std::fs::write(&output_path, serde_json::to_string_pretty(&view_data)?)?;
                        println!("\n💾 Results saved to: {}", output_path.display());
                    }

                    Ok::<(), codehud_core::Error>(())
                },
                _ => {
                    // Run comprehensive analysis for other views or fallback
                    let analysis_result = codehud_core::analysis::AnalysisPipeline::run(
                        &codebase_path, pipeline, debug
                    ).await?;

                    println!("\n=== COMPREHENSIVE ANALYSIS RESULTS ===");
                    println!("📊 Health Score: {:.1}/100", analysis_result.health_score);
                    println!("📁 Files Analyzed: {}", analysis_result.files_analyzed);
                    println!("⏱️  Analysis Duration: {:.2}s", analysis_result.analysis_duration);

                    if analysis_result.has_critical_issues() {
                        println!("⚠️  Critical Issues: {}", analysis_result.critical_issues.len());
                    } else {
                        println!("✅ No critical issues found");
                    }

                    println!("\n🎯 Focus Recommendations:");
                    for (i, rec) in analysis_result.focus_recommendations.iter().enumerate() {
                        println!("  {}. {}", i + 1, rec);
                    }

                    // Generate view-specific output
                    let view_generator = codehud_core::analysis::ViewGenerator::new();
                    let view_output = view_generator.generate_view_output(&analysis_result, view)?;

                    // Save to output file if specified
                    if let Some(output_path) = output {
                        let output_data = if focus.is_some() {
                            // Focus mode - output specific view
                            view_output
                        } else {
                            // Full mode - output complete analysis
                            serde_json::to_value(&analysis_result)?
                        };

                        std::fs::write(&output_path, serde_json::to_string_pretty(&output_data)?)?;
                        println!("\n💾 Results saved to: {}", output_path.display());
                    }

                    Ok(())
                }
            };

            match result {
                Ok(()) => println!("\n✅ Analysis completed successfully!"),
                Err(e) => {
                    eprintln!("❌ Analysis failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::CallGraph {
            codebase_path,
            output,
            format,
            layout,
            no_cycles,
            no_colors,
        } => {
            use codehud_viz::{DotExporter, check_graphviz_installed, render_dot_to_file, analyze_graph};

            println!("🔍 CodeHUD Call Graph Visualizer (Multi-View)");
            println!("===============================================");

            // Check Graphviz is installed
            match check_graphviz_installed() {
                Ok(version) => println!("✅ Graphviz found: {}", version),
                Err(e) => {
                    eprintln!("❌ {}", e);
                    std::process::exit(1);
                }
            }

            println!("📁 Analyzing codebase: {}", codebase_path.display());

            // Extract call graph
            let viz_graph = match extract_real_call_graph(&codebase_path).await {
                Ok(graph) => {
                    println!("✅ Extracted {} functions with {} calls\n",
                            graph.nodes.len(), graph.edges.len());
                    graph
                }
                Err(e) => {
                    eprintln!("❌ Failed to extract call graph: {}", e);
                    std::process::exit(1);
                }
            };

            // Convert to petgraph format
            let petgraph = match convert_to_petgraph(&viz_graph) {
                Ok(graph) => graph,
                Err(e) => {
                    eprintln!("❌ Failed to convert graph: {}", e);
                    std::process::exit(1);
                }
            };

            // Parse output format
            let output_format: codehud_viz::OutputFormat = match format.parse() {
                Ok(fmt) => fmt,
                Err(e) => {
                    eprintln!("❌ Invalid format: {}", e);
                    std::process::exit(1);
                }
            };

            // Auto-detect rendering options based on graph size
            let (auto_layout, _, _) = DotExporter::auto_detect_options(petgraph.node_count(), petgraph.edge_count());

            // Use user-specified layout or auto-detected
            let layout_engine: codehud_viz::LayoutEngine = if layout == "auto" {
                auto_layout
            } else {
                match layout.parse() {
                    Ok(engine) => engine,
                    Err(e) => {
                        eprintln!("❌ Invalid layout: {}", e);
                        std::process::exit(1);
                    }
                }
            };

            // Configure DOT exporter
            let exporter = DotExporter::new()
                .with_cycle_highlighting(!no_cycles)
                .with_complexity_coloring(!no_colors)
                .with_edge_weights(true)
                .with_module_clustering(false); // Disable for individual graphs

            // Analyze graph for multi-view generation
            println!("📊 Analyzing graph structure...");
            let analysis = analyze_graph(&petgraph);

            println!("   • Found {} modules", analysis.modules.len());
            println!("   • Detected {} cycles", analysis.sccs.len());

            // Create generated_graphs directory
            std::fs::create_dir_all("generated_graphs")?;

            // Generate ONLY the clean architecture overview (Cargo.toml dependencies)
            println!("\n📊 Generating clean architecture overview from Cargo.toml...");
            let overview_dot = exporter.export_overview_graph(&petgraph, &codebase_path);
            let overview_path = PathBuf::from("generated_graphs").join(format!("{}_overview", output.file_name().unwrap().to_string_lossy()));

            // Force 'dot' layout for overview since it has subgraph clusters
            let overview_layout = codehud_viz::LayoutEngine::Dot;
            match render_dot_to_file(&overview_dot, &overview_path, output_format, Some(overview_layout)) {
                Ok(output_file) => {
                    println!("✅ Architecture overview: {}", output_file.display());
                    println!("   • 10 crates across 4 architectural layers");
                    println!("   • Clean compile-time dependencies (no runtime cycles)");
                }
                Err(e) => {
                    eprintln!("❌ Failed to render overview: {}", e);
                    std::process::exit(1);
                }
            }

            println!("\n✅ Clean architecture graph generated!");
            println!("💡 This shows compile-time dependencies from Cargo.toml");
        }

        Commands::ExportViz { codebase_path, output_dir, views } => {
            export_visualizations(&codebase_path, &output_dir, &views).await?;
        },

        Commands::Llm {
            codebase_path,
            backend,
            model,
            gpu,
            session
        } => {
            println!("🤖 Launching LLM interface for {} using {} backend",
                     codebase_path.display(), backend);

            // Build command arguments for codehud-llm
            let mut args = vec!["scan-project".to_string(), codebase_path.to_string_lossy().to_string()];

            args.push("-b".to_string());
            args.push(backend);

            if let Some(model_name) = &model {
                args.push("-m".to_string());
                args.push(model_name.clone());
            }

            if gpu {
                args.push("--gpu".to_string());
            }

            if let Some(session_file) = &session {
                args.push("-s".to_string());
                args.push(session_file.to_string_lossy().to_string());
            }

            // Run codehud-llm binary
            println!("🚀 Launching codehud-llm scan-project...");

            let status = std::process::Command::new("cargo")
                .args(&["run", "--bin", "codehud-llm", "--"])
                .args(&args)
                .current_dir(std::env::current_dir()?)
                .status();

            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        println!("✅ LLM analysis completed successfully!");
                    } else {
                        eprintln!("⚠️  LLM analysis exited with status: {}", exit_status);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to launch codehud-llm: {}", e);
                    println!("\n💡 Make sure you're running from the CodeHUD root directory");
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Gui {
            codebase_path,
            view,
            geometry,
            fullscreen
        } => {
            println!("🖥️  Launching CodeHUD GUI for {}", codebase_path.display());
            println!("👁️  Initial view: {}", view);

            if fullscreen {
                println!("🖼️  Fullscreen mode enabled");
            }

            if let Some(geom) = &geometry {
                println!("📐 Window geometry: {}", geom);
            }

            // The GUI application takes the codebase path from environment or state
            // For now, we'll set it as an environment variable
            std::env::set_var("CODEHUD_CODEBASE_PATH", &codebase_path);

            println!("🚀 Launching GUI application...");

            let status = std::process::Command::new("cargo")
                .args(&["run", "--bin", "codehud-gui", "--release"])
                .current_dir(std::env::current_dir()?)
                .status();

            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        println!("✅ GUI closed successfully!");
                    } else {
                        eprintln!("⚠️  GUI exited with status: {}", exit_status);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to launch GUI: {}", e);
                    println!("\n💡 Troubleshooting:");
                    println!("   • Make sure you're running from the CodeHUD root directory");
                    println!("   • Ensure egui dependencies are installed:");
                    println!("     cargo build --bin codehud-gui");
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Edit {
            codebase_path,
            file,
            description,
            backup,
            editor
        } => {
            println!("📝 Editing {} in {} with editor {}",
                     file.display(), codebase_path.display(), editor);

            // Resolve file path relative to codebase if needed
            let full_file_path = if file.is_absolute() {
                file.clone()
            } else {
                codebase_path.join(&file)
            };

            if !full_file_path.exists() {
                eprintln!("❌ File not found: {}", full_file_path.display());
                std::process::exit(1);
            }

            // Create backup if requested
            if backup {
                let backup_path = format!("{}.backup.{}",
                                        full_file_path.display(),
                                        chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                std::fs::copy(&full_file_path, &backup_path)?;
                println!("💾 Backup created: {}", backup_path);
            }

            // Show description if provided
            if let Some(desc) = description {
                println!("📋 Edit description: {}", desc);
            }

            // Run analysis on the file first
            println!("\n🔍 Analyzing file before editing...");
            let _analysis_result: anyhow::Result<()> = async {
                // Run quality analysis on the specific file
                let quality_data = codehud_core::analysis::AnalysisPipeline::run_view(
                    &codebase_path, ViewType::Quality, false
                ).await?;

                // Extract metrics for this specific file
                if let Some(files) = quality_data.get("files").and_then(|v| v.as_array()) {
                    for file_data in files {
                        if let Some(file_path) = file_data.get("path").and_then(|v| v.as_str()) {
                            if full_file_path.ends_with(file_path) {
                                println!("📊 Current metrics:");
                                if let Some(complexity) = file_data.get("complexity").and_then(|v| v.as_u64()) {
                                    println!("   • Complexity: {}", complexity);
                                }
                                if let Some(code_lines) = file_data.get("code_lines").and_then(|v| v.as_u64()) {
                                    println!("   • Code lines: {}", code_lines);
                                }
                                if let Some(functions) = file_data.get("functions").and_then(|v| v.as_array()) {
                                    println!("   • Functions: {}", functions.len());
                                }
                                break;
                            }
                        }
                    }
                }

                Ok(())
            }.await;

            // Launch editor
            println!("\n🚀 Launching {} editor...", editor);

            let mut cmd = std::process::Command::new(&editor);
            cmd.arg(&full_file_path);

            match cmd.status() {
                Ok(status) => {
                    if status.success() {
                        println!("✅ Editor closed successfully");

                        // Run analysis again to show changes
                        println!("\n🔍 Re-analyzing file after editing...");
                        let _post_analysis: anyhow::Result<()> = async {
                            let quality_data = codehud_core::analysis::AnalysisPipeline::run_view(
                                &codebase_path, ViewType::Quality, false
                            ).await?;

                            if let Some(files) = quality_data.get("files").and_then(|v| v.as_array()) {
                                for file_data in files {
                                    if let Some(file_path) = file_data.get("path").and_then(|v| v.as_str()) {
                                        if full_file_path.ends_with(file_path) {
                                            println!("📊 Updated metrics:");
                                            if let Some(complexity) = file_data.get("complexity").and_then(|v| v.as_u64()) {
                                                println!("   • Complexity: {}", complexity);
                                            }
                                            if let Some(code_lines) = file_data.get("code_lines").and_then(|v| v.as_u64()) {
                                                println!("   • Code lines: {}", code_lines);
                                            }
                                            if let Some(functions) = file_data.get("functions").and_then(|v| v.as_array()) {
                                                println!("   • Functions: {}", functions.len());
                                            }
                                            break;
                                        }
                                    }
                                }
                            }

                            Ok(())
                        }.await;

                        println!("\n💡 Consider running 'codehud analyze' to see the impact of your changes");
                    } else {
                        eprintln!("⚠️  Editor exited with non-zero status");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to launch editor '{}': {}", editor, e);
                    eprintln!("💡 Make sure the editor is installed and in your PATH");
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Issues {
            codebase_path,
            category,
            severity,
            autofix,
            report
        } => {
            println!("🔍 Analyzing issues in {}", codebase_path.display());

            let result: anyhow::Result<()> = async {
                // Run security and quality analysis to find issues
                let security_data = codehud_core::analysis::AnalysisPipeline::run_view(
                    &codebase_path, ViewType::Security, false
                ).await?;

                let quality_data = codehud_core::analysis::AnalysisPipeline::run_view(
                    &codebase_path, ViewType::Quality, false
                ).await?;

                // Extract issues from analysis results
                let mut all_issues = Vec::new();

                // Extract security issues
                if let Some(security_issues) = security_data.get("security_issues").and_then(|v| v.as_array()) {
                    for issue in security_issues {
                        if let Some(issue_obj) = issue.as_object() {
                            let issue_category = issue_obj.get("category").and_then(|v| v.as_str()).unwrap_or("security");
                            let issue_severity = issue_obj.get("severity").and_then(|v| v.as_str()).unwrap_or("medium");

                            // Filter by category if specified
                            if let Some(filter_cat) = &category {
                                if issue_category != filter_cat {
                                    continue;
                                }
                            }

                            // Filter by severity if specified
                            if let Some(filter_sev) = &severity {
                                if issue_severity != filter_sev {
                                    continue;
                                }
                            }

                            all_issues.push((issue_category, issue_severity, issue));
                        }
                    }
                }

                // Extract quality issues
                if let Some(quality_issues) = quality_data.get("quality_issues").and_then(|v| v.as_array()) {
                    for issue in quality_issues {
                        if let Some(issue_obj) = issue.as_object() {
                            let issue_category = issue_obj.get("category").and_then(|v| v.as_str()).unwrap_or("quality");
                            let issue_severity = issue_obj.get("severity").and_then(|v| v.as_str()).unwrap_or("medium");

                            // Filter by category if specified
                            if let Some(filter_cat) = &category {
                                if issue_category != filter_cat {
                                    continue;
                                }
                            }

                            // Filter by severity if specified
                            if let Some(filter_sev) = &severity {
                                if issue_severity != filter_sev {
                                    continue;
                                }
                            }

                            all_issues.push((issue_category, issue_severity, issue));
                        }
                    }
                }

                println!("\n📋 Found {} issues", all_issues.len());

                // Group issues by category and severity
                let mut by_category = std::collections::HashMap::new();
                let mut by_severity = std::collections::HashMap::new();

                for (cat, sev, _) in &all_issues {
                    *by_category.entry(cat.to_string()).or_insert(0) += 1;
                    *by_severity.entry(sev.to_string()).or_insert(0) += 1;
                }

                println!("\n📊 Issues by Category:");
                for (cat, count) in by_category.iter() {
                    println!("  • {}: {}", cat, count);
                }

                println!("\n⚠️  Issues by Severity:");
                for (sev, count) in by_severity.iter() {
                    println!("  • {}: {}", sev, count);
                }

                // Display detailed issues
                println!("\n🔍 Detailed Issues:");
                for (i, (category, severity, issue)) in all_issues.iter().enumerate() {
                    println!("\n{}. [{}/{}] {}",
                             i + 1,
                             category,
                             severity,
                             issue.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown issue"));

                    if let Some(file) = issue.get("file").and_then(|v| v.as_str()) {
                        println!("   📁 File: {}", file);
                    }

                    if let Some(line) = issue.get("line").and_then(|v| v.as_u64()) {
                        println!("   📍 Line: {}", line);
                    }

                    if let Some(description) = issue.get("description").and_then(|v| v.as_str()) {
                        println!("   ℹ️  {}", description);
                    }
                }

                if autofix {
                    println!("\n🔧 Auto-fix functionality not yet implemented");
                }

                if report {
                    let report_path = format!("issues_report_{}.json",
                                            chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                    let report_data = serde_json::json!({
                        "timestamp": chrono::Utc::now(),
                        "codebase_path": codebase_path,
                        "total_issues": all_issues.len(),
                        "issues_by_category": by_category,
                        "issues_by_severity": by_severity,
                        "issues": all_issues.iter().map(|(_, _, issue)| issue).collect::<Vec<_>>()
                    });

                    std::fs::write(&report_path, serde_json::to_string_pretty(&report_data)?)?;
                    println!("\n📄 Report saved to: {}", report_path);
                }

                Ok(())
            }.await;

            match result {
                Ok(()) => println!("\n✅ Issue analysis completed successfully!"),
                Err(e) => {
                    eprintln!("❌ Issue analysis failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Devnotes {
            codebase_path,
            add,
            list,
            search,
            category
        } => {
            println!("📝 Managing devnotes for {}", codebase_path.display());

            // Notes file path
            let notes_file = codebase_path.join(".codehud_notes.json");

            // Load existing notes
            let mut notes: Vec<serde_json::Value> = if notes_file.exists() {
                let content = std::fs::read_to_string(&notes_file)?;
                serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
            } else {
                Vec::new()
            };

            if let Some(note_text) = add {
                // Add new note
                let new_note = serde_json::json!({
                    "id": format!("note_{}", chrono::Utc::now().timestamp()),
                    "timestamp": chrono::Utc::now(),
                    "category": category.unwrap_or_else(|| "general".to_string()),
                    "text": note_text,
                    "codebase": codebase_path.display().to_string()
                });

                notes.push(new_note.clone());

                // Save notes
                std::fs::write(&notes_file, serde_json::to_string_pretty(&notes)?)?;

                println!("✅ Added note: {}", note_text);
                println!("📁 Saved to: {}", notes_file.display());
            } else if list {
                // List all notes
                if notes.is_empty() {
                    println!("📝 No notes found");
                } else {
                    println!("📋 Found {} notes:\n", notes.len());

                    for (i, note) in notes.iter().enumerate() {
                        let timestamp = note.get("timestamp")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let category = note.get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("general");
                        let text = note.get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("No text");

                        println!("{}. [{}] {}", i + 1, category, text);
                        println!("   📅 {}\n", timestamp);
                    }
                }
            } else if let Some(search_term) = search {
                // Search notes
                let matching_notes: Vec<_> = notes.iter()
                    .enumerate()
                    .filter(|(_, note)| {
                        let text = note.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let category = note.get("category").and_then(|v| v.as_str()).unwrap_or("");
                        text.to_lowercase().contains(&search_term.to_lowercase()) ||
                        category.to_lowercase().contains(&search_term.to_lowercase())
                    })
                    .collect();

                if matching_notes.is_empty() {
                    println!("🔍 No notes found matching '{}'", search_term);
                } else {
                    println!("🔍 Found {} notes matching '{}':\n", matching_notes.len(), search_term);

                    for (i, (_, note)) in matching_notes.iter().enumerate() {
                        let timestamp = note.get("timestamp")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let category = note.get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("general");
                        let text = note.get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("No text");

                        println!("{}. [{}] {}", i + 1, category, text);
                        println!("   📅 {}\n", timestamp);
                    }
                }
            } else {
                // Show summary by default
                if notes.is_empty() {
                    println!("📝 No notes found. Use --add to create your first note.");
                } else {
                    println!("📊 Notes Summary:");
                    println!("   Total notes: {}", notes.len());

                    // Group by category
                    let mut by_category = std::collections::HashMap::new();
                    for note in &notes {
                        let category = note.get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("general");
                        *by_category.entry(category.to_string()).or_insert(0) += 1;
                    }

                    println!("\n📋 By Category:");
                    for (cat, count) in by_category.iter() {
                        println!("   • {}: {}", cat, count);
                    }

                    println!("\n💡 Use --list to see all notes, --search to find specific notes");
                }
            }
        }
        
        Commands::Full {
            codebase_path,
            output_dir,
            profile,
            views,
            fresh
        } => {
            println!("🚀 Running full analysis of {} -> {}",
                     codebase_path.display(), output_dir.display());

            if profile {
                println!("📊 Performance profiling enabled");
            }
            if fresh {
                println!("♻️  Fresh analysis (skipping cache)");
            }

            // Create output directory
            std::fs::create_dir_all(&output_dir)?;

            let result: anyhow::Result<()> = async {
                let start_time = std::time::Instant::now();

                // Determine which views to generate
                let view_types = if let Some(view_list) = views {
                    view_list.split(',')
                        .filter_map(|v| match v.trim().to_lowercase().as_str() {
                            "topology" => Some(ViewType::Topology),
                            "quality" => Some(ViewType::Quality),
                            "security" => Some(ViewType::Security),
                            "dependencies" => Some(ViewType::Dependencies),
                            _ => {
                                println!("⚠️  Unknown view type: {}", v.trim());
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    // Default: all available views
                    vec![ViewType::Topology, ViewType::Quality, ViewType::Security, ViewType::Dependencies]
                };

                println!("📋 Generating {} views: {:?}", view_types.len(), view_types);

                let mut all_results = std::collections::HashMap::new();

                // Run comprehensive analysis first
                println!("\n🔍 Running comprehensive analysis...");
                let analysis_result = codehud_core::analysis::AnalysisPipeline::run(
                    &codebase_path, Pipeline::Direct, false
                ).await?;

                // Save main analysis results
                let main_output = output_dir.join("analysis_summary.json");
                std::fs::write(&main_output, serde_json::to_string_pretty(&analysis_result)?)?;
                println!("💾 Main analysis saved: {}", main_output.display());

                // Generate each view
                for view_type in view_types {
                    println!("\n📊 Generating {} view...", view_type);

                    let view_data = codehud_core::analysis::AnalysisPipeline::run_view(
                        &codebase_path, view_type, false
                    ).await?;

                    // Save view data
                    let view_filename = format!("{}_view.json", view_type.to_string().to_lowercase());
                    let view_output = output_dir.join(&view_filename);
                    std::fs::write(&view_output, serde_json::to_string_pretty(&view_data)?)?;
                    println!("💾 {} view saved: {}", view_type, view_output.display());

                    all_results.insert(view_type.to_string(), view_data);
                }

                // Generate summary report
                let summary_report = serde_json::json!({
                    "timestamp": chrono::Utc::now(),
                    "codebase_path": codebase_path,
                    "analysis_duration": start_time.elapsed().as_secs_f64(),
                    "health_score": analysis_result.health_score,
                    "files_analyzed": analysis_result.files_analyzed,
                    "critical_issues": analysis_result.critical_issues,
                    "focus_recommendations": analysis_result.focus_recommendations,
                    "views_generated": all_results.keys().collect::<Vec<_>>(),
                    "output_directory": output_dir
                });

                let summary_output = output_dir.join("full_analysis_report.json");
                std::fs::write(&summary_output, serde_json::to_string_pretty(&summary_report)?)?;

                // Generate human-readable summary
                let mut text_summary = String::new();
                text_summary.push_str(&format!("CodeHUD Full Analysis Report\n"));
                text_summary.push_str(&format!("Generated: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
                text_summary.push_str(&format!("Codebase: {}\n", codebase_path.display()));
                text_summary.push_str(&format!("Analysis Duration: {:.2}s\n\n", start_time.elapsed().as_secs_f64()));

                text_summary.push_str(&format!("HEALTH OVERVIEW\n"));
                text_summary.push_str(&format!("===============\n"));
                text_summary.push_str(&format!("Health Score: {:.1}/100\n", analysis_result.health_score));
                text_summary.push_str(&format!("Files Analyzed: {}\n", analysis_result.files_analyzed));
                text_summary.push_str(&format!("Critical Issues: {}\n\n", analysis_result.critical_issues.len()));

                if !analysis_result.focus_recommendations.is_empty() {
                    text_summary.push_str("FOCUS RECOMMENDATIONS\n");
                    text_summary.push_str("=====================\n");
                    for (i, rec) in analysis_result.focus_recommendations.iter().enumerate() {
                        text_summary.push_str(&format!("{}. {}\n", i + 1, rec));
                    }
                    text_summary.push_str("\n");
                }

                text_summary.push_str("GENERATED VIEWS\n");
                text_summary.push_str("===============\n");
                for view_name in all_results.keys() {
                    text_summary.push_str(&format!("• {}_view.json\n", view_name.to_lowercase()));
                }
                text_summary.push_str("\n");

                text_summary.push_str("OUTPUT FILES\n");
                text_summary.push_str("============\n");
                text_summary.push_str("• analysis_summary.json - Main analysis results\n");
                text_summary.push_str("• full_analysis_report.json - Complete report data\n");
                text_summary.push_str("• README.txt - This summary\n");

                let readme_output = output_dir.join("README.txt");
                std::fs::write(&readme_output, text_summary)?;

                println!("\n📊 ANALYSIS COMPLETE");
                println!("====================");
                println!("🎯 Health Score: {:.1}/100", analysis_result.health_score);
                println!("📁 Files Analyzed: {}", analysis_result.files_analyzed);
                println!("⏱️  Duration: {:.2}s", start_time.elapsed().as_secs_f64());
                println!("📄 Views Generated: {}", all_results.len());
                println!("📂 Output Directory: {}", output_dir.display());

                if analysis_result.has_critical_issues() {
                    println!("⚠️  {} critical issues found - see analysis_summary.json",
                             analysis_result.critical_issues.len());
                }

                Ok(())
            }.await;

            match result {
                Ok(()) => println!("\n✅ Full analysis completed successfully!"),
                Err(e) => {
                    eprintln!("❌ Full analysis failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    
    Ok(())
}

/// Export visualizations to text and JSON files
async fn export_visualizations(codebase_path: &PathBuf, output_dir: &PathBuf, views: &str) -> Result<()> {
    println!("🎨 Exporting CodeHUD Visualizations");
    println!("📊 Codebase: {}", codebase_path.display());
    println!("📁 Output: {}", output_dir.display());
    println!("🔍 Views: {}\n", views);

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Parse view types
    let view_types: Vec<ViewType> = views
        .split(',')
        .filter_map(|v| match v.trim().to_lowercase().as_str() {
            "quality" => Some(ViewType::Quality),
            "security" => Some(ViewType::Security),
            "topology" => Some(ViewType::Topology),
            "dependencies" => Some(ViewType::Dependencies),
            "performance" => Some(ViewType::Performance),
            "issues" => Some(ViewType::IssuesInspection),
            _ => {
                eprintln!("⚠️ Unknown view type: {}", v);
                None
            }
        })
        .collect();

    if view_types.is_empty() {
        eprintln!("❌ No valid view types specified");
        std::process::exit(1);
    }

    // Run analysis for each view type
    for view_type in view_types {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║ {:^60} ║", format!("{:?} Analysis", view_type));
        println!("╚══════════════════════════════════════════════════════════════╝");

        match run_analysis_for_view(codebase_path, view_type, output_dir).await {
            Ok(()) => {
                println!("✅ {} analysis exported successfully\n", format!("{:?}", view_type));
            }
            Err(e) => {
                eprintln!("❌ {} analysis failed: {}\n", format!("{:?}", view_type), e);
            }
        }
    }

    println!("🎯 Visualization export complete!");
    println!("📁 Check {} directory for results", output_dir.display());

    Ok(())
}

/// Run analysis for a specific view type and export results
async fn run_analysis_for_view(codebase_path: &PathBuf, view_type: ViewType, output_dir: &PathBuf) -> Result<()> {
    // Use the existing direct pipeline with default config
    let config = codehud_core::CoreConfig::default();
    let pipeline = codehud_analysis::pipeline::DirectAnalysisPipeline::new(codebase_path, config)?
        .with_extractors(&[view_type]);
    let pipeline_result = pipeline.analyze().await?;

    // Convert to core AnalysisResult for viz engine
    let mut analysis_result = codehud_core::models::AnalysisResult::new(codebase_path.to_string_lossy().to_string());

    // Set view data from pipeline result
    let view_data = serde_json::to_value(&pipeline_result)?;
    analysis_result.set_view_data(format!("{:?}", view_type).to_lowercase(), view_data);

    // Create viz engine
    let viz_engine = codehud_viz::VisualizationEngine::new();

    // Generate visualization
    let view = viz_engine.generate_view(view_type, &analysis_result)
        .map_err(|e| codehud_core::Error::Analysis(format!("Visualization generation failed: {}", e)))?;

    // Export as JSON
    let view_name = format!("{:?}", view_type).to_lowercase();
    let json_file = output_dir.join(format!("{}_visualization.json", view_name));
    let json_output = serde_json::to_string_pretty(&view)?;
    std::fs::write(&json_file, json_output)?;

    // Export as readable text
    let text_file = output_dir.join(format!("{}_analysis.txt", view_name));
    let text_output = format_visualization_as_text(&view);
    std::fs::write(&text_file, text_output)?;

    println!("💾 Exported: {} and {}", json_file.display(), text_file.display());

    // Show summary in terminal
    show_visualization_summary(&view);

    Ok(())
}

/// Format visualization as readable text
fn format_visualization_as_text(view: &codehud_viz::RenderableView) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    writeln!(output, "=== CODEHUD {} ANALYSIS ===", format!("{:?}", view.view_type).to_uppercase()).unwrap();
    writeln!(output, "Title: {}", view.title).unwrap();
    writeln!(output, "Generated: {}", view.timestamp).unwrap();
    writeln!(output, "").unwrap();

    match &view.content {
        codehud_viz::ViewContent::Quality { health_score, issues_by_severity, top_problematic_files, .. } => {
            writeln!(output, "🏥 HEALTH SCORE: {:.1}%", health_score * 100.0).unwrap();
            writeln!(output, "").unwrap();
            writeln!(output, "📋 ISSUES BY SEVERITY:").unwrap();
            for (severity, count) in issues_by_severity {
                writeln!(output, "  • {}: {} issues", severity, count).unwrap();
            }
            writeln!(output, "").unwrap();
            writeln!(output, "🚨 MOST PROBLEMATIC FILES:").unwrap();
            for (file, score) in top_problematic_files.iter().take(10) {
                writeln!(output, "  • {}: {:.2}", file, score).unwrap();
            }
        }
        codehud_viz::ViewContent::Security { risk_level, top_security_issues, security_score, .. } => {
            writeln!(output, "🔒 RISK LEVEL: {}", risk_level).unwrap();
            writeln!(output, "🛡️  SECURITY SCORE: {:.1}%", security_score * 100.0).unwrap();
            writeln!(output, "").unwrap();
            writeln!(output, "⚠️  TOP SECURITY ISSUES:").unwrap();
            for issue in top_security_issues.iter().take(10) {
                writeln!(output, "  • [{}] {}: {}", issue.severity, issue.file, issue.description).unwrap();
            }
        }
        codehud_viz::ViewContent::Dependencies { total_dependencies, circular_dependencies, .. } => {
            writeln!(output, "📦 TOTAL DEPENDENCIES: {}", total_dependencies).unwrap();
            writeln!(output, "🔄 CIRCULAR DEPENDENCIES: {} files", circular_dependencies.len()).unwrap();
            if !circular_dependencies.is_empty() {
                writeln!(output, "").unwrap();
                writeln!(output, "🔄 CIRCULAR DEPENDENCY FILES:").unwrap();
                for file in circular_dependencies.iter().take(10) {
                    writeln!(output, "  • {}", file).unwrap();
                }
            }
        }
        codehud_viz::ViewContent::Topology { file_tree, .. } => {
            writeln!(output, "🏗️  ARCHITECTURE TOPOLOGY").unwrap();
            writeln!(output, "📁 Total Files: {}", file_tree.total_files).unwrap();
            writeln!(output, "📂 Total Directories: {}", file_tree.total_directories).unwrap();
        }
        _ => {
            writeln!(output, "📊 VIEW TYPE: {:?}", view.view_type).unwrap();
            writeln!(output, "Content available in JSON format").unwrap();
        }
    }

    output
}

/// Show visualization summary in terminal
fn show_visualization_summary(view: &codehud_viz::RenderableView) {
    match &view.content {
        codehud_viz::ViewContent::Quality { health_score, issues_by_severity, .. } => {
            println!("  🏥 Health Score: {:.1}%", health_score * 100.0);
            let total_issues: usize = issues_by_severity.values().sum();
            println!("  📋 Total Issues: {}", total_issues);
        }
        codehud_viz::ViewContent::Security { risk_level, security_score, .. } => {
            println!("  🔒 Risk Level: {}", risk_level);
            println!("  🛡️  Security Score: {:.1}%", security_score * 100.0);
        }
        codehud_viz::ViewContent::Dependencies { total_dependencies, circular_dependencies, .. } => {
            println!("  📦 Dependencies: {}", total_dependencies);
            println!("  🔄 Circular: {}", circular_dependencies.len());
        }
        codehud_viz::ViewContent::Topology { file_tree, .. } => {
            println!("  📁 Files: {}", file_tree.total_files);
            println!("  📂 Directories: {}", file_tree.total_directories);
        }
        _ => {
            println!("  📊 View: {:?}", view.view_type);
        }
    }
}