use std::path::PathBuf;
use codehud_llm::CrateGrouper;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Crate Discovery");
    println!("==========================");

    let project_path = PathBuf::from(".");
    let mut grouper = CrateGrouper::new(project_path);

    println!("📦 Discovering crates...");
    let crates = grouper.discover_crates()?;

    println!("\n✅ Found {} crates:", crates.len());
    for (i, crate_info) in crates.iter().enumerate() {
        println!("{}. {} (v{})", i + 1, crate_info.name, crate_info.version);
        println!("   Path: {}", crate_info.path.display());
        if let Some(description) = &crate_info.description {
            println!("   Description: {}", description);
        }
        println!();
    }

    println!("🎉 Crate discovery test completed successfully!");
    Ok(())
}