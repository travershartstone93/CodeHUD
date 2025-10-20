//! LLM Context Denoiser
//!
//! This module removes redundant information, repeated phrases, and verbose content
//! to fit large datasets within LLM context windows while preserving key insights.

use crate::comment_extractor::{FileCommentExtraction, ExtractedComment, StructuralInsights};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use aho_corasick::AhoCorasick;

/// Configuration for the denoiser
#[derive(Debug, Clone)]
pub struct DenoiserConfig {
    /// Target token reduction percentage (0.0 to 1.0)
    pub target_reduction: f32,
    /// Minimum phrase length to consider for deduplication
    pub min_phrase_length: usize,
    /// Maximum phrase length to consider
    pub max_phrase_length: usize,
    /// Preserve structural insights (narrator bullet points)
    pub preserve_structural_insights: bool,
    /// Preserve file paths and metadata
    pub preserve_metadata: bool,
}

impl Default for DenoiserConfig {
    fn default() -> Self {
        Self {
            target_reduction: 0.6, // 60% reduction
            min_phrase_length: 3,
            max_phrase_length: 20,
            preserve_structural_insights: true,
            preserve_metadata: true,
        }
    }
}

/// Statistics about denoising operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenoiserStats {
    pub original_characters: usize,
    pub cleaned_characters: usize,
    pub reduction_percentage: f32,
    pub repeated_phrases_removed: usize,
    pub common_words_consolidated: usize,
    pub files_processed: usize,
}

/// Main denoiser for LLM context preparation
pub struct LlmContextDenoiser {
    config: DenoiserConfig,
    /// Cache of common phrases found across files
    phrase_frequency: HashMap<String, usize>,
    /// Cache of common words
    word_frequency: HashMap<String, usize>,
}

impl LlmContextDenoiser {
    pub fn new(config: DenoiserConfig) -> Self {
        Self {
            config,
            phrase_frequency: HashMap::new(),
            word_frequency: HashMap::new(),
        }
    }

    /// Denoise a collection of file extractions for LLM consumption
    pub fn denoise_extractions(&mut self, extractions: &[FileCommentExtraction]) -> (Vec<FileCommentExtraction>, DenoiserStats) {
        // Phase 1: Analyze frequency patterns across all files
        self.analyze_frequency_patterns(extractions);

        // Phase 2: Clean each file extraction
        let mut cleaned_extractions = Vec::new();
        let mut original_chars = 0;
        let mut cleaned_chars = 0;
        let mut repeated_phrases_removed = 0;
        let mut common_words_consolidated = 0;

        for extraction in extractions {
            let original_content = self.calculate_content_size(extraction);
            original_chars += original_content;

            let cleaned = self.denoise_single_extraction(extraction);
            let cleaned_content = self.calculate_content_size(&cleaned);
            cleaned_chars += cleaned_content;

            // Track cleaning metrics
            repeated_phrases_removed += self.count_removed_phrases(extraction, &cleaned);
            common_words_consolidated += self.count_consolidated_words(extraction, &cleaned);

            cleaned_extractions.push(cleaned);
        }

        let reduction_percentage = if original_chars > 0 {
            ((original_chars - cleaned_chars) as f32 / original_chars as f32) * 100.0
        } else {
            0.0
        };

        let stats = DenoiserStats {
            original_characters: original_chars,
            cleaned_characters: cleaned_chars,
            reduction_percentage,
            repeated_phrases_removed,
            common_words_consolidated,
            files_processed: extractions.len(),
        };

        (cleaned_extractions, stats)
    }

    /// Analyze frequency patterns across all files to identify redundancy
    fn analyze_frequency_patterns(&mut self, extractions: &[FileCommentExtraction]) {
        self.phrase_frequency.clear();
        self.word_frequency.clear();

        for extraction in extractions {
            // Analyze comments
            for comment in &extraction.comments {
                self.analyze_text_frequency(&comment.text);
            }

            // Analyze structural insights if present
            if let Some(ref insights) = extraction.structural_insights {
                for section_items in insights.sections.values() {
                    for item in section_items {
                        self.analyze_text_frequency(item);
                    }
                }
            }
        }
    }

    /// Analyze frequency of words and phrases in text
    fn analyze_text_frequency(&mut self, text: &str) {
        let words: Vec<&str> = text.split_whitespace().collect();

        // Count word frequency
        for word in &words {
            let cleaned_word = self.normalize_word(word);
            if !cleaned_word.is_empty() && cleaned_word.len() > 2 {
                *self.word_frequency.entry(cleaned_word).or_insert(0) += 1;
            }
        }

        // Count phrase frequency
        for window_size in self.config.min_phrase_length..=self.config.max_phrase_length.min(words.len()) {
            for window in words.windows(window_size) {
                let phrase = window.join(" ");
                let normalized_phrase = self.normalize_phrase(&phrase);
                if !normalized_phrase.is_empty() {
                    *self.phrase_frequency.entry(normalized_phrase).or_insert(0) += 1;
                }
            }
        }
    }

    /// Denoise a single file extraction
    fn denoise_single_extraction(&self, extraction: &FileCommentExtraction) -> FileCommentExtraction {
        let mut cleaned = extraction.clone();

        // Clean comments
        cleaned.comments = extraction.comments.iter()
            .map(|comment| self.denoise_comment(comment))
            .collect();

        // Clean structural insights (preserve but deduplicate)
        if let Some(ref insights) = extraction.structural_insights {
            if self.config.preserve_structural_insights {
                cleaned.structural_insights = Some(self.denoise_structural_insights(insights));
            }
        }

        cleaned
    }

    /// Denoise a single comment
    fn denoise_comment(&self, comment: &ExtractedComment) -> ExtractedComment {
        let cleaned_text = self.denoise_text(&comment.text);

        ExtractedComment {
            text: cleaned_text,
            comment_type: comment.comment_type.clone(),
            start_byte: comment.start_byte,
            end_byte: comment.end_byte,
            start_position: comment.start_position.clone(),
            end_position: comment.end_position.clone(),
            context: comment.context.clone(),
        }
    }

    /// Denoise structural insights while preserving key information
    fn denoise_structural_insights(&self, insights: &StructuralInsights) -> StructuralInsights {
        let mut cleaned_sections = HashMap::new();

        for (section_name, items) in &insights.sections {
            let mut cleaned_items = Vec::new();
            let mut seen_items = HashSet::new();

            for item in items {
                let cleaned_item = self.denoise_text(item);
                // Remove exact duplicates but preserve similar items
                if !seen_items.contains(&cleaned_item) && !cleaned_item.trim().is_empty() {
                    seen_items.insert(cleaned_item.clone());
                    cleaned_items.push(cleaned_item);
                }
            }

            if !cleaned_items.is_empty() {
                cleaned_sections.insert(section_name.clone(), cleaned_items);
            }
        }

        StructuralInsights {
            source: insights.source.clone(),
            generated: insights.generated,
            sections: cleaned_sections,
        }
    }

    /// Core text denoising logic
    fn denoise_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Step 1: Remove highly repeated phrases (appears in 5+ files)
        let frequent_phrases: Vec<_> = self.phrase_frequency.iter()
            .filter(|(_, &count)| count >= 5)
            .map(|(phrase, _)| phrase.as_str())
            .collect();

        if !frequent_phrases.is_empty() {
            let ac = AhoCorasick::new(&frequent_phrases).unwrap();
            result = ac.replace_all(&result, &[""] as &[&str]).to_string();
        }

        // Step 2: Consolidate repeated words
        result = self.consolidate_repeated_words(&result);

        // Step 3: Remove common filler phrases
        result = self.remove_filler_phrases(&result);

        // Step 4: Clean up whitespace
        result = self.normalize_whitespace(&result);

        result
    }

    /// Consolidate repeated words within the same text
    fn consolidate_repeated_words(&self, text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut result_words = Vec::new();
        let mut word_count = HashMap::new();

        for word in words {
            let normalized = self.normalize_word(word);
            let count = word_count.entry(normalized.clone()).or_insert(0);
            *count += 1;

            // Only include word if it hasn't appeared too many times
            if *count <= 2 || normalized.len() <= 3 {
                result_words.push(word);
            }
        }

        result_words.join(" ")
    }

    /// Remove common filler phrases that add no value
    fn remove_filler_phrases(&self, text: &str) -> String {
        let filler_phrases = vec![
            "this function",
            "this method",
            "this file",
            "this module",
            "this struct",
            "this enum",
            "this implementation",
            "as mentioned",
            "it should be noted",
            "it is important",
            "please note",
            "note that",
            "it appears",
            "seems to",
            "appears to be",
        ];

        let mut result = text.to_string();
        for phrase in filler_phrases {
            result = result.replace(phrase, "");
        }
        result
    }

    /// Normalize whitespace and remove empty lines
    fn normalize_whitespace(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Normalize a word for frequency analysis
    fn normalize_word(&self, word: &str) -> String {
        word.to_lowercase()
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_string()
    }

    /// Normalize a phrase for frequency analysis
    fn normalize_phrase(&self, phrase: &str) -> String {
        phrase.to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 2)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Calculate content size for statistics
    fn calculate_content_size(&self, extraction: &FileCommentExtraction) -> usize {
        let mut size = 0;

        for comment in &extraction.comments {
            size += comment.text.len();
        }

        if let Some(ref insights) = extraction.structural_insights {
            for items in insights.sections.values() {
                for item in items {
                    size += item.len();
                }
            }
        }

        size
    }

    /// Count removed phrases for statistics
    fn count_removed_phrases(&self, _original: &FileCommentExtraction, _cleaned: &FileCommentExtraction) -> usize {
        // Simplified for now - could implement detailed tracking
        0
    }

    /// Count consolidated words for statistics
    fn count_consolidated_words(&self, _original: &FileCommentExtraction, _cleaned: &FileCommentExtraction) -> usize {
        // Simplified for now - could implement detailed tracking
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment_extractor::CommentType;

    #[test]
    fn test_denoise_text_removes_repeated_phrases() {
        let mut denoiser = LlmContextDenoiser::new(DenoiserConfig::default());

        // Simulate frequent phrases
        denoiser.phrase_frequency.insert("this function".to_string(), 10);

        let text = "this function does something and this function is important";
        let result = denoiser.denoise_text(text);

        assert!(!result.contains("this function"));
    }

    #[test]
    fn test_consolidate_repeated_words() {
        let denoiser = LlmContextDenoiser::new(DenoiserConfig::default());
        let text = "function function function does something something";
        let result = denoiser.consolidate_repeated_words(text);

        // Should reduce repeated instances
        assert!(result.matches("function").count() <= 2);
    }

    #[test]
    fn test_preserve_structural_insights() {
        let config = DenoiserConfig {
            preserve_structural_insights: true,
            ..Default::default()
        };
        let denoiser = LlmContextDenoiser::new(config);

        let mut sections = HashMap::new();
        sections.insert("functions".to_string(), vec!["- Important function".to_string()]);

        let insights = StructuralInsights {
            source: "test".to_string(),
            generated: true,
            sections,
        };

        let cleaned = denoiser.denoise_structural_insights(&insights);
        assert!(cleaned.sections.contains_key("functions"));
    }
}