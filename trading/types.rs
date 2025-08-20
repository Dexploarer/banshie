use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use indexmap::IndexMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResult {
    pub tx_signature: String,
    pub tokens_received: f64,
    pub tokens_sold: f64,
    pub sol_received: f64,
    pub amount_sol: f64,
    pub price: f64,
    pub rebate_earned: f64,
    pub pnl_percentage: f64,
    // Add timestamp for efficient ordering
    pub timestamp: chrono::DateTime<chrono::Utc>,
    // Add trade type for better categorization
    pub trade_type: TradeType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TradeType {
    Buy,
    Sell,
    Swap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub sol: f64,
    pub usdc: f64,
    pub total_usd_value: f64,
    // Add token balances using efficient map
    pub token_balances: BTreeMap<String, TokenBalance>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: String,
    pub amount: f64,
    pub value_usd: f64,
    pub price_per_token: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token: String,
    pub symbol: String,
    pub mint: String,
    pub amount: f64,
    pub value_usd: f64,
    pub pnl_percentage: f64,
    pub average_buy_price: f64,
    pub current_price: f64,
    // Add sorting key for efficient position management
    pub sort_key: u64,
    // Add last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.mint == other.mint
    }
}

impl Eq for Position {}

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mint.hash(state);
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value_usd.partial_cmp(&other.value_usd).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Optimized portfolio structure using BTreeMap for sorted access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub positions: BTreeMap<String, Position>, // Key: mint address
    pub total_value_usd: f64,
    pub total_pnl_percentage: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Portfolio {
    pub fn new() -> Self {
        Self {
            positions: BTreeMap::new(),
            total_value_usd: 0.0,
            total_pnl_percentage: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }
    
    pub fn add_position(&mut self, position: Position) {
        self.positions.insert(position.mint.clone(), position);
        self.recalculate_totals();
    }
    
    pub fn remove_position(&mut self, mint: &str) -> Option<Position> {
        let result = self.positions.remove(mint);
        self.recalculate_totals();
        result
    }
    
    pub fn get_top_positions(&self, limit: usize) -> Vec<&Position> {
        let mut positions: Vec<_> = self.positions.values().collect();
        positions.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap_or(std::cmp::Ordering::Equal));
        positions.into_iter().take(limit).collect()
    }
    
    fn recalculate_totals(&mut self) {
        self.total_value_usd = self.positions.values().map(|p| p.value_usd).sum();
        
        let total_investment: f64 = self.positions.values()
            .map(|p| p.amount * p.average_buy_price)
            .sum();
            
        self.total_pnl_percentage = if total_investment > 0.0 {
            ((self.total_value_usd - total_investment) / total_investment) * 100.0
        } else {
            0.0
        };
        
        self.last_updated = chrono::Utc::now();
    }
}

/// Optimized trade history using IndexMap for insertion order + fast lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHistory {
    pub trades: IndexMap<String, TradeResult>, // Key: tx_signature
    pub total_trades: usize,
    pub total_volume_sol: f64,
    pub total_rebates_sol: f64,
}

impl TradeHistory {
    pub fn new() -> Self {
        Self {
            trades: IndexMap::with_capacity(1000), // Pre-allocate for 1000 trades
            total_trades: 0,
            total_volume_sol: 0.0,
            total_rebates_sol: 0.0,
        }
    }
    
    pub fn add_trade(&mut self, trade: TradeResult) {
        self.total_volume_sol += trade.sol_received.abs() + (trade.tokens_received * trade.price);
        self.total_rebates_sol += trade.rebate_earned;
        self.total_trades += 1;
        
        self.trades.insert(trade.tx_signature.clone(), trade);
        
        // Keep only last 1000 trades for memory efficiency
        if self.trades.len() > 1000 {
            self.trades.shift_remove_index(0);
        }
    }
    
    pub fn get_recent_trades(&self, limit: usize) -> Vec<&TradeResult> {
        self.trades.values().rev().take(limit).collect()
    }
    
    pub fn get_trades_by_type(&self, trade_type: TradeType) -> Vec<&TradeResult> {
        self.trades.values().filter(|t| t.trade_type == trade_type).collect()
    }
}

/// Token-2022 restrictions for trading logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRestrictions {
    pub is_non_transferable: bool,
    pub has_transfer_fees: bool,
    pub has_transfer_hook: bool,
    pub requires_memo: bool,
}