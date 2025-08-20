use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;
use crate::constants::{MIN_TRADE_SOL, MAX_TRADE_SOL, MAX_SLIPPAGE_BPS, MAX_SESSION_DURATION_MINUTES};
use crate::errors::{BotError, TradingError, WalletError, Result};

// String interner for frequently used tokens
static TOKEN_INTERNER: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(50);
    // Common Solana tokens
    map.insert("SOL", "So11111111111111111111111111111111111112");
    map.insert("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    map.insert("USDT", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
    map.insert("RAY", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R");
    map.insert("SRM", "SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt");
    map.insert("ORCA", "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE");
    map
});

pub struct Validator;

impl Validator {
    /// Validate a Solana public key
    pub fn validate_pubkey(address: &str) -> Result<Pubkey> {
        if address.is_empty() {
            return Err(WalletError::InvalidPublicKey.into());
        }
        
        if address.len() < 32 || address.len() > 44 {
            return Err(WalletError::InvalidPublicKey.into());
        }
        
        Pubkey::from_str(address)
            .map_err(|_| WalletError::InvalidPublicKey.into())
    }
    
    /// Validate a trading amount with comprehensive bounds checking
    pub fn validate_trade_amount(amount: f64, max_allowed: f64) -> Result<()> {
        if amount.is_nan() || amount.is_infinite() {
            return Err(TradingError::InvalidAmount {
                message: "Amount is not a valid number".to_string()
            }.into());
        }
        
        if amount <= 0.0 {
            return Err(TradingError::InvalidAmount {
                message: "Amount must be positive".to_string()
            }.into());
        }
        
        if amount < MIN_TRADE_SOL {
            return Err(TradingError::InvalidAmount {
                message: format!("Amount must be at least {} SOL", MIN_TRADE_SOL)
            }.into());
        }
        
        if amount > max_allowed {
            return Err(TradingError::AmountExceedsMaximum {
                amount,
                maximum: max_allowed
            }.into());
        }
        
        if amount > MAX_TRADE_SOL {
            return Err(TradingError::AmountExceedsMaximum {
                amount,
                maximum: MAX_TRADE_SOL
            }.into());
        }
        
        Ok(())
    }
    
    /// Validate a percentage value with proper bounds
    pub fn validate_percentage(percentage: f64) -> Result<()> {
        if percentage.is_nan() || percentage.is_infinite() {
            return Err(TradingError::InvalidPercentage {
                percentage
            }.into());
        }
        
        if percentage <= 0.0 || percentage > 100.0 {
            return Err(TradingError::InvalidPercentage {
                percentage
            }.into());
        }
        
        Ok(())
    }
    
    /// Validate slippage basis points
    pub fn validate_slippage(slippage_bps: u16) -> Result<()> {
        if slippage_bps > MAX_SLIPPAGE_BPS {
            return Err(TradingError::SlippageExceeded.into());
        }
        
        Ok(())
    }
    
    /// Validate a transaction signature with proper format checking
    pub fn validate_signature(signature: &str) -> Result<()> {
        if signature.is_empty() {
            return Err(BotError::ValidationError("Transaction signature cannot be empty".to_string()));
        }
        
        if signature.len() < 86 || signature.len() > 90 {
            return Err(BotError::ValidationError("Invalid transaction signature length".to_string()));
        }
        
        // Check if it's valid base58
        bs58::decode(signature).into_vec()
            .map_err(|_| BotError::ValidationError("Invalid transaction signature encoding".to_string()))?;
        
        Ok(())
    }
    
    /// Validate session duration with bounds
    pub fn validate_session_duration(minutes: i64) -> Result<()> {
        if minutes <= 0 {
            return Err(BotError::ValidationError("Session duration must be positive".to_string()));
        }
        
        if minutes > MAX_SESSION_DURATION_MINUTES {
            return Err(BotError::ValidationError(
                format!("Session duration cannot exceed {} minutes", MAX_SESSION_DURATION_MINUTES)
            ));
        }
        
        Ok(())
    }
    
    /// Get token mint address from interner or validate symbol
    pub fn get_token_mint_optimized(symbol: &str) -> Cow<str> {
        // Check interner first for common tokens
        if let Some(&mint) = TOKEN_INTERNER.get(&symbol.to_uppercase().as_str()) {
            return Cow::Borrowed(mint);
        }
        
        // For unknown tokens, return the symbol
        Cow::Owned(symbol.to_uppercase())
    }
    
    /// Validate token symbol format
    pub fn validate_token_symbol(symbol: &str) -> Result<String> {
        let cleaned = Self::sanitize_input(symbol).to_uppercase();
        
        if cleaned.is_empty() {
            return Err(BotError::ValidationError("Token symbol cannot be empty".into()));
        }
        
        if cleaned.len() > 20 {
            return Err(BotError::ValidationError("Token symbol too long".into()));
        }
        
        // Check if symbol contains only alphanumeric characters
        if !cleaned.chars().all(|c| c.is_alphanumeric()) {
            return Err(BotError::ValidationError("Token symbol must contain only alphanumeric characters".into()));
        }
        
        Ok(cleaned)
    }
    
    /// Validate priority fee amount
    pub fn validate_priority_fee(fee_lamports: u64) -> Result<()> {
        const MAX_PRIORITY_FEE: u64 = 10_000_000; // 0.01 SOL
        
        if fee_lamports > MAX_PRIORITY_FEE {
            return Err(BotError::ValidationError(
                format!("Priority fee cannot exceed {} lamports", MAX_PRIORITY_FEE)
            ));
        }
        
        Ok(())
    }
    
    /// Validate user ID format
    pub fn validate_user_id(user_id: &str) -> Result<()> {
        if user_id.is_empty() {
            return Err(BotError::ValidationError("User ID cannot be empty".to_string()));
        }
        
        if user_id.len() > 20 {
            return Err(BotError::ValidationError("User ID too long".to_string()));
        }
        
        // Check if user_id contains only digits (Telegram user IDs are numeric)
        if !user_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(BotError::ValidationError("Invalid user ID format".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate wallet balance for trade
    pub fn validate_sufficient_balance(balance: f64, required: f64) -> Result<()> {
        if balance < required {
            return Err(TradingError::InsufficientBalance {
                required,
                available: balance
            }.into());
        }
        
        Ok(())
    }
    
    /// Validate HTTP URL format
    pub fn validate_url(url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(BotError::ValidationError("URL cannot be empty".into()));
        }
        
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(BotError::ValidationError("URL must start with http:// or https://".into()));
        }
        
        Ok(())
    }
    
    /// Sanitize user input to prevent injection attacks (optimized)
    pub fn sanitize_input(input: &str) -> Cow<str> {
        // Check if input needs sanitization
        let needs_sanitization = input.len() > 100 || 
            input.chars().any(|c| !(c.is_alphanumeric() || c.is_whitespace() || c == '.' || c == '-' || c == '_'));
        
        if !needs_sanitization {
            return Cow::Borrowed(input.trim());
        }
        
        let sanitized: String = input.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == '-' || *c == '_')
            .take(100) // Limit length
            .collect();
            
        Cow::Owned(sanitized.trim().to_string())
    }
    
    /// Advanced input sanitization for command arguments
    pub fn sanitize_command_args(input: &str) -> Result<String> {
        let sanitized = input.trim();
        
        if sanitized.is_empty() {
            return Err(BotError::ValidationError("Command arguments cannot be empty".to_string()));
        }
        
        if sanitized.len() > 200 {
            return Err(BotError::ValidationError("Command arguments too long".to_string()));
        }
        
        // Remove potentially dangerous characters
        let cleaned: String = sanitized.chars()
            .filter(|c| {
                c.is_alphanumeric() || 
                c.is_whitespace() || 
                matches!(*c, '.' | '-' | '_' | '/' | ':')
            })
            .collect();
        
        Ok(cleaned)
    }
    
    /// Validate rate limiting parameters
    pub fn validate_rate_limit(requests: u32, time_window_minutes: u32) -> Result<()> {
        if requests == 0 {
            return Err(BotError::ValidationError("Request count must be positive".to_string()));
        }
        
        if time_window_minutes == 0 {
            return Err(BotError::ValidationError("Time window must be positive".to_string()));
        }
        
        if requests > 1000 {
            return Err(BotError::ValidationError("Too many requests per time window".to_string()));
        }
        
        Ok(())
    }
}

/// Type-safe wrapper for validated amounts
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ValidatedAmount(f64);

impl ValidatedAmount {
    pub fn new(amount: f64, max_allowed: f64) -> Result<Self> {
        Validator::validate_trade_amount(amount, max_allowed)?;
        Ok(Self(amount))
    }
    
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Type-safe wrapper for validated percentages
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ValidatedPercentage(f64);

impl ValidatedPercentage {
    pub fn new(percentage: f64) -> Result<Self> {
        Validator::validate_percentage(percentage)?;
        Ok(Self(percentage))
    }
    
    pub fn value(&self) -> f64 {
        self.0
    }
    
    pub fn as_decimal(&self) -> f64 {
        self.0 / 100.0
    }
}

/// Type-safe wrapper for validated token symbols
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedTokenSymbol(String);

impl ValidatedTokenSymbol {
    pub fn new(symbol: &str) -> Result<Self> {
        let validated = Validator::validate_token_symbol(symbol)?;
        Ok(Self(validated))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Type-safe wrapper for validated user IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedUserId(String);

impl ValidatedUserId {
    pub fn new(user_id: &str) -> Result<Self> {
        Validator::validate_user_id(user_id)?;
        Ok(Self(user_id.to_string()))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    pub fn into_string(self) -> String {
        self.0
    }
}