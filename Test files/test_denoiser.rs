#!/usr/bin/env rust-script

//! Test script to apply denoiser to extracted_comments.json
//!
//! This will create extracted_comments_cleaned.json to demonstrate
//! token reduction for LLM context preparation.

use std::fs;
use std::path::Path;
use serde_json;

// We need to import from codehud-llm crate
use codehud_llm::{LlmContextDenoiser, DenoiserConfig, FileCommentExtraction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_file = "project_scan_output/extracted_comments.json";
    let output_file = "project_scan_output/extracted_comments_cleaned.json";
    let stats_file = "project_scan_output/denoiser_stats.json";

    println!("🧹 Testing LLM Context Denoiser");
    println!("📥 Input: {}", input_file);
    println!("📤 Output: {}", output_file);

    // Check if input file exists
    if !Path::new(input_file).exists() {
        eprintln!("❌ Input file not found: {}", input_file);
        eprintln!("   Run: ./target/release/codehud-llm scan-project . first");
        return Ok(());
    }

    // Read the original extracted comments
    println!("📖 Reading extracted comments...");
    let content = fs::read_to_string(input_file)?;
    let original_size = content.len();
    println!("   Original size: {} characters", original_size);

    let extractions: Vec<FileCommentExtraction> = serde_json::from_str(&content)?;
    println!("   Files with comments: {}", extractions.len());

    // Count original comments and structural insights
    let mut original_comments = 0;
    let mut original_insights = 0;
    for extraction in &extractions {
        original_comments += extraction.comments.len();
        if let Some(ref insights) = extraction.structural_insights {
            for items in insights.sections.values() {
                original_insights += items.len();
            }
        }
    }
    println!("   Total comments: {}", original_comments);
    println!("   Total structural insights: {}", original_insights);

    // Create denoiser with aggressive settings for testing
    let config = DenoiserConfig {
        target_reduction: 0.6, // 60% reduction target
        min_phrase_length: 3,
        max_phrase_length: 15,
        preserve_structural_insights: true,
        preserve_metadata: true,
    };

    let mut denoiser = LlmContextDenoiser::new(config);

    // Apply denoising
    println!("🧹 Applying denoiser...");
    let (cleaned_extractions, stats) = denoiser.denoise_extractions(&extractions);

    // Save cleaned version
    println!("💾 Saving cleaned version...");
    let cleaned_json = serde_json::to_string_pretty(&cleaned_extractions)?;
    fs::write(output_file, &cleaned_json)?;

    // Save statistics
    let stats_json = serde_json::to_string_pretty(&stats)?;
    fs::write(stats_file, &stats_json)?;

    // Print results
    println!("\n✅ Denoising complete!");
    println!("📊 Results:");
    println!("   Original characters: {}", stats.original_characters);
    println!("   Cleaned characters: {}", stats.cleaned_characters);
    println!("   Reduction: {:.1}%", stats.reduction_percentage);
    println!("   Files processed: {}", stats.files_processed);
    println!("   Repeated phrases removed: {}", stats.repeated_phrases_removed);
    println!("   Common words consolidated: {}", stats.common_words_consolidated);

    // Estimate token reduction (rough approximation: 4 chars per token)
    let original_tokens = stats.original_characters / 4;
    let cleaned_tokens = stats.cleaned_characters / 4;
    println!("\n🧮 Estimated token reduction:");
    println!("   Original tokens: ~{}", original_tokens);
    println!("   Cleaned tokens: ~{}", cleaned_tokens);
    println!("   Token reduction: ~{}", original_tokens - cleaned_tokens);

    if cleaned_tokens < 12000 {
        println!("✅ Cleaned data fits within 12K token LLM context window!");
    } else {
        println!("⚠️  Cleaned data ({} tokens) still exceeds 12K token limit", cleaned_tokens);
        println!("   Consider more aggressive denoising or hierarchical summarization");
    }

    println!("\n📁 Files generated:");
    println!("   📄 {}", output_file);
    println!("   📊 {}", stats_file);

    Ok(())
}