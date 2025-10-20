//! Constants module for CodeHUD core
//!
//! This module contains all threshold values, configuration constants,
//! and other static values used throughout the analysis engine.

pub mod complexity_thresholds;
pub mod health_score_thresholds;

pub use complexity_thresholds::{ComplexityThresholds, CyclomaticComplexityThresholds};
pub use health_score_thresholds::{HealthScoreThresholds, QualityThresholds};