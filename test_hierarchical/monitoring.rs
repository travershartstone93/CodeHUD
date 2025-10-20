use crate::{LlmError, LlmResult};
use crate::ffi::PythonLlmBridge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use tokio::time::{interval, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub response_time_ms: u64,
    pub token_throughput: f32,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub gpu_utilization_percent: Option<f32>,
    pub queue_depth: usize,
    pub concurrent_requests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub accuracy_score: f32,
    pub relevance_score: f32,
    pub coherence_score: f32,
    pub completion_rate: f32,
    pub error_rate: f32,
    pub user_satisfaction: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_tokens_per_request: f32,
    pub peak_concurrent_users: usize,
    pub bandwidth_usage_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service_name: String,
    pub status: ServiceStatus,
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub version: String,
    pub dependencies: Vec<DependencyHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealth {
    pub name: String,
    pub status: ServiceStatus,
    pub response_time_ms: Option<u64>,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub resolved: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_interval_seconds: u64,
    pub health_check_interval_seconds: u64,
    pub alert_thresholds: AlertThresholds,
    pub retention_days: u32,
    pub enable_detailed_metrics: bool,
    pub enable_performance_profiling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub max_response_time_ms: u64,
    pub max_error_rate: f32,
    pub min_accuracy_score: f32,
    pub max_memory_usage_mb: u64,
    pub max_cpu_usage_percent: f32,
    pub max_queue_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub performance: PerformanceMetrics,
    pub quality: QualityMetrics,
    pub usage: UsageMetrics,
    pub health: HealthStatus,
    pub active_alerts: Vec<Alert>,
}

pub struct LlmMonitor {
    config: MonitoringConfig,
    metrics_history: Vec<SystemSnapshot>,
    active_alerts: HashMap<String, Alert>,
    start_time: Instant,
    python_bridge: Option<PythonLlmBridge>,
    request_counter: u64,
    error_counter: u64,
}

impl LlmMonitor {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics_history: Vec::new(),
            active_alerts: HashMap::new(),
            start_time: Instant::now(),
            python_bridge: None,
            request_counter: 0,
            error_counter: 0,
        }
    }

    pub fn with_python_bridge(mut self, bridge: PythonLlmBridge) -> Self {
        self.python_bridge = Some(bridge);
        self
    }

    pub async fn start_monitoring(&mut self) -> LlmResult<()> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.start_monitoring(&self.config).await;
        }
        self.start_monitoring_native().await
    }

    async fn start_monitoring_native(&mut self) -> LlmResult<()> {
        let mut metrics_interval = interval(
            tokio::time::Duration::from_secs(self.config.metrics_interval_seconds)
        );
        let mut health_interval = interval(
            tokio::time::Duration::from_secs(self.config.health_check_interval_seconds)
        );

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = metrics_interval.tick() => {
                        // Collect metrics
                    }
                    _ = health_interval.tick() => {
                        // Run health checks
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn record_request(&mut self, duration_ms: u64, success: bool) -> LlmResult<()> {
        self.request_counter += 1;
        if !success {
            self.error_counter += 1;
        }

        self.check_performance_thresholds(duration_ms).await?;
        self.check_error_rate_thresholds().await?;

        Ok(())
    }

    pub async fn collect_metrics(&mut self) -> LlmResult<SystemSnapshot> {
        if let Some(ref bridge) = self.python_bridge {
            return bridge.collect_metrics().await;
        }
        self.collect_metrics_native().await
    }

    async fn collect_metrics_native(&mut self) -> LlmResult<SystemSnapshot> {
        let performance = self.collect_performance_metrics().await?;
        let quality = self.collect_quality_metrics().await?;
        let usage = self.collect_usage_metrics().await?;
        let health = self.collect_health_status().await?;
        let active_alerts: Vec<Alert> = self.active_alerts.values().cloned().collect();

        let snapshot = SystemSnapshot {
            timestamp: Utc::now(),
            performance,
            quality,
            usage,
            health,
            active_alerts,
        };

        self.metrics_history.push(snapshot.clone());
        self.cleanup_old_metrics().await?;

        Ok(snapshot)
    }

    async fn collect_performance_metrics(&self) -> LlmResult<PerformanceMetrics> {
        let memory_info = self.get_memory_usage().await?;
        let cpu_usage = self.get_cpu_usage().await?;

        Ok(PerformanceMetrics {
            response_time_ms: self.calculate_avg_response_time().await?,
            token_throughput: self.calculate_token_throughput().await?,
            memory_usage_mb: memory_info,
            cpu_usage_percent: cpu_usage,
            gpu_utilization_percent: self.get_gpu_utilization().await.ok(),
            queue_depth: 0, // Would be implemented based on actual queue
            concurrent_requests: 0, // Would track active requests
        })
    }

    async fn collect_quality_metrics(&self) -> LlmResult<QualityMetrics> {
        let error_rate = if self.request_counter > 0 {
            self.error_counter as f32 / self.request_counter as f32
        } else {
            0.0
        };

        Ok(QualityMetrics {
            accuracy_score: 0.95, // Would be calculated from validation results
            relevance_score: 0.92, // Would be calculated from user feedback
            coherence_score: 0.89, // Would be calculated from content analysis
            completion_rate: 1.0 - error_rate,
            error_rate,
            user_satisfaction: None, // Would be collected from user feedback
        })
    }

    async fn collect_usage_metrics(&self) -> LlmResult<UsageMetrics> {
        Ok(UsageMetrics {
            total_requests: self.request_counter,
            successful_requests: self.request_counter - self.error_counter,
            failed_requests: self.error_counter,
            avg_tokens_per_request: 150.0, // Would be calculated from actual usage
            peak_concurrent_users: 1, // Would track maximum concurrent users
            bandwidth_usage_mb: 0.0, // Would track actual bandwidth
        })
    }

    async fn collect_health_status(&self) -> LlmResult<HealthStatus> {
        let uptime = self.start_time.elapsed().as_secs();
        let status = self.determine_service_status().await?;

        let dependencies = vec![
            DependencyHealth {
                name: "Ollama".to_string(),
                status: ServiceStatus::Healthy,
                response_time_ms: Some(50),
                last_check: Utc::now(),
            },
            DependencyHealth {
                name: "GPU Driver".to_string(),
                status: ServiceStatus::Healthy,
                response_time_ms: None,
                last_check: Utc::now(),
            },
        ];

        Ok(HealthStatus {
            service_name: "CodeHUD LLM".to_string(),
            status,
            timestamp: Utc::now(),
            uptime_seconds: uptime,
            version: "0.1.0".to_string(),
            dependencies,
        })
    }

    async fn determine_service_status(&self) -> LlmResult<ServiceStatus> {
        let error_rate = if self.request_counter > 0 {
            self.error_counter as f32 / self.request_counter as f32
        } else {
            0.0
        };

        if error_rate > 0.5 {
            Ok(ServiceStatus::Unhealthy)
        } else if error_rate > 0.1 {
            Ok(ServiceStatus::Degraded)
        } else {
            Ok(ServiceStatus::Healthy)
        }
    }

    async fn check_performance_thresholds(&mut self, response_time_ms: u64) -> LlmResult<()> {
        if response_time_ms > self.config.alert_thresholds.max_response_time_ms {
            self.create_alert(
                AlertSeverity::Warning,
                "High Response Time".to_string(),
                format!("Response time {}ms exceeds threshold {}ms",
                    response_time_ms, self.config.alert_thresholds.max_response_time_ms),
                "performance".to_string(),
            ).await?;
        }

        Ok(())
    }

    async fn check_error_rate_thresholds(&mut self) -> LlmResult<()> {
        let error_rate = if self.request_counter > 0 {
            self.error_counter as f32 / self.request_counter as f32
        } else {
            0.0
        };

        if error_rate > self.config.alert_thresholds.max_error_rate {
            self.create_alert(
                AlertSeverity::Critical,
                "High Error Rate".to_string(),
                format!("Error rate {:.2}% exceeds threshold {:.2}%",
                    error_rate * 100.0, self.config.alert_thresholds.max_error_rate * 100.0),
                "errors".to_string(),
            ).await?;
        }

        Ok(())
    }

    async fn create_alert(
        &mut self,
        severity: AlertSeverity,
        title: String,
        message: String,
        source: String,
    ) -> LlmResult<String> {
        let alert_id = uuid::Uuid::new_v4().to_string();
        let alert = Alert {
            id: alert_id.clone(),
            severity,
            title,
            message,
            timestamp: Utc::now(),
            source,
            resolved: false,
            metadata: HashMap::new(),
        };

        self.active_alerts.insert(alert_id.clone(), alert);
        Ok(alert_id)
    }

    pub async fn resolve_alert(&mut self, alert_id: &str) -> LlmResult<()> {
        if let Some(alert) = self.active_alerts.get_mut(alert_id) {
            alert.resolved = true;
        }
        Ok(())
    }

    pub async fn get_active_alerts(&self) -> Vec<&Alert> {
        self.active_alerts.values().filter(|a| !a.resolved).collect()
    }

    pub async fn get_metrics_history(&self, hours: u32) -> LlmResult<Vec<&SystemSnapshot>> {
        let cutoff = Utc::now() - Duration::hours(hours as i64);
        Ok(self.metrics_history
            .iter()
            .filter(|snapshot| snapshot.timestamp > cutoff)
            .collect())
    }

    async fn cleanup_old_metrics(&mut self) -> LlmResult<()> {
        let cutoff = Utc::now() - Duration::days(self.config.retention_days as i64);
        self.metrics_history.retain(|snapshot| snapshot.timestamp > cutoff);

        self.active_alerts.retain(|_, alert| {
            !alert.resolved || (Utc::now() - alert.timestamp) < Duration::days(7)
        });

        Ok(())
    }

    async fn get_memory_usage(&self) -> LlmResult<u64> {
        // Simplified memory usage calculation
        Ok(100) // Would use actual system metrics
    }

    async fn get_cpu_usage(&self) -> LlmResult<f32> {
        // Simplified CPU usage calculation
        Ok(15.0) // Would use actual system metrics
    }

    async fn get_gpu_utilization(&self) -> LlmResult<f32> {
        // Would query actual GPU metrics
        Ok(45.0)
    }

    async fn calculate_avg_response_time(&self) -> LlmResult<u64> {
        // Would calculate from actual request history
        Ok(250)
    }

    async fn calculate_token_throughput(&self) -> LlmResult<f32> {
        // Would calculate tokens per second
        Ok(85.5)
    }

    pub fn get_dashboard_data(&self) -> LlmResult<HashMap<String, serde_json::Value>> {
        let mut dashboard = HashMap::new();

        if let Some(latest_snapshot) = self.metrics_history.last() {
            dashboard.insert("current_metrics".to_string(),
                serde_json::to_value(latest_snapshot)?);
        }

        dashboard.insert("active_alert_count".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from(self.active_alerts.len())
            ));

        dashboard.insert("total_requests".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from(self.request_counter)
            ));

        dashboard.insert("uptime_seconds".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from(self.start_time.elapsed().as_secs())
            ));

        Ok(dashboard)
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_interval_seconds: 60,
            health_check_interval_seconds: 30,
            alert_thresholds: AlertThresholds::default(),
            retention_days: 30,
            enable_detailed_metrics: true,
            enable_performance_profiling: false,
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_response_time_ms: 5000,
            max_error_rate: 0.05,
            min_accuracy_score: 0.9,
            max_memory_usage_mb: 2048,
            max_cpu_usage_percent: 80.0,
            max_queue_depth: 100,
        }
    }
}