use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, debug, warn, error};

use crate::errors::{BotError, Result};
use crate::websocket::{PriceStreamManager, PriceUpdate};
use crate::telemetry::TelemetryService;
use crate::db::Database;

/// Comprehensive price alert management system
#[derive(Clone)]
pub struct PriceAlertManager {
    database: Arc<Database>,
    telemetry: Option<Arc<TelemetryService>>,
    price_stream: Arc<PriceStreamManager>,
    active_alerts: Arc<RwLock<HashMap<String, PriceAlert>>>,
    alert_history: Arc<RwLock<VecDeque<AlertHistory>>>,
    alert_stats: Arc<RwLock<AlertStatistics>>,
    delivery_channels: Arc<RwLock<HashMap<String, Arc<dyn AlertDeliveryChannel>>>>,
    alert_queue: Arc<RwLock<mpsc::UnboundedSender<TriggeredAlert>>>,
}

/// Price alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAlert {
    pub alert_id: String,
    pub user_id: i64,
    pub name: String,
    pub symbol: String,
    pub conditions: Vec<AlertCondition>,
    pub trigger_type: AlertTriggerType,
    pub priority: AlertPriority,
    pub actions: Vec<AlertAction>,
    pub delivery_methods: Vec<AlertDeliveryMethod>,
    pub cooldown_period: Option<Duration>,
    pub expiry_time: Option<DateTime<Utc>>,
    pub max_triggers: Option<u32>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub trigger_count: u32,
    pub status: AlertStatus,
    pub metadata: HashMap<String, String>,
}

/// Alert conditions that trigger notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    PriceThreshold(PriceThreshold),
    PercentageChange(PercentageChange),
    MovingAverage(MovingAverageCondition),
    Volume(VolumeCondition),
    TechnicalIndicator(TechnicalIndicatorAlert),
    CrossAsset(CrossAssetCondition),
    TimeBasedPrice(TimeBasedPriceCondition),
    Custom(CustomCondition),
}

/// Price threshold condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceThreshold {
    pub comparison: PriceComparison,
    pub target_price: Decimal,
    pub tolerance: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceComparison {
    Above,
    Below,
    Equals,
    CrossingAbove,
    CrossingBelow,
    Between(Decimal, Decimal),
    Outside(Decimal, Decimal),
}

/// Percentage change condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentageChange {
    pub timeframe: ChangeTimeframe,
    pub change_type: ChangeType,
    pub threshold_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeTimeframe {
    Minutes(u32),
    Hours(u32),
    Days(u32),
    SinceOpen,
    Custom(Duration),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Increase,
    Decrease,
    AbsoluteChange,
}

/// Moving average condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovingAverageCondition {
    pub ma_type: MovingAverageType,
    pub period: u32,
    pub comparison: MAComparison,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MovingAverageType {
    Simple,
    Exponential,
    Weighted,
    Hull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MAComparison {
    PriceAboveMA,
    PriceBelowMA,
    PriceCrossingMA,
    MACrossover { fast_period: u32, slow_period: u32 },
}

/// Volume condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeCondition {
    pub volume_type: VolumeType,
    pub threshold: u64,
    pub timeframe: VolumeTimeframe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeType {
    TotalVolume,
    BuyVolume,
    SellVolume,
    VolumeSpike,
    UnusualVolume { deviation_multiplier: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeTimeframe {
    Minute,
    Hour,
    Day,
    Rolling(Duration),
}

/// Technical indicator alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicatorAlert {
    pub indicator: TechnicalIndicator,
    pub condition: IndicatorCondition,
    pub parameters: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnicalIndicator {
    RSI,
    MACD,
    BollingerBands,
    StochasticOscillator,
    ATR,
    Ichimoku,
    Fibonacci,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndicatorCondition {
    Above(f64),
    Below(f64),
    Between(f64, f64),
    Crossing(f64),
    Divergence,
    Signal,
}

/// Cross-asset condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossAssetCondition {
    pub reference_symbol: String,
    pub comparison: CrossAssetComparison,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossAssetComparison {
    Correlation,
    SpreadAbove,
    SpreadBelow,
    RatioAbove,
    RatioBelow,
}

/// Time-based price condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedPriceCondition {
    pub time_window: TimeWindow,
    pub price_action: PriceAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub days: Vec<chrono::Weekday>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceAction {
    HighestPrice,
    LowestPrice,
    OpeningPrice,
    ClosingPrice,
}

/// Custom condition with expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCondition {
    pub expression: String,
    pub variables: HashMap<String, String>,
}

/// Alert trigger types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertTriggerType {
    Once,           // Trigger only once
    Repeating,      // Can trigger multiple times
    Continuous,     // Trigger continuously while condition is met
    Scheduled,      // Trigger on schedule
}

/// Alert priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertPriority {
    Low,
    Medium,
    High,
    Critical,
    Emergency,
}

/// Actions to take when alert triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertAction {
    Notify,
    ExecuteTrade { order_type: String, amount: Decimal },
    PauseStrategy { strategy_id: String },
    AdjustPosition { action: String, percentage: f64 },
    RunScript { script_path: String },
    WebhookCall { url: String, payload: String },
    LogEvent { level: String },
}

/// Alert delivery methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertDeliveryMethod {
    Telegram { chat_id: i64 },
    Email { address: String },
    SMS { phone_number: String },
    Push { device_token: String },
    Webhook { url: String },
    InApp,
    Discord { webhook_url: String },
    Slack { webhook_url: String },
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    Active,
    Triggered,
    Paused,
    Expired,
    Disabled,
    Error(String),
}

/// Alert history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistory {
    pub alert_id: String,
    pub triggered_at: DateTime<Utc>,
    pub trigger_price: Decimal,
    pub condition_met: String,
    pub actions_taken: Vec<String>,
    pub delivery_status: HashMap<String, DeliveryStatus>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Sent,
    Failed(String),
    Pending,
}

/// Alert statistics
#[derive(Debug, Clone, Default)]
pub struct AlertStatistics {
    pub total_alerts_created: u64,
    pub active_alerts: u64,
    pub total_triggers: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub average_response_time: Duration,
    pub alerts_by_symbol: HashMap<String, u64>,
    pub alerts_by_priority: HashMap<AlertPriority, u64>,
}

/// Triggered alert for processing
#[derive(Debug, Clone)]
struct TriggeredAlert {
    pub alert: PriceAlert,
    pub trigger_price: Decimal,
    pub condition_details: String,
    pub timestamp: DateTime<Utc>,
}

/// Alert delivery channel trait
#[async_trait::async_trait]
pub trait AlertDeliveryChannel: Send + Sync {
    async fn deliver(&self, alert: &TriggeredAlert, message: String) -> Result<()>;
    fn channel_name(&self) -> String;
}

impl PriceAlertManager {
    /// Create new price alert manager
    pub fn new(
        database: Arc<Database>,
        price_stream: Arc<PriceStreamManager>,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("🔔 Initializing price alert manager");
        
        let (tx, rx) = mpsc::unbounded_channel();
        
        let manager = Self {
            database,
            telemetry,
            price_stream,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            alert_stats: Arc::new(RwLock::new(AlertStatistics::default())),
            delivery_channels: Arc::new(RwLock::new(HashMap::new())),
            alert_queue: Arc::new(RwLock::new(tx)),
        };
        
        // Start alert processor
        let processor = manager.clone();
        tokio::spawn(async move {
            processor.process_alert_queue(rx).await;
        });
        
        manager
    }
    
    /// Start monitoring for alerts
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🔔 Starting price alert monitoring");
        
        // Load active alerts from database
        self.load_active_alerts().await?;
        
        // Subscribe to price updates for all monitored symbols
        let symbols = self.get_monitored_symbols().await;
        
        for symbol in symbols {
            self.monitor_symbol(&symbol).await?;
        }
        
        Ok(())
    }
    
    /// Create a new price alert
    pub async fn create_alert(&self, mut alert: PriceAlert) -> Result<String> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_span("create_price_alert")
        );
        
        // Validate alert
        self.validate_alert(&alert)?;
        
        // Set defaults
        alert.alert_id = uuid::Uuid::new_v4().to_string();
        alert.created_at = Utc::now();
        alert.trigger_count = 0;
        alert.status = AlertStatus::Active;
        
        // Store in database
        self.store_alert(&alert).await?;
        
        // Add to active alerts
        let mut alerts = self.active_alerts.write().await;
        alerts.insert(alert.alert_id.clone(), alert.clone());
        
        // Update statistics
        let mut stats = self.alert_stats.write().await;
        stats.total_alerts_created += 1;
        stats.active_alerts += 1;
        *stats.alerts_by_symbol.entry(alert.symbol.clone()).or_insert(0) += 1;
        
        // Start monitoring if not already
        self.monitor_symbol(&alert.symbol).await?;
        
        info!("🔔 Created alert: {} for {}", alert.alert_id, alert.symbol);
        
        Ok(alert.alert_id)
    }
    
    /// Monitor a symbol for alerts
    async fn monitor_symbol(&self, symbol: &str) -> Result<()> {
        let manager = self.clone();
        let symbol = symbol.to_string();
        
        // Subscribe to price updates
        let subscription = crate::websocket::PriceSubscription {
            symbols: vec![symbol.clone()],
            sources: vec![crate::websocket::PriceSource::Aggregate],
            include_orderbook: false,
            orderbook_depth: 0,
            include_trades: true,
            aggregation_interval: Some(std::time::Duration::from_secs(1)),
        };
        
        let mut price_receiver = self.price_stream.subscribe_prices(subscription).await?;
        
        // Spawn monitoring task
        tokio::spawn(async move {
            while let Ok(price_update) = price_receiver.recv().await {
                if let Err(e) = manager.check_alerts_for_price(&price_update).await {
                    error!("🔔 Error checking alerts: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Check alerts for a price update
    async fn check_alerts_for_price(&self, price_update: &PriceUpdate) -> Result<()> {
        let alerts = self.active_alerts.read().await;
        
        for alert in alerts.values() {
            if alert.symbol != price_update.symbol || !alert.enabled {
                continue;
            }
            
            // Check if alert is in cooldown
            if let Some(last_triggered) = alert.last_triggered {
                if let Some(cooldown) = alert.cooldown_period {
                    if Utc::now() - last_triggered < cooldown {
                        continue;
                    }
                }
            }
            
            // Check if alert has expired
            if let Some(expiry) = alert.expiry_time {
                if Utc::now() > expiry {
                    continue;
                }
            }
            
            // Check conditions
            for condition in &alert.conditions {
                if self.check_condition(condition, price_update, alert).await? {
                    self.trigger_alert(alert.clone(), price_update.price, condition.to_string()).await?;
                    break; // Only trigger once per check
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a condition is met
    async fn check_condition(
        &self,
        condition: &AlertCondition,
        price_update: &PriceUpdate,
        _alert: &PriceAlert,
    ) -> Result<bool> {
        match condition {
            AlertCondition::PriceThreshold(threshold) => {
                match &threshold.comparison {
                    PriceComparison::Above => Ok(price_update.price > threshold.target_price),
                    PriceComparison::Below => Ok(price_update.price < threshold.target_price),
                    PriceComparison::Equals => {
                        let tolerance = threshold.tolerance.unwrap_or(Decimal::from_str("0.01").unwrap());
                        Ok((price_update.price - threshold.target_price).abs() <= tolerance)
                    },
                    PriceComparison::Between(low, high) => {
                        Ok(price_update.price >= *low && price_update.price <= *high)
                    },
                    PriceComparison::Outside(low, high) => {
                        Ok(price_update.price < *low || price_update.price > *high)
                    },
                    _ => {
                        // Would implement crossing logic with price history
                        Ok(false)
                    }
                }
            },
            AlertCondition::PercentageChange(change) => {
                // Would calculate percentage change based on timeframe
                Ok(false)
            },
            AlertCondition::Volume(volume_condition) => {
                if let Some(volume) = price_update.volume {
                    Ok(volume.to_u64().unwrap_or(0) > volume_condition.threshold)
                } else {
                    Ok(false)
                }
            },
            _ => {
                // Other conditions would be implemented
                Ok(false)
            }
        }
    }
    
    /// Trigger an alert
    async fn trigger_alert(
        &self,
        mut alert: PriceAlert,
        trigger_price: Decimal,
        condition_details: String,
    ) -> Result<()> {
        info!("🔔 Alert triggered: {} at price {}", alert.alert_id, trigger_price);
        
        // Update alert state
        alert.last_triggered = Some(Utc::now());
        alert.trigger_count += 1;
        
        // Check max triggers
        if let Some(max) = alert.max_triggers {
            if alert.trigger_count >= max {
                alert.status = AlertStatus::Triggered;
                alert.enabled = false;
            }
        }
        
        // Update in storage
        let mut alerts = self.active_alerts.write().await;
        alerts.insert(alert.alert_id.clone(), alert.clone());
        
        // Create triggered alert
        let triggered = TriggeredAlert {
            alert: alert.clone(),
            trigger_price,
            condition_details,
            timestamp: Utc::now(),
        };
        
        // Queue for processing
        let queue = self.alert_queue.read().await;
        queue.send(triggered)?;
        
        // Update statistics
        let mut stats = self.alert_stats.write().await;
        stats.total_triggers += 1;
        
        Ok(())
    }
    
    /// Process alert queue
    async fn process_alert_queue(&self, mut rx: mpsc::UnboundedReceiver<TriggeredAlert>) {
        while let Some(triggered) = rx.recv().await {
            if let Err(e) = self.process_triggered_alert(triggered).await {
                error!("🔔 Error processing triggered alert: {}", e);
            }
        }
    }
    
    /// Process a triggered alert
    async fn process_triggered_alert(&self, triggered: TriggeredAlert) -> Result<()> {
        // Execute actions
        for action in &triggered.alert.actions {
            self.execute_action(action, &triggered).await?;
        }
        
        // Send notifications
        let message = self.format_alert_message(&triggered);
        
        for method in &triggered.alert.delivery_methods {
            self.deliver_alert(&triggered, method, &message).await?;
        }
        
        // Record in history
        self.record_alert_history(triggered).await?;
        
        Ok(())
    }
    
    /// Execute alert action
    async fn execute_action(&self, action: &AlertAction, triggered: &TriggeredAlert) -> Result<()> {
        match action {
            AlertAction::Notify => {
                debug!("🔔 Notification action for alert {}", triggered.alert.alert_id);
            },
            AlertAction::ExecuteTrade { order_type, amount } => {
                warn!("🔔 Trade execution requested: {} {}", order_type, amount);
                // Would integrate with trading engine
            },
            AlertAction::LogEvent { level } => {
                match level.as_str() {
                    "error" => error!("🔔 Alert event: {}", triggered.condition_details),
                    "warn" => warn!("🔔 Alert event: {}", triggered.condition_details),
                    _ => info!("🔔 Alert event: {}", triggered.condition_details),
                }
            },
            _ => {
                debug!("🔔 Unhandled action type");
            }
        }
        
        Ok(())
    }
    
    /// Deliver alert notification
    async fn deliver_alert(
        &self,
        triggered: &TriggeredAlert,
        method: &AlertDeliveryMethod,
        message: &str,
    ) -> Result<()> {
        let channels = self.delivery_channels.read().await;
        
        match method {
            AlertDeliveryMethod::InApp => {
                info!("🔔 In-app notification: {}", message);
            },
            _ => {
                // Would use appropriate delivery channel
                debug!("🔔 Delivering alert via {:?}", method);
            }
        }
        
        Ok(())
    }
    
    /// Format alert message
    fn format_alert_message(&self, triggered: &TriggeredAlert) -> String {
        format!(
            "🔔 {} Alert: {}\n\
            Symbol: {}\n\
            Price: {}\n\
            Condition: {}\n\
            Time: {}",
            match triggered.alert.priority {
                AlertPriority::Emergency => "🚨 EMERGENCY",
                AlertPriority::Critical => "⚠️ CRITICAL",
                AlertPriority::High => "⚡ HIGH",
                AlertPriority::Medium => "📊 MEDIUM",
                AlertPriority::Low => "📌 LOW",
            },
            triggered.alert.name,
            triggered.alert.symbol,
            triggered.trigger_price,
            triggered.condition_details,
            triggered.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
    
    /// Record alert in history
    async fn record_alert_history(&self, triggered: TriggeredAlert) -> Result<()> {
        let history_entry = AlertHistory {
            alert_id: triggered.alert.alert_id.clone(),
            triggered_at: triggered.timestamp,
            trigger_price: triggered.trigger_price,
            condition_met: triggered.condition_details,
            actions_taken: triggered.alert.actions.iter()
                .map(|a| format!("{:?}", a))
                .collect(),
            delivery_status: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        let mut history = self.alert_history.write().await;
        history.push_back(history_entry);
        
        // Keep only last 10000 entries
        if history.len() > 10000 {
            history.pop_front();
        }
        
        Ok(())
    }
    
    // Helper methods
    async fn validate_alert(&self, alert: &PriceAlert) -> Result<()> {
        if alert.conditions.is_empty() {
            return Err(BotError::validation("Alert must have at least one condition".to_string()).into());
        }
        
        if alert.delivery_methods.is_empty() {
            return Err(BotError::validation("Alert must have at least one delivery method".to_string()).into());
        }
        
        Ok(())
    }
    
    async fn load_active_alerts(&self) -> Result<()> {
        // Would load from database
        Ok(())
    }
    
    async fn store_alert(&self, _alert: &PriceAlert) -> Result<()> {
        // Would store in database
        Ok(())
    }
    
    async fn get_monitored_symbols(&self) -> Vec<String> {
        let alerts = self.active_alerts.read().await;
        let mut symbols: Vec<String> = alerts.values()
            .map(|a| a.symbol.clone())
            .collect();
        symbols.sort();
        symbols.dedup();
        symbols
    }
    
    /// Get alert by ID
    pub async fn get_alert(&self, alert_id: &str) -> Option<PriceAlert> {
        let alerts = self.active_alerts.read().await;
        alerts.get(alert_id).cloned()
    }
    
    /// Update alert
    pub async fn update_alert(&self, alert_id: &str, updates: HashMap<String, serde_json::Value>) -> Result<()> {
        let mut alerts = self.active_alerts.write().await;
        
        if let Some(alert) = alerts.get_mut(alert_id) {
            // Would apply updates
            for (key, value) in updates {
                match key.as_str() {
                    "enabled" => {
                        if let Some(enabled) = value.as_bool() {
                            alert.enabled = enabled;
                        }
                    },
                    "name" => {
                        if let Some(name) = value.as_str() {
                            alert.name = name.to_string();
                        }
                    },
                    _ => {}
                }
            }
            
            Ok(())
        } else {
            Err(BotError::not_found(format!("Alert {} not found", alert_id)).into())
        }
    }
    
    /// Delete alert
    pub async fn delete_alert(&self, alert_id: &str) -> Result<bool> {
        let mut alerts = self.active_alerts.write().await;
        let removed = alerts.remove(alert_id).is_some();
        
        if removed {
            let mut stats = self.alert_stats.write().await;
            stats.active_alerts = stats.active_alerts.saturating_sub(1);
        }
        
        Ok(removed)
    }
    
    /// Get alert statistics
    pub async fn get_statistics(&self) -> AlertStatistics {
        let stats = self.alert_stats.read().await;
        stats.clone()
    }
    
    /// Get alert history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<AlertHistory> {
        let history = self.alert_history.read().await;
        let limit = limit.unwrap_or(100).min(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}