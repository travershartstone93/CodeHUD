use codehud_core::extractors::security::SecurityExtractor;
use codehud_core::extractors::BaseDataExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let codebase_path = "/home/travers/Desktop/CodeHUD (copy)/src/codehud/core/data_extractors";

    println!("Testing SecurityExtractor on Python codebase...");
    println!("Codebase path: {}", codebase_path);

    let extractor = SecurityExtractor::new(codebase_path)?;
    let start_time = std::time::Instant::now();

    let result = extractor.extract_data()?;
    let duration = start_time.elapsed();

    println!("\n=== SECURITY EXTRACTION RESULTS ===");
    println!("Extraction time: {:?}", duration);

    // Print summary
    if let Some(summary) = result.get("summary") {
        println!("\nSUMMARY:");
        println!("{}", serde_json::to_string_pretty(summary)?);
    }

    // Print risk assessment
    if let Some(risk_assessment) = result.get("risk_assessment") {
        println!("\nRISK ASSESSMENT:");
        println!("{}", serde_json::to_string_pretty(risk_assessment)?);
    }

    // Print file count and issues
    if let Some(files_analyzed) = result.get("files_analyzed") {
        println!("Files analyzed: {}", files_analyzed);
    }

    if let Some(vulnerabilities) = result.get("vulnerabilities").and_then(|v| v.as_array()) {
        println!("Vulnerabilities found: {}", vulnerabilities.len());
        for (i, vuln) in vulnerabilities.iter().take(3).enumerate() {
            println!("Vulnerability {}: {}", i + 1, serde_json::to_string_pretty(vuln)?);
        }
    }

    if let Some(security_issues) = result.get("security_issues").and_then(|v| v.as_array()) {
        println!("Security issues found: {}", security_issues.len());
        for (i, issue) in security_issues.iter().take(3).enumerate() {
            println!("Issue {}: {}", i + 1, serde_json::to_string_pretty(issue)?);
        }
    }

    if let Some(dangerous_functions) = result.get("dangerous_functions").and_then(|v| v.as_array()) {
        println!("Dangerous functions found: {}", dangerous_functions.len());
        for (i, func) in dangerous_functions.iter().take(3).enumerate() {
            println!("Dangerous function {}: {}", i + 1, serde_json::to_string_pretty(func)?);
        }
    }

    // Print recommendations
    if let Some(recommendations) = result.get("recommendations").and_then(|v| v.as_array()) {
        println!("\nRECOMMENDATIONS:");
        for (i, rec) in recommendations.iter().enumerate() {
            println!("{}. {}", i + 1, rec.as_str().unwrap_or(""));
        }
    }

    // Print bandit results
    if let Some(bandit_results) = result.get("bandit_results") {
        println!("\nBANDIT RESULTS:");
        println!("{}", serde_json::to_string_pretty(bandit_results)?);
    }

    println!("\n=== TEST COMPLETED ===");

    Ok(())
}