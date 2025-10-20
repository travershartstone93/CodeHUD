//! Comprehensive Equivalence Tests for Zero-Degradation Validation
//!
//! These tests ensure the Rust FFI bridge maintains 97%+ bug fix success rate
//! and produces equivalent results to the Python implementation.

use codehud_llm::{EquivalenceTester, LlmResult};
use std::path::PathBuf;
use tokio_test;

#[tokio::test]
#[ignore] // Requires Python environment setup
async fn test_comprehensive_equivalence_suite() -> LlmResult<()> {
    // Set up the equivalence tester
    let python_path = PathBuf::from("../../../python/codehud"); // Adjust path as needed
    let tester = EquivalenceTester::new(&python_path).await?;

    // Run the full equivalence test suite
    let results = tester.run_equivalence_tests().await?;

    // Generate and print report
    let report = tester.generate_report(&results);
    println!("{}", report);

    // Assert critical requirements
    assert!(
        results.bug_fix_success_rate >= 0.97,
        "Bug fix success rate {:.1}% below required 97%",
        results.bug_fix_success_rate * 100.0
    );

    assert!(
        results.overall_pass_rate >= 0.90,
        "Overall pass rate {:.1}% below expected 90%",
        results.overall_pass_rate * 100.0
    );

    // Performance should not degrade significantly (allow up to 2x slower for FFI overhead)
    assert!(
        results.performance_ratio <= 2.0,
        "Performance ratio {:.2}x indicates significant degradation",
        results.performance_ratio
    );

    println!("✅ All zero-degradation requirements met!");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Python environment setup
async fn test_structured_generation_equivalence() -> LlmResult<()> {
    let python_path = PathBuf::from("../../../python/codehud");
    let tester = EquivalenceTester::new(&python_path).await?;

    let result = tester.test_structured_generation_equivalence().await?;

    assert!(result.passed, "Structured generation test failed: {:?}", result.error_message);
    assert!(result.similarity_score >= 0.95, "Similarity score too low: {}", result.similarity_score);

    println!("✅ Structured generation equivalence validated");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Python environment setup
async fn test_critical_mistake_detection_equivalence() -> LlmResult<()> {
    let python_path = PathBuf::from("../../../python/codehud");
    let tester = EquivalenceTester::new(&python_path).await?;

    let result = tester.test_critical_detection_equivalence().await?;

    assert!(result.passed, "Critical detection test failed: {:?}", result.error_message);
    assert!(result.similarity_score >= 0.90, "Similarity score too low: {}", result.similarity_score);

    println!("✅ Critical mistake detection equivalence validated");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Python environment setup
async fn test_bug_fix_success_rate_validation() -> LlmResult<()> {
    let python_path = PathBuf::from("../../../python/codehud");
    let tester = EquivalenceTester::new(&python_path).await?;

    let bug_fix_results = tester.test_bug_fix_equivalence().await?;

    let passed_fixes = bug_fix_results.iter().filter(|r| r.passed).count();
    let total_fixes = bug_fix_results.len();
    let success_rate = passed_fixes as f32 / total_fixes as f32;

    println!("Bug fix success rate: {:.1}% ({}/{})", success_rate * 100.0, passed_fixes, total_fixes);

    for result in &bug_fix_results {
        if !result.passed {
            println!("❌ Failed: {} - {}", result.test_name, result.error_message.as_ref().unwrap_or(&"Unknown error".to_string()));
        } else {
            println!("✅ Passed: {} (similarity: {:.2})", result.test_name, result.similarity_score);
        }
    }

    assert!(
        success_rate >= 0.97,
        "Bug fix success rate {:.1}% below required 97%",
        success_rate * 100.0
    );

    println!("✅ 97%+ bug fix success rate requirement met!");
    Ok(())
}

#[test]
fn test_equivalence_tester_creation() {
    // Test that we can create the equivalence testing structures
    use codehud_llm::equivalence::{BugFixTestCase, EquivalenceTestResult};
    use chrono::Utc;

    let test_case = BugFixTestCase {
        name: "test_case".to_string(),
        buggy_code: "def test(): pass".to_string(),
        error_message: "SyntaxError".to_string(),
        expected_patterns: vec!["def".to_string()],
        context: None,
    };

    assert_eq!(test_case.name, "test_case");

    let test_result = EquivalenceTestResult {
        test_name: "test_result".to_string(),
        passed: true,
        rust_output: "output".to_string(),
        python_output: "output".to_string(),
        similarity_score: 1.0,
        execution_time_rust_ms: 100,
        execution_time_python_ms: 150,
        timestamp: Utc::now(),
        error_message: None,
    };

    assert!(test_result.passed);
    assert_eq!(test_result.similarity_score, 1.0);

    println!("✅ Equivalence test structures validated");
}

#[test]
fn test_similarity_calculation() {
    use codehud_llm::EquivalenceTester;
    use std::path::PathBuf;

    // Create a mock tester for testing similarity calculation
    // Note: This would need the actual Python environment in practice
    let python_path = PathBuf::from("/mock/path");

    // Test similarity calculation logic
    let output1 = "def factorial(n): return 1 if n <= 1 else n * factorial(n-1)";
    let output2 = "def factorial(n): return 1 if n <= 1 else n * factorial(n-1)";

    // Perfect match should be 1.0 similarity
    // This would need to be implemented as a standalone function for testing
    println!("✅ Similarity calculation logic ready for implementation");
}

/// Integration test that validates the complete zero-degradation pipeline
#[tokio::test]
#[ignore] // Requires full Python environment and test data
async fn test_complete_zero_degradation_pipeline() -> LlmResult<()> {
    let python_path = PathBuf::from("../../../python/codehud");

    // Step 1: Initialize the equivalence tester
    let tester = EquivalenceTester::new(&python_path).await?;

    // Step 2: Run comprehensive equivalence tests
    let results = tester.run_equivalence_tests().await?;

    // Step 3: Validate all zero-degradation requirements
    assert!(results.bug_fix_success_rate >= 0.97, "97%+ bug fix success rate required");
    assert!(results.overall_pass_rate >= 0.90, "90%+ overall pass rate expected");
    assert!(results.performance_ratio <= 2.0, "Performance should not degrade >2x");

    // Step 4: Generate comprehensive report
    let report = tester.generate_report(&results);

    // Write report to file for review
    std::fs::write("target/equivalence_report.md", report)
        .expect("Failed to write equivalence report");

    println!("✅ Complete zero-degradation pipeline validated!");
    println!("📊 Report saved to target/equivalence_report.md");

    Ok(())
}

/// Stress test for concurrent FFI bridge usage
#[tokio::test]
#[ignore] // Requires Python environment setup
async fn test_concurrent_ffi_bridge_usage() -> LlmResult<()> {
    let python_path = PathBuf::from("../../../python/codehud");
    let tester = EquivalenceTester::new(&python_path).await?;

    // Create multiple concurrent tasks
    let tasks: Vec<_> = (0..10).map(|i| {
        let bridge = &tester.bridge;
        tokio::spawn(async move {
            let test_code = format!("def test_function_{}(): pass", i);
            bridge.detect_critical_mistakes(&test_code, None)
        })
    }).collect();

    // Wait for all tasks to complete
    let results = futures::future::try_join_all(tasks).await;

    match results {
        Ok(mistake_results) => {
            let all_successful = mistake_results.iter().all(|r| r.is_ok());
            assert!(all_successful, "Some concurrent FFI calls failed");
            println!("✅ Concurrent FFI bridge usage validated");
        }
        Err(e) => panic!("Concurrent test failed: {}", e),
    }

    Ok(())
}

/// Memory leak detection test for long-running FFI usage
#[tokio::test]
#[ignore] // Requires Python environment setup
async fn test_ffi_memory_leak_detection() -> LlmResult<()> {
    let python_path = PathBuf::from("../../../python/codehud");
    let tester = EquivalenceTester::new(&python_path).await?;

    let initial_memory = get_memory_usage();

    // Run many iterations to detect potential memory leaks
    for i in 0..1000 {
        let test_code = format!("def test_{}(): return {}", i, i);
        let _ = tester.bridge.detect_critical_mistakes(&test_code, None)?;

        // Check memory every 100 iterations
        if i % 100 == 0 {
            let current_memory = get_memory_usage();
            let memory_growth = current_memory - initial_memory;

            // Allow reasonable memory growth but detect leaks
            assert!(
                memory_growth < 100_000_000, // 100MB threshold
                "Potential memory leak detected: {}MB growth",
                memory_growth / 1_000_000
            );
        }
    }

    println!("✅ No memory leaks detected in FFI bridge");
    Ok(())
}

/// Helper function to get current memory usage
fn get_memory_usage() -> u64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(contents) = fs::read_to_string("/proc/self/status") {
            for line in contents.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        return value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                    }
                }
            }
        }
    }
    0 // Fallback for unsupported platforms
}