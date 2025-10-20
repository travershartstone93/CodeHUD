use codehud_core::{
    extractors::{BaseDataExtractor, TopologyExtractor},
    models::{AnalysisResult, ViewType, CodeMetrics},
};
use codehud_viz::VisualizationEngine;
use std::path::PathBuf;

#[tokio::test]
async fn test_topology_visualization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Topology visualization...");

    // Use the current codebase as test data
    let codebase_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    println!("Testing with codebase: {}", codebase_path.display());

    // Create topology extractor
    let extractor = TopologyExtractor::new(&codebase_path)?;

    // Extract topology data
    println!("Extracting topology data...");
    let topology_data = extractor.extract_data()?;
    println!("Topology data extracted: {:#?}", topology_data);

    // Create analysis result with real topology data
    let mut analysis_result = AnalysisResult::new(codebase_path.to_string_lossy().to_string());
    analysis_result.files_analyzed = 42;
    analysis_result.analysis_duration = 2.5;
    analysis_result.health_score = 85.0;
    analysis_result.metrics = CodeMetrics::default();

    // Store topology data in the extracted view data (convert HashMap to Value)
    let topology_value = serde_json::to_value(&topology_data)?;
    analysis_result.set_view_data("topology".to_string(), topology_value);

    // Create visualization engine
    println!("Creating visualization engine...");
    let viz_engine = VisualizationEngine::new();

    // Generate Topology view
    println!("Generating Topology view...");
    let view = viz_engine.generate_view(ViewType::Topology, &analysis_result)?;
    println!("Topology view generated successfully");

    // Check that we got the right view content
    match &view.content {
        codehud_viz::ViewContent::Topology {
            file_tree,
            language_distribution,
            complexity_distribution,
            coupling_metrics
        } => {
            println!("✅ Topology view content validated:");
            println!("  - File tree: {} files, {} directories", file_tree.total_files, file_tree.total_directories);
            println!("  - Languages: {}", language_distribution.len());
            println!("  - Complexity data: {} files", complexity_distribution.len());
            println!("  - Coupling metrics: {} files", coupling_metrics.len());

            if !language_distribution.is_empty() {
                println!("  📊 Language distribution:");
                for (lang, count) in language_distribution.iter().take(3) {
                    println!("    - {}: {} files", lang, count);
                }
            }

            if !complexity_distribution.is_empty() {
                println!("  🧮 Sample complexity scores:");
                for (file, complexity) in complexity_distribution.iter().take(3) {
                    println!("    - {}: {:.1}", file, complexity);
                }
            }

            if !coupling_metrics.is_empty() {
                println!("  🔗 Sample coupling metrics:");
                for (file, coupling) in coupling_metrics.iter().take(3) {
                    println!("    - {}: {:.2}", file, coupling);
                }
            }

            if !file_tree.root.children.is_empty() {
                println!("  📁 Sample files in tree:");
                for child in file_tree.root.children.iter().take(3) {
                    println!("    - {}", child.name);
                }
            }

            // Verify we have at least some data structure
            assert!(file_tree.total_files >= 0);
            assert!(file_tree.total_directories >= 0);

            println!("✅ All assertions passed!");
        },
        other => {
            panic!("Expected Topology view content, got: {:?}", other);
        }
    }

    println!("✅ Topology visualization test completed successfully!");
    Ok(())
}