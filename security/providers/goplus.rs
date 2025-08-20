use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, error, debug};

use crate::security::types::*;
use crate::middleware::ApiRateLimiter;

const GOPLUS_API_BASE: &str = "https://api.gopluslabs.io/api/v1";
const GOPLUS_SOLANA_ENDPOINT: &str = "/token_security/solana";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoPlusResponse {
    code: i32,
    message: String,
    result: HashMap<String, GoPlusTokenData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoPlusTokenData {
    honeypot: Option<String>,           // "0" = no, "1" = yes
    buy_tax: Option<String>,
    sell_tax: Option<String>,
    is_open_source: Option<String>,     // "0" = no, "1" = yes
    is_proxy: Option<String>,           // "0" = no, "1" = yes
    is_mintable: Option<String>,        // "0" = no, "1" = yes
    can_take_back_ownership: Option<String>, // "0" = no, "1" = yes
    owner_address: Option<String>,
    creator_address: Option<String>,
    liquidity: Option<String>,
    liquidity_type: Option<String>,
    pair_liquidity: Option<Vec<PairLiquidity>>,
    holder_count: Option<String>,
    total_supply: Option<String>,
    holders: Option<Vec<GoPlusHolder>>,
    lp_holder_count: Option<String>,
    lp_total_supply: Option<String>,
    is_true_token: Option<String>,      // "0" = fake, "1" = real
    is_airdrop_scam: Option<String>,    // "0" = no, "1" = yes
    is_in_dex: Option<String>,          // "0" = no, "1" = yes
    dex: Option<Vec<DexInfo>>,
    slippage_modifiable: Option<String>, // "0" = no, "1" = yes
    is_anti_whale: Option<String>,       // "0" = no, "1" = yes
    anti_whale_modifiable: Option<String>,
    cannot_buy: Option<String>,          // "0" = can buy, "1" = cannot
    cannot_sell_all: Option<String>,     // "0" = can sell all, "1" = cannot
    trading_cooldown: Option<String>,    // "0" = no, "1" = yes
    personal_slippage_modifiable: Option<String>,
    token_name: Option<String>,
    token_symbol: Option<String>,
    note: Option<String>,
    
    // Solana specific fields
    freeze_authority: Option<String>,
    mint_authority: Option<String>,
    metadata_uri: Option<String>,
    is_token_2022: Option<String>,      // "0" = SPL, "1" = SPL2022
    extensions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PairLiquidity {
    pair_address: String,
    liquidity: String,
    liquidity_token: String,
    liquidity_usd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoPlusHolder {
    address: String,
    balance: String,
    percent: String,
    is_locked: Option<String>,
    is_contract: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DexInfo {
    name: String,
    pair: String,
    liquidity: String,
    volume_24h: Option<String>,
}

/// GoPlus Security API provider for Solana tokens
pub struct GoPlusProvider {
    client: Client,
    api_key: Option<String>,
    rate_limiter: ApiRateLimiter,
}

impl GoPlusProvider {
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("solana-trading-bot/1.0")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key,
            rate_limiter: ApiRateLimiter::new(),
        }
    }
    
    /// Check token security using GoPlus API
    pub async fn check_token_security(&self, token_address: &str) -> Result<SecurityAnalysis> {
        debug!("Checking security for token: {}", token_address);
        
        // Rate limiting
        let _permit = self.rate_limiter.check_rate_limit("goplus_security").await?;
        
        // Build URL
        let url = format!(
            "{}{}?contract_addresses={}",
            GOPLUS_API_BASE,
            GOPLUS_SOLANA_ENDPOINT,
            token_address
        );
        
        // Make request
        let mut request = self.client.get(&url);
        if let Some(api_key) = &self.api_key {
            request = request.header("X-API-KEY", api_key);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            error!("GoPlus API error: {}", response.status());
            return Err(anyhow::anyhow!("GoPlus API error: {}", response.status()));
        }
        
        let goplus_response: GoPlusResponse = response.json().await?;
        
        if goplus_response.code != 0 {
            warn!("GoPlus API returned error: {}", goplus_response.message);
            return Err(anyhow::anyhow!("GoPlus error: {}", goplus_response.message));
        }
        
        // Get token data
        let token_data = goplus_response.result
            .get(token_address.to_lowercase().as_str())
            .or_else(|| goplus_response.result.get(token_address))
            .ok_or_else(|| anyhow::anyhow!("Token not found in GoPlus response"))?;
        
        // Convert to SecurityAnalysis
        let analysis = self.convert_to_security_analysis(token_address, token_data);
        
        info!(
            "GoPlus security check complete for {}: Score {}/100",
            token_address, analysis.risk_score
        );
        
        Ok(analysis)
    }
    
    /// Convert GoPlus data to SecurityAnalysis
    fn convert_to_security_analysis(&self, token_address: &str, data: &GoPlusTokenData) -> SecurityAnalysis {
        let mut warnings = Vec::new();
        let mut passed_checks = Vec::new();
        let mut failed_checks = Vec::new();
        let mut risk_score = 100u8;
        
        // Check honeypot
        let is_honeypot = data.honeypot.as_ref().map(|h| h == "1").unwrap_or(false);
        if is_honeypot {
            failed_checks.push("Honeypot detected".to_string());
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Critical,
                category: WarningCategory::Honeypot,
                message: "Token is a honeypot - you cannot sell".to_string(),
                details: Some("This token prevents selling after buying".to_string()),
            });
            risk_score = risk_score.saturating_sub(50);
        } else {
            passed_checks.push("Not a honeypot".to_string());
        }
        
        // Check trading restrictions
        let cannot_buy = data.cannot_buy.as_ref().map(|b| b == "1").unwrap_or(false);
        let cannot_sell = data.cannot_sell_all.as_ref().map(|s| s == "1").unwrap_or(false);
        
        if cannot_buy {
            failed_checks.push("Cannot buy token".to_string());
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Critical,
                category: WarningCategory::Trading,
                message: "Token cannot be purchased".to_string(),
                details: None,
            });
            risk_score = risk_score.saturating_sub(40);
        } else {
            passed_checks.push("Can buy token".to_string());
        }
        
        if cannot_sell {
            failed_checks.push("Cannot sell all tokens".to_string());
            warnings.push(SecurityWarning {
                severity: WarningSeverity::High,
                category: WarningCategory::Trading,
                message: "Cannot sell entire position".to_string(),
                details: Some("You may not be able to sell all your tokens".to_string()),
            });
            risk_score = risk_score.saturating_sub(30);
        } else {
            passed_checks.push("Can sell tokens".to_string());
        }
        
        // Check authorities
        if data.freeze_authority.is_some() {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                category: WarningCategory::Ownership,
                message: "Freeze authority enabled".to_string(),
                details: Some("Owner can freeze token transfers".to_string()),
            });
            risk_score = risk_score.saturating_sub(10);
        } else {
            passed_checks.push("No freeze authority".to_string());
        }
        
        if data.mint_authority.is_some() {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                category: WarningCategory::Ownership,
                message: "Mint authority enabled".to_string(),
                details: Some("Owner can mint new tokens".to_string()),
            });
            risk_score = risk_score.saturating_sub(10);
        } else {
            passed_checks.push("No mint authority".to_string());
        }
        
        // Check if it's a scam
        let is_airdrop_scam = data.is_airdrop_scam.as_ref().map(|s| s == "1").unwrap_or(false);
        if is_airdrop_scam {
            failed_checks.push("Airdrop scam detected".to_string());
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Critical,
                category: WarningCategory::Contract,
                message: "Token identified as airdrop scam".to_string(),
                details: Some("This appears to be a fraudulent airdrop token".to_string()),
            });
            risk_score = risk_score.saturating_sub(40);
        }
        
        // Check liquidity
        let liquidity_usd = data.liquidity
            .as_ref()
            .and_then(|l| l.parse::<f64>().ok())
            .unwrap_or(0.0);
        
        if liquidity_usd < 1000.0 {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::High,
                category: WarningCategory::Liquidity,
                message: format!("Very low liquidity: ${:.2}", liquidity_usd),
                details: Some("Extremely low liquidity can lead to high slippage".to_string()),
            });
            risk_score = risk_score.saturating_sub(20);
        } else if liquidity_usd < 10000.0 {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                category: WarningCategory::Liquidity,
                message: format!("Low liquidity: ${:.2}", liquidity_usd),
                details: Some("Low liquidity may cause price impact".to_string()),
            });
            risk_score = risk_score.saturating_sub(10);
        } else {
            passed_checks.push(format!("Good liquidity: ${:.0}", liquidity_usd));
        }
        
        // Parse holders
        let top_holders = data.holders.as_ref().map(|holders| {
            holders.iter().take(10).map(|h| HolderInfo {
                address: h.address.clone(),
                balance: h.balance.parse::<f64>().unwrap_or(0.0),
                percentage: h.percent.parse::<f64>().unwrap_or(0.0),
                is_locked: h.is_locked.as_ref().map(|l| l == "1").unwrap_or(false),
                is_creator: false, // Would need to match with creator_address
                is_exchange: h.is_contract.as_ref().map(|c| c == "1").unwrap_or(false),
            }).collect()
        }).unwrap_or_default();
        
        // Check holder concentration
        if let Some(top_holder) = top_holders.first() {
            if top_holder.percentage > 50.0 {
                warnings.push(SecurityWarning {
                    severity: WarningSeverity::High,
                    category: WarningCategory::Distribution,
                    message: format!("Top holder owns {:.1}%", top_holder.percentage),
                    details: Some("High concentration in single wallet".to_string()),
                });
                risk_score = risk_score.saturating_sub(15);
            }
        }
        
        // Generate recommendations
        let mut recommendations = Vec::new();
        
        if risk_score >= 80 {
            recommendations.push("Token appears safe for trading".to_string());
        } else if risk_score >= 60 {
            recommendations.push("Proceed with caution".to_string());
            recommendations.push("Consider smaller position size".to_string());
        } else if risk_score >= 40 {
            recommendations.push("High risk - trade only if you understand the risks".to_string());
            recommendations.push("Use stop loss orders".to_string());
        } else {
            recommendations.push("Extremely high risk - consider avoiding".to_string());
            recommendations.push("Do additional research before trading".to_string());
        }
        
        let risk_level = SecurityAnalysis::calculate_risk_level(risk_score);
        
        SecurityAnalysis {
            token_address: token_address.to_string(),
            token_symbol: data.token_symbol.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
            token_name: data.token_name.clone().unwrap_or_else(|| "Unknown Token".to_string()),
            is_honeypot,
            can_sell: !cannot_sell,
            can_buy: !cannot_buy,
            liquidity_locked: false, // GoPlus doesn't provide this directly
            liquidity_lock_duration: None,
            freeze_authority: data.freeze_authority.clone(),
            mint_authority: data.mint_authority.clone(),
            update_authority: None,
            creator_address: data.creator_address.clone(),
            creator_balance_percent: 0.0, // Would need to calculate
            top_holders,
            holder_count: data.holder_count
                .as_ref()
                .and_then(|h| h.parse::<u32>().ok())
                .unwrap_or(0),
            risk_score,
            risk_level,
            warnings,
            passed_checks,
            failed_checks,
            recommendations,
            token_age_hours: 0.0, // Would need to calculate from chain
            total_supply: data.total_supply
                .as_ref()
                .and_then(|t| t.parse::<f64>().ok())
                .unwrap_or(0.0),
            circulating_supply: 0.0, // Would need to calculate
            liquidity_usd,
            volume_24h: 0.0, // Would need from DEX data
            transaction_count_24h: 0,
            unique_wallets_24h: 0,
            metadata: TokenMetadata {
                description: None,
                website: None,
                twitter: None,
                telegram: None,
                discord: None,
                logo_uri: data.metadata_uri.clone(),
                is_verified: data.is_true_token.as_ref().map(|t| t == "1").unwrap_or(false),
            },
            analysis_timestamp: chrono::Utc::now(),
            data_sources: vec!["GoPlus Security".to_string()],
        }
    }
}