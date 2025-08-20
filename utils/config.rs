use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use crate::constants::{DEFAULT_SLIPPAGE_BPS, DEFAULT_PRIORITY_FEE};
use crate::errors::BotError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // API Keys
    pub telegram_bot_token: String,
    pub helius_api_key: String,
    pub groq_api_key: String,
    pub database_url: String,
    pub rebate_wallet_address: String,
    
    // Network Settings
    pub network: NetworkType,
    
    // Trading Configuration
    pub max_trade_size_sol: f64,
    pub min_trade_size_sol: f64,
    pub slippage_bps: u16,
    pub priority_fee_lamports: u64,
    pub enable_backrun_rebates: bool,
    
    // User Authorization
    pub allowed_users: Vec<String>,
    pub admin_users: Vec<String>,
    
    // Feature Flags
    pub enable_ai_analysis: bool,
    pub enable_paper_trading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    Mainnet,
    Devnet,
    Testnet,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            // Required API Keys
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .map_err(|_| BotError::Config("TELEGRAM_BOT_TOKEN not set".into()))?,
            helius_api_key: env::var("HELIUS_API_KEY")
                .map_err(|_| BotError::Config("HELIUS_API_KEY not set".into()))?,
            groq_api_key: env::var("GROQ_API_KEY")
                .map_err(|_| BotError::Config("GROQ_API_KEY not set".into()))?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mock://localhost".to_string()),
            rebate_wallet_address: env::var("REBATE_WALLET_ADDRESS")
                .map_err(|_| BotError::Config("REBATE_WALLET_ADDRESS not set".into()))?,
            
            // Network Settings
            network: Self::parse_network(&env::var("NETWORK").unwrap_or_else(|_| "mainnet".to_string())),
            
            // Trading Configuration
            max_trade_size_sol: env::var("MAX_TRADE_SIZE_SOL")
                .unwrap_or_else(|_| MAX_TRADE_SOL.to_string())
                .parse()
                .unwrap_or(MAX_TRADE_SOL),
            min_trade_size_sol: env::var("MIN_TRADE_SIZE_SOL")
                .unwrap_or_else(|_| MIN_TRADE_SOL.to_string())
                .parse()
                .unwrap_or(MIN_TRADE_SOL),
            slippage_bps: env::var("SLIPPAGE_BPS")
                .unwrap_or_else(|_| DEFAULT_SLIPPAGE_BPS.to_string())
                .parse()
                .unwrap_or(DEFAULT_SLIPPAGE_BPS),
            priority_fee_lamports: env::var("PRIORITY_FEE_LAMPORTS")
                .unwrap_or_else(|_| DEFAULT_PRIORITY_FEE.to_string())
                .parse()
                .unwrap_or(DEFAULT_PRIORITY_FEE),
            enable_backrun_rebates: env::var("ENABLE_BACKRUN_REBATES")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            
            // User Authorization
            allowed_users: env::var("ALLOWED_USERS")
                .unwrap_or_else(|_| String::new())
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            admin_users: env::var("ADMIN_USERS")
                .unwrap_or_else(|_| String::new())
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            
            // Feature Flags
            enable_ai_analysis: env::var("ENABLE_AI_ANALYSIS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_paper_trading: env::var("ENABLE_PAPER_TRADING")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
    
    fn parse_network(network: &str) -> NetworkType {
        match network.to_lowercase().as_str() {
            "mainnet" | "mainnet-beta" => NetworkType::Mainnet,
            "devnet" => NetworkType::Devnet,
            "testnet" => NetworkType::Testnet,
            _ => NetworkType::Mainnet,
        }
    }
    
    pub fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.is_empty() || self.allowed_users.contains(&user_id.to_string())
    }
    
    pub fn is_admin(&self, user_id: &str) -> bool {
        self.admin_users.contains(&user_id.to_string())
    }
    
    pub fn get_rpc_url(&self) -> String {
        match self.network {
            NetworkType::Mainnet => {
                if self.enable_backrun_rebates && !self.rebate_wallet_address.is_empty() {
                    format!("{}/?api-key={}&rebate-address={}", 
                        HELIUS_BASE_URL, 
                        self.helius_api_key, 
                        self.rebate_wallet_address)
                } else {
                    format!("{}/?api-key={}", HELIUS_BASE_URL, self.helius_api_key)
                }
            },
            NetworkType::Devnet => "https://api.devnet.solana.com".to_string(),
            NetworkType::Testnet => "https://api.testnet.solana.com".to_string(),
        }
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.telegram_bot_token.is_empty() {
            return Err(BotError::Config("Telegram bot token is required".into()).into());
        }
        
        if self.max_trade_size_sol <= self.min_trade_size_sol {
            return Err(BotError::Config("Max trade size must be greater than min trade size".into()).into());
        }
        
        if self.slippage_bps > MAX_SLIPPAGE_BPS {
            return Err(BotError::Config(format!("Slippage cannot exceed {}%", MAX_SLIPPAGE_BPS / 100)).into());
        }
        
        Ok(())
    }
    
    pub fn is_production(&self) -> bool {
        matches!(self.network, NetworkType::Mainnet)
    }
}