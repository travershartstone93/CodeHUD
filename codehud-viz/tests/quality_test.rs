use codehud_core::{
    extractors::{BaseDataExtractor, QualityExtractor},
    models::{AnalysisResult, ViewType, CodeMetrics},
};
use codehud_viz::VisualizationEngine;
use std::path::PathBuf;

#[tokio::test]
async fn test_quality_visualization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Quality visualization...");

    // Use the current codebase as test data
    let codebase_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    println!("Testing with codebase: {}", codebase_path.display());

    // Create quality extractor
    let extractor = QualityExtractor::new(&codebase_path)?;

    // Extract quality data
    println!("Extracting quality data...");
    let quality_data = extractor.extract_data()?;
    println!("Quality data extracted: {:#?}", quality_data);

    // Create analysis result with real quality data
    let mut analysis_result = AnalysisResult::new(codebase_path.to_string_lossy().to_string());
    analysis_result.files_analyzed = 42;
    analysis_result.analysis_duration = 2.5;
    analysis_result.health_score = 85.0;
    analysis_result.metrics = CodeMetrics::default();

    // Store quality data in the extracted view data (convert HashMap to Value)
    let quality_value = serde_json::to_value(&quality_data)?;
    analysis_result.set_view_data("quality".to_string(), quality_value);

    // Create visualization engine
    println!("Creating visualization engine...");
    let viz_engine = VisualizationEngine::new();

    // Generate Quality view
    println!("Generating Quality view...");
    let view = viz_engine.generate_view(ViewType::Quality, &analysis_result)?;
    println!("Quality view generated successfully");

    // Check that we got the right view content
    match &view.content {
        codehud_viz::ViewContent::Quality {
            health_score,
            issues_by_severity,
            top_problematic_files,
            complexity_trend,
            maintainability_scores
        } => {
            println!("✅ Quality view content validated:");
            println!("  - Health score: {:.2}", health_score);
            println!("  - Issues by severity: {}", issues_by_severity.len());
            println!("  - Problematic files: {}", top_problematic_files.len());
            println!("  - Complexity trend: {}", complexity_trend.len());
            println!("  - Maintainability scores: {}", maintainability_scores.len());

            if !issues_by_severity.is_empty() {
                println!("  ⚠️  Issues breakdown:");
                for (severity, count) in issues_by_severity {
                    println!("    - {}: {} issues", severity, count);
                }
            }

            if !top_problematic_files.is_empty() {
                println!("  🚨 Most problematic files:");
                for (file, score) in top_problematic_files.iter().take(3) {
                    println!("    - {}: {:.2}", file, score);
                }
            }

            if !complexity_trend.is_empty() {
                println!("  🧮 Sample complexity scores:");
                for (file, complexity) in complexity_trend.iter().take(3) {
                    println!("    - {}: {:.1}", file, complexity);
                }
            }

            if !maintainability_scores.is_empty() {
                println!("  🛠️  Sample maintainability scores:");
                for (file, maintainability) in maintainability_scores.iter().take(3) {
                    println!("    - {}: {:.2}", file, maintainability);
                }
            }

            // Verify we have valid data structure
            assert!(*health_score >= 0.0);
            assert!(*health_score <= 1.0);

            println!("✅ All assertions passed!");
        },
        other => {
            panic!("Expected Quality view content, got: {:?}", other);
        }
    }

    println!("✅ Quality visualization test completed successfully!");
    Ok(())
}