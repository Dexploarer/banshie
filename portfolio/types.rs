use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Portfolio data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub wallet_address: String,
    pub total_value_usd: f64,
    pub total_value_sol: f64,
    pub holdings: Vec<TokenHolding>,
    pub performance: PortfolioPerformance,
    pub last_updated: DateTime<Utc>,
}

/// Individual token holding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolding {
    pub mint_address: String,
    pub symbol: String,
    pub name: String,
    pub balance: f64,
    pub decimals: u8,
    pub value_usd: f64,
    pub value_sol: f64,
    pub price_usd: f64,
    pub price_change_24h: Option<f64>,
    pub logo_uri: Option<String>,
    pub is_verified: bool,
}

/// Portfolio performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPerformance {
    pub total_pnl_usd: f64,
    pub total_pnl_sol: f64,
    pub pnl_percentage: f64,
    pub pnl_24h_usd: f64,
    pub pnl_24h_percentage: f64,
    pub best_performer: Option<TokenPerformance>,
    pub worst_performer: Option<TokenPerformance>,
    pub win_rate: f64,
    pub largest_holding: Option<String>,
}

/// Token performance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPerformance {
    pub symbol: String,
    pub pnl_usd: f64,
    pub pnl_percentage: f64,
}

/// Raw balance response from RPC
#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    pub jsonrpc: String,
    pub result: BalanceResult,
    pub id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResult {
    pub value: u64,
}

/// Token accounts response from RPC
#[derive(Debug, Clone, Deserialize)]
pub struct TokenAccountsResponse {
    pub jsonrpc: String,
    pub result: TokenAccountsResult,
    pub id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenAccountsResult {
    pub value: Vec<TokenAccount>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenAccount {
    pub account: TokenAccountData,
    pub pubkey: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenAccountData {
    pub data: TokenAccountInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenAccountInfo {
    pub parsed: ParsedAccountData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParsedAccountData {
    pub info: TokenInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    #[serde(rename = "tokenAmount")]
    pub token_amount: TokenAmount,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenAmount {
    pub amount: String,
    pub decimals: u8,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
}

/// Jupiter price response
#[derive(Debug, Clone, Deserialize)]
pub struct JupiterPriceResponse {
    pub data: HashMap<String, TokenPriceData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenPriceData {
    pub id: String,
    pub price: f64,
    #[serde(rename = "extraInfo")]
    pub extra_info: Option<PriceExtraInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceExtraInfo {
    #[serde(rename = "quotedPrice")]
    pub quoted_price: Option<QuotedPrice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuotedPrice {
    #[serde(rename = "buyPrice")]
    pub buy_price: Option<f64>,
    #[serde(rename = "sellPrice")]
    pub sell_price: Option<f64>,
}

/// Token metadata from Jupiter/Token List
#[derive(Debug, Clone, Deserialize)]
pub struct TokenMetadata {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    pub verified: Option<bool>,
}

/// Portfolio summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub total_value_usd: f64,
    pub total_holdings: usize,
    pub top_holdings: Vec<HoldingSummary>,
    pub performance_24h: f64,
    pub performance_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingSummary {
    pub symbol: String,
    pub balance: f64,
    pub value_usd: f64,
    pub percentage: f64,
}

/// Historical portfolio data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioHistory {
    pub wallet_address: String,
    pub snapshots: Vec<PortfolioSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_value_usd: f64,
    pub total_value_sol: f64,
    pub holding_count: usize,
}