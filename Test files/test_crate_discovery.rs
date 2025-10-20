#!/usr/bin/env rust-script

//! Test script to verify crate discovery functionality

use std::path::PathBuf;

// Import from codehud-llm crate
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

    // Test file grouping with mock data
    println!("🗂️  Testing file grouping...");

    // Create mock file extractions
    let mock_extractions = vec![
        codehud_llm::FileCommentExtraction {
            file: "./codehud-core/src/lib.rs".to_string(),
            language: "rust".to_string(),
            extraction_method: "test".to_string(),
            comments: vec![],
            structural_insights: None,
            stats: Default::default(),
        },
        codehud_llm::FileCommentExtraction {
            file: "./codehud-llm/src/lib.rs".to_string(),
            language: "rust".to_string(),
            extraction_method: "test".to_string(),
            comments: vec![],
            structural_insights: None,
            stats: Default::default(),
        },
    ];

    let grouped = grouper.group_files(&mock_extractions)?;

    println!("📊 Grouped files by crate:");
    for (crate_name, files) in &grouped {
        println!("  {}: {} files", crate_name, files.len());
        for file in files {
            println!("    - {}", file.file);
        }
    }

    println!("\n🎉 Crate discovery test completed successfully!");
    Ok(())
}