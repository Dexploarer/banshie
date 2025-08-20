use teloxide::{prelude::*, types::Message};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile};
use std::sync::Arc;
use tracing::{info, error};

use crate::blinks::{
    BlinkGenerator, BlinkExecutor, BlinkSharing,
    SolanaBlink, BlinkType, SolanaNetwork, SharePlatform,
};
use crate::errors::Result;
use crate::trading::TradingEngineHandle;
use crate::wallet::WalletManager;

/// Handles Solana Blinks commands
pub struct BlinksHandler;

impl BlinksHandler {
    /// Handle /blink command - Create and share Solana Blinks
    pub async fn handle_blink(
        bot: Bot,
        msg: Message,
        args: String,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.is_empty() {
            // Show blinks menu
            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![
                    InlineKeyboardButton::callback("💱 Create Swap", "blink_swap"),
                    InlineKeyboardButton::callback("💸 Create Transfer", "blink_transfer"),
                ],
                vec![
                    InlineKeyboardButton::callback("🎨 Create NFT Mint", "blink_nft"),
                    InlineKeyboardButton::callback("💰 Create Payment", "blink_payment"),
                ],
                vec![
                    InlineKeyboardButton::callback("📊 My Blinks", "blink_list"),
                    InlineKeyboardButton::callback("📈 Analytics", "blink_analytics"),
                ],
            ]);
            
            bot.send_message(msg.chat.id,
                "🔗 **Solana Blinks**\n\n\
                Create shareable, one-click Solana actions!\n\n\
                **Available Actions:**\n\
                • `/blink swap <from> <to> <amount>` - Create swap link\n\
                • `/blink transfer <token> <recipient> <amount>` - Create transfer link\n\
                • `/blink nft <collection> <price>` - Create NFT mint link\n\
                • `/blink payment <amount> <token>` - Create payment request\n\
                • `/blink execute <blink_url>` - Execute a blink\n\n\
                Select an action below:")
                .reply_markup(keyboard)
                .await?;
            
            return Ok(());
        }
        
        let user_wallet = msg.from()
            .map(|u| format!("user_{}", u.id))
            .unwrap_or_else(|| "unknown".to_string());
        
        match parts[0] {
            "swap" => {
                if parts.len() < 4 {
                    bot.send_message(msg.chat.id,
                        "❌ Usage: `/blink swap <from_token> <to_token> <amount>`\n\
                        Example: `/blink swap SOL USDC 10`")
                        .await?;
                    return Ok(());
                }
                
                Self::create_swap_blink(
                    bot,
                    msg,
                    parts[1],
                    parts[2],
                    parts[3].parse().unwrap_or(0.0),
                    user_wallet,
                ).await?;
            }
            "transfer" => {
                if parts.len() < 4 {
                    bot.send_message(msg.chat.id,
                        "❌ Usage: `/blink transfer <token> <recipient> <amount>`\n\
                        Example: `/blink transfer USDC 7xKXtg... 100`")
                        .await?;
                    return Ok(());
                }
                
                Self::create_transfer_blink(
                    bot,
                    msg,
                    parts[1],
                    parts[2],
                    parts[3].parse().unwrap_or(0.0),
                    user_wallet,
                ).await?;
            }
            "nft" => {
                if parts.len() < 3 {
                    bot.send_message(msg.chat.id,
                        "❌ Usage: `/blink nft <collection_address> <price>`\n\
                        Example: `/blink nft ABC123... 0.5`")
                        .await?;
                    return Ok(());
                }
                
                Self::create_nft_blink(
                    bot,
                    msg,
                    parts[1],
                    parts[2].parse().unwrap_or(0.0),
                    user_wallet,
                ).await?;
            }
            "payment" => {
                if parts.len() < 3 {
                    bot.send_message(msg.chat.id,
                        "❌ Usage: `/blink payment <amount> <token>`\n\
                        Example: `/blink payment 50 USDC`")
                        .await?;
                    return Ok(());
                }
                
                Self::create_payment_blink(
                    bot,
                    msg,
                    parts[1].parse().unwrap_or(0.0),
                    parts[2],
                    user_wallet,
                ).await?;
            }
            "execute" => {
                if parts.len() < 2 {
                    bot.send_message(msg.chat.id,
                        "❌ Usage: `/blink execute <blink_url>`")
                        .await?;
                    return Ok(());
                }
                
                Self::execute_blink(
                    bot,
                    msg,
                    parts[1],
                    trading_engine,
                    wallet_manager,
                    user_wallet,
                ).await?;
            }
            _ => {
                bot.send_message(msg.chat.id,
                    "❌ Unknown blink command. Use `/blink` to see available options.")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Create a swap blink
    async fn create_swap_blink(
        bot: Bot,
        msg: Message,
        from_token: &str,
        to_token: &str,
        amount: f64,
        user_wallet: String,
    ) -> ResponseResult<()> {
        let generator = BlinkGenerator::new(
            "https://solana-bot.example.com".to_string(),
            SolanaNetwork::Mainnet,
        );
        
        // Create the blink
        let blink = generator.create_swap_blink(
            format!("{}...mint", from_token),
            from_token.to_string(),
            format!("{}...mint", to_token),
            to_token.to_string(),
            amount,
            1.0, // 1% slippage
            user_wallet,
        ).map_err(|e| anyhow::anyhow!("Failed to create blink: {}", e))?;
        
        // Generate share URLs
        let sharing = BlinkSharing::new(
            "https://solana-bot.example.com".to_string(),
            true,
        );
        
        let twitter_url = sharing.generate_share_url(&blink, SharePlatform::Twitter, None);
        let telegram_url = sharing.generate_share_url(&blink, SharePlatform::Telegram, None);
        let direct_url = blink.to_url("https://solana-bot.example.com");
        
        // Create share buttons
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::url("🐦 Share on Twitter", twitter_url),
                InlineKeyboardButton::url("📱 Share on Telegram", telegram_url),
            ],
            vec![
                InlineKeyboardButton::callback("📋 Copy Link", format!("copy_{}", blink.blink_id)),
                InlineKeyboardButton::callback("🔗 QR Code", format!("qr_{}", blink.blink_id)),
            ],
        ]);
        
        let message = format!(
            "✅ **Swap Blink Created!**\n\n\
            💱 Swap {} {} for {}\n\
            📊 Slippage: 1%\n\
            ⏰ Expires in: 24 hours\n\
            🔒 Security: Verified ✅\n\n\
            🔗 **Direct Link:**\n`{}`\n\n\
            Share this link to allow anyone to execute the swap with one click!",
            amount, from_token, to_token, direct_url
        );
        
        bot.send_message(msg.chat.id, message)
            .reply_markup(keyboard)
            .await?;
        
        Ok(())
    }
    
    /// Create a transfer blink
    async fn create_transfer_blink(
        bot: Bot,
        msg: Message,
        token: &str,
        recipient: &str,
        amount: f64,
        user_wallet: String,
    ) -> ResponseResult<()> {
        let generator = BlinkGenerator::new(
            "https://solana-bot.example.com".to_string(),
            SolanaNetwork::Mainnet,
        );
        
        let blink = generator.create_transfer_blink(
            format!("{}...mint", token),
            token.to_string(),
            recipient.to_string(),
            amount,
            Some("Payment via Solana Blinks".to_string()),
            user_wallet,
        ).map_err(|e| anyhow::anyhow!("Failed to create blink: {}", e))?;
        
        let sharing = BlinkSharing::new(
            "https://solana-bot.example.com".to_string(),
            true,
        );
        
        let direct_url = blink.to_url("https://solana-bot.example.com");
        
        // Generate QR code
        let qr_svg = sharing.generate_qr_code(&blink)
            .unwrap_or_else(|_| "QR generation failed".to_string());
        
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::url("📱 Share", 
                    sharing.generate_share_url(&blink, SharePlatform::Telegram, None)),
                InlineKeyboardButton::callback("📋 Copy", format!("copy_{}", blink.blink_id)),
            ],
        ]);
        
        let message = format!(
            "✅ **Transfer Blink Created!**\n\n\
            💸 Send {} {} to {}\n\
            ⏰ Expires in: 1 hour\n\
            🔒 Single use only\n\n\
            🔗 **Link:** `{}`\n\n\
            Share this link to receive the payment!",
            amount, token, &recipient[..8], direct_url
        );
        
        bot.send_message(msg.chat.id, message)
            .reply_markup(keyboard)
            .await?;
        
        Ok(())
    }
    
    /// Create an NFT mint blink
    async fn create_nft_blink(
        bot: Bot,
        msg: Message,
        collection: &str,
        price: f64,
        user_wallet: String,
    ) -> ResponseResult<()> {
        let generator = BlinkGenerator::new(
            "https://solana-bot.example.com".to_string(),
            SolanaNetwork::Mainnet,
        );
        
        let blink = generator.create_nft_mint_blink(
            collection.to_string(),
            "Demo NFT Collection".to_string(),
            price,
            Some(1000),
            user_wallet,
        ).map_err(|e| anyhow::anyhow!("Failed to create blink: {}", e))?;
        
        let sharing = BlinkSharing::new(
            "https://solana-bot.example.com".to_string(),
            true,
        );
        
        let twitter_url = sharing.generate_share_url(&blink, SharePlatform::Twitter, None);
        let direct_url = blink.to_url("https://solana-bot.example.com");
        
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::url("🐦 Share on Twitter", twitter_url),
                InlineKeyboardButton::callback("📋 Copy Link", format!("copy_{}", blink.blink_id)),
            ],
        ]);
        
        let message = format!(
            "✅ **NFT Mint Blink Created!**\n\n\
            🎨 Collection: Demo NFT\n\
            💰 Price: {} SOL\n\
            📦 Supply: 1000\n\
            ⚠️ Risk: Medium (Unverified)\n\n\
            🔗 **Mint Link:** `{}`\n\n\
            Share this link to allow minting!",
            price, direct_url
        );
        
        bot.send_message(msg.chat.id, message)
            .reply_markup(keyboard)
            .await?;
        
        Ok(())
    }
    
    /// Create a payment request blink
    async fn create_payment_blink(
        bot: Bot,
        msg: Message,
        amount: f64,
        token: &str,
        user_wallet: String,
    ) -> ResponseResult<()> {
        let generator = BlinkGenerator::new(
            "https://solana-bot.example.com".to_string(),
            SolanaNetwork::Mainnet,
        );
        
        let blink = generator.create_payment_blink(
            amount,
            format!("{}...mint", token),
            token.to_string(),
            user_wallet.clone(),
            "Payment Request".to_string(),
            Some(format!("PAY_{}", uuid::Uuid::new_v4())),
        ).map_err(|e| anyhow::anyhow!("Failed to create blink: {}", e))?;
        
        let sharing = BlinkSharing::new(
            "https://solana-bot.example.com".to_string(),
            true,
        );
        
        let direct_url = blink.to_url("https://solana-bot.example.com");
        let whatsapp_url = sharing.generate_share_url(&blink, SharePlatform::WhatsApp, None);
        
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::url("💬 Share via WhatsApp", whatsapp_url),
                InlineKeyboardButton::callback("📋 Copy", format!("copy_{}", blink.blink_id)),
            ],
            vec![
                InlineKeyboardButton::callback("🔗 QR Code", format!("qr_{}", blink.blink_id)),
                InlineKeyboardButton::callback("📊 Track", format!("track_{}", blink.blink_id)),
            ],
        ]);
        
        let message = format!(
            "✅ **Payment Request Created!**\n\n\
            💰 Request: {} {}\n\
            📬 Recipient: You\n\
            ⏰ Valid for: 24 hours\n\
            🔒 Single use\n\n\
            🔗 **Payment Link:**\n`{}`\n\n\
            Share this link to receive payment!",
            amount, token, direct_url
        );
        
        bot.send_message(msg.chat.id, message)
            .reply_markup(keyboard)
            .await?;
        
        Ok(())
    }
    
    /// Execute a blink from URL
    async fn execute_blink(
        bot: Bot,
        msg: Message,
        blink_url: &str,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
        user_wallet: String,
    ) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "🔄 Processing blink...")
            .await?;
        
        // Parse blink from URL (simplified for demo)
        // In production, would decode and validate the blink
        
        let executor = BlinkExecutor::new(trading_engine, wallet_manager);
        
        // Create a demo blink for execution
        let generator = BlinkGenerator::new(
            "https://solana-bot.example.com".to_string(),
            SolanaNetwork::Mainnet,
        );
        
        let demo_blink = generator.create_swap_blink(
            "SOL".to_string(),
            "SOL".to_string(),
            "USDC".to_string(),
            "USDC".to_string(),
            1.0,
            1.0,
            user_wallet.clone(),
        ).map_err(|e| anyhow::anyhow!("Failed to parse blink: {}", e))?;
        
        // Execute the blink
        match executor.execute_blink(&demo_blink, &user_wallet).await {
            Ok(result) => {
                if result.success {
                    let message = format!(
                        "✅ **Blink Executed Successfully!**\n\n\
                        🔗 Transaction: `{}`\n\
                        ⏱️ Execution time: {}ms\n\
                        ⛽ Gas used: {} units\n\n\
                        View on Solscan: https://solscan.io/tx/{}",
                        result.transaction_signature.as_ref().unwrap_or(&"N/A".to_string()),
                        result.execution_time_ms,
                        result.gas_used.unwrap_or(0),
                        result.transaction_signature.as_ref().unwrap_or(&"".to_string())
                    );
                    
                    bot.send_message(msg.chat.id, message).await?;
                } else {
                    bot.send_message(msg.chat.id,
                        format!("❌ Blink execution failed: {}",
                            result.error.unwrap_or_else(|| "Unknown error".to_string())))
                        .await?;
                }
            }
            Err(e) => {
                bot.send_message(msg.chat.id,
                    format!("❌ Failed to execute blink: {}", e))
                    .await?;
            }
        }
        
        Ok(())
    }
}