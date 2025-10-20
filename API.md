# CodeHUD API Reference

This document provides comprehensive API documentation for the CodeHUD Rust implementation.

## 📚 Core Architecture

### Workspace Structure
```
codehud/
├── codehud-core/          # Core analysis engine
├── codehud-analysis/      # Analysis pipeline orchestration
├── codehud-cli/           # Command-line interface
├── codehud-gui/           # Desktop GUI application
├── codehud-tui/           # Terminal user interface
├── codehud-viz/           # Visualization engine
├── codehud-llm/           # LLM integration
├── codehud-transform/     # Code transformation tools
├── codehud-realtime/      # Real-time file monitoring
└── codehud-utils/         # Shared utilities
```

## 🔧 Core API (`codehud-core`)

### Analysis Pipeline

#### `AnalysisPipeline`
Main entry point for code analysis operations.

```rust
pub struct DirectAnalysisPipeline {
    config: CoreConfig,
}

impl DirectAnalysisPipeline {
    /// Create a new analysis pipeline
    pub fn new(config: CoreConfig) -> Self;

    /// Run comprehensive analysis on a codebase
    pub async fn run_comprehensive_analysis(
        codebase_path: &Path,
        config: &CoreConfig
    ) -> Result<HashMap<String, Value>>;

    /// Generate a specific view of analysis data
    pub async fn run_view(
        codebase_path: &Path,
        view_type: ViewType,
        debug: bool
    ) -> Result<Value>;
}
```

**Usage Example**:
```rust
use codehud_core::analysis::AnalysisPipeline;
use codehud_core::models::ViewType;

let result = AnalysisPipeline::run_view(
    Path::new("./src"),
    ViewType::Topology,
    false
).await?;
```

### Data Extractors

#### `BaseDataExtractor` Trait
Common interface for all data extractors.

```rust
pub trait BaseDataExtractor {
    /// Extract analysis data from the codebase
    fn extract_data(&self) -> Result<HashMap<String, Value>>;
}
```

#### Available Extractors

##### `TopologyExtractor`
Analyzes codebase structure and dependencies.

```rust
pub struct TopologyExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
}

impl TopologyExtractor {
    /// Create new topology extractor
    pub fn new(codebase_path: impl AsRef<Path>) -> Result<Self>;

    /// Get source files for analysis
    fn get_source_files(&self) -> Vec<PathBuf>;

    /// Analyze file dependencies
    fn analyze_file_dependencies(&mut self, file_path: &Path) -> Result<Value>;
}
```

**Output Structure**:
```json
{
  "coupling": {
    "average_dependencies": 3.93,
    "total_dependencies": 656,
    "highly_coupled_files": [
      ["file_path", dependency_count]
    ]
  },
  "dependencies": {
    "file_path": ["dep1", "dep2", "dep3"]
  },
  "files": [
    {
      "path": "src/lib.rs",
      "dependencies": ["std", "serde"],
      "functions": [...],
      "complexity": 42
    }
  ]
}
```

##### `QualityExtractor`
Analyzes code quality metrics and issues.

```rust
pub struct QualityExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
}

impl QualityExtractor {
    /// Create new quality extractor
    pub fn new(codebase_path: impl AsRef<Path>) -> Result<Self>;

    /// Analyze file quality metrics
    async fn analyze_file_quality(&mut self, file_path: &Path) -> Result<Option<Value>>;
}
```

**Rust Quality Metrics**:
```rust
pub struct RustQualityMetrics {
    pub unsafe_blocks: usize,
    pub result_usage: usize,
    pub option_usage: usize,
    pub unwrap_calls: usize,
    pub expect_calls: usize,
    pub question_mark_operators: usize,
    pub lifetime_annotations: usize,
    pub trait_implementations: usize,
}
```

##### `SecurityExtractor`
Identifies security vulnerabilities and risks.

```rust
pub struct SecurityExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
    external_tools: ExternalToolManager,
    dangerous_functions: Vec<&'static str>,
    sensitive_patterns: Vec<(Regex, &'static str, &'static str)>,
}

impl SecurityExtractor {
    /// Create new security extractor
    pub fn new(codebase_path: impl AsRef<Path>) -> Result<Self>;

    /// Run Rust-specific security analysis
    fn run_rust_security_analysis(&self, source_files: &[PathBuf]) -> Result<Value>;
}
```

##### `DependenciesExtractor`
Analyzes dependency relationships and coupling.

```rust
pub struct DependenciesExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    parser: Parser,
}

impl DependenciesExtractor {
    /// Create new dependencies extractor
    pub fn new(codebase_path: impl AsRef<Path>) -> Result<Self>;

    /// Detect circular dependencies
    fn detect_circular_dependencies(&self, import_graph: &HashMap<String, HashSet<String>>) -> Vec<Vec<String>>;

    /// Analyze coupling strength
    fn analyze_coupling_strength(&self, import_graph: &HashMap<String, HashSet<String>>, internal_imports: &HashMap<String, HashSet<String>>) -> f64;
}
```

### External Tool Integration

#### `ExternalToolManager`
Manages integration with external analysis tools.

```rust
pub struct ExternalToolManager {
    pub ruff_integration: ruff::RuffIntegration,
    pub pylint_integration: pylint::PylintIntegration,
    pub mypy_integration: mypy::MypyIntegration,
    pub bandit_integration: bandit::BanditIntegration,
    pub coverage_integration: coverage::CoverageIntegration,
    pub git_integration: git::GitIntegration,
    pub ripgrep_integration: ripgrep::RipgrepTool,
    tool_availability: HashMap<String, bool>,
    codebase_path: PathBuf,
}
```

#### `RustToolManager`
Specialized tool manager for Rust-specific tools.

```rust
pub struct RustToolManager {
    pub clippy_integration: clippy::ClippyIntegration,
    pub cargo_audit_integration: cargo_audit::CargoAuditIntegration,
    pub cargo_test_integration: cargo_test::CargoTestIntegration,
    pub rustfmt_integration: rustfmt::RustfmtIntegration,
    pub git_integration: git::GitIntegration,
    pub ripgrep_integration: ripgrep::RipgrepTool,
    tool_availability: HashMap<String, bool>,
    codebase_path: PathBuf,
}

impl RustToolManager {
    /// Create new Rust tool manager
    pub fn new(codebase_path: impl AsRef<Path>) -> Self;

    /// Check availability of all Rust tools
    pub async fn check_tool_availability(&mut self) -> Result<()>;

    /// Run clippy analysis
    pub async fn run_clippy_analysis(&self) -> Result<clippy::ClippyResult>;

    /// Run cargo audit security scan
    pub async fn run_cargo_audit(&self) -> Result<cargo_audit::AuditResult>;
}
```

#### Rust Tool Integrations

##### Clippy Integration
```rust
pub struct ClippyIntegration {
    codebase_path: PathBuf,
}

impl ClippyIntegration {
    /// Run clippy analysis
    pub async fn analyze_codebase(&self) -> Result<ClippyResult>;
}

pub struct ClippyResult {
    pub issues: Vec<ClippyIssue>,
    pub total_issues: usize,
    pub warnings: usize,
    pub errors: usize,
}
```

##### Cargo Audit Integration
```rust
pub struct CargoAuditIntegration {
    codebase_path: PathBuf,
}

impl CargoAuditIntegration {
    /// Run security audit
    pub async fn analyze_dependencies(&self) -> Result<AuditResult>;
}

pub struct AuditResult {
    pub vulnerabilities: Vec<Vulnerability>,
    pub warnings: Vec<Advisory>,
    pub total_dependencies: usize,
}
```

## 🎨 Visualization API (`codehud-viz`)

### Visualization Engine

#### `VisualizationEngine`
Main visualization coordinator.

```rust
pub struct VisualizationEngine {
    config: VizConfig,
}

impl VisualizationEngine {
    /// Create new visualization engine
    pub fn new() -> Self;

    /// Generate view from analysis result
    pub fn generate_view(
        view_type: ViewType,
        analysis_result: &AnalysisResult
    ) -> Result<RenderableView>;

    /// Render view to terminal
    pub fn render_to_terminal<B: Backend>(
        frame: &mut Frame<B>,
        view: &RenderableView
    );
}
```

#### View Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewType {
    Summary,
    Topology,
    Quality,
    Security,
    Dependencies,
    Performance,
    Evolution,
    Issues,
    Testing,
    Flow,
}
```

#### Renderable Views
```rust
pub struct RenderableView {
    pub title: String,
    pub view_type: ViewType,
    pub content: ViewContent,
    pub metadata: ViewMetadata,
}

pub enum ViewContent {
    Summary(SummaryContent),
    Topology(TopologyContent),
    Quality(QualityContent),
    Security(SecurityContent),
    Dependencies(DependenciesContent),
    // ... other content types
}
```

## 🖥️ CLI API (`codehud-cli`)

### Command Structure

#### Main CLI Interface
```rust
#[derive(Parser)]
#[command(name = "codehud")]
#[command(about = "CodeHUD - Comprehensive code analysis platform")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze codebase with specific view
    Analyze {
        /// Path to codebase directory
        path: PathBuf,

        /// Analysis view type
        #[arg(short, long)]
        view: ViewType,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable debug output
        #[arg(short, long)]
        debug: bool,
    },

    /// Run comprehensive analysis
    Full {
        /// Path to codebase directory
        path: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Views to generate
        #[arg(long)]
        views: Option<String>,
    },
}
```

### Usage Examples

#### Basic Analysis
```bash
# Analyze with specific view
cargo run --bin codehud -- analyze ./src --view topology --output result.json

# Full analysis with multiple views
cargo run --bin codehud -- full ./src --output-dir ./analysis_results/
```

#### Programmatic Usage
```rust
use codehud_cli::{Cli, Commands};
use clap::Parser;

let cli = Cli::parse();
match cli.command {
    Commands::Analyze { path, view, output, debug } => {
        // Handle analysis command
    },
    Commands::Full { path, output_dir, views } => {
        // Handle full analysis command
    },
}
```

## 🖼️ GUI API (`codehud-gui`)

### Application Structure

#### Main Application
```rust
pub struct CodeHudApp {
    state: AppState,
    components: HashMap<String, Box<dyn GuiComponent>>,
}

impl eframe::App for CodeHudApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame);
}
```

#### Component System
```rust
pub trait GuiComponent: Send + Sync {
    fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState) -> GuiResult<()>;
    fn handle_message(&mut self, message: GuiMessage) -> GuiResult<()>;
}
```

#### Available Components
- **TopologyViewComponent**: Dependency graph visualization
- **QualityDashboardComponent**: Quality metrics dashboard
- **SecurityViewComponent**: Security analysis results
- **MetricsPanelComponent**: Real-time metrics display

## 📡 Real-time API (`codehud-realtime`)

### File Monitoring

#### `RealtimeAnalyzer`
Provides real-time file change monitoring and analysis.

```rust
pub struct RealtimeAnalyzer {
    codebase_path: PathBuf,
    config: RealtimeConfig,
    analysis_pipeline: Box<dyn AnalysisPipeline>,
}

impl RealtimeAnalyzer {
    /// Create new real-time analyzer
    pub fn new(codebase_path: PathBuf, config: RealtimeConfig) -> Result<Self>;

    /// Start monitoring file changes
    pub async fn start_monitoring(&mut self) -> Result<()>;

    /// Process file change events
    pub async fn process_events(&mut self, events: Vec<Event>) -> Result<()>;
}
```

## 🔄 Transform API (`codehud-transform`)

### Code Transformation

#### `TransformationEngine`
Handles code refactoring and transformation operations.

```rust
pub struct TransformationEngine {
    config: TransformConfig,
    rollback_system: RollbackSystem,
}

impl TransformationEngine {
    /// Apply transformation to codebase
    pub async fn apply_transformation(
        &mut self,
        transformation: Box<dyn Transformer>,
        target: &Path
    ) -> Result<TransformationResult>;
}

pub trait Transformer: Send + Sync {
    async fn transform(&self, input: TransformInput) -> Result<TransformationResult>;
}
```

## 🧠 LLM API (`codehud-llm`)

### AI Integration

#### `LlmEngine`
Provides AI-powered analysis and suggestions.

```rust
pub trait LlmEngine: Send + Sync {
    async fn analyze_code(&self, request: LlmRequest) -> LlmResult<LlmResponse>;
    async fn generate_suggestions(&self, context: AnalysisContext) -> LlmResult<Vec<Suggestion>>;
}

pub struct NativeLlmEngine {
    config: LlmConfig,
    model: Box<dyn LanguageModel>,
}
```

## 📊 Models and Data Structures

### Core Models

#### `AnalysisResult`
Main container for analysis results.

```rust
pub struct AnalysisResult {
    pub codebase_path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub view_type: ViewType,
    pub data: HashMap<String, Value>,
    pub metadata: AnalysisMetadata,
}
```

#### `ViewType`
Enumeration of available analysis views.

```rust
impl ViewType {
    pub fn all_views() -> Vec<ViewType>;
    pub fn from_str(s: &str) -> Result<ViewType>;
    pub fn to_string(&self) -> String;
}
```

### Error Handling

#### `CodeHudError`
Comprehensive error type for all operations.

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("External tool error: {0}")]
    ExternalTool(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

---

This API reference provides comprehensive documentation for integrating with and extending the CodeHUD Rust implementation. For more specific examples and advanced usage patterns, refer to the source code and integration tests.