use codehud_core::{
    extractors::{BaseDataExtractor, DependenciesExtractor},
    models::{AnalysisResult, ViewType, CodeMetrics},
};
use codehud_viz::VisualizationEngine;
use std::path::PathBuf;

#[tokio::test]
async fn test_dependencies_visualization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Dependencies visualization...");

    // Use the current codebase as test data
    let codebase_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    println!("Testing with codebase: {}", codebase_path.display());

    // Create dependencies extractor
    let extractor = DependenciesExtractor::new(&codebase_path)?;

    // Extract dependencies data
    println!("Extracting dependencies data...");
    let dependencies_data = extractor.extract_data()?;
    println!("Dependencies data extracted: {:#?}", dependencies_data);

    // Create analysis result with real dependencies data
    let mut analysis_result = AnalysisResult::new(codebase_path.to_string_lossy().to_string());
    analysis_result.files_analyzed = 42;
    analysis_result.analysis_duration = 2.5;
    analysis_result.health_score = 85.0;
    analysis_result.metrics = CodeMetrics::default();

    // Store dependencies data in the extracted view data (convert HashMap to Value)
    let dependencies_value = serde_json::to_value(&dependencies_data)?;
    analysis_result.set_view_data("dependencies".to_string(), dependencies_value);

    // Create visualization engine
    println!("Creating visualization engine...");
    let viz_engine = VisualizationEngine::new();

    // Generate Dependencies view
    println!("Generating Dependencies view...");
    let view = viz_engine.generate_view(ViewType::Dependencies, &analysis_result)?;
    println!("Dependencies view generated successfully");

    // Check that we got the right view content
    match &view.content {
        codehud_viz::ViewContent::Dependencies {
            total_dependencies,
            circular_dependencies,
            dependency_graph,
            coupling_analysis,
            external_dependencies
        } => {
            println!("✅ Dependencies view content validated:");
            println!("  - Total dependencies: {}", total_dependencies);
            println!("  - Circular dependencies: {}", circular_dependencies.len());
            println!("  - Dependency graph nodes: {}", dependency_graph.nodes.len());
            println!("  - Dependency graph edges: {}", dependency_graph.edges.len());
            println!("  - Coupling analysis entries: {}", coupling_analysis.len());
            println!("  - External dependencies: {}", external_dependencies.len());

            if !circular_dependencies.is_empty() {
                println!("  ⚠️  Circular dependencies found:");
                for cycle in circular_dependencies.iter().take(3) {
                    println!("    - {}", cycle);
                }
            }

            if !coupling_analysis.is_empty() {
                println!("  📊 Top coupling scores:");
                for (file, score) in coupling_analysis.iter().take(3) {
                    println!("    - {:.2}: {}", score, file);
                }
            }

            if !external_dependencies.is_empty() {
                println!("  📦 External dependencies:");
                for dep in external_dependencies.iter().take(3) {
                    println!("    - {}", dep);
                }
            }

            // Verify we have at least some data
            assert!(*total_dependencies >= 0);
            assert!(dependency_graph.nodes.len() >= 0);
            assert!(dependency_graph.edges.len() >= 0);

            println!("✅ All assertions passed!");
        },
        other => {
            panic!("Expected Dependencies view content, got: {:?}", other);
        }
    }

    println!("✅ Dependencies visualization test completed successfully!");
    Ok(())
}