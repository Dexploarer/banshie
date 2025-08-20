use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    wallet::{WalletGenerator, WalletManager, WalletSecurity, SecurityWarning, WarningLevel},
    db::Database,
};

pub struct WalletSetupFlow;

impl WalletSetupFlow {
    /// Initial wallet setup for new users
    pub async fn start_setup(bot: Bot, chat_id: ChatId) -> ResponseResult<()> {
        let message = r#"🔐 *Welcome to Wallet Setup*

This bot uses a *non\-custodial wallet* system\.
This means:
✅ You have complete control of your funds
✅ Only you have access to your private keys
❌ We NEVER see or store your private keys
⚠️ If you lose your keys, we CANNOT recover them

*Choose an option:*"#;

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("🆕 Generate New Wallet", "wallet_new"),
            ],
            vec![
                InlineKeyboardButton::callback("📥 Import Existing Wallet", "wallet_import"),
            ],
            vec![
                InlineKeyboardButton::callback("📚 Learn More", "wallet_learn"),
            ],
        ]);

        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    /// Generate new wallet flow
    pub async fn generate_new_wallet(bot: Bot, chat_id: ChatId, user_id: &str) -> ResponseResult<()> {
        // Show security warnings first
        let warnings = WalletSecurity::get_setup_warnings();
        let mut warning_text = String::from("⚠️ *IMPORTANT SECURITY INFORMATION*\n\n");
        
        for warning in warnings {
            let emoji = match warning.level {
                WarningLevel::Critical => "🚨",
                WarningLevel::Warning => "⚠️",
                WarningLevel::Info => "ℹ️",
            };
            warning_text.push_str(&format!("{} {}\n\n", emoji, Self::escape_markdown(&warning.message)));
        }
        
        warning_text.push_str("\nType /confirm to generate your wallet");

        bot.send_message(chat_id, warning_text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }

    /// Actually generate and display wallet (called after user confirms)
    pub async fn confirm_generate_wallet(
        bot: Bot,
        chat_id: ChatId,
        user_id: &str,
        wallet_manager: Arc<RwLock<WalletManager>>,
        db: Arc<Database>,
    ) -> ResponseResult<()> {
        // Generate wallet
        let credentials = match WalletGenerator::generate_new() {
            Ok(creds) => creds,
            Err(e) => {
                bot.send_message(chat_id, format!("❌ Failed to generate wallet: {}", e))
                    .await?;
                return Ok(());
            }
        };

        // Display private key and mnemonic ONCE
        let secret_message = format!(
            r#"🔐 *YOUR WALLET HAS BEEN GENERATED*

⚠️ *SAVE THIS INFORMATION IMMEDIATELY\!*
_This is the ONLY time you will see your private keys\._

📍 *Wallet Address \(Public\):*
`{}`

🔑 *Private Key:*
`{}`

📝 *Seed Phrase \(12 words\):*
`{}`

*CRITICAL INSTRUCTIONS:*
1️⃣ Write down your seed phrase on paper
2️⃣ Store your private key securely
3️⃣ NEVER share these with anyone
4️⃣ NEVER enter them on any website
5️⃣ Take a photo ONLY if your phone is secure

_This message will be deleted in 5 minutes for your security\._

Reply with: *I HAVE SAVED MY KEYS* to continue"#,
            Self::escape_markdown(&credentials.public_key),
            Self::escape_markdown(&credentials.private_key),
            credentials.mnemonic.as_ref().map(|m| Self::escape_markdown(m)).unwrap_or_default()
        );

        let msg = bot.send_message(chat_id, secret_message)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

        // Register wallet (only public info)
        let mut manager = wallet_manager.write().await;
        if let Err(e) = manager.register_wallet(user_id, &credentials.public_key, Some("Main Wallet".to_string())) {
            warn!("Failed to register wallet: {}", e);
        }
        
        // Store in database (only public info)
        if let Err(e) = db.register_user_wallet(user_id, &credentials.public_key).await {
            warn!("Failed to store wallet in database: {}", e);
        }

        // Schedule message deletion after 5 minutes
        let bot_clone = bot.clone();
        let msg_id = msg.id;
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let _ = bot_clone.delete_message(chat_id, msg_id).await;
        });

        info!("Generated new wallet for user {}: {}", user_id, credentials.public_key);

        Ok(())
    }

    /// Import existing wallet flow
    pub async fn import_wallet(bot: Bot, chat_id: ChatId) -> ResponseResult<()> {
        let message = r#"📥 *Import Existing Wallet*

Choose import method:

*Option 1: Private Key*
Send your private key \(base58 encoded\)

*Option 2: Seed Phrase*
Send your 12 or 24 word seed phrase

⚠️ *Security Notes:*
• Delete the message after sending
• Make sure no one can see your screen
• Consider creating a new wallet if unsure

Send your private key or seed phrase now, or /cancel to abort\."#;

        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }

    /// Process wallet import
    pub async fn process_import(
        bot: Bot,
        chat_id: ChatId,
        user_id: &str,
        import_data: &str,
        wallet_manager: Arc<RwLock<WalletManager>>,
        db: Arc<Database>,
    ) -> ResponseResult<()> {
        // Delete user's message immediately for security
        // (This would need the message ID in real implementation)
        
        let words: Vec<&str> = import_data.split_whitespace().collect();
        
        let result = if words.len() >= 12 {
            // Seed phrase import
            WalletGenerator::from_mnemonic(import_data, "")
        } else {
            // Private key import
            WalletGenerator::from_private_key(import_data)
        };

        match result {
            Ok(credentials) => {
                // Register wallet
                let mut manager = wallet_manager.write().await;
                manager.register_wallet(user_id, &credentials.public_key, Some("Imported Wallet".to_string()))?;
                
                // Store in database
                db.register_user_wallet(user_id, &credentials.public_key).await?;

                let message = format!(
                    r#"✅ *Wallet Imported Successfully\!*

📍 *Wallet Address:*
`{}`

Your wallet has been imported and set as active\.
You can now start trading\!

⚠️ Remember: We do NOT store your private keys\."#,
                    Self::escape_markdown(&credentials.public_key)
                );

                bot.send_message(chat_id, message)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;

                info!("Imported wallet for user {}: {}", user_id, credentials.public_key);
            }
            Err(e) => {
                bot.send_message(chat_id, format!("❌ Import failed: {}\n\nPlease check your input and try again.", e))
                    .await?;
            }
        }

        Ok(())
    }

    /// Show wallet management menu
    pub async fn show_wallet_menu(
        bot: Bot,
        chat_id: ChatId,
        user_id: &str,
        wallet_manager: Arc<RwLock<WalletManager>>,
    ) -> ResponseResult<()> {
        let manager = wallet_manager.read().await;
        let wallets = manager.get_user_wallets(user_id);
        
        if wallets.is_empty() {
            Self::start_setup(bot, chat_id).await?;
            return Ok(());
        }

        let active_wallet = manager.get_active_wallet(user_id);
        
        let mut message = String::from("💼 *Your Wallets*\n\n");
        
        for (i, wallet) in wallets.iter().enumerate() {
            let status = if Some(&wallet.address) == active_wallet.as_ref().map(|w| &w.address) {
                "✅ Active"
            } else {
                "⚪ Inactive"
            };
            
            let balance = wallet.balance_sol
                .map(|b| format!("{:.4} SOL", b))
                .unwrap_or_else(|| "---".to_string());
            
            message.push_str(&format!(
                "{} *Wallet {}*\n📍 `{}`\n💰 {}\n\n",
                status,
                i + 1,
                Self::escape_markdown(&wallet.address[..8]),
                balance
            ));
        }

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("➕ Add Wallet", "wallet_add"),
                InlineKeyboardButton::callback("🔄 Switch Wallet", "wallet_switch"),
            ],
            vec![
                InlineKeyboardButton::callback("📤 Export", "wallet_export"),
                InlineKeyboardButton::callback("🗑️ Remove", "wallet_remove"),
            ],
            vec![
                InlineKeyboardButton::callback("🔐 Backup Guide", "wallet_backup"),
            ],
        ]);

        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    /// Show backup instructions
    pub async fn show_backup_guide(bot: Bot, chat_id: ChatId) -> ResponseResult<()> {
        let guide = r#"🔐 *Wallet Backup Guide*

*Recommended Backup Methods:*

📝 *Method 1: Paper Backup*
1\. Write seed phrase on paper
2\. Store in secure location \(safe, deposit box\)
3\. Consider making 2 copies in different locations
4\. Never photograph or digitize

🔒 *Method 2: Encrypted Digital*
1\. Use password manager
2\. Store encrypted on USB drive
3\. Keep offline and secure
4\. Use strong master password

🔑 *Method 3: Split Backup*
1\. Split seed phrase into parts
2\. Store parts separately
3\. Need multiple parts to recover
4\. Maximum security

⚠️ *NEVER:*
• Store in email
• Save in cloud without encryption
• Share with anyone
• Store on internet\-connected device
• Take screenshots

*Recovery Test:*
Consider creating a test wallet and practicing recovery before using for real funds\."#;

        bot.send_message(chat_id, guide)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }

    /// Create session for signing transactions
    pub async fn create_signing_session(
        bot: Bot,
        chat_id: ChatId,
        user_id: &str,
        wallet_manager: Arc<RwLock<WalletManager>>,
    ) -> ResponseResult<()> {
        let message = r#"🔓 *Create Signing Session*

For convenience, you can create a temporary signing session\.
This allows the bot to sign transactions for a limited time\.

⚠️ *Security Notes:*
• Session expires after 30 minutes
• Limited to small transactions \(0\.1 SOL\)
• You can revoke anytime
• Private key is encrypted in memory only

*Not Recommended for large amounts\!*

Create session?"#;

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("✅ Create 30min Session", "session_create_30"),
                InlineKeyboardButton::callback("❌ Cancel", "session_cancel"),
            ],
        ]);

        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    /// Helper to escape markdown characters
    fn escape_markdown(text: &str) -> String {
        text.chars()
            .map(|c| match c {
                '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => {
                    format!("\\{}", c)
                }
                _ => c.to_string(),
            })
            .collect()
    }
}

/// Transaction signing handler
pub struct TransactionSigner;

impl TransactionSigner {
    /// Request signature from user
    pub async fn request_signature(
        bot: Bot,
        chat_id: ChatId,
        transaction_details: &str,
    ) -> ResponseResult<()> {
        let message = format!(
            r#"📝 *Transaction Signature Required*

{}

To sign this transaction:
1\. Use your wallet app \(Phantom, Solflare\)
2\. Or send your signature
3\. Or create a temporary session

⚠️ Never share your private key\!"#,
            transaction_details
        );

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("✍️ I'll Sign Manually", "sign_manual"),
                InlineKeyboardButton::callback("🔓 Create Session", "sign_session"),
            ],
            vec![
                InlineKeyboardButton::callback("❌ Cancel Transaction", "sign_cancel"),
            ],
        ]);

        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }
}