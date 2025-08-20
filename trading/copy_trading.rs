use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::db::Database;
use crate::errors::BotError;
use crate::trading::{TradingEngineHandle, TradeResult};
use crate::wallet::WalletManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyTradingConfig {
    pub master_wallet: String,
    pub master_user_id: i64,
    pub master_username: String,
    pub follower_user_id: i64,
    pub follower_wallet: String,
    pub allocation_percent: f64, // Percentage of master's position size to copy
    pub max_position_sol: f64,   // Maximum SOL per trade
    pub min_position_sol: f64,   // Minimum SOL per trade
    pub copy_buys: bool,
    pub copy_sells: bool,
    pub auto_stop_loss: bool,
    pub stop_loss_percent: f64,
    pub auto_take_profit: bool,
    pub take_profit_percent: f64,
    pub slippage_tolerance: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub performance: CopyPerformance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyPerformance {
    pub total_trades_copied: u32,
    pub successful_trades: u32,
    pub failed_trades: u32,
    pub total_profit_sol: f64,
    pub total_profit_percent: f64,
    pub fees_paid_sol: f64,
    pub last_copied_trade: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterTrader {
    pub user_id: i64,
    pub username: String,
    pub wallet_address: String,
    pub copy_fee_percent: f64,
    pub min_copy_amount_sol: f64,
    pub total_followers: u32,
    pub total_volume_copied_sol: f64,
    pub fees_earned_sol: f64,
    pub is_accepting_followers: bool,
    pub performance_7d: f64,
    pub performance_30d: f64,
    pub win_rate: f64,
    pub avg_trade_size_sol: f64,
    pub trading_style: TradingStyle,
    pub restrictions: Vec<CopyRestriction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradingStyle {
    Scalper,      // High frequency, small profits
    SwingTrader,  // Medium term positions
    DayTrader,    // Intraday positions
    Sniper,       // New token launches
    Fundamental,  // Long term value investing
    Mixed,        // Various strategies
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CopyRestriction {
    MinBalance(f64),           // Minimum SOL balance required
    MaxFollowers(u32),          // Maximum number of followers
    RequireVerification,        // KYC/verification required
    RestrictedTokens(Vec<String>), // Tokens not to copy
    TradingHoursOnly,          // Only copy during specific hours
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyTradeExecution {
    pub execution_id: String,
    pub master_trade_id: String,
    pub master_user_id: i64,
    pub follower_user_id: i64,
    pub token_address: String,
    pub token_symbol: String,
    pub trade_type: CopyTradeType,
    pub master_amount_sol: f64,
    pub copied_amount_sol: f64,
    pub master_price: f64,
    pub execution_price: f64,
    pub slippage_percent: f64,
    pub fee_paid_sol: f64,
    pub status: CopyTradeStatus,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CopyTradeType {
    Buy,
    Sell,
    StopLoss,
    TakeProfit,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CopyTradeStatus {
    Pending,
    Executing,
    Success,
    Failed,
    PartialFill,
    Cancelled,
}

/// Manages copy trading relationships and executions
pub struct CopyTradingManager {
    db: Arc<Database>,
    trading_engine: TradingEngineHandle,
    wallet_manager: Arc<WalletManager>,
    relationships: Arc<RwLock<HashMap<i64, Vec<CopyTradingConfig>>>>, // follower_id -> configs
    master_traders: Arc<RwLock<HashMap<i64, MasterTrader>>>,
    active_positions: Arc<RwLock<HashMap<String, Vec<Position>>>>, // token -> positions
    execution_history: Arc<RwLock<Vec<CopyTradeExecution>>>,
}

#[derive(Debug, Clone)]
struct Position {
    user_id: i64,
    token_address: String,
    amount: f64,
    entry_price: f64,
    current_price: f64,
    pnl_percent: f64,
    opened_at: DateTime<Utc>,
}

impl CopyTradingManager {
    pub fn new(
        db: Arc<Database>,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
    ) -> Self {
        Self {
            db,
            trading_engine,
            wallet_manager,
            relationships: Arc::new(RwLock::new(HashMap::new())),
            master_traders: Arc::new(RwLock::new(HashMap::new())),
            active_positions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start following a master trader
    pub async fn start_following(
        &self,
        follower_user_id: i64,
        master_identifier: &str, // Can be user_id, username, or wallet
        allocation_percent: f64,
        max_position_sol: f64,
    ) -> Result<CopyTradingConfig> {
        info!("User {} starting to follow {}", follower_user_id, master_identifier);
        
        // Validate allocation
        if allocation_percent <= 0.0 || allocation_percent > 100.0 {
            return Err(BotError::validation("Allocation must be between 1-100%").into());
        }
        
        // Find master trader
        let master = self.find_master_trader(master_identifier).await?;
        
        // Check if master is accepting followers
        if !master.is_accepting_followers {
            return Err(BotError::validation("This trader is not accepting new followers").into());
        }
        
        // Check follower balance
        let follower_balance = self.get_user_balance(follower_user_id).await?;
        if follower_balance < master.min_copy_amount_sol {
            return Err(BotError::validation(format!(
                "Minimum balance required: {} SOL",
                master.min_copy_amount_sol
            )).into());
        }
        
        // Check for existing relationship
        let mut relationships = self.relationships.write().await;
        let follower_configs = relationships.entry(follower_user_id).or_insert_with(Vec::new);
        
        if follower_configs.iter().any(|c| c.master_user_id == master.user_id) {
            return Err(BotError::validation("Already following this trader").into());
        }
        
        // Create new copy trading config
        let config = CopyTradingConfig {
            master_wallet: master.wallet_address.clone(),
            master_user_id: master.user_id,
            master_username: master.username.clone(),
            follower_user_id,
            follower_wallet: format!("wallet_{}", follower_user_id), // Would get from wallet manager
            allocation_percent,
            max_position_sol,
            min_position_sol: master.min_copy_amount_sol,
            copy_buys: true,
            copy_sells: true,
            auto_stop_loss: true,
            stop_loss_percent: 15.0, // Default 15% stop loss
            auto_take_profit: true,
            take_profit_percent: 50.0, // Default 50% take profit
            slippage_tolerance: 2.0, // 2% slippage tolerance
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            performance: CopyPerformance {
                total_trades_copied: 0,
                successful_trades: 0,
                failed_trades: 0,
                total_profit_sol: 0.0,
                total_profit_percent: 0.0,
                fees_paid_sol: 0.0,
                last_copied_trade: None,
            },
        };
        
        // Save to database (in production)
        // self.db.save_copy_config(&config).await?;
        
        // Add to active relationships
        follower_configs.push(config.clone());
        
        // Update master's follower count
        let mut masters = self.master_traders.write().await;
        if let Some(master_mut) = masters.get_mut(&master.user_id) {
            master_mut.total_followers += 1;
        }
        
        info!(
            "User {} now following {} with {}% allocation",
            follower_user_id, master.username, allocation_percent
        );
        
        Ok(config)
    }

    /// Stop following a master trader
    pub async fn stop_following(
        &self,
        follower_user_id: i64,
        master_user_id: i64,
    ) -> Result<()> {
        let mut relationships = self.relationships.write().await;
        
        if let Some(configs) = relationships.get_mut(&follower_user_id) {
            let initial_len = configs.len();
            configs.retain(|c| c.master_user_id != master_user_id);
            
            if configs.len() < initial_len {
                // Update master's follower count
                let mut masters = self.master_traders.write().await;
                if let Some(master) = masters.get_mut(&master_user_id) {
                    master.total_followers = master.total_followers.saturating_sub(1);
                }
                
                info!("User {} stopped following master {}", follower_user_id, master_user_id);
                Ok(())
            } else {
                Err(BotError::validation("Not following this trader").into())
            }
        } else {
            Err(BotError::validation("No active copy trading relationships").into())
        }
    }

    /// Execute a copy trade when master makes a trade
    pub async fn execute_copy_trade(
        &self,
        master_user_id: i64,
        token_address: &str,
        token_symbol: &str,
        trade_type: CopyTradeType,
        master_amount_sol: f64,
        master_price: f64,
    ) -> Result<Vec<CopyTradeExecution>> {
        info!(
            "Master {} executing {:?} trade: {} {} for {} SOL",
            master_user_id, trade_type, token_symbol, token_address, master_amount_sol
        );
        
        let mut executions = Vec::new();
        
        // Get all followers of this master
        let relationships = self.relationships.read().await;
        let followers: Vec<CopyTradingConfig> = relationships
            .values()
            .flatten()
            .filter(|c| c.master_user_id == master_user_id && c.enabled)
            .cloned()
            .collect();
        
        drop(relationships); // Release lock early
        
        info!("Found {} active followers for master {}", followers.len(), master_user_id);
        
        // Get master trader info for fee calculation
        let masters = self.master_traders.read().await;
        let master = masters.get(&master_user_id);
        let copy_fee_percent = master.map(|m| m.copy_fee_percent).unwrap_or(5.0);
        drop(masters);
        
        // Execute copy trades for each follower
        for config in followers {
            // Check if this trade type should be copied
            match trade_type {
                CopyTradeType::Buy if !config.copy_buys => continue,
                CopyTradeType::Sell if !config.copy_sells => continue,
                _ => {}
            }
            
            // Calculate copy amount based on allocation
            let mut copy_amount = master_amount_sol * (config.allocation_percent / 100.0);
            
            // Apply position limits
            copy_amount = copy_amount.min(config.max_position_sol);
            copy_amount = copy_amount.max(config.min_position_sol);
            
            // Check follower balance
            match self.get_user_balance(config.follower_user_id).await {
                Ok(balance) => {
                    if balance < copy_amount * 1.05 { // Include 5% buffer for fees/slippage
                        warn!(
                            "Follower {} has insufficient balance: {} SOL < {} SOL required",
                            config.follower_user_id, balance, copy_amount * 1.05
                        );
                        
                        executions.push(CopyTradeExecution {
                            execution_id: uuid::Uuid::new_v4().to_string(),
                            master_trade_id: format!("{}_{}", master_user_id, Utc::now().timestamp()),
                            master_user_id,
                            follower_user_id: config.follower_user_id,
                            token_address: token_address.to_string(),
                            token_symbol: token_symbol.to_string(),
                            trade_type: trade_type.clone(),
                            master_amount_sol,
                            copied_amount_sol: copy_amount,
                            master_price,
                            execution_price: 0.0,
                            slippage_percent: 0.0,
                            fee_paid_sol: 0.0,
                            status: CopyTradeStatus::Failed,
                            error_message: Some("Insufficient balance".to_string()),
                            timestamp: Utc::now(),
                        });
                        continue;
                    }
                }
                Err(e) => {
                    error!("Failed to get balance for follower {}: {}", config.follower_user_id, e);
                    continue;
                }
            }
            
            // Execute the trade
            let execution = self.execute_follower_trade(
                &config,
                token_address,
                token_symbol,
                trade_type.clone(),
                copy_amount,
                master_price,
                copy_fee_percent,
            ).await;
            
            executions.push(execution);
        }
        
        // Store execution history
        let mut history = self.execution_history.write().await;
        history.extend(executions.clone());
        
        // Keep only last 1000 executions
        if history.len() > 1000 {
            history.drain(0..history.len() - 1000);
        }
        
        Ok(executions)
    }

    /// Execute individual follower trade
    async fn execute_follower_trade(
        &self,
        config: &CopyTradingConfig,
        token_address: &str,
        token_symbol: &str,
        trade_type: CopyTradeType,
        amount_sol: f64,
        master_price: f64,
        fee_percent: f64,
    ) -> CopyTradeExecution {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let fee_amount = amount_sol * (fee_percent / 100.0);
        let trade_amount = amount_sol - fee_amount;
        
        debug!(
            "Executing copy trade for follower {}: {} SOL in {} (fee: {} SOL)",
            config.follower_user_id, trade_amount, token_symbol, fee_amount
        );
        
        // Execute via trading engine
        // In production, this would use the actual trading engine message format
        // For now, simulate the trade execution
        let result = match trade_type {
            CopyTradeType::Buy | CopyTradeType::Sell => {
                // Simulate trade execution
                Ok(TradeResult {
                    success: true,
                    amount: trade_amount,
                    price: master_price * (1.0 + (rand::thread_rng().gen::<f64>() - 0.5) * 0.02), // Simulate ±1% slippage
                    signature: Some(format!("sim_tx_{}", uuid::Uuid::new_v4())),
                    message: format!("Copy trade executed: {} {} SOL of {}", 
                        if matches!(trade_type, CopyTradeType::Buy) { "Bought" } else { "Sold" },
                        trade_amount, token_symbol),
                })
            }
            _ => {
                Ok(TradeResult {
                    success: false,
                    amount: 0.0,
                    price: 0.0,
                    signature: None,
                    message: "Trade type not implemented".to_string(),
                })
            }
        };
        
        match result {
            Ok(trade_result) => {
                let slippage = ((trade_result.price - master_price) / master_price * 100.0).abs();
                
                // Update config performance
                // This would be persisted to database in production
                
                CopyTradeExecution {
                    execution_id,
                    master_trade_id: format!("{}_{}", config.master_user_id, Utc::now().timestamp()),
                    master_user_id: config.master_user_id,
                    follower_user_id: config.follower_user_id,
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    trade_type,
                    master_amount_sol: amount_sol,
                    copied_amount_sol: trade_amount,
                    master_price,
                    execution_price: trade_result.price,
                    slippage_percent: slippage,
                    fee_paid_sol: fee_amount,
                    status: if trade_result.success {
                        CopyTradeStatus::Success
                    } else {
                        CopyTradeStatus::Failed
                    },
                    error_message: if !trade_result.success {
                        Some(trade_result.message)
                    } else {
                        None
                    },
                    timestamp: Utc::now(),
                }
            }
            Err(e) => CopyTradeExecution {
                execution_id,
                master_trade_id: format!("{}_{}", config.master_user_id, Utc::now().timestamp()),
                master_user_id: config.master_user_id,
                follower_user_id: config.follower_user_id,
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                trade_type,
                master_amount_sol: amount_sol,
                copied_amount_sol: trade_amount,
                master_price,
                execution_price: 0.0,
                slippage_percent: 0.0,
                fee_paid_sol: 0.0,
                status: CopyTradeStatus::Failed,
                error_message: Some(e.to_string()),
                timestamp: Utc::now(),
            },
        }
    }

    /// Monitor positions for stop loss and take profit
    pub async fn monitor_positions(&self) -> Result<()> {
        let positions = self.active_positions.read().await;
        let relationships = self.relationships.read().await;
        
        for (token_address, token_positions) in positions.iter() {
            for position in token_positions {
                // Check if user has copy trading config with auto SL/TP
                if let Some(configs) = relationships.get(&position.user_id) {
                    for config in configs {
                        if !config.enabled {
                            continue;
                        }
                        
                        let pnl_percent = position.pnl_percent;
                        
                        // Check stop loss
                        if config.auto_stop_loss && pnl_percent <= -config.stop_loss_percent {
                            warn!(
                                "Stop loss triggered for user {} on {}: {:.2}% loss",
                                position.user_id, token_address, pnl_percent
                            );
                            
                            // Execute stop loss sell
                            let _ = self.execute_follower_trade(
                                config,
                                token_address,
                                "TOKEN", // Would get actual symbol
                                CopyTradeType::StopLoss,
                                position.amount,
                                position.current_price,
                                0.0, // No fee on stop loss
                            ).await;
                        }
                        
                        // Check take profit
                        if config.auto_take_profit && pnl_percent >= config.take_profit_percent {
                            info!(
                                "Take profit triggered for user {} on {}: {:.2}% profit",
                                position.user_id, token_address, pnl_percent
                            );
                            
                            // Execute take profit sell
                            let _ = self.execute_follower_trade(
                                config,
                                token_address,
                                "TOKEN", // Would get actual symbol
                                CopyTradeType::TakeProfit,
                                position.amount,
                                position.current_price,
                                0.0, // No fee on take profit
                            ).await;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Get copy trading statistics for a user
    pub async fn get_user_stats(
        &self,
        user_id: i64,
    ) -> Result<(Vec<CopyTradingConfig>, Vec<CopyTradeExecution>)> {
        let relationships = self.relationships.read().await;
        let configs = relationships.get(&user_id).cloned().unwrap_or_default();
        
        let history = self.execution_history.read().await;
        let user_executions: Vec<CopyTradeExecution> = history
            .iter()
            .filter(|e| e.follower_user_id == user_id)
            .take(50) // Last 50 executions
            .cloned()
            .collect();
        
        Ok((configs, user_executions))
    }

    /// Get available master traders
    pub async fn get_available_masters(&self, limit: usize) -> Result<Vec<MasterTrader>> {
        // In production, this would query from database
        // For now, return sample data
        Ok(vec![
            MasterTrader {
                user_id: 1001,
                username: "AlphaTrader".to_string(),
                wallet_address: "Alpha123...xyz".to_string(),
                copy_fee_percent: 10.0,
                min_copy_amount_sol: 1.0,
                total_followers: 234,
                total_volume_copied_sol: 45678.0,
                fees_earned_sol: 4567.8,
                is_accepting_followers: true,
                performance_7d: 45.2,
                performance_30d: 127.8,
                win_rate: 78.5,
                avg_trade_size_sol: 25.0,
                trading_style: TradingStyle::Sniper,
                restrictions: vec![CopyRestriction::MinBalance(5.0)],
            },
            MasterTrader {
                user_id: 1002,
                username: "DiamondHands".to_string(),
                wallet_address: "Diamond456...abc".to_string(),
                copy_fee_percent: 8.0,
                min_copy_amount_sol: 0.5,
                total_followers: 189,
                total_volume_copied_sol: 32100.0,
                fees_earned_sol: 2568.0,
                is_accepting_followers: true,
                performance_7d: 22.1,
                performance_30d: 89.3,
                win_rate: 71.2,
                avg_trade_size_sol: 15.0,
                trading_style: TradingStyle::SwingTrader,
                restrictions: vec![],
            },
            MasterTrader {
                user_id: 1003,
                username: "ScalpMaster".to_string(),
                wallet_address: "Scalp789...def".to_string(),
                copy_fee_percent: 15.0,
                min_copy_amount_sol: 2.0,
                total_followers: 567,
                total_volume_copied_sol: 123456.0,
                fees_earned_sol: 18518.4,
                is_accepting_followers: true,
                performance_7d: 18.9,
                performance_30d: 76.2,
                win_rate: 82.1,
                avg_trade_size_sol: 5.0,
                trading_style: TradingStyle::Scalper,
                restrictions: vec![
                    CopyRestriction::MinBalance(10.0),
                    CopyRestriction::MaxFollowers(1000),
                ],
            },
        ])
    }

    /// Find master trader by identifier
    async fn find_master_trader(&self, identifier: &str) -> Result<MasterTrader> {
        // Try to parse as user ID
        if let Ok(user_id) = identifier.parse::<i64>() {
            let masters = self.master_traders.read().await;
            if let Some(master) = masters.get(&user_id) {
                return Ok(master.clone());
            }
        }
        
        // Search by username or wallet
        let masters = self.get_available_masters(100).await?;
        masters
            .into_iter()
            .find(|m| {
                m.username.eq_ignore_ascii_case(identifier) ||
                m.wallet_address.starts_with(identifier)
            })
            .ok_or_else(|| BotError::validation("Master trader not found").into())
    }

    /// Get user balance (mock implementation)
    async fn get_user_balance(&self, user_id: i64) -> Result<f64> {
        // In production, this would query actual wallet balance
        Ok(10.0 + (user_id as f64 * 0.1)) // Mock balance
    }

    /// Format copy trading config for display
    pub fn format_config(config: &CopyTradingConfig) -> String {
        format!(
            "📋 **Copy Trading Configuration**\n\
            Master: {} (@{})\n\
            Allocation: {}%\n\
            Max Position: {} SOL\n\
            Min Position: {} SOL\n\
            Copy Buys: {}\n\
            Copy Sells: {}\n\
            Auto Stop Loss: {} ({}%)\n\
            Auto Take Profit: {} ({}%)\n\
            Status: {}\n\
            \n\
            📊 **Performance**\n\
            Total Trades: {}\n\
            Success Rate: {:.1}%\n\
            Total Profit: {:.2} SOL ({:.1}%)\n\
            Fees Paid: {:.2} SOL",
            config.master_username,
            config.master_user_id,
            config.allocation_percent,
            config.max_position_sol,
            config.min_position_sol,
            if config.copy_buys { "✅" } else { "❌" },
            if config.copy_sells { "✅" } else { "❌" },
            if config.auto_stop_loss { "✅" } else { "❌" },
            config.stop_loss_percent,
            if config.auto_take_profit { "✅" } else { "❌" },
            config.take_profit_percent,
            if config.enabled { "🟢 Active" } else { "🔴 Paused" },
            config.performance.total_trades_copied,
            if config.performance.total_trades_copied > 0 {
                (config.performance.successful_trades as f64 / 
                 config.performance.total_trades_copied as f64) * 100.0
            } else { 0.0 },
            config.performance.total_profit_sol,
            config.performance.total_profit_percent,
            config.performance.fees_paid_sol
        )
    }

    /// Format master trader info for display
    pub fn format_master_trader(master: &MasterTrader) -> String {
        let style_emoji = match master.trading_style {
            TradingStyle::Scalper => "⚡",
            TradingStyle::SwingTrader => "🌊",
            TradingStyle::DayTrader => "☀️",
            TradingStyle::Sniper => "🎯",
            TradingStyle::Fundamental => "📊",
            TradingStyle::Mixed => "🎨",
        };
        
        let mut message = format!(
            "👤 **{}** {} @{}\n\
            Wallet: {}\n\
            \n\
            📈 **Performance**\n\
            7 Day: {:+.1}%\n\
            30 Day: {:+.1}%\n\
            Win Rate: {:.1}%\n\
            Avg Trade: {} SOL\n\
            \n\
            👥 **Copy Trading**\n\
            Followers: {}\n\
            Volume Copied: {:.0} SOL\n\
            Copy Fee: {}%\n\
            Min Copy: {} SOL\n\
            Status: {}\n",
            master.username,
            style_emoji,
            master.user_id,
            master.wallet_address,
            master.performance_7d,
            master.performance_30d,
            master.win_rate,
            master.avg_trade_size_sol,
            master.total_followers,
            master.total_volume_copied_sol,
            master.copy_fee_percent,
            master.min_copy_amount_sol,
            if master.is_accepting_followers { "✅ Accepting" } else { "❌ Full" }
        );
        
        if !master.restrictions.is_empty() {
            message.push_str("\n⚠️ **Requirements:**\n");
            for restriction in &master.restrictions {
                match restriction {
                    CopyRestriction::MinBalance(amount) => {
                        message.push_str(&format!("• Minimum {} SOL balance\n", amount));
                    }
                    CopyRestriction::MaxFollowers(max) => {
                        message.push_str(&format!("• Limited to {} followers\n", max));
                    }
                    CopyRestriction::RequireVerification => {
                        message.push_str("• Verification required\n");
                    }
                    CopyRestriction::RestrictedTokens(tokens) => {
                        message.push_str(&format!("• Excludes {} tokens\n", tokens.len()));
                    }
                    CopyRestriction::TradingHoursOnly => {
                        message.push_str("• Trading hours only\n");
                    }
                }
            }
        }
        
        message
    }
}