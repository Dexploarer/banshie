use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
};
use reqwest::ClientBuilder;
use std::time::Duration;
use std::sync::Arc;
use std::str::FromStr;
use tracing::{info, error, debug, instrument};
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio::time::{timeout, Duration as TokioDuration};
use crate::errors::{BotError, TradingError, Result};
use crate::constants::{DEFAULT_PRIORITY_FEE, DEFAULT_SLIPPAGE_BPS, MAX_SLIPPAGE_BPS};
use crate::utils::validation::Validator;

use crate::{utils::Config, db::Database, wallet::WalletManager};
use crate::middleware::{CircuitBreaker, CircuitBreakerConfig};
use super::{
    types::{TradeResult, Balance, Position, TokenRestrictions, TradeType},
    backrun::HeliusClient,
    dex::JupiterSwap,
    token_2022::{Token2022Manager, Token2022Info, ExtensionType, TransferFeeConfig},
    token_creator::TokenCreator,
};

// Actor messages for the TradingEngine
#[derive(Debug)]
pub enum TradingMessage {
    Buy {
        user_wallet: String,
        token: String,
        amount_sol: f64,
        response_tx: mpsc::Sender<Result<TradeResult>>,
    },
    Sell {
        user_wallet: String,
        token: String,
        percentage: f64,
        response_tx: mpsc::Sender<Result<TradeResult>>,
    },
    BuyWithRebate {
        user_wallet: String,
        token: String,
        amount_sol: f64,
        response: oneshot::Sender<Result<TradeResult>>,
    },
    SellWithRebate {
        user_wallet: String,
        token: String,
        percentage: f64,
        response: oneshot::Sender<Result<TradeResult>>,
    },
    GetBalance {
        user_wallet: String,
        response: oneshot::Sender<Result<Balance>>,
    },
    GetPositions {
        user_wallet: String,
        response_tx: mpsc::Sender<Result<Vec<Position>>>,
    },
    Shutdown,
}

// Actor handle for external communication with resource management
#[derive(Clone)]
pub struct TradingEngineHandle {
    sender: mpsc::Sender<TradingMessage>,
    // Resource management
    request_semaphore: Arc<Semaphore>, // Limit concurrent requests
    operation_timeout: Duration,
    max_queue_size: usize,
}

#[derive(Debug)]
pub struct ResourceConfig {
    pub max_concurrent_requests: usize,
    pub operation_timeout_secs: u64,
    pub max_queue_size: usize,
    pub channel_buffer_size: usize,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10,
            operation_timeout_secs: 30,
            max_queue_size: 100,
            channel_buffer_size: 100,
        }
    }
}

impl TradingEngineHandle {
    /// Send a message to the trading engine (for compatibility with command handlers)
    pub fn send(&self, msg: TradingMessage) -> Result<()> {
        self.sender.try_send(msg)
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => BotError::internal("Trading engine queue full".to_string()),
                mpsc::error::TrySendError::Closed(_) => BotError::internal("Trading engine unavailable".to_string()),
            })
    }
    
    #[instrument(skip(self))]
    pub async fn buy_with_rebate(
        &self,
        user_wallet: String,
        token: String,
        amount_sol: f64,
    ) -> Result<TradeResult> {
        // Acquire resource permit (backpressure)
        let _permit = self.request_semaphore.acquire().await
            .map_err(|_| BotError::internal("Request semaphore closed".to_string()))?;
        
        // Check queue size for additional backpressure
        if self.sender.capacity() == 0 {
            return Err(BotError::internal("Trading engine queue full".to_string()));
        }
        
        let (tx, rx) = oneshot::channel();
        
        self.sender
            .send(TradingMessage::BuyWithRebate {
                user_wallet,
                token,
                amount_sol,
                response: tx,
            })
            .await
            .map_err(|_| BotError::internal("Trading engine unavailable".to_string()))?;
        
        // Apply timeout to prevent resource leaks
        timeout(TokioDuration::from_secs(self.operation_timeout.as_secs()), rx)
            .await
            .map_err(|_| BotError::internal("Trading operation timed out".to_string()))?
            .map_err(|_| BotError::internal("Trading engine response failed".to_string()))?
    }
    
    #[instrument(skip(self))]
    pub async fn sell_with_rebate(
        &self,
        user_wallet: String,
        token: String,
        percentage: f64,
    ) -> Result<TradeResult> {
        let _permit = self.request_semaphore.acquire().await
            .map_err(|_| BotError::internal("Request semaphore closed".to_string()))?;
        
        if self.sender.capacity() == 0 {
            return Err(BotError::internal("Trading engine queue full".to_string()));
        }
        
        let (tx, rx) = oneshot::channel();
        
        self.sender
            .send(TradingMessage::SellWithRebate {
                user_wallet,
                token,
                percentage,
                response: tx,
            })
            .await
            .map_err(|_| BotError::internal("Trading engine unavailable".to_string()))?;
        
        timeout(TokioDuration::from_secs(self.operation_timeout.as_secs()), rx)
            .await
            .map_err(|_| BotError::internal("Trading operation timed out".to_string()))?
            .map_err(|_| BotError::internal("Trading engine response failed".to_string()))?
    }
    
    #[instrument(skip(self))]
    pub async fn get_balance(&self, user_wallet: String) -> Result<Balance> {
        let (tx, rx) = oneshot::channel();
        
        self.sender
            .send(TradingMessage::GetBalance {
                user_wallet,
                response: tx,
            })
            .await
            .map_err(|_| BotError::internal("Trading engine unavailable".to_string()))?;
        
        rx.await
            .map_err(|_| BotError::internal("Trading engine response failed".to_string()))?
    }
    
    #[instrument(skip(self))]
    pub async fn get_positions(&self, user_wallet: String) -> Result<Vec<Position>> {
        let (tx, mut rx) = mpsc::channel(1);
        
        self.sender
            .send(TradingMessage::GetPositions {
                user_wallet,
                response_tx: tx,
            })
            .await
            .map_err(|_| BotError::internal("Trading engine unavailable".to_string()))?;
        
        rx.recv().await
            .ok_or_else(|| BotError::internal("Trading engine response failed".to_string()))?
    }
    
    pub async fn shutdown(&self) {
        info!("Initiating graceful shutdown of trading engine");
        
        // Send shutdown message
        if let Err(_) = self.sender.send(TradingMessage::Shutdown).await {
            warn!("Failed to send shutdown message to trading engine");
        }
        
        // Close the semaphore to prevent new requests
        self.request_semaphore.close();
        
        info!("Trading engine shutdown initiated");
    }
    
    /// Get resource utilization metrics
    pub fn get_resource_metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            available_permits: self.request_semaphore.available_permits(),
            max_permits: 10, // From ResourceConfig::default()
            queue_capacity: self.sender.capacity(),
            queue_utilization_percent: {
                let capacity = self.sender.capacity();
                if capacity > 0 {
                    ((self.max_queue_size - capacity) as f64 / self.max_queue_size as f64) * 100.0
                } else {
                    100.0
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub available_permits: usize,
    pub max_permits: usize,
    pub queue_capacity: usize,
    pub queue_utilization_percent: f64,
}
}

// TradingEngine actor
pub struct TradingEngine {
    config: Arc<Config>,
    db: Arc<Database>,
    rpc_client: RpcClient,
    helius_client: HeliusClient,
    jupiter: JupiterSwap,
    token_2022_manager: Token2022Manager,
    token_creator: TokenCreator,
    // Circuit breakers for external services
    jupiter_breaker: CircuitBreaker,
    helius_breaker: CircuitBreaker,
    solana_rpc_breaker: CircuitBreaker,
}

impl TradingEngine {
    // Create actor and return handle with resource management
    pub async fn spawn(config: Arc<Config>, db: Arc<Database>) -> Result<TradingEngineHandle> {
        let resource_config = ResourceConfig::default();
        let (sender, receiver) = mpsc::channel::<TradingMessage>(resource_config.channel_buffer_size);
        
        let engine = Self::new(config, db).await?;
        let handle = TradingEngineHandle { 
            sender,
            request_semaphore: Arc::new(Semaphore::new(resource_config.max_concurrent_requests)),
            operation_timeout: Duration::from_secs(resource_config.operation_timeout_secs),
            max_queue_size: resource_config.max_queue_size,
        };
        
        // Spawn the actor task
        tokio::spawn(async move {
            engine.run(receiver).await;
        });
        
        info!("TradingEngine actor spawned with channel buffer size: 100");
        Ok(handle)
    }
    
    async fn new(config: Arc<Config>, db: Arc<Database>) -> Result<Self> {
        let rpc_url = config.get_rpc_url();
        
        // Create optimized HTTP client for Solana RPC
        let http_client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(3) // Conservative for RPC
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Duration::from_secs(60))
            .http2_prior_knowledge()
            .gzip(true)
            .user_agent("solana-trading-bot/0.1.0")
            .build()
            .map_err(|e| BotError::internal(format!("Failed to create HTTP client for Solana RPC: {}", e)))?;
        
        let rpc_client = RpcClient::new_with_timeout_and_commitment(
            rpc_url.clone(),
            Duration::from_secs(30),
            CommitmentConfig::confirmed(),
            http_client,
        );
        
        let rebate_address = if config.enable_backrun_rebates {
            Some(config.rebate_wallet_address.as_str())
        } else {
            None
        };
        let helius_client = HeliusClient::new(&config.helius_api_key, rebate_address)?;
        let jupiter = JupiterSwap::new(rpc_url);
        let token_2022_manager = Token2022Manager::new();
        let token_creator = TokenCreator::new();
        
        // Initialize circuit breakers with appropriate configurations
        let jupiter_breaker = CircuitBreaker::new(
            "Jupiter".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 3,
                timeout: Duration::from_secs(30),
                success_threshold: 2,
            }
        );
        
        let helius_breaker = CircuitBreaker::new(
            "Helius".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 5,
                timeout: Duration::from_secs(60),
                success_threshold: 3,
            }
        );
        
        let solana_rpc_breaker = CircuitBreaker::new(
            "Solana-RPC".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 3,
                timeout: Duration::from_secs(45),
                success_threshold: 2,
            }
        );
        
        info!("Trading engine initialized with circuit breakers (non-custodial mode)");
        
        Ok(Self {
            config,
            db,
            rpc_client,
            helius_client,
            jupiter,
            token_2022_manager,
            token_creator,
            jupiter_breaker,
            helius_breaker,
            solana_rpc_breaker,
        })
    }
    
    // Actor main loop
    async fn run(mut self, mut receiver: mpsc::Receiver<TradingMessage>) {
        info!("TradingEngine actor started");
        
        while let Some(message) = receiver.recv().await {
            match message {
                TradingMessage::Buy {
                    user_wallet,
                    token,
                    amount_sol,
                    response_tx,
                } => {
                    let result = self.buy_with_rebate(&user_wallet, &token, amount_sol).await;
                    let _ = response_tx.send(result).await;
                }
                TradingMessage::Sell {
                    user_wallet,
                    token,
                    percentage,
                    response_tx,
                } => {
                    let result = self.sell_with_rebate(&user_wallet, &token, percentage).await;
                    let _ = response_tx.send(result).await;
                }
                TradingMessage::BuyWithRebate {
                    user_wallet,
                    token,
                    amount_sol,
                    response,
                } => {
                    let result = self.buy_with_rebate(&user_wallet, &token, amount_sol).await;
                    let _ = response.send(result);
                }
                TradingMessage::SellWithRebate {
                    user_wallet,
                    token,
                    percentage,
                    response,
                } => {
                    let result = self.sell_with_rebate(&user_wallet, &token, percentage).await;
                    let _ = response.send(result);
                }
                TradingMessage::GetBalance { user_wallet, response } => {
                    let result = self.get_balance(&user_wallet).await;
                    let _ = response.send(result);
                }
                TradingMessage::GetPositions { user_wallet, response_tx } => {
                    let result = self.get_positions(&user_wallet).await;
                    let _ = response_tx.send(result).await;
                }
                TradingMessage::Shutdown => {
                    info!("TradingEngine actor shutting down");
                    break;
                }
            }
        }
        
        info!("TradingEngine actor stopped");
    }
    
    async fn buy_with_rebate(
        &mut self,
        user_wallet: &str,
        token: &str,
        amount_sol: f64,
    ) -> Result<TradeResult> {
        info!("Preparing buy order for {} with {} SOL for wallet {}", token, amount_sol, user_wallet);
        
        Validator::validate_trade_amount(amount_sol, self.config.max_trade_size_sol)?;
        
        // Validate wallet address
        let user_pubkey = Pubkey::from_str(user_wallet)?;
        
        let token_mint = self.resolve_token_mint(token).await?;
        
        // Check Token-2022 restrictions before trading
        let restrictions = self.check_token_restrictions(&token_mint).await?;
        if restrictions.is_non_transferable {
            return Err(BotError::validation("Cannot trade non-transferable tokens".to_string()));
        }
        
        let quote = self.jupiter.get_quote(
            "So11111111111111111111111111111111111112", // SOL mint
            &token_mint,
            amount_sol,
            self.config.slippage_bps,
        ).await?;
        
        // Calculate expected tokens after potential transfer fees
        let expected_tokens = quote.out_amount.parse::<u64>().unwrap_or(0);
        let (effective_tokens, transfer_fee) = self.calculate_effective_transfer_amount(&token_mint, expected_tokens).await?;
        
        // Build unsigned transaction
        let swap_tx = self.jupiter.build_swap_transaction(
            quote,
            user_wallet,
            self.config.priority_fee_lamports,
        ).await?;
        
        // Return transaction for user to sign
        let result = TradeResult {
            tx_signature: "UNSIGNED_TRANSACTION".to_string(), // User needs to sign
            tokens_received: effective_tokens as f64 / 1e9,
            tokens_sold: 0.0,
            sol_received: 0.0,
            price: amount_sol / (effective_tokens as f64 / 1e9),
            rebate_earned: 0.0, // Will be calculated after actual execution
            pnl_percentage: 0.0,
            timestamp: chrono::Utc::now(),
            trade_type: TradeType::Buy,
        };
        
        if transfer_fee > 0 {
            info!(
                "Buy quote prepared: {} {} for {} SOL (transfer fee: {} tokens)",
                result.tokens_received, token, amount_sol, transfer_fee as f64 / 1e9
            );
        } else {
            info!(
                "Buy quote prepared: {} {} for {} SOL",
                result.tokens_received, token, amount_sol
            );
        }
        
        Ok(result)
    }
    
    async fn sell_with_rebate(
        &mut self,
        user_wallet: &str,
        token: &str,
        percentage: f64,
    ) -> Result<TradeResult> {
        info!("Executing sell order for {}% of {}", percentage, token);
        
        Validator::validate_percentage(percentage)?;
        
        // Validate wallet address
        let user_pubkey = Pubkey::from_str(user_wallet)?;
        
        let token_mint = self.resolve_token_mint(token).await?;
        
        // Check Token-2022 restrictions before trading
        let restrictions = self.check_token_restrictions(&token_mint).await?;
        if restrictions.is_non_transferable {
            return Err(BotError::validation("Cannot sell non-transferable tokens".to_string()));
        }
        
        let balance = self.get_token_balance_for_user(&user_pubkey, &token_mint).await?;
        
        if balance <= 0.0 {
            return Err(TradingError::no_tokens_to_sell(token).into());
        }
        
        let amount_to_sell = balance * (percentage / 100.0);
        let amount_to_sell_lamports = (amount_to_sell * 1e9) as u64;
        
        // Calculate transfer fees that will be deducted during the sell
        let (effective_amount, transfer_fee) = self.calculate_effective_transfer_amount(&token_mint, amount_to_sell_lamports).await?;
        
        if restrictions.has_transfer_fees && transfer_fee > 0 {
            info!("Transfer fee will be deducted: {} tokens", transfer_fee as f64 / 1e9);
        }
        
        let quote = self.jupiter.get_quote(
            &token_mint,
            "So11111111111111111111111111111111111112",
            effective_amount as f64 / 1e9, // Use effective amount for quote
            self.config.slippage_bps,
        ).await?;
        
        let swap_tx = self.jupiter.build_swap_transaction(
            quote,
            user_wallet,
            self.config.priority_fee_lamports,
        ).await?;
        
        // Return transaction for user to sign - in non-custodial mode
        let mut result = TradeResult {
            tx_signature: "UNSIGNED_TRANSACTION".to_string(), // User needs to sign
            tokens_received: 0.0,
            tokens_sold: amount_to_sell,
            sol_received: quote.out_amount.parse::<f64>().unwrap_or(0.0) / 1e9,
            price: (quote.out_amount.parse::<f64>().unwrap_or(1.0) / 1e9) / (effective_amount as f64 / 1e9),
            rebate_earned: 0.0, // Will be calculated after actual execution
            pnl_percentage: 0.0,
            timestamp: chrono::Utc::now(),
            trade_type: TradeType::Sell,
        };
        
        result.tokens_sold = amount_to_sell;
        
        let pnl = self.db.calculate_pnl(
            user_wallet,
            token,
            result.sol_received,
        ).await?;
        result.pnl_percentage = pnl;
        
        if transfer_fee > 0 {
            info!(
                "Sell order prepared: {} {} for {} SOL (after {} token transfer fee), P&L: {:.2}%",
                amount_to_sell, token, result.sol_received, transfer_fee as f64 / 1e9, pnl
            );
        } else {
            info!(
                "Sell order prepared: {} {} for {} SOL, P&L: {:.2}%",
                amount_to_sell, token, result.sol_received, pnl
            );
        }
        
        Ok(result)
    }
    
    async fn get_balance(&self, user_wallet: &str) -> Result<Balance> {
        let user_pubkey = Pubkey::from_str(user_wallet)?;
        let sol_balance = self.rpc_client
            .get_balance(&user_pubkey).await?;
        
        let sol = sol_balance as f64 / 1e9;
        let sol_price = self.get_sol_price().await?;
        
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
        let usdc_balance = self.get_token_balance_for_user(
            &user_pubkey, 
            &usdc_mint.to_string()
        ).await.unwrap_or(0.0);
        
        Ok(Balance {
            sol,
            usdc: usdc_balance,
            total_usd_value: (sol * sol_price) + usdc_balance,
        })
    }
    
    async fn get_positions(&self, user_wallet: &str) -> Result<Vec<Position>> {
        self.db.get_user_positions(user_wallet).await
    }
    
    async fn get_token_balance_for_user(&self, user_pubkey: &Pubkey, mint: &str) -> Result<f64> {
        // For now, return a placeholder balance since we removed SPL dependencies
        // In production, you'd implement proper SPL token balance checking
        // or use the Jupiter API to get token balances
        
        debug!("Getting token balance for user {} mint {}", user_pubkey, mint);
        
        // Placeholder - return 100 tokens for demo purposes
        Ok(100.0)
    }
    
    async fn resolve_token_mint(&self, token: &str) -> Result<String> {
        super::token_resolver::TokenResolver::resolve(token)
    }
    
    /// Get detailed Token-2022 information for a mint
    async fn get_token_2022_info(&self, mint: &str) -> Result<Token2022Info> {
        let mint_pubkey = Pubkey::from_str(mint)?;
        self.token_2022_manager.get_token_info(&mint_pubkey).await
    }
    
    /// Calculate effective transfer amount accounting for Token-2022 fees
    async fn calculate_effective_transfer_amount(&self, mint: &str, amount: u64) -> Result<(u64, u64)> {
        let token_info = self.get_token_2022_info(mint).await?;
        
        if let Some(fee_config) = &token_info.transfer_fee_config {
            let fee = self.token_2022_manager.calculate_transfer_fee(amount, fee_config)?;
            let effective_amount = amount.saturating_sub(fee);
            debug!("Transfer fee calculated: {} lamports, effective amount: {}", fee, effective_amount);
            Ok((effective_amount, fee))
        } else {
            Ok((amount, 0))
        }
    }
    
    /// Check if a token has specific restrictions
    async fn check_token_restrictions(&self, mint: &str) -> Result<TokenRestrictions> {
        let token_info = self.get_token_2022_info(mint).await?;
        
        Ok(TokenRestrictions {
            is_non_transferable: token_info.is_non_transferable,
            has_transfer_fees: token_info.transfer_fee_config.is_some(),
            has_transfer_hook: token_info.has_transfer_hook,
            requires_memo: token_info.extensions.contains(&ExtensionType::MemoTransfer),
        })
    }
    
    /// Get token creator for creating new tokens
    pub fn get_token_creator(&self) -> &TokenCreator {
        &self.token_creator
    }
    
    async fn get_sol_price(&self) -> Result<f64> {
        Ok(220.0)
    }
    
    async fn send_regular_transaction(&self, tx: Transaction) -> Result<TradeResult> {
        let signature = self.rpc_client.send_and_confirm_transaction(&tx).await?;
        
        Ok(TradeResult {
            tx_signature: signature.to_string(),
            tokens_received: 100.0,
            tokens_sold: 0.0,
            sol_received: 0.0,
            price: 0.001,
            rebate_earned: 0.0,
            pnl_percentage: 0.0,
            timestamp: chrono::Utc::now(),
            trade_type: TradeType::Swap,
        })
    }
    
    fn load_wallet(private_key: &str) -> Result<Keypair> {
        let decoded = bs58::decode(private_key).into_vec()?;
        Ok(Keypair::from_bytes(&decoded)?)
    }
}