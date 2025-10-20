//! String processing utilities with Python compatibility
//!
//! This module provides string operations that behave identically
//! to Python's string methods and utilities.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use regex::Regex;

/// Safely truncate text to maximum length (matches Python textwrap behavior)
pub fn safe_truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    
    // Find grapheme boundary at or before max_len
    let mut truncated = String::new();
    let mut current_len = 0;
    
    for grapheme in text.graphemes(true) {
        let grapheme_len = grapheme.len();
        if current_len + grapheme_len > max_len {
            break;
        }
        truncated.push_str(grapheme);
        current_len += grapheme_len;
    }
    
    // Add ellipsis if truncated and there's space
    if current_len < text.len() && current_len + 3 <= max_len {
        truncated.push_str("...");
    } else if current_len < text.len() && max_len >= 3 {
        // Replace last characters with ellipsis
        truncated.truncate(max_len - 3);
        truncated.push_str("...");
    }
    
    truncated
}

/// Normalize whitespace (matches Python string normalization)
pub fn normalize_whitespace(text: &str) -> String {
    // Normalize Unicode
    let normalized: String = text.nfc().collect();
    
    // Replace all whitespace sequences with single spaces
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    let result = whitespace_regex.replace_all(&normalized, " ");
    
    // Strip leading and trailing whitespace
    result.trim().to_string()
}

/// Extract function names from code (Python-like regex patterns)
pub fn extract_function_names(code: &str, language: &str) -> Vec<String> {
    let pattern = match language.to_lowercase().as_str() {
        "python" => r"def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(",
        "javascript" | "typescript" => r"function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(",
        "rust" => r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(",
        "java" => r"(?:public|private|protected)?\s*(?:static)?\s*\w+\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(",
        _ => return Vec::new(),
    };
    
    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    
    regex.captures_iter(code)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Calculate string similarity (Levenshtein distance-based, 0.0 to 1.0)
pub fn calculate_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }
    
    let distance = levenshtein_distance(s1, s2);
    let max_len = s1.len().max(s2.len());
    
    1.0 - (distance as f64 / max_len as f64)
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();
    
    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];
    
    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    // Fill the matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[len1][len2]
}

/// Convert camelCase to snake_case (Python convention)
pub fn camel_to_snake_case(text: &str) -> String {
    let regex = Regex::new(r"([a-z0-9])([A-Z])").unwrap();
    regex.replace_all(text, "${1}_${2}").to_lowercase()
}

/// Convert snake_case to camelCase
pub fn snake_to_camel_case(text: &str) -> String {
    let parts: Vec<&str> = text.split('_').collect();
    if parts.is_empty() {
        return String::new();
    }
    
    let mut result = parts[0].to_lowercase();
    for part in &parts[1..] {
        if !part.is_empty() {
            result.push_str(&capitalize_first_letter(part));
        }
    }
    
    result
}

/// Capitalize first letter of a string
pub fn capitalize_first_letter(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if !chars.is_empty() {
        chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
    }
    chars.into_iter().collect()
}

/// Split text into lines preserving line endings (matches Python splitlines)
pub fn splitlines(text: &str) -> Vec<String> {
    text.lines().map(|line| line.to_string()).collect()
}

/// Remove common leading whitespace from lines (like Python textwrap.dedent)
pub fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    
    // Find minimum indentation (excluding empty lines)
    let min_indent = lines.iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    
    // Remove common indentation
    lines.iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(min_indent).collect()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Check if string is valid identifier (Python rules)
pub fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    
    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }
    
    // Remaining characters must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate() {
        assert_eq!(safe_truncate("hello world", 20), "hello world");
        assert_eq!(safe_truncate("hello world", 8), "hello...");
        assert_eq!(safe_truncate("hello", 3), "hel");
        assert_eq!(safe_truncate("", 5), "");
        
        // Test with Unicode
        assert_eq!(safe_truncate("café", 3), "caf");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("  hello   world  "), "hello world");
        assert_eq!(normalize_whitespace("hello\n\tworld"), "hello world");
        assert_eq!(normalize_whitespace(""), "");
    }

    #[test]
    fn test_extract_function_names() {
        let python_code = "def hello():\n    pass\ndef world(x):\n    return x";
        let names = extract_function_names(python_code, "python");
        assert_eq!(names, vec!["hello", "world"]);
        
        let rust_code = "fn test() -> i32 {\n    42\n}\nfn main() {\n}";
        let names = extract_function_names(rust_code, "rust");
        assert_eq!(names, vec!["test", "main"]);
    }

    #[test]
    fn test_calculate_similarity() {
        assert_eq!(calculate_similarity("hello", "hello"), 1.0);
        assert_eq!(calculate_similarity("", ""), 1.0);
        assert_eq!(calculate_similarity("hello", ""), 0.0);
        assert!(calculate_similarity("hello", "hallo") > 0.5);
        assert!(calculate_similarity("completely", "different") < 0.5);
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(camel_to_snake_case("camelCase"), "camel_case");
        assert_eq!(camel_to_snake_case("XMLParser"), "xml_parser");
        assert_eq!(snake_to_camel_case("snake_case"), "snakeCase");
        assert_eq!(snake_to_camel_case("xml_parser"), "xmlParser");
    }

    #[test]
    fn test_dedent() {
        let indented = "    line 1\n    line 2\n        line 3";
        let expected = "line 1\nline 2\n    line 3";
        assert_eq!(dedent(indented), expected);
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("valid_name"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("name123"));
        assert!(!is_valid_identifier("123name"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("with-dash"));
    }

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(capitalize_first_letter("hello"), "Hello");
        assert_eq!(capitalize_first_letter("HELLO"), "HELLO");
        assert_eq!(capitalize_first_letter(""), "");
    }
}