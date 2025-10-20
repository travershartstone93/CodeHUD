//! Magic Number Transformer
//!
//! Extracts magic numbers to named constants, matching Python behavior exactly

use crate::{
    transformers::Transformer,
    types::{
        TransformationSuggestion, TransformationResult, TransformationType, TransformationStatus,
    },
    libcst::{LibCstTransformer, CstTransform, CstNode},
    Result, TransformError,
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use codehud_core::models::AnalysisResult;

/// Represents the context around a magic number for intelligent extraction
/// Matches Python MagicNumberContext exactly
#[derive(Debug, Clone)]
pub struct MagicNumberContext {
    /// The numeric value found
    pub number: f64,
    /// Line number where found (1-based)
    pub line: usize,
    /// Column number where found (1-based)  
    pub column: usize,
    /// Surrounding code context
    pub context_code: String,
    /// AI-suggested constant name
    pub suggested_constant_name: Option<String>,
    /// Suggested grouping class for organization
    pub suggested_group_class: Option<String>,
    /// Confidence in the suggestion (0.0 to 1.0)
    pub confidence: f64,
}

impl MagicNumberContext {
    /// Create new context for a magic number
    pub fn new(number: f64, line: usize, column: usize, context_code: String) -> Self {
        Self {
            number,
            line,
            column,
            context_code,
            suggested_constant_name: None,
            suggested_group_class: None,
            confidence: 0.0,
        }
    }

    /// Analyze surrounding code to determine appropriate constant name
    /// Matches Python analyze_context method exactly
    pub fn analyze_context(&mut self) {
        let context_lower = self.context_code.to_lowercase();

        // God Class pattern detection
        if context_lower.contains("god") || (context_lower.contains("method") && context_lower.contains("class")) {
            self.analyze_god_class_context();
        }
        // Health score pattern detection  
        else if context_lower.contains("health") && context_lower.contains("score") {
            self.analyze_health_score_context();
        }
        // Complexity pattern detection
        else if context_lower.contains("complexity") {
            self.analyze_complexity_context();
        }
        // HTTP status code detection
        else if context_lower.contains("http") || context_lower.contains("status") {
            self.analyze_http_status_context();
        }
        // Threshold pattern detection
        else if context_lower.contains("threshold") || context_lower.contains("limit") {
            self.analyze_threshold_context();
        }
        // Configuration pattern detection
        else if context_lower.contains("config") || context_lower.contains("setting") {
            self.analyze_config_context();
        }
        // Default pattern analysis
        else {
            self.analyze_generic_context();
        }
    }

    /// Analyze God Class specific patterns
    fn analyze_god_class_context(&mut self) {
        if self.number as i32 == 100 {
            self.suggested_constant_name = Some("GOD_CLASS_METHOD_THRESHOLD".to_string());
            self.suggested_group_class = Some("CodeQualityThresholds".to_string());
            self.confidence = 0.9;
        } else if self.number as i32 == 20 {
            self.suggested_constant_name = Some("MAX_METHODS_PER_CLASS".to_string());
            self.suggested_group_class = Some("ArchitecturalLimits".to_string());
            self.confidence = 0.85;
        }
    }

    /// Analyze health score patterns
    fn analyze_health_score_context(&mut self) {
        if self.number as i32 == 100 {
            self.suggested_constant_name = Some("PERFECT_HEALTH_SCORE".to_string());
            self.suggested_group_class = Some("HealthMetrics".to_string());
            self.confidence = 0.95;
        } else if self.number == 0.0 {
            self.suggested_constant_name = Some("MIN_HEALTH_SCORE".to_string());
            self.suggested_group_class = Some("HealthMetrics".to_string());
            self.confidence = 0.9;
        }
    }

    /// Analyze complexity patterns
    fn analyze_complexity_context(&mut self) {
        if self.number as i32 == 10 {
            self.suggested_constant_name = Some("MAX_CYCLOMATIC_COMPLEXITY".to_string());
            self.suggested_group_class = Some("ComplexityThresholds".to_string());
            self.confidence = 0.9;
        } else if self.number as i32 == 15 {
            self.suggested_constant_name = Some("HIGH_COMPLEXITY_THRESHOLD".to_string());
            self.suggested_group_class = Some("ComplexityThresholds".to_string());
            self.confidence = 0.85;
        }
    }

    /// Analyze HTTP status codes
    fn analyze_http_status_context(&mut self) {
        match self.number as i32 {
            200 => {
                self.suggested_constant_name = Some("HTTP_OK".to_string());
                self.suggested_group_class = Some("HttpStatusCodes".to_string());
                self.confidence = 0.98;
            }
            404 => {
                self.suggested_constant_name = Some("HTTP_NOT_FOUND".to_string());
                self.suggested_group_class = Some("HttpStatusCodes".to_string());
                self.confidence = 0.98;
            }
            500 => {
                self.suggested_constant_name = Some("HTTP_INTERNAL_ERROR".to_string());
                self.suggested_group_class = Some("HttpStatusCodes".to_string());
                self.confidence = 0.98;
            }
            _ => self.analyze_generic_context(),
        }
    }

    /// Analyze threshold patterns
    fn analyze_threshold_context(&mut self) {
        if self.number == 0.8 {
            self.suggested_constant_name = Some("DEFAULT_CONFIDENCE_THRESHOLD".to_string());
            self.suggested_group_class = Some("AnalysisThresholds".to_string());
            self.confidence = 0.8;
        } else if self.number == 0.5 {
            self.suggested_constant_name = Some("MEDIUM_CONFIDENCE_THRESHOLD".to_string());
            self.suggested_group_class = Some("AnalysisThresholds".to_string());
            self.confidence = 0.75;
        }
    }

    /// Analyze configuration patterns
    fn analyze_config_context(&mut self) {
        self.suggested_constant_name = Some(format!("CONFIG_VALUE_{}", self.number as i32));
        self.suggested_group_class = Some("ConfigurationConstants".to_string());
        self.confidence = 0.6;
    }

    /// Generic context analysis fallback
    fn analyze_generic_context(&mut self) {
        // Generate a reasonable constant name based on the number
        if self.number.fract() == 0.0 {
            self.suggested_constant_name = Some(format!("CONSTANT_{}", self.number as i32));
        } else {
            self.suggested_constant_name = Some(format!("CONSTANT_{:.2}", self.number).replace('.', "_"));
        }
        self.suggested_group_class = Some("NumericConstants".to_string());
        self.confidence = 0.4;
    }
}

/// Magic number transformer - extracts magic numbers to constants
pub struct MagicNumberTransformer {
    /// CST transformer for preserving formatting
    cst_transformer: LibCstTransformer,
    /// Configuration for magic number detection
    config: MagicNumberConfig,
}

/// Configuration for magic number detection
#[derive(Debug, Clone)]
pub struct MagicNumberConfig {
    /// Numbers to ignore (common constants)
    pub ignore_numbers: HashSet<String>,
    /// Minimum value to consider as magic number
    pub min_value: f64,
    /// Maximum value to consider as magic number  
    pub max_value: f64,
    /// Whether to extract floating point numbers
    pub extract_floats: bool,
    /// Whether to extract numbers in strings
    pub extract_from_strings: bool,
    /// Prefix for generated constant names
    pub constant_prefix: String,
    /// Where to place constants (top of file, separate constants file)
    pub placement_strategy: ConstantPlacement,
}

/// Strategy for placing extracted constants
#[derive(Debug, Clone, Copy)]
pub enum ConstantPlacement {
    /// At the top of the same file
    TopOfFile,
    /// In a separate constants file
    SeparateFile,
    /// Near first usage
    NearUsage,
}

impl Default for MagicNumberConfig {
    fn default() -> Self {
        let mut ignore_numbers = HashSet::new();
        // Common numbers that are usually not magic
        for num in &["0", "1", "2", "10", "100", "1000", "-1"] {
            ignore_numbers.insert(num.to_string());
        }
        
        Self {
            ignore_numbers,
            min_value: 2.0,
            max_value: 999999.0,
            extract_floats: true,
            extract_from_strings: false,
            constant_prefix: "CONST_".to_string(),
            placement_strategy: ConstantPlacement::TopOfFile,
        }
    }
}

/// Information about a detected magic number
#[derive(Debug, Clone)]
struct MagicNumber {
    /// The numeric value
    value: String,
    /// Line number where it appears
    line_number: usize,
    /// Column position
    column: usize,
    /// Context around the number
    context: String,
    /// Suggested constant name
    suggested_name: String,
    /// How many times this number appears
    usage_count: usize,
}

impl MagicNumberTransformer {
    /// Create new magic number transformer
    pub fn new() -> Result<Self> {
        Ok(Self {
            cst_transformer: LibCstTransformer::new("python")?,
            config: MagicNumberConfig::default(),
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(config: MagicNumberConfig) -> Result<Self> {
        Ok(Self {
            cst_transformer: LibCstTransformer::new("python")?,
            config,
        })
    }
    
    /// Find all magic numbers in the source code
    fn find_magic_numbers(&self, source: &str) -> Result<Vec<MagicNumber>> {
        let mut magic_numbers = Vec::new();
        let mut usage_counts: HashMap<String, usize> = HashMap::new();
        
        // Regex patterns for different number types
        let integer_pattern = Regex::new(r"\b(\d+)\b")?;
        let float_pattern = Regex::new(r"\b(\d+\.\d+)\b")?;
        let hex_pattern = Regex::new(r"\b(0[xX][0-9a-fA-F]+)\b")?;
        let binary_pattern = Regex::new(r"\b(0[bB][01]+)\b")?;
        
        let lines: Vec<&str> = source.lines().collect();
        
        for (line_num, line) in lines.iter().enumerate() {
            let mut numbers_in_line = Vec::new();
            
            // Find integer literals
            for captures in integer_pattern.captures_iter(line) {
                if let Some(matched) = captures.get(1) {
                    let value = matched.as_str();
                    if self.should_extract_number(value, false) {
                        numbers_in_line.push((value.to_string(), matched.start()));
                    }
                }
            }
            
            // Find float literals if enabled
            if self.config.extract_floats {
                for captures in float_pattern.captures_iter(line) {
                    if let Some(matched) = captures.get(1) {
                        let value = matched.as_str();
                        if self.should_extract_number(value, true) {
                            numbers_in_line.push((value.to_string(), matched.start()));
                        }
                    }
                }
            }
            
            // Find hex literals
            for captures in hex_pattern.captures_iter(line) {
                if let Some(matched) = captures.get(1) {
                    let value = matched.as_str();
                    numbers_in_line.push((value.to_string(), matched.start()));
                }
            }
            
            // Find binary literals
            for captures in binary_pattern.captures_iter(line) {
                if let Some(matched) = captures.get(1) {
                    let value = matched.as_str();
                    numbers_in_line.push((value.to_string(), matched.start()));
                }
            }
            
            // Process numbers found in this line
            for (value, column) in numbers_in_line {
                *usage_counts.entry(value.clone()).or_insert(0) += 1;
                
                let suggested_name = self.generate_constant_name(&value, line, line_num);
                let context = self.extract_context(line, column);
                
                magic_numbers.push(MagicNumber {
                    value: value.clone(),
                    line_number: line_num + 1,
                    column,
                    context,
                    suggested_name,
                    usage_count: 1, // Will be updated later
                });
            }
        }
        
        // Update usage counts
        for magic_number in &mut magic_numbers {
            magic_number.usage_count = *usage_counts.get(&magic_number.value).unwrap_or(&1);
        }
        
        Ok(magic_numbers)
    }
    
    /// Check if a number should be extracted as a magic number
    fn should_extract_number(&self, value: &str, is_float: bool) -> bool {
        // Skip ignored numbers
        if self.config.ignore_numbers.contains(value) {
            return false;
        }
        
        // Parse the number to check range
        let num_value = if is_float {
            match value.parse::<f64>() {
                Ok(val) => val,
                Err(_) => return false,
            }
        } else {
            match value.parse::<i64>() {
                Ok(val) => val as f64,
                Err(_) => return false,
            }
        };
        
        if num_value < self.config.min_value || num_value > self.config.max_value {
            return false;
        }
        
        true
    }
    
    /// Generate a constant name for a magic number
    fn generate_constant_name(&self, value: &str, line: &str, line_num: usize) -> String {
        // Try to infer meaning from context
        let context_lower = line.to_lowercase();
        
        // Common patterns
        if context_lower.contains("timeout") || context_lower.contains("delay") {
            return format!("{}TIMEOUT_{}", self.config.constant_prefix, value.replace('.', "_"));
        }
        if context_lower.contains("max") || context_lower.contains("limit") {
            return format!("{}MAX_{}", self.config.constant_prefix, value.replace('.', "_"));
        }
        if context_lower.contains("min") {
            return format!("{}MIN_{}", self.config.constant_prefix, value.replace('.', "_"));
        }
        if context_lower.contains("size") || context_lower.contains("length") {
            return format!("{}SIZE_{}", self.config.constant_prefix, value.replace('.', "_"));
        }
        if context_lower.contains("port") {
            return format!("{}PORT_{}", self.config.constant_prefix, value);
        }
        if context_lower.contains("status") || context_lower.contains("code") {
            return format!("{}STATUS_{}", self.config.constant_prefix, value);
        }
        if context_lower.contains("version") {
            return format!("{}VERSION_{}", self.config.constant_prefix, value.replace('.', "_"));
        }
        
        // Default naming
        format!("{}VALUE_{}_LINE_{}", 
            self.config.constant_prefix, 
            value.replace('.', "_").replace('-', "NEG"), 
            line_num + 1)
    }
    
    /// Extract context around a magic number
    fn extract_context(&self, line: &str, column: usize) -> String {
        let start = column.saturating_sub(20);
        let end = (column + 20).min(line.len());
        let context = &line[start..end];
        
        // Clean up the context
        context.trim().to_string()
    }
    
    /// Generate the replacement code with constants
    fn generate_replacement_code(
        &self,
        source: &str,
        magic_numbers: &[MagicNumber],
    ) -> Result<String> {
        let mut result = String::new();
        
        // Generate constants section
        let constants = self.generate_constants_section(magic_numbers)?;
        
        match self.config.placement_strategy {
            ConstantPlacement::TopOfFile => {
                // Add constants at the top after imports
                let lines: Vec<&str> = source.lines().collect();
                let mut import_end = 0;
                
                // Find end of import statements
                for (i, line) in lines.iter().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("import ") || 
                       trimmed.starts_with("from ") ||
                       trimmed.starts_with("#") ||
                       trimmed.is_empty() {
                        import_end = i;
                    } else {
                        break;
                    }
                }
                
                // Add imports
                for (i, line) in lines.iter().enumerate() {
                    if i <= import_end {
                        result.push_str(line);
                        result.push('\n');
                    } else {
                        break;
                    }
                }
                
                // Add constants
                result.push('\n');
                result.push_str(&constants);
                result.push('\n');
                
                // Add rest of the code with replacements
                for (i, line) in lines.iter().enumerate() {
                    if i > import_end {
                        let replaced_line = self.replace_magic_numbers_in_line(line, magic_numbers);
                        result.push_str(&replaced_line);
                        result.push('\n');
                    }
                }
            }
            
            ConstantPlacement::SeparateFile => {
                // Just replace magic numbers with constant references
                for line in source.lines() {
                    let replaced_line = self.replace_magic_numbers_in_line(line, magic_numbers);
                    result.push_str(&replaced_line);
                    result.push('\n');
                }
            }
            
            ConstantPlacement::NearUsage => {
                // Place constants near their first usage
                let lines: Vec<&str> = source.lines().collect();
                let mut constants_added = HashSet::new();
                
                for (line_num, line) in lines.iter().enumerate() {
                    // Check if any magic numbers appear in this line for the first time
                    for magic_number in magic_numbers {
                        if magic_number.line_number == line_num + 1 && 
                           !constants_added.contains(&magic_number.value) {
                            // Add constant definition before this line
                            result.push_str(&format!("{} = {}\n", 
                                magic_number.suggested_name, magic_number.value));
                            constants_added.insert(magic_number.value.clone());
                        }
                    }
                    
                    // Add the line with replacements
                    let replaced_line = self.replace_magic_numbers_in_line(line, magic_numbers);
                    result.push_str(&replaced_line);
                    result.push('\n');
                }
            }
        }
        
        Ok(result)
    }
    
    /// Generate constants section
    fn generate_constants_section(&self, magic_numbers: &[MagicNumber]) -> Result<String> {
        let mut constants = String::new();
        let mut seen_values = HashSet::new();
        
        constants.push_str("# Constants extracted from magic numbers\n");
        
        // Group by value to avoid duplicates
        for magic_number in magic_numbers {
            if seen_values.insert(magic_number.value.clone()) {
                constants.push_str(&format!(
                    "{} = {}  # Used {} time{} (line {})\n",
                    magic_number.suggested_name,
                    magic_number.value,
                    magic_number.usage_count,
                    if magic_number.usage_count == 1 { "" } else { "s" },
                    magic_number.line_number
                ));
            }
        }
        
        Ok(constants)
    }
    
    /// Replace magic numbers in a single line with constant references
    fn replace_magic_numbers_in_line(&self, line: &str, magic_numbers: &[MagicNumber]) -> String {
        let mut result = line.to_string();
        
        // Sort by column position (descending) to avoid position shifts
        let mut line_magic_numbers: Vec<_> = magic_numbers.iter()
            .filter(|mn| {
                // Check if this magic number appears in this line
                result.contains(&mn.value)
            })
            .collect();
        
        line_magic_numbers.sort_by_key(|mn| std::cmp::Reverse(mn.column));
        
        // Group by value to get the suggested name
        let mut value_to_name: HashMap<String, String> = HashMap::new();
        for magic_number in magic_numbers {
            value_to_name.insert(magic_number.value.clone(), magic_number.suggested_name.clone());
        }
        
        // Replace each unique value
        for (value, name) in value_to_name {
            // Use word boundaries to avoid partial replacements
            let pattern = format!(r"\b{}\b", regex::escape(&value));
            if let Ok(re) = Regex::new(&pattern) {
                result = re.replace_all(&result, name).to_string();
            }
        }
        
        result
    }

    /// Extract magic numbers from analysis result
    fn extract_magic_numbers_from_analysis(&self, analysis_result: &AnalysisResult) -> Result<Vec<MagicNumberContext>> {
        let mut magic_numbers = Vec::new();

        // For now, create a simple implementation
        // In a real implementation, this would parse the analysis result
        // and extract actual magic numbers from the code
        
        // This is a placeholder that would be replaced with actual analysis
        let mut sample_context = MagicNumberContext::new(
            100.0,
            1,
            1,
            "health_score = 100".to_string()
        );
        sample_context.analyze_context();
        magic_numbers.push(sample_context);

        Ok(magic_numbers)
    }

    /// Apply magic number extraction to source code
    fn apply_magic_number_extraction(&self, source_code: &str, number: f64, constant_name: &str) -> Result<String> {
        // Simple regex-based replacement for now
        // In a real implementation, this would use LibCST for precise transformation
        let number_str = if number.fract() == 0.0 {
            format!("{}", number as i64)
        } else {
            format!("{}", number)
        };

        let constant_definition = format!("{} = {}\n", constant_name, number_str);
        let transformed = format!("{}\n{}", constant_definition, source_code.replace(&number_str, constant_name));
        
        Ok(transformed)
    }
}

#[async_trait]
impl Transformer for MagicNumberTransformer {
    /// Analyze code and suggest magic number extraction opportunities
    async fn analyze_transformation_opportunities(
        &self,
        analysis_result: &AnalysisResult,
    ) -> Result<Vec<TransformationSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Extract magic numbers from analysis result
        let magic_numbers = self.extract_magic_numbers_from_analysis(analysis_result)?;
        
        for magic_number in magic_numbers {
            let suggestion = TransformationSuggestion {
                transformation_type: TransformationType::MagicNumbers,
                description: format!(
                    "Extract magic number {} to constant {}",
                    magic_number.number,
                    magic_number.suggested_constant_name.as_deref().unwrap_or("UNKNOWN_CONSTANT")
                ),
                affected_files: vec![analysis_result.codebase_path.clone()],
                confidence: magic_number.confidence,
                estimated_impact: format!("Improve code readability and maintainability"),
                prerequisites: vec!["LibCST availability".to_string()],
                metadata: {
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("number".to_string(), serde_json::json!(magic_number.number));
                    metadata.insert("line".to_string(), serde_json::json!(magic_number.line));
                    metadata.insert("constant_name".to_string(), serde_json::json!(magic_number.suggested_constant_name));
                    metadata.insert("group_class".to_string(), serde_json::json!(magic_number.suggested_group_class));
                    metadata
                },
            };
            suggestions.push(suggestion);
        }
        
        Ok(suggestions)
    }
    
    /// Apply magic number extraction transformation
    async fn apply_transformation(
        &self,
        suggestion: &TransformationSuggestion,
        codebase_path: &std::path::Path,
    ) -> Result<TransformationResult> {
        // Read the source file
        let source_code = std::fs::read_to_string(codebase_path)?;
        
        // Extract transformation parameters from metadata
        let number = suggestion.metadata.get("number")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let constant_name = suggestion.metadata.get("constant_name")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN_CONSTANT");
        
        // Apply the transformation
        let transformed_code = self.apply_magic_number_extraction(&source_code, number, constant_name)?;
        
        // Write transformed code back
        std::fs::write(codebase_path, &transformed_code)?;
        
        Ok(TransformationResult {
            transformation_type: TransformationType::MagicNumbers,
            status: TransformationStatus::Completed,
            files_modified: vec![codebase_path.to_string_lossy().to_string()],
            backup_commit: None, // Will be set by engine
            validation_results: std::collections::HashMap::new(),
            error_message: None,
            rollback_available: true,
        })
    }
    
    /// Validate that transformation was successful
    /// Returns detailed validation results as dict[str, Any] matching Python
    async fn validate_transformation(
        &self,
        result: &TransformationResult,
        codebase_path: &std::path::Path
    ) -> Result<HashMap<String, serde_json::Value>> {
        use std::collections::HashMap;
        use serde_json::json;
        
        let mut validation_results = HashMap::new();
        
        // Basic validation - check if transformation completed successfully
        let is_successful = result.status == TransformationStatus::Completed && result.error_message.is_none();
        validation_results.insert("success".to_string(), json!(is_successful));
        validation_results.insert("status".to_string(), json!(result.status.as_str()));
        
        if let Some(ref error) = result.error_message {
            validation_results.insert("error_message".to_string(), json!(error));
        }
        
        // Validate files were actually modified if claimed
        let mut files_exist = true;
        for file_path in &result.files_modified {
            let full_path = codebase_path.join(file_path);
            if !full_path.exists() {
                files_exist = false;
                break;
            }
        }
        validation_results.insert("files_exist".to_string(), json!(files_exist));
        validation_results.insert("files_modified_count".to_string(), json!(result.files_modified.len()));
        
        Ok(validation_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TransformConfig;

    #[test]
    fn test_magic_number_detection() {
        let transformer = MagicNumberTransformer::new().unwrap();
        let source = r#"
def calculate_price(quantity):
    base_price = 100
    if quantity > 50:
        discount = 0.15
    else:
        discount = 0.05
    
    shipping = 25
    return (base_price * quantity * (1 - discount)) + shipping
"#;
        
        let magic_numbers = transformer.find_magic_numbers(source).unwrap();
        
        // Should find: 100, 50, 0.15, 0.05, 25
        // Should NOT find: 1 (ignored)
        assert!(magic_numbers.len() >= 4);
        
        let values: HashSet<_> = magic_numbers.iter().map(|mn| &mn.value).collect();
        assert!(values.contains(&"100".to_string()));
        assert!(values.contains(&"50".to_string()));
        assert!(values.contains(&"25".to_string()));
    }

    #[test]
    fn test_constant_name_generation() {
        let transformer = MagicNumberTransformer::new().unwrap();
        
        let timeout_name = transformer.generate_constant_name("30", "timeout = 30", 5);
        assert!(timeout_name.contains("TIMEOUT"));
        
        let max_name = transformer.generate_constant_name("100", "max_users = 100", 10);
        assert!(max_name.contains("MAX"));
        
        let port_name = transformer.generate_constant_name("8080", "port = 8080", 15);
        assert!(port_name.contains("PORT"));
    }

    #[test]
    fn test_transformation_with_no_magic_numbers() {
        let transformer = MagicNumberTransformer::new().unwrap();
        let input = TransformInput {
            source_code: "print('hello world')".to_string(),
            file_path: "test.py".to_string(),
            language: "python".to_string(),
            config: TransformConfig::default(),
            analysis_context: None,
        };
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(transformer.transform(&input)).unwrap();
        
        assert!(result.success);
        assert_eq!(result.statistics.transformations_applied, 0);
        assert!(!result.warnings.is_empty());
    }
}