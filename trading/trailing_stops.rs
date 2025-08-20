use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use crate::errors::{BotError, Result};
use crate::api::jupiter_price_v3::JupiterPriceV3Client;
use crate::telemetry::TelemetryService;
use crate::trading::orders::{OrderManager, Order, OrderType, OrderStatus};

/// Advanced trailing stop manager with multiple trailing strategies
#[derive(Clone)]
pub struct TrailingStopManager {
    order_manager: Arc<OrderManager>,
    price_client: Arc<JupiterPriceV3Client>,
    telemetry: Option<Arc<TelemetryService>>,
    active_trailing_stops: Arc<RwLock<HashMap<String, TrailingStopState>>>,
    price_tracker: Arc<RwLock<HashMap<String, PriceTracker>>>,
}

/// Trailing stop configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailingStopState {
    pub stop_id: String,
    pub order_id: String,
    pub user_id: i64,
    pub token_mint: String,
    pub strategy: TrailingStrategy,
    pub position_side: PositionSide,
    pub current_stop_price: Decimal,
    pub highest_price: Decimal,      // For long positions
    pub lowest_price: Decimal,       // For short positions
    pub entry_price: Decimal,
    pub position_size: Decimal,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub status: TrailingStopStatus,
    pub performance_metrics: TrailingPerformanceMetrics,
    pub risk_controls: TrailingRiskControls,
}

/// Different trailing stop strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrailingStrategy {
    /// Fixed amount trailing (e.g., trail by $10)
    FixedAmount {
        trailing_amount: Decimal,
        activation_threshold: Option<Decimal>,
    },
    /// Percentage-based trailing (e.g., trail by 5%)
    Percentage {
        trailing_percentage: f64,
        activation_threshold: Option<f64>,
    },
    /// ATR-based trailing using Average True Range
    ATR {
        atr_multiplier: f64,
        atr_periods: u32,
        min_trailing_amount: Decimal,
        max_trailing_amount: Decimal,
    },
    /// Volatility-adjusted trailing
    VolatilityAdjusted {
        base_percentage: f64,
        volatility_multiplier: f64,
        lookback_periods: u32,
        min_percentage: f64,
        max_percentage: f64,
    },
    /// Adaptive trailing based on market conditions
    Adaptive {
        base_percentage: f64,
        trend_factor: f64,
        volume_factor: f64,
        volatility_factor: f64,
        sentiment_factor: f64,
    },
    /// Time-based trailing (tighten over time)
    TimeBased {
        initial_percentage: f64,
        final_percentage: f64,
        time_period: Duration,
        curve_type: TimeCurveType,
    },
    /// Support/Resistance level trailing
    TechnicalLevels {
        support_resistance_buffer: f64,
        level_strength_threshold: f64,
        max_trail_percentage: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeCurveType {
    Linear,
    Exponential,
    Logarithmic,
    StepFunction(Vec<f64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrailingStopStatus {
    Active,
    Triggered,
    Cancelled,
    Expired,
    Paused,
}

/// Performance metrics for trailing stops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailingPerformanceMetrics {
    pub max_favorable_excursion: Decimal,  // Best price reached
    pub max_adverse_excursion: Decimal,    // Worst price reached
    pub total_adjustments: u32,
    pub average_adjustment_size: Decimal,
    pub time_in_profit: Duration,
    pub time_in_loss: Duration,
    pub efficiency_ratio: f64,            // Profit capture efficiency
}

/// Risk controls for trailing stops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailingRiskControls {
    pub max_loss_percentage: f64,
    pub profit_lock_percentage: Option<f64>, // Lock in profits at this level
    pub time_stop: Option<DateTime<Utc>>,     // Exit at specific time
    pub volume_threshold: Option<u64>,        // Pause if volume too low
    pub correlation_stop: Option<f64>,        // Stop if correlation changes
    pub drawdown_limit: Option<f64>,          // Stop if drawdown exceeds limit
}

/// Price tracking for calculating trailing stops
#[derive(Debug, Clone)]
pub struct PriceTracker {
    pub token_mint: String,
    pub current_price: Decimal,
    pub price_history: Vec<PriceCandle>,
    pub volatility_metrics: VolatilityMetrics,
    pub technical_levels: TechnicalLevels,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PriceCandle {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct VolatilityMetrics {
    pub atr: f64,                    // Average True Range
    pub realized_volatility: f64,    // Historical volatility
    pub implied_volatility: Option<f64>,
    pub bollinger_width: f64,        // Bollinger band width
}

#[derive(Debug, Clone)]
pub struct TechnicalLevels {
    pub support_levels: Vec<SupportResistanceLevel>,
    pub resistance_levels: Vec<SupportResistanceLevel>,
    pub trend_direction: TrendDirection,
    pub trend_strength: f64,
}

#[derive(Debug, Clone)]
pub struct SupportResistanceLevel {
    pub price: Decimal,
    pub strength: f64,           // 0.0 to 1.0
    pub touches: u32,
    pub last_test: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum TrendDirection {
    Bullish,
    Bearish,
    Sideways,
}

impl TrailingStopManager {
    /// Create new trailing stop manager
    pub fn new(
        order_manager: Arc<OrderManager>,
        price_client: Arc<JupiterPriceV3Client>,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("🔄 Initializing advanced trailing stop manager");
        
        Self {
            order_manager,
            price_client,
            telemetry,
            active_trailing_stops: Arc::new(RwLock::new(HashMap::new())),
            price_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start the trailing stop monitoring background task
    pub async fn start(&self) -> Result<()> {
        info!("🔄 Starting trailing stop monitoring background task");
        
        let manager = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = manager.monitor_trailing_stops().await {
                    error!("🔄 Trailing stop monitoring error: {}", e);
                }
                
                // Check trailing stops every 1 second for high frequency monitoring
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        
        // Start price tracking task
        let manager = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = manager.update_price_tracking().await {
                    error!("🔄 Price tracking error: {}", e);
                }
                
                // Update price data every 500ms for real-time tracking
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
        
        Ok(())
    }
    
    /// Create a new trailing stop
    pub async fn create_trailing_stop(
        &self,
        user_id: i64,
        token_mint: String,
        strategy: TrailingStrategy,
        position_side: PositionSide,
        entry_price: Decimal,
        position_size: Decimal,
        risk_controls: TrailingRiskControls,
    ) -> Result<String> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_trading_span("create_trailing_stop", Some(&token_mint))
        );
        
        // Calculate initial stop price
        let initial_stop_price = self.calculate_initial_stop_price(
            &strategy,
            &position_side,
            entry_price,
        ).await?;
        
        // Create underlying order
        let order = Order::create_stop_loss(
            user_id,
            token_mint.clone(),
            initial_stop_price,
            position_size,
        );
        
        let order_id = self.order_manager.create_order(order).await?;
        
        // Create trailing stop state
        let stop_id = uuid::Uuid::new_v4().to_string();
        let trailing_stop = TrailingStopState {
            stop_id: stop_id.clone(),
            order_id,
            user_id,
            token_mint: token_mint.clone(),
            strategy,
            position_side,
            current_stop_price: initial_stop_price,
            highest_price: entry_price,
            lowest_price: entry_price,
            entry_price,
            position_size,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            status: TrailingStopStatus::Active,
            performance_metrics: TrailingPerformanceMetrics::new(),
            risk_controls,
        };
        
        // Store trailing stop
        let mut stops = self.active_trailing_stops.write().await;
        stops.insert(stop_id.clone(), trailing_stop);
        
        // Initialize price tracking for this token
        self.initialize_price_tracking(&token_mint).await?;
        
        info!("🔄 Created trailing stop: {} for token {}", stop_id, token_mint);
        
        Ok(stop_id)
    }
    
    /// Monitor all active trailing stops
    async fn monitor_trailing_stops(&self) -> Result<()> {
        let stops: Vec<TrailingStopState> = {
            let stops_lock = self.active_trailing_stops.read().await;
            stops_lock.values()
                .filter(|s| s.status == TrailingStopStatus::Active)
                .cloned()
                .collect()
        };
        
        for stop in stops {
            if let Err(e) = self.update_trailing_stop(&stop).await {
                error!("🔄 Failed to update trailing stop {}: {}", stop.stop_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Update a specific trailing stop based on current price
    async fn update_trailing_stop(&self, stop: &TrailingStopState) -> Result<()> {
        let current_price = self.get_current_price(&stop.token_mint).await?;
        let price_tracker = self.get_price_tracker(&stop.token_mint).await?;
        
        // Calculate new stop price based on strategy
        let new_stop_price = self.calculate_new_stop_price(
            stop,
            current_price,
            &price_tracker,
        ).await?;
        
        // Check if stop should be updated
        let should_update = match stop.position_side {
            PositionSide::Long => new_stop_price > stop.current_stop_price,
            PositionSide::Short => new_stop_price < stop.current_stop_price,
        };
        
        if should_update {
            self.update_stop_price(&stop.stop_id, new_stop_price, current_price).await?;
        }
        
        // Update performance metrics
        self.update_performance_metrics(&stop.stop_id, current_price).await?;
        
        // Check risk controls
        self.check_risk_controls(stop, current_price).await?;
        
        Ok(())
    }
    
    /// Calculate new stop price based on trailing strategy
    async fn calculate_new_stop_price(
        &self,
        stop: &TrailingStopState,
        current_price: Decimal,
        price_tracker: &PriceTracker,
    ) -> Result<Decimal> {
        match &stop.strategy {
            TrailingStrategy::FixedAmount { trailing_amount, activation_threshold } => {
                // Check activation threshold
                if let Some(threshold) = activation_threshold {
                    let profit = match stop.position_side {
                        PositionSide::Long => current_price - stop.entry_price,
                        PositionSide::Short => stop.entry_price - current_price,
                    };
                    if profit < *threshold {
                        return Ok(stop.current_stop_price);
                    }
                }
                
                match stop.position_side {
                    PositionSide::Long => Ok(current_price - trailing_amount),
                    PositionSide::Short => Ok(current_price + trailing_amount),
                }
            },
            
            TrailingStrategy::Percentage { trailing_percentage, activation_threshold } => {
                // Check activation threshold
                if let Some(threshold) = activation_threshold {
                    let profit_percentage = match stop.position_side {
                        PositionSide::Long => ((current_price - stop.entry_price) / stop.entry_price * Decimal::from(100)).to_f64().unwrap_or(0.0),
                        PositionSide::Short => ((stop.entry_price - current_price) / stop.entry_price * Decimal::from(100)).to_f64().unwrap_or(0.0),
                    };
                    if profit_percentage < *threshold {
                        return Ok(stop.current_stop_price);
                    }
                }
                
                let trail_amount = current_price * Decimal::from_f64_retain(*trailing_percentage / 100.0).unwrap_or(Decimal::ZERO);
                match stop.position_side {
                    PositionSide::Long => Ok(current_price - trail_amount),
                    PositionSide::Short => Ok(current_price + trail_amount),
                }
            },
            
            TrailingStrategy::ATR { atr_multiplier, min_trailing_amount, max_trailing_amount, .. } => {
                let atr_value = Decimal::from_f64_retain(price_tracker.volatility_metrics.atr * atr_multiplier)
                    .unwrap_or(Decimal::ZERO);
                let trail_amount = atr_value.max(*min_trailing_amount).min(*max_trailing_amount);
                
                match stop.position_side {
                    PositionSide::Long => Ok(current_price - trail_amount),
                    PositionSide::Short => Ok(current_price + trail_amount),
                }
            },
            
            TrailingStrategy::VolatilityAdjusted { 
                base_percentage, 
                volatility_multiplier, 
                min_percentage, 
                max_percentage, 
                .. 
            } => {
                let volatility_factor = price_tracker.volatility_metrics.realized_volatility * volatility_multiplier;
                let adjusted_percentage = (base_percentage + volatility_factor)
                    .max(*min_percentage)
                    .min(*max_percentage);
                    
                let trail_amount = current_price * Decimal::from_f64_retain(adjusted_percentage / 100.0).unwrap_or(Decimal::ZERO);
                match stop.position_side {
                    PositionSide::Long => Ok(current_price - trail_amount),
                    PositionSide::Short => Ok(current_price + trail_amount),
                }
            },
            
            TrailingStrategy::Adaptive { 
                base_percentage, 
                trend_factor, 
                volume_factor, 
                volatility_factor, 
                sentiment_factor 
            } => {
                // Calculate adaptive percentage based on market conditions
                let mut adaptive_percentage = *base_percentage;
                
                // Adjust for trend
                match price_tracker.technical_levels.trend_direction {
                    TrendDirection::Bullish => adaptive_percentage *= 1.0 + (trend_factor * price_tracker.technical_levels.trend_strength),
                    TrendDirection::Bearish => adaptive_percentage *= 1.0 - (trend_factor * price_tracker.technical_levels.trend_strength),
                    TrendDirection::Sideways => {}, // No adjustment
                }
                
                // Adjust for volatility
                adaptive_percentage *= 1.0 + (volatility_factor * price_tracker.volatility_metrics.realized_volatility);
                
                let trail_amount = current_price * Decimal::from_f64_retain(adaptive_percentage / 100.0).unwrap_or(Decimal::ZERO);
                match stop.position_side {
                    PositionSide::Long => Ok(current_price - trail_amount),
                    PositionSide::Short => Ok(current_price + trail_amount),
                }
            },
            
            TrailingStrategy::TimeBased { 
                initial_percentage, 
                final_percentage, 
                time_period, 
                curve_type 
            } => {
                let elapsed = Utc::now() - stop.created_at;
                let progress = (elapsed.num_milliseconds() as f64) / (time_period.num_milliseconds() as f64);
                let progress = progress.min(1.0).max(0.0);
                
                let current_percentage = match curve_type {
                    TimeCurveType::Linear => {
                        initial_percentage + (final_percentage - initial_percentage) * progress
                    },
                    TimeCurveType::Exponential => {
                        initial_percentage + (final_percentage - initial_percentage) * progress.powf(2.0)
                    },
                    TimeCurveType::Logarithmic => {
                        initial_percentage + (final_percentage - initial_percentage) * progress.ln().abs()
                    },
                    TimeCurveType::StepFunction(steps) => {
                        let step_index = (progress * steps.len() as f64) as usize;
                        steps.get(step_index.min(steps.len() - 1)).cloned().unwrap_or(*initial_percentage)
                    },
                };
                
                let trail_amount = current_price * Decimal::from_f64_retain(current_percentage / 100.0).unwrap_or(Decimal::ZERO);
                match stop.position_side {
                    PositionSide::Long => Ok(current_price - trail_amount),
                    PositionSide::Short => Ok(current_price + trail_amount),
                }
            },
            
            TrailingStrategy::TechnicalLevels { 
                support_resistance_buffer, 
                level_strength_threshold, 
                max_trail_percentage 
            } => {
                // Find nearest support/resistance level
                let levels = match stop.position_side {
                    PositionSide::Long => &price_tracker.technical_levels.support_levels,
                    PositionSide::Short => &price_tracker.technical_levels.resistance_levels,
                };
                
                let nearest_level = levels.iter()
                    .filter(|level| level.strength >= *level_strength_threshold)
                    .min_by(|a, b| {
                        let a_distance = (a.price - current_price).abs();
                        let b_distance = (b.price - current_price).abs();
                        a_distance.cmp(&b_distance)
                    });
                
                if let Some(level) = nearest_level {
                    let buffer_amount = level.price * Decimal::from_f64_retain(*support_resistance_buffer / 100.0).unwrap_or(Decimal::ZERO);
                    match stop.position_side {
                        PositionSide::Long => Ok(level.price - buffer_amount),
                        PositionSide::Short => Ok(level.price + buffer_amount),
                    }
                } else {
                    // Fallback to percentage-based trailing
                    let trail_amount = current_price * Decimal::from_f64_retain(*max_trail_percentage / 100.0).unwrap_or(Decimal::ZERO);
                    match stop.position_side {
                        PositionSide::Long => Ok(current_price - trail_amount),
                        PositionSide::Short => Ok(current_price + trail_amount),
                    }
                }
            },
        }
    }
    
    /// Update stop price and order
    async fn update_stop_price(&self, stop_id: &str, new_stop_price: Decimal, current_price: Decimal) -> Result<()> {
        let mut stops = self.active_trailing_stops.write().await;
        if let Some(stop) = stops.get_mut(stop_id) {
            let old_stop_price = stop.current_stop_price;
            stop.current_stop_price = new_stop_price;
            stop.last_updated = Utc::now();
            
            // Update highest/lowest price tracking
            match stop.position_side {
                PositionSide::Long => {
                    if current_price > stop.highest_price {
                        stop.highest_price = current_price;
                    }
                },
                PositionSide::Short => {
                    if current_price < stop.lowest_price {
                        stop.lowest_price = current_price;
                    }
                },
            }
            
            stop.performance_metrics.total_adjustments += 1;
            let adjustment_size = (new_stop_price - old_stop_price).abs();
            stop.performance_metrics.average_adjustment_size = 
                (stop.performance_metrics.average_adjustment_size * Decimal::from(stop.performance_metrics.total_adjustments - 1) + adjustment_size) 
                / Decimal::from(stop.performance_metrics.total_adjustments);
            
            debug!("🔄 Updated trailing stop {} from {} to {}", 
                stop_id, old_stop_price, new_stop_price);
        }
        
        Ok(())
    }
    
    // Additional helper methods for price tracking, performance metrics, etc.
    async fn calculate_initial_stop_price(
        &self,
        strategy: &TrailingStrategy,
        position_side: &PositionSide,
        entry_price: Decimal,
    ) -> Result<Decimal> {
        match strategy {
            TrailingStrategy::FixedAmount { trailing_amount, .. } => {
                match position_side {
                    PositionSide::Long => Ok(entry_price - trailing_amount),
                    PositionSide::Short => Ok(entry_price + trailing_amount),
                }
            },
            TrailingStrategy::Percentage { trailing_percentage, .. } => {
                let trail_amount = entry_price * Decimal::from_f64_retain(*trailing_percentage / 100.0).unwrap_or(Decimal::ZERO);
                match position_side {
                    PositionSide::Long => Ok(entry_price - trail_amount),
                    PositionSide::Short => Ok(entry_price + trail_amount),
                }
            },
            _ => {
                // For other strategies, use a default 5% stop
                let trail_amount = entry_price * Decimal::from_f64_retain(0.05).unwrap();
                match position_side {
                    PositionSide::Long => Ok(entry_price - trail_amount),
                    PositionSide::Short => Ok(entry_price + trail_amount),
                }
            }
        }
    }
    
    async fn get_current_price(&self, token_mint: &str) -> Result<Decimal> {
        let prices = self.price_client
            .get_prices(vec![token_mint.to_string()])
            .await?;
            
        let price_data = prices.prices
            .get(token_mint)
            .ok_or_else(|| BotError::trading(format!("Price data not found for token {}", token_mint)))?;
        
        Ok(Decimal::from_f64_retain(price_data.usd_price).unwrap_or(Decimal::ZERO))
    }
    
    async fn get_price_tracker(&self, token_mint: &str) -> Result<PriceTracker> {
        let trackers = self.price_tracker.read().await;
        trackers.get(token_mint)
            .cloned()
            .ok_or_else(|| BotError::not_found(format!("Price tracker not found for token {}", token_mint)).into())
    }
    
    async fn initialize_price_tracking(&self, token_mint: &str) -> Result<()> {
        let mut trackers = self.price_tracker.write().await;
        if !trackers.contains_key(token_mint) {
            let current_price = self.get_current_price(token_mint).await?;
            
            let tracker = PriceTracker {
                token_mint: token_mint.to_string(),
                current_price,
                price_history: Vec::new(),
                volatility_metrics: VolatilityMetrics {
                    atr: 0.0,
                    realized_volatility: 0.0,
                    implied_volatility: None,
                    bollinger_width: 0.0,
                },
                technical_levels: TechnicalLevels {
                    support_levels: Vec::new(),
                    resistance_levels: Vec::new(),
                    trend_direction: TrendDirection::Sideways,
                    trend_strength: 0.5,
                },
                last_updated: Utc::now(),
            };
            
            trackers.insert(token_mint.to_string(), tracker);
        }
        Ok(())
    }
    
    async fn update_price_tracking(&self) -> Result<()> {
        // Implementation would update price data and calculate technical indicators
        Ok(())
    }
    
    async fn update_performance_metrics(&self, stop_id: &str, current_price: Decimal) -> Result<()> {
        // Implementation would update performance tracking
        Ok(())
    }
    
    async fn check_risk_controls(&self, stop: &TrailingStopState, current_price: Decimal) -> Result<()> {
        // Implementation would check various risk controls
        Ok(())
    }
    
    /// Cancel a trailing stop
    pub async fn cancel_trailing_stop(&self, stop_id: &str) -> Result<bool> {
        let mut stops = self.active_trailing_stops.write().await;
        if let Some(mut stop) = stops.remove(stop_id) {
            stop.status = TrailingStopStatus::Cancelled;
            
            // Cancel underlying order
            self.order_manager.cancel_order(&stop.order_id).await?;
            
            info!("🔄 Cancelled trailing stop: {}", stop_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get trailing stop performance
    pub async fn get_trailing_stop_performance(&self, stop_id: &str) -> Option<TrailingPerformanceMetrics> {
        let stops = self.active_trailing_stops.read().await;
        stops.get(stop_id).map(|stop| stop.performance_metrics.clone())
    }
}

impl TrailingPerformanceMetrics {
    fn new() -> Self {
        Self {
            max_favorable_excursion: Decimal::ZERO,
            max_adverse_excursion: Decimal::ZERO,
            total_adjustments: 0,
            average_adjustment_size: Decimal::ZERO,
            time_in_profit: Duration::zero(),
            time_in_loss: Duration::zero(),
            efficiency_ratio: 0.0,
        }
    }
}

/// Helper functions for creating common trailing stop configurations
impl TrailingStopState {
    /// Create a simple percentage-based trailing stop
    pub fn create_percentage_trailing(
        user_id: i64,
        token_mint: String,
        position_side: PositionSide,
        entry_price: Decimal,
        position_size: Decimal,
        trailing_percentage: f64,
    ) -> Self {
        Self {
            stop_id: uuid::Uuid::new_v4().to_string(),
            order_id: String::new(), // Will be set after order creation
            user_id,
            token_mint,
            strategy: TrailingStrategy::Percentage {
                trailing_percentage,
                activation_threshold: None,
            },
            position_side,
            current_stop_price: entry_price, // Will be calculated
            highest_price: entry_price,
            lowest_price: entry_price,
            entry_price,
            position_size,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            status: TrailingStopStatus::Active,
            performance_metrics: TrailingPerformanceMetrics::new(),
            risk_controls: TrailingRiskControls::default(),
        }
    }
}

impl Default for TrailingRiskControls {
    fn default() -> Self {
        Self {
            max_loss_percentage: 20.0, // 20% max loss
            profit_lock_percentage: None,
            time_stop: None,
            volume_threshold: None,
            correlation_stop: None,
            drawdown_limit: Some(15.0), // 15% drawdown limit
        }
    }
}