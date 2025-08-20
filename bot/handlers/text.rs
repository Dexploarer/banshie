use teloxide::{prelude::*, types::Message};
use std::sync::Arc;
use tracing::error;

use crate::{
    trading::TradingEngineHandle,
    ai::GroqAnalyzer,
    db::Database,
    utils::Config,
    wallet::WalletManager,
    errors::Result,
};
use super::{menu::*, trading::TradingHandler, wallet::WalletHandler};

/// Handler for text messages (keyboard button presses)
pub struct TextMessageHandler;

impl TextMessageHandler {
    /// Handle text messages from the main menu keyboard
    pub async fn handle(
        bot: Bot,
        msg: Message,
        trading_engine: TradingEngineHandle,
        ai_analyzer: Arc<GroqAnalyzer>,
        db: Arc<Database>,
        config: Arc<Config>,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        let user_id = msg.from()
            .map(|u| u.id.0.to_string())
            .unwrap_or_default();
        
        if !config.is_user_allowed(&user_id) {
            return Ok(());
        }
        
        if let Some(text) = msg.text() {
            match text {
                "💰 Balance" => {
                    Self::handle_balance_button(bot, msg, trading_engine, wallet_manager, user_id).await?;
                }
                "📊 Portfolio" => {
                    Self::handle_portfolio_button(bot, msg).await?;
                }
                "⚡ Trade" => {
                    Self::handle_trade_button(bot, msg).await?;
                }
                "💎 Rebates" => {
                    Self::handle_rebates_button(bot, msg, db, user_id).await?;
                }
                "🤖 AI Analysis" => {
                    Self::handle_ai_analysis_button(bot, msg).await?;
                }
                "💼 Wallet" => {
                    Self::handle_wallet_button(bot, msg).await?;
                }
                "⚙️ Settings" => {
                    Self::handle_settings_button(bot, msg).await?;
                }
                "📚 Help" => {
                    Self::handle_help_button(bot, msg).await?;
                }
                "📈 Charts" => {
                    Self::handle_charts_button(bot, msg).await?;
                }
                _ => {
                    Self::handle_unknown_text(bot, msg, text).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle balance button press
    async fn handle_balance_button(
        bot: Bot,
        msg: Message,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
        user_id: String,
    ) -> ResponseResult<()> {
        // Check if user has a wallet configured
        let user_wallet = match wallet_manager.get_user_wallet(&user_id).await {
            Ok(Some(wallet)) => wallet.public_key,
            Ok(None) => {
                bot.send_message(msg.chat.id, 
                    "❌ No wallet configured. Please use /start to set up your wallet first.")
                    .await?;
                return Ok(());
            }
            Err(e) => {
                error!("Failed to get user wallet: {}", e);
                bot.send_message(msg.chat.id, "❌ Error accessing wallet")
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
                
                bot.send_message(msg.chat.id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Err(e) => {
                error!("Failed to get balance: {}", e);
                bot.send_message(msg.chat.id, "❌ Failed to fetch balance")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle portfolio button press
    async fn handle_portfolio_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "📊 *Portfolio Menu*\\n\\nChoose an option:")
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(create_portfolio_menu())
            .await?;
        Ok(())
    }
    
    /// Handle trade button press
    async fn handle_trade_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "⚡ *Trading Menu*\\n\\nChoose your trading action:")
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(create_trading_menu())
            .await?;
        Ok(())
    }
    
    /// Handle rebates button press
    async fn handle_rebates_button(bot: Bot, msg: Message, db: Arc<Database>, user_id: String) -> ResponseResult<()> {
        match db.get_user_rebates(&user_id).await {
            Ok(rebates) => {
                let message = format!(
                    "💎 *MEV Rebates Earned*\\n\\n\
                    Today: {:.6} SOL\\n\
                    This Week: {:.6} SOL\\n\
                    This Month: {:.6} SOL\\n\
                    All Time: {:.6} SOL\\n\\n\
                    _Rebates are paid instantly with each trade\\!_",
                    rebates.today,
                    rebates.week,
                    rebates.month,
                    rebates.all_time
                );
                
                bot.send_message(msg.chat.id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Err(e) => {
                error!("Failed to get rebates: {}", e);
                bot.send_message(msg.chat.id, "❌ Failed to fetch rebate information")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle AI analysis button press
    async fn handle_ai_analysis_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "🤖 *AI Analysis Menu*\\n\\nWhat would you like to analyze?")
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(create_analytics_menu())
            .await?;
        Ok(())
    }
    
    /// Handle wallet button press
    async fn handle_wallet_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "💼 *Wallet Menu*\\n\\nManage your wallets:")
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(create_wallet_menu())
            .await?;
        Ok(())
    }
    
    /// Handle settings button press
    async fn handle_settings_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "⚙️ *Settings Menu*\\n\\nCustomize your bot:")
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(create_settings_menu())
            .await?;
        Ok(())
    }
    
    /// Handle help button press
    async fn handle_help_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        let help_text = r#"📚 *Solana Trading Bot Help*

*Main Features:*
• 💰 Balance \\- Check your wallet balance
• 📊 Portfolio \\- View positions and P&L
• ⚡ Trade \\- Buy/sell tokens instantly
• 💎 Rebates \\- Track MEV rebate earnings
• 🤖 AI Analysis \\- Get market insights
• 💼 Wallet \\- Manage your wallets
• ⚙️ Settings \\- Configure the bot

*Commands:*
/start \\- Initialize the bot
/balance \\- Quick balance check
/buy <token> <amount> \\- Buy tokens
/sell <token> <percentage> \\- Sell tokens
/portfolio \\- View portfolio
/analyze <token> \\- AI analysis
/rebates \\- View rebate stats
/help \\- Show this help

*Security:*
• 🔒 Non\\-custodial \\(you control keys\\)
• 🛡️ MEV protection enabled
• 💎 Instant rebate payments
• 🔐 Private key never stored

*Support:*
Contact @support for help\\."#;
        
        bot.send_message(msg.chat.id, help_text)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        Ok(())
    }
    
    /// Handle charts button press
    async fn handle_charts_button(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "📈 Coming soon! Chart integration with TradingView.")
            .await?;
        Ok(())
    }
    
    /// Handle unknown text input
    async fn handle_unknown_text(bot: Bot, msg: Message, text: &str) -> ResponseResult<()> {
        if text.starts_with('/') {
            // Command without proper parsing, ignore
            return Ok(());
        }
        
        bot.send_message(msg.chat.id, "❓ Unknown command. Use /help or the menu buttons below.")
            .reply_markup(create_main_menu())
            .await?;
        Ok(())
    }
}