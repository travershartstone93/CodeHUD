//! Ollama Pipeline Integration - GPU accelerated local LLM inference
//!
//! This module provides integration with Ollama for local LLM inference with 4+ model types
//! and GPU acceleration support, matching Python implementation exactly.

#[cfg(feature = "candle")]
use crate::{LlmConfig, ModelType, GpuType, LlmResult, LlmError, ffi::PythonLlmBridge, native::NativeLlmEngine};

#[cfg(not(feature = "candle"))]
use crate::{LlmConfig, ModelType, GpuType, LlmResult, LlmError, ffi::PythonLlmBridge, native_stub::NativeLlmEngine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;

/// Ollama configuration matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama server URL
    pub server_url: String,
    /// Connection timeout
    pub timeout: Duration,
    /// Model configuration
    pub model_config: ModelConfig,
    /// GPU settings
    pub gpu_config: GpuConfig,
    /// Session settings
    pub session_config: SessionConfig,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:11434".to_string(),
            timeout: Duration::from_secs(120),
            model_config: ModelConfig::default(),
            gpu_config: GpuConfig::default(),
            session_config: SessionConfig::default(),
        }
    }
}

/// Model-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model type to use
    pub model_type: ModelType,
    /// Model parameters
    pub parameters: ModelParameters,
    /// Context length
    pub context_length: usize,
    /// Whether to keep model loaded
    pub keep_alive: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::DeepSeekCoder,
            parameters: ModelParameters::default(),
            context_length: 4096,
            keep_alive: true,
        }
    }
}

/// Model parameters for inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Sampling temperature
    pub temperature: f32,
    /// Top-p nucleus sampling
    pub top_p: f32,
    /// Top-k sampling
    pub top_k: i32,
    /// Repeat penalty
    pub repeat_penalty: f32,
    /// Random seed
    pub seed: Option<u64>,
    /// Stop sequences
    pub stop: Vec<String>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.1,
            top_p: 0.95,
            top_k: 40,
            repeat_penalty: 1.1,
            seed: None,
            stop: vec![],
        }
    }
}

/// GPU acceleration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// GPU type to use
    pub gpu_type: GpuType,
    /// Number of GPU layers to use
    pub gpu_layers: Option<i32>,
    /// GPU memory fraction to use
    pub memory_fraction: Option<f32>,
    /// Whether to enable memory mapping
    pub use_mmap: bool,
    /// Whether to enable memory locking
    pub use_mlock: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            gpu_type: GpuType::Cuda,
            gpu_layers: None, // Auto-detect
            memory_fraction: Some(0.8),
            use_mmap: true,
            use_mlock: false,
        }
    }
}

/// Session management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum conversation length
    pub max_conversation_length: usize,
    /// Session timeout
    pub session_timeout: Duration,
    /// Whether to persist conversations
    pub persist_conversations: bool,
    /// Context window management
    pub context_window: ContextWindow,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_conversation_length: 50,
            session_timeout: Duration::from_secs(3600),
            persist_conversations: true,
            context_window: ContextWindow::default(),
        }
    }
}

/// Context window management strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Strategy for handling context overflow
    pub overflow_strategy: OverflowStrategy,
    /// Target context utilization (0.0-1.0)
    pub target_utilization: f32,
    /// Whether to summarize old context
    pub enable_summarization: bool,
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self {
            overflow_strategy: OverflowStrategy::SlidingWindow,
            target_utilization: 0.8,
            enable_summarization: true,
        }
    }
}

/// Strategy for handling context window overflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverflowStrategy {
    /// Truncate oldest messages
    Truncate,
    /// Use sliding window approach
    SlidingWindow,
    /// Summarize and compress
    Summarize,
    /// Fail on overflow
    Fail,
}

/// Ollama API request structure
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<i32>>,
    #[serde(flatten)]
    options: ModelParameters,
    stream: bool,
}

/// Ollama API response structure
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_eval_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eval_count: Option<i32>,
}

/// Ollama model information
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: ModelDetails,
}

/// Detailed model information
#[derive(Debug, Deserialize)]
pub struct ModelDetails {
    pub format: String,
    pub family: String,
    pub families: Vec<String>,
    pub parameter_size: String,
    pub quantization_level: String,
}

/// Ollama pipeline for local LLM inference with GPU acceleration
/// 
/// This implementation provides a Rust interface while delegating to the Python
/// implementation during Phase 5a to ensure zero-degradation compatibility.
pub struct OllamaPipeline {
    /// Configuration
    config: OllamaConfig,
    /// HTTP client for API communication
    client: Client,
    /// Native LLM engine (Phase 5b primary implementation)
    native_engine: Option<NativeLlmEngine>,
    /// Python FFI bridge (Phase 5a fallback)
    python_bridge: Option<PythonLlmBridge>,
    /// Current conversation context
    conversation_context: Option<Vec<i32>>,
    /// Session ID for tracking
    session_id: Option<String>,
}

impl OllamaPipeline {
    /// Create a new Ollama pipeline
    pub fn new(config: OllamaConfig) -> LlmResult<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| LlmError::Http(e))?;
            
        Ok(Self {
            config,
            client,
            native_engine: None,
            python_bridge: None,
            conversation_context: None,
            session_id: None,
        })
    }
    
    /// Create with native engine for Phase 5b (primary implementation)
    pub async fn with_native_engine(config: OllamaConfig) -> LlmResult<Self> {
        let mut pipeline = Self::new(config.clone())?;

        // Initialize native engine with LLM config
        let llm_config = LlmConfig {
            model_type: config.model_config.model_type.clone(),
            gpu_type: config.gpu_config.gpu_type.clone(),
            max_tokens: 4096,
            temperature: config.model_config.parameters.temperature,
            top_p: config.model_config.parameters.top_p,
            seed: config.model_config.parameters.seed,
            structured_generation: true,
            critical_detection: true,
            constitutional_ai: true,
            session_timeout: 3600,
        };

        let native_engine = NativeLlmEngine::new(llm_config).await?;
        pipeline.native_engine = Some(native_engine);

        Ok(pipeline)
    }

    /// Create with Python FFI bridge for Phase 5a compatibility
    pub fn with_python_bridge(
        config: OllamaConfig,
        python_bridge: PythonLlmBridge,
    ) -> LlmResult<Self> {
        let mut pipeline = Self::new(config)?;
        pipeline.python_bridge = Some(python_bridge);
        Ok(pipeline)
    }
    
    /// Check if Ollama is available and running
    pub async fn is_available(&self) -> bool {
        match self.client.get(&format!("{}/api/tags", self.config.server_url)).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
    
    /// List available models
    pub async fn list_models(&self) -> LlmResult<Vec<ModelInfo>> {
        let response = self
            .client
            .get(&format!("{}/api/tags", self.config.server_url))
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(LlmError::Http(reqwest::Error::from(response.error_for_status().unwrap_err())));
        }
        
        #[derive(Deserialize)]
        struct ModelsResponse {
            models: Vec<ModelInfo>,
        }
        
        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response.models)
    }
    
    /// Pull a model from the Ollama registry
    pub async fn pull_model(&self, model_name: &str) -> LlmResult<()> {
        #[derive(Serialize)]
        struct PullRequest {
            name: String,
        }
        
        let request = PullRequest {
            name: model_name.to_string(),
        };
        
        let response = self
            .client
            .post(&format!("{}/api/pull", self.config.server_url))
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(LlmError::Inference(format!(
                "Failed to pull model {}: {}",
                model_name,
                response.status()
            )));
        }
        
        Ok(())
    }
    
    /// Generate text using Ollama with GPU acceleration
    pub async fn generate(
        &mut self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> LlmResult<String> {
        // Phase 5b: Use native engine as primary implementation
        if let Some(ref native_engine) = self.native_engine {
            let full_prompt = if let Some(sys) = system_prompt {
                format!("{}\n\nUser: {}\nAssistant:", sys, prompt)
            } else {
                format!("User: {}\nAssistant:", prompt)
            };

            // Generate structured code using constraints for better quality
            let constraints = crate::structured::GenerationConstraints {
                json_schema: None,
                grammar_rules: Some("natural_language_response".to_string()),
                max_length: Some(2048),
                output_format: crate::structured::OutputFormat::PlainText,
                validation_rules: vec!["coherent_response".to_string()],
            };

            let ffi_constraints = crate::ffi::GenerationConstraints {
                json_schema: constraints.json_schema.clone(),
                grammar_rules: constraints.grammar_rules.clone(),
                max_length: constraints.max_length,
                output_format: match constraints.output_format {
                    crate::structured::OutputFormat::PlainText => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::Text => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::JsonObject => crate::ffi::OutputFormat::Json,
                    crate::structured::OutputFormat::Json => crate::ffi::OutputFormat::Json,
                    crate::structured::OutputFormat::PythonCode => crate::ffi::OutputFormat::PythonCode,
                    crate::structured::OutputFormat::RustCode => crate::ffi::OutputFormat::RustCode,
                    crate::structured::OutputFormat::Markdown => crate::ffi::OutputFormat::Markdown,
                    crate::structured::OutputFormat::JavaScriptCode => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::Html => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::Custom(_) => crate::ffi::OutputFormat::Text,
                },
                validation_rules: constraints.validation_rules.clone(),
            };
            return native_engine.generate_structured_code(&full_prompt, &ffi_constraints).await;
        }

        // Phase 5a: Fallback to Python bridge if available for guaranteed compatibility
        if let Some(ref python_bridge) = self.python_bridge {
            return self.generate_via_python_bridge(prompt, system_prompt, python_bridge);
        }

        // Final fallback: Native Ollama HTTP API implementation
        self.generate_native(prompt, system_prompt).await
    }
    
    /// Generate via Python bridge (Phase 5a)
    fn generate_via_python_bridge(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        python_bridge: &PythonLlmBridge,
    ) -> LlmResult<String> {
        // Call Python Ollama pipeline directly
        use pyo3::prelude::*;
        use pyo3::types::PyDict;
        
        Python::with_gil(|py| {
            let kwargs = PyDict::new(py);
            kwargs.set_item("prompt", prompt)?;
            
            if let Some(system) = system_prompt {
                kwargs.set_item("system", system)?;
            }
            
            kwargs.set_item("model", self.config.model_config.model_type.ollama_name())?;
            kwargs.set_item("temperature", self.config.model_config.parameters.temperature)?;
            kwargs.set_item("top_p", self.config.model_config.parameters.top_p)?;
            kwargs.set_item("top_k", self.config.model_config.parameters.top_k)?;
            
            if let Some(seed) = self.config.model_config.parameters.seed {
                kwargs.set_item("seed", seed)?;
            }
            
            let result = python_bridge.ollama_pipeline
                .call_method(py, "generate", (), Some(kwargs))?;
                
            let generated_text: String = result.extract(py)?;
            Ok(generated_text)
        })
    }
    
    /// Native Rust generation (will be used in Phase 5b)
    async fn generate_native(
        &mut self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> LlmResult<String> {
        let request = OllamaRequest {
            model: self.config.model_config.model_type.ollama_name().to_string(),
            prompt: prompt.to_string(),
            system: system_prompt.map(|s| s.to_string()),
            context: self.conversation_context.clone(),
            options: self.config.model_config.parameters.clone(),
            stream: false,
        };
        
        let response = self
            .client
            .post(&format!("{}/api/generate", self.config.server_url))
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(LlmError::Inference(format!(
                "Ollama generation failed: {}",
                response.status()
            )));
        }
        
        let ollama_response: OllamaResponse = response.json().await?;
        
        // Update conversation context for session continuity
        if let Some(context) = ollama_response.context {
            self.conversation_context = Some(context);
        }
        
        Ok(ollama_response.response)
    }
    
    /// Generate structured code with constraints
    pub async fn generate_structured_code(
        &mut self,
        prompt: &str,
        constraints: &crate::structured::GenerationConstraints,
    ) -> LlmResult<String> {
        // Phase 5b: Use native engine as primary implementation
        if let Some(ref native_engine) = self.native_engine {
            let ffi_constraints = crate::ffi::GenerationConstraints {
                json_schema: constraints.json_schema.clone(),
                grammar_rules: constraints.grammar_rules.clone(),
                max_length: constraints.max_length,
                output_format: match constraints.output_format {
                    crate::structured::OutputFormat::PlainText => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::JsonObject => crate::ffi::OutputFormat::Json,
                    crate::structured::OutputFormat::PythonCode => crate::ffi::OutputFormat::PythonCode,
                    crate::structured::OutputFormat::RustCode => crate::ffi::OutputFormat::RustCode,
                    crate::structured::OutputFormat::Markdown => crate::ffi::OutputFormat::Markdown,
                    crate::structured::OutputFormat::JavaScriptCode => crate::ffi::OutputFormat::Text,
                    _ => crate::ffi::OutputFormat::Text,
                },
                validation_rules: constraints.validation_rules.clone(),
            };
            return native_engine.generate_structured_code(prompt, &ffi_constraints).await;
        }

        // Phase 5a: Fallback to Python bridge for guaranteed compatibility
        if let Some(ref python_bridge) = self.python_bridge {
            let ffi_constraints = crate::ffi::GenerationConstraints {
                json_schema: constraints.json_schema.clone(),
                grammar_rules: constraints.grammar_rules.clone(),
                max_length: constraints.max_length,
                output_format: match constraints.output_format {
                    crate::structured::OutputFormat::PlainText => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::Text => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::JsonObject => crate::ffi::OutputFormat::Json,
                    crate::structured::OutputFormat::Json => crate::ffi::OutputFormat::Json,
                    crate::structured::OutputFormat::PythonCode => crate::ffi::OutputFormat::PythonCode,
                    crate::structured::OutputFormat::RustCode => crate::ffi::OutputFormat::RustCode,
                    crate::structured::OutputFormat::Markdown => crate::ffi::OutputFormat::Markdown,
                    crate::structured::OutputFormat::JavaScriptCode => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::Html => crate::ffi::OutputFormat::Text,
                    crate::structured::OutputFormat::Custom(_) => crate::ffi::OutputFormat::Text,
                },
                validation_rules: constraints.validation_rules.clone(),
            };
            return Ok(python_bridge.generate_structured_code(prompt, &ffi_constraints)?);
        }

        // Final fallback: Native structured generation - convert constraints
        let ffi_constraints = crate::ffi::GenerationConstraints {
            json_schema: constraints.json_schema.clone(),
            grammar_rules: constraints.grammar_rules.clone(),
            max_length: constraints.max_length,
            output_format: match constraints.output_format {
                crate::structured::OutputFormat::PlainText => crate::ffi::OutputFormat::Text,
                crate::structured::OutputFormat::JsonObject => crate::ffi::OutputFormat::Json,
                crate::structured::OutputFormat::PythonCode => crate::ffi::OutputFormat::PythonCode,
                crate::structured::OutputFormat::RustCode => crate::ffi::OutputFormat::RustCode,
                crate::structured::OutputFormat::Markdown => crate::ffi::OutputFormat::Markdown,
                crate::structured::OutputFormat::JavaScriptCode => crate::ffi::OutputFormat::Text,
                _ => crate::ffi::OutputFormat::Text,
            },
            validation_rules: constraints.validation_rules.clone(),
        };
        self.generate_structured_native(prompt, &ffi_constraints).await
    }
    
    /// Native structured generation implementation
    async fn generate_structured_native(
        &mut self,
        prompt: &str,
        constraints: &crate::ffi::GenerationConstraints,
    ) -> LlmResult<String> {
        // Enhanced prompt with constraints
        let mut enhanced_prompt = prompt.to_string();
        
        match constraints.output_format {
            crate::ffi::OutputFormat::Json => {
                enhanced_prompt.push_str("\n\nPlease respond with valid JSON only.");
                if let Some(ref schema) = constraints.json_schema {
                    enhanced_prompt.push_str(&format!(
                        " Follow this JSON schema: {}",
                        schema
                    ));
                }
            }
            crate::ffi::OutputFormat::PythonCode => {
                enhanced_prompt.push_str("\n\nPlease respond with valid Python code only.");
            }
            crate::ffi::OutputFormat::RustCode => {
                enhanced_prompt.push_str("\n\nPlease respond with valid Rust code only.");
            }
            crate::ffi::OutputFormat::Markdown => {
                enhanced_prompt.push_str("\n\nPlease respond with properly formatted Markdown.");
            }
            crate::ffi::OutputFormat::Text => {
                // No special formatting required
            }
        }
        
        if let Some(max_length) = constraints.max_length {
            enhanced_prompt.push_str(&format!(
                " Keep the response under {} characters.",
                max_length
            ));
        }
        
        // Add validation rules as instructions
        if !constraints.validation_rules.is_empty() {
            enhanced_prompt.push_str("\n\nImportant constraints:");
            for rule in &constraints.validation_rules {
                enhanced_prompt.push_str(&format!("\n- {}", rule));
            }
        }
        
        self.generate_native(&enhanced_prompt, None).await
    }
    
    /// Start a new conversation session
    pub fn start_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
        self.conversation_context = None;
    }
    
    /// End the current session
    pub fn end_session(&mut self) {
        self.session_id = None;
        self.conversation_context = None;
    }
    
    /// Get current session ID
    pub fn get_session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
    
    /// Check if model is loaded
    pub async fn is_model_loaded(&self) -> LlmResult<bool> {
        let models = self.list_models().await?;
        let model_name = self.config.model_config.model_type.ollama_name();
        
        Ok(models.iter().any(|m| m.name.contains(model_name)))
    }
    
    /// Ensure model is available (pull if necessary)
    pub async fn ensure_model_available(&self) -> LlmResult<()> {
        if !self.is_model_loaded().await? {
            let model_name = self.config.model_config.model_type.ollama_name();
            self.pull_model(model_name).await?;
        }
        Ok(())
    }
    
    /// Get GPU information
    pub async fn get_gpu_info(&self) -> LlmResult<HashMap<String, serde_json::Value>> {
        // Check GPU availability through system commands
        let mut gpu_info = HashMap::new();
        
        // Check NVIDIA GPU
        if let Ok(output) = Command::new("nvidia-smi")
            .arg("--query-gpu=name,memory.total,memory.free")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
        {
            if output.status.success() {
                let gpu_output = String::from_utf8_lossy(&output.stdout);
                gpu_info.insert("nvidia".to_string(), serde_json::json!(gpu_output.trim()));
            }
        }
        
        // Check AMD GPU (ROCm)
        if let Ok(output) = Command::new("rocm-smi")
            .arg("--showmeminfo")
            .arg("vram")
            .output()
            .await
        {
            if output.status.success() {
                let gpu_output = String::from_utf8_lossy(&output.stdout);
                gpu_info.insert("amd".to_string(), serde_json::json!(gpu_output.trim()));
            }
        }
        
        // Check Metal (macOS)
        #[cfg(target_os = "macos")]
        {
            gpu_info.insert("metal".to_string(), serde_json::json!("Available on macOS"));
        }
        
        gpu_info.insert("configured_type".to_string(), serde_json::json!(self.config.gpu_config.gpu_type));
        
        Ok(gpu_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.server_url, "http://localhost:11434");
        assert_eq!(config.model_config.model_type, ModelType::DeepSeekCoder);
    }
    
    #[tokio::test]
    async fn test_ollama_pipeline_creation() {
        let config = OllamaConfig::default();
        let pipeline = OllamaPipeline::new(config);
        assert!(pipeline.is_ok());
    }
    
    #[tokio::test]
    #[ignore] // Requires Ollama server running
    async fn test_ollama_availability() {
        let config = OllamaConfig::default();
        let pipeline = OllamaPipeline::new(config).unwrap();
        
        // This test requires Ollama to be running
        let is_available = pipeline.is_available().await;
        println!("Ollama available: {}", is_available);
    }
    
    #[test]
    fn test_model_type_names() {
        assert_eq!(ModelType::DeepSeekCoder.ollama_name(), "deepseek-coder");
        assert_eq!(ModelType::Qwen2_5Coder.ollama_name(), "qwen2.5-coder");
        assert_eq!(ModelType::CodeLlama.ollama_name(), "codellama");
        assert_eq!(ModelType::Mistral.ollama_name(), "mistral");
    }
    
    #[test]
    fn test_generation_constraints() {
        let constraints = crate::ffi::GenerationConstraints {
            json_schema: Some(serde_json::json!({"type": "object"})),
            grammar_rules: None,
            max_length: Some(1000),
            output_format: crate::ffi::OutputFormat::Json,
            validation_rules: vec!["no_dangerous_code".to_string()],
        };
        
        assert_eq!(constraints.output_format, crate::ffi::OutputFormat::Json);
        assert_eq!(constraints.max_length, Some(1000));
        assert_eq!(constraints.validation_rules.len(), 1);
    }
}