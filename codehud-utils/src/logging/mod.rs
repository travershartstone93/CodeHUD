//! Logging utilities with Python logging compatibility
//!
//! This module provides logging capabilities that behave similarly
//! to Python's logging module while leveraging Rust's tracing ecosystem.

use tracing::Level;
#[allow(unused_imports)]
use tracing::Subscriber;
use tracing_subscriber::{
    fmt::{self},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
#[allow(unused_imports)]
use tracing_subscriber::{fmt::format::FmtSpan, Layer};
use std::io::{self, Write};

/// Log levels matching Python logging levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 10,
    Info = 20,
    Warning = 30,
    Error = 40,
    Critical = 50,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warning => Level::WARN,
            LogLevel::Error => Level::ERROR,
            LogLevel::Critical => Level::ERROR, // Rust doesn't have CRITICAL, use ERROR
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Critical => "CRITICAL",
        };
        write!(f, "{}", name)
    }
}

/// Logger configuration matching Python logging behavior
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub level: LogLevel,
    pub format: String,
    pub date_format: Option<String>,
    pub include_location: bool,
    pub include_thread_id: bool,
    pub colored_output: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: "%(asctime)s - %(name)s - %(levelname)s - %(message)s".to_string(),
            date_format: Some("%Y-%m-%d %H:%M:%S".to_string()),
            include_location: false,
            include_thread_id: false,
            colored_output: true,
        }
    }
}

/// Initialize logging with configuration
pub fn init_logging(config: LoggerConfig) -> crate::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(Level::from(config.level).into())
        .from_env_lossy();

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(config.include_thread_id)
        .with_file(config.include_location)
        .with_line_number(config.include_location)
        .with_ansi(config.colored_output);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| crate::UtilError::Config(format!("Failed to initialize logging: {}", e)))?;

    Ok(())
}

/// Initialize basic logging (equivalent to Python's basicConfig)
pub fn basic_config(level: Option<LogLevel>) -> crate::Result<()> {
    let config = LoggerConfig {
        level: level.unwrap_or(LogLevel::Info),
        ..Default::default()
    };
    init_logging(config)
}

/// Logger struct that mimics Python logger behavior
#[derive(Debug, Clone)]
pub struct Logger {
    name: String,
    level: LogLevel,
}

impl Logger {
    /// Create a new logger with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            level: LogLevel::Info,
        }
    }

    /// Set the logging level
    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    /// Get the current logging level
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Check if a level is enabled
    pub fn is_enabled(&self, level: LogLevel) -> bool {
        level >= self.level
    }

    /// Log a debug message
    pub fn debug(&self, message: &str) {
        if self.is_enabled(LogLevel::Debug) {
            tracing::debug!("[{}] {}", self.name, message);
        }
    }

    /// Log an info message
    pub fn info(&self, message: &str) {
        if self.is_enabled(LogLevel::Info) {
            tracing::info!("[{}] {}", self.name, message);
        }
    }

    /// Log a warning message
    pub fn warning(&self, message: &str) {
        if self.is_enabled(LogLevel::Warning) {
            tracing::warn!("[{}] {}", self.name, message);
        }
    }

    /// Log an error message
    pub fn error(&self, message: &str) {
        if self.is_enabled(LogLevel::Error) {
            tracing::error!("[{}] {}", self.name, message);
        }
    }

    /// Log a critical message
    pub fn critical(&self, message: &str) {
        if self.is_enabled(LogLevel::Critical) {
            tracing::error!("[{}] CRITICAL: {}", self.name, message);
        }
    }

    /// Log an exception (error with extra context)
    pub fn exception(&self, message: &str, error: &dyn std::error::Error) {
        tracing::error!("[{}] {}: {}", self.name, message, error);
    }
}

/// Get a logger by name (equivalent to Python's logging.getLogger)
pub fn get_logger(name: &str) -> Logger {
    Logger::new(name)
}

/// Structured logging macros that work with tracing
#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.is_enabled($crate::logging::LogLevel::Debug) {
            tracing::debug!(target: &$logger.name, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.is_enabled($crate::logging::LogLevel::Info) {
            tracing::info!(target: &$logger.name, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.is_enabled($crate::logging::LogLevel::Warning) {
            tracing::warn!(target: &$logger.name, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.is_enabled($crate::logging::LogLevel::Error) {
            tracing::error!(target: &$logger.name, $($arg)*);
        }
    };
}

/// File handler for logging to files (equivalent to Python's FileHandler)
pub struct FileHandler {
    file: std::fs::File,
}

impl FileHandler {
    /// Create a new file handler
    pub fn new(filename: &std::path::Path) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)?;
        
        Ok(Self { file })
    }

    /// Write a log record to the file
    pub fn write_record(&mut self, level: LogLevel, target: &str, message: &str) -> io::Result<()> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(self.file, "{} - {} - {} - {}", timestamp, target, level, message)
    }
}

/// Rotating file handler (basic implementation)
pub struct RotatingFileHandler {
    base_filename: std::path::PathBuf,
    current_file: std::fs::File,
    max_bytes: u64,
    backup_count: u32,
    current_size: u64,
}

impl RotatingFileHandler {
    /// Create a new rotating file handler
    pub fn new(
        filename: &std::path::Path, 
        max_bytes: u64, 
        backup_count: u32
    ) -> io::Result<Self> {
        let current_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)?;

        let current_size = current_file.metadata()?.len();

        Ok(Self {
            base_filename: filename.to_path_buf(),
            current_file,
            max_bytes,
            backup_count,
            current_size,
        })
    }

    /// Write a log record, rotating if necessary
    pub fn write_record(&mut self, level: LogLevel, target: &str, message: &str) -> io::Result<()> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let log_line = format!("{} - {} - {} - {}\n", timestamp, target, level, message);
        
        // Check if rotation is needed
        if self.current_size + log_line.len() as u64 > self.max_bytes {
            self.rotate()?;
        }
        
        self.current_file.write_all(log_line.as_bytes())?;
        self.current_size += log_line.len() as u64;
        
        Ok(())
    }

    /// Perform log rotation
    fn rotate(&mut self) -> io::Result<()> {
        // Close current file
        drop(std::mem::replace(&mut self.current_file, unsafe { std::mem::zeroed() }));

        // Rotate backup files
        for i in (1..self.backup_count).rev() {
            let old_name = format!("{}.{}", self.base_filename.to_string_lossy(), i);
            let new_name = format!("{}.{}", self.base_filename.to_string_lossy(), i + 1);
            
            if std::path::Path::new(&old_name).exists() {
                std::fs::rename(&old_name, &new_name)?;
            }
        }

        // Move current log to .1
        if self.backup_count > 0 {
            let backup_name = format!("{}.1", self.base_filename.to_string_lossy());
            std::fs::rename(&self.base_filename, &backup_name)?;
        }

        // Create new log file
        self.current_file = std::fs::File::create(&self.base_filename)?;
        self.current_size = 0;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_log_levels() {
        assert!(LogLevel::Critical > LogLevel::Error);
        assert!(LogLevel::Error > LogLevel::Warning);
        assert!(LogLevel::Warning > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
    }

    #[test]
    fn test_logger_creation() {
        let logger = get_logger("test.module");
        assert_eq!(logger.name, "test.module");
        assert_eq!(logger.level(), LogLevel::Info);
    }

    #[test]
    fn test_logger_level_filtering() {
        let mut logger = Logger::new("test");
        logger.set_level(LogLevel::Warning);
        
        assert!(!logger.is_enabled(LogLevel::Debug));
        assert!(!logger.is_enabled(LogLevel::Info));
        assert!(logger.is_enabled(LogLevel::Warning));
        assert!(logger.is_enabled(LogLevel::Error));
        assert!(logger.is_enabled(LogLevel::Critical));
    }

    #[test]
    fn test_file_handler() -> io::Result<()> {
        let temp_dir = tempdir()?;
        let log_path = temp_dir.path().join("test.log");
        
        let mut handler = FileHandler::new(&log_path)?;
        handler.write_record(LogLevel::Info, "test", "Test message")?;
        
        let content = std::fs::read_to_string(&log_path)?;
        assert!(content.contains("INFO"));
        assert!(content.contains("test"));
        assert!(content.contains("Test message"));
        
        Ok(())
    }

    #[test]
    fn test_rotating_file_handler() -> io::Result<()> {
        let temp_dir = tempdir()?;
        let log_path = temp_dir.path().join("rotating.log");
        
        let mut handler = RotatingFileHandler::new(&log_path, 100, 2)?;
        
        // Write enough to trigger rotation
        for i in 0..10 {
            handler.write_record(LogLevel::Info, "test", &format!("Message {}", i))?;
        }
        
        // Check that backup files were created
        let backup1 = temp_dir.path().join("rotating.log.1");
        assert!(backup1.exists());
        
        Ok(())
    }
}