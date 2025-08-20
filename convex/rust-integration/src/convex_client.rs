use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use tokio::time::{sleep, Duration};
use anyhow::{anyhow, Result};

/// HTTP client for communicating with Convex backend
#[derive(Clone)]
pub struct ConvexClient {
    client: Client,
    base_url: String,
    site_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConvexResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub telegram_id: i64,
    pub username: String,
    pub is_premium: bool,
    pub settings: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub total_value: String,
    pub total_pnl: String,
    pub total_pnl_percentage: String,
    pub position_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingSignal {
    pub token_mint: String,
    pub signal_type: String,
    pub strength: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub user_id: String,
    pub order_type: String,
    pub token_mint: String,
    pub side: String,
    pub amount: String,
    pub price: Option<String>,
    pub slippage: Option<f64>,
}

impl ConvexClient {
    /// Create a new Convex client
    pub fn new() -> Result<Self> {
        let base_url = env::var("CONVEX_URL")
            .unwrap_or_else(|_| "https://your-convex-app.convex.site".to_string());
        let site_url = env::var("CONVEX_SITE_URL")
            .unwrap_or_else(|_| "https://your-convex-app.convex.cloud".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url,
            site_url,
        })
    }

    /// Execute a Convex query
    pub async fn query<T>(&self, function_name: &str, args: Value) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/api/query", self.base_url);
        
        let payload = json!({
            "path": function_name,
            "args": args,
            "format": "json"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Query failed with status: {}", response.status()));
        }

        let result: Value = response.json().await?;
        
        // Handle Convex response format
        if let Some(error) = result.get("error") {
            return Err(anyhow!("Convex error: {}", error));
        }

        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    /// Execute a Convex mutation
    pub async fn mutation<T>(&self, function_name: &str, args: Value) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/api/mutation", self.base_url);
        
        let payload = json!({
            "path": function_name,
            "args": args,
            "format": "json"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Mutation failed with status: {}", response.status()));
        }

        let result: Value = response.json().await?;
        
        if let Some(error) = result.get("error") {
            return Err(anyhow!("Convex error: {}", error));
        }

        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    /// Execute a Convex action
    pub async fn action<T>(&self, function_name: &str, args: Value) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/api/action", self.base_url);
        
        let payload = json!({
            "path": function_name,
            "args": args,
            "format": "json"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Action failed with status: {}", response.status()));
        }

        let result: Value = response.json().await?;
        
        if let Some(error) = result.get("error") {
            return Err(anyhow!("Convex error: {}", error));
        }

        serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    // User Management
    pub async fn get_user_by_telegram_id(&self, telegram_id: i64) -> Result<Option<UserProfile>> {
        let args = json!({
            "telegramId": telegram_id
        });

        self.query("queries/users:getUserByTelegramId", args).await
    }

    pub async fn create_or_update_user(&self, telegram_id: i64, username: &str) -> Result<String> {
        let args = json!({
            "telegramId": telegram_id,
            "username": username,
            "isPremium": false,
            "settings": {
                "defaultSlippage": 1.0,
                "riskTolerance": "medium",
                "notifications": true
            }
        });

        self.mutation("mutations/users:createOrUpdateUser", args).await
    }

    // Portfolio Management
    pub async fn get_portfolio(&self, user_id: &str) -> Result<PortfolioSummary> {
        let args = json!({
            "userId": user_id
        });

        self.query("queries/portfolio:getPortfolio", args).await
    }

    pub async fn sync_wallet_balances(&self, user_id: &str, wallet_address: &str) -> Result<Value> {
        let args = json!({
            "userId": user_id,
            "walletAddress": wallet_address
        });

        self.action("actions/wallets:syncBalances", args).await
    }

    // Trading
    pub async fn place_order(&self, order: OrderRequest) -> Result<String> {
        let args = serde_json::to_value(order)?;
        self.mutation("mutations/trading:placeTrade", args).await
    }

    pub async fn get_order_status(&self, order_id: &str) -> Result<Value> {
        let args = json!({
            "orderId": order_id
        });

        self.query("queries/trading:getOrderStatus", args).await
    }

    // AI Signals
    pub async fn get_latest_signals(&self, limit: u32) -> Result<Vec<TradingSignal>> {
        let args = json!({
            "limit": limit
        });

        self.query("queries/ai:getLatestSignals", args).await
    }

    pub async fn generate_signal(&self, token_mint: &str) -> Result<TradingSignal> {
        let args = json!({
            "tokenMint": token_mint
        });

        self.action("actions/ai:generateTradingSignals", args).await
    }

    // Price Data
    pub async fn get_token_price(&self, token_mint: &str) -> Result<Value> {
        let args = json!({
            "mint": token_mint
        });

        self.query("queries/prices:getTokenPrice", args).await
    }

    pub async fn update_prices(&self, tokens: Vec<&str>) -> Result<Value> {
        let args = json!({
            "tokens": tokens
        });

        self.action("actions/prices:updateTokenPrices", args).await
    }

    // DCA Strategies
    pub async fn get_user_dca_strategies(&self, user_id: &str) -> Result<Vec<Value>> {
        let args = json!({
            "userId": user_id
        });

        self.query("queries/dca:getUserStrategies", args).await
    }

    pub async fn create_dca_strategy(&self, user_id: &str, token_mint: &str, amount: f64, frequency: &str) -> Result<String> {
        let args = json!({
            "userId": user_id,
            "fromMint": "So11111111111111111111111111111111111111112", // SOL
            "toMint": token_mint,
            "amount": amount.to_string(),
            "frequency": frequency,
            "enabled": true,
            "conditions": {}
        });

        self.mutation("mutations/dca:createStrategy", args).await
    }

    // Alerts
    pub async fn create_price_alert(&self, user_id: &str, token_mint: &str, target_price: f64, condition: &str) -> Result<String> {
        let args = json!({
            "userId": user_id,
            "alertType": "price",
            "tokenMint": token_mint,
            "conditions": {
                "price": target_price,
                "condition": condition
            },
            "enabled": true
        });

        self.mutation("mutations/alerts:createAlert", args).await
    }

    pub async fn get_user_alerts(&self, user_id: &str) -> Result<Vec<Value>> {
        let args = json!({
            "userId": user_id
        });

        self.query("queries/alerts:getUserAlerts", args).await
    }

    // Analytics
    pub async fn calculate_indicators(&self, token_mint: &str) -> Result<Value> {
        let args = json!({
            "tokenMint": token_mint,
            "periods": 100
        });

        self.action("actions/analytics:calculateTokenIndicators", args).await
    }

    // Utility methods
    pub async fn health_check(&self) -> Result<bool> {
        match self.query::<Value>("queries/system:healthCheck", json!({})).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Retry a function with exponential backoff
    pub async fn retry_with_backoff<F, T, E>(&self, mut f: F, max_retries: u32) -> Result<T>
    where
        F: FnMut() -> Result<T, E>,
        E: std::fmt::Display,
    {
        let mut retry_count = 0;
        
        loop {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if retry_count >= max_retries {
                        return Err(anyhow!("Max retries exceeded. Last error: {}", e));
                    }
                    
                    let delay_ms = 1000 * (2_u64.pow(retry_count));
                    sleep(Duration::from_millis(delay_ms)).await;
                    retry_count += 1;
                }
            }
        }
    }
}

// Convenience functions for common operations
impl ConvexClient {
    /// Get user portfolio with automatic user creation if needed
    pub async fn get_or_create_user_portfolio(&self, telegram_id: i64, username: &str) -> Result<(String, PortfolioSummary)> {
        let user = match self.get_user_by_telegram_id(telegram_id).await? {
            Some(user) => user,
            None => {
                let user_id = self.create_or_update_user(telegram_id, username).await?;
                UserProfile {
                    telegram_id,
                    username: username.to_string(),
                    is_premium: false,
                    settings: json!({}),
                }
            }
        };

        // Get the user ID - this would be returned from the create_or_update_user call
        let user_id = format!("user_{}", telegram_id); // Simplified for example
        let portfolio = self.get_portfolio(&user_id).await?;

        Ok((user_id, portfolio))
    }

    /// Execute a trade with proper error handling
    pub async fn execute_trade_with_retry(&self, order: OrderRequest) -> Result<String> {
        let client = self.clone();
        
        self.retry_with_backoff(|| {
            let order = order.clone();
            let client = client.clone();
            
            async move {
                client.place_order(order).await
            }
        }, 3).await
    }

    /// Get comprehensive market data for a token
    pub async fn get_token_data(&self, token_mint: &str) -> Result<Value> {
        let price_data = self.get_token_price(token_mint).await?;
        let indicators = self.calculate_indicators(token_mint).await?;
        
        Ok(json!({
            "price": price_data,
            "indicators": indicators,
            "timestamp": chrono::Utc::now().timestamp()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_convex_client_creation() {
        let client = ConvexClient::new();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        // This would require a running Convex instance
        // Uncomment and configure for integration testing
        /*
        let client = ConvexClient::new().unwrap();
        let health = client.health_check().await;
        assert!(health.is_ok());
        */
    }
}