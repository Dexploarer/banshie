use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::Utc;

use super::types::*;
use crate::errors::BotError;

/// Fetches real portfolio data from Solana RPC and price APIs
pub struct PortfolioFetcher {
    client: Client,
    rpc_url: String,
    jupiter_price_api: String,
    token_list_cache: Arc<RwLock<HashMap<String, TokenMetadata>>>,
}

impl PortfolioFetcher {
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: Client::new(),
            rpc_url,
            jupiter_price_api: "https://price.jup.ag/v4/price".to_string(),
            token_list_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Fetch complete portfolio for a wallet
    pub async fn fetch_portfolio(&self, wallet_address: &str) -> Result<Portfolio> {
        info!("Fetching portfolio for wallet: {}", wallet_address);
        
        // Fetch SOL balance
        let sol_balance = self.fetch_sol_balance(wallet_address).await?;
        debug!("SOL balance: {}", sol_balance);
        
        // Fetch token accounts
        let token_holdings = self.fetch_token_holdings(wallet_address).await?;
        debug!("Found {} token holdings", token_holdings.len());
        
        // Fetch prices for all tokens
        let mut holdings_with_prices = Vec::new();
        let mut total_value_usd = 0.0;
        
        // Add SOL as first holding
        if sol_balance > 0.0 {
            let sol_price = self.fetch_token_price("So11111111111111111111111111111111111111112").await?;
            let sol_holding = TokenHolding {
                mint_address: "So11111111111111111111111111111111111111112".to_string(),
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
                balance: sol_balance,
                decimals: 9,
                value_usd: sol_balance * sol_price,
                value_sol: sol_balance,
                price_usd: sol_price,
                price_change_24h: None, // Would need historical data
                logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png".to_string()),
                is_verified: true,
            };
            total_value_usd += sol_holding.value_usd;
            holdings_with_prices.push(sol_holding);
        }
        
        // Process token holdings
        for holding in token_holdings {
            if holding.balance > 0.0 {
                let price = self.fetch_token_price(&holding.mint_address).await.unwrap_or(0.0);
                let metadata = self.get_token_metadata(&holding.mint_address).await;
                
                let mut token_holding = holding;
                token_holding.price_usd = price;
                token_holding.value_usd = token_holding.balance * price;
                token_holding.value_sol = token_holding.value_usd / self.fetch_token_price("So11111111111111111111111111111111111111112").await.unwrap_or(1.0);
                
                if let Some(meta) = metadata {
                    token_holding.name = meta.name;
                    token_holding.symbol = meta.symbol;
                    token_holding.logo_uri = meta.logo_uri;
                    token_holding.is_verified = meta.verified.unwrap_or(false);
                }
                
                total_value_usd += token_holding.value_usd;
                holdings_with_prices.push(token_holding);
            }
        }
        
        // Calculate performance metrics
        let performance = self.calculate_performance(&holdings_with_prices).await;
        
        let portfolio = Portfolio {
            wallet_address: wallet_address.to_string(),
            total_value_usd,
            total_value_sol: total_value_usd / self.fetch_token_price("So11111111111111111111111111111111111111112").await.unwrap_or(1.0),
            holdings: holdings_with_prices,
            performance,
            last_updated: Utc::now(),
        };
        
        info!("Portfolio fetched successfully. Total value: ${:.2}", portfolio.total_value_usd);
        Ok(portfolio)
    }
    
    /// Fetch SOL balance for wallet
    async fn fetch_sol_balance(&self, wallet_address: &str) -> Result<f64> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [wallet_address]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(BotError::api(format!("RPC request failed: {}", response.status())).into());
        }
        
        let balance_response: BalanceResponse = response.json().await?;
        Ok(balance_response.result.value as f64 / 1_000_000_000.0) // Convert lamports to SOL
    }
    
    /// Fetch token holdings for wallet
    async fn fetch_token_holdings(&self, wallet_address: &str) -> Result<Vec<TokenHolding>> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet_address,
                {
                    "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(BotError::api(format!("Token accounts request failed: {}", response.status())).into());
        }
        
        let accounts_response: TokenAccountsResponse = response.json().await?;
        let mut holdings = Vec::new();
        
        for account in accounts_response.result.value {
            let token_info = &account.account.data.parsed.info;
            let mint = &token_info.mint;
            let amount = token_info.token_amount.ui_amount.unwrap_or(0.0);
            
            if amount > 0.0 {
                holdings.push(TokenHolding {
                    mint_address: mint.clone(),
                    symbol: mint[..8].to_string(), // Truncated, will be updated with metadata
                    name: "Unknown Token".to_string(), // Will be updated with metadata
                    balance: amount,
                    decimals: token_info.token_amount.decimals,
                    value_usd: 0.0, // Will be calculated with price
                    value_sol: 0.0, // Will be calculated with price
                    price_usd: 0.0, // Will be fetched
                    price_change_24h: None,
                    logo_uri: None,
                    is_verified: false,
                });
            }
        }
        
        Ok(holdings)
    }
    
    /// Fetch token price from Jupiter
    async fn fetch_token_price(&self, mint_address: &str) -> Result<f64> {
        let url = format!("{}?ids={}", self.jupiter_price_api, mint_address);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            warn!("Failed to fetch price for {}: {}", mint_address, response.status());
            return Ok(0.0);
        }
        
        let price_response: JupiterPriceResponse = response.json().await?;
        
        if let Some(price_data) = price_response.data.get(mint_address) {
            Ok(price_data.price)
        } else {
            debug!("No price data found for {}", mint_address);
            Ok(0.0)
        }
    }
    
    /// Get token metadata from Jupiter token list
    async fn get_token_metadata(&self, mint_address: &str) -> Option<TokenMetadata> {
        // Check cache first
        {
            let cache = self.token_list_cache.read().await;
            if let Some(metadata) = cache.get(mint_address) {
                return Some(metadata.clone());
            }
        }
        
        // Fetch from Jupiter token list
        let url = "https://token.jup.ag/all";
        
        let response = self.client
            .get(url)
            .send()
            .await
            .ok()?;
        
        if !response.status().is_success() {
            return None;
        }
        
        let token_list: Vec<TokenMetadata> = response.json().await.ok()?;
        
        // Update cache
        {
            let mut cache = self.token_list_cache.write().await;
            for token in &token_list {
                cache.insert(token.address.clone(), token.clone());
            }
        }
        
        // Find our token
        token_list.into_iter().find(|t| t.address == mint_address)
    }
    
    /// Calculate portfolio performance metrics
    async fn calculate_performance(&self, holdings: &[TokenHolding]) -> PortfolioPerformance {
        let total_value = holdings.iter().map(|h| h.value_usd).sum::<f64>();
        
        // Find best and worst performers based on 24h change
        let mut best_performer = None;
        let mut worst_performer = None;
        let mut best_change = f64::NEG_INFINITY;
        let mut worst_change = f64::INFINITY;
        
        for holding in holdings {
            if let Some(change_24h) = holding.price_change_24h {
                if change_24h > best_change {
                    best_change = change_24h;
                    best_performer = Some(TokenPerformance {
                        symbol: holding.symbol.clone(),
                        pnl_usd: holding.value_usd * (change_24h / 100.0),
                        pnl_percentage: change_24h,
                    });
                }
                
                if change_24h < worst_change {
                    worst_change = change_24h;
                    worst_performer = Some(TokenPerformance {
                        symbol: holding.symbol.clone(),
                        pnl_usd: holding.value_usd * (change_24h / 100.0),
                        pnl_percentage: change_24h,
                    });
                }
            }
        }
        
        // Find largest holding
        let largest_holding = holdings
            .iter()
            .max_by(|a, b| a.value_usd.partial_cmp(&b.value_usd).unwrap())
            .map(|h| h.symbol.clone());
        
        // Calculate 24h P&L (simplified)
        let pnl_24h_usd = holdings
            .iter()
            .map(|h| {
                h.price_change_24h
                    .map(|change| h.value_usd * (change / 100.0))
                    .unwrap_or(0.0)
            })
            .sum::<f64>();
        
        let pnl_24h_percentage = if total_value > 0.0 {
            (pnl_24h_usd / total_value) * 100.0
        } else {
            0.0
        };
        
        PortfolioPerformance {
            total_pnl_usd: 0.0, // Would need historical cost basis
            total_pnl_sol: 0.0, // Would need historical cost basis  
            pnl_percentage: 0.0, // Would need historical cost basis
            pnl_24h_usd,
            pnl_24h_percentage,
            best_performer,
            worst_performer,
            win_rate: 0.0, // Would need transaction history
            largest_holding,
        }
    }
    
    /// Get portfolio summary for quick display
    pub async fn get_portfolio_summary(&self, wallet_address: &str) -> Result<PortfolioSummary> {
        let portfolio = self.fetch_portfolio(wallet_address).await?;
        
        // Get top 5 holdings by value
        let mut holdings = portfolio.holdings.clone();
        holdings.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap());
        
        let top_holdings = holdings
            .iter()
            .take(5)
            .map(|h| HoldingSummary {
                symbol: h.symbol.clone(),
                balance: h.balance,
                value_usd: h.value_usd,
                percentage: if portfolio.total_value_usd > 0.0 {
                    (h.value_usd / portfolio.total_value_usd) * 100.0
                } else {
                    0.0
                },
            })
            .collect();
        
        Ok(PortfolioSummary {
            total_value_usd: portfolio.total_value_usd,
            total_holdings: portfolio.holdings.len(),
            top_holdings,
            performance_24h: portfolio.performance.pnl_24h_percentage,
            performance_total: portfolio.performance.pnl_percentage,
        })
    }
}