use std::sync::Arc;
use tracing::{info, error};
use tokio::task::JoinHandle;

use super::{
    metrics::MetricsCollector,
    telemetry::{TelemetryService, TelemetryConfig},
    health::{HealthCheck, HealthCheckConfig},
    dashboard::{DashboardServer, DashboardConfig},
    alerts::{AlertManager, AlertRule, AlertSeverity, AlertCondition, NotificationChannel},
};
use crate::errors::Result;

/// Complete monitoring integration
pub struct MonitoringIntegration {
    pub metrics: Arc<MetricsCollector>,
    pub telemetry: Arc<TelemetryService>,
    pub health_check: Arc<HealthCheck>,
    pub alert_manager: Arc<AlertManager>,
    dashboard_handle: Option<JoinHandle<()>>,
}

impl MonitoringIntegration {
    /// Initialize complete monitoring stack
    pub async fn new() -> Result<Self> {
        info!("🔧 Initializing monitoring integration...");
        
        // Initialize metrics collector
        let metrics = Arc::new(MetricsCollector::new()
            .map_err(|e| crate::errors::BotError::monitoring(format!("Failed to initialize metrics: {}", e)))?);
        info!("✅ Metrics collector initialized");
        
        // Initialize telemetry
        let telemetry_config = TelemetryConfig::default();
        let telemetry = Arc::new(TelemetryService::new(telemetry_config).await
            .map_err(|e| crate::errors::BotError::monitoring(format!("Failed to initialize telemetry: {}", e)))?);
        info!("✅ Telemetry service initialized");
        
        // Initialize health checks
        let health_check = Arc::new(HealthCheck::new("0.2.0".to_string()));
        Self::register_health_checks(&health_check).await;
        info!("✅ Health checks registered");
        
        // Initialize alert manager
        let alert_manager = Arc::new(AlertManager::new());
        alert_manager.initialize_default_rules().await;
        info!("✅ Alert manager initialized");
        
        Ok(Self {
            metrics,
            telemetry,
            health_check,
            alert_manager,
            dashboard_handle: None,
        })
    }
    
    /// Start monitoring services
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting monitoring services...");
        
        // Start periodic health checks
        self.health_check.start_periodic_checks(30).await;
        info!("✅ Periodic health checks started (30s interval)");
        
        // Start dashboard server
        let dashboard_config = DashboardConfig::default();
        let dashboard = DashboardServer::new(
            dashboard_config,
            Arc::clone(&self.metrics),
            Arc::clone(&self.health_check),
            Arc::clone(&self.telemetry),
        );
        
        let dashboard_handle = tokio::spawn(async move {
            if let Err(e) = dashboard.start().await {
                error!("Dashboard server error: {}", e);
            }
        });
        
        self.dashboard_handle = Some(dashboard_handle);
        info!("✅ Dashboard server started on http://127.0.0.1:3000");
        
        // Start telemetry cleanup task
        let telemetry_cleanup = Arc::clone(&self.telemetry);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;
                telemetry_cleanup.cleanup_old_spans(24).await; // Keep 24 hours
            }
        });
        info!("✅ Telemetry cleanup task started");
        
        info!("🎉 All monitoring services started successfully!");
        Ok(())
    }
    
    /// Register health check components
    async fn register_health_checks(health_check: &Arc<HealthCheck>) {
        // Database health check
        health_check.register_check(HealthCheckConfig {
            component: "database".to_string(),
            check_interval_seconds: 60,
            timeout_seconds: 10,
            retries: 3,
            critical: true,
        }).await;
        
        // Redis cache health check
        health_check.register_check(HealthCheckConfig {
            component: "redis_cache".to_string(),
            check_interval_seconds: 30,
            timeout_seconds: 5,
            retries: 2,
            critical: false,
        }).await;
        
        // Solana RPC health check
        health_check.register_check(HealthCheckConfig {
            component: "solana_rpc".to_string(),
            check_interval_seconds: 30,
            timeout_seconds: 10,
            retries: 3,
            critical: true,
        }).await;
        
        // Jupiter API health check
        health_check.register_check(HealthCheckConfig {
            component: "jupiter_api".to_string(),
            check_interval_seconds: 60,
            timeout_seconds: 15,
            retries: 2,
            critical: true,
        }).await;
        
        // Pump.fun API health check
        health_check.register_check(HealthCheckConfig {
            component: "pump_fun_api".to_string(),
            check_interval_seconds: 120,
            timeout_seconds: 10,
            retries: 2,
            critical: false,
        }).await;
        
        // Telegram bot health check
        health_check.register_check(HealthCheckConfig {
            component: "telegram_bot".to_string(),
            check_interval_seconds: 60,
            timeout_seconds: 10,
            retries: 3,
            critical: true,
        }).await;
        
        // Wallet manager health check
        health_check.register_check(HealthCheckConfig {
            component: "wallet_manager".to_string(),
            check_interval_seconds: 120,
            timeout_seconds: 5,
            retries: 1,
            critical: true,
        }).await;
        
        // Trading engine health check
        health_check.register_check(HealthCheckConfig {
            component: "trading_engine".to_string(),
            check_interval_seconds: 30,
            timeout_seconds: 5,
            retries: 2,
            critical: true,
        }).await;
        
        // MEV protection health check
        health_check.register_check(HealthCheckConfig {
            component: "mev_protection".to_string(),
            check_interval_seconds: 60,
            timeout_seconds: 10,
            retries: 2,
            critical: false,
        }).await;
        
        // AI analyzer health check
        health_check.register_check(HealthCheckConfig {
            component: "ai_analyzer".to_string(),
            check_interval_seconds: 300,
            timeout_seconds: 30,
            retries: 2,
            critical: false,
        }).await;
    }
    
    /// Record a trade in metrics
    pub fn record_trade(&self, token: &str, action: &str, user: &str, success: bool, volume_sol: f64, latency_ms: f64) {
        self.metrics.record_trade(token, action, user, success, volume_sol, latency_ms);
    }
    
    /// Record API call
    pub fn record_api_call(&self, endpoint: &str, method: &str, success: bool, latency_ms: f64) {
        self.metrics.record_api_call(endpoint, method, success, latency_ms);
    }
    
    /// Record error
    pub fn record_error(&self, error_type: &str, severity: &str, component: &str) {
        self.metrics.record_error(error_type, severity, component);
    }
    
    /// Start tracing span
    pub async fn start_span(&self, operation: &str) -> String {
        self.telemetry.start_span(operation).await
    }
    
    /// Finish tracing span
    pub async fn finish_span(&self, span_id: &str) {
        self.telemetry.finish_span(span_id).await;
    }
    
    /// Check metric against alerts
    pub async fn check_alert(&self, metric_name: &str, value: f64) {
        self.alert_manager.check_metric(metric_name, value, std::collections::HashMap::new()).await;
    }
    
    /// Get monitoring status
    pub async fn get_status(&self) -> MonitoringStatus {
        let metrics_summary = self.metrics.get_summary().await;
        let health = self.health_check.get_health().await;
        let telemetry_stats = self.telemetry.get_telemetry_stats().await;
        let active_alerts = self.alert_manager.get_active_alerts().await;
        
        MonitoringStatus {
            metrics_summary,
            health,
            telemetry_stats,
            active_alerts_count: active_alerts.len(),
            dashboard_running: self.dashboard_handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false),
        }
    }
}

/// Monitoring status summary
#[derive(Debug, Clone)]
pub struct MonitoringStatus {
    pub metrics_summary: crate::monitoring::metrics::MetricsSummary,
    pub health: crate::monitoring::health::SystemHealth,
    pub telemetry_stats: crate::monitoring::telemetry::TelemetryStats,
    pub active_alerts_count: usize,
    pub dashboard_running: bool,
}

/// Helper macros for convenient monitoring
#[macro_export]
macro_rules! monitor_trade {
    ($monitoring:expr, $token:expr, $action:expr, $user:expr, $result:expr) => {
        match $result {
            Ok(volume) => {
                $monitoring.record_trade($token, $action, $user, true, volume, 0.0);
            }
            Err(_) => {
                $monitoring.record_trade($token, $action, $user, false, 0.0, 0.0);
            }
        }
    };
}

#[macro_export]
macro_rules! monitor_api {
    ($monitoring:expr, $endpoint:expr, $method:expr, $result:expr) => {
        match $result {
            Ok(_) => {
                $monitoring.record_api_call($endpoint, $method, true, 0.0);
            }
            Err(_) => {
                $monitoring.record_api_call($endpoint, $method, false, 0.0);
                $monitoring.record_error("api_error", "warning", "api");
            }
        }
    };
}

#[macro_export]
macro_rules! trace_async {
    ($monitoring:expr, $operation:expr, $block:block) => {{
        let span_id = $monitoring.start_span($operation).await;
        let result = $block;
        $monitoring.finish_span(&span_id).await;
        result
    }};
}