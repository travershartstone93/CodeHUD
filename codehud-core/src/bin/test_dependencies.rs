use codehud_core::extractors::dependencies::DependenciesExtractor;
use codehud_core::extractors::BaseDataExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let codebase_path = "/home/travers/Desktop/CodeHUD (copy)/src/codehud/core/data_extractors";

    println!("Testing DependenciesExtractor on Python codebase...");
    println!("Codebase path: {}", codebase_path);

    let extractor = DependenciesExtractor::new(codebase_path)?;
    let start_time = std::time::Instant::now();

    let result = extractor.extract_data()?;
    let duration = start_time.elapsed();

    println!("\n=== DEPENDENCIES EXTRACTION RESULTS ===");
    println!("Extraction time: {:?}", duration);

    // Print summary
    if let Some(summary) = result.get("summary") {
        println!("\nSUMMARY:");
        println!("{}", serde_json::to_string_pretty(summary)?);
    }

    // Print dependency metrics
    if let Some(metrics) = result.get("dependency_metrics") {
        println!("\nDEPENDENCY METRICS:");
        println!("{}", serde_json::to_string_pretty(metrics)?);
    }

    // Print coupling analysis
    if let Some(coupling) = result.get("coupling_analysis") {
        println!("\nCOUPLING ANALYSIS:");
        println!("{}", serde_json::to_string_pretty(coupling)?);
    }

    // Print file count and dependencies
    if let Some(files_analyzed) = result.get("summary").and_then(|s| s.get("total_files_analyzed")) {
        println!("Files analyzed: {}", files_analyzed);
    }

    if let Some(circular_deps) = result.get("circular_dependencies").and_then(|v| v.as_array()) {
        println!("Circular dependencies found: {}", circular_deps.len());
        for (i, dep) in circular_deps.iter().take(3).enumerate() {
            println!("Circular dependency {}: {}", i + 1, serde_json::to_string_pretty(dep)?);
        }
    }

    if let Some(external_deps) = result.get("external_dependencies") {
        println!("\nEXTERNAL DEPENDENCIES:");
        println!("{}", serde_json::to_string_pretty(external_deps)?);
    }

    // Print recommendations
    if let Some(recommendations) = result.get("recommendations").and_then(|v| v.as_array()) {
        println!("\nRECOMMENDATIONS:");
        for (i, rec) in recommendations.iter().enumerate() {
            println!("{}. {}", i + 1, rec.as_str().unwrap_or(""));
        }
    }

    println!("\n=== TEST COMPLETED ===");

    Ok(())
}