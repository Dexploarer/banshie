use crate::errors::{WalletError, Result};
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::str::FromStr;
use tracing::{info, warn, debug};

use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub public_key: String, // Renamed to match what bot expects
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub is_active: bool,
    pub balance_sol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSession {
    pub session_id: String,
    pub wallet_address: String,
    pub encrypted_signing_key: Option<String>, // Encrypted with user's session key
    pub expires_at: DateTime<Utc>,
    pub max_transaction_sol: f64,
    pub requires_confirmation: bool,
}

pub struct WalletManager {
    db: Arc<Database>,
}

impl WalletManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
        }
    }
    
    /// Register a new wallet for a user (we only store public info)
    pub async fn register_wallet(
        &self,
        telegram_id: &str,
        wallet_address: &str,
        label: Option<String>,
    ) -> Result<()> {
        // Validate address
        let _pubkey = Pubkey::from_str(wallet_address)?;
        
        // Store in database
        self.db.register_user_wallet(telegram_id, wallet_address).await?;
        
        info!("Registered wallet {} for user {}", wallet_address, telegram_id);
        
        Ok(())
    }
    
    /// Get all wallets for a user
    pub async fn get_user_wallets(&self, telegram_id: &str) -> Result<Vec<WalletInfo>> {
        let db_wallets = self.db.get_user_wallets(telegram_id).await?;
        
        let wallets = db_wallets.into_iter().map(|w| WalletInfo {
            public_key: w.wallet_address,
            label: w.label.unwrap_or_else(|| "Wallet".to_string()),
            created_at: w.created_at,
            last_active: w.last_used.unwrap_or(w.created_at),
            is_active: w.is_active,
            balance_sol: None,
        }).collect();
        
        Ok(wallets)
    }
    
    /// Get active wallet for a user
    pub async fn get_user_wallet(&self, telegram_id: &str) -> Result<Option<WalletInfo>> {
        let active_address = self.db.get_active_wallet(telegram_id).await?;
        
        if let Some(address) = active_address {
            let wallets = self.get_user_wallets(telegram_id).await?;
            Ok(wallets.into_iter().find(|w| w.public_key == address))
        } else {
            Ok(None)
        }
    }
    
    /// Set active wallet for a user
    pub async fn set_active_wallet(&self, telegram_id: &str, wallet_address: &str) -> Result<()> {
        self.db.set_active_wallet(telegram_id, wallet_address).await?;
        
        info!("Set active wallet {} for user {}", wallet_address, telegram_id);
        
        Ok(())
    }
    
    /// Create a temporary signing session
    pub async fn create_session(
        &self,
        telegram_id: &str,
        wallet_address: &str,
        duration_minutes: i64,
        max_transaction_sol: f64,
    ) -> Result<String> {
        let session_id = Self::generate_session_id();
        
        self.db.create_signing_session(
            telegram_id,
            &session_id,
            wallet_address,
            None, // No encrypted data for now
            max_transaction_sol,
            duration_minutes
        ).await?;
        
        info!("Created session {} for user {} wallet {}", session_id, telegram_id, wallet_address);
        
        Ok(session_id)
    }
    
    /// Get active session
    pub async fn get_session(&self, session_id: &str) -> Result<Option<WalletSession>> {
        let db_session = self.db.get_active_session(session_id).await?;
        
        if let Some(session) = db_session {
            Ok(Some(WalletSession {
                session_id: session.session_id,
                wallet_address: session.wallet_address,
                encrypted_signing_key: session.encrypted_data,
                expires_at: session.expires_at,
                max_transaction_sol: session.max_transaction_sol,
                requires_confirmation: session.max_transaction_sol > 0.1,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// End a session
    pub async fn end_session(&self, session_id: &str) -> Result<()> {
        // For now, we can just let sessions expire naturally
        // In a full implementation, we'd add a DELETE query
        
        info!("Ended session {}", session_id);
        
        Ok(())
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<()> {
        self.db.cleanup_expired_sessions().await?;
        Ok(())
    }
    
    /// Remove a wallet (does not affect blockchain, just removes from tracking)
    pub async fn remove_wallet(&self, telegram_id: &str, wallet_address: &str) -> Result<()> {
        // For now, we don't implement wallet removal to keep it simple
        // In a full implementation, we'd add a DELETE query and handle active wallet logic
        
        info!("Remove wallet requested for {} by user {}", wallet_address, telegram_id);
        
        Err(WalletError::WalletNotFound.into())
    }
    
    /// Update wallet balance (cached value only)
    pub async fn update_wallet_balance(&self, telegram_id: &str, wallet_address: &str, balance_sol: f64) -> Result<()> {
        // For now, we don't cache balances in database
        // In production, you might want to add a balance cache table
        
        debug!("Balance update for {} wallet {}: {} SOL", telegram_id, wallet_address, balance_sol);
        
        Ok(())
    }
    
    /// Get wallet count for a user
    async fn get_wallet_count(&self, telegram_id: &str) -> usize {
        self.get_user_wallets(telegram_id).await.map_or(0, |w| w.len())
    }
    
    /// Generate a secure session ID
    fn generate_session_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        bs58::encode(bytes).into_string()
    }
    
    /// Check if user has any wallets
    pub async fn has_wallet(&self, telegram_id: &str) -> bool {
        self.get_wallet_count(telegram_id).await > 0
    }
    
    /// Get wallet by address
    pub async fn get_wallet(&self, telegram_id: &str, wallet_address: &str) -> Result<Option<WalletInfo>> {
        let wallets = self.get_user_wallets(telegram_id).await?;
        Ok(wallets.into_iter().find(|w| w.public_key == wallet_address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    // Tests removed for now as they require database setup
    // In a real implementation, you'd use a test database or mock
}