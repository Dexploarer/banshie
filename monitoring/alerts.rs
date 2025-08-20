use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, warn, error};

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub metric: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub cooldown_minutes: u32,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    ChangePercent,
    RateOfChange,
}

/// Notification channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Console,
    Telegram { chat_id: String, bot_token: String },
    Discord { webhook_url: String },
    Email { recipients: Vec<String> },
    Slack { webhook_url: String },
    Custom { endpoint: String, headers: HashMap<String, String> },
}

/// Triggered alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub metric_value: f64,
    pub threshold: f64,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

/// Alert manager
pub struct AlertManager {
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<Vec<Alert>>>,
    cooldowns: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl AlertManager {
    /// Create new alert manager
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add alert rule
    pub async fn add_rule(&self, rule: AlertRule) {
        info!("Adding alert rule: {} ({})", rule.name, rule.id);
        
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
    }
    
    /// Remove alert rule
    pub async fn remove_rule(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write().await;
        rules.remove(rule_id).is_some()
    }
    
    /// Enable/disable rule
    pub async fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> bool {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.get_mut(rule_id) {
            rule.enabled = enabled;
            info!("Rule {} {} {}", rule_id, if enabled { "enabled" } else { "disabled" }, rule.name);
            return true;
        }
        false
    }
    
    /// Check metric against all rules
    pub async fn check_metric(&self, metric_name: &str, value: f64, metadata: HashMap<String, String>) {
        let rules = self.rules.read().await.clone();
        
        for rule in rules.values() {
            if rule.enabled && rule.metric == metric_name {
                self.evaluate_rule(rule, value, &metadata).await;
            }
        }
    }
    
    /// Evaluate a specific rule
    async fn evaluate_rule(&self, rule: &AlertRule, value: f64, metadata: &HashMap<String, String>) {
        // Check if rule is in cooldown
        {
            let cooldowns = self.cooldowns.read().await;
            if let Some(cooldown_until) = cooldowns.get(&rule.id) {
                if Utc::now() < *cooldown_until {
                    return;
                }
            }
        }
        
        let triggered = match rule.condition {
            AlertCondition::GreaterThan => value > rule.threshold,
            AlertCondition::LessThan => value < rule.threshold,
            AlertCondition::Equal => (value - rule.threshold).abs() < f64::EPSILON,
            AlertCondition::NotEqual => (value - rule.threshold).abs() > f64::EPSILON,
            AlertCondition::ChangePercent => {
                // Would need historical data to calculate
                false
            }
            AlertCondition::RateOfChange => {
                // Would need time-series data to calculate
                false
            }
        };
        
        if triggered {
            self.trigger_alert(rule, value, metadata.clone()).await;
        } else {
            // Check if we should resolve an existing alert
            self.maybe_resolve_alert(&rule.id).await;
        }
    }
    
    /// Trigger an alert
    async fn trigger_alert(&self, rule: &AlertRule, value: f64, metadata: HashMap<String, String>) {
        let alert_id = format!("{}_{}", rule.id, Utc::now().timestamp());
        
        let alert = Alert {
            id: alert_id.clone(),
            rule_id: rule.id.clone(),
            title: format!("Alert: {}", rule.name),
            description: format!(
                "{}\nMetric: {} = {}\nThreshold: {} {}",
                rule.description,
                rule.metric,
                value,
                match rule.condition {
                    AlertCondition::GreaterThan => ">",
                    AlertCondition::LessThan => "<",
                    AlertCondition::Equal => "=",
                    AlertCondition::NotEqual => "!=",
                    AlertCondition::ChangePercent => "% change >",
                    AlertCondition::RateOfChange => "rate >",
                },
                rule.threshold
            ),
            severity: rule.severity.clone(),
            metric_value: value,
            threshold: rule.threshold,
            triggered_at: Utc::now(),
            resolved_at: None,
            metadata,
        };
        
        // Add to active alerts
        {
            let mut active_alerts = self.active_alerts.write().await;
            active_alerts.insert(alert_id.clone(), alert.clone());
        }
        
        // Add to history
        {
            let mut history = self.alert_history.write().await;
            history.push(alert.clone());
            
            // Keep only last 1000 alerts
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }
        
        // Set cooldown
        {
            let mut cooldowns = self.cooldowns.write().await;
            let cooldown_until = Utc::now() + Duration::minutes(rule.cooldown_minutes as i64);
            cooldowns.insert(rule.id.clone(), cooldown_until);
        }
        
        // Send notifications
        self.send_notifications(&alert, &rule.notification_channels).await;
        
        match alert.severity {
            AlertSeverity::Emergency => error!("🚨 EMERGENCY ALERT: {}", alert.title),
            AlertSeverity::Critical => error!("❌ CRITICAL ALERT: {}", alert.title),
            AlertSeverity::Warning => warn!("⚠️ WARNING ALERT: {}", alert.title),
            AlertSeverity::Info => info!("ℹ️ INFO ALERT: {}", alert.title),
        }
    }
    
    /// Resolve an alert
    async fn maybe_resolve_alert(&self, rule_id: &str) {
        let mut active_alerts = self.active_alerts.write().await;
        
        // Find active alert for this rule
        let mut to_resolve = None;
        for (alert_id, alert) in active_alerts.iter() {
            if alert.rule_id == rule_id {
                to_resolve = Some(alert_id.clone());
                break;
            }
        }
        
        if let Some(alert_id) = to_resolve {
            if let Some(mut alert) = active_alerts.remove(&alert_id) {
                alert.resolved_at = Some(Utc::now());
                
                info!("✅ RESOLVED: {}", alert.title);
                
                // Update in history
                let mut history = self.alert_history.write().await;
                if let Some(historical_alert) = history.iter_mut().find(|a| a.id == alert_id) {
                    historical_alert.resolved_at = alert.resolved_at;
                }
            }
        }
    }
    
    /// Send notifications for an alert
    async fn send_notifications(&self, alert: &Alert, channels: &[NotificationChannel]) {
        for channel in channels {
            match channel {
                NotificationChannel::Console => {
                    println!("🚨 ALERT: {} | {} | Value: {} | Threshold: {}", 
                        alert.title, alert.description, alert.metric_value, alert.threshold);
                }
                NotificationChannel::Telegram { chat_id, bot_token } => {
                    self.send_telegram_notification(alert, chat_id, bot_token).await;
                }
                NotificationChannel::Discord { webhook_url } => {
                    self.send_discord_notification(alert, webhook_url).await;
                }
                NotificationChannel::Email { recipients } => {
                    self.send_email_notification(alert, recipients).await;
                }
                NotificationChannel::Slack { webhook_url } => {
                    self.send_slack_notification(alert, webhook_url).await;
                }
                NotificationChannel::Custom { endpoint, headers } => {
                    self.send_custom_notification(alert, endpoint, headers).await;
                }
            }
        }
    }
    
    /// Send Telegram notification
    async fn send_telegram_notification(&self, alert: &Alert, chat_id: &str, bot_token: &str) {
        let emoji = match alert.severity {
            AlertSeverity::Emergency => "🚨",
            AlertSeverity::Critical => "❌",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Info => "ℹ️",
        };
        
        let message = format!(
            "{} *{}*\n\n{}\n\n📊 Value: {}\n🎯 Threshold: {}\n⏰ Time: {}",
            emoji,
            alert.title.replace(".", "\\.").replace("-", "\\-"),
            alert.description.replace(".", "\\.").replace("-", "\\-"),
            alert.metric_value,
            alert.threshold,
            alert.triggered_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "MarkdownV2"
        });
        
        let client = reqwest::Client::new();
        if let Err(e) = client.post(&url).json(&payload).send().await {
            error!("Failed to send Telegram alert: {}", e);
        }
    }
    
    /// Send Discord notification
    async fn send_discord_notification(&self, alert: &Alert, webhook_url: &str) {
        let color = match alert.severity {
            AlertSeverity::Emergency => 0xFF0000, // Red
            AlertSeverity::Critical => 0xFF4500,  // Orange Red
            AlertSeverity::Warning => 0xFFA500,   // Orange
            AlertSeverity::Info => 0x0099FF,      // Blue
        };
        
        let payload = serde_json::json!({
            "embeds": [{
                "title": alert.title,
                "description": alert.description,
                "color": color,
                "fields": [
                    {
                        "name": "Metric Value",
                        "value": alert.metric_value.to_string(),
                        "inline": true
                    },
                    {
                        "name": "Threshold",
                        "value": alert.threshold.to_string(),
                        "inline": true
                    },
                    {
                        "name": "Severity",
                        "value": format!("{:?}", alert.severity),
                        "inline": true
                    }
                ],
                "timestamp": alert.triggered_at.to_rfc3339()
            }]
        });
        
        let client = reqwest::Client::new();
        if let Err(e) = client.post(webhook_url).json(&payload).send().await {
            error!("Failed to send Discord alert: {}", e);
        }
    }
    
    /// Send email notification (placeholder)
    async fn send_email_notification(&self, alert: &Alert, _recipients: &[String]) {
        // Would implement actual email sending
        info!("Email alert would be sent: {}", alert.title);
    }
    
    /// Send Slack notification
    async fn send_slack_notification(&self, alert: &Alert, webhook_url: &str) {
        let color = match alert.severity {
            AlertSeverity::Emergency => "danger",
            AlertSeverity::Critical => "danger",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Info => "good",
        };
        
        let payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": alert.title,
                "text": alert.description,
                "fields": [
                    {
                        "title": "Value",
                        "value": alert.metric_value.to_string(),
                        "short": true
                    },
                    {
                        "title": "Threshold",
                        "value": alert.threshold.to_string(),
                        "short": true
                    }
                ],
                "ts": alert.triggered_at.timestamp()
            }]
        });
        
        let client = reqwest::Client::new();
        if let Err(e) = client.post(webhook_url).json(&payload).send().await {
            error!("Failed to send Slack alert: {}", e);
        }
    }
    
    /// Send custom notification
    async fn send_custom_notification(&self, alert: &Alert, endpoint: &str, headers: &HashMap<String, String>) {
        let client = reqwest::Client::new();
        let mut request = client.post(endpoint).json(alert);
        
        for (key, value) in headers {
            request = request.header(key, value);
        }
        
        if let Err(e) = request.send().await {
            error!("Failed to send custom alert to {}: {}", endpoint, e);
        }
    }
    
    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values().cloned().collect()
    }
    
    /// Get alert history
    pub async fn get_alert_history(&self, limit: Option<usize>) -> Vec<Alert> {
        let history = self.alert_history.read().await;
        let mut alerts: Vec<Alert> = history.iter().rev().cloned().collect();
        
        if let Some(limit) = limit {
            alerts.truncate(limit);
        }
        
        alerts
    }
    
    /// Get alert rules
    pub async fn get_rules(&self) -> Vec<AlertRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }
    
    /// Initialize default alert rules
    pub async fn initialize_default_rules(&self) {
        // Trading failure rate alert
        self.add_rule(AlertRule {
            id: "trading_failure_rate".to_string(),
            name: "High Trading Failure Rate".to_string(),
            description: "Trading failure rate is above threshold".to_string(),
            metric: "trading_failure_rate".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 0.1, // 10%
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown_minutes: 15,
            notification_channels: vec![
                NotificationChannel::Console,
            ],
        }).await;
        
        // High error rate alert
        self.add_rule(AlertRule {
            id: "error_rate_high".to_string(),
            name: "High Error Rate".to_string(),
            description: "System error rate is critically high".to_string(),
            metric: "error_rate".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 0.05, // 5%
            severity: AlertSeverity::Critical,
            enabled: true,
            cooldown_minutes: 5,
            notification_channels: vec![
                NotificationChannel::Console,
            ],
        }).await;
        
        // Low wallet balance alert
        self.add_rule(AlertRule {
            id: "wallet_balance_low".to_string(),
            name: "Low Wallet Balance".to_string(),
            description: "Main wallet balance is critically low".to_string(),
            metric: "wallet_balance_sol".to_string(),
            condition: AlertCondition::LessThan,
            threshold: 0.1, // 0.1 SOL
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown_minutes: 60,
            notification_channels: vec![
                NotificationChannel::Console,
            ],
        }).await;
        
        // MEV protection failure alert
        self.add_rule(AlertRule {
            id: "mev_protection_failure".to_string(),
            name: "MEV Protection Failure".to_string(),
            description: "MEV protection success rate is below threshold".to_string(),
            metric: "mev_success_rate".to_string(),
            condition: AlertCondition::LessThan,
            threshold: 0.8, // 80%
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown_minutes: 30,
            notification_channels: vec![
                NotificationChannel::Console,
            ],
        }).await;
        
        info!("Initialized {} default alert rules", 4);
    }
}