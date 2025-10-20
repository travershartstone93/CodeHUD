// Simple test file to verify file-to-crate mapping
use std::path::PathBuf;

fn main() {
    println!("Testing file-to-crate mapping");

    let test_files = vec![
        "/home/travers/Desktop/CodeHUD (copy)/Rust_copy/codehud-core/src/lib.rs",
        "/home/travers/Desktop/CodeHUD (copy)/Rust_copy/codehud-cli/src/main.rs",
        "/home/travers/Desktop/CodeHUD (copy)/Rust_copy/codehud-viz/src/lib.rs",
    ];

    for file in test_files {
        println!("File: {}", file);
        // Check which crate this file should belong to
        if file.contains("codehud-core") {
            println!("  Should map to: codehud-core");
        } else if file.contains("codehud-cli") {
            println!("  Should map to: codehud-cli");
        } else if file.contains("codehud-viz") {
            println!("  Should map to: codehud-viz");
        }
    }
}