use teloxide::{prelude::*, utils::command::BotCommands};
use std::sync::Arc;
use tracing::{info, error};

use crate::{
    trading::TradingEngineHandle,
    ai::GroqAnalyzer,
    db::Database,
    utils::Config,
    wallet::WalletManager,
    errors::Result,
};

use super::{
    commands::Command,
    handlers::{CommandHandler, TextMessageHandler, CallbackHandler},
};

/// Main Telegram bot struct
pub struct TelegramBot {
    config: Arc<Config>,
    trading_engine: TradingEngineHandle,
    ai_analyzer: Arc<GroqAnalyzer>,
    db: Arc<Database>,
    wallet_manager: Arc<WalletManager>,
}

impl TelegramBot {
    /// Create a new TelegramBot instance
    pub fn new(
        config: Arc<Config>,
        trading_engine: TradingEngineHandle,
        ai_analyzer: Arc<GroqAnalyzer>,
        db: Arc<Database>,
        wallet_manager: Arc<WalletManager>,
    ) -> Self {
        Self {
            config,
            trading_engine,
            ai_analyzer,
            db,
            wallet_manager,
        }
    }
    
    /// Run the bot dispatcher
    pub async fn run(&self) -> Result<()> {
        let bot = Bot::new(&self.config.telegram_bot_token);
        
        info!("🤖 Starting Telegram bot...");
        
        let handler = dptree::entry()
            .branch(Update::filter_message()
                .filter_command::<Command>()
                .endpoint(Self::handle_command))
            .branch(Update::filter_message()
                .endpoint(TextMessageHandler::handle))
            .branch(Update::filter_callback_query()
                .endpoint(CallbackHandler::handle));
        
        Dispatcher::builder(bot.clone(), handler)
            .dependencies(dptree::deps![
                self.trading_engine.clone(),
                self.ai_analyzer.clone(),
                self.db.clone(),
                self.config.clone(),
                self.wallet_manager.clone()
            ])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
            
        Ok(())
    }
    
    /// Handle bot commands by delegating to CommandHandler
    async fn handle_command(
        bot: Bot,
        msg: Message,
        cmd: Command,
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
            bot.send_message(msg.chat.id, "⛔ Unauthorized access")
                .await?;
            return Ok(());
        }
        
        info!("Processing command {:?} from user {}", cmd, user_id);
        
        match cmd {
            Command::Start => {
                CommandHandler::handle_start(bot, msg).await?;
            }
            Command::Balance => {
                CommandHandler::handle_balance(bot, msg, trading_engine, wallet_manager, user_id).await?;
            }
            Command::Buy(args) => {
                CommandHandler::handle_buy(bot, msg, args, trading_engine, db, wallet_manager, user_id).await?;
            }
            Command::Sell(args) => {
                CommandHandler::handle_sell(bot, msg, args, trading_engine, db, wallet_manager, user_id).await?;
            }
            Command::Portfolio => {
                CommandHandler::handle_portfolio(bot, msg, trading_engine, wallet_manager, user_id).await?;
            }
            Command::Analyze(token) => {
                CommandHandler::handle_analyze(bot, msg, token, ai_analyzer).await?;
            }
            Command::Rebates => {
                CommandHandler::handle_rebates(bot, msg, db, user_id).await?;
            }
            Command::Settings => {
                CommandHandler::handle_settings(bot, msg).await?;
            }
            Command::Help => {
                CommandHandler::handle_help(bot, msg).await?;
            }
            Command::Deposit => {
                CommandHandler::handle_deposit(bot, msg, wallet_manager, user_id).await?;
            }
            Command::Export => {
                CommandHandler::handle_export(bot, msg, wallet_manager, user_id).await?;
            }
            Command::Backup => {
                CommandHandler::handle_backup(bot, msg).await?;
            }
            Command::Confirm => {
                CommandHandler::handle_confirm(bot, msg).await?;
            }
            Command::Cancel => {
                CommandHandler::handle_cancel(bot, msg).await?;
            }
            // MVP Trading Commands
            Command::Snipe(args) => {
                CommandHandler::handle_snipe(bot, msg, args, trading_engine, db, wallet_manager, user_id).await?;
            }
            Command::Copy(args) => {
                CommandHandler::handle_copy(bot, msg, args, db, user_id, trading_engine.clone(), wallet_manager.clone()).await?;
            }
            Command::Unfollow(args) => {
                CommandHandler::handle_unfollow(bot, msg, args, db, user_id).await?;
            }
            Command::Larp(args) => {
                CommandHandler::handle_larp(bot, msg, args, ai_analyzer).await?;
            }
            Command::Trending => {
                CommandHandler::handle_trending(bot, msg, ai_analyzer).await?;
            }
            Command::Launch => {
                CommandHandler::handle_launch(bot, msg, trading_engine, user_id).await?;
            }
            Command::Blink(args) => {
                CommandHandler::handle_blink(bot, msg, args, trading_engine, user_id).await?;
            }
            Command::Alert(args) => {
                CommandHandler::handle_alert(bot, msg, args, db, user_id).await?;
            }
            Command::Leaderboard => {
                CommandHandler::handle_leaderboard(bot, msg, db).await?;
            }
            Command::Signals => {
                CommandHandler::handle_signals(bot, msg, ai_analyzer).await?;
            }
            Command::Pump(args) => {
                CommandHandler::handle_pump(bot, msg, args, trading_engine, user_id).await?;
            }
            Command::QuickBuy(args) => {
                CommandHandler::handle_quick_buy(bot, msg, args, trading_engine, wallet_manager, user_id).await?;
            }
            Command::QuickSell(args) => {
                CommandHandler::handle_quick_sell(bot, msg, args, trading_engine, wallet_manager, user_id).await?;
            }
            Command::StopLoss(args) => {
                CommandHandler::handle_stop_loss(bot, msg, args, db, user_id).await?;
            }
            // Legacy commands - redirect to menu
            Command::Wallet => {
                bot.send_message(msg.chat.id, "💼 Use the Wallet button in the main menu instead!")
                    .await?;
            }
            Command::NewWallet => {
                bot.send_message(msg.chat.id, "🆕 Use the 💼 Wallet → 🆕 New Wallet buttons instead!")
                    .await?;
            }
            Command::Import => {
                bot.send_message(msg.chat.id, "📥 Use the 💼 Wallet → 📥 Import Wallet buttons instead!")
                    .await?;
            }
        }
        
        Ok(())
    }
}