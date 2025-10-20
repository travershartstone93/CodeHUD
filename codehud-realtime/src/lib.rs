//! CodeHUD Real-time - Real-time analysis and file watching
//!
//! This crate provides real-time file system monitoring and incremental
//! analysis capabilities matching the Python implementation.

#![warn(clippy::all, clippy::pedantic)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use codehud_core::{
    extractors::{BaseDataExtractor, TopologyExtractor, QualityExtractor, SecurityExtractor},
    models::{AnalysisResult, ViewType},
    Pipeline,
};
use codehud_utils::logging::get_logger;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc, RwLock},
    time::{interval, timeout},
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

/// Real-time file monitoring and analysis system
pub struct RealtimeMonitor {
    codebase_path: PathBuf,
    pipeline: Pipeline,
    watcher: Option<RecommendedWatcher>,
    analysis_cache: Arc<RwLock<AnalysisCache>>,
    config: MonitorConfig,
}

/// Configuration for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Debounce delay for file changes (milliseconds)
    pub debounce_ms: u64,
    /// Maximum files to analyze in a single batch
    pub batch_size: usize,
    /// Analysis timeout per file (seconds)
    pub analysis_timeout_secs: u64,
    /// Enable incremental analysis (only changed files)
    pub incremental: bool,
    /// File patterns to watch
    pub watch_patterns: Vec<String>,
    /// File patterns to ignore
    pub ignore_patterns: Vec<String>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            batch_size: 10,
            analysis_timeout_secs: 30,
            incremental: true,
            watch_patterns: vec![
                "*.py".to_string(),
                "*.js".to_string(),
                "*.ts".to_string(),
                "*.rs".to_string(),
            ],
            ignore_patterns: vec![
                ".git/**".to_string(),
                "__pycache__/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                ".venv/**".to_string(),
            ],
        }
    }
}

/// Cache for analysis results
#[derive(Debug, Default)]
struct AnalysisCache {
    file_hashes: HashMap<PathBuf, String>,
    file_results: HashMap<PathBuf, FileAnalysisResult>,
    last_full_analysis: Option<DateTime<Utc>>,
}

/// Result of analyzing a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResult {
    pub file_path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub topology_data: Option<serde_json::Value>,
    pub quality_data: Option<serde_json::Value>,
    pub security_data: Option<serde_json::Value>,
    pub analysis_duration_ms: u64,
    pub errors: Vec<String>,
}

/// Events emitted by the real-time monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitorEvent {
    /// File was modified
    FileChanged {
        path: PathBuf,
        timestamp: DateTime<Utc>,
    },
    /// Analysis started for a batch of files
    AnalysisStarted {
        files: Vec<PathBuf>,
        timestamp: DateTime<Utc>,
    },
    /// Analysis completed for a file
    FileAnalyzed {
        result: FileAnalysisResult,
    },
    /// Batch analysis completed
    BatchCompleted {
        files_analyzed: usize,
        total_duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Error occurred during monitoring
    Error {
        message: String,
        timestamp: DateTime<Utc>,
    },
}

impl RealtimeMonitor {
    /// Create a new real-time monitor
    pub fn new(codebase_path: impl AsRef<Path>, pipeline: Pipeline) -> Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();

        if !codebase_path.exists() {
            return Err(anyhow::anyhow!(
                "Codebase path does not exist: {}",
                codebase_path.display()
            ));
        }

        Ok(Self {
            codebase_path,
            pipeline,
            watcher: None,
            analysis_cache: Arc::new(RwLock::new(AnalysisCache::default())),
            config: MonitorConfig::default(),
        })
    }

    /// Configure the monitor
    pub fn with_config(mut self, config: MonitorConfig) -> Self {
        self.config = config;
        self
    }

    /// Start real-time monitoring
    pub async fn start(&mut self) -> Result<mpsc::Receiver<MonitorEvent>> {
        let logger = get_logger("codehud.realtime");
        let (event_tx, event_rx) = mpsc::channel(1000);
        let (file_tx, file_rx) = mpsc::channel(100);

        // Set up file watcher
        let file_tx_clone = file_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    if let Err(e) = file_tx_clone.try_send(event) {
                        eprintln!("Failed to send file event: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("File watcher error: {}", e);
                }
            }
        })?;

        // Watch the codebase directory
        watcher.watch(&self.codebase_path, RecursiveMode::Recursive)
            .context("Failed to start watching directory")?;

        self.watcher = Some(watcher);

        // Start the event processing loop
        let codebase_path = self.codebase_path.clone();
        let pipeline = self.pipeline.clone();
        let config = self.config.clone();
        let cache = self.analysis_cache.clone();

        tokio::spawn(async move {
            Self::process_events(
                file_rx,
                event_tx,
                codebase_path,
                pipeline,
                config,
                cache,
            ).await
        });

        logger.info(&format!("Real-time monitoring started for {}", self.codebase_path.display()));
        Ok(event_rx)
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
            let logger = get_logger("codehud.realtime");
            logger.info("Real-time monitoring stopped");
        }
    }

    /// Process file system events
    async fn process_events(
        mut file_rx: mpsc::Receiver<Event>,
        event_tx: mpsc::Sender<MonitorEvent>,
        codebase_path: PathBuf,
        pipeline: Pipeline,
        config: MonitorConfig,
        cache: Arc<RwLock<AnalysisCache>>,
    ) {
        let logger = get_logger("codehud.realtime.processor");
        let mut debounce_timer = interval(Duration::from_millis(config.debounce_ms));
        let mut pending_files = HashSet::new();

        loop {
            tokio::select! {
                // Handle file system events
                event = file_rx.recv() => {
                    match event {
                        Some(event) => {
                            if let Some(files) = Self::extract_relevant_files(&event, &config) {
                                for file in files {
                                    if Self::should_analyze_file(&file, &codebase_path, &config) {
                                        pending_files.insert(file);
                                    }
                                }
                            }
                        }
                        None => {
                            logger.info("File watcher channel closed");
                            break;
                        }
                    }
                }

                // Handle debounced batch processing
                _ = debounce_timer.tick() => {
                    if !pending_files.is_empty() {
                        let files_to_analyze: Vec<_> = pending_files.drain().collect();
                        let batch_size = config.batch_size.min(files_to_analyze.len());

                        for chunk in files_to_analyze.chunks(batch_size) {
                            let files = chunk.to_vec();
                            let event_tx = event_tx.clone();
                            let codebase_path = codebase_path.clone();
                            let pipeline = pipeline.clone();
                            let config = config.clone();
                            let cache = cache.clone();

                            tokio::spawn(async move {
                                Self::analyze_file_batch(
                                    files,
                                    event_tx,
                                    codebase_path,
                                    pipeline,
                                    config,
                                    cache,
                                ).await;
                            });
                        }
                    }
                }
            }
        }
    }

    /// Extract relevant files from a file system event
    fn extract_relevant_files(event: &Event, _config: &MonitorConfig) -> Option<Vec<PathBuf>> {
        match &event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                Some(event.paths.clone())
            }
            _ => None,
        }
    }

    /// Check if a file should be analyzed
    fn should_analyze_file(file: &Path, codebase_path: &Path, config: &MonitorConfig) -> bool {
        // Must be within codebase
        if !file.starts_with(codebase_path) {
            return false;
        }

        // Check ignore patterns
        let relative_path = file.strip_prefix(codebase_path).unwrap_or(file);
        let relative_str = relative_path.to_string_lossy();

        for ignore_pattern in &config.ignore_patterns {
            if glob_match(ignore_pattern, &relative_str) {
                return false;
            }
        }

        // Check watch patterns
        for watch_pattern in &config.watch_patterns {
            if glob_match(watch_pattern, &relative_str) {
                return true;
            }
        }

        false
    }

    /// Analyze a batch of files
    async fn analyze_file_batch(
        files: Vec<PathBuf>,
        event_tx: mpsc::Sender<MonitorEvent>,
        codebase_path: PathBuf,
        _pipeline: Pipeline,
        config: MonitorConfig,
        cache: Arc<RwLock<AnalysisCache>>,
    ) {
        let logger = get_logger("codehud.realtime.analyzer");
        let start_time = std::time::Instant::now();

        // Notify analysis started
        let _ = event_tx.send(MonitorEvent::AnalysisStarted {
            files: files.clone(),
            timestamp: Utc::now(),
        }).await;

        let mut files_analyzed = 0;

        for file_path in files {
            // Check if we need to analyze this file (incremental analysis)
            if config.incremental {
                if let Ok(should_skip) = Self::should_skip_file(&file_path, &cache).await {
                    if should_skip {
                        continue;
                    }
                }
            }

            // Analyze the file with timeout
            let analysis_future = Self::analyze_single_file(&file_path, &codebase_path);
            let timeout_duration = Duration::from_secs(config.analysis_timeout_secs);

            match timeout(timeout_duration, analysis_future).await {
                Ok(Ok(result)) => {
                    // Update cache
                    Self::update_cache(&file_path, &result, &cache).await;

                    // Send result
                    let _ = event_tx.send(MonitorEvent::FileAnalyzed { result }).await;
                    files_analyzed += 1;
                }
                Ok(Err(e)) => {
                    logger.error(&format!("Analysis failed for {}: {}", file_path.display(), e));
                    let _ = event_tx.send(MonitorEvent::Error {
                        message: format!("Analysis failed for {}: {}", file_path.display(), e),
                        timestamp: Utc::now(),
                    }).await;
                }
                Err(_) => {
                    logger.warning(&format!("Analysis timeout for {}", file_path.display()));
                    let _ = event_tx.send(MonitorEvent::Error {
                        message: format!("Analysis timeout for {}", file_path.display()),
                        timestamp: Utc::now(),
                    }).await;
                }
            }
        }

        // Notify batch completed
        let total_duration = start_time.elapsed();
        let _ = event_tx.send(MonitorEvent::BatchCompleted {
            files_analyzed,
            total_duration_ms: total_duration.as_millis() as u64,
            timestamp: Utc::now(),
        }).await;
    }

    /// Analyze a single file
    async fn analyze_single_file(
        file_path: &Path,
        codebase_path: &Path,
    ) -> Result<FileAnalysisResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();

        // Initialize extractors
        let topology_extractor = TopologyExtractor::new(codebase_path);
        let quality_extractor = QualityExtractor::new(codebase_path);
        let security_extractor = SecurityExtractor::new(codebase_path);

        let mut topology_data = None;
        let mut quality_data = None;
        let mut security_data = None;

        // Run topology analysis
        if let Ok(extractor) = topology_extractor {
            match extractor.extract_data() {
                Ok(data) => topology_data = Some(serde_json::to_value(data)?),
                Err(e) => errors.push(format!("Topology analysis failed: {}", e)),
            }
        }

        // Run quality analysis
        if let Ok(extractor) = quality_extractor {
            match extractor.extract_data() {
                Ok(data) => quality_data = Some(serde_json::to_value(data)?),
                Err(e) => errors.push(format!("Quality analysis failed: {}", e)),
            }
        }

        // Run security analysis
        if let Ok(extractor) = security_extractor {
            match extractor.extract_data() {
                Ok(data) => security_data = Some(serde_json::to_value(data)?),
                Err(e) => errors.push(format!("Security analysis failed: {}", e)),
            }
        }

        let analysis_duration = start_time.elapsed();

        Ok(FileAnalysisResult {
            file_path: file_path.to_path_buf(),
            timestamp: Utc::now(),
            topology_data,
            quality_data,
            security_data,
            analysis_duration_ms: analysis_duration.as_millis() as u64,
            errors,
        })
    }

    /// Check if file should be skipped (for incremental analysis)
    async fn should_skip_file(
        file_path: &Path,
        cache: &Arc<RwLock<AnalysisCache>>,
    ) -> Result<bool> {
        let file_hash = Self::calculate_file_hash(file_path)?;
        let cache_guard = cache.read().await;

        if let Some(cached_hash) = cache_guard.file_hashes.get(file_path) {
            Ok(*cached_hash == file_hash)
        } else {
            Ok(false)
        }
    }

    /// Update analysis cache
    async fn update_cache(
        file_path: &Path,
        result: &FileAnalysisResult,
        cache: &Arc<RwLock<AnalysisCache>>,
    ) {
        if let Ok(file_hash) = Self::calculate_file_hash(file_path) {
            let mut cache_guard = cache.write().await;
            cache_guard.file_hashes.insert(file_path.to_path_buf(), file_hash);
            cache_guard.file_results.insert(file_path.to_path_buf(), result.clone());
        }
    }

    /// Calculate hash of file contents
    fn calculate_file_hash(file_path: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = std::fs::read_to_string(file_path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple implementation - in production would use glob crate
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    }

    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    }

    pattern == text
}