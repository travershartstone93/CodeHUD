use crate::{LlmError, LlmResult};
use crate::ffi::PythonLlmBridge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionalPrinciple {
    pub id: String,
    pub name: String,
    pub description: String,
    pub weight: f32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailViolation {
    pub principle_id: String,
    pub severity: ViolationSeverity,
    pub description: String,
    pub suggestion: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionalConfig {
    pub principles: Vec<ConstitutionalPrinciple>,
    pub strict_mode: bool,
    pub auto_correction: bool,
    pub violation_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionalAssessment {
    pub passed: bool,
    pub violations: Vec<GuardrailViolation>,
    pub overall_score: f32,
    pub timestamp: DateTime<Utc>,
    pub suggestions: Vec<String>,
}

pub struct ConstitutionalAI {
    config: ConstitutionalConfig,
    python_bridge: Option<PythonLlmBridge>,
    principle_cache: HashMap<String, ConstitutionalPrinciple>,
}

impl ConstitutionalAI {
    pub fn new(config: ConstitutionalConfig) -> Self {
        let mut principle_cache = HashMap::new();
        for principle in &config.principles {
            principle_cache.insert(principle.id.clone(), principle.clone());
        }

        Self {
            config,
            python_bridge: None,
            principle_cache,
        }
    }

    pub fn with_python_bridge(mut self, bridge: PythonLlmBridge) -> Self {
        self.python_bridge = Some(bridge);
        self
    }

    pub async fn assess_content(&self, content: &str) -> LlmResult<ConstitutionalAssessment> {
        if let Some(ref bridge) = self.python_bridge {
            return self.assess_via_python_bridge(content, bridge).await;
        }
        self.assess_native(content).await
    }

    async fn assess_via_python_bridge(
        &self,
        content: &str,
        bridge: &PythonLlmBridge,
    ) -> LlmResult<ConstitutionalAssessment> {
        bridge.assess_constitutional_ai(content, &self.config).await
    }

    async fn assess_native(&self, content: &str) -> LlmResult<ConstitutionalAssessment> {
        let mut violations = Vec::new();
        let mut total_score = 0.0f32;
        let mut weighted_sum = 0.0f32;

        for principle in &self.config.principles {
            if !principle.active {
                continue;
            }

            let violation_score = self.check_principle_violation(content, principle).await?;
            weighted_sum += principle.weight;

            if violation_score > self.config.violation_threshold {
                let severity = match violation_score {
                    s if s > 0.9 => ViolationSeverity::Critical,
                    s if s > 0.7 => ViolationSeverity::High,
                    s if s > 0.5 => ViolationSeverity::Medium,
                    _ => ViolationSeverity::Low,
                };

                violations.push(GuardrailViolation {
                    principle_id: principle.id.clone(),
                    severity,
                    description: format!("Violation of principle: {}", principle.name),
                    suggestion: self.generate_suggestion(principle, violation_score),
                    confidence: violation_score,
                });
            } else {
                total_score += principle.weight * (1.0 - violation_score);
            }
        }

        let overall_score = if weighted_sum > 0.0 { total_score / weighted_sum } else { 1.0 };
        let passed = violations.is_empty() || (!self.config.strict_mode && overall_score > 0.7);

        let suggestions = if self.config.auto_correction {
            self.generate_corrections(&violations)
        } else {
            Vec::new()
        };

        Ok(ConstitutionalAssessment {
            passed,
            violations,
            overall_score,
            timestamp: Utc::now(),
            suggestions,
        })
    }

    async fn check_principle_violation(
        &self,
        content: &str,
        principle: &ConstitutionalPrinciple,
    ) -> LlmResult<f32> {
        match principle.id.as_str() {
            "safety" => self.check_safety_violation(content).await,
            "privacy" => self.check_privacy_violation(content).await,
            "accuracy" => self.check_accuracy_violation(content).await,
            "transparency" => self.check_transparency_violation(content).await,
            "fairness" => self.check_fairness_violation(content).await,
            "accountability" => self.check_accountability_violation(content).await,
            "robustness" => self.check_robustness_violation(content).await,
            _ => Ok(0.0),
        }
    }

    async fn check_safety_violation(&self, content: &str) -> LlmResult<f32> {
        let safety_patterns = vec![
            r"(?i)\b(dangerous|harmful|unsafe|risky|hazardous)\b",
            r"(?i)\b(exploit|vulnerability|attack|malicious)\b",
            r"(?i)\b(delete|destroy|corrupt|damage)\s+(?:all|everything|system)",
        ];

        let mut violation_score = 0.0f32;
        for pattern in safety_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.3;
            }
        }

        Ok(violation_score.min(1.0))
    }

    async fn check_privacy_violation(&self, content: &str) -> LlmResult<f32> {
        let privacy_patterns = vec![
            r"\b\d{3}-\d{2}-\d{4}\b", // SSN
            r"\b\d{4}\s?\d{4}\s?\d{4}\s?\d{4}\b", // Credit card
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", // Email
            r"\b\d{3}-\d{3}-\d{4}\b", // Phone
        ];

        let mut violation_score = 0.0f32;
        for pattern in privacy_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.4;
            }
        }

        Ok(violation_score.min(1.0))
    }

    async fn check_accuracy_violation(&self, content: &str) -> LlmResult<f32> {
        let inaccuracy_patterns = vec![
            r"(?i)\b(probably|maybe|might be|could be|not sure)\b",
            r"(?i)\b(unverified|unconfirmed|speculation|rumor)\b",
        ];

        let mut violation_score = 0.0f32;
        for pattern in inaccuracy_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.2;
            }
        }

        Ok(violation_score.min(1.0))
    }

    async fn check_transparency_violation(&self, content: &str) -> LlmResult<f32> {
        let opacity_patterns = vec![
            r"(?i)\b(hidden|secret|undisclosed|confidential)\b",
            r"(?i)\b(black box|opaque|mysterious)\b",
        ];

        let mut violation_score = 0.0f32;
        for pattern in opacity_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.25;
            }
        }

        Ok(violation_score.min(1.0))
    }

    async fn check_fairness_violation(&self, content: &str) -> LlmResult<f32> {
        let bias_patterns = vec![
            r"(?i)\b(discriminate|prejudice|stereotype|bias)\b",
            r"(?i)\b(unfair|unjust|inequitable)\b",
        ];

        let mut violation_score = 0.0f32;
        for pattern in bias_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.3;
            }
        }

        Ok(violation_score.min(1.0))
    }

    async fn check_accountability_violation(&self, content: &str) -> LlmResult<f32> {
        let unaccountable_patterns = vec![
            r"(?i)\b(not responsible|no accountability|blame others)\b",
            r"(?i)\b(deny|refuse|avoid) responsibility\b",
        ];

        let mut violation_score = 0.0f32;
        for pattern in unaccountable_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.35;
            }
        }

        Ok(violation_score.min(1.0))
    }

    async fn check_robustness_violation(&self, content: &str) -> LlmResult<f32> {
        let fragility_patterns = vec![
            r"(?i)\b(fragile|brittle|unstable|unreliable)\b",
            r"(?i)\b(fail|crash|break) easily\b",
        ];

        let mut violation_score = 0.0f32;
        for pattern in fragility_patterns {
            if regex::Regex::new(pattern)?.is_match(content) {
                violation_score += 0.25;
            }
        }

        Ok(violation_score.min(1.0))
    }

    fn generate_suggestion(&self, principle: &ConstitutionalPrinciple, score: f32) -> Option<String> {
        match principle.id.as_str() {
            "safety" => Some(format!("Consider rephrasing to avoid potentially harmful content (confidence: {:.2})", score)),
            "privacy" => Some(format!("Remove or anonymize personal information detected (confidence: {:.2})", score)),
            "accuracy" => Some(format!("Provide more concrete and verifiable information (confidence: {:.2})", score)),
            "transparency" => Some(format!("Be more explicit about methods and reasoning (confidence: {:.2})", score)),
            "fairness" => Some(format!("Review content for potential bias or discrimination (confidence: {:.2})", score)),
            "accountability" => Some(format!("Clarify responsibility and ownership (confidence: {:.2})", score)),
            "robustness" => Some(format!("Strengthen reliability and error handling (confidence: {:.2})", score)),
            _ => None,
        }
    }

    fn generate_corrections(&self, violations: &[GuardrailViolation]) -> Vec<String> {
        violations.iter()
            .filter_map(|v| v.suggestion.clone())
            .collect()
    }

    pub fn add_principle(&mut self, principle: ConstitutionalPrinciple) -> LlmResult<()> {
        self.principle_cache.insert(principle.id.clone(), principle.clone());
        self.config.principles.push(principle);
        Ok(())
    }

    pub fn update_principle(&mut self, principle: ConstitutionalPrinciple) -> LlmResult<()> {
        if let Some(pos) = self.config.principles.iter().position(|p| p.id == principle.id) {
            self.config.principles[pos] = principle.clone();
            self.principle_cache.insert(principle.id.clone(), principle);
            Ok(())
        } else {
            Err(LlmError::ConfigurationError(
                format!("Principle '{}' not found", principle.id)
            ))
        }
    }

    pub fn remove_principle(&mut self, principle_id: &str) -> LlmResult<()> {
        self.config.principles.retain(|p| p.id != principle_id);
        self.principle_cache.remove(principle_id);
        Ok(())
    }

    pub fn get_principle(&self, principle_id: &str) -> Option<&ConstitutionalPrinciple> {
        self.principle_cache.get(principle_id)
    }

    pub fn list_active_principles(&self) -> Vec<&ConstitutionalPrinciple> {
        self.config.principles
            .iter()
            .filter(|p| p.active)
            .collect()
    }
}

impl Default for ConstitutionalConfig {
    fn default() -> Self {
        Self {
            principles: vec![
                ConstitutionalPrinciple {
                    id: "safety".to_string(),
                    name: "Safety".to_string(),
                    description: "Ensure content is safe and non-harmful".to_string(),
                    weight: 1.0,
                    active: true,
                },
                ConstitutionalPrinciple {
                    id: "privacy".to_string(),
                    name: "Privacy".to_string(),
                    description: "Protect personal and sensitive information".to_string(),
                    weight: 0.9,
                    active: true,
                },
                ConstitutionalPrinciple {
                    id: "accuracy".to_string(),
                    name: "Accuracy".to_string(),
                    description: "Maintain factual correctness".to_string(),
                    weight: 0.8,
                    active: true,
                },
                ConstitutionalPrinciple {
                    id: "transparency".to_string(),
                    name: "Transparency".to_string(),
                    description: "Be clear about methods and limitations".to_string(),
                    weight: 0.7,
                    active: true,
                },
                ConstitutionalPrinciple {
                    id: "fairness".to_string(),
                    name: "Fairness".to_string(),
                    description: "Avoid bias and discrimination".to_string(),
                    weight: 0.8,
                    active: true,
                },
                ConstitutionalPrinciple {
                    id: "accountability".to_string(),
                    name: "Accountability".to_string(),
                    description: "Take responsibility for outputs".to_string(),
                    weight: 0.6,
                    active: true,
                },
                ConstitutionalPrinciple {
                    id: "robustness".to_string(),
                    name: "Robustness".to_string(),
                    description: "Ensure reliable and stable operation".to_string(),
                    weight: 0.7,
                    active: true,
                },
            ],
            strict_mode: false,
            auto_correction: true,
            violation_threshold: 0.4,
        }
    }
}