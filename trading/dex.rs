use crate::errors::{TradingError, Result};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::transaction::Transaction;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, debug, warn, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: f64,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlanStep {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: Option<String>,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

#[derive(Debug, Clone)]
struct CachedQuote {
    quote: JupiterQuote,
    expires_at: std::time::Instant,
}

impl CachedQuote {
    fn new(quote: JupiterQuote, ttl_seconds: u64) -> Self {
        Self {
            quote,
            expires_at: std::time::Instant::now() + Duration::from_secs(ttl_seconds),
        }
    }
    
    fn is_expired(&self) -> bool {
        std::time::Instant::now() > self.expires_at
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterSwapRequest {
    #[serde(rename = "quoteResponse")]
    pub quote_response: JupiterQuote,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "useSharedAccounts")]
    pub use_shared_accounts: bool,
    #[serde(rename = "feeAccount")]
    pub fee_account: Option<String>,
    #[serde(rename = "trackingAccount")]
    pub tracking_account: Option<String>,
    #[serde(rename = "computeUnitPriceMicroLamports")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "asLegacyTransaction")]
    pub as_legacy_transaction: bool,
    #[serde(rename = "useTokenLedger")]
    pub use_token_ledger: bool,
    #[serde(rename = "destinationTokenAccount")]
    pub destination_token_account: Option<String>,
    #[serde(rename = "dynamicComputeUnitLimit")]
    pub dynamic_compute_unit_limit: bool,
    #[serde(rename = "skipUserAccountsRpcCalls")]
    pub skip_user_accounts_rpc_calls: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
}

pub struct JupiterSwap {
    client: Client,
    api_url: String,
    price_api_url: String,
    // High-performance features
    quote_cache: Arc<RwLock<HashMap<String, CachedQuote>>>,
    price_cache: Arc<RwLock<HashMap<String, (f64, std::time::Instant)>>>,
    rate_limiter: Arc<Semaphore>, // Rate limiting for API calls
    // Request deduplication (prevent identical concurrent requests)
    pending_quotes: Arc<RwLock<HashMap<String, Arc<tokio::sync::Notify>>>>,
}

impl JupiterSwap {
    pub fn new(_rpc_url: String) -> Self {
        // Create optimized HTTP client with connection pooling
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(15)) // Reduced timeout for faster failure detection
            .pool_idle_timeout(Duration::from_secs(90)) // Keep connections alive
            .pool_max_idle_per_host(10) // Connection pool per host
            .connect_timeout(Duration::from_secs(5)) // Quick connection timeout
            .tcp_keepalive(Duration::from_secs(60)) // Keep TCP connections alive
            .http2_prior_knowledge() // Use HTTP/2 when possible
            .gzip(true) // Enable compression
            .brotli(true) // Enable brotli compression
            .user_agent("solana-trading-bot/0.1.0") // Identify our bot
            .build()
            .expect("Failed to create optimized HTTP client");
        
        Self {
            client,
            api_url: "https://quote-api.jup.ag/v6".to_string(),
            price_api_url: "https://price.jup.ag/v6".to_string(),
            quote_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(Semaphore::new(10)), // Max 10 concurrent requests
            pending_quotes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    #[instrument(skip(self), fields(input_mint, output_mint, amount, slippage_bps))]
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: f64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let amount_lamports = (amount * 1e9) as u64;
        
        // Create cache key for deduplication
        let cache_key = format!("{}:{}:{}:{}", input_mint, output_mint, amount_lamports, slippage_bps);
        
        // Check cache first
        {
            let cache = self.quote_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if !cached.is_expired() {
                    debug!("Returning cached quote for {}", cache_key);
                    return Ok(cached.quote.clone());
                }
            }
        }
        
        // Request deduplication - check if identical request is in progress
        let notify = {
            let mut pending = self.pending_quotes.write().await;
            if let Some(existing_notify) = pending.get(&cache_key) {
                let notify = existing_notify.clone();
                drop(pending);
                
                debug!("Waiting for duplicate request to complete: {}", cache_key);
                notify.notified().await;
                
                // Check cache again after waiting
                let cache = self.quote_cache.read().await;
                if let Some(cached) = cache.get(&cache_key) {
                    if !cached.is_expired() {
                        return Ok(cached.quote.clone());
                    }
                }
            } else {
                let notify = Arc::new(tokio::sync::Notify::new());
                pending.insert(cache_key.clone(), notify.clone());
                notify
            }
        };
        
        // Acquire rate limiter permit
        let _permit = self.rate_limiter.acquire().await
            .map_err(|_| TradingError::QuoteFailed("Rate limiter closed".to_string()))?;
        
        // Make the actual API call
        let result = self.fetch_quote_from_api(input_mint, output_mint, amount_lamports, slippage_bps).await;
        
        // Clean up pending request and notify waiters
        {
            let mut pending = self.pending_quotes.write().await;
            pending.remove(&cache_key);
        }
        notify.notify_waiters();
        
        match result {
            Ok(quote) => {
                // Cache the successful result for 5 seconds (quotes change frequently)
                {
                    let mut cache = self.quote_cache.write().await;
                    cache.insert(cache_key, CachedQuote::new(quote.clone(), 5));
                }
                
                info!(
                    "Quote received: {} {} -> {} {}, price impact: {:.4}%, routes: {}",
                    amount,
                    input_mint,
                    quote.out_amount.parse::<u64>().unwrap_or(0) as f64 / 1e9,
                    output_mint,
                    quote.price_impact_pct,
                    quote.route_plan.len()
                );
                
                if quote.price_impact_pct > 5.0 {
                    warn!("High price impact detected: {:.2}%", quote.price_impact_pct);
                }
                
                Ok(quote)
            }
            Err(e) => Err(e)
        }
    }
    
    // Separated API call for cleaner code and better error handling
    async fn fetch_quote_from_api(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount_lamports: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false&asLegacyTransaction=false",
            self.api_url,
            input_mint,
            output_mint,
            amount_lamports,
            slippage_bps
        );
        
        debug!("Fetching Jupiter V6 quote: {}", url);
        
        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .header("Cache-Control", "no-cache") // Ensure fresh data
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(TradingError::QuoteFailed(
                format!("Jupiter quote failed ({}): {}", status, error_text)
            ).into());
        }
        
        let quote: JupiterQuote = response.json().await
            .map_err(|e| TradingError::QuoteFailed(format!("Failed to parse quote response: {}", e)))?;
        
        Ok(quote)
    }
    
    pub async fn get_quote_with_retry(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: f64,
        slippage_bps: u16,
        max_retries: u32,
    ) -> Result<JupiterQuote> {
        let mut attempts = 0;
        loop {
            match self.get_quote(input_mint, output_mint, amount, slippage_bps).await {
                Ok(quote) => return Ok(quote),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retries {
                        return Err(e);
                    }
                    warn!("Quote attempt {} failed, retrying: {}", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempts as u64)).await;
                }
            }
        }
    }
    
    pub async fn build_swap_transaction(
        &self,
        quote: JupiterQuote,
        user_public_key: &str,
        priority_fee_lamports: u64,
    ) -> Result<Transaction> {
        let swap_request = JupiterSwapRequest {
            quote_response: quote.clone(),
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: Some(priority_fee_lamports * 1000),
            prioritization_fee_lamports: Some(priority_fee_lamports),
            as_legacy_transaction: false,
            use_token_ledger: false,
            destination_token_account: None,
            dynamic_compute_unit_limit: true,
            skip_user_accounts_rpc_calls: false,
        };
        
        debug!("Building swap transaction for user: {}", user_public_key);
        
        let response = self.client
            .post(format!("{}/swap", self.api_url))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&swap_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(TradingError::TransactionFailed(format!("Jupiter swap failed: {}", error_text)).into());
        }
        
        let swap_response: JupiterSwapResponse = response.json().await?;
        
        info!(
            "Swap transaction built, valid until block: {}, priority fee: {} lamports",
            swap_response.last_valid_block_height,
            swap_response.prioritization_fee_lamports.unwrap_or(0)
        );
        
        let tx_bytes = base64::decode(&swap_response.swap_transaction)?;
        let tx: Transaction = bincode::deserialize(&tx_bytes)?;
        
        Ok(tx)
    }
    
    pub async fn simulate_swap(
        &self,
        quote: &JupiterQuote,
        user_public_key: &str,
    ) -> Result<bool> {
        debug!("Simulating swap for validation");
        
        let simulation_url = format!("{}/swap/simulate", self.api_url);
        let simulation_request = serde_json::json!({
            "quoteResponse": quote,
            "userPublicKey": user_public_key,
        });
        
        let response = self.client
            .post(&simulation_url)
            .json(&simulation_request)
            .send()
            .await?;
        
        if response.status().is_success() {
            info!("Swap simulation successful");
            Ok(true)
        } else {
            let error_text = response.text().await?;
            warn!("Swap simulation failed: {}", error_text);
            Ok(false)
        }
    }
    
    #[instrument(skip(self), fields(mint))]
    pub async fn get_token_price(&self, mint: &str) -> Result<f64> {
        // Check cache first (prices cached for 30 seconds)
        {
            let cache = self.price_cache.read().await;
            if let Some((price, cached_at)) = cache.get(mint) {
                if cached_at.elapsed() < Duration::from_secs(30) {
                    debug!("Returning cached price for {}: ${}", mint, price);
                    return Ok(*price);
                }
            }
        }
        
        // Acquire rate limiter permit
        let _permit = self.rate_limiter.acquire().await
            .map_err(|_| TradingError::TokenNotFound("Rate limiter closed".to_string()))?;
        
        let url = format!("{}/price?ids={}", self.price_api_url, mint);
        
        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(TradingError::TokenNotFound(
                format!("Failed to get token price ({}): {}", status, error_text)
            ).into());
        }
        
        let data: Value = response.json().await?;
        
        let price = data["data"][mint]["price"]
            .as_f64()
            .unwrap_or(0.0);
        
        // Cache the price
        {
            let mut cache = self.price_cache.write().await;
            cache.insert(mint.to_string(), (price, std::time::Instant::now()));
        }
        
        debug!("Token {} price: ${} (cached)", mint, price);
        
        Ok(price)
    }
    
    pub async fn get_token_prices(&self, mints: &[&str]) -> Result<std::collections::HashMap<String, f64>> {
        let ids = mints.join(",");
        let url = format!("{}/price?ids={}", self.price_api_url, ids);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        let data: Value = response.json().await?;
        let mut prices = std::collections::HashMap::new();
        
        if let Some(price_data) = data["data"].as_object() {
            for (mint, price_info) in price_data {
                if let Some(price) = price_info["price"].as_f64() {
                    prices.insert(mint.clone(), price);
                }
            }
        }
        
        Ok(prices)
    }
    
    /// Clean up expired cache entries to prevent memory leaks
    pub async fn cleanup_caches(&self) {
        // Clean up expired quotes
        {
            let mut cache = self.quote_cache.write().await;
            cache.retain(|_, cached_quote| !cached_quote.is_expired());
        }
        
        // Clean up expired prices (older than 5 minutes)
        {
            let mut cache = self.price_cache.write().await;
            let cutoff = std::time::Instant::now() - Duration::from_secs(300);
            cache.retain(|_, (_, cached_at)| *cached_at > cutoff);
        }
        
        debug!("Cache cleanup completed");
    }
    
    /// Get cache statistics for monitoring
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let quote_count = self.quote_cache.read().await.len();
        let price_count = self.price_cache.read().await.len();
        (quote_count, price_count)
    }
}