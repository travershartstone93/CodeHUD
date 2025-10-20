use codehud_core::extractors::quality::QualityExtractor;
use codehud_core::extractors::BaseDataExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let codebase_path = "/home/travers/Desktop/CodeHUD (copy)/src/codehud/core/data_extractors";

    println!("Testing QualityExtractor on Python codebase...");
    println!("Codebase path: {}", codebase_path);

    let extractor = QualityExtractor::new(codebase_path)?;
    let start_time = std::time::Instant::now();

    let result = extractor.extract_data()?;
    let duration = start_time.elapsed();

    println!("\n=== QUALITY EXTRACTION RESULTS ===");
    println!("Extraction time: {:?}", duration);

    // Print summary
    if let Some(summary) = result.get("summary") {
        println!("\nSUMMARY:");
        println!("{}", serde_json::to_string_pretty(summary)?);
    }

    // Print health score
    if let Some(health_score) = result.get("health_score") {
        println!("\nOVERALL HEALTH SCORE: {}", health_score);
    }

    // Print file count and issues
    if let Some(files_analyzed) = result.get("files_analyzed") {
        println!("Files analyzed: {}", files_analyzed);
    }

    if let Some(quality_issues) = result.get("quality_issues").and_then(|v| v.as_array()) {
        println!("Total issues found: {}", quality_issues.len());

        // Show first few issues as examples
        for (i, issue) in quality_issues.iter().take(5).enumerate() {
            println!("Issue {}: {}", i + 1, serde_json::to_string_pretty(issue)?);
        }
    }

    // Print external tool results
    if let Some(external_results) = result.get("external_tool_results") {
        println!("\nEXTERNAL TOOL RESULTS:");
        println!("{}", serde_json::to_string_pretty(external_results)?);
    }

    println!("\n=== TEST COMPLETED ===");

    Ok(())
}