use chrono::{DateTime, Utc, Duration, Timelike, Weekday, NaiveTime};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration as TokioDuration};
use tracing::{info, debug, warn, error};

use crate::errors::{BotError, Result};
use crate::trading::dca::{DCAEngine, DCAStrategy, DCAInterval};
use crate::telemetry::TelemetryService;

/// Advanced DCA scheduler with multiple scheduling strategies
#[derive(Clone)]
pub struct DCAScheduler {
    dca_engine: Arc<DCAEngine>,
    telemetry: Option<Arc<TelemetryService>>,
    schedule_queue: Arc<RwLock<BinaryHeap<Reverse<ScheduledExecution>>>>,
    active_schedules: Arc<RwLock<HashMap<String, ScheduleConfig>>>,
    timezone_manager: Arc<TimezoneManager>,
    market_hours: Arc<MarketHoursManager>,
    execution_stats: Arc<RwLock<ExecutionStats>>,
}

/// Scheduled execution entry
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScheduledExecution {
    pub execute_at: DateTime<Utc>,
    pub strategy_id: String,
    pub execution_type: ExecutionType,
    pub priority: u8, // 0 = highest priority
}

/// Types of scheduled executions
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionType {
    Regular,
    PriceAlert,
    MarketOpen,
    MarketClose,
    Emergency,
    Rebalance,
}

/// Advanced schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub schedule_id: String,
    pub strategy_id: String,
    pub name: String,
    pub schedule_type: ScheduleType,
    pub timezone: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_executed: Option<DateTime<Utc>>,
    pub next_execution: DateTime<Utc>,
    pub execution_count: u64,
    pub max_executions: Option<u64>,
    pub execution_window: Option<TimeWindow>,
    pub market_hours_only: bool,
    pub skip_weekends: bool,
    pub skip_holidays: bool,
    pub conditions: Vec<ExecutionCondition>,
    pub notifications: NotificationConfig,
}

/// Advanced schedule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    /// Fixed interval scheduling
    Interval { 
        interval: DCAInterval,
        offset_minutes: Option<i32>,
    },
    /// Cron-based scheduling
    Cron { 
        expression: String,
        description: Option<String>,
    },
    /// Market event based scheduling
    MarketEvent {
        event: MarketEvent,
        delay_minutes: Option<i32>,
    },
    /// Price condition based scheduling
    PriceBased {
        price_conditions: Vec<PriceCondition>,
        check_interval_minutes: u32,
    },
    /// Volume spike based scheduling
    VolumeBased {
        volume_threshold: f64,
        spike_percentage: f64,
        check_interval_minutes: u32,
    },
    /// Technical indicator based scheduling
    TechnicalBased {
        indicators: Vec<TechnicalIndicator>,
        check_interval_minutes: u32,
    },
    /// Custom algorithm based scheduling
    Algorithm {
        algorithm_name: String,
        parameters: HashMap<String, String>,
    },
}

/// Market events for scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    MarketOpen,
    MarketClose,
    PreMarket,
    AfterHours,
    EarningsAnnouncement,
    FedAnnouncement,
    CryptoMaintenance,
}

/// Price condition for scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceCondition {
    pub token_mint: String,
    pub condition_type: PriceConditionType,
    pub target_price: f64,
    pub tolerance_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceConditionType {
    Above,
    Below,
    MovingAverageAbove { periods: u32 },
    MovingAverageBelow { periods: u32 },
    PercentageChange { timeframe_hours: u32 },
    Volatility { threshold: f64 },
}

/// Technical indicators for scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicator {
    pub indicator_type: IndicatorType,
    pub condition: IndicatorCondition,
    pub parameters: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndicatorType {
    RSI,
    MACD,
    BollingerBands,
    StochasticOscillator,
    FearGreedIndex,
    VolumeProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndicatorCondition {
    Above(f64),
    Below(f64),
    Between(f64, f64),
    CrossingAbove(f64),
    CrossingBelow(f64),
    Divergence,
}

/// Execution conditions for complex scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCondition {
    pub condition_id: String,
    pub condition_type: ConditionType,
    pub is_required: bool,
    pub weight: f64, // For weighted conditions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    MinimumBalance { token_mint: String, minimum_amount: f64 },
    MaximumSlippage { max_slippage_bps: u16 },
    MarketVolatility { max_volatility: f64 },
    NetworkCongestion { max_priority_fee: u64 },
    ExternalApiHealth { api_name: String },
    UserApproval { require_confirmation: bool },
    PriceStability { max_change_percent: f64, timeframe_minutes: u32 },
}

/// Time window for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub days_of_week: Vec<Weekday>,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub notify_on_execution: bool,
    pub notify_on_failure: bool,
    pub notify_on_conditions_met: bool,
    pub notification_channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Telegram { chat_id: i64 },
    Email { address: String },
    Webhook { url: String, secret: Option<String> },
    Discord { webhook_url: String },
}

/// Timezone management
#[derive(Debug)]
pub struct TimezoneManager {
    user_timezones: RwLock<HashMap<i64, String>>, // user_id -> timezone
    default_timezone: String,
}

/// Market hours management
#[derive(Debug)]
pub struct MarketHoursManager {
    market_schedules: HashMap<String, MarketSchedule>,
}

#[derive(Debug, Clone)]
pub struct MarketSchedule {
    pub open_time: NaiveTime,
    pub close_time: NaiveTime,
    pub timezone: String,
    pub trading_days: Vec<Weekday>,
    pub holidays: Vec<DateTime<Utc>>,
}

/// Execution statistics
#[derive(Debug, Default)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub last_execution: Option<DateTime<Utc>>,
    pub execution_history: Vec<ExecutionRecord>,
}

#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub timestamp: DateTime<Utc>,
    pub strategy_id: String,
    pub execution_type: ExecutionType,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

impl DCAScheduler {
    /// Create new DCA scheduler
    pub fn new(
        dca_engine: Arc<DCAEngine>,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("⏰ Initializing DCA scheduler");
        
        let timezone_manager = Arc::new(TimezoneManager {
            user_timezones: RwLock::new(HashMap::new()),
            default_timezone: "UTC".to_string(),
        });
        
        let mut market_schedules = HashMap::new();
        
        // Add major market schedules
        market_schedules.insert("NYSE".to_string(), MarketSchedule {
            open_time: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            close_time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            timezone: "America/New_York".to_string(),
            trading_days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            holidays: vec![], // Would be populated with market holidays
        });
        
        market_schedules.insert("CRYPTO".to_string(), MarketSchedule {
            open_time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            close_time: NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            timezone: "UTC".to_string(),
            trading_days: vec![
                Weekday::Mon, Weekday::Tue, Weekday::Wed, 
                Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun
            ],
            holidays: vec![],
        });
        
        let market_hours = Arc::new(MarketHoursManager {
            market_schedules,
        });
        
        Self {
            dca_engine,
            telemetry,
            schedule_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            active_schedules: Arc::new(RwLock::new(HashMap::new())),
            timezone_manager,
            market_hours,
            execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
        }
    }
    
    /// Start the scheduler background task
    pub async fn start(&self) -> Result<()> {
        info!("⏰ Starting DCA scheduler background task");
        
        let scheduler = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = scheduler.process_scheduled_executions().await {
                    error!("⏰ Scheduler error: {}", e);
                }
                
                // Check every 30 seconds
                sleep(TokioDuration::from_secs(30)).await;
            }
        });
        
        Ok(())
    }
    
    /// Add a new schedule configuration
    pub async fn add_schedule(&self, config: ScheduleConfig) -> Result<String> {
        // Validate schedule configuration
        self.validate_schedule(&config).await?;
        
        // Calculate next execution time
        let next_execution = self.calculate_next_execution(&config).await?;
        
        let mut updated_config = config.clone();
        updated_config.next_execution = next_execution;
        
        // Add to active schedules
        let schedule_id = config.schedule_id.clone();
        let mut schedules = self.active_schedules.write().await;
        schedules.insert(schedule_id.clone(), updated_config.clone());
        
        // Add to execution queue
        self.enqueue_execution(&updated_config).await?;
        
        info!("⏰ Added schedule: {} for strategy {}", 
            schedule_id, config.strategy_id);
        
        Ok(schedule_id)
    }
    
    /// Remove a schedule
    pub async fn remove_schedule(&self, schedule_id: &str) -> Result<bool> {
        let mut schedules = self.active_schedules.write().await;
        let removed = schedules.remove(schedule_id).is_some();
        
        if removed {
            // Remove from queue (would need to rebuild queue)
            self.rebuild_schedule_queue().await?;
            info!("⏰ Removed schedule: {}", schedule_id);
        }
        
        Ok(removed)
    }
    
    /// Process scheduled executions
    async fn process_scheduled_executions(&self) -> Result<()> {
        let now = Utc::now();
        let mut executions_to_process = Vec::new();
        
        // Get executions that are ready
        {
            let mut queue = self.schedule_queue.write().await;
            while let Some(Reverse(execution)) = queue.peek() {
                if execution.execute_at <= now {
                    let execution = queue.pop().unwrap().0;
                    executions_to_process.push(execution);
                } else {
                    break;
                }
            }
        }
        
        // Process each execution
        for execution in executions_to_process {
            let start_time = std::time::Instant::now();
            let mut success = false;
            let mut error_message = None;
            
            // Create tracing span
            let _span = self.telemetry.as_ref().map(|t| 
                t.create_trading_span("dca_scheduled_execution", None)
            );
            
            match self.execute_scheduled_task(&execution).await {
                Ok(_) => {
                    success = true;
                    debug!("⏰ Successfully executed scheduled task for strategy {}", 
                        execution.strategy_id);
                },
                Err(e) => {
                    error_message = Some(e.to_string());
                    error!("⏰ Failed to execute scheduled task for strategy {}: {}", 
                        execution.strategy_id, e);
                }
            }
            
            let duration = start_time.elapsed();
            
            // Record execution statistics
            self.record_execution_stats(&execution, duration.as_millis() as u64, success, error_message).await;
            
            // Schedule next execution if this was a recurring schedule
            if success {
                self.schedule_next_execution(&execution.strategy_id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Execute a scheduled task
    async fn execute_scheduled_task(&self, execution: &ScheduledExecution) -> Result<()> {
        // Get the schedule configuration
        let schedules = self.active_schedules.read().await;
        let schedule = schedules.get(&execution.strategy_id)
            .ok_or_else(|| BotError::not_found(format!("Schedule for strategy {} not found", execution.strategy_id)))?
            .clone();
        drop(schedules);
        
        // Check execution conditions
        if !self.check_execution_conditions(&schedule).await? {
            debug!("⏰ Execution conditions not met for strategy {}, skipping", execution.strategy_id);
            return Ok(());
        }
        
        // Check market hours if required
        if schedule.market_hours_only && !self.is_market_open(&schedule).await? {
            debug!("⏰ Market closed for strategy {}, skipping", execution.strategy_id);
            return Ok(());
        }
        
        // Execute the DCA strategy
        match execution.execution_type {
            ExecutionType::Regular => {
                // Find the DCA strategy and execute it
                // This would integrate with the DCA engine
                info!("⏰ Executing regular DCA for strategy {}", execution.strategy_id);
            },
            ExecutionType::PriceAlert => {
                info!("⏰ Executing price alert triggered DCA for strategy {}", execution.strategy_id);
            },
            ExecutionType::MarketOpen => {
                info!("⏰ Executing market open DCA for strategy {}", execution.strategy_id);
            },
            ExecutionType::Emergency => {
                info!("⏰ Executing emergency DCA for strategy {}", execution.strategy_id);
            },
            _ => {
                info!("⏰ Executing DCA for strategy {} (type: {:?})", 
                    execution.strategy_id, execution.execution_type);
            }
        }
        
        // Send notifications if configured
        self.send_execution_notification(&schedule, true).await?;
        
        Ok(())
    }
    
    /// Calculate next execution time based on schedule type
    async fn calculate_next_execution(&self, config: &ScheduleConfig) -> Result<DateTime<Utc>> {
        let now = Utc::now();
        
        let next = match &config.schedule_type {
            ScheduleType::Interval { interval, offset_minutes } => {
                let base_next = self.calculate_interval_next(interval, now)?;
                if let Some(offset) = offset_minutes {
                    base_next + Duration::minutes(*offset as i64)
                } else {
                    base_next
                }
            },
            
            ScheduleType::Cron { expression, .. } => {
                let schedule = Schedule::from_str(expression)
                    .map_err(|e| BotError::config(format!("Invalid cron expression: {}", e)))?;
                    
                schedule.upcoming(chrono::Utc)
                    .take(1)
                    .next()
                    .ok_or_else(|| BotError::config("No future execution time found".to_string()))?
            },
            
            ScheduleType::MarketEvent { event, delay_minutes } => {
                let market_time = self.calculate_market_event_time(event, now).await?;
                if let Some(delay) = delay_minutes {
                    market_time + Duration::minutes(*delay as i64)
                } else {
                    market_time
                }
            },
            
            ScheduleType::PriceBased { check_interval_minutes, .. } => {
                now + Duration::minutes(*check_interval_minutes as i64)
            },
            
            ScheduleType::VolumeBased { check_interval_minutes, .. } => {
                now + Duration::minutes(*check_interval_minutes as i64)
            },
            
            ScheduleType::TechnicalBased { check_interval_minutes, .. } => {
                now + Duration::minutes(*check_interval_minutes as i64)
            },
            
            ScheduleType::Algorithm { .. } => {
                // Custom algorithm scheduling
                now + Duration::hours(1) // Default fallback
            },
        };
        
        // Apply time window restrictions if configured
        let adjusted_next = if let Some(window) = &config.execution_window {
            self.adjust_for_time_window(next, window)?
        } else {
            next
        };
        
        // Skip weekends if configured
        let final_next = if config.skip_weekends {
            self.skip_weekend(adjusted_next)
        } else {
            adjusted_next
        };
        
        Ok(final_next)
    }
    
    /// Helper methods for scheduling logic
    fn calculate_interval_next(&self, interval: &DCAInterval, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
        let next = match interval {
            DCAInterval::Minutes(m) => now + Duration::minutes(*m as i64),
            DCAInterval::Hourly => now + Duration::hours(1),
            DCAInterval::Daily => now + Duration::days(1),
            DCAInterval::Weekly => now + Duration::weeks(1),
            DCAInterval::Biweekly => now + Duration::weeks(2),
            DCAInterval::Monthly => now + Duration::days(30), // Approximate
            DCAInterval::Custom { cron_expression } => {
                let schedule = Schedule::from_str(cron_expression)
                    .map_err(|e| BotError::config(format!("Invalid cron expression: {}", e)))?;
                    
                schedule.upcoming(chrono::Utc)
                    .take(1)
                    .next()
                    .ok_or_else(|| BotError::config("No future execution time found".to_string()))?
            }
        };
        
        Ok(next)
    }
    
    async fn calculate_market_event_time(&self, event: &MarketEvent, _now: DateTime<Utc>) -> Result<DateTime<Utc>> {
        match event {
            MarketEvent::MarketOpen => {
                // Calculate next market open time
                let crypto_schedule = self.market_hours.market_schedules.get("CRYPTO")
                    .ok_or_else(|| BotError::config("Crypto market schedule not found".to_string()))?;
                    
                // For crypto, market is always open, so return current time
                Ok(Utc::now())
            },
            MarketEvent::MarketClose => {
                // Calculate next market close time
                Ok(Utc::now() + Duration::hours(24)) // Placeholder
            },
            _ => {
                // Other market events would be calculated here
                Ok(Utc::now() + Duration::hours(1))
            }
        }
    }
    
    fn adjust_for_time_window(&self, datetime: DateTime<Utc>, window: &TimeWindow) -> Result<DateTime<Utc>> {
        let weekday = datetime.weekday();
        
        // Check if the day is allowed
        if !window.days_of_week.contains(&weekday) {
            // Find the next allowed day
            let mut next_date = datetime.date_naive();
            loop {
                next_date = next_date.succ_opt()
                    .ok_or_else(|| BotError::config("Date calculation overflow".to_string()))?;
                if window.days_of_week.contains(&next_date.weekday()) {
                    break;
                }
            }
            
            // Set to start of time window on the next allowed day
            return Ok(next_date.and_time(window.start_time).and_utc());
        }
        
        // Check if the time is within the window
        let time = datetime.time();
        if time >= window.start_time && time <= window.end_time {
            Ok(datetime) // Already within window
        } else if time < window.start_time {
            // Move to start of window same day
            Ok(datetime.date_naive().and_time(window.start_time).and_utc())
        } else {
            // Move to start of window next allowed day
            let mut next_date = datetime.date_naive().succ_opt()
                .ok_or_else(|| BotError::config("Date calculation overflow".to_string()))?;
                
            while !window.days_of_week.contains(&next_date.weekday()) {
                next_date = next_date.succ_opt()
                    .ok_or_else(|| BotError::config("Date calculation overflow".to_string()))?;
            }
            
            Ok(next_date.and_time(window.start_time).and_utc())
        }
    }
    
    fn skip_weekend(&self, datetime: DateTime<Utc>) -> DateTime<Utc> {
        let weekday = datetime.weekday();
        match weekday {
            Weekday::Sat => datetime + Duration::days(2), // Move to Monday
            Weekday::Sun => datetime + Duration::days(1), // Move to Monday
            _ => datetime, // Weekday, no change needed
        }
    }
    
    async fn check_execution_conditions(&self, _schedule: &ScheduleConfig) -> Result<bool> {
        // Implementation would check all conditions
        // For now, always return true
        Ok(true)
    }
    
    async fn is_market_open(&self, _schedule: &ScheduleConfig) -> Result<bool> {
        // Implementation would check market hours
        // For crypto, market is always open
        Ok(true)
    }
    
    async fn enqueue_execution(&self, config: &ScheduleConfig) -> Result<()> {
        let execution = ScheduledExecution {
            execute_at: config.next_execution,
            strategy_id: config.strategy_id.clone(),
            execution_type: ExecutionType::Regular,
            priority: 5, // Default priority
        };
        
        let mut queue = self.schedule_queue.write().await;
        queue.push(Reverse(execution));
        
        Ok(())
    }
    
    async fn rebuild_schedule_queue(&self) -> Result<()> {
        let mut queue = self.schedule_queue.write().await;
        queue.clear();
        
        let schedules = self.active_schedules.read().await;
        for config in schedules.values() {
            if config.is_active {
                let execution = ScheduledExecution {
                    execute_at: config.next_execution,
                    strategy_id: config.strategy_id.clone(),
                    execution_type: ExecutionType::Regular,
                    priority: 5,
                };
                queue.push(Reverse(execution));
            }
        }
        
        Ok(())
    }
    
    async fn schedule_next_execution(&self, strategy_id: &str) -> Result<()> {
        let mut schedules = self.active_schedules.write().await;
        if let Some(config) = schedules.get_mut(strategy_id) {
            config.last_executed = Some(Utc::now());
            config.execution_count += 1;
            
            // Check if we've reached max executions
            if let Some(max) = config.max_executions {
                if config.execution_count >= max {
                    config.is_active = false;
                    info!("⏰ Schedule {} completed after {} executions", 
                        config.schedule_id, config.execution_count);
                    return Ok(());
                }
            }
            
            // Calculate next execution
            config.next_execution = self.calculate_next_execution(config).await?;
            
            // Re-enqueue
            self.enqueue_execution(config).await?;
        }
        
        Ok(())
    }
    
    async fn record_execution_stats(
        &self, 
        execution: &ScheduledExecution, 
        duration_ms: u64, 
        success: bool, 
        error: Option<String>
    ) {
        let mut stats = self.execution_stats.write().await;
        
        stats.total_executions += 1;
        if success {
            stats.successful_executions += 1;
        } else {
            stats.failed_executions += 1;
        }
        
        // Update average execution time
        stats.average_execution_time_ms = (stats.average_execution_time_ms * (stats.total_executions - 1) as f64 + duration_ms as f64) / stats.total_executions as f64;
        stats.last_execution = Some(Utc::now());
        
        // Add to history (keep last 1000 records)
        stats.execution_history.push(ExecutionRecord {
            timestamp: Utc::now(),
            strategy_id: execution.strategy_id.clone(),
            execution_type: execution.execution_type.clone(),
            duration_ms,
            success,
            error,
        });
        
        if stats.execution_history.len() > 1000 {
            stats.execution_history.drain(0..stats.execution_history.len() - 1000);
        }
    }
    
    async fn send_execution_notification(&self, _schedule: &ScheduleConfig, _success: bool) -> Result<()> {
        // Implementation would send notifications based on config
        Ok(())
    }
    
    async fn validate_schedule(&self, _config: &ScheduleConfig) -> Result<()> {
        // Implementation would validate schedule configuration
        Ok(())
    }
    
    /// Get execution statistics
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        let stats = self.execution_stats.read().await;
        stats.clone()
    }
    
    /// Get active schedules
    pub async fn get_active_schedules(&self) -> Vec<ScheduleConfig> {
        let schedules = self.active_schedules.read().await;
        schedules.values()
            .filter(|config| config.is_active)
            .cloned()
            .collect()
    }
}

/// Helper functions for creating common schedule configurations
impl ScheduleConfig {
    /// Create a simple daily schedule
    pub fn create_daily_schedule(
        strategy_id: String,
        name: String,
        hour: u32,
        minute: u32,
        timezone: String,
    ) -> Self {
        let cron_expression = format!("0 {} {} * * *", minute, hour);
        
        Self {
            schedule_id: uuid::Uuid::new_v4().to_string(),
            strategy_id,
            name,
            schedule_type: ScheduleType::Cron {
                expression: cron_expression,
                description: Some(format!("Daily at {}:{:02}", hour, minute)),
            },
            timezone,
            is_active: true,
            created_at: Utc::now(),
            last_executed: None,
            next_execution: Utc::now(), // Will be calculated properly
            execution_count: 0,
            max_executions: None,
            execution_window: None,
            market_hours_only: false,
            skip_weekends: false,
            skip_holidays: false,
            conditions: vec![],
            notifications: NotificationConfig::default(),
        }
    }
    
    /// Create a market hours only schedule
    pub fn create_market_hours_schedule(
        strategy_id: String,
        name: String,
        interval: DCAInterval,
    ) -> Self {
        Self {
            schedule_id: uuid::Uuid::new_v4().to_string(),
            strategy_id,
            name,
            schedule_type: ScheduleType::Interval {
                interval,
                offset_minutes: None,
            },
            timezone: "UTC".to_string(),
            is_active: true,
            created_at: Utc::now(),
            last_executed: None,
            next_execution: Utc::now(),
            execution_count: 0,
            max_executions: None,
            execution_window: Some(TimeWindow {
                start_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                days_of_week: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            }),
            market_hours_only: true,
            skip_weekends: true,
            skip_holidays: true,
            conditions: vec![],
            notifications: NotificationConfig::default(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            notify_on_execution: false,
            notify_on_failure: true,
            notify_on_conditions_met: false,
            notification_channels: vec![],
        }
    }
}