use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use crate::errors::{BotError, Result};
use crate::api::jupiter_v6::{JupiterV6Client, QuoteRequestV6, SwapMode};
use crate::api::jupiter_price_v3::JupiterPriceV3Client;
use crate::telemetry::TelemetryService;
use crate::db::Database;

/// DCA (Dollar Cost Averaging) engine for automated trading
#[derive(Clone)]
pub struct DCAEngine {
    jupiter_client: Arc<JupiterV6Client>,
    price_client: Arc<JupiterPriceV3Client>,
    database: Arc<Database>,
    telemetry: Option<Arc<TelemetryService>>,
    strategies: Arc<RwLock<HashMap<String, DCAStrategy>>>,
    execution_history: Arc<RwLock<HashMap<String, Vec<DCAExecution>>>>,
}

/// DCA strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DCAStrategy {
    pub strategy_id: String,
    pub user_id: i64,
    pub name: String,
    pub input_token: String,      // Token to sell (usually USDC/SOL)
    pub output_token: String,     // Token to buy
    pub total_amount: Decimal,    // Total amount to invest
    pub interval: DCAInterval,    // How often to execute
    pub amount_per_execution: Decimal, // Fixed amount per execution
    pub strategy_type: DCAStrategyType,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub next_execution: DateTime<Utc>,
    pub status: DCAStatus,
    pub execution_count: u32,
    pub max_executions: Option<u32>,
    pub end_date: Option<DateTime<Utc>>,
    pub risk_parameters: RiskParameters,
    pub advanced_config: AdvancedDCAConfig,
}

/// DCA execution intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DCAInterval {
    Minutes(u32),
    Hourly,
    Daily,
    Weekly,
    Biweekly,
    Monthly,
    Custom { cron_expression: String },
}

/// Different DCA strategy types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DCAStrategyType {
    /// Fixed amount each interval
    Fixed,
    /// Amount varies based on price momentum
    ValueAveraging,
    /// More aggressive buying during dips
    BuyTheDip { dip_threshold: f64 },
    /// Reduce buying during pumps
    MomentumBased { rsi_threshold: f64 },
    /// Grid-based DCA with multiple price levels
    Grid { levels: Vec<GridLevel> },
    /// AI-enhanced DCA using market signals
    AIEnhanced { confidence_threshold: f64 },
}

/// Grid level for grid-based DCA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLevel {
    pub price_level: Decimal,
    pub allocation_percentage: f64,
    pub is_active: bool,
}

/// DCA strategy status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DCAStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

/// Risk management parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParameters {
    pub max_slippage_bps: u16,           // Maximum slippage in basis points
    pub stop_loss_percentage: Option<f64>, // Stop DCA if token drops X%
    pub take_profit_percentage: Option<f64>, // Stop DCA if token rises X%
    pub max_drawdown_percentage: f64,     // Maximum portfolio drawdown
    pub volatility_threshold: f64,        // Pause if volatility too high
    pub liquidity_threshold: Decimal,     // Minimum liquidity required
}

/// Advanced DCA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedDCAConfig {
    pub dynamic_sizing: bool,             // Adjust size based on market conditions
    pub fear_greed_factor: bool,         // Use fear & greed index
    pub social_sentiment_factor: bool,    // Consider social sentiment
    pub technical_analysis: bool,         // Use technical indicators
    pub correlation_analysis: bool,       // Consider market correlation
    pub time_decay_factor: Option<f64>,  // Reduce amounts over time
    pub acceleration_factor: Option<f64>, // Increase amounts during opportunities
}

/// DCA execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DCAExecution {
    pub execution_id: String,
    pub strategy_id: String,
    pub executed_at: DateTime<Utc>,
    pub input_amount: Decimal,
    pub output_amount: Decimal,
    pub price_at_execution: Decimal,
    pub slippage_bps: u16,
    pub gas_fees: Decimal,
    pub transaction_signature: Option<String>,
    pub execution_reason: ExecutionReason,
    pub market_conditions: MarketConditions,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Reason for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionReason {
    ScheduledInterval,
    PriceDip,
    MomentumSignal,
    GridLevel,
    AISignal,
    ManualTrigger,
}

/// Market conditions at execution time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub token_price: Decimal,
    pub volume_24h: Option<u64>,
    pub volatility: Option<f64>,
    pub rsi: Option<f64>,
    pub fear_greed_index: Option<u8>,
    pub social_sentiment: Option<f64>,
    pub market_cap_rank: Option<u32>,
}

/// DCA performance metrics
#[derive(Debug, Clone, Serialize)]
pub struct DCAPerformance {
    pub strategy_id: String,
    pub total_invested: Decimal,
    pub total_tokens_acquired: Decimal,
    pub average_entry_price: Decimal,
    pub current_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_percentage: f64,
    pub total_fees_paid: Decimal,
    pub execution_success_rate: f64,
    pub sharpe_ratio: Option<f64>,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub time_to_break_even: Option<Duration>,
    pub risk_adjusted_return: Option<f64>,
}

impl DCAEngine {
    /// Create new DCA engine
    pub fn new(
        jupiter_client: Arc<JupiterV6Client>,
        price_client: Arc<JupiterPriceV3Client>,
        database: Arc<Database>,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("💰 Initializing DCA trading engine");
        
        Self {
            jupiter_client,
            price_client,
            database,
            telemetry,
            strategies: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create a new DCA strategy
    pub async fn create_strategy(&self, mut strategy: DCAStrategy) -> Result<String> {
        // Validate strategy
        self.validate_strategy(&strategy).await?;
        
        // Set next execution time
        strategy.next_execution = self.calculate_next_execution(&strategy.interval)?;
        strategy.status = DCAStatus::Active;
        strategy.created_at = Utc::now();
        
        // Store in database
        self.store_strategy(&strategy).await?;
        
        // Add to active strategies
        let strategy_id = strategy.strategy_id.clone();
        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy_id.clone(), strategy);
        
        info!("💰 Created DCA strategy: {} for {}/{}", 
            strategy_id, strategy.input_token, strategy.output_token);
        
        Ok(strategy_id)
    }
    
    /// Execute pending DCA strategies
    pub async fn execute_pending_strategies(&self) -> Result<u32> {
        let now = Utc::now();
        let mut executed_count = 0;
        
        // Get all active strategies
        let strategies: Vec<DCAStrategy> = {
            let strategies_lock = self.strategies.read().await;
            strategies_lock.values()
                .filter(|s| s.status == DCAStatus::Active && s.next_execution <= now)
                .cloned()
                .collect()
        };
        
        for strategy in strategies {
            match self.execute_strategy(&strategy).await {
                Ok(_) => {
                    executed_count += 1;
                    self.update_strategy_next_execution(&strategy.strategy_id).await?;
                },
                Err(e) => {
                    error!("💰 Failed to execute DCA strategy {}: {}", strategy.strategy_id, e);
                    self.handle_execution_failure(&strategy.strategy_id, &e.to_string()).await?;
                }
            }
        }
        
        if executed_count > 0 {
            info!("💰 Executed {} DCA strategies", executed_count);
        }
        
        Ok(executed_count)
    }
    
    /// Execute a specific DCA strategy
    pub async fn execute_strategy(&self, strategy: &DCAStrategy) -> Result<DCAExecution> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_trading_span("dca_execution", Some(&format!("{}/{}", strategy.input_token, strategy.output_token)))
        );
        
        debug!("💰 Executing DCA strategy: {}", strategy.strategy_id);
        
        // Get current market conditions
        let market_conditions = self.get_market_conditions(&strategy.output_token).await?;
        
        // Check risk parameters
        if !self.check_risk_parameters(strategy, &market_conditions).await? {
            warn!("💰 Risk parameters exceeded for strategy {}, skipping execution", strategy.strategy_id);
            return Err(BotError::trading("Risk parameters exceeded".to_string()).into());
        }
        
        // Calculate execution amount based on strategy type
        let execution_amount = self.calculate_execution_amount(strategy, &market_conditions).await?;
        
        if execution_amount <= Decimal::ZERO {
            warn!("💰 Calculated execution amount is zero for strategy {}", strategy.strategy_id);
            return Err(BotError::trading("Execution amount is zero".to_string()).into());
        }
        
        // Get quote from Jupiter
        let quote_request = QuoteRequestV6 {
            input_mint: strategy.input_token.clone(),
            output_mint: strategy.output_token.clone(),
            amount: execution_amount.to_u64().unwrap_or(0),
            slippage_bps: strategy.risk_parameters.max_slippage_bps,
            swap_mode: Some(SwapMode::ExactIn),
            dexes: None,
            exclude_dexes: None,
            max_accounts: Some(32),
            quote_mint: None,
            minimize_slippage: Some(true),
            only_direct_routes: Some(false),
        };
        
        let quote = self.jupiter_client.get_quote(quote_request).await?;
        
        // Calculate slippage and validate
        let expected_output = execution_amount * market_conditions.token_price;
        let actual_output = Decimal::from_str(&quote.out_amount)
            .map_err(|e| BotError::parsing(format!("Invalid output amount: {}", e)))?;
        
        let slippage = ((expected_output - actual_output) / expected_output * Decimal::from(10000))
            .to_u16().unwrap_or(u16::MAX);
            
        if slippage > strategy.risk_parameters.max_slippage_bps {
            return Err(BotError::trading(format!(
                "Slippage {} exceeds maximum {}", slippage, strategy.risk_parameters.max_slippage_bps
            )).into());
        }
        
        // Execute the trade (this would integrate with your existing swap logic)
        // For now, we'll simulate execution
        let execution = DCAExecution {
            execution_id: uuid::Uuid::new_v4().to_string(),
            strategy_id: strategy.strategy_id.clone(),
            executed_at: Utc::now(),
            input_amount: execution_amount,
            output_amount: actual_output,
            price_at_execution: market_conditions.token_price,
            slippage_bps: slippage,
            gas_fees: Decimal::from_str("0.001").unwrap(), // Estimated
            transaction_signature: None, // Would be filled after actual execution
            execution_reason: self.determine_execution_reason(strategy, &market_conditions),
            market_conditions: market_conditions.clone(),
            success: true,
            error_message: None,
        };
        
        // Store execution record
        self.store_execution(&execution).await?;
        
        // Update execution history in memory
        let mut history = self.execution_history.write().await;
        history.entry(strategy.strategy_id.clone())
            .or_insert_with(Vec::new)
            .push(execution.clone());
        
        info!("💰 DCA execution completed: {} {} -> {} {}", 
            execution.input_amount, strategy.input_token,
            execution.output_amount, strategy.output_token);
        
        Ok(execution)
    }
    
    /// Get DCA strategy performance metrics
    pub async fn get_strategy_performance(&self, strategy_id: &str) -> Result<DCAPerformance> {
        let strategy = self.get_strategy(strategy_id).await?;
        let executions = self.get_strategy_executions(strategy_id).await?;
        
        if executions.is_empty() {
            return Err(BotError::trading("No executions found for strategy".to_string()).into());
        }
        
        // Calculate basic metrics
        let total_invested = executions.iter().map(|e| e.input_amount).sum();
        let total_tokens_acquired = executions.iter().map(|e| e.output_amount).sum();
        let average_entry_price = if total_tokens_acquired > Decimal::ZERO {
            total_invested / total_tokens_acquired
        } else {
            Decimal::ZERO
        };
        
        // Get current token price
        let prices = self.price_client
            .get_prices(vec![strategy.output_token.clone()])
            .await?;
        let current_price = prices.prices
            .get(&strategy.output_token)
            .map(|p| Decimal::from_f64_retain(p.usd_price).unwrap_or(Decimal::ZERO))
            .unwrap_or(Decimal::ZERO);
        
        let current_value = total_tokens_acquired * current_price;
        let unrealized_pnl = current_value - total_invested;
        let unrealized_pnl_percentage = if total_invested > Decimal::ZERO {
            (unrealized_pnl / total_invested * Decimal::from(100)).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };
        
        let total_fees_paid = executions.iter().map(|e| e.gas_fees).sum();
        let execution_success_rate = executions.iter().filter(|e| e.success).count() as f64 / executions.len() as f64 * 100.0;
        
        // Calculate advanced metrics
        let sharpe_ratio = self.calculate_sharpe_ratio(&executions, current_price);
        let max_drawdown = self.calculate_max_drawdown(&executions, current_price);
        let win_rate = self.calculate_win_rate(&executions);
        let time_to_break_even = self.calculate_time_to_break_even(&executions, current_price);
        let risk_adjusted_return = self.calculate_risk_adjusted_return(&executions, current_price);
        
        Ok(DCAPerformance {
            strategy_id: strategy_id.to_string(),
            total_invested,
            total_tokens_acquired,
            average_entry_price,
            current_value,
            unrealized_pnl,
            unrealized_pnl_percentage,
            total_fees_paid,
            execution_success_rate,
            sharpe_ratio,
            max_drawdown,
            win_rate,
            time_to_break_even,
            risk_adjusted_return,
        })
    }
    
    /// Validate DCA strategy parameters
    async fn validate_strategy(&self, strategy: &DCAStrategy) -> Result<()> {
        // Basic validation
        if strategy.total_amount <= Decimal::ZERO {
            return Err(BotError::validation("Total amount must be positive".to_string()).into());
        }
        
        if strategy.amount_per_execution <= Decimal::ZERO {
            return Err(BotError::validation("Amount per execution must be positive".to_string()).into());
        }
        
        if strategy.amount_per_execution > strategy.total_amount {
            return Err(BotError::validation("Amount per execution cannot exceed total amount".to_string()).into());
        }
        
        // Validate tokens exist
        let token_prices = self.price_client
            .get_prices(vec![strategy.input_token.clone(), strategy.output_token.clone()])
            .await?;
            
        if !token_prices.prices.contains_key(&strategy.input_token) {
            return Err(BotError::validation(format!("Input token {} not found", strategy.input_token)).into());
        }
        
        if !token_prices.prices.contains_key(&strategy.output_token) {
            return Err(BotError::validation(format!("Output token {} not found", strategy.output_token)).into());
        }
        
        // Validate risk parameters
        if strategy.risk_parameters.max_slippage_bps > 1000 { // 10%
            return Err(BotError::validation("Maximum slippage cannot exceed 10%".to_string()).into());
        }
        
        Ok(())
    }
    
    /// Calculate next execution time based on interval
    fn calculate_next_execution(&self, interval: &DCAInterval) -> Result<DateTime<Utc>> {
        let now = Utc::now();
        
        let next = match interval {
            DCAInterval::Minutes(m) => now + Duration::minutes(*m as i64),
            DCAInterval::Hourly => now + Duration::hours(1),
            DCAInterval::Daily => now + Duration::days(1),
            DCAInterval::Weekly => now + Duration::weeks(1),
            DCAInterval::Biweekly => now + Duration::weeks(2),
            DCAInterval::Monthly => now + Duration::days(30), // Approximate
            DCAInterval::Custom { cron_expression: _ } => {
                // Would implement cron parsing here
                now + Duration::hours(1) // Fallback
            }
        };
        
        Ok(next)
    }
    
    /// Get current market conditions for a token
    async fn get_market_conditions(&self, token_mint: &str) -> Result<MarketConditions> {
        let prices = self.price_client
            .get_prices(vec![token_mint.to_string()])
            .await?;
            
        let price_data = prices.prices
            .get(token_mint)
            .ok_or_else(|| BotError::trading(format!("Price data not found for token {}", token_mint)))?;
        
        Ok(MarketConditions {
            token_price: Decimal::from_f64_retain(price_data.usd_price)
                .unwrap_or(Decimal::ZERO),
            volume_24h: price_data.volume_24h,
            volatility: None, // Would calculate from historical data
            rsi: None,        // Would calculate from price history
            fear_greed_index: None, // Would fetch from external API
            social_sentiment: None, // Would analyze social media
            market_cap_rank: None,  // Would get from token analytics
        })
    }
    
    /// Check if risk parameters allow execution
    async fn check_risk_parameters(&self, strategy: &DCAStrategy, conditions: &MarketConditions) -> Result<bool> {
        // Check volatility threshold
        if let Some(volatility) = conditions.volatility {
            if volatility > strategy.risk_parameters.volatility_threshold {
                return Ok(false);
            }
        }
        
        // Check liquidity threshold
        if let Some(volume_24h) = conditions.volume_24h {
            if Decimal::from(volume_24h) < strategy.risk_parameters.liquidity_threshold {
                return Ok(false);
            }
        }
        
        // Additional risk checks would go here
        
        Ok(true)
    }
    
    /// Calculate execution amount based on strategy type
    async fn calculate_execution_amount(&self, strategy: &DCAStrategy, conditions: &MarketConditions) -> Result<Decimal> {
        let base_amount = strategy.amount_per_execution;
        
        match &strategy.strategy_type {
            DCAStrategyType::Fixed => Ok(base_amount),
            
            DCAStrategyType::ValueAveraging => {
                // Implement value averaging logic
                Ok(base_amount)
            },
            
            DCAStrategyType::BuyTheDip { dip_threshold } => {
                // Increase amount during dips
                if let Some(volume_24h) = conditions.volume_24h {
                    // Simple implementation - would be more sophisticated
                    let volume_factor = if volume_24h > 1000000 { 1.5 } else { 1.0 };
                    Ok(base_amount * Decimal::from_f64_retain(volume_factor).unwrap_or(Decimal::ONE))
                } else {
                    Ok(base_amount)
                }
            },
            
            DCAStrategyType::MomentumBased { rsi_threshold: _ } => {
                // Adjust based on RSI
                Ok(base_amount)
            },
            
            DCAStrategyType::Grid { levels: _ } => {
                // Grid-based calculation
                Ok(base_amount)
            },
            
            DCAStrategyType::AIEnhanced { confidence_threshold: _ } => {
                // AI-based amount calculation
                Ok(base_amount)
            },
        }
    }
    
    /// Additional helper methods would be implemented here for:
    /// - Storing strategies in database
    /// - Loading strategies from database
    /// - Calculating performance metrics
    /// - Risk management
    /// - Market analysis
    
    async fn store_strategy(&self, _strategy: &DCAStrategy) -> Result<()> {
        // Database storage implementation
        Ok(())
    }
    
    async fn store_execution(&self, _execution: &DCAExecution) -> Result<()> {
        // Database storage implementation
        Ok(())
    }
    
    async fn get_strategy(&self, strategy_id: &str) -> Result<DCAStrategy> {
        let strategies = self.strategies.read().await;
        strategies.get(strategy_id)
            .cloned()
            .ok_or_else(|| BotError::not_found(format!("Strategy {} not found", strategy_id)).into())
    }
    
    async fn get_strategy_executions(&self, strategy_id: &str) -> Result<Vec<DCAExecution>> {
        let history = self.execution_history.read().await;
        Ok(history.get(strategy_id).cloned().unwrap_or_default())
    }
    
    async fn update_strategy_next_execution(&self, strategy_id: &str) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        if let Some(strategy) = strategies.get_mut(strategy_id) {
            strategy.next_execution = self.calculate_next_execution(&strategy.interval)?;
            strategy.execution_count += 1;
            
            // Check if strategy should be completed
            if let Some(max_executions) = strategy.max_executions {
                if strategy.execution_count >= max_executions {
                    strategy.status = DCAStatus::Completed;
                }
            }
        }
        Ok(())
    }
    
    async fn handle_execution_failure(&self, strategy_id: &str, error: &str) -> Result<()> {
        warn!("💰 DCA execution failed for strategy {}: {}", strategy_id, error);
        // Could implement retry logic, strategy pausing, etc.
        Ok(())
    }
    
    fn determine_execution_reason(&self, _strategy: &DCAStrategy, _conditions: &MarketConditions) -> ExecutionReason {
        ExecutionReason::ScheduledInterval // Simplified
    }
    
    // Performance calculation helpers
    fn calculate_sharpe_ratio(&self, _executions: &[DCAExecution], _current_price: Decimal) -> Option<f64> {
        None // Would implement proper Sharpe ratio calculation
    }
    
    fn calculate_max_drawdown(&self, _executions: &[DCAExecution], _current_price: Decimal) -> f64 {
        0.0 // Would implement proper drawdown calculation
    }
    
    fn calculate_win_rate(&self, executions: &[DCAExecution]) -> f64 {
        if executions.is_empty() {
            return 0.0;
        }
        
        executions.iter().filter(|e| e.success).count() as f64 / executions.len() as f64 * 100.0
    }
    
    fn calculate_time_to_break_even(&self, _executions: &[DCAExecution], _current_price: Decimal) -> Option<Duration> {
        None // Would implement break-even calculation
    }
    
    fn calculate_risk_adjusted_return(&self, _executions: &[DCAExecution], _current_price: Decimal) -> Option<f64> {
        None // Would implement risk-adjusted return calculation
    }
}

/// Helper functions for creating DCA strategies
impl DCAStrategy {
    /// Create a simple daily DCA strategy
    pub fn create_daily_dca(
        user_id: i64,
        name: String,
        input_token: String,
        output_token: String,
        total_amount: Decimal,
        daily_amount: Decimal,
    ) -> Self {
        Self {
            strategy_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            name,
            input_token,
            output_token,
            total_amount,
            interval: DCAInterval::Daily,
            amount_per_execution: daily_amount,
            strategy_type: DCAStrategyType::Fixed,
            created_at: Utc::now(),
            started_at: None,
            next_execution: Utc::now() + Duration::days(1),
            status: DCAStatus::Active,
            execution_count: 0,
            max_executions: Some((total_amount / daily_amount).to_u32().unwrap_or(100)),
            end_date: None,
            risk_parameters: RiskParameters::default(),
            advanced_config: AdvancedDCAConfig::default(),
        }
    }
}

impl Default for RiskParameters {
    fn default() -> Self {
        Self {
            max_slippage_bps: 100, // 1%
            stop_loss_percentage: None,
            take_profit_percentage: None,
            max_drawdown_percentage: 20.0, // 20%
            volatility_threshold: 50.0,     // 50%
            liquidity_threshold: Decimal::from(10000), // $10k
        }
    }
}

impl Default for AdvancedDCAConfig {
    fn default() -> Self {
        Self {
            dynamic_sizing: false,
            fear_greed_factor: false,
            social_sentiment_factor: false,
            technical_analysis: false,
            correlation_analysis: false,
            time_decay_factor: None,
            acceleration_factor: None,
        }
    }
}