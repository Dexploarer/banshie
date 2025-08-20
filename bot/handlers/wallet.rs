use teloxide::{prelude::*, types::{Message, CallbackQuery}};
use std::sync::Arc;
use tracing::{info, error};

use crate::{
    trading::TradingEngineHandle,
    wallet::WalletManager,
    errors::Result,
    utils::validation::{Validator, ValidatedUserId},
};

/// Handler for wallet-related operations
pub struct WalletHandler;

impl WalletHandler {
    /// Handle balance callback from inline keyboard
    pub async fn handle_balance_callback(
        bot: &Bot,
        q: &CallbackQuery,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        if let Some(msg) = &q.message {
            let user_id_str = q.from.id.0.to_string();
            let user_id = match ValidatedUserId::new(&user_id_str) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid user ID {}: {}", user_id_str, e);
                    bot.send_message(msg.chat.id, "❌ Invalid user session")
                        .await?;
                    return Ok(());
                }
            };
            Self::show_balance(bot.clone(), msg.chat.id, user_id.as_str(), trading_engine, wallet_manager).await?;
        }
        Ok(())
    }
    
    /// Handle deposit callback
    pub async fn handle_deposit_callback(
        bot: &Bot,
        q: &CallbackQuery,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        if let Some(msg) = &q.message {
            let user_id_str = q.from.id.0.to_string();
            let user_id = match ValidatedUserId::new(&user_id_str) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid user ID {}: {}", user_id_str, e);
                    bot.send_message(msg.chat.id, "❌ Invalid user session")
                        .await?;
                    return Ok(());
                }
            };
            Self::show_deposit_info(bot.clone(), msg.chat.id, user_id.as_str(), wallet_manager).await?;
        }
        Ok(())
    }
    
    /// Handle new wallet callback
    pub async fn handle_new_wallet_callback(
        bot: &Bot,
        q: &CallbackQuery,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        if let Some(msg) = &q.message {
            let user_id_str = q.from.id.0.to_string();
            let user_id = match ValidatedUserId::new(&user_id_str) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid user ID {}: {}", user_id_str, e);
                    bot.send_message(msg.chat.id, "❌ Invalid user session")
                        .await?;
                    return Ok(());
                }
            };
            Self::create_new_wallet(bot.clone(), msg.chat.id, user_id.as_str(), wallet_manager).await?;
        }
        Ok(())
    }
    
    /// Handle export callback
    pub async fn handle_export_callback(
        bot: &Bot,
        q: &CallbackQuery,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        if let Some(msg) = &q.message {
            let user_id_str = q.from.id.0.to_string();
            let user_id = match ValidatedUserId::new(&user_id_str) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid user ID {}: {}", user_id_str, e);
                    bot.send_message(msg.chat.id, "❌ Invalid user session")
                        .await?;
                    return Ok(());
                }
            };
            Self::export_wallet_keys(bot.clone(), msg.chat.id, user_id.as_str(), wallet_manager).await?;
        }
        Ok(())
    }
    
    /// Handle backup callback
    pub async fn handle_backup_callback(bot: &Bot, q: &CallbackQuery) -> ResponseResult<()> {
        if let Some(msg) = &q.message {
            Self::show_backup_guide(bot.clone(), msg.chat.id).await?;
        }
        Ok(())
    }
    
    /// Show wallet balance
    async fn show_balance(
        bot: Bot,
        chat_id: teloxide::types::ChatId,
        user_id: &str,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        // Check if user has a wallet configured
        let user_wallet = match wallet_manager.get_user_wallet(user_id).await {
            Ok(Some(wallet)) => wallet.public_key,
            Ok(None) => {
                bot.send_message(chat_id, 
                    "❌ No wallet configured\\. Please use /start to set up your wallet first\\.")
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
                return Ok(());
            }
            Err(e) => {
                error!("Failed to get user wallet: {}", e);
                bot.send_message(chat_id, "❌ Error accessing wallet")
                    .await?;
                return Ok(());
            }
        };
        
        match trading_engine.get_balance(user_wallet.clone()).await {
            Ok(balance) => {
                let message = format!(
                    "💰 *Wallet Balance*\\n\\n\
                    SOL: {:.4}\\n\
                    USDC: {:.2}\\n\\n\
                    Total Value: ${:.2}\\n\\n\
                    _Last updated: {}_",
                    balance.sol,
                    balance.usdc,
                    balance.total_usd_value,
                    chrono::Utc::now().format("%H:%M:%S UTC")
                );
                
                bot.send_message(chat_id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Err(e) => {
                error!("Failed to get balance: {}", e);
                bot.send_message(chat_id, "❌ Failed to fetch balance")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Show deposit information
    async fn show_deposit_info(
        bot: Bot,
        chat_id: teloxide::types::ChatId,
        user_id: &str,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        match wallet_manager.get_user_wallet(user_id).await {
            Ok(Some(wallet)) => {
                let message = format!(
                    "📥 *Deposit to Your Wallet*\\n\\n\
                    📍 *Your Wallet Address:*\\n\
                    `{}`\\n\\n\
                    🔗 [View on Solscan](https://solscan\\.io/account/{})\\n\\n\
                    ⚠️ *Important:*\\n\
                    • Only send Solana \\(SOL\\) or SPL tokens\\n\
                    • Double\\-check the address\\n\
                    • Tokens arrive instantly\\n\
                    • Network fees apply",
                    wallet.public_key,
                    wallet.public_key
                );
                
                bot.send_message(chat_id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Ok(None) => {
                bot.send_message(chat_id, 
                    "❌ No wallet found\\. Use /start to create a wallet first\\.")
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Err(e) => {
                error!("Failed to get wallet: {}", e);
                bot.send_message(chat_id, "❌ Error accessing wallet")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Create a new wallet
    async fn create_new_wallet(
        bot: Bot,
        chat_id: teloxide::types::ChatId,
        user_id: &str,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        // Show security warning first
        let warning = r#"⚠️ *SECURITY WARNING*

You are about to create a new wallet\\. Please read carefully:

🔐 *Non\\-Custodial Wallet:*
• You own and control your private keys
• We never see or store your private keys
• If you lose your keys, we CANNOT recover them
• Your funds are entirely under your control

📝 *You Must Save:*
• Your 12\\-word recovery phrase
• Your private key
• Store them safely offline

*Are you ready to create a secure wallet?*

Type `/confirm` to proceed or `/cancel` to abort\\."#;
        
        bot.send_message(chat_id, warning)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        
        Ok(())
    }
    
    /// Export wallet keys
    async fn export_wallet_keys(
        bot: Bot,
        chat_id: teloxide::types::ChatId,
        user_id: &str,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        match wallet_manager.export_user_wallet(user_id).await {
            Ok(Some(wallet_data)) => {
                let message = format!(
                    "🔑 *Wallet Export*\\n\\n\
                    📍 *Address:*\\n`{}`\\n\\n\
                    🔑 *Private Key:*\\n||{}||\\n\\n\
                    🔗 [View on Solscan](https://solscan\\.io/account/{})\\n\\n\
                    ⚠️ *CRITICAL SECURITY WARNING:*\\n\
                    • Never share this private key\\n\
                    • Delete this message after saving\\n\
                    • Anyone with this key controls your funds",
                    wallet_data.address,
                    wallet_data.private_key,
                    wallet_data.address
                );
                
                bot.send_message(chat_id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Ok(None) => {
                bot.send_message(chat_id, 
                    "❌ No wallet found\\. Use /start to create one first\\.")
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Err(e) => {
                error!("Failed to export wallet: {}", e);
                bot.send_message(chat_id, "❌ Error exporting wallet")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Show backup guide
    async fn show_backup_guide(
        bot: Bot,
        chat_id: teloxide::types::ChatId,
    ) -> ResponseResult<()> {
        let guide = r#"💾 *Wallet Backup Guide*

🔐 *What to Backup:*
• Your 12\\-word mnemonic phrase
• Your private key \\(from /export\\)

📝 *How to Backup:*
1\\. Write down your mnemonic on paper
2\\. Store in multiple secure locations
3\\. Never save digitally \\(photos, cloud, etc\\.\\)
4\\. Test recovery before using

❌ *Never Share:*
• Your mnemonic phrase
• Your private key
• Screenshots of sensitive data

✅ *Safe Storage Options:*
• Hardware wallet \\(Ledger, Trezor\\)
• Metal seed phrase backup
• Safe deposit box
• Multiple paper copies

⚠️ *Remember:* If you lose your backup, you lose your funds\\!
There is NO way to recover a lost wallet\\."#;
        
        bot.send_message(chat_id, guide)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        
        Ok(())
    }
}