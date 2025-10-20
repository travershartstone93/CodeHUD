//! Complexity Analysis Thresholds
//!
//! Constants for measuring and categorizing code complexity metrics.
//! These thresholds help identify overly complex code that may need refactoring.
//!
//! This is a 1:1 translation from Python src/codehud/constants/complexity_thresholds.py
//! to ensure zero degradation in complexity analysis behavior.

use serde::{Deserialize, Serialize};

/// General code complexity thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplexityThresholds;

impl ComplexityThresholds {
    /// Base complexity thresholds
    pub const LOW_COMPLEXITY: i32 = 5;              // Low complexity threshold
    pub const MODERATE_COMPLEXITY: i32 = 10;        // Moderate complexity threshold  
    pub const HIGH_COMPLEXITY: i32 = 15;            // High complexity threshold
    pub const EXCESSIVE_COMPLEXITY: i32 = 25;       // Excessive complexity threshold
    pub const CRITICAL_COMPLEXITY: i32 = 50;        // Critical complexity threshold

    /// Function-specific thresholds
    pub const FUNCTION_WARNING_COMPLEXITY: i32 = 10;  // Function complexity warning
    pub const FUNCTION_CRITICAL_COMPLEXITY: i32 = 20; // Function complexity critical

    /// Class-specific thresholds  
    pub const CLASS_WARNING_COMPLEXITY: i32 = 25;    // Class complexity warning
    pub const CLASS_CRITICAL_COMPLEXITY: i32 = 50;   // Class complexity critical

    /// Get complexity status description
    /// 
    /// # Arguments
    /// * `complexity` - The complexity score to evaluate
    /// 
    /// # Returns
    /// A string describing the complexity level
    pub fn get_complexity_status(complexity: i32) -> &'static str {
        match complexity {
            c if c <= Self::LOW_COMPLEXITY => "Low",
            c if c <= Self::MODERATE_COMPLEXITY => "Moderate", 
            c if c <= Self::HIGH_COMPLEXITY => "High",
            c if c <= Self::EXCESSIVE_COMPLEXITY => "Excessive",
            _ => "Critical",
        }
    }

    /// Determine if code needs refactoring based on complexity
    /// 
    /// # Arguments
    /// * `complexity` - The complexity score to evaluate
    /// * `context` - The context ("function", "class", or other)
    /// 
    /// # Returns
    /// `true` if the code needs refactoring, `false` otherwise
    pub fn needs_refactoring(complexity: i32, context: &str) -> bool {
        match context {
            "function" => complexity > Self::FUNCTION_CRITICAL_COMPLEXITY,
            "class" => complexity > Self::CLASS_CRITICAL_COMPLEXITY,
            _ => complexity > Self::EXCESSIVE_COMPLEXITY,
        }
    }
}

/// Cyclomatic complexity specific thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CyclomaticComplexityThresholds;

impl CyclomaticComplexityThresholds {
    /// McCabe complexity thresholds (widely accepted industry standards)
    pub const SIMPLE_COMPLEXITY: i32 = 1;           // Simple, no branching
    pub const LOW_RISK: i32 = 10;                   // Low risk, simple procedure
    pub const MODERATE_RISK: i32 = 20;              // Moderate risk, complex procedure
    pub const HIGH_RISK: i32 = 50;                  // High risk, very complex procedure
    pub const UNTESTABLE: i32 = 100;                // Untestable, extremely complex

    /// Maintenance thresholds
    pub const EASY_TO_MAINTAIN: i32 = 5;            // Easy to understand and maintain
    pub const MAINTAINABLE: i32 = 10;               // Maintainable with effort
    pub const DIFFICULT_TO_MAINTAIN: i32 = 20;      // Difficult to maintain
    pub const ERROR_PRONE: i32 = 30;                // Highly error-prone

    /// Get risk level based on cyclomatic complexity
    /// 
    /// # Arguments
    /// * `cyclomatic_complexity` - The cyclomatic complexity score
    /// 
    /// # Returns
    /// A string describing the risk level
    pub fn get_risk_level(cyclomatic_complexity: i32) -> &'static str {
        match cyclomatic_complexity {
            c if c <= Self::SIMPLE_COMPLEXITY => "No Risk",
            c if c <= Self::LOW_RISK => "Low Risk",
            c if c <= Self::MODERATE_RISK => "Moderate Risk", 
            c if c <= Self::HIGH_RISK => "High Risk",
            _ => "Very High Risk",
        }
    }

    /// Get maintainability assessment based on cyclomatic complexity
    /// 
    /// # Arguments
    /// * `cyclomatic_complexity` - The cyclomatic complexity score
    /// 
    /// # Returns
    /// A string describing the maintainability level
    pub fn get_maintainability(cyclomatic_complexity: i32) -> &'static str {
        match cyclomatic_complexity {
            c if c <= Self::EASY_TO_MAINTAIN => "Easy to Maintain",
            c if c <= Self::MAINTAINABLE => "Maintainable",
            c if c <= Self::DIFFICULT_TO_MAINTAIN => "Difficult to Maintain",
            c if c <= Self::ERROR_PRONE => "Error Prone", 
            _ => "Unmaintainable",
        }
    }

    /// Determine if complexity requires immediate attention
    /// 
    /// # Arguments
    /// * `cyclomatic_complexity` - The cyclomatic complexity score
    /// 
    /// # Returns
    /// `true` if complexity requires immediate attention
    pub fn requires_immediate_attention(cyclomatic_complexity: i32) -> bool {
        cyclomatic_complexity > Self::HIGH_RISK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_status() {
        assert_eq!(ComplexityThresholds::get_complexity_status(3), "Low");
        assert_eq!(ComplexityThresholds::get_complexity_status(8), "Moderate");
        assert_eq!(ComplexityThresholds::get_complexity_status(12), "High");
        assert_eq!(ComplexityThresholds::get_complexity_status(30), "Excessive");
        assert_eq!(ComplexityThresholds::get_complexity_status(60), "Critical");
    }

    #[test]
    fn test_needs_refactoring() {
        // Function context
        assert!(!ComplexityThresholds::needs_refactoring(15, "function"));
        assert!(ComplexityThresholds::needs_refactoring(25, "function"));
        
        // Class context
        assert!(!ComplexityThresholds::needs_refactoring(40, "class"));
        assert!(ComplexityThresholds::needs_refactoring(60, "class"));
        
        // Default context
        assert!(!ComplexityThresholds::needs_refactoring(20, "module"));
        assert!(ComplexityThresholds::needs_refactoring(30, "module"));
    }

    #[test]
    fn test_risk_level() {
        assert_eq!(CyclomaticComplexityThresholds::get_risk_level(1), "No Risk");
        assert_eq!(CyclomaticComplexityThresholds::get_risk_level(5), "Low Risk");
        assert_eq!(CyclomaticComplexityThresholds::get_risk_level(15), "Moderate Risk");
        assert_eq!(CyclomaticComplexityThresholds::get_risk_level(40), "High Risk");
        assert_eq!(CyclomaticComplexityThresholds::get_risk_level(80), "Very High Risk");
    }

    #[test]
    fn test_maintainability() {
        assert_eq!(CyclomaticComplexityThresholds::get_maintainability(3), "Easy to Maintain");
        assert_eq!(CyclomaticComplexityThresholds::get_maintainability(8), "Maintainable");
        assert_eq!(CyclomaticComplexityThresholds::get_maintainability(18), "Difficult to Maintain");
        assert_eq!(CyclomaticComplexityThresholds::get_maintainability(25), "Error Prone");
        assert_eq!(CyclomaticComplexityThresholds::get_maintainability(50), "Unmaintainable");
    }

    #[test]
    fn test_requires_immediate_attention() {
        assert!(!CyclomaticComplexityThresholds::requires_immediate_attention(30));
        assert!(CyclomaticComplexityThresholds::requires_immediate_attention(60));
    }

    // Property-based tests to ensure mathematical equivalence with Python
    #[cfg(feature = "proptest")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn complexity_status_consistency(complexity in 0i32..1000) {
                let status = ComplexityThresholds::get_complexity_status(complexity);
                // Verify status matches expected boundaries
                match status {
                    "Low" => prop_assert!(complexity <= ComplexityThresholds::LOW_COMPLEXITY),
                    "Moderate" => prop_assert!(complexity > ComplexityThresholds::LOW_COMPLEXITY && 
                                             complexity <= ComplexityThresholds::MODERATE_COMPLEXITY),
                    "High" => prop_assert!(complexity > ComplexityThresholds::MODERATE_COMPLEXITY && 
                                          complexity <= ComplexityThresholds::HIGH_COMPLEXITY),
                    "Excessive" => prop_assert!(complexity > ComplexityThresholds::HIGH_COMPLEXITY && 
                                               complexity <= ComplexityThresholds::EXCESSIVE_COMPLEXITY),
                    "Critical" => prop_assert!(complexity > ComplexityThresholds::EXCESSIVE_COMPLEXITY),
                    _ => prop_assert!(false, "Invalid status returned"),
                }
            }
        }
    }
}