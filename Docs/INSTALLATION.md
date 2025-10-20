# CodeHUD Installation Guide

This guide covers the complete installation process for the CodeHUD Rust implementation.

## 📋 Prerequisites

### Required Dependencies
- **Rust 1.70+**: The latest stable Rust toolchain
- **Cargo**: Rust package manager (included with Rust installation)
- **Git**: Version control system for cloning the repository

### System Requirements
- **OS**: Windows, macOS, or Linux
- **RAM**: Minimum 4GB (8GB+ recommended for large codebases)
- **Storage**: ~500MB for compilation and dependencies
- **Network**: Internet connection for dependency downloads

## 🚀 Quick Installation

### 1. Install Rust Toolchain
```bash
# Install Rust via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart your shell or run:
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Clone Repository
```bash
git clone <repository-url>
cd CodeHUD/Rust_copy
```

### 3. Build CodeHUD
```bash
# Build all components (debug mode)
cargo build

# Or build optimized release version
cargo build --release
```

### 4. Install External Tools
```bash
# Essential security scanning tool
cargo install cargo-audit

# Optional: Install additional Rust tools
rustup component add clippy rustfmt
```

## 🔧 Detailed Installation

### Development Environment Setup

#### IDE/Editor Configuration
**VS Code (Recommended)**:
```bash
# Install rust-analyzer extension
code --install-extension rust-lang.rust-analyzer
```

**Other Editors**:
- **IntelliJ IDEA**: Install Rust plugin
- **Vim/Neovim**: Use rust.vim + coc-rust-analyzer
- **Emacs**: Use rust-mode + lsp-mode

#### Additional Components
```bash
# Add useful Rust components
rustup component add rustfmt clippy rust-src rust-docs

# Install development tools
cargo install cargo-watch cargo-edit cargo-tree
```

### External Tool Dependencies

#### Core Analysis Tools
```bash
# Security auditing (Required)
cargo install cargo-audit

# Code formatting verification (Recommended)
rustup component add rustfmt

# Static analysis (Recommended)
rustup component add clippy
```

#### Optional Enhancement Tools
```bash
# Advanced dependency analysis
cargo install cargo-deps

# Performance profiling
cargo install cargo-flamegraph

# Documentation generation
cargo install cargo-doc-api
```

## ⚙️ Configuration

### Environment Variables
```bash
# Optional: Set Rust backtrace for debugging
export RUST_BACKTRACE=1

# Optional: Set log level for detailed output
export RUST_LOG=debug
```

### Cargo Configuration
Create `~/.cargo/config.toml`:
```toml
[build]
# Use multiple CPU cores for faster builds
jobs = 4

[net]
# Use sparse registry for faster dependency resolution
git-fetch-with-cli = true
```

## 📦 Component-Specific Installation

### Core Analysis Engine
```bash
# Build just the core components
cargo build -p codehud-core -p codehud-analysis
```

### Command Line Interface
```bash
# Build CLI tool
cargo build --bin codehud --release

# Install globally (optional)
cargo install --path codehud-cli
```

### Graphical User Interface
```bash
# Build GUI (requires additional system dependencies)
cargo build -p codehud-gui --release

# Linux: Install GUI dependencies
sudo apt install libgtk-3-dev libxcb-shape0-dev libxcb-xfixes0-dev

# macOS: No additional dependencies needed

# Windows: Ensure Visual Studio Build Tools are installed
```

### Terminal User Interface
```bash
# Build TUI
cargo build -p codehud-tui --release
```

## 🧪 Verification

### Test Installation
```bash
# Run all tests
cargo test

# Test specific component
cargo test -p codehud-core

# Run integration tests
cargo test --test integration_tests
```

### Verify Functionality
```bash
# Test CLI analysis
cargo run --bin codehud -- analyze . --view topology

# Test external tool integration
cargo audit --version
cargo clippy --version
cargo fmt --version
```

### Performance Verification
```bash
# Run benchmark analysis
cargo run --release --bin codehud -- analyze . --view topology --output test_output.json

# Verify output
ls -la test_output.json
```

## 🐛 Troubleshooting

### Common Issues

#### Build Failures
**Issue**: Compilation errors with tree-sitter
```bash
# Solution: Clean and rebuild
cargo clean
cargo build
```

**Issue**: Linking errors on Linux
```bash
# Install development packages
sudo apt install build-essential pkg-config libssl-dev
```

#### Runtime Issues
**Issue**: "Cannot find library" errors
```bash
# Update LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

**Issue**: Permission denied errors
```bash
# Ensure proper file permissions
chmod +x target/release/codehud
```

### Platform-Specific Issues

#### Windows
- **MSVC Build Tools**: Install Visual Studio Build Tools
- **Git Bash**: Use Git Bash for Unix-like commands
- **Path Issues**: Ensure Cargo bin directory is in PATH

#### macOS
- **Xcode Command Line Tools**: `xcode-select --install`
- **Homebrew Dependencies**: Use Homebrew for system libraries

#### Linux
- **System Libraries**: Install development packages for your distribution
- **Permissions**: Ensure user has access to necessary directories

## 🚀 Performance Optimization

### Build Optimizations
```bash
# Enable link-time optimization
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Use release profile with debug info
cargo build --profile release-with-debug
```

### Runtime Optimizations
```bash
# Set optimal stack size
export RUST_MIN_STACK=8388608

# Enable parallel compilation
export CARGO_BUILD_JOBS=8
```

## 📊 Installation Verification

### Successful Installation Checklist
- [ ] Rust toolchain installed and updated
- [ ] Repository cloned successfully
- [ ] All components build without errors
- [ ] External tools installed and accessible
- [ ] CLI analysis runs successfully
- [ ] Test suite passes
- [ ] Sample analysis generates correct output

### Expected Output
After successful installation, you should be able to run:
```bash
cargo run --bin codehud -- analyze . --view topology
```

And see output similar to:
```
🔍 Analyzing . with topology view using direct pipeline
[INFO] Extracting topology data from .

=== TOPOLOGY ANALYSIS RESULTS ===
{
  "coupling": {
    "average_dependencies": 3.93,
    "total_dependencies": 656
  },
  "files": [...]
}
```

## 🔄 Updates and Maintenance

### Keeping CodeHUD Updated
```bash
# Pull latest changes
git pull origin main

# Update dependencies
cargo update

# Rebuild with latest changes
cargo build --release
```

### Updating External Tools
```bash
# Update cargo-audit
cargo install cargo-audit --force

# Update Rust components
rustup update
```

---

For additional help or issues not covered here, please refer to the troubleshooting section or open an issue in the project repository.