use codehud_core::{
    models::{AnalysisResult, ViewType, CodeMetrics},
};
use codehud_viz::VisualizationEngine;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_security_visualization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Security visualization...");

    // Use the current codebase as test data
    let codebase_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    println!("Testing with codebase: {}", codebase_path.display());

    // Create analysis result with mock security data to test visualization
    let mut analysis_result = AnalysisResult::new(codebase_path.to_string_lossy().to_string());
    analysis_result.files_analyzed = 42;
    analysis_result.analysis_duration = 2.5;
    analysis_result.health_score = 85.0;
    analysis_result.metrics = CodeMetrics::default();

    // Create mock security data that matches what the SecurityExtractor would produce
    let security_data = json!({
        "all_vulnerabilities": [
            {
                "severity": "high",
                "description": "SQL injection vulnerability detected",
                "file_path": "/home/user/project/db.py",
                "line_number": 42,
                "vulnerability_type": "sql_injection",
                "confidence": "high"
            },
            {
                "severity": "medium",
                "description": "Hardcoded password found",
                "file_path": "/home/user/project/config.py",
                "line_number": 15,
                "vulnerability_type": "hardcoded_secret",
                "confidence": "medium"
            }
        ],
        "all_security_issues": [
            {
                "severity": "low",
                "description": "Insecure random number generation",
                "file_path": "/home/user/project/utils.py",
                "line_number": 23,
                "issue_type": "weak_random"
            }
        ],
        "all_dangerous_functions": [
            {
                "severity": "high",
                "function_name": "eval",
                "file_path": "/home/user/project/parser.py",
                "line_number": 67
            }
        ],
        "summary": {
            "risk_level": "medium",
            "security_score": 75.0,
            "total_vulnerabilities": 4
        }
    });

    // Store security data in the extracted view data
    analysis_result.set_view_data("security".to_string(), security_data);

    // Create visualization engine
    println!("Creating visualization engine...");
    let viz_engine = VisualizationEngine::new();

    // Generate Security view
    println!("Generating Security view...");
    let view = viz_engine.generate_view(ViewType::Security, &analysis_result)?;
    println!("Security view generated successfully");

    // Check that we got the right view content
    match &view.content {
        codehud_viz::ViewContent::Security {
            risk_level,
            vulnerabilities_by_severity,
            top_security_issues,
            security_score,
            files_with_issues
        } => {
            println!("✅ Security view content validated:");
            println!("  - Risk level: {}", risk_level);
            println!("  - Security score: {:.1}", security_score);
            println!("  - Vulnerabilities by severity: {}", vulnerabilities_by_severity.len());
            println!("  - Top security issues: {}", top_security_issues.len());
            println!("  - Files with issues: {}", files_with_issues.len());

            if !vulnerabilities_by_severity.is_empty() {
                println!("  🛡️  Vulnerabilities breakdown:");
                for (severity, count) in vulnerabilities_by_severity {
                    println!("    - {}: {} issues", severity, count);
                }
            }

            if !top_security_issues.is_empty() {
                println!("  🚨 Sample security issues:");
                for issue in top_security_issues.iter().take(3) {
                    println!("    - [{}] {} ({})", issue.severity, issue.description, issue.file);
                }
            }

            if !files_with_issues.is_empty() {
                println!("  📁 Sample affected files:");
                for file in files_with_issues.iter().take(3) {
                    println!("    - {}", file);
                }
            }

            // Verify we have valid data structure
            assert!(*security_score >= 0.0);
            assert!(*security_score <= 100.0);
            assert!(matches!(risk_level.as_str(), "low" | "medium" | "high" | "critical"));

            // Verify mock data was processed correctly
            assert_eq!(*security_score, 75.0);
            assert_eq!(risk_level, "medium");
            assert!(!vulnerabilities_by_severity.is_empty());
            assert!(!top_security_issues.is_empty());
            assert!(!files_with_issues.is_empty());

            println!("✅ All assertions passed!");
        },
        other => {
            panic!("Expected Security view content, got: {:?}", other);
        }
    }

    println!("✅ Security visualization test completed successfully!");
    Ok(())
}