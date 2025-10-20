//! Evolution Data Extractor - Analyzes code evolution patterns and version history

use super::BaseDataExtractor;
use crate::external_tools::ExternalToolManager;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc, NaiveDateTime};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileEvolution {
    file_path: String,
    total_commits: usize,
    lines_added: i32,
    lines_removed: i32,
    first_commit: String,
    last_commit: String,
    commit_frequency: f64, // commits per month
    authors: Vec<String>,
    primary_author: String,
    stability_score: f64,
    complexity_trend: String, // "increasing", "decreasing", "stable"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitPattern {
    pattern_type: String,
    description: String,
    frequency: usize,
    files_affected: Vec<String>,
    time_pattern: String, // "weekdays", "weekends", "late_night", etc.
    size_pattern: String, // "small", "medium", "large"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthorMetrics {
    author_name: String,
    total_commits: usize,
    files_touched: usize,
    lines_added: i32,
    lines_removed: i32,
    commit_frequency: f64,
    primary_languages: Vec<String>,
    activity_periods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvolutionHotspot {
    file_path: String,
    change_frequency: f64,
    bug_proneness: f64,
    author_count: usize,
    complexity_changes: i32,
    risk_score: f64,
}

pub struct EvolutionExtractor {
    codebase_path: PathBuf,
    extraction_timestamp: DateTime<Utc>,
    external_tools: ExternalToolManager,
    is_git_repo: bool,
}

impl EvolutionExtractor {
    pub fn new(codebase_path: impl AsRef<Path>) -> crate::Result<Self> {
        let codebase_path = codebase_path.as_ref().to_path_buf();
        if !codebase_path.exists() {
            return Err(crate::Error::Config(format!("Codebase path does not exist: {}", codebase_path.display())));
        }

        let external_tools = ExternalToolManager::new(&codebase_path);
        let is_git_repo = codebase_path.join(".git").exists();

        Ok(Self {
            codebase_path,
            extraction_timestamp: Utc::now(),
            external_tools,
            is_git_repo,
        })
    }

    fn get_all_python_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_files_recursive(&self.codebase_path, &mut files);
        files
    }

    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                    files.push(path);
                } else if path.is_dir() && !self.is_excluded_directory(&path) {
                    self.collect_files_recursive(&path, files);
                }
            }
        }
    }

    fn is_excluded_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "__pycache__" | ".git" | ".pytest_cache" | "node_modules" | ".venv" | "venv")
        } else {
            false
        }
    }

    fn analyze_git_history(&self) -> crate::Result<(Vec<FileEvolution>, Vec<AuthorMetrics>, Vec<CommitPattern>)> {
        if !self.is_git_repo {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }

        let files = self.get_all_python_files();
        let mut file_evolutions = Vec::new();
        let mut author_stats: HashMap<String, AuthorMetrics> = HashMap::new();
        let mut commit_patterns = Vec::new();

        // Analyze each file's evolution
        for file_path in &files {
            if let Ok(evolution) = self.analyze_file_evolution(file_path) {
                file_evolutions.push(evolution);
            }
        }

        // Analyze overall commit patterns
        if let Ok(patterns) = self.analyze_commit_patterns() {
            commit_patterns = patterns;
        }

        // Extract author metrics from file evolutions
        for evolution in &file_evolutions {
            for author in &evolution.authors {
                let metrics = author_stats.entry(author.clone()).or_insert_with(|| AuthorMetrics {
                    author_name: author.clone(),
                    total_commits: 0,
                    files_touched: 0,
                    lines_added: 0,
                    lines_removed: 0,
                    commit_frequency: 0.0,
                    primary_languages: vec!["python".to_string()],
                    activity_periods: Vec::new(),
                });

                metrics.files_touched += 1;
                if author == &evolution.primary_author {
                    metrics.total_commits += evolution.total_commits;
                    metrics.lines_added += evolution.lines_added;
                    metrics.lines_removed += evolution.lines_removed;
                }
            }
        }

        let author_metrics: Vec<AuthorMetrics> = author_stats.into_values().collect();

        Ok((file_evolutions, author_metrics, commit_patterns))
    }

    fn analyze_file_evolution(&self, file_path: &Path) -> crate::Result<FileEvolution> {
        let relative_path = file_path.strip_prefix(&self.codebase_path)
            .unwrap_or(file_path)
            .display()
            .to_string();

        // Get git log for this file
        let output = Command::new("git")
            .args(&["log", "--follow", "--pretty=format:%H|%an|%ad", "--date=iso", "--", &relative_path])
            .current_dir(&self.codebase_path)
            .output()
            .map_err(|e| crate::Error::Io(e))?;

        let log_output = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = log_output.lines().collect();

        if lines.is_empty() {
            return Err(crate::Error::Analysis("No git history found for file".to_string()));
        }

        let mut authors = HashSet::new();
        let mut commits = Vec::new();

        for line in &lines {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let commit_hash = parts[0];
                let author = parts[1];
                let date = parts[2];

                authors.insert(author.to_string());
                commits.push((commit_hash.to_string(), author.to_string(), date.to_string()));
            }
        }

        // Get line statistics
        let (lines_added, lines_removed) = self.get_file_line_stats(&relative_path)?;

        // Calculate primary author (most commits)
        let mut author_counts: HashMap<String, usize> = HashMap::new();
        for (_, author, _) in &commits {
            *author_counts.entry(author.clone()).or_insert(0) += 1;
        }

        let primary_author = author_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(author, _)| author.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Calculate commit frequency (commits per month)
        let commit_frequency = if commits.len() > 1 {
            let first_date = &commits.last().unwrap().2;
            let last_date = &commits.first().unwrap().2;

            if let (Ok(first), Ok(last)) = (
                NaiveDateTime::parse_from_str(first_date, "%Y-%m-%d %H:%M:%S %z"),
                NaiveDateTime::parse_from_str(last_date, "%Y-%m-%d %H:%M:%S %z")
            ) {
                let duration = last.signed_duration_since(first);
                let months = duration.num_days() as f64 / 30.0;
                if months > 0.0 {
                    commits.len() as f64 / months
                } else {
                    commits.len() as f64
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate stability score (lower is more stable)
        let stability_score = 1.0 - (commit_frequency / 10.0).min(1.0);

        // Determine complexity trend (simplified)
        let complexity_trend = if lines_added > lines_removed * 2 {
            "increasing".to_string()
        } else if lines_removed > lines_added * 2 {
            "decreasing".to_string()
        } else {
            "stable".to_string()
        };

        Ok(FileEvolution {
            file_path: relative_path,
            total_commits: commits.len(),
            lines_added,
            lines_removed,
            first_commit: commits.last().map(|(hash, _, _)| hash.clone()).unwrap_or_default(),
            last_commit: commits.first().map(|(hash, _, _)| hash.clone()).unwrap_or_default(),
            commit_frequency,
            authors: authors.into_iter().collect(),
            primary_author,
            stability_score,
            complexity_trend,
        })
    }

    fn get_file_line_stats(&self, file_path: &str) -> crate::Result<(i32, i32)> {
        let output = Command::new("git")
            .args(&["log", "--follow", "--numstat", "--pretty=format:", "--", file_path])
            .current_dir(&self.codebase_path)
            .output()
            .map_err(|e| crate::Error::Io(e))?;

        let stats_output = String::from_utf8_lossy(&output.stdout);
        let mut total_added = 0i32;
        let mut total_removed = 0i32;

        for line in stats_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                if let (Ok(added), Ok(removed)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                    total_added += added;
                    total_removed += removed;
                }
            }
        }

        Ok((total_added, total_removed))
    }

    fn analyze_commit_patterns(&self) -> crate::Result<Vec<CommitPattern>> {
        let output = Command::new("git")
            .args(&["log", "--pretty=format:%H|%s|%ad|%an", "--date=iso"])
            .current_dir(&self.codebase_path)
            .output()
            .map_err(|e| crate::Error::Io(e))?;

        let log_output = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = log_output.lines().collect();

        let mut patterns = Vec::new();
        let mut small_commits = 0;
        let mut medium_commits = 0;
        let mut large_commits = 0;
        let mut weekend_commits = 0;
        let mut weekday_commits = 0;

        for line in &lines {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let message = parts[1];
                let date_str = parts[2];

                // Analyze commit size based on message
                if message.len() < 30 {
                    small_commits += 1;
                } else if message.len() < 100 {
                    medium_commits += 1;
                } else {
                    large_commits += 1;
                }

                // Analyze time patterns (simplified)
                if date_str.contains("Sat") || date_str.contains("Sun") {
                    weekend_commits += 1;
                } else {
                    weekday_commits += 1;
                }
            }
        }

        // Create patterns based on analysis
        if small_commits > medium_commits + large_commits {
            patterns.push(CommitPattern {
                pattern_type: "frequent_small_commits".to_string(),
                description: "Many small, frequent commits".to_string(),
                frequency: small_commits,
                files_affected: Vec::new(),
                time_pattern: "regular".to_string(),
                size_pattern: "small".to_string(),
            });
        }

        if weekend_commits > weekday_commits / 3 {
            patterns.push(CommitPattern {
                pattern_type: "weekend_activity".to_string(),
                description: "Significant weekend development activity".to_string(),
                frequency: weekend_commits,
                files_affected: Vec::new(),
                time_pattern: "weekends".to_string(),
                size_pattern: "mixed".to_string(),
            });
        }

        Ok(patterns)
    }

    fn identify_evolution_hotspots(&self, file_evolutions: &[FileEvolution]) -> Vec<EvolutionHotspot> {
        let mut hotspots = Vec::new();

        for evolution in file_evolutions {
            // Calculate risk score based on multiple factors
            let change_frequency = evolution.commit_frequency;
            let author_count = evolution.authors.len();
            let complexity_changes = evolution.lines_added - evolution.lines_removed;

            // Bug proneness heuristic (high change frequency + multiple authors)
            let bug_proneness = if author_count > 3 && change_frequency > 2.0 {
                0.8
            } else if author_count > 2 && change_frequency > 1.0 {
                0.6
            } else if change_frequency > 3.0 {
                0.7
            } else {
                0.3
            };

            // Overall risk score calculation
            let risk_score = (change_frequency / 10.0).min(1.0) * 0.4 +
                           (author_count as f64 / 10.0).min(1.0) * 0.3 +
                           bug_proneness * 0.3;

            if risk_score > 0.5 {  // Threshold for hotspot
                hotspots.push(EvolutionHotspot {
                    file_path: evolution.file_path.clone(),
                    change_frequency,
                    bug_proneness,
                    author_count,
                    complexity_changes,
                    risk_score,
                });
            }
        }

        // Sort by risk score (highest first)
        hotspots.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());

        hotspots
    }

    fn calculate_project_health_metrics(&self, file_evolutions: &[FileEvolution], author_metrics: &[AuthorMetrics]) -> HashMap<String, Value> {
        let mut metrics = HashMap::new();

        // Code churn metrics
        let total_files = file_evolutions.len();
        let active_files = file_evolutions.iter().filter(|f| f.commit_frequency > 0.1).count();
        let stable_files = file_evolutions.iter().filter(|f| f.stability_score > 0.7).count();

        // Author diversity
        let total_authors = author_metrics.len();
        let active_authors = author_metrics.iter().filter(|a| a.total_commits > 5).count();

        // Trend analysis
        let increasing_complexity = file_evolutions.iter().filter(|f| f.complexity_trend == "increasing").count();
        let decreasing_complexity = file_evolutions.iter().filter(|f| f.complexity_trend == "decreasing").count();

        metrics.insert("total_files_tracked".to_string(), json!(total_files));
        metrics.insert("active_files_percentage".to_string(), json!(active_files as f64 / total_files as f64 * 100.0));
        metrics.insert("stable_files_percentage".to_string(), json!(stable_files as f64 / total_files as f64 * 100.0));
        metrics.insert("total_contributors".to_string(), json!(total_authors));
        metrics.insert("active_contributors".to_string(), json!(active_authors));
        metrics.insert("files_increasing_complexity".to_string(), json!(increasing_complexity));
        metrics.insert("files_decreasing_complexity".to_string(), json!(decreasing_complexity));

        // Overall health score
        let health_score = (stable_files as f64 / total_files as f64) * 0.4 +
                          (active_authors as f64 / total_authors as f64) * 0.3 +
                          ((decreasing_complexity as f64) / (increasing_complexity + decreasing_complexity + 1) as f64) * 0.3;

        metrics.insert("overall_health_score".to_string(), json!(health_score));

        metrics
    }
}

impl BaseDataExtractor for EvolutionExtractor {
    fn extract_data(&self) -> crate::Result<HashMap<String, Value>> {
        let mut result = HashMap::new();

        if !self.is_git_repo {
            result.insert("evolution_analysis".to_string(), json!({
                "error": "Not a git repository - evolution analysis requires git history"
            }));
            result.insert("files_analyzed".to_string(), json!(0));
            return Ok(result);
        }

        // Analyze git history
        let (file_evolutions, author_metrics, commit_patterns) = self.analyze_git_history()?;

        // Identify hotspots
        let hotspots = self.identify_evolution_hotspots(&file_evolutions);

        // Calculate project health metrics
        let health_metrics = self.calculate_project_health_metrics(&file_evolutions, &author_metrics);

        // Generate statistics
        let total_files = file_evolutions.len();
        let total_commits: usize = file_evolutions.iter().map(|f| f.total_commits).sum();
        let total_authors = author_metrics.len();
        let hotspot_count = hotspots.len();

        result.insert("extraction_timestamp".to_string(), json!(self.extraction_timestamp.to_rfc3339()));
        result.insert("files_analyzed".to_string(), json!(total_files));
        result.insert("total_commits".to_string(), json!(total_commits));
        result.insert("total_authors".to_string(), json!(total_authors));
        result.insert("hotspot_count".to_string(), json!(hotspot_count));
        result.insert("file_evolutions".to_string(), json!(file_evolutions));
        result.insert("author_metrics".to_string(), json!(author_metrics));
        result.insert("commit_patterns".to_string(), json!(commit_patterns));
        result.insert("evolution_hotspots".to_string(), json!(hotspots));
        result.insert("project_health_metrics".to_string(), json!(health_metrics));

        // Add recommendations
        let mut recommendations = Vec::new();
        if hotspot_count > 0 {
            recommendations.push(format!("Found {} evolution hotspots that may need attention", hotspot_count));
        }
        if total_authors < 3 {
            recommendations.push("Low contributor diversity - consider code review processes".to_string());
        }
        if let Some(health_score) = health_metrics.get("overall_health_score") {
            if health_score.as_f64().unwrap_or(0.0) < 0.5 {
                recommendations.push("Low project health score - consider technical debt management".to_string());
            }
        }

        result.insert("recommendations".to_string(), json!(recommendations));

        println!("Evolution analysis complete: {} files analyzed, {} commits, {} authors, {} hotspots found",
                 total_files, total_commits, total_authors, hotspot_count);

        Ok(result)
    }

    fn extractor_type(&self) -> &'static str {
        "EvolutionExtractor"
    }

    fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    fn extraction_timestamp(&self) -> DateTime<Utc> {
        self.extraction_timestamp
    }
}