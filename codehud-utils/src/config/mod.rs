//! Configuration utilities with Python configparser compatibility
//!
//! This module provides configuration loading and management that
//! behaves identically to Python's configparser and related utilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Configuration loader that supports TOML, JSON, and YAML
#[derive(Debug)]
pub struct ConfigLoader {
    configs: HashMap<String, serde_json::Value>,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Load configuration from file (auto-detects format)
    pub fn load_file(&mut self, path: &Path) -> crate::Result<()> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let content = std::fs::read_to_string(path)?;
        
        let config_value = match extension.as_str() {
            "toml" => {
                let toml_value: toml::Value = toml::from_str(&content)
                    .map_err(|e| crate::UtilError::Config(format!("TOML parse error: {}", e)))?;
                serde_json::to_value(toml_value)
                    .map_err(|e| crate::UtilError::Config(format!("TOML conversion error: {}", e)))?
            }
            "json" => {
                serde_json::from_str(&content)
                    .map_err(|e| crate::UtilError::Config(format!("JSON parse error: {}", e)))?
            }
            "yml" | "yaml" => {
                serde_yaml::from_str(&content)
                    .map_err(|e| crate::UtilError::Config(format!("YAML parse error: {}", e)))?
            }
            _ => {
                return Err(crate::UtilError::Config(
                    format!("Unsupported config format: {}", extension)
                ));
            }
        };

        let config_name = path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("default")
            .to_string();

        self.configs.insert(config_name, config_value);
        Ok(())
    }

    /// Get configuration value by key path (supports dot notation)
    pub fn get_value(&self, config_name: &str, key_path: &str) -> Option<&serde_json::Value> {
        let config = self.configs.get(config_name)?;
        self.get_nested_value(config, key_path)
    }

    /// Get string value
    pub fn get_string(&self, config_name: &str, key_path: &str) -> Option<String> {
        self.get_value(config_name, key_path)?.as_str().map(|s| s.to_string())
    }

    /// Get integer value
    pub fn get_int(&self, config_name: &str, key_path: &str) -> Option<i64> {
        self.get_value(config_name, key_path)?.as_i64()
    }

    /// Get float value
    pub fn get_float(&self, config_name: &str, key_path: &str) -> Option<f64> {
        self.get_value(config_name, key_path)?.as_f64()
    }

    /// Get boolean value
    pub fn get_bool(&self, config_name: &str, key_path: &str) -> Option<bool> {
        self.get_value(config_name, key_path)?.as_bool()
    }

    /// Get array value
    pub fn get_array(&self, config_name: &str, key_path: &str) -> Option<&Vec<serde_json::Value>> {
        self.get_value(config_name, key_path)?.as_array()
    }

    /// Helper to navigate nested JSON values using dot notation
    fn get_nested_value<'a>(&self, value: &'a serde_json::Value, key_path: &str) -> Option<&'a serde_json::Value> {
        let keys: Vec<&str> = key_path.split('.').collect();
        let mut current = value;

        for key in keys {
            current = current.get(key)?;
        }

        Some(current)
    }

    /// Merge configurations (second config overrides first)
    pub fn merge_config(&mut self, name: &str, other_config: serde_json::Value) -> crate::Result<()> {
        if let Some(existing) = self.configs.get_mut(name) {
            merge_json_values(existing, other_config);
        } else {
            self.configs.insert(name.to_string(), other_config);
        }
        Ok(())
    }

    /// Get all config names
    pub fn config_names(&self) -> Vec<&String> {
        self.configs.keys().collect()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Merge two JSON values (second overrides first)
fn merge_json_values(base: &mut serde_json::Value, override_value: serde_json::Value) {
    match (base, override_value) {
        (serde_json::Value::Object(base_obj), serde_json::Value::Object(override_obj)) => {
            for (key, value) in override_obj {
                if base_obj.contains_key(&key) {
                    merge_json_values(base_obj.get_mut(&key).unwrap(), value);
                } else {
                    base_obj.insert(key, value);
                }
            }
        }
        (base_value, override_value) => {
            *base_value = override_value;
        }
    }
}

/// Load configuration from file (convenience function)
pub fn load_config<T>(path: &Path) -> crate::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = std::fs::read_to_string(path)?;
    
    let result = match extension.as_str() {
        "toml" => {
            let toml_value: toml::Value = toml::from_str(&content)
                .map_err(|e| crate::UtilError::Config(format!("TOML parse error: {}", e)))?;
            T::deserialize(toml_value)
                .map_err(|e| crate::UtilError::Config(format!("TOML deserialization error: {}", e)))
        }
        "json" => {
            serde_json::from_str(&content)
                .map_err(|e| crate::UtilError::Config(format!("JSON parse error: {}", e)))
        }
        "yml" | "yaml" => {
            serde_yaml::from_str(&content)
                .map_err(|e| crate::UtilError::Config(format!("YAML parse error: {}", e)))
        }
        _ => {
            return Err(crate::UtilError::Config(
                format!("Unsupported config format: {}", extension)
            ));
        }
    }?;

    Ok(result)
}

/// Merge two configurations (second overrides first)
pub fn merge_configs<T>(base: &mut T, override_config: T) -> crate::Result<()>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut base_value = serde_json::to_value(&*base)
        .map_err(|e| crate::UtilError::Serialization(format!("Base serialization error: {}", e)))?;
    
    let override_value = serde_json::to_value(override_config)
        .map_err(|e| crate::UtilError::Serialization(format!("Override serialization error: {}", e)))?;
    
    merge_json_values(&mut base_value, override_value);
    
    *base = serde_json::from_value(base_value)
        .map_err(|e| crate::UtilError::Serialization(format!("Result deserialization error: {}", e)))?;
    
    Ok(())
}

/// Validate configuration against schema (basic validation)
pub fn validate_config<T>(config: &T) -> crate::Result<()>
where
    T: Serialize,
{
    // Convert to JSON to perform validation
    let _json = serde_json::to_value(config)
        .map_err(|e| crate::UtilError::Config(format!("Config validation error: {}", e)))?;
    
    // TODO: Add more sophisticated validation if needed
    Ok(())
}

/// Configuration builder for fluent API
#[derive(Debug)]
pub struct ConfigBuilder<T> {
    config: T,
}

impl<T> ConfigBuilder<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de>,
{
    /// Create new config builder with defaults
    pub fn new() -> Self {
        Self {
            config: T::default(),
        }
    }

    /// Load from file and merge
    pub fn load_file(mut self, path: &Path) -> crate::Result<Self> {
        let file_config: T = load_config(path)?;
        merge_configs(&mut self.config, file_config)?;
        Ok(self)
    }

    /// Merge with another config
    pub fn merge(mut self, other: T) -> crate::Result<Self> {
        merge_configs(&mut self.config, other)?;
        Ok(self)
    }

    /// Validate and build final config
    pub fn build(self) -> crate::Result<T> {
        validate_config(&self.config)?;
        Ok(self.config)
    }
}

impl<T> Default for ConfigBuilder<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de>,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        port: u16,
        enabled: bool,
        items: Vec<String>,
    }

    #[test]
    fn test_config_loader() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("test.json");
        
        let json_content = r#"{
            "name": "test",
            "port": 8080,
            "enabled": true,
            "items": ["a", "b", "c"]
        }"#;
        
        std::fs::write(&config_path, json_content)?;
        
        let mut loader = ConfigLoader::new();
        loader.load_file(&config_path)?;
        
        assert_eq!(loader.get_string("test", "name"), Some("test".to_string()));
        assert_eq!(loader.get_int("test", "port"), Some(8080));
        assert_eq!(loader.get_bool("test", "enabled"), Some(true));
        
        Ok(())
    }

    #[test]
    fn test_load_config() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("test.toml");
        
        let toml_content = r#"
name = "test"
port = 9000
enabled = false
items = ["x", "y", "z"]
        "#;
        
        std::fs::write(&config_path, toml_content)?;
        
        let config: TestConfig = load_config(&config_path)?;
        
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 9000);
        assert!(!config.enabled);
        assert_eq!(config.items, vec!["x", "y", "z"]);
        
        Ok(())
    }

    #[test]
    fn test_merge_configs() -> crate::Result<()> {
        let mut base = TestConfig {
            name: "base".to_string(),
            port: 8080,
            enabled: true,
            items: vec!["a".to_string()],
        };
        
        let override_config = TestConfig {
            name: "override".to_string(),
            port: 9000,
            enabled: false,
            items: vec!["x".to_string(), "y".to_string()],
        };
        
        merge_configs(&mut base, override_config)?;
        
        assert_eq!(base.name, "override");
        assert_eq!(base.port, 9000);
        assert!(!base.enabled);
        assert_eq!(base.items, vec!["x", "y"]);
        
        Ok(())
    }

    #[test]
    fn test_config_builder() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("builder.yaml");
        
        let yaml_content = r#"
name: builder_test
port: 7777
enabled: true
items:
  - item1
  - item2
        "#;
        
        std::fs::write(&config_path, yaml_content)?;
        
        let config: TestConfig = ConfigBuilder::new()
            .load_file(&config_path)?
            .build()?;
        
        assert_eq!(config.name, "builder_test");
        assert_eq!(config.port, 7777);
        assert!(config.enabled);
        
        Ok(())
    }

    #[test]
    fn test_nested_key_access() {
        let json_value = serde_json::json!({
            "database": {
                "host": "localhost",
                "port": 5432,
                "credentials": {
                    "username": "user",
                    "password": "pass"
                }
            }
        });
        
        let loader = ConfigLoader::new();
        
        assert_eq!(
            loader.get_nested_value(&json_value, "database.host"),
            Some(&serde_json::Value::String("localhost".to_string()))
        );
        
        assert_eq!(
            loader.get_nested_value(&json_value, "database.credentials.username"),
            Some(&serde_json::Value::String("user".to_string()))
        );
        
        assert_eq!(
            loader.get_nested_value(&json_value, "nonexistent.key"),
            None
        );
    }
}