use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnalysis {
    pub token_address: String,
    pub token_symbol: String,
    pub token_name: String,
    pub is_honeypot: bool,
    pub can_sell: bool,
    pub can_buy: bool,
    pub liquidity_locked: bool,
    pub liquidity_lock_duration: Option<i64>, // Days
    pub freeze_authority: Option<String>,
    pub mint_authority: Option<String>,
    pub update_authority: Option<String>,
    pub creator_address: Option<String>,
    pub creator_balance_percent: f64,
    pub top_holders: Vec<HolderInfo>,
    pub holder_count: u32,
    pub risk_score: u8, // 0-100, higher is safer
    pub risk_level: RiskLevel,
    pub warnings: Vec<SecurityWarning>,
    pub passed_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub recommendations: Vec<String>,
    pub token_age_hours: f64,
    pub total_supply: f64,
    pub circulating_supply: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub transaction_count_24h: u32,
    pub unique_wallets_24h: u32,
    pub metadata: TokenMetadata,
    pub analysis_timestamp: DateTime<Utc>,
    pub data_sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolderInfo {
    pub address: String,
    pub balance: f64,
    pub percentage: f64,
    pub is_locked: bool,
    pub is_creator: bool,
    pub is_exchange: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    VeryLow,   // 80-100 score
    Low,       // 60-79 score
    Medium,    // 40-59 score
    High,      // 20-39 score
    VeryHigh,  // 0-19 score
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    pub severity: WarningSeverity,
    pub category: WarningCategory,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WarningSeverity {
    Critical, // Block trading
    High,     // Strong warning
    Medium,   // Caution advised
    Low,      // Informational
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WarningCategory {
    Honeypot,
    Liquidity,
    Ownership,
    Distribution,
    Contract,
    Trading,
    Age,
    Social,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub description: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub logo_uri: Option<String>,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityInfo {
    pub pool_address: String,
    pub dex_name: String,
    pub liquidity_usd: f64,
    pub liquidity_token: f64,
    pub liquidity_quote: f64,
    pub is_locked: bool,
    pub lock_duration: Option<i64>,
    pub lock_platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAnalysis {
    pub has_freeze_authority: bool,
    pub has_mint_authority: bool,
    pub has_update_authority: bool,
    pub is_mutable: bool,
    pub has_transfer_fee: bool,
    pub transfer_fee_percent: Option<f64>,
    pub has_burn_function: bool,
    pub has_pause_function: bool,
    pub is_proxy: bool,
    pub is_upgradeable: bool,
    pub token_program: TokenProgram,
    pub extensions: Vec<TokenExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenProgram {
    SPL,
    SPL2022,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenExtension {
    TransferFee,
    InterestBearing,
    NonTransferable,
    PermanentDelegate,
    TransferHook,
    MetadataPointer,
    ConfidentialTransfers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingAnalysis {
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub max_buy_amount: Option<f64>,
    pub max_sell_amount: Option<f64>,
    pub max_wallet_amount: Option<f64>,
    pub can_trade: bool,
    pub trading_enabled_at: Option<DateTime<Utc>>,
    pub price_impact_1_sol: f64,
    pub price_impact_10_sol: f64,
    pub price_impact_100_sol: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCheckResult {
    pub check_name: String,
    pub passed: bool,
    pub score_impact: i8, // How much this affects the overall score
    pub details: String,
}

impl SecurityAnalysis {
    /// Calculate overall risk level from score
    pub fn calculate_risk_level(score: u8) -> RiskLevel {
        match score {
            80..=100 => RiskLevel::VeryLow,
            60..=79 => RiskLevel::Low,
            40..=59 => RiskLevel::Medium,
            20..=39 => RiskLevel::High,
            _ => RiskLevel::VeryHigh,
        }
    }
    
    /// Generate recommendation based on risk level
    pub fn generate_recommendation(&self) -> String {
        match self.risk_level {
            RiskLevel::VeryLow => {
                "✅ LOW RISK - Token appears safe for trading".to_string()
            }
            RiskLevel::Low => {
                "✅ ACCEPTABLE RISK - Proceed with normal caution".to_string()
            }
            RiskLevel::Medium => {
                "⚠️ MODERATE RISK - Consider smaller position size".to_string()
            }
            RiskLevel::High => {
                "⛔ HIGH RISK - Not recommended unless you understand the risks".to_string()
            }
            RiskLevel::VeryHigh => {
                "🚫 EXTREME RISK - Avoid this token".to_string()
            }
        }
    }
    
    /// Get risk emoji
    pub fn get_risk_emoji(&self) -> &str {
        match self.risk_level {
            RiskLevel::VeryLow => "✅",
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🟠",
            RiskLevel::VeryHigh => "🔴",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSecurityRequest {
    pub token_addresses: Vec<String>,
    pub include_liquidity: bool,
    pub include_holders: bool,
    pub include_trading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSecurityResponse {
    pub analyses: Vec<SecurityAnalysis>,
    pub failed_tokens: Vec<(String, String)>, // (address, error)
    pub timestamp: DateTime<Utc>,
}