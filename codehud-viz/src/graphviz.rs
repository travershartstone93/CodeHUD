//! Graphviz Integration for Graph Rendering
//!
//! Provides functions to render DOT format graphs to various output formats
//! using the Graphviz `dot` command-line tool.

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Result, Context, bail};

/// Output format for rendered graphs
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// Scalable Vector Graphics (recommended for LLM analysis)
    Svg,
    /// Portable Network Graphics
    Png,
    /// Portable Document Format
    Pdf,
}

impl OutputFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
        }
    }

    /// Get Graphviz format string
    fn graphviz_format(&self) -> &str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "svg" => Ok(Self::Svg),
            "png" => Ok(Self::Png),
            "pdf" => Ok(Self::Pdf),
            _ => bail!("Unknown output format: {}. Supported: svg, png, pdf", s),
        }
    }
}

/// Layout engine for graph rendering
#[derive(Debug, Clone, Copy)]
pub enum LayoutEngine {
    /// Hierarchical layout (best for call graphs and dependencies)
    Dot,
    /// Spring model layout (good for small graphs)
    Neato,
    /// Force-directed placement (good for large graphs)
    Fdp,
    /// Scalable force-directed placement (best for huge graphs)
    Sfdp,
    /// Circular layout
    Circo,
    /// Radial layout
    Twopi,
}

impl LayoutEngine {
    /// Get command name for this layout engine
    fn command(&self) -> &str {
        match self {
            Self::Dot => "dot",
            Self::Neato => "neato",
            Self::Fdp => "fdp",
            Self::Sfdp => "sfdp",
            Self::Circo => "circo",
            Self::Twopi => "twopi",
        }
    }
}

impl std::fmt::Display for LayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command())
    }
}

impl std::str::FromStr for LayoutEngine {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "dot" => Ok(Self::Dot),
            "neato" => Ok(Self::Neato),
            "fdp" => Ok(Self::Fdp),
            "sfdp" => Ok(Self::Sfdp),
            "circo" => Ok(Self::Circo),
            "twopi" => Ok(Self::Twopi),
            _ => bail!("Unknown layout engine: {}. Supported: dot, neato, fdp, sfdp, circo, twopi", s),
        }
    }
}

/// Check if Graphviz is installed and available
pub fn check_graphviz_installed() -> Result<String> {
    let output = Command::new("dot")
        .arg("-V")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stderr);
                Ok(version.trim().to_string())
            } else {
                bail!("Graphviz 'dot' command failed")
            }
        }
        Err(_) => {
            bail!(
                "Graphviz not installed. Install with:\n  \
                 Ubuntu/Debian: sudo apt install graphviz\n  \
                 macOS: brew install graphviz\n  \
                 Windows: choco install graphviz\n  \
                 Or download from: https://graphviz.org/download/"
            )
        }
    }
}

/// Render DOT content to a file using Graphviz
///
/// # Arguments
/// * `dot_content` - The DOT format graph content
/// * `output_path` - Output path (without extension, will be added based on format)
/// * `format` - Output format (SVG, PNG, PDF)
/// * `layout` - Layout engine to use (default: Dot)
///
/// # Returns
/// Path to the generated output file
pub fn render_dot_to_file(
    dot_content: &str,
    output_path: &Path,
    format: OutputFormat,
    layout: Option<LayoutEngine>,
) -> Result<PathBuf> {
    // Write DOT file
    let dot_file = output_path.with_extension("dot");
    std::fs::write(&dot_file, dot_content)
        .with_context(|| format!("Failed to write DOT file: {}", dot_file.display()))?;

    // Determine layout engine
    let layout_engine = layout.unwrap_or(LayoutEngine::Dot);
    let layout_cmd = layout_engine.command();

    // Render with Graphviz
    let output_file = output_path.with_extension(format.extension());

    let status = Command::new(layout_cmd)
        .arg(format!("-T{}", format.graphviz_format()))
        .arg(&dot_file)
        .arg("-o")
        .arg(&output_file)
        .status()
        .with_context(|| format!("Failed to execute Graphviz '{}' command", layout_cmd))?;

    if !status.success() {
        bail!("Graphviz rendering failed with non-zero exit code");
    }

    Ok(output_file)
}

/// Render DOT content directly to a string (SVG only)
pub fn render_dot_to_string(
    dot_content: &str,
    layout: Option<LayoutEngine>,
) -> Result<String> {
    let layout_engine = layout.unwrap_or(LayoutEngine::Dot);
    let layout_cmd = layout_engine.command();

    let mut child = Command::new(layout_cmd)
        .arg("-Tsvg")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn Graphviz '{}' process", layout_cmd))?;

    // Write DOT content to stdin
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(dot_content.as_bytes())
            .context("Failed to write DOT content to Graphviz stdin")?;
    }

    let result = child.wait_with_output()
        .context("Failed to wait for Graphviz process")?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!("Graphviz rendering failed: {}", stderr);
    }

    String::from_utf8(result.stdout)
        .context("Graphviz output is not valid UTF-8")
}

/// Get list of available Graphviz layout engines on this system
pub fn list_available_engines() -> Vec<String> {
    let engines = [
        "dot", "neato", "fdp", "sfdp", "circo", "twopi",
        "osage", "patchwork", // Additional engines
    ];

    engines
        .iter()
        .filter(|&&cmd| {
            Command::new(cmd)
                .arg("-V")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|&s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert!(matches!("svg".parse::<OutputFormat>(), Ok(OutputFormat::Svg)));
        assert!(matches!("png".parse::<OutputFormat>(), Ok(OutputFormat::Png)));
        assert!(matches!("pdf".parse::<OutputFormat>(), Ok(OutputFormat::Pdf)));
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_layout_engine_parsing() {
        assert!(matches!("dot".parse::<LayoutEngine>(), Ok(LayoutEngine::Dot)));
        assert!(matches!("neato".parse::<LayoutEngine>(), Ok(LayoutEngine::Neato)));
        assert!("invalid".parse::<LayoutEngine>().is_err());
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Svg.to_string(), "svg");
        assert_eq!(OutputFormat::Png.to_string(), "png");
        assert_eq!(OutputFormat::Pdf.to_string(), "pdf");
    }

    #[test]
    fn test_layout_engine_display() {
        assert_eq!(LayoutEngine::Dot.to_string(), "dot");
        assert_eq!(LayoutEngine::Sfdp.to_string(), "sfdp");
    }

    // Note: Actual rendering tests would require Graphviz to be installed
    // These would be integration tests rather than unit tests
}
