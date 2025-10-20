use std::fs;
use std::env;
use serde_json;

fn main() {
    let input_file = "project_scan_output/extracted_comments.json";
    let output_file = "project_scan_output/extracted_comments_cleaned.json";
    let stats_file = "project_scan_output/denoiser_stats.json";

    println!("🧹 Testing LLM Context Denoiser");
    println!("📥 Input: {}", input_file);
    println!("📤 Output: {}", output_file);

    // Check if input file exists
    if !std::path::Path::new(input_file).exists() {
        eprintln!("❌ Input file not found: {}", input_file);
        eprintln!("   Run: ./target/release/codehud-llm scan-project . first");
        return;
    }

    // Read the original extracted comments
    println!("📖 Reading extracted comments...");
    let content = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Failed to read input file: {}", e);
            return;
        }
    };

    let original_size = content.len();
    println!("   Original size: {} characters", original_size);

    let extractions: Vec<serde_json::Value> = match serde_json::from_str(&content) {
        Ok(extractions) => extractions,
        Err(e) => {
            eprintln!("❌ Failed to parse JSON: {}", e);
            return;
        }
    };

    println!("   Files with comments: {}", extractions.len());

    // Simple text-based denoising
    println!("🧹 Applying basic text denoising...");

    let mut cleaned_extractions = Vec::new();
    let mut original_chars = 0;
    let mut cleaned_chars = 0;

    for extraction in &extractions {
        let mut cleaned_extraction = extraction.clone();

        // Get comments array
        if let Some(comments) = extraction["comments"].as_array() {
            let mut cleaned_comments = Vec::new();

            for comment in comments {
                if let Some(text) = comment["text"].as_str() {
                    original_chars += text.len();

                    // Basic denoising: remove repeated phrases
                    let cleaned_text = denoise_text(text);
                    cleaned_chars += cleaned_text.len();

                    let mut cleaned_comment = comment.clone();
                    cleaned_comment["text"] = serde_json::Value::String(cleaned_text);
                    cleaned_comments.push(cleaned_comment);
                } else {
                    cleaned_comments.push(comment.clone());
                }
            }

            cleaned_extraction["comments"] = serde_json::Value::Array(cleaned_comments);
        }

        // Also clean structural insights if present
        if let Some(insights) = extraction["structural_insights"].as_object() {
            if let Some(sections) = insights["sections"].as_object() {
                let mut cleaned_sections = serde_json::Map::new();

                for (section_name, section_items) in sections {
                    if let Some(items) = section_items.as_array() {
                        let mut cleaned_items = Vec::new();
                        let mut seen_items = std::collections::HashSet::new();

                        for item in items {
                            if let Some(item_text) = item.as_str() {
                                original_chars += item_text.len();
                                let cleaned_item = denoise_text(item_text);
                                cleaned_chars += cleaned_item.len();

                                // Remove duplicates
                                if !seen_items.contains(&cleaned_item) && !cleaned_item.trim().is_empty() {
                                    seen_items.insert(cleaned_item.clone());
                                    cleaned_items.push(serde_json::Value::String(cleaned_item));
                                }
                            }
                        }

                        if !cleaned_items.is_empty() {
                            cleaned_sections.insert(section_name.clone(), serde_json::Value::Array(cleaned_items));
                        }
                    }
                }

                let mut cleaned_insights = insights.clone();
                cleaned_insights["sections"] = serde_json::Value::Object(cleaned_sections);
                cleaned_extraction["structural_insights"] = serde_json::Value::Object(cleaned_insights);
            }
        }

        cleaned_extractions.push(cleaned_extraction);
    }

    // Save cleaned version
    println!("💾 Saving cleaned version...");
    let cleaned_json = serde_json::to_string_pretty(&cleaned_extractions).unwrap();
    if let Err(e) = fs::write(output_file, &cleaned_json) {
        eprintln!("❌ Failed to write output file: {}", e);
        return;
    }

    // Calculate and save statistics
    let reduction_percentage = if original_chars > 0 {
        ((original_chars - cleaned_chars) as f32 / original_chars as f32) * 100.0
    } else {
        0.0
    };

    let stats = serde_json::json!({
        "original_characters": original_chars,
        "cleaned_characters": cleaned_chars,
        "reduction_percentage": reduction_percentage,
        "files_processed": extractions.len(),
        "repeated_phrases_removed": 0,
        "common_words_consolidated": 0
    });

    let stats_json = serde_json::to_string_pretty(&stats).unwrap();
    if let Err(e) = fs::write(stats_file, &stats_json) {
        eprintln!("❌ Failed to write stats file: {}", e);
        return;
    }

    // Print results
    println!("\n✅ Denoising complete!");
    println!("📊 Results:");
    println!("   Original characters: {}", original_chars);
    println!("   Cleaned characters: {}", cleaned_chars);
    println!("   Reduction: {:.1}%", reduction_percentage);
    println!("   Files processed: {}", extractions.len());

    // Estimate token reduction (rough approximation: 4 chars per token)
    let original_tokens = original_chars / 4;
    let cleaned_tokens = cleaned_chars / 4;
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
}

fn denoise_text(text: &str) -> String {
    let mut result = text.to_string();

    // Remove common filler phrases
    let filler_phrases = vec![
        "this function",
        "this method",
        "this file",
        "this module",
        "this struct",
        "this enum",
        "this implementation",
        "as mentioned",
        "it should be noted",
        "it is important",
        "please note",
        "note that",
        "it appears",
        "seems to",
        "appears to be",
        "the function",
        "the method",
        "the module",
        "the struct",
        "uses serde",
        "for serialization",
        "for deserialization",
    ];

    for phrase in filler_phrases {
        result = result.replace(phrase, "");
    }

    // Consolidate repeated words
    let words: Vec<&str> = result.split_whitespace().collect();
    let mut result_words = Vec::new();
    let mut word_count = std::collections::HashMap::new();

    for word in words {
        let normalized = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        if normalized.len() <= 2 {
            result_words.push(word);
            continue;
        }

        let count = word_count.entry(normalized.clone()).or_insert(0);
        *count += 1;

        // Only include word if it hasn't appeared too many times
        if *count <= 2 {
            result_words.push(word);
        }
    }

    // Clean up whitespace
    result_words.join(" ")
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}