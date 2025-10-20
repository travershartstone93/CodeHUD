//! Progress Monitor for Background Execution
//!
//! Displays a popup terminal window with progress bar when the main process
//! runs in the background, ensuring visibility of long-running operations.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Progress information shared between main process and popup window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Current step being processed
    pub current_step: u64,
    /// Total number of steps
    pub total_steps: u64,
    /// Current file being processed
    pub current_file: u64,
    /// Total files to process
    pub total_files: u64,
    /// Current operation description
    pub message: String,
    /// Whether the operation is complete
    pub completed: bool,
    /// Elapsed time in seconds
    pub elapsed_seconds: f64,
    /// Any error message
    pub error: Option<String>,
}

/// Dynamic progress tracker that can add steps on-the-fly
pub struct DynamicProgress {
    pub current: u64,
    pub total: u64,
}

impl DynamicProgress {
    pub fn new() -> Self {
        Self { current: 0, total: 0 }
    }

    /// Add steps to the total (e.g., when files are discovered)
    pub fn add_steps(&mut self, count: u64) {
        self.total += count;
    }

    /// Increment current step
    pub fn inc(&mut self) {
        self.current += 1;
    }

    /// Get progress percentage
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self {
            current_step: 0,
            total_steps: 0,
            current_file: 0,
            total_files: 0,
            message: "Starting...".to_string(),
            completed: false,
            elapsed_seconds: 0.0,
            error: None,
        }
    }
}

/// Progress monitor that can spawn popup windows for background processes
pub struct ProgressMonitor {
    /// Path to the progress file
    progress_file: PathBuf,
    /// Whether to use popup mode
    use_popup: bool,
    /// Start time for elapsed calculation
    start_time: std::time::Instant,
}

impl ProgressMonitor {
    /// Create a new progress monitor
    pub fn new() -> Self {
        let progress_file = std::env::temp_dir().join("codehud_progress.json");
        let use_popup = Self::should_use_popup();

        Self {
            progress_file,
            use_popup,
            start_time: std::time::Instant::now(),
        }
    }

    /// Detect if we should use popup mode (running in background)
    fn should_use_popup() -> bool {
        // Check if we're running in background by testing if stdout is a terminal
        use std::io::IsTerminal;

        // If stdout is not a terminal, we're likely running in background
        if !std::io::stdout().is_terminal() {
            return true;
        }

        // Also check for explicit background indicators
        if std::env::var("CODEHUD_BACKGROUND").is_ok() {
            return true;
        }

        // Check if DISPLAY is set (we're in GUI environment) and no terminal
        if std::env::var("DISPLAY").is_ok() && !std::io::stdout().is_terminal() {
            return true;
        }

        false
    }

    /// Initialize progress monitoring (spawn popup if needed)
    pub fn init(&self, total_steps: u64, title: &str) -> Result<()> {
        let progress = ProgressInfo {
            total_steps,
            message: format!("Starting {}", title),
            ..Default::default()
        };

        // Write initial progress
        self.write_progress(&progress)?;

        if self.use_popup {
            self.spawn_popup_window(title)?;
        }

        Ok(())
    }

    /// Update progress
    pub fn update(&self, current_step: u64, message: &str) -> Result<()> {
        let prev = self.read_progress()?;
        let progress = ProgressInfo {
            current_step,
            total_steps: prev.total_steps,
            current_file: prev.current_file,
            total_files: prev.total_files,
            message: message.to_string(),
            completed: false,
            elapsed_seconds: self.start_time.elapsed().as_secs_f64(),
            error: None,
        };

        self.write_progress(&progress)
    }

    /// Add steps to total dynamically (called when new work is discovered)
    pub fn add_steps(&self, additional_steps: u64) -> Result<()> {
        let mut progress = self.read_progress()?;
        progress.total_steps += additional_steps;
        self.write_progress(&progress)
    }

    /// Set total steps (useful for recalculating after discovery)
    pub fn set_total_steps(&self, total: u64) -> Result<()> {
        let mut progress = self.read_progress()?;
        progress.total_steps = total;
        self.write_progress(&progress)
    }

    /// Update progress with file count
    pub fn update_with_files(&self, current_step: u64, current_file: u64, message: &str) -> Result<()> {
        let prev = self.read_progress()?;
        let progress = ProgressInfo {
            current_step,
            total_steps: prev.total_steps,
            current_file,
            total_files: prev.total_files,
            message: message.to_string(),
            completed: false,
            elapsed_seconds: self.start_time.elapsed().as_secs_f64(),
            error: None,
        };

        self.write_progress(&progress)
    }

    /// Mark as completed
    pub fn complete(&self, message: &str) -> Result<()> {
        let mut progress = self.read_progress()?;
        progress.completed = true;
        progress.message = message.to_string();
        progress.elapsed_seconds = self.start_time.elapsed().as_secs_f64();

        self.write_progress(&progress)?;

        // Clean up after a short delay
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = fs::remove_file(&self.progress_file);

        Ok(())
    }

    /// Mark as failed
    pub fn fail(&self, error: &str) -> Result<()> {
        let mut progress = self.read_progress()?;
        progress.completed = true;
        progress.error = Some(error.to_string());
        progress.elapsed_seconds = self.start_time.elapsed().as_secs_f64();

        self.write_progress(&progress)?;

        // Clean up after a longer delay for errors
        std::thread::sleep(std::time::Duration::from_secs(5));
        let _ = fs::remove_file(&self.progress_file);

        Ok(())
    }

    /// Write progress to shared file
    fn write_progress(&self, progress: &ProgressInfo) -> Result<()> {
        let json = serde_json::to_string_pretty(progress)?;
        fs::write(&self.progress_file, json)?;
        Ok(())
    }

    /// Read progress from shared file
    fn read_progress(&self) -> Result<ProgressInfo> {
        if !self.progress_file.exists() {
            return Ok(ProgressInfo::default());
        }

        let content = fs::read_to_string(&self.progress_file)?;
        let progress = serde_json::from_str(&content)?;
        Ok(progress)
    }

    /// Spawn a popup terminal window to display progress
    fn spawn_popup_window(&self, title: &str) -> Result<()> {
        let progress_file = self.progress_file.clone();

        let progress_file_str = progress_file.display().to_string();

        // Create a script that monitors the progress file
        let script = format!(r#"#!/bin/bash
echo "📊 CodeHUD Progress Monitor - {title}"
echo "================================"
echo ""

while true; do
    if [ -f "{file}" ]; then
        # Clear screen and show progress
        clear
        echo "📊 CodeHUD Progress Monitor - {title}"
        echo "================================"
        echo ""

        # Parse JSON and display progress bar
        if command -v jq > /dev/null 2>&1; then
            current=$(jq -r '.current_step // 0' "{file}" 2>/dev/null || echo "0")
            total=$(jq -r '.total_steps // 1' "{file}" 2>/dev/null || echo "1")
            current_file=$(jq -r '.current_file // 0' "{file}" 2>/dev/null || echo "0")
            total_files=$(jq -r '.total_files // 0' "{file}" 2>/dev/null || echo "0")
            message=$(jq -r '.message // "Processing..."' "{file}" 2>/dev/null || echo "Processing...")
            completed=$(jq -r '.completed // false' "{file}" 2>/dev/null || echo "false")
            elapsed=$(jq -r '.elapsed_seconds // 0' "{file}" 2>/dev/null || echo "0")
            error=$(jq -r '.error // empty' "{file}" 2>/dev/null || echo "")

            if [ "$completed" = "true" ]; then
                if [ -n "$error" ]; then
                    echo "❌ Failed: $error"
                else
                    echo "✅ Completed successfully!"
                    echo "📝 $message"
                fi
                echo "⏱️  Total time: ${{elapsed}}s"
                echo ""
                echo "Press any key to close..."
                read -n 1
                break
            else
                # Calculate percentage
                if [ "$total" -gt 0 ]; then
                    percentage=$((current * 100 / total))
                else
                    percentage=0
                fi

                # Create progress bar
                bar_length=40
                filled_length=$((percentage * bar_length / 100))
                bar=""
                for i in $(seq 1 $filled_length); do bar="${{bar}}█"; done
                for i in $(seq $((filled_length + 1)) $bar_length); do bar="${{bar}}░"; done

                echo "Progress: [$bar] $percentage%"
                echo "Crates: $current/$total"
                if [ "$total_files" -gt 0 ]; then
                    echo "Files:  $current_file/$total_files"
                fi
                echo "Status: $message"
                echo "Elapsed: ${{elapsed}}s"
            fi
        else
            # Fallback without jq
            echo "Monitoring progress..."
            echo "File: {file}"
            if [ -f "{file}" ]; then
                echo "Content:"
                cat "{file}" | head -10
            fi
        fi
    else
        echo "Waiting for progress information..."
    fi

    sleep 1
done
"#, title = title, file = progress_file_str);

        // Write script to temp file
        let script_file = std::env::temp_dir().join("codehud_progress_monitor.sh");
        fs::write(&script_file, script)?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_file)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_file, perms)?;
        }

        // Prepare command arguments with proper lifetimes
        let window_title = format!("CodeHUD Progress - {}", title);
        let script_path = script_file.to_str().unwrap();

        // Try different terminal emulators
        let terminals = [
            ("gnome-terminal", vec!["--title", &window_title, "--", "bash", script_path]),
            ("konsole", vec!["--title", &window_title, "-e", "bash", script_path]),
            ("xterm", vec!["-title", &window_title, "-e", "bash", script_path]),
            ("terminator", vec!["--title", &window_title, "-x", "bash", script_path]),
        ];

        for (terminal, args) in &terminals {
            if Command::new(terminal)
                .args(args)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .is_ok()
            {
                println!("🖥️  Spawned progress window with {}", terminal);
                return Ok(());
            }
        }

        // Fallback: just print a message
        println!("⚠️  Could not spawn progress window. Install gnome-terminal, konsole, xterm, or terminator for better experience.");
        println!("📊 Progress will be logged to console instead.");

        Ok(())
    }
}

impl Default for ProgressMonitor {
    fn default() -> Self {
        Self::new()
    }
}