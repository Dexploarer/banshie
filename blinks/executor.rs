use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use super::types::*;
use crate::errors::BotError;
use crate::trading::{TradingEngineHandle, TradeResult};
use crate::wallet::WalletManager;

/// Executes Solana Blinks
pub struct BlinkExecutor {
    trading_engine: TradingEngineHandle,
    wallet_manager: Arc<WalletManager>,
    execution_cache: Arc<RwLock<HashMap<String, BlinkExecutionResult>>>,
}

impl BlinkExecutor {
    pub fn new(
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
    ) -> Self {
        Self {
            trading_engine,
            wallet_manager,
            execution_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Execute a Solana Blink
    pub async fn execute_blink(
        &self,
        blink: &SolanaBlink,
        user_wallet: String,
    ) -> Result<BlinkExecutionResult> {
        info!("Executing blink: {} for wallet: {}", blink.blink_id, user_wallet);
        
        let start_time = std::time::Instant::now();
        
        // Validate blink first
        let validation = blink.validate();
        if !validation.is_valid {
            return Err(BotError::validation(format!(
                "Blink validation failed: {:?}",
                validation.errors
            )).into());
        }
        
        // Check if blink has expired
        if let Some(expires_at) = blink.expires_at {
            if expires_at < Utc::now() {
                return Err(BotError::validation("Blink has expired").into());
            }
        }
        
        // Check max uses if specified
        if let Some(max_uses) = blink.security.max_uses {
            let cache = self.execution_cache.read().await;
            let executions = cache.values()
                .filter(|e| e.success)
                .count() as u32;
            
            if executions >= max_uses {
                return Err(BotError::validation("Blink has reached maximum uses").into());
            }
        }
        
        // Check allowed wallets if specified
        if let Some(allowed) = &blink.security.allowed_wallets {
            if !allowed.contains(&user_wallet) {
                return Err(BotError::validation("Wallet not authorized for this blink").into());
            }
        }
        
        // Execute based on action type
        let result = match &blink.action.action_type {
            ActionType::Swap { from_token, to_token, amount } => {
                self.execute_swap(from_token, to_token, *amount, &user_wallet).await
            }
            ActionType::Transfer { token, recipient, amount } => {
                self.execute_transfer(token, recipient, *amount, &user_wallet).await
            }
            ActionType::Mint { collection, price } => {
                self.execute_mint(collection, *price, &user_wallet).await
            }
            ActionType::Stake { validator, amount } => {
                self.execute_stake(validator, *amount, &user_wallet).await
            }
            ActionType::Vote { proposal_id, choice } => {
                self.execute_vote(proposal_id, choice, &user_wallet).await
            }
            ActionType::Custom { program_id, instruction_data } => {
                self.execute_custom(program_id, instruction_data, &user_wallet).await
            }
        };
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        let execution_result = match result {
            Ok(signature) => BlinkExecutionResult {
                success: true,
                transaction_signature: Some(signature),
                error: None,
                execution_time_ms,
                gas_used: Some(5000), // Estimated
                outputs: HashMap::new(),
            },
            Err(e) => BlinkExecutionResult {
                success: false,
                transaction_signature: None,
                error: Some(e.to_string()),
                execution_time_ms,
                gas_used: None,
                outputs: HashMap::new(),
            },
        };
        
        // Cache the result
        {
            let mut cache = self.execution_cache.write().await;
            cache.insert(blink.blink_id.clone(), execution_result.clone());
            
            // Clean old entries if cache is too large
            if cache.len() > 1000 {
                cache.clear();
            }
        }
        
        info!(
            "Blink execution completed: {} in {}ms",
            if execution_result.success { "SUCCESS" } else { "FAILED" },
            execution_time_ms
        );
        
        Ok(execution_result)
    }
    
    /// Execute a swap action
    async fn execute_swap(
        &self,
        from_token: &str,
        to_token: &str,
        amount: f64,
        user_wallet: &str,
    ) -> Result<String> {
        debug!("Executing swap: {} -> {} for {} SOL", from_token, to_token, amount);
        
        // In production, this would use Jupiter API for actual swap
        // For now, return a simulated transaction
        
        // Simulate the swap execution
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Generate mock transaction signature
        let signature = format!("swap_{}_{}", 
            &uuid::Uuid::new_v4().to_string()[..8],
            chrono::Utc::now().timestamp()
        );
        
        info!("Swap executed successfully: {}", signature);
        Ok(signature)
    }
    
    /// Execute a transfer action
    async fn execute_transfer(
        &self,
        token: &str,
        recipient: &str,
        amount: f64,
        user_wallet: &str,
    ) -> Result<String> {
        debug!("Executing transfer: {} {} to {}", amount, token, recipient);
        
        // Validate recipient address
        if recipient.len() != 44 && recipient.len() != 43 {
            return Err(BotError::validation("Invalid recipient address").into());
        }
        
        // In production, this would create and send actual SPL token transfer
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        
        let signature = format!("transfer_{}_{}", 
            &uuid::Uuid::new_v4().to_string()[..8],
            chrono::Utc::now().timestamp()
        );
        
        info!("Transfer executed successfully: {}", signature);
        Ok(signature)
    }
    
    /// Execute an NFT mint action
    async fn execute_mint(
        &self,
        collection: &str,
        price: f64,
        user_wallet: &str,
    ) -> Result<String> {
        debug!("Executing NFT mint from collection: {} for {} SOL", collection, price);
        
        // In production, this would use Metaplex SDK for minting
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        
        let signature = format!("mint_{}_{}", 
            &uuid::Uuid::new_v4().to_string()[..8],
            chrono::Utc::now().timestamp()
        );
        
        info!("NFT minted successfully: {}", signature);
        Ok(signature)
    }
    
    /// Execute a staking action
    async fn execute_stake(
        &self,
        validator: &str,
        amount: f64,
        user_wallet: &str,
    ) -> Result<String> {
        debug!("Executing stake: {} SOL to validator {}", amount, validator);
        
        // In production, this would create stake account and delegate
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
        
        let signature = format!("stake_{}_{}", 
            &uuid::Uuid::new_v4().to_string()[..8],
            chrono::Utc::now().timestamp()
        );
        
        info!("Stake executed successfully: {}", signature);
        Ok(signature)
    }
    
    /// Execute a governance vote
    async fn execute_vote(
        &self,
        proposal_id: &str,
        choice: &str,
        user_wallet: &str,
    ) -> Result<String> {
        debug!("Executing vote: {} on proposal {}", choice, proposal_id);
        
        // In production, this would interact with governance program
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
        
        let signature = format!("vote_{}_{}", 
            &uuid::Uuid::new_v4().to_string()[..8],
            chrono::Utc::now().timestamp()
        );
        
        info!("Vote cast successfully: {}", signature);
        Ok(signature)
    }
    
    /// Execute a custom program instruction
    async fn execute_custom(
        &self,
        program_id: &str,
        instruction_data: &str,
        user_wallet: &str,
    ) -> Result<String> {
        debug!("Executing custom instruction on program: {}", program_id);
        
        // In production, this would build and send custom transaction
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let signature = format!("custom_{}_{}", 
            &uuid::Uuid::new_v4().to_string()[..8],
            chrono::Utc::now().timestamp()
        );
        
        info!("Custom instruction executed: {}", signature);
        Ok(signature)
    }
    
    /// Get execution history for a blink
    pub async fn get_execution_history(&self, blink_id: &str) -> Vec<BlinkExecutionResult> {
        let cache = self.execution_cache.read().await;
        cache.get(blink_id)
            .map(|r| vec![r.clone()])
            .unwrap_or_default()
    }
    
    /// Validate if a blink can be executed
    pub async fn validate_execution(
        &self,
        blink: &SolanaBlink,
        user_wallet: &str,
    ) -> Result<()> {
        // Check blink validation
        let validation = blink.validate();
        if !validation.is_valid {
            return Err(BotError::validation(format!(
                "Invalid blink: {:?}",
                validation.errors
            )).into());
        }
        
        // Check expiration
        if let Some(expires_at) = blink.expires_at {
            if expires_at < Utc::now() {
                return Err(BotError::validation("Blink has expired").into());
            }
        }
        
        // Check wallet authorization
        if let Some(allowed) = &blink.security.allowed_wallets {
            if !allowed.contains(&user_wallet.to_string()) {
                return Err(BotError::validation("Wallet not authorized").into());
            }
        }
        
        // Check balance for the action
        match &blink.action.action_type {
            ActionType::Swap { amount, .. } | 
            ActionType::Transfer { amount, .. } |
            ActionType::Stake { amount, .. } => {
                // In production, would check actual wallet balance
                if *amount <= 0.0 {
                    return Err(BotError::validation("Invalid amount").into());
                }
            }
            ActionType::Mint { price, .. } => {
                if *price < 0.0 {
                    return Err(BotError::validation("Invalid mint price").into());
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}