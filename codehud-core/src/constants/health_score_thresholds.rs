//! Health Score and Quality Thresholds
//!
//! Constants for calculating and categorizing codebase health metrics.
//! These thresholds determine the boundaries between different health levels
//! and quality categories.
//!
//! This is a 1:1 translation from Python src/codehud/constants/health_score_thresholds.py
//! to ensure zero degradation in health scoring behavior.

use serde::{Deserialize, Serialize};

/// Thresholds for overall codebase health scoring
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HealthScoreThresholds;

impl HealthScoreThresholds {
    /// Health score percentage thresholds (0.0 to 1.0)
    pub const HEALTH_SCORE_80_PERCENT: f64 = 0.8;    // Excellent health threshold
    pub const HEALTH_SCORE_60_PERCENT: f64 = 0.6;    // Good health threshold
    pub const HEALTH_SCORE_40_PERCENT: f64 = 0.4;    // Acceptable health threshold
    pub const HEALTH_SCORE_20_PERCENT: f64 = 0.2;    // Poor health threshold

    /// Alternative named thresholds for readability
    pub const EXCELLENT_THRESHOLD: f64 = Self::HEALTH_SCORE_80_PERCENT;
    pub const GOOD_THRESHOLD: f64 = Self::HEALTH_SCORE_60_PERCENT;
    pub const ACCEPTABLE_THRESHOLD: f64 = Self::HEALTH_SCORE_40_PERCENT;
    pub const POOR_THRESHOLD: f64 = Self::HEALTH_SCORE_20_PERCENT;

    /// Get health status description based on score
    /// 
    /// # Arguments
    /// * `health_score` - The health score to evaluate (0.0 to 1.0)
    /// 
    /// # Returns
    /// A string describing the health status with emoji
    pub fn get_health_status(health_score: f64) -> &'static str {
        match health_score {
            score if score >= Self::EXCELLENT_THRESHOLD => "Excellent ✅",
            score if score >= Self::GOOD_THRESHOLD => "Good 👍",
            score if score >= Self::ACCEPTABLE_THRESHOLD => "Needs Attention ⚠️",
            _ => "Critical Issues ❌",
        }
    }

    /// Get color coding for health score display
    /// 
    /// # Arguments
    /// * `health_score` - The health score to evaluate (0.0 to 1.0)
    /// 
    /// # Returns
    /// A string representing the color for display
    pub fn get_health_color(health_score: f64) -> &'static str {
        match health_score {
            score if score >= Self::EXCELLENT_THRESHOLD => "green",
            score if score >= Self::GOOD_THRESHOLD => "blue",
            score if score >= Self::ACCEPTABLE_THRESHOLD => "yellow",
            _ => "red",
        }
    }

    /// Determine if health score requires immediate attention
    /// 
    /// # Arguments
    /// * `health_score` - The health score to evaluate (0.0 to 1.0)
    /// 
    /// # Returns
    /// `true` if the health score requires immediate attention
    pub fn requires_attention(health_score: f64) -> bool {
        health_score < Self::ACCEPTABLE_THRESHOLD
    }
}

/// Thresholds for code quality metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QualityThresholds;

impl QualityThresholds {
    /// Test coverage thresholds
    pub const EXCELLENT_COVERAGE: f64 = 0.9;         // 90% coverage or above
    pub const GOOD_COVERAGE: f64 = 0.8;              // 80% coverage threshold
    pub const ACCEPTABLE_COVERAGE: f64 = 0.7;        // 70% coverage threshold
    pub const MINIMUM_COVERAGE: f64 = 0.5;           // 50% minimum coverage

    /// Code duplication thresholds
    pub const LOW_DUPLICATION: f64 = 0.05;           // 5% duplication or less
    pub const MODERATE_DUPLICATION: f64 = 0.1;       // 10% duplication threshold
    pub const HIGH_DUPLICATION: f64 = 0.15;          // 15% duplication threshold

    /// Documentation coverage thresholds
    pub const EXCELLENT_DOCUMENTATION: f64 = 0.85;   // 85% documentation coverage
    pub const GOOD_DOCUMENTATION: f64 = 0.7;         // 70% documentation coverage
    pub const MINIMUM_DOCUMENTATION: f64 = 0.5;      // 50% minimum documentation

    /// Get coverage status description
    /// 
    /// # Arguments
    /// * `coverage` - The coverage ratio to evaluate (0.0 to 1.0)
    /// 
    /// # Returns
    /// A string describing the coverage status
    pub fn get_coverage_status(coverage: f64) -> &'static str {
        match coverage {
            c if c >= Self::EXCELLENT_COVERAGE => "Excellent",
            c if c >= Self::GOOD_COVERAGE => "Good",
            c if c >= Self::ACCEPTABLE_COVERAGE => "Acceptable",
            c if c >= Self::MINIMUM_COVERAGE => "Needs Improvement",
            _ => "Critical",
        }
    }

    /// Get code duplication status description
    /// 
    /// # Arguments
    /// * `duplication` - The duplication ratio to evaluate (0.0 to 1.0)
    /// 
    /// # Returns
    /// A string describing the duplication status
    pub fn get_duplication_status(duplication: f64) -> &'static str {
        match duplication {
            d if d <= Self::LOW_DUPLICATION => "Low",
            d if d <= Self::MODERATE_DUPLICATION => "Moderate",
            d if d <= Self::HIGH_DUPLICATION => "High",
            _ => "Excessive",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert_eq!(HealthScoreThresholds::get_health_status(0.9), "Excellent ✅");
        assert_eq!(HealthScoreThresholds::get_health_status(0.7), "Good 👍");
        assert_eq!(HealthScoreThresholds::get_health_status(0.5), "Needs Attention ⚠️");
        assert_eq!(HealthScoreThresholds::get_health_status(0.1), "Critical Issues ❌");
    }

    #[test]
    fn test_health_color() {
        assert_eq!(HealthScoreThresholds::get_health_color(0.9), "green");
        assert_eq!(HealthScoreThresholds::get_health_color(0.7), "blue");
        assert_eq!(HealthScoreThresholds::get_health_color(0.5), "yellow");
        assert_eq!(HealthScoreThresholds::get_health_color(0.1), "red");
    }

    #[test]
    fn test_requires_attention() {
        assert!(!HealthScoreThresholds::requires_attention(0.5));
        assert!(HealthScoreThresholds::requires_attention(0.3));
    }

    #[test]
    fn test_coverage_status() {
        assert_eq!(QualityThresholds::get_coverage_status(0.95), "Excellent");
        assert_eq!(QualityThresholds::get_coverage_status(0.85), "Good");
        assert_eq!(QualityThresholds::get_coverage_status(0.75), "Acceptable");
        assert_eq!(QualityThresholds::get_coverage_status(0.65), "Needs Improvement");
        assert_eq!(QualityThresholds::get_coverage_status(0.3), "Critical");
    }

    #[test]
    fn test_duplication_status() {
        assert_eq!(QualityThresholds::get_duplication_status(0.02), "Low");
        assert_eq!(QualityThresholds::get_duplication_status(0.08), "Moderate");
        assert_eq!(QualityThresholds::get_duplication_status(0.13), "High");
        assert_eq!(QualityThresholds::get_duplication_status(0.25), "Excessive");
    }

    // Boundary tests to ensure exact Python equivalence
    #[test]
    fn test_boundary_values() {
        // Health score boundaries
        assert_eq!(HealthScoreThresholds::get_health_status(0.8), "Excellent ✅");
        assert_eq!(HealthScoreThresholds::get_health_status(0.7999), "Good 👍");
        assert_eq!(HealthScoreThresholds::get_health_status(0.6), "Good 👍");
        assert_eq!(HealthScoreThresholds::get_health_status(0.5999), "Needs Attention ⚠️");
        
        // Coverage boundaries  
        assert_eq!(QualityThresholds::get_coverage_status(0.9), "Excellent");
        assert_eq!(QualityThresholds::get_coverage_status(0.8999), "Good");
        assert_eq!(QualityThresholds::get_coverage_status(0.8), "Good");
        assert_eq!(QualityThresholds::get_coverage_status(0.7999), "Acceptable");
    }

    // Property-based tests for mathematical equivalence
    #[cfg(feature = "proptest")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn health_score_consistency(score in 0.0f64..1.0) {
                let status = HealthScoreThresholds::get_health_status(score);
                let color = HealthScoreThresholds::get_health_color(score);
                
                // Verify status and color alignment
                match status {
                    "Excellent ✅" => {
                        prop_assert_eq!(color, "green");
                        prop_assert!(score >= HealthScoreThresholds::EXCELLENT_THRESHOLD);
                    },
                    "Good 👍" => {
                        prop_assert_eq!(color, "blue");
                        prop_assert!(score >= HealthScoreThresholds::GOOD_THRESHOLD);
                        prop_assert!(score < HealthScoreThresholds::EXCELLENT_THRESHOLD);
                    },
                    "Needs Attention ⚠️" => {
                        prop_assert_eq!(color, "yellow");
                        prop_assert!(score >= HealthScoreThresholds::ACCEPTABLE_THRESHOLD);
                        prop_assert!(score < HealthScoreThresholds::GOOD_THRESHOLD);
                    },
                    "Critical Issues ❌" => {
                        prop_assert_eq!(color, "red");
                        prop_assert!(score < HealthScoreThresholds::ACCEPTABLE_THRESHOLD);
                    },
                    _ => prop_assert!(false, "Invalid health status returned"),
                }
            }
        }
    }
}