//! Git Version Control Integration
//!
//! Zero-degradation integration with Git for version control analysis

use super::ExternalTool;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use anyhow::{Result, Context};
use tracing::{debug, warn};

pub struct GitIntegration {
    codebase_path: PathBuf,
}

impl GitIntegration {
    pub fn new(codebase_path: &Path) -> Self {
        Self {
            codebase_path: codebase_path.to_path_buf(),
        }
    }

    pub fn is_git_repository(&self) -> bool {
        self.codebase_path.join(".git").exists()
    }
}

#[async_trait::async_trait]
impl ExternalTool for GitIntegration {
    type Result = GitResult;

    async fn is_available(&self) -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn analyze(&self) -> Result<Self::Result> {
        debug!("Running git analysis on {}", self.codebase_path.display());

        if !self.is_git_repository() {
            return Ok(GitResult {
                is_git_repo: false,
                ..Default::default()
            });
        }

        // Get repository statistics
        let stats = self.get_repository_stats().await?;

        // Get commit history
        let recent_commits = self.get_recent_commits(20).await?;

        // Get file change statistics
        let file_changes = self.get_file_change_stats().await?;

        // Get author statistics
        let author_stats = self.get_author_stats().await?;

        // Get branch information
        let branch_info = self.get_branch_info().await?;

        Ok(GitResult {
            is_git_repo: true,
            repository_stats: stats,
            recent_commits,
            file_changes,
            author_stats,
            branch_info,
        })
    }

    fn tool_name(&self) -> &'static str {
        "git"
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("--version")
            .output()
            .await
            .context("Failed to get git version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get git version"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

impl GitIntegration {
    async fn get_repository_stats(&self) -> Result<RepositoryStats> {
        // Get total commits
        let commit_count_output = Command::new("git")
            .arg("rev-list")
            .arg("--count")
            .arg("HEAD")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        let total_commits = if commit_count_output.status.success() {
            String::from_utf8_lossy(&commit_count_output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        // Get current branch
        let branch_output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        let current_branch = if branch_output.status.success() {
            String::from_utf8_lossy(&branch_output.stdout).trim().to_string()
        } else {
            "unknown".to_string()
        };

        // Get repository status
        let status_output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        let has_uncommitted_changes = if status_output.status.success() {
            !String::from_utf8_lossy(&status_output.stdout).trim().is_empty()
        } else {
            false
        };

        // Get last commit info
        let last_commit_output = Command::new("git")
            .arg("log")
            .arg("-1")
            .arg("--pretty=format:%H|%an|%ad|%s")
            .arg("--date=iso")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        let last_commit = if last_commit_output.status.success() {
            let output = String::from_utf8_lossy(&last_commit_output.stdout);
            self.parse_commit_line(&output).unwrap_or_else(|| CommitInfo {
                hash: "unknown".to_string(),
                author: "unknown".to_string(),
                date: "unknown".to_string(),
                message: "unknown".to_string(),
            })
        } else {
            CommitInfo {
                hash: "unknown".to_string(),
                author: "unknown".to_string(),
                date: "unknown".to_string(),
                message: "unknown".to_string(),
            }
        };

        Ok(RepositoryStats {
            total_commits,
            current_branch,
            has_uncommitted_changes,
            last_commit,
        })
    }

    async fn get_recent_commits(&self, limit: usize) -> Result<Vec<CommitInfo>> {
        let output = Command::new("git")
            .arg("log")
            .arg(format!("-{}", limit))
            .arg("--pretty=format:%H|%an|%ad|%s")
            .arg("--date=iso")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = stdout
            .lines()
            .filter_map(|line| self.parse_commit_line(line))
            .collect();

        Ok(commits)
    }

    async fn get_file_change_stats(&self) -> Result<Vec<FileChangeStats>> {
        let output = Command::new("git")
            .arg("log")
            .arg("--name-only")
            .arg("--pretty=format:")
            .arg("--since=30.days.ago")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut file_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() {
                *file_counts.entry(line.to_string()).or_insert(0) += 1;
            }
        }

        let mut file_stats: Vec<FileChangeStats> = file_counts
            .into_iter()
            .map(|(file_path, change_count)| FileChangeStats {
                file_path,
                change_count,
            })
            .collect();

        // Sort by change count (most changed first)
        file_stats.sort_by(|a, b| b.change_count.cmp(&a.change_count));

        // Limit to top 50 most changed files
        file_stats.truncate(50);

        Ok(file_stats)
    }

    async fn get_author_stats(&self) -> Result<Vec<AuthorStats>> {
        let output = Command::new("git")
            .arg("shortlog")
            .arg("-sn")
            .arg("--since=1.year.ago")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let author_stats = stdout
            .lines()
            .filter_map(|line| self.parse_author_line(line))
            .collect();

        Ok(author_stats)
    }

    async fn get_branch_info(&self) -> Result<BranchInfo> {
        // Get all branches
        let branches_output = Command::new("git")
            .arg("branch")
            .arg("-a")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        let branch_count = if branches_output.status.success() {
            String::from_utf8_lossy(&branches_output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        } else {
            0
        };

        // Get remote info
        let remote_output = Command::new("git")
            .arg("remote")
            .arg("-v")
            .current_dir(&self.codebase_path)
            .output()
            .await?;

        let remotes = if remote_output.status.success() {
            String::from_utf8_lossy(&remote_output.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        Some(format!("{} {}", parts[0], parts[1]))
                    } else {
                        None
                    }
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };

        Ok(BranchInfo {
            total_branches: branch_count,
            remotes,
        })
    }

    fn parse_commit_line(&self, line: &str) -> Option<CommitInfo> {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() >= 4 {
            Some(CommitInfo {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3].to_string(),
            })
        } else {
            None
        }
    }

    fn parse_author_line(&self, line: &str) -> Option<AuthorStats> {
        let line = line.trim();
        if let Some(first_tab) = line.find('\t') {
            let count_str = &line[..first_tab];
            let author = &line[first_tab + 1..];

            if let Ok(commit_count) = count_str.trim().parse::<usize>() {
                return Some(AuthorStats {
                    author: author.to_string(),
                    commit_count,
                });
            }
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitResult {
    pub is_git_repo: bool,
    pub repository_stats: RepositoryStats,
    pub recent_commits: Vec<CommitInfo>,
    pub file_changes: Vec<FileChangeStats>,
    pub author_stats: Vec<AuthorStats>,
    pub branch_info: BranchInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryStats {
    pub total_commits: usize,
    pub current_branch: String,
    pub has_uncommitted_changes: bool,
    pub last_commit: CommitInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileChangeStats {
    pub file_path: String,
    pub change_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorStats {
    pub author: String,
    pub commit_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BranchInfo {
    pub total_branches: usize,
    pub remotes: Vec<String>,
}

impl Default for GitResult {
    fn default() -> Self {
        Self {
            is_git_repo: false,
            repository_stats: RepositoryStats {
                total_commits: 0,
                current_branch: "unknown".to_string(),
                has_uncommitted_changes: false,
                last_commit: CommitInfo {
                    hash: "unknown".to_string(),
                    author: "unknown".to_string(),
                    date: "unknown".to_string(),
                    message: "unknown".to_string(),
                },
            },
            recent_commits: Vec::new(),
            file_changes: Vec::new(),
            author_stats: Vec::new(),
            branch_info: BranchInfo {
                total_branches: 0,
                remotes: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_git_integration() {
        let git_integration = GitIntegration::new(Path::new("/tmp"));

        assert_eq!(git_integration.tool_name(), "git");

        // Test availability (git should be available on most systems)
        let is_available = git_integration.is_available().await;
        println!("Git available: {}", is_available);
    }

    #[tokio::test]
    async fn test_git_version() {
        let git_integration = GitIntegration::new(Path::new("/tmp"));

        if git_integration.is_available().await {
            let version = git_integration.get_version().await.unwrap();
            println!("Git version: {}", version);
            assert!(!version.is_empty());
            assert!(version.contains("git"));
        }
    }

    #[test]
    fn test_commit_line_parsing() {
        let git_integration = GitIntegration::new(Path::new("/tmp"));

        let test_line = "abc123|John Doe|2023-11-20 10:30:00 +0000|Add new feature";
        let commit = git_integration.parse_commit_line(test_line).unwrap();

        assert_eq!(commit.hash, "abc123");
        assert_eq!(commit.author, "John Doe");
        assert_eq!(commit.date, "2023-11-20 10:30:00 +0000");
        assert_eq!(commit.message, "Add new feature");
    }

    #[test]
    fn test_author_line_parsing() {
        let git_integration = GitIntegration::new(Path::new("/tmp"));

        let test_line = "   25\tJohn Doe";
        let author = git_integration.parse_author_line(test_line).unwrap();

        assert_eq!(author.author, "John Doe");
        assert_eq!(author.commit_count, 25);
    }
}