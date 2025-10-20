//! CodeHUD Analysis - Pipeline and Rendering System
//!
//! This crate provides the analysis pipeline and rendering system for CodeHUD,
//! implementing both direct and legacy analysis modes with comprehensive
//! markdown export capabilities.

#![warn(clippy::all, clippy::pedantic)]

pub mod pipeline;
pub mod health_score;

// Re-export main types for convenience
pub use pipeline::{
    DirectAnalysisPipeline, AnalysisResult, AnalysisMetadata, 
    ExtractorPerformance, SystemInfo, AnalysisExporter
};
pub use health_score::{HealthScoreCalculator, HealthScore};

/// Result type for analysis operations
pub type Result<T> = std::result::Result<T, codehud_core::Error>;