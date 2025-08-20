use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};

/// Pump.fun API client for token operations
pub struct PumpFunClient {
    client: Client,
    api_url: String,
    timeout: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PumpToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub image_url: Option<String>,
    pub created_at: String,
    pub market_cap: f64,
    pub price: f64,
    pub volume_24h: f64,
    pub price_change_24h: f64,
    pub holders: u32,
    pub bonding_curve_progress: f64,
    pub liquidity_locked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub image_url: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub success: bool,
    pub token_address: String,
    pub transaction_hash: String,
    pub bonding_curve_address: String,
    pub initial_price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuyTokenRequest {
    pub token_address: String,
    pub amount_sol: f64,
    pub slippage_bps: u16,
    pub user_wallet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuyTokenResponse {
    pub success: bool,
    pub transaction_hash: String,
    pub tokens_received: f64,
    pub price_per_token: f64,
    pub price_impact: f64,
}

impl PumpFunClient {
    /// Create a new Pump.fun API client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("solana-trading-bot/0.1.0")
            .gzip(true)
            .build()?;
        
        Ok(Self {
            client,
            api_url: "https://api.pump.fun".to_string(), // Replace with actual API URL
            timeout: Duration::from_secs(30),
        })
    }
    
    /// Get trending tokens on Pump.fun with timeout handling
    pub async fn get_trending(&self, limit: usize) -> Result<Vec<PumpToken>> {
        use crate::utils::with_timeout;
        
        let url = format!("{}/tokens/trending?limit={}", self.api_url, limit);
        
        let operation = async {
            let response = self.client.get(&url).send().await?;
            
            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to fetch trending tokens: {}",
                    response.status()
                ));
            }
            
            let tokens: Vec<PumpToken> = response.json().await?;
            info!("Fetched {} trending tokens from Pump.fun", tokens.len());
            
            Ok(tokens)
        };
        
        with_timeout(operation, self.timeout, "pump_fun_get_trending").await
    }
    
    /// Get token details by address
    pub async fn get_token(&self, token_address: &str) -> Result<PumpToken> {
        let url = format!("{}/tokens/{}", self.api_url, token_address);
        
        let response = tokio::time::timeout(
            self.timeout,
            self.client.get(&url).send()
        ).await??;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch token {}: {}",
                token_address,
                response.status()
            ));
        }
        
        let token: PumpToken = response.json().await?;
        Ok(token)
    }
    
    /// Create a new token on Pump.fun
    pub async fn create_token(&self, request: CreateTokenRequest) -> Result<CreateTokenResponse> {
        let url = format!("{}/tokens/create", self.api_url);
        
        info!("Creating token: {} ({})", request.name, request.symbol);
        
        let response = tokio::time::timeout(
            self.timeout,
            self.client
                .post(&url)
                .json(&request)
                .send()
        ).await??;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to create token: {}",
                error_text
            ));
        }
        
        let result: CreateTokenResponse = response.json().await?;
        
        if result.success {
            info!("Successfully created token at {}", result.token_address);
        } else {
            warn!("Token creation failed");
        }
        
        Ok(result)
    }
    
    /// Buy tokens on Pump.fun
    pub async fn buy_token(&self, request: BuyTokenRequest) -> Result<BuyTokenResponse> {
        let url = format!("{}/trade/buy", self.api_url);
        
        info!(
            "Buying {} SOL worth of token {}",
            request.amount_sol,
            request.token_address
        );
        
        let response = tokio::time::timeout(
            self.timeout,
            self.client
                .post(&url)
                .json(&request)
                .send()
        ).await??;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to buy token: {}",
                error_text
            ));
        }
        
        let result: BuyTokenResponse = response.json().await?;
        
        if result.success {
            info!(
                "Successfully bought {} tokens for {} SOL",
                result.tokens_received,
                request.amount_sol
            );
        } else {
            warn!("Token purchase failed");
        }
        
        Ok(result)
    }
    
    /// Search tokens by name or symbol
    pub async fn search_tokens(&self, query: &str) -> Result<Vec<PumpToken>> {
        let url = format!("{}/tokens/search?q={}", self.api_url, urlencoding::encode(query));
        
        let response = tokio::time::timeout(
            self.timeout,
            self.client.get(&url).send()
        ).await??;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to search tokens: {}",
                response.status()
            ));
        }
        
        let tokens: Vec<PumpToken> = response.json().await?;
        info!("Found {} tokens matching '{}'", tokens.len(), query);
        
        Ok(tokens)
    }
    
    /// Get user's portfolio on Pump.fun
    pub async fn get_portfolio(&self, wallet_address: &str) -> Result<Vec<PumpToken>> {
        let url = format!("{}/portfolio/{}", self.api_url, wallet_address);
        
        let response = tokio::time::timeout(
            self.timeout,
            self.client.get(&url).send()
        ).await??;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch portfolio: {}",
                response.status()
            ));
        }
        
        let portfolio: Vec<PumpToken> = response.json().await?;
        info!("Fetched {} tokens in portfolio", portfolio.len());
        
        Ok(portfolio)
    }
    
    /// Check if a token is graduating to Raydium
    pub async fn check_graduation_status(&self, token_address: &str) -> Result<bool> {
        let token = self.get_token(token_address).await?;
        
        // Token graduates when bonding curve reaches 100%
        Ok(token.bonding_curve_progress >= 100.0)
    }
}

/// Mock implementation for development/testing
pub struct MockPumpFunClient;

impl MockPumpFunClient {
    pub async fn get_trending(&self, limit: usize) -> Result<Vec<PumpToken>> {
        Ok(vec![
            PumpToken {
                address: "MEMECAT123...".to_string(),
                name: "Meme Cat".to_string(),
                symbol: "MEMECAT".to_string(),
                description: "The ultimate meme cat token".to_string(),
                image_url: Some("https://example.com/memecat.png".to_string()),
                created_at: "2024-01-20T10:00:00Z".to_string(),
                market_cap: 47000.0,
                price: 0.000012,
                volume_24h: 125000.0,
                price_change_24h: 890.0,
                holders: 523,
                bonding_curve_progress: 85.5,
                liquidity_locked: false,
            },
            PumpToken {
                address: "DOGEAI456...".to_string(),
                name: "Doge AI".to_string(),
                symbol: "DOGEAI".to_string(),
                description: "AI-powered doge token".to_string(),
                image_url: Some("https://example.com/dogeai.png".to_string()),
                created_at: "2024-01-20T09:00:00Z".to_string(),
                market_cap: 23000.0,
                price: 0.00008,
                volume_24h: 89000.0,
                price_change_24h: 340.0,
                holders: 312,
                bonding_curve_progress: 62.3,
                liquidity_locked: false,
            },
        ].into_iter().take(limit).collect())
    }
    
    pub async fn buy_token(&self, request: BuyTokenRequest) -> Result<BuyTokenResponse> {
        Ok(BuyTokenResponse {
            success: true,
            transaction_hash: "5xMockTxHash123...".to_string(),
            tokens_received: 1_500_000.0,
            price_per_token: request.amount_sol / 1_500_000.0,
            price_impact: 0.5,
        })
    }
    
    pub async fn create_token(&self, request: CreateTokenRequest) -> Result<CreateTokenResponse> {
        Ok(CreateTokenResponse {
            success: true,
            token_address: format!("{}123456789", &request.symbol[..3]),
            transaction_hash: "5xMockCreateTx...".to_string(),
            bonding_curve_address: "BCurve123...".to_string(),
            initial_price: 0.000001,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_get_trending() {
        let client = MockPumpFunClient;
        let tokens = client.get_trending(5).await.unwrap();
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].symbol, "MEMECAT");
    }
    
    #[tokio::test]
    async fn test_mock_buy_token() {
        let client = MockPumpFunClient;
        let request = BuyTokenRequest {
            token_address: "TEST123".to_string(),
            amount_sol: 0.1,
            slippage_bps: 300,
            user_wallet: "User123".to_string(),
        };
        
        let response = client.buy_token(request).await.unwrap();
        assert!(response.success);
        assert!(response.tokens_received > 0.0);
    }
}