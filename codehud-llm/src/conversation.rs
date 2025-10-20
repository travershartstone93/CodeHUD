use crate::{LlmError, LlmResult};
use crate::ffi::PythonLlmBridge;
use crate::crate_summarizer::CrateSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub parent_id: Option<String>,
    pub children_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub turn_id: String,
    pub user_message: ConversationMessage,
    pub assistant_message: Option<ConversationMessage>,
    pub system_messages: Vec<ConversationMessage>,
    pub function_calls: Vec<ConversationMessage>,
    pub turn_metrics: TurnMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnMetrics {
    pub response_time_ms: u64,
    pub token_count: Option<u32>,
    pub model_used: String,
    pub temperature: f32,
    pub quality_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub conversation_id: String,
    pub participant_ids: Vec<String>,
    pub topic: Option<String>,
    pub language: String,
    pub context_window: usize,
    pub max_turns: Option<usize>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub summary: String,
    pub key_points: Vec<String>,
    pub participant_count: usize,
    pub turn_count: usize,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub quality_metrics: ConversationQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationQuality {
    pub coherence_score: f32,
    pub engagement_score: f32,
    pub information_density: f32,
    pub response_relevance: f32,
    pub overall_quality: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationConfig {
    pub max_context_length: usize,
    pub summary_threshold: usize,
    pub auto_summarize: bool,
    pub track_quality: bool,
    pub preserve_system_messages: bool,
    pub compression_strategy: CompressionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionStrategy {
    TruncateOldest,
    SummarizeOldest,
    PreserveImportant,
    AdaptivePriority,
}

pub struct ConversationTracker {
    config: ConversationConfig,
    conversations: HashMap<String, Vec<ConversationTurn>>,
    contexts: HashMap<String, ConversationContext>,
    summaries: HashMap<String, ConversationSummary>,
    python_bridge: Option<PythonLlmBridge>,
}

impl ConversationTracker {
    pub fn new(config: ConversationConfig) -> Self {
        Self {
            config,
            conversations: HashMap::new(),
            contexts: HashMap::new(),
            summaries: HashMap::new(),
            python_bridge: None,
        }
    }

    pub fn with_python_bridge(mut self, bridge: PythonLlmBridge) -> Self {
        self.python_bridge = Some(bridge);
        self
    }

    pub async fn start_conversation(&mut self, context: ConversationContext) -> LlmResult<String> {
        let conversation_id = context.conversation_id.clone();
        self.contexts.insert(conversation_id.clone(), context);
        self.conversations.insert(conversation_id.clone(), Vec::new());

        if let Some(ref bridge) = self.python_bridge {
            bridge.start_conversation(&conversation_id).await?;
        }

        Ok(conversation_id)
    }

    pub async fn add_message(
        &mut self,
        conversation_id: &str,
        role: MessageRole,
        content: String,
        metadata: Option<HashMap<String, String>>,
    ) -> LlmResult<String> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.add_message(conversation_id, &role, &content, metadata.as_ref()).await;
        }
        self.add_message_native(conversation_id, role, content, metadata).await
    }

    async fn add_message_native(
        &mut self,
        conversation_id: &str,
        role: MessageRole,
        content: String,
        metadata: Option<HashMap<String, String>>,
    ) -> LlmResult<String> {
        let message_id = Uuid::new_v4().to_string();
        let message = ConversationMessage {
            id: message_id.clone(),
            role,
            content,
            timestamp: Utc::now(),
            metadata: metadata.unwrap_or_default(),
            parent_id: None,
            children_ids: Vec::new(),
        };

        let conversation = self.conversations
            .get_mut(conversation_id)
            .ok_or_else(|| LlmError::ConversationError(
                format!("Conversation '{}' not found", conversation_id)
            ))?;

        if conversation.is_empty() || matches!(message.role, MessageRole::User) {
            let turn_id = Uuid::new_v4().to_string();
            let turn = ConversationTurn {
                turn_id: turn_id.clone(),
                user_message: message.clone(),
                assistant_message: None,
                system_messages: Vec::new(),
                function_calls: Vec::new(),
                turn_metrics: TurnMetrics {
                    response_time_ms: 0,
                    token_count: None,
                    model_used: "unknown".to_string(),
                    temperature: 0.7,
                    quality_score: None,
                },
            };
            conversation.push(turn);
        } else if let Some(last_turn) = conversation.last_mut() {
            match message.role {
                MessageRole::Assistant => last_turn.assistant_message = Some(message),
                MessageRole::System => last_turn.system_messages.push(message),
                MessageRole::Function => last_turn.function_calls.push(message),
                _ => {},
            }
        }

        self.check_context_window(conversation_id).await?;
        Ok(message_id)
    }

    pub async fn get_conversation_history(
        &self,
        conversation_id: &str,
        limit: Option<usize>,
    ) -> LlmResult<Vec<ConversationTurn>> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.get_conversation_history(conversation_id, limit).await;
        }
        self.get_conversation_history_native(conversation_id, limit).await
    }

    async fn get_conversation_history_native(
        &self,
        conversation_id: &str,
        limit: Option<usize>,
    ) -> LlmResult<Vec<ConversationTurn>> {
        let conversation = self.conversations
            .get(conversation_id)
            .ok_or_else(|| LlmError::ConversationError(
                format!("Conversation '{}' not found", conversation_id)
            ))?;

        let result = if let Some(limit) = limit {
            conversation.iter().rev().take(limit).cloned().collect::<Vec<_>>()
                .into_iter().rev().collect()
        } else {
            conversation.clone()
        };

        Ok(result)
    }

    pub async fn generate_summary(
        &mut self,
        conversation_id: &str,
    ) -> LlmResult<ConversationSummary> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.generate_conversation_summary(conversation_id).await;
        }
        self.generate_summary_native(conversation_id).await
    }

    async fn generate_summary_native(
        &mut self,
        conversation_id: &str,
    ) -> LlmResult<ConversationSummary> {
        let conversation = self.conversations
            .get(conversation_id)
            .ok_or_else(|| LlmError::ConversationError(
                format!("Conversation '{}' not found", conversation_id)
            ))?;

        let context = self.contexts
            .get(conversation_id)
            .ok_or_else(|| LlmError::ConversationError(
                format!("Context for '{}' not found", conversation_id)
            ))?;

        let turn_count = conversation.len();
        let start_time = conversation.first()
            .map(|turn| turn.user_message.timestamp)
            .unwrap_or_else(Utc::now);
        let end_time = conversation.last()
            .and_then(|turn| turn.assistant_message.as_ref())
            .map(|msg| msg.timestamp);

        let summary_text = self.extract_conversation_summary(conversation).await?;
        let key_points = self.extract_key_points(conversation).await?;
        let quality_metrics = self.calculate_quality_metrics(conversation).await?;

        let summary = ConversationSummary {
            conversation_id: conversation_id.to_string(),
            summary: summary_text,
            key_points,
            participant_count: context.participant_ids.len(),
            turn_count,
            start_time,
            end_time,
            quality_metrics,
        };

        self.summaries.insert(conversation_id.to_string(), summary.clone());
        Ok(summary)
    }

    async fn extract_conversation_summary(&self, conversation: &[ConversationTurn]) -> LlmResult<String> {
        if conversation.is_empty() {
            return Ok("Empty conversation".to_string());
        }

        let mut summary_parts = Vec::new();

        for turn in conversation.iter().take(5) {
            let user_preview = turn.user_message.content
                .chars()
                .take(100)
                .collect::<String>();
            summary_parts.push(format!("User: {}", user_preview));

            if let Some(ref assistant_msg) = turn.assistant_message {
                let assistant_preview = assistant_msg.content
                    .chars()
                    .take(100)
                    .collect::<String>();
                summary_parts.push(format!("Assistant: {}", assistant_preview));
            }
        }

        Ok(format!("Conversation with {} turns: {}",
            conversation.len(),
            summary_parts.join(" | ")
        ))
    }

    async fn extract_key_points(&self, conversation: &[ConversationTurn]) -> LlmResult<Vec<String>> {
        let mut key_points = Vec::new();

        for turn in conversation {
            if turn.user_message.content.len() > 50 {
                let words: Vec<&str> = turn.user_message.content.split_whitespace().collect();
                if words.len() > 5 {
                    key_points.push(words[..5].join(" ") + "...");
                }
            }
        }

        Ok(key_points.into_iter().take(10).collect())
    }

    async fn calculate_quality_metrics(&self, conversation: &[ConversationTurn]) -> LlmResult<ConversationQuality> {
        let mut total_response_time = 0u64;
        let mut coherence_scores = Vec::new();
        let mut engagement_scores = Vec::new();

        for turn in conversation {
            total_response_time += turn.turn_metrics.response_time_ms;

            let coherence = self.calculate_coherence_score(&turn.user_message.content,
                turn.assistant_message.as_ref().map(|m| &m.content)).await?;
            coherence_scores.push(coherence);

            let engagement = self.calculate_engagement_score(&turn.user_message.content).await?;
            engagement_scores.push(engagement);
        }

        let avg_coherence = coherence_scores.iter().sum::<f32>() / coherence_scores.len() as f32;
        let avg_engagement = engagement_scores.iter().sum::<f32>() / engagement_scores.len() as f32;
        let avg_response_time = total_response_time / conversation.len() as u64;

        let response_relevance = if avg_response_time < 1000 { 0.9 } else { 0.7 };
        let information_density = (avg_coherence + avg_engagement) / 2.0;
        let overall_quality = (avg_coherence + avg_engagement + response_relevance + information_density) / 4.0;

        Ok(ConversationQuality {
            coherence_score: avg_coherence,
            engagement_score: avg_engagement,
            information_density,
            response_relevance,
            overall_quality,
        })
    }

    async fn calculate_coherence_score(&self, user_msg: &str, assistant_msg: Option<&String>) -> LlmResult<f32> {
        if let Some(assistant_content) = assistant_msg {
            let user_words: Vec<&str> = user_msg.split_whitespace().collect();
            let assistant_words: Vec<&str> = assistant_content.split_whitespace().collect();

            let common_words = user_words.iter()
                .filter(|word| assistant_words.contains(word))
                .count();

            Ok((common_words as f32 / user_words.len().max(1) as f32).min(1.0))
        } else {
            Ok(0.0)
        }
    }

    async fn calculate_engagement_score(&self, content: &str) -> LlmResult<f32> {
        let question_marks = content.matches('?').count();
        let exclamations = content.matches('!').count();
        let word_count = content.split_whitespace().count();

        let engagement = ((question_marks + exclamations) as f32 / word_count.max(1) as f32) * 10.0;
        Ok(engagement.min(1.0))
    }

    async fn check_context_window(&mut self, conversation_id: &str) -> LlmResult<()> {
        let conversation = self.conversations.get(conversation_id)
            .ok_or_else(|| LlmError::ConversationError(
                format!("Conversation '{}' not found", conversation_id)
            ))?;

        let total_length = conversation.iter()
            .map(|turn| {
                turn.user_message.content.len() +
                turn.assistant_message.as_ref().map(|m| m.content.len()).unwrap_or(0) +
                turn.system_messages.iter().map(|m| m.content.len()).sum::<usize>()
            })
            .sum::<usize>();

        if total_length > self.config.max_context_length {
            self.apply_compression(conversation_id).await?;
        }

        Ok(())
    }

    async fn apply_compression(&mut self, conversation_id: &str) -> LlmResult<()> {
        match self.config.compression_strategy {
            CompressionStrategy::TruncateOldest => {
                if let Some(conversation) = self.conversations.get_mut(conversation_id) {
                    while conversation.len() > self.config.max_context_length / 200 {
                        conversation.remove(0);
                    }
                }
            },
            CompressionStrategy::SummarizeOldest => {
                if self.config.auto_summarize {
                    let _ = self.generate_summary(conversation_id).await?;
                }
            },
            _ => {
                // Other strategies would be implemented here
            }
        }
        Ok(())
    }

    pub async fn end_conversation(&mut self, conversation_id: &str) -> LlmResult<ConversationSummary> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.end_conversation(conversation_id).await;
        }

        let summary = self.generate_summary(conversation_id).await?;

        if self.config.auto_summarize {
            self.conversations.remove(conversation_id);
        }

        Ok(summary)
    }

    pub fn get_conversation_context(&self, conversation_id: &str) -> Option<&ConversationContext> {
        self.contexts.get(conversation_id)
    }

    pub fn list_active_conversations(&self) -> Vec<String> {
        self.conversations.keys().cloned().collect()
    }

    pub fn get_conversation_summary(&self, conversation_id: &str) -> Option<&ConversationSummary> {
        self.summaries.get(conversation_id)
    }
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_context_length: 100000,
            summary_threshold: 50,
            auto_summarize: true,
            track_quality: true,
            preserve_system_messages: true,
            compression_strategy: CompressionStrategy::SummarizeOldest,
        }
    }
}

impl ConversationMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            parent_id: None,
            children_ids: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }
}

/// Project-level analysis memory for hierarchical summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysisMemory {
    pub project_context: String,
    pub discovered_patterns: Vec<String>,
    pub architectural_insights: Vec<String>,
    pub crate_relationships: HashMap<String, Vec<String>>,
    pub technology_stack: Vec<String>,
    pub design_patterns: Vec<String>,
    pub processed_crates: Vec<String>,
    pub total_token_budget_used: usize,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl Default for ProjectAnalysisMemory {
    fn default() -> Self {
        Self {
            project_context: String::new(),
            discovered_patterns: Vec::new(),
            architectural_insights: Vec::new(),
            crate_relationships: HashMap::new(),
            technology_stack: Vec::new(),
            design_patterns: Vec::new(),
            processed_crates: Vec::new(),
            total_token_budget_used: 0,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }
}

impl ProjectAnalysisMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulate insights from a crate summary
    pub fn accumulate_crate_insights(&mut self, crate_summary: &CrateSummary) {
        self.processed_crates.push(crate_summary.crate_name.clone());
        self.total_token_budget_used += crate_summary.token_count;
        self.last_updated = Utc::now();

        // Extract patterns from crate summary
        self.extract_patterns_from_summary(&crate_summary.summary_text);

        // Extract architectural insights
        self.extract_architectural_insights(&crate_summary.summary_text);

        // Extract technology stack information
        self.extract_technology_stack(&crate_summary.summary_text);

        // Build crate relationships
        self.build_crate_relationships(&crate_summary.crate_name, &crate_summary.summary_text);
    }

    /// Build enhanced context for final summary generation
    pub fn build_enhanced_context(&self) -> String {
        let mut context = Vec::new();

        if !self.project_context.is_empty() {
            context.push(format!("PROJECT CONTEXT:\n{}", self.project_context));
        }

        if !self.architectural_insights.is_empty() {
            context.push(format!("ARCHITECTURAL INSIGHTS:\n{}",
                self.architectural_insights.join("\n- ")));
        }

        if !self.discovered_patterns.is_empty() {
            context.push(format!("DISCOVERED PATTERNS:\n{}",
                self.discovered_patterns.join("\n- ")));
        }

        if !self.technology_stack.is_empty() {
            context.push(format!("TECHNOLOGY STACK:\n{}",
                self.technology_stack.join(", ")));
        }

        if !self.crate_relationships.is_empty() {
            let relationships: Vec<String> = self.crate_relationships.iter()
                .map(|(crate_name, deps)| format!("{} -> [{}]", crate_name, deps.join(", ")))
                .collect();
            context.push(format!("CRATE RELATIONSHIPS:\n{}", relationships.join("\n")));
        }

        context.push(format!("PROCESSING SUMMARY:\n- Crates analyzed: {}\n- Token budget used: {} tokens",
            self.processed_crates.len(), self.total_token_budget_used));

        context.join("\n\n")
    }

    fn extract_patterns_from_summary(&mut self, summary: &str) {
        let pattern_keywords = [
            "pattern", "architecture", "design", "structure", "framework",
            "async", "thread", "concurrent", "parallel", "event-driven",
            "singleton", "factory", "builder", "observer", "mvc", "mvp", "mvvm"
        ];

        for keyword in &pattern_keywords {
            if summary.to_lowercase().contains(keyword) &&
               !self.discovered_patterns.iter().any(|p| p.contains(keyword)) {
                self.discovered_patterns.push(format!("Uses {} pattern/approach", keyword));
            }
        }
    }

    fn extract_architectural_insights(&mut self, summary: &str) {
        let architecture_keywords = [
            "microservice", "monolith", "layered", "modular", "plugin",
            "api", "rest", "graphql", "database", "cache", "queue",
            "auth", "security", "logging", "monitoring", "testing"
        ];

        for keyword in &architecture_keywords {
            if summary.to_lowercase().contains(keyword) &&
               !self.architectural_insights.iter().any(|i| i.contains(keyword)) {
                self.architectural_insights.push(format!("Implements {} architecture", keyword));
            }
        }
    }

    fn extract_technology_stack(&mut self, summary: &str) {
        let tech_keywords = [
            "rust", "tokio", "async", "serde", "clap", "axum", "warp",
            "diesel", "sqlx", "redis", "postgres", "sqlite", "mongodb",
            "docker", "kubernetes", "grpc", "protobuf", "json", "yaml",
            "tree-sitter", "regex", "chrono", "uuid", "anyhow", "thiserror"
        ];

        for keyword in &tech_keywords {
            if summary.to_lowercase().contains(keyword) &&
               !self.technology_stack.contains(&keyword.to_string()) {
                self.technology_stack.push(keyword.to_string());
            }
        }
    }

    fn build_crate_relationships(&mut self, crate_name: &str, summary: &str) {
        let mut dependencies = Vec::new();

        // Look for crate references in summary
        for processed_crate in &self.processed_crates {
            if processed_crate != crate_name && summary.contains(processed_crate) {
                dependencies.push(processed_crate.clone());
            }
        }

        if !dependencies.is_empty() {
            self.crate_relationships.insert(crate_name.to_string(), dependencies);
        }
    }
}

/// Enhanced conversation tracker with project analysis memory
impl ConversationTracker {
    /// Create a new project analysis memory for hierarchical processing
    pub fn create_project_memory(&mut self, project_path: &str) -> LlmResult<()> {
        let memory = ProjectAnalysisMemory::new();
        // Store in conversation context with special project ID
        let project_id = format!("project:{}", project_path);

        let mut metadata = HashMap::new();
        metadata.insert("tags".to_string(), "project_analysis,hierarchical".to_string());
        metadata.insert("start_time".to_string(), Utc::now().to_rfc3339());

        let context = ConversationContext {
            conversation_id: project_id.clone(),
            participant_ids: vec!["system".to_string(), "hierarchical_analyzer".to_string()],
            topic: Some(format!("Project analysis: {}", project_path)),
            language: "en".to_string(),
            context_window: 12000, // 12K token budget for hierarchical analysis
            max_turns: Some(10),
            metadata,
        };

        self.contexts.insert(project_id.clone(), context);
        self.conversations.insert(project_id, Vec::new());

        Ok(())
    }

    /// Get project analysis memory for a project
    pub fn get_project_memory(&self, project_path: &str) -> Option<ProjectAnalysisMemory> {
        let project_id = format!("project:{}", project_path);
        // For now, create empty memory - in full implementation would persist/retrieve
        Some(ProjectAnalysisMemory::new())
    }

    /// Update project memory with crate insights
    pub fn update_project_memory(&mut self, project_path: &str, crate_summary: &CrateSummary) -> LlmResult<()> {
        // In full implementation, would retrieve, update, and persist memory
        // For now, just log the accumulation
        println!("🧠 Accumulating insights from crate: {} ({} tokens)",
            crate_summary.crate_name, crate_summary.token_count);
        Ok(())
    }
}