# Development Guide

This guide covers development workflows, contribution guidelines, and technical details for CodeHUD contributors.

## 🛠️ Development Setup

### Prerequisites
- **Rust 1.70+**: Latest stable toolchain
- **IDE**: VS Code with rust-analyzer (recommended)
- **Git**: Version control system
- **External Tools**: `cargo install cargo-audit`

### Quick Start
```bash
# Clone and setup
git clone <repository-url>
cd CodeHUD/Rust_copy

# Install development tools
rustup component add clippy rustfmt rust-src
cargo install cargo-watch cargo-edit

# Build and test
cargo build
cargo test
```

## 🏗️ Project Structure

### Workspace Architecture
```
Rust_copy/
├── Cargo.toml                 # Workspace configuration
├── codehud-core/              # Core analysis engine
│   ├── src/
│   │   ├── extractors/        # Data extraction modules
│   │   ├── external_tools/    # Tool integrations
│   │   ├── graph/             # Graph analysis algorithms
│   │   ├── models/            # Data structures
│   │   └── analysis/          # Pipeline orchestration
│   └── Cargo.toml
├── codehud-cli/               # Command-line interface
├── codehud-gui/               # Desktop GUI (egui)
├── codehud-viz/               # Visualization engine
└── docs/                      # Documentation files
```

### Key Components

#### Core Engine (`codehud-core`)
**Purpose**: Central analysis engine and data extraction
**Key Files**:
- `src/extractors/mod.rs` - Extractor framework
- `src/analysis/pipeline.rs` - Analysis orchestration
- `src/external_tools/mod.rs` - Tool integration management

#### CLI (`codehud-cli`)
**Purpose**: Command-line interface and batch processing
**Key Files**:
- `src/main.rs` - CLI entry point and argument parsing
- `src/direct.rs` - Direct analysis commands

#### GUI (`codehud-gui`)
**Purpose**: Desktop application with rich visualizations
**Key Files**:
- `src/main.rs` - GUI application entry point
- `src/components/` - UI component modules

## 🔄 Development Workflow

### Daily Development
```bash
# Start development with auto-rebuild
cargo watch -x "build" -x "test"

# Run specific component
cargo run --bin codehud -- analyze . --view topology

# Run tests with output
cargo test -- --nocapture

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check
```

### Testing Strategy
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_tests

# Component-specific tests
cargo test -p codehud-core

# Test with external tools
cargo test test_external_tools -- --ignored
```

### Code Quality
```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check documentation
cargo doc --no-deps --open

# Security audit
cargo audit
```

## 📝 Adding New Features

### Adding a New Extractor

1. **Create Extractor Module**
```rust
// codehud-core/src/extractors/my_extractor.rs
use super::BaseDataExtractor;
use crate::Result;
use std::collections::HashMap;
use serde_json::Value;

pub struct MyExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
}

impl BaseDataExtractor for MyExtractor {
    fn extract_data(&self) -> Result<HashMap<String, Value>> {
        // Implementation here
        Ok(HashMap::new())
    }
}
```

2. **Register in Module**
```rust
// codehud-core/src/extractors/mod.rs
pub mod my_extractor;
pub use my_extractor::MyExtractor;
```

3. **Add to Pipeline**
```rust
// codehud-core/src/analysis/pipeline.rs
let my_extractor = MyExtractor::new(&codebase_path)?;
let my_data = my_extractor.extract_data()?;
```

4. **Add Tests**
```rust
// codehud-core/src/extractors/my_extractor.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_my_extractor() {
        // Test implementation
    }
}
```

### Adding External Tool Integration

1. **Create Tool Module**
```rust
// codehud-core/src/external_tools/my_tool.rs
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MyToolResult {
    pub issues: Vec<MyToolIssue>,
    pub summary: String,
}

pub struct MyToolIntegration {
    codebase_path: PathBuf,
}

impl MyToolIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }

    pub async fn analyze(&self) -> Result<MyToolResult> {
        // Tool execution logic
    }
}
```

2. **Add to Tool Manager**
```rust
// codehud-core/src/external_tools/mod.rs
pub mod my_tool;

pub struct RustToolManager {
    // ... existing tools
    pub my_tool_integration: my_tool::MyToolIntegration,
}
```

### Adding CLI Commands

1. **Extend CLI Structure**
```rust
// codehud-cli/src/main.rs
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands
    MyCommand {
        #[arg(short, long)]
        option: String,
    },
}
```

2. **Implement Command Handler**
```rust
// codehud-cli/src/main.rs
match cli.command {
    Commands::MyCommand { option } => {
        // Command implementation
    },
}
```

## 🧪 Testing Guidelines

### Test Organization
```
tests/
├── integration/           # Integration tests
├── fixtures/             # Test data
└── common/               # Shared test utilities
```

### Writing Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_async_functionality() {
        // Async test implementation
    }

    #[test]
    fn test_sync_functionality() {
        // Sync test implementation
    }
}
```

### Test Data Management
```rust
// Use tempfile for isolated test environments
#[test]
fn test_with_temp_directory() {
    let temp_dir = tempdir().unwrap();
    let test_path = temp_dir.path();

    // Create test files
    std::fs::write(test_path.join("test.rs"), "fn main() {}").unwrap();

    // Run test
    let result = my_function(test_path);
    assert!(result.is_ok());
}
```

## 🐛 Debugging

### Debug Configuration
```rust
// Enable debug logging
RUST_LOG=debug cargo run

// Enable backtraces
RUST_BACKTRACE=1 cargo run

// Detailed backtraces
RUST_BACKTRACE=full cargo run
```

### Common Debug Patterns
```rust
// Use tracing for structured logging
use tracing::{debug, info, warn, error};

debug!("Processing file: {}", file_path.display());
info!("Analysis complete: {} files processed", file_count);
```

### Performance Profiling
```bash
# Install profiling tools
cargo install cargo-flamegraph

# Generate flame graph
cargo flamegraph --bin codehud -- analyze . --view topology

# Memory profiling with valgrind
valgrind --tool=massif --stacks=yes target/release/codehud analyze . --view topology
```

## 📊 Benchmarking

### Performance Tests
```rust
// benches/analysis_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_topology_analysis(c: &mut Criterion) {
    c.bench_function("topology_analysis", |b| {
        b.iter(|| {
            // Benchmark code
        });
    });
}

criterion_group!(benches, benchmark_topology_analysis);
criterion_main!(benches);
```

### Running Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench topology_analysis

# Generate detailed reports
cargo bench -- --output-format html
```

## 📚 Documentation

### Code Documentation
```rust
/// Analyzes the topology of a Rust codebase
///
/// This function performs comprehensive topology analysis including:
/// - File dependency mapping
/// - Coupling metrics calculation
/// - Module structure analysis
///
/// # Arguments
///
/// * `codebase_path` - Path to the codebase root directory
/// * `config` - Analysis configuration options
///
/// # Returns
///
/// Returns `TopologyResult` containing analysis data or an error
///
/// # Examples
///
/// ```rust
/// use codehud_core::analysis::analyze_topology;
///
/// let result = analyze_topology("./src", &config).await?;
/// println!("Found {} dependencies", result.total_dependencies);
/// ```
pub async fn analyze_topology(
    codebase_path: &Path,
    config: &AnalysisConfig
) -> Result<TopologyResult> {
    // Implementation
}
```

### Generating Documentation
```bash
# Generate documentation
cargo doc --no-deps

# Open documentation in browser
cargo doc --no-deps --open

# Generate documentation with private items
cargo doc --no-deps --document-private-items
```

## 🚀 Release Process

### Version Management
```bash
# Update version in Cargo.toml files
cargo edit set-version 1.1.0

# Update CHANGELOG.md with new features
# Tag release
git tag v1.1.0
git push origin v1.1.0
```

### Release Checklist
- [ ] All tests passing
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version numbers bumped
- [ ] Security audit clean
- [ ] Performance benchmarks acceptable

## 🤝 Contributing

### Pull Request Process
1. **Fork and Branch**
```bash
git checkout -b feature/my-new-feature
```

2. **Develop and Test**
```bash
cargo test
cargo clippy
cargo fmt
```

3. **Update Documentation**
- Add/update API documentation
- Update README if needed
- Add changelog entry

4. **Submit PR**
- Clear description of changes
- Reference related issues
- Include test results

### Code Style Guidelines
- **Follow Rust conventions**: Use `rustfmt` and `clippy`
- **Error handling**: Use `Result<T, E>` consistently
- **Documentation**: Document public APIs
- **Testing**: Include tests for new functionality

## 🔧 Troubleshooting

### Common Issues
**Build Failures**:
```bash
# Clean build
cargo clean && cargo build

# Update dependencies
cargo update
```

**Test Failures**:
```bash
# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --exact
```

**External Tool Issues**:
```bash
# Check tool availability
cargo audit --version
cargo clippy --version

# Reinstall tools
cargo install cargo-audit --force
```

---

This development guide should help contributors understand the codebase structure and development workflows for the CodeHUD Rust implementation.