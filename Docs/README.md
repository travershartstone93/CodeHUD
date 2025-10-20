**CodeHUD** is a comprehensive code analysis and visualization platform that provides deep insights into codebase health, dependencies, security vulnerabilities, and architectural patterns. This Rust implementation offers robust analysis with Rust-specific enhancements for performance, safety, and usability. I mostly use it as a Claude Code attachment, I dont even use the CLI TUI or GUI. I vibe coded it, it took about 5 months and I prototyped it in python, there are bugs, its my first Rust project. 

## Key Features

### Core Analysis Capabilities

* **Topology Analysis**: Codebase structure, coupling metrics, and dependency graphs
* **Quality Assessment**: Code quality metrics, complexity analysis, and maintainability scores
* **Security Analysis**: Vulnerability detection, unsafe pattern identification, and risk assessment
* **Dependency Analysis**: Import analysis, circular dependency detection, and coupling measurement
* **Performance Profiling**: Runtime analysis, bottleneck detection, and optimization recommendations

### Rust-Specific Enhancements

* **Unsafe Block Detection**: Identifies and tracks unsafe code blocks for security review
* **Panic Pattern Analysis**: Detects `.unwrap()` and `.expect()` calls that may cause panics
* **Memory Safety Analysis**: Tracks Result/Option usage and error handling patterns
* **Ownership Analysis**: Monitors lifetime annotations and borrowing patterns

### External Tool Integration

* **Clippy**: Static analysis and linting
* **Rustfmt**: Code formatting validation
* **Cargo Audit**: Security vulnerability scanning
* **Cargo Test**: Test execution and coverage
* **Tree-sitter**: Advanced AST parsing

## Architecture

CodeHUD is organized as a modular Rust workspace:

### Core Components

* **`codehud-core`**: Core analysis engine and extractors
* **`codehud-analysis`**: Analysis pipeline orchestration
* **`codehud-cli`**: Command-line interface
* **`codehud-gui`**: Desktop GUI application (egui-based)
* **`codehud-tui`**: Terminal user interface (ratatui-based)
* **`codehud-viz`**: Visualization and rendering engine

### Specialized Components

* **`codehud-llm`**: AI-powered code summarization
* **`codehud-transform`**: Code transformation and refactoring tools
* **`codehud-realtime`**: Real-time file monitoring and analysis
* **`codehud-utils`**: Shared utilities and helpers

## Installation

### Prerequisites

* **Rust 1.70+**: Latest stable Rust toolchain
* **Cargo**: Package manager (included with Rust)
* **Git**: Version control system

### Quick Install

```bash
# Clone the repository
git clone <repository-url>
cd CodeHUD

# Build all components
cargo build --release

# Install external tools
cargo install cargo-audit
```

### Optional Dependencies

```bash
# For enhanced analysis capabilities
rustup component add clippy rustfmt
```

## Usage

### Command Line Interface

```bash
# Basic analysis
cargo run --bin codehud -- analyze . --view topology

# Comprehensive analysis with output
cargo run --bin codehud -- analyze . --view topology --output analysis.json

# Multi-view analysis
cargo run --bin codehud -- full . --output-dir results/

# Available views
cargo run --bin codehud -- analyze . --view [topology|quality|security|dependencies]
```

### Call Graph & Dependency Visualization

```bash
# Install Graphviz for visualization
sudo apt install graphviz    # Ubuntu/Debian
brew install graphviz        # macOS

# Generate call graphs automatically
cargo run --bin codehud -- call-graph /path/to/codebase

# Customize output format and layout
cargo run --bin codehud -- call-graph . -o my_graph -f svg -l auto
```

**Multi-View Output:**

1. **Overview Graph**: Module-level architecture
2. **Per-Module Graphs**: Function-level call graphs with complexity coloring
3. **Cycle Detection Graph**: Highlights circular dependencies

**Polyglot Support:**

* Rust (Cargo.toml dependencies)
* JavaScript/TypeScript, Python, Java, C/C++, Go, Ruby, PHP, C#, Swift, Kotlin
* Automatic language detection, import/external dependency separation, cycle detection, and complexity coloring

### Graphical User Interface

```bash
# Launch desktop GUI
cargo run --bin codehud-gui

# Launch terminal UI
cargo run --bin codehud-tui
```

**GUI Features:**

* Multiple analysis views: Topology, Quality, Health, Metrics, Dependencies, Tests, Documentation
* Interactive call graph visualizations
* Real-time analysis and live updates
* Project management for multiple codebases

### AI-Powered Summarization

* Generates hierarchical summaries of codebases using multi-pass reasoning
* Supports local and cloud AI engines (e.g., Google Gemini)
* Produces detailed file, module, and project-level summaries

## Analysis Output

**Topology Analysis**: File dependencies, coupling metrics, module hierarchy, circular dependency detection
**Quality Analysis**: Maintainability scores, complexity analysis, unsafe/error patterns
**Security Analysis**: Vulnerability detection, panic potential, dependency scanning

## Development

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run linting
cargo clippy

# Format code
cargo fmt
```

**Always use `cargo run`** to ensure binaries are built with the latest changes.

```bash
cargo run --bin codehud -- analyze . --view topology
```

## Performance Metrics

* **Files Processed**: 169 Rust source files
* **Dependencies Tracked**: 656 total dependencies
* **External Dependencies**: 721 crate dependencies scanned
* **Security Issues**: 2 vulnerabilities + 11 warnings
* **Analysis Speed**: Sub-second for most views
