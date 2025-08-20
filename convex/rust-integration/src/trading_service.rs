use crate::convex_client::{ConvexClient, OrderRequest};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Trading service that integrates Solana trading with Convex backend
#[derive(Clone)]
pub struct TradingService {
    convex: Arc<ConvexClient>,
    jupiter_client: JupiterClient,
}

#[derive(Clone)]
pub struct JupiterClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapRequest {
    pub user_public_key: String,
    pub quote_response: QuoteResponse,
    pub config: SwapConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapConfig {
    pub wrap_and_unwrap_sol: bool,
    pub fee_account: Option<String>,
    pub compute_unit_price_micro_lamports: Option<u64>,
    pub priority_level: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
    pub prioritization_fee_lamports: u64,
}

impl JupiterClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://quote-api.jup.ag".to_string(),
        }
    }

    /// Get a quote for a token swap
    pub async fn get_quote(&self, request: QuoteRequest) -> Result<QuoteResponse> {
        let url = format!("{}/v6/quote", self.base_url);
        
        let params = [
            ("inputMint", request.input_mint.as_str()),
            ("outputMint", request.output_mint.as_str()),
            ("amount", &request.amount.to_string()),
            ("slippageBps", &request.slippage_bps.unwrap_or(100).to_string()),
        ];

        let response = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Jupiter quote failed: {}", response.status()));
        }

        let quote: QuoteResponse = response.json().await?;
        Ok(quote)
    }

    /// Get swap transaction
    pub async fn get_swap_transaction(&self, request: SwapRequest) -> Result<SwapResponse> {
        let url = format!("{}/v6/swap", self.base_url);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter swap failed: {}", error_text));
        }

        let swap_response: SwapResponse = response.json().await?;
        Ok(swap_response)
    }

    /// Get token list
    pub async fn get_token_list(&self) -> Result<Vec<Value>> {
        let url = format!("{}/v6/tokens", self.base_url);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch token list: {}", response.status()));
        }

        let tokens: Vec<Value> = response.json().await?;
        Ok(tokens)
    }
}

impl TradingService {
    pub fn new(convex: Arc<ConvexClient>) -> Self {
        Self {
            convex,
            jupiter_client: JupiterClient::new(),
        }
    }

    /// Execute a market buy order
    pub async fn execute_market_buy(
        &self,
        user_id: &str,
        input_token_mint: &str,
        output_token_mint: &str,
        input_amount: u64,
        slippage_bps: Option<u16>,
        wallet_address: &str,
    ) -> Result<String> {
        // 1. Get quote from Jupiter
        let quote_request = QuoteRequest {
            input_mint: input_token_mint.to_string(),
            output_mint: output_token_mint.to_string(),
            amount: input_amount,
            slippage_bps,
        };

        let quote = self.jupiter_client.get_quote(quote_request).await?;
        
        // 2. Create order in Convex
        let order_request = OrderRequest {
            user_id: user_id.to_string(),
            order_type: "market".to_string(),
            token_mint: output_token_mint.to_string(),
            side: "buy".to_string(),
            amount: quote.out_amount.clone(),
            price: None,
            slippage: slippage_bps.map(|bps| bps as f64 / 10000.0),
        };

        let order_id = self.convex.place_order(order_request).await?;

        // 3. Get swap transaction
        let swap_request = SwapRequest {
            user_public_key: wallet_address.to_string(),
            quote_response: quote,
            config: SwapConfig {
                wrap_and_unwrap_sol: true,
                fee_account: None,
                compute_unit_price_micro_lamports: Some(1000),
                priority_level: Some("medium".to_string()),
            },
        };

        let swap_response = self.jupiter_client.get_swap_transaction(swap_request).await?;

        // 4. Store swap transaction in Convex for execution
        let _ = self.convex.mutation(
            "mutations/trading:storeSwapTransaction",
            json!({
                "orderId": order_id,
                "swapTransaction": swap_response.swap_transaction,
                "lastValidBlockHeight": swap_response.last_valid_block_height,
                "prioritizationFee": swap_response.prioritization_fee_lamports
            })
        ).await?;

        Ok(order_id)
    }

    /// Execute a market sell order
    pub async fn execute_market_sell(
        &self,
        user_id: &str,
        input_token_mint: &str,
        output_token_mint: &str,
        input_amount: u64,
        slippage_bps: Option<u16>,
        wallet_address: &str,
    ) -> Result<String> {
        // Similar to buy but with reversed tokens
        let quote_request = QuoteRequest {
            input_mint: input_token_mint.to_string(),
            output_mint: output_token_mint.to_string(),
            amount: input_amount,
            slippage_bps,
        };

        let quote = self.jupiter_client.get_quote(quote_request).await?;
        
        let order_request = OrderRequest {
            user_id: user_id.to_string(),
            order_type: "market".to_string(),
            token_mint: input_token_mint.to_string(),
            side: "sell".to_string(),
            amount: input_amount.to_string(),
            price: None,
            slippage: slippage_bps.map(|bps| bps as f64 / 10000.0),
        };

        let order_id = self.convex.place_order(order_request).await?;

        let swap_request = SwapRequest {
            user_public_key: wallet_address.to_string(),
            quote_response: quote,
            config: SwapConfig {
                wrap_and_unwrap_sol: true,
                fee_account: None,
                compute_unit_price_micro_lamports: Some(1000),
                priority_level: Some("medium".to_string()),
            },
        };

        let swap_response = self.jupiter_client.get_swap_transaction(swap_request).await?;

        let _ = self.convex.mutation(
            "mutations/trading:storeSwapTransaction",
            json!({
                "orderId": order_id,
                "swapTransaction": swap_response.swap_transaction,
                "lastValidBlockHeight": swap_response.last_valid_block_height,
                "prioritizationFee": swap_response.prioritization_fee_lamports
            })
        ).await?;

        Ok(order_id)
    }

    /// Get the best route for a swap
    pub async fn get_best_route(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<QuoteResponse> {
        let quote_request = QuoteRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount,
            slippage_bps: Some(100), // 1% default
        };

        self.jupiter_client.get_quote(quote_request).await
    }

    /// Calculate price impact for a trade
    pub async fn calculate_price_impact(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<f64> {
        let quote = self.get_best_route(input_mint, output_mint, amount).await?;
        
        let price_impact = quote.price_impact_pct
            .parse::<f64>()
            .unwrap_or(0.0);
        
        Ok(price_impact)
    }

    /// Monitor order execution
    pub async fn monitor_order_execution(&self, order_id: &str) -> Result<String> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 60; // 5 minutes with 5-second intervals
        
        while attempts < MAX_ATTEMPTS {
            let order_status = self.convex.get_order_status(order_id).await?;
            
            let status = order_status["status"].as_str().unwrap_or("unknown");
            
            match status {
                "completed" => {
                    let tx_signature = order_status["transactionSignature"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();
                    return Ok(tx_signature);
                }
                "failed" => {
                    let error = order_status["error"]
                        .as_str()
                        .unwrap_or("Unknown error");
                    return Err(anyhow!("Order failed: {}", error));
                }
                "pending" | "submitted" => {
                    // Continue monitoring
                }
                _ => {
                    return Err(anyhow!("Unknown order status: {}", status));
                }
            }
            
            sleep(Duration::from_secs(5)).await;
            attempts += 1;
        }
        
        Err(anyhow!("Order monitoring timeout"))
    }

    /// Execute DCA strategy
    pub async fn execute_dca_order(
        &self,
        strategy_id: &str,
        user_id: &str,
        wallet_address: &str,
    ) -> Result<String> {
        // Get DCA strategy details from Convex
        let strategy = self.convex.query(
            "queries/dca:getStrategy",
            json!({ "strategyId": strategy_id })
        ).await?;

        let from_mint = strategy["fromMint"].as_str()
            .ok_or_else(|| anyhow!("Invalid from mint"))?;
        let to_mint = strategy["toMint"].as_str()
            .ok_or_else(|| anyhow!("Invalid to mint"))?;
        let amount_str = strategy["amount"].as_str()
            .ok_or_else(|| anyhow!("Invalid amount"))?;
        let amount = amount_str.parse::<f64>()
            .map_err(|_| anyhow!("Cannot parse amount"))?;

        // Convert USD amount to token amount (assuming SOL as base)
        let sol_price = self.get_sol_price().await?;
        let sol_amount = (amount / sol_price) as u64 * 1_000_000_000; // Convert to lamports

        // Execute the DCA buy order
        self.execute_market_buy(
            user_id,
            from_mint,
            to_mint,
            sol_amount,
            Some(200), // 2% slippage for DCA
            wallet_address,
        ).await
    }

    /// Get current SOL price in USD
    async fn get_sol_price(&self) -> Result<f64> {
        let price_data = self.convex.get_token_price(
            "So11111111111111111111111111111111111111112"
        ).await?;

        price_data["price"].as_f64()
            .ok_or_else(|| anyhow!("Invalid SOL price data"))
    }

    /// Validate trading parameters
    pub fn validate_trade_params(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: Option<u16>,
    ) -> Result<()> {
        // Validate mints (basic validation)
        if input_mint.len() != 44 || output_mint.len() != 44 {
            return Err(anyhow!("Invalid token mint address"));
        }

        if input_mint == output_mint {
            return Err(anyhow!("Input and output mints cannot be the same"));
        }

        // Validate amount
        if amount == 0 {
            return Err(anyhow!("Amount must be greater than 0"));
        }

        // Validate slippage
        if let Some(slippage) = slippage_bps {
            if slippage > 5000 { // 50% max slippage
                return Err(anyhow!("Slippage too high (max 50%)"));
            }
        }

        Ok(())
    }

    /// Get supported tokens
    pub async fn get_supported_tokens(&self) -> Result<Vec<Value>> {
        self.jupiter_client.get_token_list().await
    }

    /// Calculate trade fees
    pub async fn calculate_trade_fees(&self, quote: &QuoteResponse) -> Result<Value> {
        let input_amount = quote.in_amount.parse::<u64>().unwrap_or(0);
        let output_amount = quote.out_amount.parse::<u64>().unwrap_or(0);
        
        // Jupiter fee is typically 0.25% but varies by route
        let jupiter_fee = (input_amount as f64 * 0.0025) as u64;
        
        // Solana network fee
        let network_fee = 5000; // ~0.000005 SOL
        
        Ok(json!({
            "jupiterFee": jupiter_fee,
            "networkFee": network_fee,
            "totalFee": jupiter_fee + network_fee,
            "feePercentage": ((jupiter_fee + network_fee) as f64 / input_amount as f64) * 100.0
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_jupiter_client() {
        let client = JupiterClient::new();
        
        // Test token list fetch
        let tokens = client.get_token_list().await;
        assert!(tokens.is_ok());
    }

    #[tokio::test]
    async fn test_trade_validation() {
        let convex = Arc::new(ConvexClient::new().unwrap());
        let service = TradingService::new(convex);
        
        // Valid parameters
        let result = service.validate_trade_params(
            "So11111111111111111111111111111111111111112",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            1000000000,
            Some(100)
        );
        assert!(result.is_ok());
        
        // Invalid parameters
        let result = service.validate_trade_params(
            "invalid_mint",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            1000000000,
            Some(100)
        );
        assert!(result.is_err());
    }
}