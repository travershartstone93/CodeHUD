//! Rollback System with Git Integration
//!
//! This module provides comprehensive rollback functionality with Git backup
//! integration, matching Python's behavior exactly for zero degradation.

use crate::{Result, TransformError};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;

/// Rollback system managing backups and restoration
#[derive(Debug)]
pub struct RollbackSystem {
    /// Configuration for rollback system
    config: RollbackConfig,
    /// Active backups by ID
    backups: HashMap<String, BackupEntry>,
    /// Backup storage directory
    backup_directory: PathBuf,
}

/// Configuration for rollback system
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Maximum number of backups to keep
    pub max_backups: usize,
    /// Whether to use Git for backups
    pub use_git: bool,
    /// Backup directory path
    pub backup_directory: Option<PathBuf>,
    /// Whether to compress backups
    pub compress_backups: bool,
    /// Retention policy in days
    pub retention_days: u32,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            max_backups: 100,
            use_git: true,
            backup_directory: None,
            compress_backups: true,
            retention_days: 30,
        }
    }
}

/// Entry for a single backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Unique backup identifier
    pub backup_id: String,
    /// Files included in backup
    pub files: Vec<PathBuf>,
    /// Timestamp when backup was created
    pub timestamp: DateTime<Utc>,
    /// Git commit hash if using Git
    pub git_commit: Option<String>,
    /// Backup directory path
    pub backup_path: PathBuf,
    /// Description of what was backed up
    pub description: String,
    /// Size of backup in bytes
    pub size_bytes: u64,
}

/// Git backup integration system
#[derive(Debug)]
pub struct GitBackupIntegration {
    /// Git repository path
    repo_path: PathBuf,
    /// Whether Git is available
    git_available: bool,
    /// Configuration
    config: GitBackupConfig,
}

/// Configuration for Git backup integration
#[derive(Debug, Clone)]
pub struct GitBackupConfig {
    /// Whether to auto-commit backups
    pub auto_commit: bool,
    /// Branch prefix for backup branches
    pub backup_branch_prefix: String,
    /// Whether to create tags for backups
    pub create_tags: bool,
    /// Tag prefix for backup tags
    pub tag_prefix: String,
}

impl Default for GitBackupConfig {
    fn default() -> Self {
        Self {
            auto_commit: true,
            backup_branch_prefix: "codehud-backup".to_string(),
            create_tags: true,
            tag_prefix: "codehud-transform".to_string(),
        }
    }
}

impl RollbackSystem {
    /// Create new rollback system
    pub fn new(config: &crate::engine::EngineConfig) -> Result<Self> {
        let rollback_config = RollbackConfig {
            max_backups: 100,
            use_git: config.enable_git_backup,
            backup_directory: config.backup_directory.clone(),
            compress_backups: true,
            retention_days: 30,
        };

        let backup_directory = rollback_config.backup_directory
            .clone()
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(".codehud_backups")
            });

        // Create backup directory if it doesn't exist
        if !backup_directory.exists() {
            fs::create_dir_all(&backup_directory)?;
        }

        Ok(Self {
            config: rollback_config,
            backups: HashMap::new(),
            backup_directory,
        })
    }

    /// Create a backup of the specified file
    pub fn create_backup(&mut self, file_path: &str) -> Result<crate::types::BackupInfo> {
        let backup_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        
        let source_path = Path::new(file_path);
        if !source_path.exists() {
            return Err(TransformError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path),
            )));
        }

        // Create backup subdirectory
        let backup_subdir = self.backup_directory.join(&backup_id);
        fs::create_dir_all(&backup_subdir)?;

        // Copy file to backup location
        let backup_file_path = backup_subdir.join(
            source_path.file_name()
                .ok_or_else(|| TransformError::Config("Invalid file path".to_string()))?
        );
        
        fs::copy(source_path, &backup_file_path)?;

        // Calculate file size
        let size_bytes = fs::metadata(&backup_file_path)?.len();

        // Create backup entry
        let backup_entry = BackupEntry {
            backup_id: backup_id.clone(),
            files: vec![source_path.to_path_buf()],
            timestamp,
            git_commit: None,
            backup_path: backup_subdir.clone(),
            description: format!("Backup of {}", file_path),
            size_bytes,
        };

        self.backups.insert(backup_id.clone(), backup_entry);

        // Create backup info for return
        let backup_info = crate::types::BackupInfo {
            backup_id: backup_id.clone(),
            git_commit: None,
            backup_path: backup_subdir.to_string_lossy().to_string(),
            timestamp,
            files: vec![file_path.to_string()],
        };

        // Clean old backups if needed
        self.cleanup_old_backups()?;

        Ok(backup_info)
    }

    /// Restore from backup
    pub fn restore_from_backup(&self, backup_info: &crate::types::BackupInfo) -> Result<()> {
        let backup_entry = self.backups.get(&backup_info.backup_id)
            .ok_or_else(|| TransformError::Rollback(
                format!("Backup not found: {}", backup_info.backup_id)
            ))?;

        // Restore each file
        for original_file in &backup_entry.files {
            let backup_file = backup_entry.backup_path.join(
                original_file.file_name()
                    .ok_or_else(|| TransformError::Config("Invalid file path".to_string()))?
            );

            if !backup_file.exists() {
                return Err(TransformError::Rollback(
                    format!("Backup file not found: {:?}", backup_file)
                ));
            }

            // Create parent directories if needed
            if let Some(parent) = original_file.parent() {
                fs::create_dir_all(parent)?;
            }

            // Restore file
            fs::copy(&backup_file, original_file)?;
        }

        Ok(())
    }

    /// List all available backups
    pub fn list_backups(&self) -> Vec<&BackupEntry> {
        let mut backups: Vec<&BackupEntry> = self.backups.values().collect();
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        backups
    }

    /// Remove a specific backup
    pub fn remove_backup(&mut self, backup_id: &str) -> Result<()> {
        if let Some(backup_entry) = self.backups.remove(backup_id) {
            // Remove backup directory
            if backup_entry.backup_path.exists() {
                fs::remove_dir_all(&backup_entry.backup_path)?;
            }
        }
        Ok(())
    }

    /// Clean up old backups based on retention policy
    fn cleanup_old_backups(&mut self) -> Result<()> {
        let now = Utc::now();
        let retention_threshold = now - chrono::Duration::days(self.config.retention_days as i64);

        // Find backups to remove
        let mut to_remove = Vec::new();
        for (backup_id, backup_entry) in &self.backups {
            if backup_entry.timestamp < retention_threshold {
                to_remove.push(backup_id.clone());
            }
        }

        // Also enforce max backup limit
        if self.backups.len() > self.config.max_backups {
            let mut sorted_backups: Vec<_> = self.backups.iter().collect();
            sorted_backups.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp));
            
            let excess_count = self.backups.len() - self.config.max_backups;
            for (backup_id, _) in sorted_backups.into_iter().take(excess_count) {
                to_remove.push(backup_id.clone());
            }
        }

        // Remove old backups
        for backup_id in to_remove {
            self.remove_backup(&backup_id)?;
        }

        Ok(())
    }

    /// Get backup statistics
    pub fn get_statistics(&self) -> BackupStatistics {
        let total_size: u64 = self.backups.values()
            .map(|b| b.size_bytes)
            .sum();

        let oldest_backup = self.backups.values()
            .min_by_key(|b| b.timestamp)
            .map(|b| b.timestamp);

        let newest_backup = self.backups.values()
            .max_by_key(|b| b.timestamp)
            .map(|b| b.timestamp);

        BackupStatistics {
            total_backups: self.backups.len(),
            total_size_bytes: total_size,
            oldest_backup,
            newest_backup,
        }
    }
}

impl GitBackupIntegration {
    /// Create new Git backup integration
    pub fn new(config: &crate::engine::EngineConfig) -> Result<Self> {
        let repo_path = std::env::current_dir()?;
        let git_available = Self::check_git_availability(&repo_path)?;

        Ok(Self {
            repo_path,
            git_available,
            config: GitBackupConfig::default(),
        })
    }

    /// Create a session backup using Git
    pub fn create_session_backup(&self, session_id: &str) -> Result<String> {
        if !self.git_available {
            return Err(TransformError::Config("Git not available".to_string()));
        }

        // Create commit for current state
        let commit_message = format!("CodeHUD transformation session backup: {}", session_id);
        
        // Add all changes
        let add_output = Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&self.repo_path)
            .output()?;

        if !add_output.status.success() {
            return Err(TransformError::Git(
                format!("Git add failed: {}", String::from_utf8_lossy(&add_output.stderr))
            ));
        }

        // Create commit
        let commit_output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(&commit_message)
            .arg("--allow-empty")
            .current_dir(&self.repo_path)
            .output()?;

        if !commit_output.status.success() {
            return Err(TransformError::Git(
                format!("Git commit failed: {}", String::from_utf8_lossy(&commit_output.stderr))
            ));
        }

        // Get commit hash
        let hash_output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()?;

        if !hash_output.status.success() {
            return Err(TransformError::Git(
                "Failed to get commit hash".to_string()
            ));
        }

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

        // Create tag if enabled
        if self.config.create_tags {
            let tag_name = format!("{}-{}", self.config.tag_prefix, session_id);
            let tag_output = Command::new("git")
                .arg("tag")
                .arg(&tag_name)
                .current_dir(&self.repo_path)
                .output()?;

            if !tag_output.status.success() {
                // Tag creation failure is not critical
                eprintln!("Warning: Failed to create Git tag: {}", tag_name);
            }
        }

        Ok(commit_hash)
    }

    /// Restore to a specific Git commit
    pub fn restore_to_commit(&self, commit_hash: &str) -> Result<()> {
        if !self.git_available {
            return Err(TransformError::Config("Git not available".to_string()));
        }

        // Reset to specified commit
        let reset_output = Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg(commit_hash)
            .current_dir(&self.repo_path)
            .output()?;

        if !reset_output.status.success() {
            return Err(TransformError::Git(
                format!("Git reset failed: {}", String::from_utf8_lossy(&reset_output.stderr))
            ));
        }

        Ok(())
    }

    /// Check if Git is available and repo is initialized
    fn check_git_availability(repo_path: &Path) -> Result<bool> {
        // Check if git command is available
        let git_version = Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if git_version.is_err() || !git_version.unwrap().success() {
            return Ok(false);
        }

        // Check if current directory is a git repository
        let git_status = Command::new("git")
            .arg("status")
            .current_dir(repo_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        Ok(git_status.is_ok() && git_status.unwrap().success())
    }

    /// Get Git repository status
    pub fn get_repo_status(&self) -> Result<GitRepoStatus> {
        if !self.git_available {
            return Ok(GitRepoStatus {
                is_clean: false,
                current_branch: None,
                uncommitted_changes: 0,
                untracked_files: 0,
            });
        }

        // Get current branch
        let branch_output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.repo_path)
            .output()?;

        let current_branch = if branch_output.status.success() {
            Some(String::from_utf8_lossy(&branch_output.stdout).trim().to_string())
        } else {
            None
        };

        // Get status
        let status_output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.repo_path)
            .output()?;

        let mut uncommitted_changes = 0;
        let mut untracked_files = 0;

        if status_output.status.success() {
            let status_text = String::from_utf8_lossy(&status_output.stdout);
            for line in status_text.lines() {
                if line.starts_with("??") {
                    untracked_files += 1;
                } else if !line.trim().is_empty() {
                    uncommitted_changes += 1;
                }
            }
        }

        Ok(GitRepoStatus {
            is_clean: uncommitted_changes == 0 && untracked_files == 0,
            current_branch,
            uncommitted_changes,
            untracked_files,
        })
    }
}

/// Backup system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatistics {
    /// Total number of backups
    pub total_backups: usize,
    /// Total size of all backups in bytes
    pub total_size_bytes: u64,
    /// Timestamp of oldest backup
    pub oldest_backup: Option<DateTime<Utc>>,
    /// Timestamp of newest backup
    pub newest_backup: Option<DateTime<Utc>>,
}

/// Git repository status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepoStatus {
    /// Whether the repository is clean (no uncommitted changes)
    pub is_clean: bool,
    /// Current branch name
    pub current_branch: Option<String>,
    /// Number of uncommitted changes
    pub uncommitted_changes: usize,
    /// Number of untracked files
    pub untracked_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rollback_system_creation() {
        let config = crate::engine::EngineConfig::default();
        let rollback_system = RollbackSystem::new(&config);
        assert!(rollback_system.is_ok());
    }

    #[test]
    fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let config = crate::engine::EngineConfig {
            backup_directory: Some(temp_dir.path().join("backups")),
            ..Default::default()
        };

        let mut rollback_system = RollbackSystem::new(&config).unwrap();
        let backup_info = rollback_system.create_backup(test_file.to_str().unwrap());
        assert!(backup_info.is_ok());
    }

    #[test]
    fn test_git_availability_check() {
        let temp_dir = TempDir::new().unwrap();
        let is_available = GitBackupIntegration::check_git_availability(temp_dir.path());
        // This will return false since temp directory is not a git repo
        assert!(is_available.is_ok());
    }
}