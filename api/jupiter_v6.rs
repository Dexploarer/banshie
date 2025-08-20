use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::{DateTime, Utc};

use crate::errors::{BotError, Result};
use crate::telemetry::TelemetryService;

/// Jupiter API v6 client with enhanced 2025 features
#[derive(Clone)]
pub struct JupiterV6Client {
    client: Client,
    api_tier: ApiTier,
    base_url: String,
    telemetry: Option<Arc<TelemetryService>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

/// API tier configuration for Jupiter v6
#[derive(Debug, Clone)]
pub enum ApiTier {
    Lite,                    // Free, no API key
    Ultra { api_key: String }, // Free with registration, enhanced limits
    Pro { api_key: String },   // Paid, custom limits
}

impl ApiTier {
    pub fn base_url(&self) -> &'static str {
        match self {
            ApiTier::Lite => "https://lite-api.jup.ag",
            ApiTier::Ultra { .. } => "https://api.jup.ag/ultra",
            ApiTier::Pro { .. } => "https://api.jup.ag",
        }
    }
    
    pub fn api_key(&self) -> Option<&str> {
        match self {
            ApiTier::Lite => None,
            ApiTier::Ultra { api_key } | ApiTier::Pro { api_key } => Some(api_key),
        }
    }
}

/// Enhanced quote request with v6 features
#[derive(Debug, Serialize)]
pub struct QuoteRequestV6 {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    pub amount: u64,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    
    // V6 enhanced features
    #[serde(rename = "swapMode", skip_serializing_if = "Option::is_none")]
    pub swap_mode: Option<SwapMode>,
    #[serde(rename = "dexes", skip_serializing_if = "Option::is_none")]
    pub dexes: Option<Vec<String>>,
    #[serde(rename = "excludeDexes", skip_serializing_if = "Option::is_none")]
    pub exclude_dexes: Option<Vec<String>>,
    #[serde(rename = "maxAccounts", skip_serializing_if = "Option::is_none")]
    pub max_accounts: Option<u8>, // Max 64
    #[serde(rename = "quoteMint", skip_serializing_if = "Option::is_none")]
    pub quote_mint: Option<String>,
    #[serde(rename = "minimizeSlippage", skip_serializing_if = "Option::is_none")]
    pub minimize_slippage: Option<bool>,
    #[serde(rename = "onlyDirectRoutes", skip_serializing_if = "Option::is_none")]
    pub only_direct_routes: Option<bool>,
}

/// Swap modes for Jupiter v6
#[derive(Debug, Serialize)]
pub enum SwapMode {
    ExactIn,
    ExactOut,
}

/// Enhanced quote response with v6 data
#[derive(Debug, Deserialize)]
pub struct QuoteResponseV6 {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<PlatformFee>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlan>,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

#[derive(Debug, Deserialize)]
pub struct RoutePlan {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Deserialize)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: String,
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

/// Enhanced swap request with v6 features
#[derive(Debug, Serialize)]
pub struct SwapRequestV6 {
    #[serde(rename = "quoteResponse")]
    pub quote_response: QuoteResponseV6,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    
    // V6 enhanced transaction features
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "useSharedAccounts")]
    pub use_shared_accounts: bool,
    #[serde(rename = "feeAccount", skip_serializing_if = "Option::is_none")]
    pub fee_account: Option<String>,
    #[serde(rename = "trackingAccount", skip_serializing_if = "Option::is_none")]
    pub tracking_account: Option<String>,
    #[serde(rename = "computeUnitPriceMicroLamports", skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports", skip_serializing_if = "Option::is_none")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "asLegacyTransaction", skip_serializing_if = "Option::is_none")]
    pub as_legacy_transaction: Option<bool>,
    #[serde(rename = "useTokenLedger", skip_serializing_if = "Option::is_none")]
    pub use_token_ledger: Option<bool>,
    #[serde(rename = "destinationTokenAccount", skip_serializing_if = "Option::is_none")]
    pub destination_token_account: Option<String>,
}

/// Swap response with transaction data
#[derive(Debug, Deserialize)]
pub struct SwapResponseV6 {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String, // Base64 encoded
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "computeUnitLimit")]
    pub compute_unit_limit: Option<u64>,
    #[serde(rename = "dynamicSlippageReport", skip_serializing_if = "Option::is_none")]
    pub dynamic_slippage_report: Option<DynamicSlippageReport>,
    #[serde(rename = "simulationError", skip_serializing_if = "Option::is_none")]
    pub simulation_error: Option<SimulationError>,
}

#[derive(Debug, Deserialize)]
pub struct DynamicSlippageReport {
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
}

#[derive(Debug, Deserialize)]
pub struct SimulationError {
    pub error: String,
    pub message: String,
}

/// Price API V3 structures
#[derive(Debug, Deserialize)]
pub struct PriceResponseV3 {
    #[serde(flatten)]
    pub prices: HashMap<String, PriceDataV3>,
}

#[derive(Debug, Deserialize)]
pub struct PriceDataV3 {
    #[serde(rename = "usdPrice")]
    pub usd_price: f64,
    #[serde(rename = "blockId")]
    pub block_id: u64,
    pub decimals: u8,
    #[serde(rename = "priceChange24h")]
    pub price_change_24h: Option<f64>,
}

/// Token API V2 structures
#[derive(Debug, Deserialize)]
pub struct TokenResponseV2 {
    pub tokens: Vec<TokenDataV2>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TokenDataV2 {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    pub tags: Vec<String>,
    #[serde(rename = "organicScore")]
    pub organic_score: Option<f64>,
    #[serde(rename = "socialScore")]
    pub social_score: Option<f64>,
    pub verified: bool,
    #[serde(rename = "dailyVolume")]
    pub daily_volume: Option<u64>,
    pub freeze_authority: Option<String>,
    pub mint_authority: Option<String>,
}

/// Rate limiter for API calls
#[derive(Debug)]
struct RateLimiter {
    requests: HashMap<String, Vec<DateTime<Utc>>>,
    limits: RateLimits,
}

#[derive(Debug)]
struct RateLimits {
    per_minute: usize,
    per_hour: usize,
    per_day: usize,
}

impl JupiterV6Client {
    /// Create new Jupiter v6 client
    pub fn new(api_tier: ApiTier, telemetry: Option<Arc<TelemetryService>>) -> Self {
        let base_url = api_tier.base_url().to_string();
        
        let limits = match api_tier {
            ApiTier::Lite => RateLimits { per_minute: 10, per_hour: 100, per_day: 1000 },
            ApiTier::Ultra { .. } => RateLimits { per_minute: 60, per_hour: 1000, per_day: 10000 },
            ApiTier::Pro { .. } => RateLimits { per_minute: 600, per_hour: 10000, per_day: 100000 },
        };
        
        info!("🪐 Initialized Jupiter v6 client with {:?} tier", std::mem::discriminant(&api_tier));
        
        Self {
            client: Client::new(),
            api_tier,
            base_url,
            telemetry,
            rate_limiter: Arc::new(RwLock::new(RateLimiter {
                requests: HashMap::new(),
                limits,
            })),
        }
    }
    
    /// Get quote using Jupiter v6 API
    pub async fn get_quote(&self, request: QuoteRequestV6) -> Result<QuoteResponseV6> {
        self.check_rate_limit("quote").await?;
        
        let url = format!("{}/v6/quote", self.base_url);
        
        // Create tracing context if telemetry available
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_jupiter_span(&url, "GET")
        );
        
        let mut req = self.client
            .get(&url)
            .query(&request);
            
        // Add API key header if available
        if let Some(api_key) = self.api_tier.api_key() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Quote request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Quote failed with status {}: {}", status, error_text
            )).into());
        }
        
        let quote = response
            .json::<QuoteResponseV6>()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse quote response: {}", e)))?;
            
        debug!("✅ Got Jupiter quote: {} {} -> {} {}", 
            quote.in_amount, quote.input_mint, quote.out_amount, quote.output_mint);
            
        Ok(quote)
    }
    
    /// Execute swap using Jupiter v6 API
    pub async fn execute_swap(&self, request: SwapRequestV6) -> Result<SwapResponseV6> {
        self.check_rate_limit("swap").await?;
        
        let url = format!("{}/v6/swap", self.base_url);
        
        // Create tracing context if telemetry available
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_jupiter_span(&url, "POST")
        );
        
        let mut req = self.client
            .post(&url)
            .json(&request);
            
        // Add API key header if available
        if let Some(api_key) = self.api_tier.api_key() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Swap request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Swap failed with status {}: {}", status, error_text
            )).into());
        }
        
        let swap_response = response
            .json::<SwapResponseV6>()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse swap response: {}", e)))?;
            
        info!("✅ Generated swap transaction, last valid block: {}", 
            swap_response.last_valid_block_height);
            
        Ok(swap_response)
    }
    
    /// Get token prices using Price API V3
    pub async fn get_token_prices_v3(&self, token_mints: Vec<String>) -> Result<PriceResponseV3> {
        self.check_rate_limit("price").await?;
        
        let url = format!("{}/price/v3", self.base_url);
        let ids = token_mints.join(",");
        
        let mut req = self.client
            .get(&url)
            .query(&[("ids", &ids)]);
            
        // Add API key header if available
        if let Some(api_key) = self.api_tier.api_key() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Price request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Price API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let prices = response
            .json::<PriceResponseV3>()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse price response: {}", e)))?;
            
        debug!("📈 Retrieved prices for {} tokens", prices.prices.len());
        
        Ok(prices)
    }
    
    /// Get tokens using Token API V2
    pub async fn get_tokens_v2(&self) -> Result<TokenResponseV2> {
        self.check_rate_limit("tokens").await?;
        
        let url = format!("{}/token/v2/tokens", self.base_url);
        
        let mut req = self.client.get(&url);
            
        // Add API key header if available
        if let Some(api_key) = self.api_tier.api_key() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Token request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Token API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let tokens = response
            .json::<TokenResponseV2>()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse tokens response: {}", e)))?;
            
        info!("🪙 Retrieved {} tokens from Jupiter Token API V2", tokens.tokens.len());
        
        Ok(tokens)
    }
    
    /// Check rate limits
    async fn check_rate_limit(&self, endpoint: &str) -> Result<()> {
        let mut limiter = self.rate_limiter.write().await;
        let now = Utc::now();
        let key = format!("{}_{}", endpoint, match &self.api_tier {
            ApiTier::Lite => "lite",
            ApiTier::Ultra { .. } => "ultra", 
            ApiTier::Pro { .. } => "pro",
        });
        
        // Clean old requests
        let requests = limiter.requests.entry(key.clone()).or_insert_with(Vec::new);
        requests.retain(|&timestamp| {
            now.signed_duration_since(timestamp).num_minutes() < 60
        });
        
        // Check per-minute limit
        let recent_requests = requests.iter()
            .filter(|&&timestamp| now.signed_duration_since(timestamp).num_minutes() < 1)
            .count();
            
        if recent_requests >= limiter.limits.per_minute {
            return Err(BotError::rate_limited(format!(
                "Rate limit exceeded for {}: {} requests per minute", 
                endpoint, limiter.limits.per_minute
            )).into());
        }
        
        requests.push(now);
        Ok(())
    }
}

/// Helper function to create default swap request
pub fn create_enhanced_swap_request(
    quote_response: QuoteResponseV6,
    user_public_key: String,
) -> SwapRequestV6 {
    SwapRequestV6 {
        quote_response,
        user_public_key,
        wrap_and_unwrap_sol: true,
        use_shared_accounts: true,
        fee_account: None,
        tracking_account: None,
        compute_unit_price_micro_lamports: Some(1000), // 0.001 SOL per compute unit
        prioritization_fee_lamports: Some(5000), // 0.000005 SOL priority fee
        as_legacy_transaction: Some(false), // Use versioned transactions
        use_token_ledger: Some(false),
        destination_token_account: None,
    }
}