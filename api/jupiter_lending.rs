use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::{DateTime, Utc, Duration};

use crate::errors::{BotError, Result};
use crate::api::jupiter_auth::{JupiterAuthManager, ApiTierLevel};

/// Jupiter Lending API client for 95% LTV lending
#[derive(Clone)]
pub struct JupiterLendingClient {
    client: Client,
    auth_manager: Arc<JupiterAuthManager>,
    base_url: String,
    position_cache: Arc<RwLock<PositionCache>>,
}

/// Lending action types
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LendingAction {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Liquidate,
}

/// Lending request for Jupiter Lend API
#[derive(Debug, Serialize)]
pub struct LendingRequest {
    #[serde(rename = "vaultId")]
    pub vault_id: String,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    pub action: LendingAction,
    pub amount: u64, // In token's smallest unit
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    
    // Optional parameters
    #[serde(rename = "maxLtv", skip_serializing_if = "Option::is_none")]
    pub max_ltv: Option<f64>, // Max 95%
    #[serde(rename = "slippageBps", skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u16>,
    #[serde(rename = "priorityFeeLamports", skip_serializing_if = "Option::is_none")]
    pub priority_fee_lamports: Option<u64>,
}

/// Lending response with transaction data
#[derive(Debug, Deserialize)]
pub struct LendingResponse {
    #[serde(rename = "transaction")]
    pub transaction: String, // Base64 encoded
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "positionId")]
    pub position_id: Option<String>,
    #[serde(rename = "estimatedGas")]
    pub estimated_gas: Option<u64>,
    #[serde(rename = "lendingDetails")]
    pub lending_details: LendingDetails,
}

/// Detailed lending information
#[derive(Debug, Deserialize)]
pub struct LendingDetails {
    #[serde(rename = "currentLtv")]
    pub current_ltv: f64,
    #[serde(rename = "maxLtv")]
    pub max_ltv: f64,
    #[serde(rename = "healthFactor")]
    pub health_factor: f64, // > 1.0 is safe
    #[serde(rename = "liquidationThreshold")]
    pub liquidation_threshold: f64,
    #[serde(rename = "interestRate")]
    pub interest_rate: f64, // APR
    #[serde(rename = "collateralValue")]
    pub collateral_value: u64,
    #[serde(rename = "borrowedValue")]
    pub borrowed_value: u64,
    #[serde(rename = "availableToBorrow")]
    pub available_to_borrow: u64,
}

/// Vault information for lending
#[derive(Debug, Deserialize)]
pub struct LendingVault {
    #[serde(rename = "vaultId")]
    pub vault_id: String,
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    #[serde(rename = "tokenSymbol")]
    pub token_symbol: String,
    #[serde(rename = "totalSupply")]
    pub total_supply: u64,
    #[serde(rename = "totalBorrowed")]
    pub total_borrowed: u64,
    #[serde(rename = "utilizationRate")]
    pub utilization_rate: f64,
    #[serde(rename = "supplyApr")]
    pub supply_apr: f64,
    #[serde(rename = "borrowApr")]
    pub borrow_apr: f64,
    #[serde(rename = "maxLtv")]
    pub max_ltv: f64,
    #[serde(rename = "liquidationPenalty")]
    pub liquidation_penalty: f64, // 1% for Jupiter
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "riskTier")]
    pub risk_tier: RiskTier,
}

/// Risk tiers for lending vaults
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskTier {
    Conservative,
    Moderate,
    Aggressive,
    Speculative,
}

/// User lending position
#[derive(Debug, Clone, Deserialize)]
pub struct LendingPosition {
    #[serde(rename = "positionId")]
    pub position_id: String,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "vaultId")]
    pub vault_id: String,
    #[serde(rename = "tokenMint")]
    pub token_mint: String,
    #[serde(rename = "collateralAmount")]
    pub collateral_amount: u64,
    #[serde(rename = "borrowedAmount")]
    pub borrowed_amount: u64,
    #[serde(rename = "currentLtv")]
    pub current_ltv: f64,
    #[serde(rename = "healthFactor")]
    pub health_factor: f64,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: Option<f64>,
    #[serde(rename = "interestAccrued")]
    pub interest_accrued: u64,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: DateTime<Utc>,
    #[serde(rename = "status")]
    pub status: PositionStatus,
}

/// Position status
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionStatus {
    Active,
    AtRisk, // Health factor < 1.2
    Liquidatable, // Health factor < 1.0
    Liquidated,
    Closed,
}

/// Liquidation information
#[derive(Debug, Deserialize)]
pub struct LiquidationInfo {
    #[serde(rename = "positionId")]
    pub position_id: String,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: f64,
    #[serde(rename = "healthFactor")]
    pub health_factor: f64,
    #[serde(rename = "liquidationReward")]
    pub liquidation_reward: f64, // Percentage reward for liquidator
    #[serde(rename = "timeToLiquidation")]
    pub time_to_liquidation: Option<Duration>,
}

/// Position cache for performance
#[derive(Debug)]
struct PositionCache {
    positions: HashMap<String, CachedPosition>,
    vaults: HashMap<String, CachedVault>,
    last_cleanup: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedPosition {
    position: LendingPosition,
    cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedVault {
    vault: LendingVault,
    cached_at: DateTime<Utc>,
}

impl JupiterLendingClient {
    /// Create new Jupiter lending client
    pub fn new(auth_manager: Arc<JupiterAuthManager>) -> Self {
        info!("🏦 Initializing Jupiter Lending API client");
        
        Self {
            client: Client::new(),
            auth_manager,
            base_url: "https://api.jup.ag/lend/v1".to_string(),
            position_cache: Arc::new(RwLock::new(PositionCache {
                positions: HashMap::new(),
                vaults: HashMap::new(),
                last_cleanup: Utc::now(),
            })),
        }
    }
    
    /// Get all available lending vaults
    pub async fn get_vaults(&self) -> Result<Vec<LendingVault>> {
        let api_key_config = self.auth_manager.select_best_key("lending_vaults").await?
            .ok_or_else(|| BotError::jupiter_api("Lending API requires authentication".to_string()))?;
        
        // Lending requires Ultra tier or above
        if matches!(api_key_config.tier, ApiTierLevel::Lite) {
            return Err(BotError::jupiter_api(
                "Lending API requires Ultra tier or above".to_string()
            ).into());
        }
        
        let url = format!("{}/vaults", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Vaults request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Vaults API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let vaults: Vec<LendingVault> = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse vaults response: {}", e)))?;
            
        // Cache the vaults
        self.cache_vaults(&vaults).await;
        
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "lending_vaults").await;
        
        info!("🏦 Retrieved {} lending vaults", vaults.len());
        
        Ok(vaults)
    }
    
    /// Execute lending action (deposit, borrow, withdraw, repay)
    pub async fn execute_lending_action(&self, request: LendingRequest) -> Result<LendingResponse> {
        let api_key_config = self.auth_manager.select_best_key("lending_action").await?
            .ok_or_else(|| BotError::jupiter_api("Lending API requires authentication".to_string()))?;
        
        // Validate request
        self.validate_lending_request(&request).await?;
        
        let url = match request.action {
            LendingAction::Deposit => format!("{}/deposit", self.base_url),
            LendingAction::Withdraw => format!("{}/withdraw", self.base_url),
            LendingAction::Borrow => format!("{}/borrow", self.base_url),
            LendingAction::Repay => format!("{}/repay", self.base_url),
            LendingAction::Liquidate => format!("{}/liquidate", self.base_url),
        };
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Lending action request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Lending action failed with status {}: {}", status, error_text
            )).into());
        }
        
        let lending_response: LendingResponse = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse lending response: {}", e)))?;
            
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "lending_action").await;
        
        info!("🏦 Executed lending action {:?} for vault {}", 
            request.action, request.vault_id);
        
        Ok(lending_response)
    }
    
    /// Get user's lending positions
    pub async fn get_user_positions(&self, user_public_key: &str) -> Result<Vec<LendingPosition>> {
        // Check cache first
        if let Some(cached_positions) = self.check_position_cache(user_public_key).await {
            return Ok(cached_positions);
        }
        
        let api_key_config = self.auth_manager.select_best_key("lending_positions").await?
            .ok_or_else(|| BotError::jupiter_api("Lending API requires authentication".to_string()))?;
        
        let url = format!("{}/positions/{}", self.base_url, user_public_key);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Positions request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Positions API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let positions: Vec<LendingPosition> = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse positions response: {}", e)))?;
            
        // Cache the positions
        self.cache_positions(user_public_key, &positions).await;
        
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "lending_positions").await;
        
        debug!("🏦 Retrieved {} lending positions for user {}", positions.len(), user_public_key);
        
        Ok(positions)
    }
    
    /// Get positions at risk of liquidation
    pub async fn get_liquidatable_positions(&self) -> Result<Vec<LiquidationInfo>> {
        let api_key_config = self.auth_manager.select_best_key("liquidations").await?
            .ok_or_else(|| BotError::jupiter_api("Liquidation API requires authentication".to_string()))?;
        
        let url = format!("{}/liquidations", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key_config.key))
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Liquidations request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Liquidations API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let liquidations: Vec<LiquidationInfo> = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse liquidations response: {}", e)))?;
            
        // Record usage
        let key_id = format!("key_{}", &api_key_config.key[..8]);
        self.auth_manager.record_usage(&key_id, "liquidations").await;
        
        info!("🚨 Found {} positions at risk of liquidation", liquidations.len());
        
        Ok(liquidations)
    }
    
    /// Calculate maximum borrowable amount
    pub fn calculate_max_borrow(&self, collateral_value: f64, max_ltv: f64) -> f64 {
        collateral_value * max_ltv
    }
    
    /// Calculate health factor
    pub fn calculate_health_factor(&self, collateral_value: f64, borrowed_value: f64, liquidation_threshold: f64) -> f64 {
        if borrowed_value == 0.0 {
            f64::INFINITY
        } else {
            (collateral_value * liquidation_threshold) / borrowed_value
        }
    }
    
    /// Check if position is safe
    pub fn is_position_safe(&self, health_factor: f64) -> bool {
        health_factor > 1.2 // Safety margin above liquidation threshold
    }
    
    /// Get recommended action for position
    pub fn get_position_recommendation(&self, position: &LendingPosition) -> PositionRecommendation {
        if position.health_factor > 2.0 {
            PositionRecommendation::CanBorrowMore
        } else if position.health_factor > 1.5 {
            PositionRecommendation::Healthy
        } else if position.health_factor > 1.2 {
            PositionRecommendation::MonitorClosely
        } else if position.health_factor > 1.0 {
            PositionRecommendation::AddCollateral
        } else {
            PositionRecommendation::Liquidatable
        }
    }
    
    /// Validate lending request
    async fn validate_lending_request(&self, request: &LendingRequest) -> Result<()> {
        // Validate LTV
        if let Some(max_ltv) = request.max_ltv {
            if max_ltv > 0.95 {
                return Err(BotError::validation("Max LTV cannot exceed 95%".to_string()).into());
            }
        }
        
        // Validate amount
        if request.amount == 0 {
            return Err(BotError::validation("Amount must be greater than 0".to_string()).into());
        }
        
        // Additional validation could include:
        // - Check if vault exists and is active
        // - Verify user has sufficient balance for deposits
        // - Check position health for borrows
        
        Ok(())
    }
    
    /// Check position cache
    async fn check_position_cache(&self, user_public_key: &str) -> Option<Vec<LendingPosition>> {
        let cache = self.position_cache.read().await;
        let mut positions = Vec::new();
        
        for (position_id, cached) in &cache.positions {
            if cached.position.user_public_key == user_public_key {
                let age = Utc::now().signed_duration_since(cached.cached_at);
                if age < Duration::minutes(2) { // 2-minute cache for positions
                    positions.push(cached.position.clone());
                }
            }
        }
        
        if positions.is_empty() {
            None
        } else {
            Some(positions)
        }
    }
    
    /// Cache user positions
    async fn cache_positions(&self, user_public_key: &str, positions: &[LendingPosition]) {
        let mut cache = self.position_cache.write().await;
        
        for position in positions {
            cache.positions.insert(position.position_id.clone(), CachedPosition {
                position: position.clone(),
                cached_at: Utc::now(),
            });
        }
    }
    
    /// Cache vaults
    async fn cache_vaults(&self, vaults: &[LendingVault]) {
        let mut cache = self.position_cache.write().await;
        
        for vault in vaults {
            cache.vaults.insert(vault.vault_id.clone(), CachedVault {
                vault: vault.clone(),
                cached_at: Utc::now(),
            });
        }
    }
}

/// Position recommendation based on health factor
#[derive(Debug, Clone)]
pub enum PositionRecommendation {
    CanBorrowMore,
    Healthy,
    MonitorClosely,
    AddCollateral,
    Liquidatable,
}

/// Helper functions for lending calculations
impl JupiterLendingClient {
    /// Create a deposit request
    pub fn create_deposit_request(
        vault_id: String,
        user_public_key: String,
        token_mint: String,
        amount: u64,
    ) -> LendingRequest {
        LendingRequest {
            vault_id,
            user_public_key,
            action: LendingAction::Deposit,
            amount,
            token_mint,
            max_ltv: None,
            slippage_bps: Some(50), // 0.5% slippage
            priority_fee_lamports: Some(5000),
        }
    }
    
    /// Create a borrow request
    pub fn create_borrow_request(
        vault_id: String,
        user_public_key: String,
        token_mint: String,
        amount: u64,
        max_ltv: Option<f64>,
    ) -> LendingRequest {
        LendingRequest {
            vault_id,
            user_public_key,
            action: LendingAction::Borrow,
            amount,
            token_mint,
            max_ltv,
            slippage_bps: Some(100), // 1% slippage for borrows
            priority_fee_lamports: Some(10000),
        }
    }
}