use codehud_core::{
    extractors::{BaseDataExtractor, FlowExtractor},
    models::{AnalysisResult, ViewType, CodeMetrics},
};
use codehud_viz::VisualizationEngine;
use std::path::PathBuf;

#[tokio::test]
async fn test_flow_visualization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Flow visualization...");

    // Use the current codebase as test data
    let codebase_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    println!("Testing with codebase: {}", codebase_path.display());

    // Create flow extractor
    let extractor = FlowExtractor::new(&codebase_path)?;

    // Extract flow data
    println!("Extracting flow data...");
    let flow_data = extractor.extract_data()?;
    println!("Flow data extracted: {:#?}", flow_data);

    // Create analysis result with real flow data
    let mut analysis_result = AnalysisResult::new(codebase_path.to_string_lossy().to_string());
    analysis_result.files_analyzed = 42;
    analysis_result.analysis_duration = 2.5;
    analysis_result.health_score = 85.0;
    analysis_result.metrics = CodeMetrics::default();

    // Store flow data in the extracted view data (convert HashMap to Value)
    let flow_value = serde_json::to_value(&flow_data)?;
    analysis_result.set_view_data("flow".to_string(), flow_value);

    // Create visualization engine
    println!("Creating visualization engine...");
    let viz_engine = VisualizationEngine::new();

    // Generate Flow view
    println!("Generating Flow view...");
    let view = viz_engine.generate_view(ViewType::Flow, &analysis_result)?;
    println!("Flow view generated successfully");

    // Check that we got the right view content
    match &view.content {
        codehud_viz::ViewContent::Flow {
            data_flows,
            control_flows,
            flow_complexity,
            bottlenecks
        } => {
            println!("✅ Flow view content validated:");
            println!("  - Data flows: {}", data_flows.len());
            println!("  - Control flows: {}", control_flows.len());
            println!("  - Flow complexity: {:.2}", flow_complexity);
            println!("  - Bottlenecks: {}", bottlenecks.len());

            if !data_flows.is_empty() {
                println!("  📝 Sample data flows:");
                for flow in data_flows.iter().take(3) {
                    println!("    - {} → {} ({})", flow.from, flow.to, flow.flow_type);
                }
            }

            if !control_flows.is_empty() {
                println!("  📞 Sample control flows:");
                for flow in control_flows.iter().take(3) {
                    println!("    - {} → {} ({})", flow.from, flow.to, flow.flow_type);
                }
            }

            if !bottlenecks.is_empty() {
                println!("  ⚠️  Flow bottlenecks:");
                for bottleneck in bottlenecks.iter().take(3) {
                    println!("    - {}", bottleneck);
                }
            }

            // Verify we have at least some data structure
            assert!(*flow_complexity >= 0.0);
            assert!(*flow_complexity <= 1.0);

            println!("✅ All assertions passed!");
        },
        other => {
            panic!("Expected Flow view content, got: {:?}", other);
        }
    }

    println!("✅ Flow visualization test completed successfully!");
    Ok(())
}