//! Google AI Studio (Gemini) API Integration
//!
//! This module provides integration with Google AI Studio's Gemini models
//! for hierarchical summarization as an alternative to local LLMs.

use crate::{LlmResult, LlmError};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GEMINI_MODEL: &str = "gemini-2.0-flash-exp"; // Free tier model

/// Request structure for Gemini API
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
}

/// Response structure from Gemini API
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}

/// Gemini API client for hierarchical summarization
pub struct GeminiClient {
    api_key: String,
    client: Client,
}

impl GeminiClient {
    /// Create a new Gemini client with API key
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    /// Generate text using Gemini Flash model
    pub async fn generate(&self, prompt: &str) -> LlmResult<String> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE_URL, GEMINI_MODEL, self.api_key
        );

        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: Some(8192),
                top_p: Some(0.95),
                top_k: Some(40),
            }),
        };

        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| LlmError::Inference(format!("Gemini API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::Inference(format!("Gemini API error ({}): {}", status, error_text)));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::Inference(format!("Failed to parse Gemini response: {}", e)))?;

        // Extract text from first candidate
        gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| LlmError::Inference("No response from Gemini API".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_gemini_generation() {
        let api_key = std::env::var("GEMINI_API_KEY")
            .expect("GEMINI_API_KEY must be set for this test");

        let client = GeminiClient::new(api_key);
        let response = client.generate("What is 2+2? Answer with just the number.").await;

        assert!(response.is_ok());
        let text = response.unwrap();
        assert!(text.contains("4"));
    }
}
