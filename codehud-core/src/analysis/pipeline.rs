//! Analysis Pipeline Implementation
//!
//! Handles different analysis pipeline types (direct, legacy, etc.)

use crate::{Result, Pipeline};
use crate::models::analysis_result::AnalysisResult;
use super::AnalysisOrchestrator;
use std::path::Path;

pub struct AnalysisPipeline;

impl AnalysisPipeline {
    /// Run analysis using the specified pipeline
    pub async fn run(codebase_path: impl AsRef<Path>, pipeline: Pipeline, debug: bool) -> Result<AnalysisResult> {
        let orchestrator = AnalysisOrchestrator::new(codebase_path, pipeline)?
            .with_debug(debug);

        match pipeline {
            Pipeline::Direct => {
                // Fast, direct extraction pipeline
                orchestrator.analyze().await
            },
            Pipeline::Legacy => {
                // Legacy pipeline with full processing (future implementation)
                // For now, use the same pipeline but could be extended
                orchestrator.analyze().await
            },
            Pipeline::Hybrid => {
                // Hybrid analysis pipeline (future implementation)
                orchestrator.analyze().await
            },
        }
    }

    /// Run analysis for a specific view only
    pub async fn run_view(codebase_path: impl AsRef<Path>,
                         view_type: crate::ViewType,
                         debug: bool) -> Result<serde_json::Value> {
        let orchestrator = AnalysisOrchestrator::new(codebase_path, Pipeline::Direct)?
            .with_debug(debug);

        orchestrator.generate_view(view_type).await
    }
}