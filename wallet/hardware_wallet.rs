use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::Transaction,
    message::Message,
};
use std::str::FromStr;

use crate::errors::{BotError, Result};
use crate::telemetry::TelemetryService;

/// Hardware wallet integration for secure transaction signing
#[derive(Clone)]
pub struct HardwareWalletManager {
    telemetry: Option<Arc<TelemetryService>>,
    wallets: Arc<RwLock<Vec<HardwareWallet>>>,
    active_wallet: Arc<RwLock<Option<String>>>,
    security_policies: Arc<RwLock<SecurityPolicies>>,
    transaction_cache: Arc<RwLock<TransactionCache>>,
}

/// Represents a connected hardware wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareWallet {
    pub wallet_id: String,
    pub wallet_type: WalletType,
    pub device_info: DeviceInfo,
    pub status: WalletStatus,
    pub derivation_path: String,
    pub public_key: String,
    pub capabilities: WalletCapabilities,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

/// Supported hardware wallet types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletType {
    Ledger {
        model: LedgerModel,
        firmware_version: String,
        app_version: String,
    },
    Trezor {
        model: TrezorModel,
        firmware_version: String,
        bootloader_version: String,
    },
    Keystone {
        model: String,
        firmware_version: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LedgerModel {
    NanoS,
    NanoSPlus,
    NanoX,
    Stax,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrezorModel {
    One,
    ModelT,
    Safe3,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub serial_number: Option<String>,
    pub label: Option<String>,
    pub initialized: bool,
    pub passphrase_protection: bool,
    pub pin_protection: bool,
    pub needs_backup: bool,
}

/// Wallet connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalletStatus {
    Connected,
    Disconnected,
    Locked,
    Unlocked,
    Error(String),
}

/// Wallet capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCapabilities {
    pub blind_signing: bool,
    pub message_signing: bool,
    pub multi_account: bool,
    pub u2f_support: bool,
    pub webusb_support: bool,
    pub bluetooth_support: bool,
}

/// Security policies for hardware wallet operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicies {
    pub require_physical_confirmation: bool,
    pub max_transaction_value: Option<u64>,
    pub daily_transaction_limit: Option<u64>,
    pub whitelist_only: bool,
    pub whitelisted_addresses: Vec<String>,
    pub require_2fa: bool,
    pub auto_lock_timeout_seconds: u32,
    pub transaction_review_mode: TransactionReviewMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionReviewMode {
    Minimal,    // Show only essential details
    Standard,   // Show standard transaction details
    Detailed,   // Show all transaction details
    Expert,     // Show raw transaction data
}

/// Transaction cache for offline signing
#[derive(Debug, Clone)]
pub struct TransactionCache {
    pub pending_transactions: Vec<PendingTransaction>,
    pub signed_transactions: Vec<SignedTransaction>,
    pub rejected_transactions: Vec<RejectedTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    pub transaction_id: String,
    pub transaction: Transaction,
    pub metadata: TransactionMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction_id: String,
    pub transaction: Transaction,
    pub signature: Signature,
    pub signed_at: chrono::DateTime<chrono::Utc>,
    pub wallet_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedTransaction {
    pub transaction_id: String,
    pub reason: String,
    pub rejected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    pub description: String,
    pub transaction_type: TransactionType,
    pub estimated_fees: u64,
    pub priority: TransactionPriority,
    pub risk_level: RiskLevel,
    pub requires_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    Swap,
    StakeDelegate,
    StakeDeactivate,
    CreateAccount,
    CloseAccount,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Ledger-specific transport interface
#[async_trait]
pub trait LedgerTransport: Send + Sync {
    async fn exchange(&self, apdu: &[u8]) -> Result<Vec<u8>>;
    async fn is_connected(&self) -> bool;
}

/// USB HID transport for Ledger devices
pub struct LedgerHIDTransport {
    device_path: String,
    // In production, this would use hidapi or similar
}

/// WebUSB transport for browser-based Ledger integration
pub struct LedgerWebUSBTransport {
    // WebUSB implementation details
}

/// Ledger Solana app interface
pub struct LedgerSolanaApp {
    transport: Arc<dyn LedgerTransport>,
    derivation_path: Vec<u32>,
}

impl LedgerSolanaApp {
    /// APDU command codes for Ledger Solana app
    const CLA: u8 = 0xe0;
    const INS_GET_APP_CONFIGURATION: u8 = 0x01;
    const INS_GET_PUBKEY: u8 = 0x02;
    const INS_SIGN_MESSAGE: u8 = 0x03;
    const INS_SIGN_OFFCHAIN_MESSAGE: u8 = 0x04;
    
    /// Create new Ledger Solana app instance
    pub fn new(transport: Arc<dyn LedgerTransport>, derivation_path: Vec<u32>) -> Self {
        Self {
            transport,
            derivation_path,
        }
    }
    
    /// Get app configuration
    pub async fn get_app_configuration(&self) -> Result<AppConfiguration> {
        let apdu = self.build_apdu(Self::INS_GET_APP_CONFIGURATION, 0, 0, &[]);
        let response = self.transport.exchange(&apdu).await?;
        
        if response.len() < 5 {
            return Err(BotError::hardware_wallet("Invalid app configuration response".to_string()).into());
        }
        
        Ok(AppConfiguration {
            app_version: format!("{}.{}.{}", response[1], response[2], response[3]),
            settings_mask: response[4],
        })
    }
    
    /// Get public key for derivation path
    pub async fn get_pubkey(&self, display_on_device: bool) -> Result<Pubkey> {
        let p1 = if display_on_device { 1 } else { 0 };
        let path_bytes = self.serialize_derivation_path();
        
        let apdu = self.build_apdu(Self::INS_GET_PUBKEY, p1, 0, &path_bytes);
        let response = self.transport.exchange(&apdu).await?;
        
        if response.len() < 32 {
            return Err(BotError::hardware_wallet("Invalid public key response".to_string()).into());
        }
        
        let pubkey_bytes: [u8; 32] = response[0..32].try_into()
            .map_err(|_| BotError::hardware_wallet("Invalid public key format".to_string()))?;
        
        Ok(Pubkey::new_from_array(pubkey_bytes))
    }
    
    /// Sign a transaction
    pub async fn sign_transaction(&self, transaction: &Transaction) -> Result<Signature> {
        let message_bytes = transaction.message_data();
        
        // Send transaction in chunks if needed (Ledger has APDU size limits)
        let chunks = self.chunk_data(&message_bytes, 255);
        
        for (i, chunk) in chunks.iter().enumerate() {
            let p1 = if i == 0 { 0x01 } else { 0x80 };
            let p2 = if i == chunks.len() - 1 { 0x80 } else { 0x00 };
            
            let mut data = Vec::new();
            if i == 0 {
                data.extend_from_slice(&self.serialize_derivation_path());
            }
            data.extend_from_slice(chunk);
            
            let apdu = self.build_apdu(Self::INS_SIGN_MESSAGE, p1, p2, &data);
            let response = self.transport.exchange(&apdu).await?;
            
            if i == chunks.len() - 1 {
                // Last chunk contains the signature
                if response.len() < 64 {
                    return Err(BotError::hardware_wallet("Invalid signature response".to_string()).into());
                }
                
                let sig_bytes: [u8; 64] = response[0..64].try_into()
                    .map_err(|_| BotError::hardware_wallet("Invalid signature format".to_string()))?;
                
                return Ok(Signature::new(&sig_bytes));
            }
        }
        
        Err(BotError::hardware_wallet("Failed to sign transaction".to_string()).into())
    }
    
    /// Build APDU command
    fn build_apdu(&self, ins: u8, p1: u8, p2: u8, data: &[u8]) -> Vec<u8> {
        let mut apdu = vec![Self::CLA, ins, p1, p2, data.len() as u8];
        apdu.extend_from_slice(data);
        apdu
    }
    
    /// Serialize derivation path for APDU
    fn serialize_derivation_path(&self) -> Vec<u8> {
        let mut bytes = vec![self.derivation_path.len() as u8];
        for component in &self.derivation_path {
            bytes.extend_from_slice(&component.to_be_bytes());
        }
        bytes
    }
    
    /// Chunk data for APDU transmission
    fn chunk_data(&self, data: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
        data.chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct AppConfiguration {
    pub app_version: String,
    pub settings_mask: u8,
}

impl HardwareWalletManager {
    /// Create new hardware wallet manager
    pub fn new(telemetry: Option<Arc<TelemetryService>>) -> Self {
        info!("🔐 Initializing hardware wallet manager");
        
        Self {
            telemetry,
            wallets: Arc::new(RwLock::new(Vec::new())),
            active_wallet: Arc::new(RwLock::new(None)),
            security_policies: Arc::new(RwLock::new(SecurityPolicies::default())),
            transaction_cache: Arc::new(RwLock::new(TransactionCache {
                pending_transactions: Vec::new(),
                signed_transactions: Vec::new(),
                rejected_transactions: Vec::new(),
            })),
        }
    }
    
    /// Scan for connected hardware wallets
    pub async fn scan_for_wallets(&self) -> Result<Vec<HardwareWallet>> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_trading_span("hardware_wallet_scan", None)
        );
        
        info!("🔐 Scanning for hardware wallets...");
        
        let mut found_wallets = Vec::new();
        
        // Scan for Ledger devices
        if let Ok(ledger_wallets) = self.scan_ledger_devices().await {
            found_wallets.extend(ledger_wallets);
        }
        
        // Scan for Trezor devices
        if let Ok(trezor_wallets) = self.scan_trezor_devices().await {
            found_wallets.extend(trezor_wallets);
        }
        
        // Update stored wallets
        let mut wallets = self.wallets.write().await;
        *wallets = found_wallets.clone();
        
        info!("🔐 Found {} hardware wallet(s)", found_wallets.len());
        
        Ok(found_wallets)
    }
    
    /// Connect to a specific hardware wallet
    pub async fn connect_wallet(&self, wallet_id: &str) -> Result<()> {
        let wallets = self.wallets.read().await;
        let wallet = wallets.iter()
            .find(|w| w.wallet_id == wallet_id)
            .ok_or_else(|| BotError::not_found(format!("Wallet {} not found", wallet_id)))?;
        
        match &wallet.wallet_type {
            WalletType::Ledger { .. } => {
                self.connect_ledger(wallet).await?;
            },
            WalletType::Trezor { .. } => {
                self.connect_trezor(wallet).await?;
            },
            WalletType::Keystone { .. } => {
                return Err(BotError::hardware_wallet("Keystone not yet supported".to_string()).into());
            },
        }
        
        // Set as active wallet
        let mut active = self.active_wallet.write().await;
        *active = Some(wallet_id.to_string());
        
        info!("🔐 Connected to hardware wallet: {}", wallet_id);
        
        Ok(())
    }
    
    /// Sign a transaction with the active hardware wallet
    pub async fn sign_transaction(&self, transaction: &Transaction) -> Result<Signature> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_trading_span("hardware_wallet_sign", None)
        );
        
        // Get active wallet
        let active_wallet_id = {
            let active = self.active_wallet.read().await;
            active.clone().ok_or_else(|| BotError::hardware_wallet("No active wallet".to_string()))?
        };
        
        let wallets = self.wallets.read().await;
        let wallet = wallets.iter()
            .find(|w| w.wallet_id == active_wallet_id)
            .ok_or_else(|| BotError::not_found("Active wallet not found".to_string()))?;
        
        // Check security policies
        self.check_security_policies(transaction).await?;
        
        // Create transaction metadata
        let metadata = self.analyze_transaction(transaction).await?;
        
        // Add to pending transactions
        let pending_tx = PendingTransaction {
            transaction_id: uuid::Uuid::new_v4().to_string(),
            transaction: transaction.clone(),
            metadata,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
        };
        
        {
            let mut cache = self.transaction_cache.write().await;
            cache.pending_transactions.push(pending_tx.clone());
        }
        
        // Sign based on wallet type
        let signature = match &wallet.wallet_type {
            WalletType::Ledger { .. } => {
                self.sign_with_ledger(wallet, transaction).await?
            },
            WalletType::Trezor { .. } => {
                self.sign_with_trezor(wallet, transaction).await?
            },
            WalletType::Keystone { .. } => {
                return Err(BotError::hardware_wallet("Keystone not yet supported".to_string()).into());
            },
        };
        
        // Move to signed transactions
        {
            let mut cache = self.transaction_cache.write().await;
            cache.pending_transactions.retain(|tx| tx.transaction_id != pending_tx.transaction_id);
            cache.signed_transactions.push(SignedTransaction {
                transaction_id: pending_tx.transaction_id,
                transaction: transaction.clone(),
                signature,
                signed_at: chrono::Utc::now(),
                wallet_id: active_wallet_id,
            });
        }
        
        info!("🔐 Transaction signed successfully");
        
        Ok(signature)
    }
    
    /// Get wallet balance and account info
    pub async fn get_wallet_info(&self, wallet_id: &str) -> Result<WalletInfo> {
        let wallets = self.wallets.read().await;
        let wallet = wallets.iter()
            .find(|w| w.wallet_id == wallet_id)
            .ok_or_else(|| BotError::not_found(format!("Wallet {} not found", wallet_id)))?;
        
        let pubkey = Pubkey::from_str(&wallet.public_key)
            .map_err(|e| BotError::parsing(format!("Invalid public key: {}", e)))?;
        
        // Would fetch actual balance from blockchain
        Ok(WalletInfo {
            wallet_id: wallet_id.to_string(),
            public_key: wallet.public_key.clone(),
            balance: 0, // Would fetch actual balance
            token_accounts: Vec::new(), // Would fetch token accounts
        })
    }
    
    /// Update security policies
    pub async fn update_security_policies(&self, policies: SecurityPolicies) -> Result<()> {
        let mut current_policies = self.security_policies.write().await;
        *current_policies = policies;
        
        info!("🔐 Updated hardware wallet security policies");
        Ok(())
    }
    
    // Private helper methods
    async fn scan_ledger_devices(&self) -> Result<Vec<HardwareWallet>> {
        // In production, this would use hidapi to scan for Ledger devices
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }
    
    async fn scan_trezor_devices(&self) -> Result<Vec<HardwareWallet>> {
        // In production, this would use trezor-client to scan for devices
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }
    
    async fn connect_ledger(&self, _wallet: &HardwareWallet) -> Result<()> {
        // Implementation would establish connection to Ledger device
        Ok(())
    }
    
    async fn connect_trezor(&self, _wallet: &HardwareWallet) -> Result<()> {
        // Implementation would establish connection to Trezor device
        Ok(())
    }
    
    async fn sign_with_ledger(&self, wallet: &HardwareWallet, transaction: &Transaction) -> Result<Signature> {
        // In production, this would use the LedgerSolanaApp to sign
        // For now, return a dummy signature
        debug!("🔐 Signing transaction with Ledger wallet: {}", wallet.wallet_id);
        
        // Parse derivation path
        let derivation_path = self.parse_derivation_path(&wallet.derivation_path)?;
        
        // Create Ledger app instance (would use actual transport in production)
        // let transport = Arc::new(LedgerHIDTransport { device_path: "".to_string() });
        // let app = LedgerSolanaApp::new(transport, derivation_path);
        // let signature = app.sign_transaction(transaction).await?;
        
        // Placeholder signature
        Ok(Signature::default())
    }
    
    async fn sign_with_trezor(&self, wallet: &HardwareWallet, _transaction: &Transaction) -> Result<Signature> {
        // In production, this would use trezor-client to sign
        debug!("🔐 Signing transaction with Trezor wallet: {}", wallet.wallet_id);
        
        // Placeholder signature
        Ok(Signature::default())
    }
    
    async fn check_security_policies(&self, transaction: &Transaction) -> Result<()> {
        let policies = self.security_policies.read().await;
        
        // Check transaction value limits
        if let Some(max_value) = policies.max_transaction_value {
            // Would calculate actual transaction value
            let tx_value = 0u64; // Placeholder
            if tx_value > max_value {
                return Err(BotError::hardware_wallet(
                    format!("Transaction value {} exceeds maximum {}", tx_value, max_value)
                ).into());
            }
        }
        
        // Check whitelist
        if policies.whitelist_only {
            // Would check if recipient is whitelisted
        }
        
        Ok(())
    }
    
    async fn analyze_transaction(&self, _transaction: &Transaction) -> Result<TransactionMetadata> {
        // Would analyze transaction to determine type, fees, risk level, etc.
        Ok(TransactionMetadata {
            description: "Solana transaction".to_string(),
            transaction_type: TransactionType::Transfer,
            estimated_fees: 5000,
            priority: TransactionPriority::Normal,
            risk_level: RiskLevel::Low,
            requires_review: false,
        })
    }
    
    fn parse_derivation_path(&self, path_str: &str) -> Result<Vec<u32>> {
        // Parse BIP44 derivation path like "m/44'/501'/0'/0'"
        let components: Result<Vec<u32>, _> = path_str
            .trim_start_matches("m/")
            .split('/')
            .map(|s| {
                let hardened = s.ends_with('\'');
                let num_str = if hardened { &s[..s.len()-1] } else { s };
                let num: u32 = num_str.parse()
                    .map_err(|_| BotError::parsing(format!("Invalid derivation path component: {}", s)))?;
                Ok(if hardened { num | 0x80000000 } else { num })
            })
            .collect();
        
        components
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub wallet_id: String,
    pub public_key: String,
    pub balance: u64,
    pub token_accounts: Vec<TokenAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccount {
    pub mint: String,
    pub balance: u64,
    pub decimals: u8,
}

impl Default for SecurityPolicies {
    fn default() -> Self {
        Self {
            require_physical_confirmation: true,
            max_transaction_value: None,
            daily_transaction_limit: None,
            whitelist_only: false,
            whitelisted_addresses: Vec::new(),
            require_2fa: false,
            auto_lock_timeout_seconds: 300, // 5 minutes
            transaction_review_mode: TransactionReviewMode::Standard,
        }
    }
}

/// Create a standard Solana derivation path
pub fn solana_derivation_path(account: u32, change: u32) -> String {
    format!("m/44'/501'/{}'/0'/{}'", account, change)
}