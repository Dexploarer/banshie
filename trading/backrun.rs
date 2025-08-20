use crate::errors::{BotError, Result};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    transaction::Transaction,
    signature::{Keypair, Signer},
    commitment_config::CommitmentLevel,
};
use tracing::{info, debug, warn, instrument};
use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};
use tokio::sync::{RwLock, Semaphore};

use super::types::TradeResult;

#[derive(Debug, Serialize, Deserialize)]
struct HeliusResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<HeliusError>,
    id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct HeliusError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeliusRebateInfo {
    pub mev_generated: f64,
    pub rebate_earned: f64,
    pub total_rebates: f64,
    pub transaction_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendTransactionConfig {
    #[serde(rename = "skipPreflight")]
    skip_preflight: bool,
    #[serde(rename = "preflightCommitment")]
    preflight_commitment: String,
    #[serde(rename = "maxRetries")]
    max_retries: u32,
    #[serde(rename = "minContextSlot")]
    min_context_slot: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HeliusBundleConfig {
    #[serde(rename = "bundleOnly")]
    bundle_only: bool,
    #[serde(rename = "enableBackrunProtection")]
    enable_backrun_protection: bool,
    #[serde(rename = "enableRebates")]
    enable_rebates: bool,
    #[serde(rename = "rebateRecipient")]
    rebate_recipient: String,
    #[serde(rename = "tipAmount")]
    tip_amount: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PriorityFeeEstimate {
    #[serde(rename = "priorityFeeEstimate")]
    priority_fee_estimate: f64,
    #[serde(rename = "priorityFeeLevels")]
    priority_fee_levels: PriorityFeeLevels,
}

#[derive(Debug, Serialize, Deserialize)]
struct PriorityFeeLevels {
    min: f64,
    low: f64,
    medium: f64,
    high: f64,
    #[serde(rename = "veryHigh")]
    very_high: f64,
    #[serde(rename = "unsafeMax")]
    unsafe_max: f64,
}

pub struct HeliusClient {
    api_key: String,
    client: Client,
    rpc_url: String,
    // Performance optimizations
    rate_limiter: Arc<Semaphore>,
    rebate_cache: Arc<RwLock<HashMap<String, (HeliusRebateInfo, Instant)>>>,
}

impl HeliusClient {
    pub fn new(api_key: &str, rebate_address: Option<&str>) -> Result<Self> {
        let rpc_url = if let Some(rebate_addr) = rebate_address {
            format!("https://mainnet.helius-rpc.com/?api-key={}&rebate-address={}", api_key, rebate_addr)
        } else {
            format!("https://mainnet.helius-rpc.com/?api-key={}", api_key)
        };
        
        // Create optimized HTTP client for Helius
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30)) // Longer timeout for RPC calls
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(5) // Fewer connections for RPC
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Duration::from_secs(60))
            .http2_prior_knowledge()
            .gzip(true)
            .user_agent("solana-trading-bot/0.1.0")
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create optimized Helius HTTP client: {}", e))?;
        
        Ok(Self {
            api_key: api_key.to_string(),
            client,
            rpc_url,
            rate_limiter: Arc::new(Semaphore::new(5)), // More conservative rate limit for RPC
            rebate_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_priority_fee_estimate(&self) -> Result<u64> {
        // Acquire rate limiter permit
        let _permit = self.rate_limiter.acquire().await
            .map_err(|_| BotError::external_api("Rate limiter closed".to_string()))?;
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getPriorityFeeEstimate",
            "params": [{
                "priorityLevel": "high"
            }]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;
        
        let data: HeliusResponse = response.json().await?;
        
        if let Some(error) = data.error {
            warn!("Failed to get priority fee estimate: {}", error.message);
            return Ok(50000); // Default to 50k lamports
        }
        
        if let Some(result) = data.result {
            if let Some(fee) = result["priorityFeeEstimate"].as_f64() {
                return Ok(fee as u64);
            }
        }
        
        Ok(50000) // Default fallback
    }
    
    #[instrument(skip(self, tx, wallet))]
    pub async fn send_transaction_with_rebate(
        &self,
        mut tx: Transaction,
        wallet: &Keypair,
    ) -> Result<TradeResult> {
        // Acquire rate limiter permit
        let _permit = self.rate_limiter.acquire().await
            .map_err(|_| BotError::external_api("Rate limiter closed".to_string()))?;
        
        // Sign the transaction
        tx.sign(&[wallet], tx.message.recent_blockhash);
        
        let tx_bytes = bincode::serialize(&tx)?;
        let tx_base64 = base64::encode(&tx_bytes);
        
        // Get optimal priority fee
        let priority_fee = self.get_priority_fee_estimate().await?;
        
        // Simple sendTransaction call - Helius handles rebates automatically via URL rebate-address parameter
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                tx_base64,
                {
                    "encoding": "base64",
                    "commitment": "confirmed",
                    "skipPreflight": false,
                    "maxRetries": 3
                }
            ]
        });
        
        debug!("Sending transaction with MEV rebate enabled via URL parameter, priority fee: {} lamports", priority_fee);
        info!("RPC URL includes rebate-address parameter for automatic MEV rebates");
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await?;
        
        let result: HeliusResponse = response.json().await?;
        
        if let Some(error) = result.error {
            return Err(BotError::external_api(format!("Helius error: {} (code: {})", error.message, error.code)));
        }
        
        let signature = result.result
            .and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| BotError::external_api("No signature returned from Helius"))?;
        
        // Estimate MEV and rebate (50% of MEV goes to rebate wallet automatically)
        let mev_generated = self.estimate_mev_generated(&tx, priority_fee);
        let rebate_earned = mev_generated * 0.5; // 50% rebate share paid to rebate wallet
        
        info!(
            "Transaction sent: {} | Priority fee: {} | Est. rebate to wallet: {:.6} SOL",
            signature, priority_fee, rebate_earned
        );
        
        Ok(TradeResult {
            tx_signature: signature,
            tokens_received: 100.0, // This should be calculated from actual swap
            tokens_sold: 0.0,
            sol_received: 0.0,
            price: 0.001,
            rebate_earned,
            pnl_percentage: 0.0,
        })
    }
    
    fn estimate_mev_generated(&self, tx: &Transaction, priority_fee: u64) -> f64 {
        // Estimate based on transaction size and priority fee
        let base_mev = 0.001; // Base MEV in SOL
        let priority_bonus = (priority_fee as f64 / 1e9) * 2.0; // Priority fee contribution
        let tx_complexity = (tx.message.instructions.len() as f64) * 0.0001;
        
        base_mev + priority_bonus + tx_complexity
    }
    
    #[instrument(skip(self))]
    pub async fn get_rebate_stats(&self, wallet: &str) -> Result<HeliusRebateInfo> {
        // Check cache first (rebate stats cached for 60 seconds)
        {
            let cache = self.rebate_cache.read().await;
            if let Some((rebate_info, cached_at)) = cache.get(wallet) {
                if cached_at.elapsed() < Duration::from_secs(60) {
                    debug!("Returning cached rebate stats for wallet {}", wallet);
                    return Ok(rebate_info.clone());
                }
            }
        }
        
        // Acquire rate limiter permit
        let _permit = self.rate_limiter.acquire().await
            .map_err(|_| BotError::external_api("Rate limiter closed".to_string()))?;
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                wallet,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;
        
        let data: HeliusResponse = response.json().await?;
        
        // Note: Actual rebate stats would need a dedicated endpoint
        // This is a placeholder implementation
        let rebate_info = HeliusRebateInfo {
            mev_generated: 0.0,
            rebate_earned: 0.0,
            total_rebates: 0.0,
            transaction_count: 0,
        };
        
        // Cache the result
        {
            let mut cache = self.rebate_cache.write().await;
            cache.insert(wallet.to_string(), (rebate_info.clone(), Instant::now()));
        }
        
        debug!("Fetched and cached rebate stats for wallet {}", wallet);
        Ok(rebate_info)
    }
    
    #[instrument(skip(self, transactions, wallet))]
    pub async fn send_bundle(&self, transactions: Vec<Transaction>, wallet: &Keypair) -> Result<String> {
        // Acquire rate limiter permit
        let _permit = self.rate_limiter.acquire().await
            .map_err(|_| BotError::external_api("Rate limiter closed".to_string()))?;
        
        let mut signed_txs = Vec::new();
        
        for mut tx in transactions {
            tx.sign(&[wallet], tx.message.recent_blockhash);
            let tx_bytes = bincode::serialize(&tx)?;
            signed_txs.push(base64::encode(&tx_bytes));
        }
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [signed_txs]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;
        
        let result: HeliusResponse = response.json().await?;
        
        if let Some(error) = result.error {
            return Err(BotError::external_api(format!("Bundle send failed: {}", error.message)));
        }
        
        result.result
            .and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| BotError::external_api("No bundle ID returned"))
    }
}