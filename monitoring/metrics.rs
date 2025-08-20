use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, GaugeVec, HistogramVec, Registry,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

/// Types of metrics to collect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metrics collector for the trading bot
pub struct MetricsCollector {
    registry: Registry,
    
    // Trading metrics
    trades_total: CounterVec,
    trades_successful: CounterVec,
    trades_failed: CounterVec,
    trade_volume: GaugeVec,
    trade_latency: HistogramVec,
    
    // Wallet metrics
    wallet_balance: GaugeVec,
    wallet_transactions: CounterVec,
    gas_fees_total: CounterVec,
    
    // Bot performance metrics
    bot_uptime: GaugeVec,
    commands_processed: CounterVec,
    api_calls: CounterVec,
    api_latency: HistogramVec,
    cache_hits: CounterVec,
    cache_misses: CounterVec,
    
    // MEV metrics
    mev_bundles_sent: CounterVec,
    mev_bundles_landed: CounterVec,
    mev_protection_saved: GaugeVec,
    
    // Market data metrics
    market_data_updates: CounterVec,
    price_feed_latency: HistogramVec,
    
    // Error metrics
    errors_total: CounterVec,
    
    // Custom metrics storage
    custom_metrics: Arc<RwLock<HashMap<String, CustomMetric>>>,
}

#[derive(Debug, Clone)]
struct CustomMetric {
    name: String,
    value: f64,
    metric_type: MetricType,
    labels: HashMap<String, String>,
    timestamp: DateTime<Utc>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        // Initialize trading metrics
        let trades_total = register_counter_vec!(
            "trades_total",
            "Total number of trades executed",
            &["token", "action", "user"]
        )?;
        registry.register(Box::new(trades_total.clone()))?;
        
        let trades_successful = register_counter_vec!(
            "trades_successful",
            "Number of successful trades",
            &["token", "action"]
        )?;
        registry.register(Box::new(trades_successful.clone()))?;
        
        let trades_failed = register_counter_vec!(
            "trades_failed",
            "Number of failed trades",
            &["token", "action", "reason"]
        )?;
        registry.register(Box::new(trades_failed.clone()))?;
        
        let trade_volume = register_gauge_vec!(
            "trade_volume_sol",
            "Trading volume in SOL",
            &["token", "period"]
        )?;
        registry.register(Box::new(trade_volume.clone()))?;
        
        let trade_latency = register_histogram_vec!(
            "trade_latency_ms",
            "Trade execution latency in milliseconds",
            &["action", "token"],
            vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0]
        )?;
        registry.register(Box::new(trade_latency.clone()))?;
        
        // Initialize wallet metrics
        let wallet_balance = register_gauge_vec!(
            "wallet_balance_sol",
            "Wallet balance in SOL",
            &["wallet", "token"]
        )?;
        registry.register(Box::new(wallet_balance.clone()))?;
        
        let wallet_transactions = register_counter_vec!(
            "wallet_transactions_total",
            "Total wallet transactions",
            &["wallet", "type"]
        )?;
        registry.register(Box::new(wallet_transactions.clone()))?;
        
        let gas_fees_total = register_counter_vec!(
            "gas_fees_sol_total",
            "Total gas fees paid in SOL",
            &["wallet"]
        )?;
        registry.register(Box::new(gas_fees_total.clone()))?;
        
        // Initialize bot performance metrics
        let bot_uptime = register_gauge_vec!(
            "bot_uptime_seconds",
            "Bot uptime in seconds",
            &["instance"]
        )?;
        registry.register(Box::new(bot_uptime.clone()))?;
        
        let commands_processed = register_counter_vec!(
            "commands_processed_total",
            "Total commands processed",
            &["command", "status"]
        )?;
        registry.register(Box::new(commands_processed.clone()))?;
        
        let api_calls = register_counter_vec!(
            "api_calls_total",
            "Total API calls made",
            &["endpoint", "status"]
        )?;
        registry.register(Box::new(api_calls.clone()))?;
        
        let api_latency = register_histogram_vec!(
            "api_latency_ms",
            "API call latency in milliseconds",
            &["endpoint", "method"],
            vec![10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0]
        )?;
        registry.register(Box::new(api_latency.clone()))?;
        
        let cache_hits = register_counter_vec!(
            "cache_hits_total",
            "Total cache hits",
            &["cache_type"]
        )?;
        registry.register(Box::new(cache_hits.clone()))?;
        
        let cache_misses = register_counter_vec!(
            "cache_misses_total",
            "Total cache misses",
            &["cache_type"]
        )?;
        registry.register(Box::new(cache_misses.clone()))?;
        
        // Initialize MEV metrics
        let mev_bundles_sent = register_counter_vec!(
            "mev_bundles_sent_total",
            "Total MEV bundles sent",
            &["strategy"]
        )?;
        registry.register(Box::new(mev_bundles_sent.clone()))?;
        
        let mev_bundles_landed = register_counter_vec!(
            "mev_bundles_landed_total",
            "Total MEV bundles landed",
            &["strategy"]
        )?;
        registry.register(Box::new(mev_bundles_landed.clone()))?;
        
        let mev_protection_saved = register_gauge_vec!(
            "mev_protection_saved_sol",
            "SOL saved through MEV protection",
            &["period"]
        )?;
        registry.register(Box::new(mev_protection_saved.clone()))?;
        
        // Initialize market data metrics
        let market_data_updates = register_counter_vec!(
            "market_data_updates_total",
            "Total market data updates received",
            &["source", "token"]
        )?;
        registry.register(Box::new(market_data_updates.clone()))?;
        
        let price_feed_latency = register_histogram_vec!(
            "price_feed_latency_ms",
            "Price feed update latency",
            &["source"],
            vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0]
        )?;
        registry.register(Box::new(price_feed_latency.clone()))?;
        
        // Initialize error metrics
        let errors_total = register_counter_vec!(
            "errors_total",
            "Total errors occurred",
            &["type", "severity", "component"]
        )?;
        registry.register(Box::new(errors_total.clone()))?;
        
        Ok(Self {
            registry,
            trades_total,
            trades_successful,
            trades_failed,
            trade_volume,
            trade_latency,
            wallet_balance,
            wallet_transactions,
            gas_fees_total,
            bot_uptime,
            commands_processed,
            api_calls,
            api_latency,
            cache_hits,
            cache_misses,
            mev_bundles_sent,
            mev_bundles_landed,
            mev_protection_saved,
            market_data_updates,
            price_feed_latency,
            errors_total,
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Record a trade
    pub fn record_trade(
        &self,
        token: &str,
        action: &str,
        user: &str,
        success: bool,
        volume_sol: f64,
        latency_ms: f64,
    ) {
        self.trades_total
            .with_label_values(&[token, action, user])
            .inc();
        
        if success {
            self.trades_successful
                .with_label_values(&[token, action])
                .inc();
        } else {
            self.trades_failed
                .with_label_values(&[token, action, "execution_failed"])
                .inc();
        }
        
        self.trade_volume
            .with_label_values(&[token, "24h"])
            .add(volume_sol);
        
        self.trade_latency
            .with_label_values(&[action, token])
            .observe(latency_ms);
        
        debug!("Recorded trade: {} {} {}, success: {}, volume: {} SOL", 
            action, token, user, success, volume_sol);
    }
    
    /// Record wallet balance
    pub fn record_wallet_balance(&self, wallet: &str, token: &str, balance: f64) {
        self.wallet_balance
            .with_label_values(&[wallet, token])
            .set(balance);
    }
    
    /// Record wallet transaction
    pub fn record_wallet_transaction(&self, wallet: &str, tx_type: &str, gas_fee: f64) {
        self.wallet_transactions
            .with_label_values(&[wallet, tx_type])
            .inc();
        
        if gas_fee > 0.0 {
            self.gas_fees_total
                .with_label_values(&[wallet])
                .inc_by(gas_fee);
        }
    }
    
    /// Record command processed
    pub fn record_command(&self, command: &str, success: bool) {
        let status = if success { "success" } else { "failed" };
        self.commands_processed
            .with_label_values(&[command, status])
            .inc();
    }
    
    /// Record API call
    pub fn record_api_call(&self, endpoint: &str, method: &str, success: bool, latency_ms: f64) {
        let status = if success { "success" } else { "failed" };
        self.api_calls
            .with_label_values(&[endpoint, status])
            .inc();
        
        self.api_latency
            .with_label_values(&[endpoint, method])
            .observe(latency_ms);
    }
    
    /// Record cache hit/miss
    pub fn record_cache_access(&self, cache_type: &str, hit: bool) {
        if hit {
            self.cache_hits
                .with_label_values(&[cache_type])
                .inc();
        } else {
            self.cache_misses
                .with_label_values(&[cache_type])
                .inc();
        }
    }
    
    /// Record MEV bundle
    pub fn record_mev_bundle(&self, strategy: &str, sent: bool, landed: bool) {
        if sent {
            self.mev_bundles_sent
                .with_label_values(&[strategy])
                .inc();
        }
        if landed {
            self.mev_bundles_landed
                .with_label_values(&[strategy])
                .inc();
        }
    }
    
    /// Record MEV protection savings
    pub fn record_mev_savings(&self, amount_sol: f64) {
        self.mev_protection_saved
            .with_label_values(&["total"])
            .add(amount_sol);
        
        self.mev_protection_saved
            .with_label_values(&["24h"])
            .set(amount_sol);
    }
    
    /// Record market data update
    pub fn record_market_update(&self, source: &str, token: &str, latency_ms: f64) {
        self.market_data_updates
            .with_label_values(&[source, token])
            .inc();
        
        self.price_feed_latency
            .with_label_values(&[source])
            .observe(latency_ms);
    }
    
    /// Record error
    pub fn record_error(&self, error_type: &str, severity: &str, component: &str) {
        self.errors_total
            .with_label_values(&[error_type, severity, component])
            .inc();
        
        warn!("Error recorded: {} in {} (severity: {})", error_type, component, severity);
    }
    
    /// Update bot uptime
    pub fn update_uptime(&self, seconds: f64) {
        self.bot_uptime
            .with_label_values(&["main"])
            .set(seconds);
    }
    
    /// Add custom metric
    pub async fn add_custom_metric(
        &self,
        name: String,
        value: f64,
        metric_type: MetricType,
        labels: HashMap<String, String>,
    ) {
        let metric = CustomMetric {
            name: name.clone(),
            value,
            metric_type,
            labels,
            timestamp: Utc::now(),
        };
        
        let mut metrics = self.custom_metrics.write().await;
        metrics.insert(name, metric);
    }
    
    /// Get custom metric
    pub async fn get_custom_metric(&self, name: &str) -> Option<CustomMetric> {
        let metrics = self.custom_metrics.read().await;
        metrics.get(name).cloned()
    }
    
    /// Get all metrics as Prometheus format
    pub fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }
    
    /// Get metrics summary
    pub async fn get_summary(&self) -> MetricsSummary {
        let custom_metrics = self.custom_metrics.read().await;
        
        MetricsSummary {
            total_trades: self.get_counter_value(&self.trades_total),
            successful_trades: self.get_counter_value(&self.trades_successful),
            failed_trades: self.get_counter_value(&self.trades_failed),
            total_volume_sol: self.get_gauge_value(&self.trade_volume),
            total_commands: self.get_counter_value(&self.commands_processed),
            total_api_calls: self.get_counter_value(&self.api_calls),
            cache_hit_rate: self.calculate_cache_hit_rate(),
            mev_bundles_success_rate: self.calculate_mev_success_rate(),
            total_errors: self.get_counter_value(&self.errors_total),
            custom_metrics_count: custom_metrics.len(),
            uptime_seconds: self.get_gauge_value(&self.bot_uptime),
        }
    }
    
    /// Helper to get counter value
    fn get_counter_value(&self, counter: &CounterVec) -> f64 {
        // In production, would properly aggregate counter values
        0.0
    }
    
    /// Helper to get gauge value
    fn get_gauge_value(&self, gauge: &GaugeVec) -> f64 {
        // In production, would properly get gauge value
        0.0
    }
    
    /// Calculate cache hit rate
    fn calculate_cache_hit_rate(&self) -> f64 {
        // In production, would calculate from actual metrics
        0.95
    }
    
    /// Calculate MEV success rate
    fn calculate_mev_success_rate(&self) -> f64 {
        // In production, would calculate from actual metrics
        0.92
    }
}

/// Metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_trades: f64,
    pub successful_trades: f64,
    pub failed_trades: f64,
    pub total_volume_sol: f64,
    pub total_commands: f64,
    pub total_api_calls: f64,
    pub cache_hit_rate: f64,
    pub mev_bundles_success_rate: f64,
    pub total_errors: f64,
    pub custom_metrics_count: usize,
    pub uptime_seconds: f64,
}