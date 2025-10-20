//! File system utilities with Python pathlib compatibility
//!
//! This module provides file system operations that behave identically
//! to Python's pathlib and related utilities.

#[allow(unused_imports)]
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

/// Safely join paths, preventing directory traversal attacks (matches Python os.path.join behavior)
pub fn safe_path_join(base: &Path, relative: &Path) -> crate::Result<PathBuf> {
    // Normalize the relative path to prevent .. traversal
    let normalized = normalize_path(relative);
    
    // Check for absolute path or .. traversal attempts
    if normalized.is_absolute() || normalized.to_string_lossy().contains("..") {
        return Err(crate::UtilError::PathOperation(
            format!("Unsafe path join attempted: {:?} + {:?}", base, relative)
        ));
    }
    
    Ok(base.join(normalized))
}

/// Normalize path (equivalent to Python pathlib.Path.resolve())
pub fn normalize_path(path: &Path) -> PathBuf {
    path_clean::clean(path)
}

/// Find project root by looking for common project markers (matches Python behavior)
pub fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    let markers = [
        "pyproject.toml",
        "Cargo.toml", 
        "package.json",
        "setup.py",
        "requirements.txt",
        ".git",
        ".gitignore",
        "Makefile",
        "README.md",
        "README.txt",
    ];
    
    let mut current = start_dir;
    
    loop {
        // Check if any project markers exist in current directory
        for marker in &markers {
            if current.join(marker).exists() {
                return Some(current.to_path_buf());
            }
        }
        
        // Move up to parent directory
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    
    None
}

/// Create a timestamped backup of a file (matches Python behavior)
pub fn create_backup(file_path: &Path) -> crate::Result<PathBuf> {
    if !file_path.exists() {
        return Err(crate::UtilError::Io(
            std::io::Error::new(std::io::ErrorKind::NotFound, "File does not exist")
        ));
    }
    
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!(
        "{}.backup.{}",
        file_path.file_name()
            .ok_or_else(|| crate::UtilError::PathOperation("Invalid file name".to_string()))?
            .to_string_lossy(),
        timestamp
    );
    
    let backup_dir = file_path.parent()
        .ok_or_else(|| crate::UtilError::PathOperation("Cannot determine parent directory".to_string()))?
        .join(".codehud_backups");
        
    fs::create_dir_all(&backup_dir)?;
    
    let backup_path = backup_dir.join(backup_name);
    fs::copy(file_path, &backup_path)?;
    
    Ok(backup_path)
}

/// Copy file with automatic backup creation
pub fn copy_with_backup(src: &Path, dst: &Path) -> crate::Result<()> {
    // Create backup of destination if it exists
    if dst.exists() {
        create_backup(dst)?;
    }
    
    // Ensure destination directory exists
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::copy(src, dst)?;
    Ok(())
}

/// Get file metadata in a Python-compatible format
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub created: Option<DateTime<Utc>>,
    pub is_file: bool,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub stem: Option<String>,
}

impl FileMetadata {
    /// Create FileMetadata from a path
    pub fn from_path(path: &Path) -> crate::Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified = DateTime::from(metadata.modified()?);
        
        // Creation time is not reliable on all platforms (like Python's behavior)
        let created = metadata.created()
            .map(DateTime::from)
            .ok();
            
        Ok(Self {
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            modified,
            created,
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            extension: path.extension().map(|s| s.to_string_lossy().to_string()),
            stem: path.file_stem().map(|s| s.to_string_lossy().to_string()),
        })
    }
}

/// Recursively find files matching patterns (equivalent to Python glob/pathlib)
pub fn find_files(
    root: &Path,
    patterns: Option<&[String]>,
    exclude_patterns: Option<&std::collections::HashSet<String>>,
    max_depth: Option<usize>,
) -> crate::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    find_files_recursive(root, patterns, exclude_patterns, max_depth, 0, &mut files)?;
    Ok(files)
}

/// Recursive helper for find_files
fn find_files_recursive(
    dir: &Path,
    patterns: Option<&[String]>,
    exclude_patterns: Option<&std::collections::HashSet<String>>,
    max_depth: Option<usize>,
    current_depth: usize,
    files: &mut Vec<PathBuf>,
) -> crate::Result<()> {
    // Check depth limit
    if let Some(max) = max_depth {
        if current_depth >= max {
            return Ok(());
        }
    }
    
    // Check if directory should be excluded
    if let Some(excludes) = exclude_patterns {
        if crate::should_exclude_path(dir, excludes) {
            return Ok(());
        }
    }
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            find_files_recursive(&path, patterns, exclude_patterns, max_depth, current_depth + 1, files)?;
        } else if path.is_file() {
            // Check if file should be excluded
            if let Some(excludes) = exclude_patterns {
                if crate::should_exclude_path(&path, excludes) {
                    continue;
                }
            }
            
            // Check if file matches patterns
            if let Some(patterns) = patterns {
                let matches_pattern = patterns.iter().any(|pattern| {
                    if pattern.starts_with("*.") {
                        let ext = &pattern[2..];
                        path.extension().and_then(|e| e.to_str()) == Some(ext)
                    } else {
                        path.to_string_lossy().contains(pattern)
                    }
                });
                
                if matches_pattern {
                    files.push(path);
                }
            } else {
                files.push(path);
            }
        }
    }
    
    Ok(())
}

/// Calculate file content hash (for caching)
pub fn calculate_file_hash(path: &Path) -> crate::Result<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let content = fs::read_to_string(path)?;
    let metadata = fs::metadata(path)?;
    
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    
    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
            duration.as_secs().hash(&mut hasher);
        }
    }
    
    Ok(format!("{:x}", hasher.finish()))
}

/// Read file with encoding detection (matches Python behavior)
pub fn read_text_file(path: &Path) -> crate::Result<String> {
    // Try UTF-8 first (most common)
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(_) => {
            // Fallback to bytes and try to decode as UTF-8 with replacement
            let bytes = fs::read(path)?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
    }
}

/// Ensure directory exists (equivalent to pathlib.Path.mkdir(parents=True, exist_ok=True))
pub fn ensure_dir(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    } else if !path.is_dir() {
        return Err(crate::UtilError::PathOperation(
            format!("Path exists but is not a directory: {:?}", path)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_safe_path_join() {
        let base = Path::new("/safe/base");
        
        // Safe join
        let result = safe_path_join(base, Path::new("subdir/file.txt")).unwrap();
        assert!(result.to_string_lossy().contains("safe/base/subdir/file.txt"));
        
        // Unsafe joins should fail
        assert!(safe_path_join(base, Path::new("../../../etc/passwd")).is_err());
        assert!(safe_path_join(base, Path::new("/absolute/path")).is_err());
    }

    #[test]
    fn test_normalize_path() {
        let path = Path::new("/a/b/../c/./d");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/a/c/d"));
    }

    #[test]
    fn test_find_project_root() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let project_dir = temp_dir.path().join("project");
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&src_dir)?;
        
        // Create a project marker
        fs::write(project_dir.join("pyproject.toml"), "[project]\nname = \"test\"")?;
        
        // Should find project root from subdirectory
        let found = find_project_root(&src_dir);
        assert_eq!(found, Some(project_dir));
        
        Ok(())
    }

    #[test] 
    fn test_file_metadata() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.py");
        fs::write(&file_path, "print('hello')")?;
        
        let metadata = FileMetadata::from_path(&file_path)?;
        assert!(metadata.is_file);
        assert!(!metadata.is_dir);
        assert_eq!(metadata.extension, Some("py".to_string()));
        assert_eq!(metadata.stem, Some("test".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_find_files() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path();
        
        // Create test files
        fs::write(root.join("test.py"), "python code")?;
        fs::write(root.join("test.js"), "javascript code")?;
        fs::write(root.join("README.md"), "documentation")?;
        
        let src_dir = root.join("src");
        fs::create_dir_all(&src_dir)?;
        fs::write(src_dir.join("main.py"), "main python code")?;
        
        // Find Python files
        let python_files = find_files(
            root,
            Some(&["*.py".to_string()]),
            None,
            None,
        )?;
        
        assert_eq!(python_files.len(), 2);
        assert!(python_files.iter().any(|p| p.file_name().unwrap() == "test.py"));
        assert!(python_files.iter().any(|p| p.file_name().unwrap() == "main.py"));
        
        Ok(())
    }

    #[test]
    fn test_calculate_file_hash() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content")?;
        
        let hash1 = calculate_file_hash(&file_path)?;
        let hash2 = calculate_file_hash(&file_path)?;
        
        // Same file should have same hash
        assert_eq!(hash1, hash2);
        
        // Modified file should have different hash
        fs::write(&file_path, "different content")?;
        let hash3 = calculate_file_hash(&file_path)?;
        assert_ne!(hash1, hash3);
        
        Ok(())
    }
}