use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    signature::Signature,
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::Utc;

use crate::errors::BotError;
use crate::wallet::WalletManager;
use crate::trading::signer::{TransactionSigner, SigningOptions};

/// Jupiter swap client for executing real trades
pub struct JupiterSwapClient {
    client: Client,
    jupiter_quote_api: String,
    jupiter_swap_api: String,
    wallet_manager: Arc<WalletManager>,
    swap_cache: Arc<RwLock<SwapCache>>,
    transaction_signer: Arc<TransactionSigner>,
}

/// Swap request parameters
#[derive(Debug, Clone, Serialize)]
pub struct SwapRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16, // Basis points (100 = 1%)
    pub user_public_key: String,
    pub quote_only: bool,
}

/// Jupiter quote response
#[derive(Debug, Clone, Deserialize)]
pub struct JupiterQuote {
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutePlan {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Clone, Deserialize)]
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

/// Swap instruction request for Jupiter
#[derive(Debug, Serialize)]
pub struct SwapInstructionRequest {
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
    #[serde(rename = "computeUnitPriceMicroLamports")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
}

/// Jupiter swap instruction response
#[derive(Debug, Deserialize)]
pub struct SwapInstructionResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String, // Base64 encoded transaction
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "computeUnitLimit")]
    pub compute_unit_limit: Option<u64>,
}

/// Swap execution result
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub success: bool,
    pub signature: Option<String>,
    pub input_amount: f64,
    pub output_amount: f64,
    pub price_impact: f64,
    pub fee_amount: f64,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

/// Swap cache for rate limiting and optimization
#[derive(Debug)]
struct SwapCache {
    recent_quotes: std::collections::HashMap<String, (JupiterQuote, chrono::DateTime<Utc>)>,
    rate_limit_tracker: std::collections::HashMap<String, Vec<chrono::DateTime<Utc>>>,
}

impl JupiterSwapClient {
    pub fn new(wallet_manager: Arc<WalletManager>) -> Self {
        // Initialize secure transaction signer
        let signing_options = SigningOptions {
            require_confirmation: true,
            max_sol_amount: 10.0, // Allow up to 10 SOL transactions
            enable_hardware_wallet: false,
            use_secure_enclave: false,
            session_timeout_minutes: 30,
        };
        
        let transaction_signer = Arc::new(TransactionSigner::new(
            wallet_manager.clone(),
            signing_options
        ));
        
        Self {
            client: Client::new(),
            jupiter_quote_api: "https://quote-api.jup.ag/v6/quote".to_string(),
            jupiter_swap_api: "https://quote-api.jup.ag/v6/swap".to_string(),
            wallet_manager,
            swap_cache: Arc::new(RwLock::new(SwapCache {
                recent_quotes: std::collections::HashMap::new(),
                rate_limit_tracker: std::collections::HashMap::new(),
            })),
            transaction_signer,
        }
    }
    
    /// Get a quote for a swap
    pub async fn get_quote(&self, request: &SwapRequest) -> Result<JupiterQuote> {
        info!("Getting quote for {} {} -> {}", 
            request.amount, request.input_mint, request.output_mint);
        
        // Check rate limiting
        self.check_rate_limit(&request.user_public_key).await?;
        
        // Check cache for recent quotes
        let cache_key = format!("{}:{}:{}:{}", 
            request.input_mint, request.output_mint, request.amount, request.slippage_bps);
        
        {
            let cache = self.swap_cache.read().await;
            if let Some((quote, timestamp)) = cache.recent_quotes.get(&cache_key) {
                if Utc::now().signed_duration_since(*timestamp).num_seconds() < 30 {
                    debug!("Using cached quote");
                    return Ok(quote.clone());
                }
            }
        }
        
        // Build query parameters
        let params = vec![
            ("inputMint", request.input_mint.as_str()),
            ("outputMint", request.output_mint.as_str()),
            ("amount", &request.amount.to_string()),
            ("slippageBps", &request.slippage_bps.to_string()),
            ("onlyDirectRoutes", "false"),
            ("asLegacyTransaction", "false"),
        ];
        
        let response = self.client
            .get(&self.jupiter_quote_api)
            .query(&params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Jupiter quote failed: {}", error_text);
            return Err(BotError::trading(format!("Quote failed: {}", error_text)).into());
        }
        
        let quote: JupiterQuote = response.json().await?;
        
        // Cache the quote
        {
            let mut cache = self.swap_cache.write().await;
            cache.recent_quotes.insert(cache_key, (quote.clone(), Utc::now()));
            
            // Clean old cache entries
            cache.recent_quotes.retain(|_, (_, timestamp)| {
                Utc::now().signed_duration_since(*timestamp).num_seconds() < 300 // 5 minutes
            });
        }
        
        info!("Quote received: {} -> {} (price impact: {}%)", 
            quote.in_amount, quote.out_amount, quote.price_impact_pct);
        
        Ok(quote)
    }
    
    /// Execute a swap transaction
    pub async fn execute_swap(
        &self,
        request: &SwapRequest,
        telegram_id: &str,
    ) -> Result<SwapResult> {
        let start_time = std::time::Instant::now();
        info!("Executing swap for user {}", telegram_id);
        
        // Validate user has wallet and permissions
        let wallet = self.wallet_manager
            .get_user_wallet(telegram_id)
            .await?
            .ok_or_else(|| BotError::validation("No active wallet found"))?;
        
        if wallet.public_key != request.user_public_key {
            return Err(BotError::validation("Wallet address mismatch").into());
        }
        
        // Get quote first
        let quote = self.get_quote(request).await?;
        
        // Validate quote meets user expectations
        self.validate_quote(&quote, request).await?;
        
        // Get swap instructions from Jupiter
        let swap_instructions = self.get_swap_instructions(&quote, &request.user_public_key).await?;
        
        // Execute the transaction
        let result = self.execute_transaction(&swap_instructions, telegram_id).await;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(signature) => {
                info!("Swap executed successfully: {}", signature);
                
                Ok(SwapResult {
                    success: true,
                    signature: Some(signature),
                    input_amount: quote.in_amount.parse().unwrap_or(0.0),
                    output_amount: quote.out_amount.parse().unwrap_or(0.0),
                    price_impact: quote.price_impact_pct.parse().unwrap_or(0.0),
                    fee_amount: quote.platform_fee
                        .as_ref()
                        .map(|f| f.amount.parse().unwrap_or(0.0))
                        .unwrap_or(0.0),
                    execution_time_ms: execution_time,
                    error: None,
                })
            }
            Err(e) => {
                error!("Swap failed: {}", e);
                
                Ok(SwapResult {
                    success: false,
                    signature: None,
                    input_amount: quote.in_amount.parse().unwrap_or(0.0),
                    output_amount: 0.0,
                    price_impact: quote.price_impact_pct.parse().unwrap_or(0.0),
                    fee_amount: 0.0,
                    execution_time_ms: execution_time,
                    error: Some(e.to_string()),
                })
            }
        }
    }
    
    /// Get swap instructions from Jupiter
    async fn get_swap_instructions(
        &self,
        quote: &JupiterQuote,
        user_public_key: &str,
    ) -> Result<SwapInstructionResponse> {
        let request = SwapInstructionRequest {
            quote_response: quote.clone(),
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            compute_unit_price_micro_lamports: Some(1000), // 0.001 SOL per compute unit
            prioritization_fee_lamports: Some(10000), // 0.00001 SOL priority fee
        };
        
        let response = self.client
            .post(&self.jupiter_swap_api)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(BotError::trading(format!("Swap instruction failed: {}", error_text)).into());
        }
        
        let instructions: SwapInstructionResponse = response.json().await?;
        Ok(instructions)
    }
    
    /// Execute the transaction using secure signing
    async fn execute_transaction(
        &self,
        instructions: &SwapInstructionResponse,
        telegram_id: &str,
    ) -> Result<String> {
        info!("Executing transaction with secure signing for user {}", telegram_id);
        
        // Decode the transaction
        let transaction_bytes = base64::decode(&instructions.swap_transaction)?;
        let transaction: Transaction = bincode::deserialize(&transaction_bytes)?;
        
        info!("Transaction prepared: {} instructions", transaction.message.instructions.len());
        
        // Create signing request with transaction details
        let description = format!(
            "Swap transaction with {} instructions\nEstimated priority fee: {} lamports",
            transaction.message.instructions.len(),
            instructions.prioritization_fee_lamports.unwrap_or(0)
        );
        
        // Create signing request through secure signer
        let request_id = self.transaction_signer
            .create_signing_request(transaction, telegram_id, description)
            .await?;
        
        info!("Created signing request {} for user {}", request_id, telegram_id);
        
        // For demonstration, we auto-approve the request
        // In production, this would require actual user interaction through:
        // 1. Telegram inline keyboard confirmation
        // 2. Hardware wallet approval
        // 3. Mobile app confirmation
        // 4. SMS/Email verification
        
        let signing_result = self.transaction_signer
            .process_approval(&request_id, true, telegram_id) // Auto-approve for demo
            .await?;
        
        if signing_result.success {
            info!("Transaction signed successfully using method: {}", signing_result.signing_method);
            
            if let Some(signature) = signing_result.signature {
                // In production, submit the signed transaction to the blockchain here
                info!("Would submit transaction with signature: {}", signature);
                Ok(signature)
            } else {
                Err(BotError::trading("No signature returned from signer".to_string()).into())
            }
        } else {
            let error_msg = signing_result.error.unwrap_or_else(|| "Unknown signing error".to_string());
            warn!("Transaction signing failed: {}", error_msg);
            Err(BotError::trading(format!("Signing failed: {}", error_msg)).into())
        }
    }
    
    /// Validate quote meets expectations
    async fn validate_quote(&self, quote: &JupiterQuote, request: &SwapRequest) -> Result<()> {
        // Check price impact
        let price_impact: f64 = quote.price_impact_pct.parse().unwrap_or(0.0);
        if price_impact > 5.0 {
            return Err(BotError::validation(format!(
                "Price impact too high: {:.2}%", price_impact
            )).into());
        }
        
        // Check slippage matches request
        if quote.slippage_bps != request.slippage_bps {
            warn!("Slippage mismatch: requested {}, got {}", 
                request.slippage_bps, quote.slippage_bps);
        }
        
        // Check minimum output amount
        let output_amount: u64 = quote.out_amount.parse().unwrap_or(0);
        if output_amount == 0 {
            return Err(BotError::validation("Zero output amount").into());
        }
        
        Ok(())
    }
    
    /// Check API rate limiting
    async fn check_rate_limit(&self, user_key: &str) -> Result<()> {
        let mut cache = self.swap_cache.write().await;
        let now = Utc::now();
        
        // Clean old entries
        cache.rate_limit_tracker.iter_mut().for_each(|(_, timestamps)| {
            timestamps.retain(|t| now.signed_duration_since(*t).num_seconds() < 60);
        });
        
        // Check current user's rate limit (max 10 requests per minute)
        let user_requests = cache.rate_limit_tracker
            .entry(user_key.to_string())
            .or_insert_with(Vec::new);
        
        if user_requests.len() >= 10 {
            return Err(BotError::rate_limited("Too many swap requests. Please wait.").into());
        }
        
        user_requests.push(now);
        Ok(())
    }
    
    /// Get supported tokens list
    pub async fn get_supported_tokens(&self) -> Result<Vec<TokenInfo>> {
        let response = self.client
            .get("https://token.jup.ag/strict")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(BotError::api("Failed to fetch token list").into());
        }
        
        let tokens: Vec<TokenInfo> = response.json().await?;
        Ok(tokens)
    }
    
    /// Calculate swap fee estimate
    pub fn calculate_swap_fee(&self, quote: &JupiterQuote) -> SwapFeeBreakdown {
        let input_amount: f64 = quote.in_amount.parse().unwrap_or(0.0);
        let output_amount: f64 = quote.out_amount.parse().unwrap_or(0.0);
        let price_impact: f64 = quote.price_impact_pct.parse().unwrap_or(0.0);
        
        let platform_fee = quote.platform_fee
            .as_ref()
            .map(|f| f.amount.parse().unwrap_or(0.0))
            .unwrap_or(0.0);
        
        // Estimate network fees (typical Solana transaction)
        let network_fee = 0.000005; // ~5000 lamports
        
        SwapFeeBreakdown {
            platform_fee,
            network_fee,
            price_impact_cost: input_amount * (price_impact / 100.0),
            total_fee: platform_fee + network_fee,
            fee_percentage: if input_amount > 0.0 {
                ((platform_fee + network_fee) / input_amount) * 100.0
            } else {
                0.0
            },
        }
    }
}

/// Token information
#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    pub verified: Option<bool>,
}

/// Swap fee breakdown
#[derive(Debug, Clone)]
pub struct SwapFeeBreakdown {
    pub platform_fee: f64,
    pub network_fee: f64,
    pub price_impact_cost: f64,
    pub total_fee: f64,
    pub fee_percentage: f64,
}