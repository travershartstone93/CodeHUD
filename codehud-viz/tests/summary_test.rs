use codehud_core::{
    models::{AnalysisResult, ViewType, CodeMetrics},
};
use codehud_viz::VisualizationEngine;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_summary_visualization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Summary/FixRollbackDevnotes visualization...");

    // Use the current codebase as test data
    let codebase_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    println!("Testing with codebase: {}", codebase_path.display());

    // Create analysis result with comprehensive mock data from multiple extractors
    let mut analysis_result = AnalysisResult::new(codebase_path.to_string_lossy().to_string());
    analysis_result.files_analyzed = 42;
    analysis_result.analysis_duration = 2.5;
    analysis_result.health_score = 85.0;
    analysis_result.metrics = CodeMetrics::default();
    analysis_result.focus_recommendations = vec![
        "Existing recommendation from analysis".to_string(),
    ];

    // Add comprehensive view data from multiple extractors to test aggregation

    // Quality data
    let quality_data = json!({
        "summary": {
            "total_functions": 150.0,
            "total_classes": 25.0,
            "total_code_lines": 5000.0,
            "health_score": 86.6
        },
        "file_metrics": [
            {
                "file": "test_file.py",
                "maintainability_score": 94.14,
                "complexity_score": 1.0
            }
        ]
    });
    analysis_result.set_view_data("quality".to_string(), quality_data);

    // Security data
    let security_data = json!({
        "all_vulnerabilities": [
            {"severity": "high", "description": "SQL injection", "file_path": "db.py"},
            {"severity": "medium", "description": "Hardcoded secret", "file_path": "config.py"}
        ],
        "all_security_issues": [
            {"severity": "low", "description": "Weak random", "file_path": "utils.py"}
        ],
        "summary": {
            "security_score": 75.0,
            "risk_level": "medium"
        }
    });
    analysis_result.set_view_data("security".to_string(), security_data);

    // Testing data
    let testing_data = json!({
        "summary": {
            "coverage_percentage": 65.0,
            "test_files_count": 8.0
        }
    });
    analysis_result.set_view_data("testing".to_string(), testing_data);

    // Dependencies data
    let deps_data = json!({
        "summary": {
            "total_dependencies": 25.0,
            "circular_dependencies_count": 2.0
        }
    });
    analysis_result.set_view_data("dependencies".to_string(), deps_data);

    // Performance data
    let perf_data = json!({
        "summary": {
            "hotspots_count": 3.0
        }
    });
    analysis_result.set_view_data("performance".to_string(), perf_data);

    // Create visualization engine
    println!("Creating visualization engine...");
    let viz_engine = VisualizationEngine::new();

    // Generate Summary view
    println!("Generating Summary view...");
    let view = viz_engine.generate_view(ViewType::FixRollbackDevnotes, &analysis_result)?;
    println!("Summary view generated successfully");

    // Check that we got the right view content
    match &view.content {
        codehud_viz::ViewContent::Summary {
            health_score,
            files_analyzed,
            critical_issues,
            recommendations,
            metrics
        } => {
            println!("✅ Summary view content validated:");
            println!("  - Health score: {:.1}", health_score);
            println!("  - Files analyzed: {}", files_analyzed);
            println!("  - Critical issues: {}", critical_issues);
            println!("  - Recommendations: {}", recommendations.len());
            println!("  - Metrics: {}", metrics.len());

            println!("  📊 Key metrics:");
            for (key, value) in metrics.iter().take(10) {
                println!("    - {}: {:.1}", key, value);
            }

            if !recommendations.is_empty() {
                println!("  💡 Sample recommendations:");
                for rec in recommendations.iter().take(3) {
                    println!("    - {}", rec);
                }
            }

            // Verify we have valid data structure
            assert_eq!(*health_score, 85.0);
            assert_eq!(*files_analyzed, 42);
            assert_eq!(*critical_issues, 0);
            assert!(!recommendations.is_empty());
            assert!(!metrics.is_empty());

            // Verify enhanced metrics were extracted
            assert!(metrics.contains_key("Total Functions"));
            assert!(metrics.contains_key("Security Score"));
            assert!(metrics.contains_key("Test Coverage %"));
            assert!(metrics.contains_key("Dependencies"));
            assert!(metrics.contains_key("Lines of Code"));

            // Check specific values
            assert_eq!(metrics.get("Total Functions"), Some(&150.0));
            assert_eq!(metrics.get("Security Score"), Some(&75.0));
            assert_eq!(metrics.get("Test Coverage %"), Some(&65.0));
            assert_eq!(metrics.get("Dependencies"), Some(&25.0));
            assert_eq!(metrics.get("Lines of Code"), Some(&5000.0));

            // Verify enhanced recommendations were added
            assert!(recommendations.len() > 1); // Should have original + enhanced
            let recommendations_text = recommendations.join(" ");
            assert!(recommendations_text.contains("security vulnerabilities") ||
                   recommendations_text.contains("test coverage") ||
                   recommendations_text.contains("circular dependencies"));

            println!("✅ All assertions passed!");
        },
        other => {
            panic!("Expected Summary view content, got: {:?}", other);
        }
    }

    println!("✅ Summary visualization test completed successfully!");
    Ok(())
}