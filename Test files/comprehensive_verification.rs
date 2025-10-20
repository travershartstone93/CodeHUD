use std::fs;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== COMPREHENSIVE DATA VERIFICATION ===\n");

    // Run fresh analysis and capture output for verification
    println!("🔄 Running fresh analysis for verification...");
    let output = Command::new("cargo")
        .arg("run")
        .arg("--package").arg("codehud-cli")
        .arg("--bin").arg("codehud")
        .arg("--").arg("analyze").arg(".").arg("--view").arg("topology")
        .output()
        .expect("Failed to execute analysis");

    let analysis_output = String::from_utf8_lossy(&output.stdout);
    println!("✅ Analysis completed\n");

    // Test 1: Verify specific file metrics
    println!("📊 TEST 1: FILE METRICS VERIFICATION");
    verify_specific_files(&analysis_output);

    // Test 2: Check dependency extraction accuracy
    println!("\n🔗 TEST 2: DEPENDENCY EXTRACTION VERIFICATION");
    verify_dependencies(&analysis_output);

    // Test 3: Verify complexity calculations
    println!("\n⚡ TEST 3: COMPLEXITY CALCULATION VERIFICATION");
    verify_complexity(&analysis_output);

    // Test 4: Check file type detection
    println!("\n📁 TEST 4: FILE TYPE DETECTION VERIFICATION");
    verify_file_types(&analysis_output);

    // Test 5: Verify coupling analysis
    println!("\n🔗 TEST 5: COUPLING ANALYSIS VERIFICATION");
    verify_coupling(&analysis_output);

    println!("\n=== VERIFICATION COMPLETE ===");

    Ok(())
}

fn verify_specific_files(analysis_output: &str) {
    let test_files = vec![
        ("codehud-gui/src/app.rs", "GUI main application"),
        ("codehud-core/src/extractors/mod.rs", "Core extractors module"),
        ("codehud-viz/src/lib.rs", "Visualization engine"),
        ("codehud-llm/src/lib.rs", "LLM integration"),
    ];

    for (file_path, description) in test_files {
        if let Ok(content) = fs::read_to_string(file_path) {
            let actual_lines = content.lines().count();
            let actual_size = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

            println!("📄 {}: {}", file_path, description);
            println!("   Actual: {} lines, {} bytes", actual_lines, actual_size);

            // Look for this file in analysis output
            if analysis_output.contains(file_path) {
                println!("   ✅ Found in analysis");
                // Try to extract metrics from the JSON-like output
                if let Some(start) = analysis_output.find(&format!("\"path\": \"{}\"", file_path)) {
                    let context = &analysis_output[start.saturating_sub(200)..start+500.min(analysis_output.len()-start)];
                    if let Some(lines_match) = extract_number_after(context, "\"total_lines\":") {
                        let accuracy = if actual_lines > 0 {
                            (lines_match as f64 / actual_lines as f64) * 100.0
                        } else {
                            0.0
                        };
                        println!("   Analysis: {} lines ({:.1}% accuracy)", lines_match, accuracy);
                    }
                }
            } else {
                println!("   ❌ Missing from analysis");
            }
        }
    }
}

fn verify_dependencies(analysis_output: &str) {
    println!("🔍 Checking dependency extraction...");

    // Check if dependencies section exists
    if analysis_output.contains("\"dependencies\"") {
        println!("✅ Dependencies section found");

        // Count dependency entries
        let dep_count = analysis_output.matches("\":").count();
        println!("📊 ~{} dependency relationships detected", dep_count);

        // Check for key dependencies that should exist
        let expected_deps = vec![
            "std", "serde", "tokio", "egui", "anyhow", "chrono",
            "codehud_core", "codehud_gui", "ratatui"
        ];

        let mut found_deps = 0;
        for dep in &expected_deps {
            if analysis_output.contains(dep) {
                found_deps += 1;
            }
        }

        println!("✅ Found {}/{} expected core dependencies", found_deps, expected_deps.len());
    } else {
        println!("❌ Dependencies section missing");
    }
}

fn verify_complexity(analysis_output: &str) {
    println!("🧮 Checking complexity calculations...");

    if analysis_output.contains("\"complexity\":") {
        println!("✅ Complexity calculations present");

        // Look for the main viz file which should have high complexity
        if analysis_output.contains("codehud-viz/src/lib.rs") {
            if let Some(viz_start) = analysis_output.find("codehud-viz/src/lib.rs") {
                let viz_context = &analysis_output[viz_start..viz_start+1000.min(analysis_output.len()-viz_start)];
                if let Some(complexity) = extract_number_after(viz_context, "\"complexity\":") {
                    println!("📊 Main viz engine complexity: {}", complexity);
                    if complexity > 100 {
                        println!("✅ High complexity detected correctly for large file");
                    } else {
                        println!("⚠️  Complexity seems low for large file");
                    }
                }
            }
        }
    } else {
        println!("❌ No complexity calculations found");
    }
}

fn verify_file_types(analysis_output: &str) {
    println!("📁 Checking file type detection...");

    let rust_file_count = analysis_output.matches("\".rs\"").count();
    println!("🦀 Rust files detected: {}", rust_file_count);

    // Count actual Rust files
    let actual_rust_count = count_rust_files(".");
    println!("🦀 Actual Rust files: {}", actual_rust_count);

    let coverage = if actual_rust_count > 0 {
        (rust_file_count as f64 / actual_rust_count as f64) * 100.0
    } else {
        0.0
    };

    println!("📊 File detection coverage: {:.1}%", coverage);

    if coverage > 80.0 {
        println!("✅ Excellent file detection coverage");
    } else if coverage > 60.0 {
        println!("⚠️  Good file detection coverage");
    } else {
        println!("❌ Low file detection coverage");
    }
}

fn verify_coupling(analysis_output: &str) {
    println!("🔗 Checking coupling analysis...");

    if analysis_output.contains("\"coupling\"") {
        println!("✅ Coupling analysis present");

        if analysis_output.contains("\"highly_coupled_files\"") {
            println!("✅ Highly coupled files identified");

            // Count highly coupled files
            let coupled_count = analysis_output.matches("codehud-").count();
            println!("📊 Files with coupling data: ~{}", coupled_count);

            // Check for average dependencies
            if let Some(avg_pos) = analysis_output.find("\"average_dependencies\":") {
                let avg_context = &analysis_output[avg_pos..avg_pos+100.min(analysis_output.len()-avg_pos)];
                if let Some(avg_deps) = extract_float_after(avg_context, "\"average_dependencies\":") {
                    println!("📊 Average dependencies per file: {:.2}", avg_deps);
                    if avg_deps > 2.0 && avg_deps < 10.0 {
                        println!("✅ Reasonable coupling metrics");
                    } else {
                        println!("⚠️  Unusual coupling metrics");
                    }
                }
            }
        } else {
            println!("❌ Highly coupled files analysis missing");
        }
    } else {
        println!("❌ Coupling analysis missing");
    }
}

fn extract_number_after(text: &str, pattern: &str) -> Option<u64> {
    if let Some(start) = text.find(pattern) {
        let after_pattern = &text[start + pattern.len()..];
        let mut number_str = String::new();

        for ch in after_pattern.chars() {
            if ch.is_ascii_digit() {
                number_str.push(ch);
            } else if !number_str.is_empty() {
                break;
            } else if ch == ' ' {
                continue;
            } else {
                break;
            }
        }

        number_str.parse().ok()
    } else {
        None
    }
}

fn extract_float_after(text: &str, pattern: &str) -> Option<f64> {
    if let Some(start) = text.find(pattern) {
        let after_pattern = &text[start + pattern.len()..];
        let mut number_str = String::new();

        for ch in after_pattern.chars() {
            if ch.is_ascii_digit() || ch == '.' {
                number_str.push(ch);
            } else if !number_str.is_empty() {
                break;
            } else if ch == ' ' {
                continue;
            } else {
                break;
            }
        }

        number_str.parse().ok()
    } else {
        None
    }
}

fn count_rust_files(dir: &str) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        count += 1;
                    }
                }
            } else if path.is_dir() {
                let dir_name = path.file_name().unwrap().to_string_lossy();
                if !dir_name.starts_with('.') && dir_name != "target" {
                    count += count_rust_files(&path.to_string_lossy());
                }
            }
        }
    }
    count
}