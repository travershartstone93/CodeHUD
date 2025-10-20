fn main() {
    println!("=== RUST CODEHUD SELF-ANALYSIS VERIFICATION ===");

    // Read the existing analysis file
    let analysis_content = std::fs::read_to_string(".codehud_analysis.json")
        .expect("Failed to read analysis file");

    println!("Analysis file found: {} bytes", analysis_content.len());

    // Count actual Rust files
    let rust_files = std::fs::read_dir(".")
        .expect("Failed to read directory")
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let path = entry.path();
            count_rust_files_in_dir(&path)
        })
        .sum::<usize>();

    println!("Actual .rs files found: {}", rust_files);

    // Extract file count from analysis
    if let Some(start) = analysis_content.find("\"total_files\":") {
        if let Some(end) = analysis_content[start..].find(',') {
            let total_files_str = &analysis_content[start + 14..start + end];
            let total_files: usize = total_files_str.trim().parse().unwrap_or(0);
            println!("Analysis reports: {} files", total_files);

            let accuracy = if rust_files > 0 {
                (total_files as f64 / rust_files as f64) * 100.0
            } else {
                0.0
            };
            println!("Coverage accuracy: {:.1}%", accuracy);
        }
    }

    // Test specific key files
    println!("\n=== KEY FILE VERIFICATION ===");
    verify_file_analysis("codehud-viz/src/lib.rs", &analysis_content);
    verify_file_analysis("codehud-gui/src/app.rs", &analysis_content);
    verify_file_analysis("codehud-core/src/analysis/mod.rs", &analysis_content);
}

fn count_rust_files_in_dir(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
        .flatten()
        .filter(|entry| {
            let path = entry.path();
            if path.is_file() {
                path.extension().map_or(false, |ext| ext == "rs")
            } else if path.is_dir() && !path.file_name().unwrap().to_string_lossy().starts_with('.') {
                count_rust_files_in_dir(&path) > 0
            } else {
                false
            }
        })
        .count()
}

fn verify_file_analysis(file_path: &str, analysis_content: &str) {
    if let Ok(metadata) = std::fs::metadata(file_path) {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let actual_lines = content.lines().count();
            let actual_size = metadata.len();

            println!("File: {}", file_path);
            println!("  Actual lines: {}", actual_lines);
            println!("  Actual size: {} bytes", actual_size);

            // Try to find this file in analysis
            if let Some(file_start) = analysis_content.find(&format!("\"path\": \"{}\"", file_path)) {
                if let Some(context_start) = analysis_content[..file_start].rfind('{') {
                    if let Some(context_end) = analysis_content[file_start..].find('}') {
                        let file_context = &analysis_content[context_start..file_start + context_end + 1];

                        if let Some(lines_start) = file_context.find("\"total_lines\":") {
                            if let Some(lines_end) = file_context[lines_start..].find(',') {
                                let lines_str = &file_context[lines_start + 14..lines_start + lines_end];
                                if let Ok(analyzed_lines) = lines_str.trim().parse::<usize>() {
                                    let accuracy = if actual_lines > 0 {
                                        (analyzed_lines as f64 / actual_lines as f64) * 100.0
                                    } else {
                                        0.0
                                    };
                                    println!("  Analyzed lines: {}", analyzed_lines);
                                    println!("  Line accuracy: {:.1}%", accuracy);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}