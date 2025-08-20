use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::errors::{BotError, Result};
use crate::api::jupiter_auth::{JupiterAuthManager, ApiTierLevel};

/// Jupiter Send API client for magic link transfers
#[derive(Clone)]
pub struct JupiterSendClient {
    client: Client,
    auth_manager: Arc<JupiterAuthManager>,
    base_url: String,
    send_cache: Arc<RwLock<SendCache>>,
}

/// Send request for creating magic links
#[derive(Debug, Serialize)]
pub struct SendRequest {
    #[serde(rename = "senderPublicKey")]
    pub sender_public_key: String,
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    pub amount: u64, // In token's smallest unit
    
    // Optional recipient (for direct transfers)
    #[serde(rename = "recipientPublicKey", skip_serializing_if = "Option::is_none")]
    pub recipient_public_key: Option<String>,
    #[serde(rename = "recipientEmail", skip_serializing_if = "Option::is_none")]
    pub recipient_email: Option<String>,
    #[serde(rename = "recipientPhone", skip_serializing_if = "Option::is_none")]
    pub recipient_phone: Option<String>,
    
    // Magic link configuration
    #[serde(rename = "expiryHours", skip_serializing_if = "Option::is_none")]
    pub expiry_hours: Option<u32>, // Default 24 hours
    pub message: Option<String>,
    #[serde(rename = "requireAuth", skip_serializing_if = "Option::is_none")]
    pub require_auth: Option<bool>, // Require recipient verification
    
    // Advanced options
    #[serde(rename = "allowPartialClaim", skip_serializing_if = "Option::is_none")]
    pub allow_partial_claim: Option<bool>,
    #[serde(rename = "maxClaims", skip_serializing_if = "Option::is_none")]
    pub max_claims: Option<u32>,
    #[serde(rename = "notifyOnClaim", skip_serializing_if = "Option::is_none")]
    pub notify_on_claim: Option<bool>,
    
    // Security features
    #[serde(rename = "requirePhoneVerification", skip_serializing_if = "Option::is_none")]
    pub require_phone_verification: Option<bool>,
    #[serde(rename = "allowedCountries", skip_serializing_if = "Option::is_none")]
    pub allowed_countries: Option<Vec<String>>,
    #[serde(rename = "blockedCountries", skip_serializing_if = "Option::is_none")]
    pub blocked_countries: Option<Vec<String>>,
    
    // Transaction options
    #[serde(rename = "priorityFeeLamports", skip_serializing_if = "Option::is_none")]
    pub priority_fee_lamports: Option<u64>,
    #[serde(rename = "computeUnitPrice", skip_serializing_if = "Option::is_none")]
    pub compute_unit_price: Option<u64>,
}

/// Response from creating a send link
#[derive(Debug, Deserialize)]
pub struct SendResponse {
    #[serde(rename = "sendId")]
    pub send_id: String,
    #[serde(rename = "magicLink")]
    pub magic_link: String,
    #[serde(rename = "shortLink")]
    pub short_link: String, // Shortened version for SMS/messaging
    #[serde(rename = "qrCodeUrl")]
    pub qr_code_url: String,
    #[serde(rename = "qrCodeData")]
    pub qr_code_data: String, // Base64 QR code image
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    pub status: SendStatus,
    #[serde(rename = "shareableMessage")]
    pub shareable_message: Option<String>,
}

/// Send status tracking
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SendStatus {
    Created,     // Link created, pending claim
    Partially,   // Partially claimed (if partial claims allowed)
    Claimed,     // Fully claimed
    Expired,     // Expired unclaimed
    Cancelled,   // Cancelled by sender
    Failed,      // Technical failure
}

/// Information about a send/magic link
#[derive(Debug, Deserialize)]
pub struct SendInfo {
    #[serde(rename = "sendId")]
    pub send_id: String,
    #[serde(rename = "senderPublicKey")]
    pub sender_public_key: String,
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    #[serde(rename = "tokenSymbol")]
    pub token_symbol: String,
    #[serde(rename = "originalAmount")]
    pub original_amount: u64,
    #[serde(rename = "remainingAmount")]
    pub remaining_amount: u64,
    #[serde(rename = "claimedAmount")]
    pub claimed_amount: u64,
    #[serde(rename = "magicLink")]
    pub magic_link: String,
    #[serde(rename = "shortLink")]
    pub short_link: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    pub status: SendStatus,
    pub message: Option<String>,
    #[serde(rename = "claimHistory")]
    pub claim_history: Vec<ClaimRecord>,
    #[serde(rename = "analytics")]
    pub analytics: SendAnalytics,
}

/// Claim record for tracking who claimed what
#[derive(Debug, Deserialize)]
pub struct ClaimRecord {
    #[serde(rename = "claimId")]
    pub claim_id: String,
    #[serde(rename = "recipientPublicKey")]
    pub recipient_public_key: String,
    #[serde(rename = "claimedAmount")]
    pub claimed_amount: u64,
    #[serde(rename = "claimedAt")]
    pub claimed_at: DateTime<Utc>,
    #[serde(rename = "transactionSignature")]
    pub transaction_signature: String,
    #[serde(rename = "ipAddress")]
    pub ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub user_agent: Option<String>,
    #[serde(rename = "country")]
    pub country: Option<String>,
}

/// Analytics for send links
#[derive(Debug, Deserialize)]
pub struct SendAnalytics {
    #[serde(rename = "viewCount")]
    pub view_count: u32,
    #[serde(rename = "uniqueViews")]
    pub unique_views: u32,
    #[serde(rename = "claimAttempts")]
    pub claim_attempts: u32,
    #[serde(rename = "successfulClaims")]
    pub successful_claims: u32,
    #[serde(rename = "firstViewedAt")]
    pub first_viewed_at: Option<DateTime<Utc>>,
    #[serde(rename = "lastViewedAt")]
    pub last_viewed_at: Option<DateTime<Utc>>,
    #[serde(rename = "avgClaimTime")]
    pub avg_claim_time_minutes: Option<f64>,
    #[serde(rename = "topCountries")]
    pub top_countries: Option<HashMap<String, u32>>,
}

/// Bulk send request for multiple recipients
#[derive(Debug, Serialize)]
pub struct BulkSendRequest {
    #[serde(rename = "senderPublicKey")]
    pub sender_public_key: String,
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    pub recipients: Vec<BulkRecipient>,
    pub message: Option<String>,
    #[serde(rename = "expiryHours")]
    pub expiry_hours: Option<u32>,
    #[serde(rename = "priorityFeeLamports")]
    pub priority_fee_lamports: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BulkRecipient {
    #[serde(rename = "recipientId")]
    pub recipient_id: String, // User-defined ID
    pub amount: u64,
    #[serde(rename = "recipientEmail", skip_serializing_if = "Option::is_none")]
    pub recipient_email: Option<String>,
    #[serde(rename = "recipientPhone", skip_serializing_if = "Option::is_none")]
    pub recipient_phone: Option<String>,
    #[serde(rename = "personalMessage", skip_serializing_if = "Option::is_none")]
    pub personal_message: Option<String>,
}

/// Bulk send response
#[derive(Debug, Deserialize)]
pub struct BulkSendResponse {
    #[serde(rename = "batchId")]
    pub batch_id: String,
    #[serde(rename = "totalAmount")]
    pub total_amount: u64,
    #[serde(rename = "recipientCount")]
    pub recipient_count: u32,
    #[serde(rename = "successfulSends")]
    pub successful_sends: u32,
    #[serde(rename = "failedSends")]
    pub failed_sends: u32,
    pub sends: Vec<BulkSendResult>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BulkSendResult {
    #[serde(rename = "recipientId")]
    pub recipient_id: String,
    #[serde(rename = "sendId", skip_serializing_if = "Option::is_none")]
    pub send_id: Option<String>,
    #[serde(rename = "magicLink", skip_serializing_if = "Option::is_none")]
    pub magic_link: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// Send template for reusable send configurations
#[derive(Debug, Serialize, Deserialize)]
pub struct SendTemplate {
    #[serde(rename = "templateId")]
    pub template_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    #[serde(rename = "defaultAmount")]
    pub default_amount: Option<u64>,
    #[serde(rename = "defaultMessage")]
    pub default_message: Option<String>,
    #[serde(rename = "defaultExpiryHours")]
    pub default_expiry_hours: Option<u32>,
    #[serde(rename = "requireAuth")]
    pub require_auth: bool,
    #[serde(rename = "allowPartialClaim")]
    pub allow_partial_claim: bool,
    #[serde(rename = "maxClaims")]
    pub max_claims: Option<u32>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

/// Cache for send operations
#[derive(Debug)]
struct SendCache {
    sends: HashMap<String, CachedSend>,
    templates: HashMap<String, SendTemplate>,
    last_cleanup: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedSend {
    send_info: SendInfo,
    cached_at: DateTime<Utc>,
}

impl JupiterSendClient {
    /// Create new Jupiter Send client
    pub fn new(auth_manager: Arc<JupiterAuthManager>) -> Self {
        info!("📤 Initializing Jupiter Send API client");
        
        Self {
            client: Client::new(),
            auth_manager,
            base_url: "https://api.jup.ag/send/v1".to_string(),
            send_cache: Arc::new(RwLock::new(SendCache {
                sends: HashMap::new(),
                templates: HashMap::new(),
                last_cleanup: Utc::now(),
            })),
        }
    }
    
    /// Create a magic link send
    pub async fn create_send(&self, request: SendRequest) -> Result<SendResponse> {
        let api_key_config = self.auth_manager.select_best_key("send").await?
            .ok_or_else(|| BotError::jupiter_api("Send API requires authentication".to_string()))?;
        
        // Send requires Ultra tier or above
        if matches!(api_key_config.tier, ApiTierLevel::Lite) {
            return Err(BotError::jupiter_api(
                "Send API requires Ultra tier or above".to_string()
            ).into());
        }
        
        // Validate request
        self.validate_send_request(&request)?;
        
        let url = format!("{}/create", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Send creation request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Send creation failed with status {}: {}", status, error_text
            )).into());
        }
        
        let send_response: SendResponse = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse send response: {}", e)))?;
            
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "send").await;
        
        info!("📤 Created send link {} for {} tokens", 
            send_response.send_id, request.amount);
        
        Ok(send_response)
    }
    
    /// Create bulk send for multiple recipients
    pub async fn create_bulk_send(&self, request: BulkSendRequest) -> Result<BulkSendResponse> {
        let api_key_config = self.auth_manager.select_best_key("bulk_send").await?
            .ok_or_else(|| BotError::jupiter_api("Bulk send API requires authentication".to_string()))?;
        
        // Bulk send requires Pro tier or above
        if matches!(api_key_config.tier, ApiTierLevel::Lite | ApiTierLevel::Ultra { .. }) {
            return Err(BotError::jupiter_api(
                "Bulk send API requires Pro tier or above".to_string()
            ).into());
        }
        
        // Validate bulk request
        if request.recipients.is_empty() {
            return Err(BotError::validation("Recipients list cannot be empty".to_string()).into());
        }
        
        if request.recipients.len() > 1000 {
            return Err(BotError::validation("Maximum 1000 recipients per bulk send".to_string()).into());
        }
        
        let url = format!("{}/bulk", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Bulk send request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Bulk send failed with status {}: {}", status, error_text
            )).into());
        }
        
        let bulk_response: BulkSendResponse = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse bulk send response: {}", e)))?;
            
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "bulk_send").await;
        
        info!("📤 Created bulk send batch {} with {} recipients ({} successful)", 
            bulk_response.batch_id, 
            bulk_response.recipient_count,
            bulk_response.successful_sends);
        
        Ok(bulk_response)
    }
    
    /// Get information about a send
    pub async fn get_send_info(&self, send_id: &str) -> Result<SendInfo> {
        // Check cache first
        if let Some(cached_send) = self.check_send_cache(send_id).await {
            return Ok(cached_send);
        }
        
        let api_key_config = self.auth_manager.select_best_key("send_info").await?
            .ok_or_else(|| BotError::jupiter_api("Send API requires authentication".to_string()))?;
        
        let url = format!("{}/info/{}", self.base_url, send_id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Send info request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Send info failed with status {}: {}", status, error_text
            )).into());
        }
        
        let send_info: SendInfo = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse send info response: {}", e)))?;
            
        // Cache the result
        self.cache_send_info(&send_info).await;
        
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "send_info").await;
        
        debug!("📤 Retrieved info for send {}", send_id);
        
        Ok(send_info)
    }
    
    /// Cancel an active send
    pub async fn cancel_send(&self, send_id: &str) -> Result<bool> {
        let api_key_config = self.auth_manager.select_best_key("send_cancel").await?
            .ok_or_else(|| BotError::jupiter_api("Send API requires authentication".to_string()))?;
        
        let url = format!("{}/cancel/{}", self.base_url, send_id);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Send cancel request failed: {}", e)))?;
            
        let success = response.status().is_success();
        
        if success {
            info!("📤 Cancelled send {}", send_id);
        } else {
            let error_text = response.text().await.unwrap_or_default();
            warn!("📤 Failed to cancel send {}: {}", send_id, error_text);
        }
        
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "send_cancel").await;
        
        Ok(success)
    }
    
    /// Get user's send history
    pub async fn get_user_sends(&self, user_public_key: &str, limit: Option<u32>) -> Result<Vec<SendInfo>> {
        let api_key_config = self.auth_manager.select_best_key("send_history").await?
            .ok_or_else(|| BotError::jupiter_api("Send API requires authentication".to_string()))?;
        
        let url = format!("{}/history/{}", self.base_url, user_public_key);
        
        let mut request = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key));
            
        if let Some(limit) = limit {
            request = request.query(&[("limit", limit.to_string())]);
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Send history request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Send history failed with status {}: {}", status, error_text
            )).into());
        }
        
        let sends: Vec<SendInfo> = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse send history response: {}", e)))?;
            
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "send_history").await;
        
        debug!("📤 Retrieved {} sends for user {}", sends.len(), user_public_key);
        
        Ok(sends)
    }
    
    /// Validate send request
    fn validate_send_request(&self, request: &SendRequest) -> Result<()> {
        // Validate amount
        if request.amount == 0 {
            return Err(BotError::validation("Amount must be greater than 0".to_string()).into());
        }
        
        // Validate expiry
        if let Some(hours) = request.expiry_hours {
            if hours > 168 { // Max 7 days
                return Err(BotError::validation("Maximum expiry is 168 hours (7 days)".to_string()).into());
            }
        }
        
        // Validate max claims
        if let Some(max_claims) = request.max_claims {
            if max_claims > 1000 {
                return Err(BotError::validation("Maximum 1000 claims per send".to_string()).into());
            }
        }
        
        // Validate at least one recipient method
        if request.recipient_public_key.is_none() && 
           request.recipient_email.is_none() && 
           request.recipient_phone.is_none() {
            return Err(BotError::validation("At least one recipient method must be specified or leave empty for magic link".to_string()).into());
        }
        
        Ok(())
    }
    
    /// Check send cache
    async fn check_send_cache(&self, send_id: &str) -> Option<SendInfo> {
        let mut cache = self.send_cache.write().await;
        
        if let Some(cached) = cache.sends.get_mut(send_id) {
            let age = Utc::now().signed_duration_since(cached.cached_at);
            
            if age < Duration::minutes(1) { // 1-minute cache for send info
                return Some(cached.send_info.clone());
            }
        }
        
        None
    }
    
    /// Cache send info
    async fn cache_send_info(&self, send_info: &SendInfo) {
        let mut cache = self.send_cache.write().await;
        
        cache.sends.insert(send_info.send_id.clone(), CachedSend {
            send_info: send_info.clone(),
            cached_at: Utc::now(),
        });
    }
}

/// Helper functions for creating common send requests
impl JupiterSendClient {
    /// Create a simple magic link send
    pub fn create_simple_send_request(
        sender_public_key: String,
        token_mint: String,
        amount: u64,
        message: Option<String>,
    ) -> SendRequest {
        SendRequest {
            sender_public_key,
            token_mint,
            amount,
            recipient_public_key: None,
            recipient_email: None,
            recipient_phone: None,
            expiry_hours: Some(24),
            message,
            require_auth: Some(false),
            allow_partial_claim: Some(false),
            max_claims: Some(1),
            notify_on_claim: Some(true),
            require_phone_verification: Some(false),
            allowed_countries: None,
            blocked_countries: None,
            priority_fee_lamports: Some(5000),
            compute_unit_price: Some(1000),
        }
    }
    
    /// Create an email send request
    pub fn create_email_send_request(
        sender_public_key: String,
        recipient_email: String,
        token_mint: String,
        amount: u64,
        message: Option<String>,
    ) -> SendRequest {
        SendRequest {
            sender_public_key,
            token_mint,
            amount,
            recipient_public_key: None,
            recipient_email: Some(recipient_email),
            recipient_phone: None,
            expiry_hours: Some(48),
            message,
            require_auth: Some(true),
            allow_partial_claim: Some(false),
            max_claims: Some(1),
            notify_on_claim: Some(true),
            require_phone_verification: Some(false),
            allowed_countries: None,
            blocked_countries: None,
            priority_fee_lamports: Some(5000),
            compute_unit_price: Some(1000),
        }
    }
}