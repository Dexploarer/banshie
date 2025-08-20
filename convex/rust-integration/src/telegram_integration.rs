use crate::convex_client::ConvexClient;
use anyhow::Result;
use serde_json::{json, Value};
use teloxide::{prelude::*, types::InlineKeyboardMarkup, utils::command::BotCommands};
use std::sync::Arc;

/// Telegram bot integration with Convex backend
#[derive(Clone)]
pub struct TelegramConvexBridge {
    convex: Arc<ConvexClient>,
    bot: Bot,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Solana Trading Bot Commands")]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Show portfolio overview")]
    Portfolio,
    #[command(description = "Quick trade interface")]
    Trade { token: Option<String> },
    #[command(description = "Manage DCA strategies")]
    Dca,
    #[command(description = "Price alerts")]
    Alerts,
    #[command(description = "AI trading signals")]
    Signals,
    #[command(description = "Connect wallet")]
    Wallet,
    #[command(description = "Get help")]
    Help,
}

impl TelegramConvexBridge {
    pub fn new(bot: Bot, convex: Arc<ConvexClient>) -> Self {
        Self { convex, bot }
    }

    /// Handle incoming messages
    pub async fn handle_message(&self, msg: Message) -> Result<()> {
        let user_id = msg.from().unwrap().id.0 as i64;
        let username = msg.from()
            .unwrap()
            .username
            .as_deref()
            .unwrap_or("unknown")
            .to_string();

        // Ensure user exists in Convex
        self.ensure_user_exists(user_id, &username).await?;

        if let Some(text) = msg.text() {
            if let Ok(command) = Command::parse(text, "SolanaBot") {
                self.handle_command(&msg, command).await?;
            } else {
                self.handle_text_message(&msg, text).await?;
            }
        }

        Ok(())
    }

    /// Handle bot commands
    pub async fn handle_command(&self, msg: &Message, command: Command) -> Result<()> {
        let chat_id = msg.chat.id;
        let user_id = msg.from().unwrap().id.0 as i64;

        match command {
            Command::Start => {
                self.handle_start_command(chat_id, user_id).await?;
            }
            Command::Portfolio => {
                self.handle_portfolio_command(chat_id, user_id).await?;
            }
            Command::Trade { token } => {
                self.handle_trade_command(chat_id, user_id, token).await?;
            }
            Command::Dca => {
                self.handle_dca_command(chat_id, user_id).await?;
            }
            Command::Alerts => {
                self.handle_alerts_command(chat_id, user_id).await?;
            }
            Command::Signals => {
                self.handle_signals_command(chat_id, user_id).await?;
            }
            Command::Wallet => {
                self.handle_wallet_command(chat_id, user_id).await?;
            }
            Command::Help => {
                self.handle_help_command(chat_id).await?;
            }
        }

        Ok(())
    }

    /// Handle inline queries
    pub async fn handle_inline_query(&self, query: InlineQuery) -> Result<()> {
        let query_text = &query.query;
        let user_id = query.from.id.0 as i64;

        let results = match query_text.to_lowercase().as_str() {
            "portfolio" => self.create_portfolio_inline_results(user_id).await?,
            "dca" => self.create_dca_inline_results(user_id).await?,
            "trending" => self.create_trending_inline_results().await?,
            _ if query_text.len() >= 2 => {
                self.create_token_search_results(query_text).await?
            }
            _ => Vec::new(),
        };

        self.bot
            .answer_inline_query(&query.id, results)
            .await?;

        Ok(())
    }

    // Command Handlers

    async fn handle_start_command(&self, chat_id: ChatId, user_id: i64) -> Result<()> {
        // Get or detect user language
        let user_lang = self.get_user_language(user_id).await?;
        
        // Check if this is a first-time user (needs language setup)
        let user_exists = self.convex.get_user_by_telegram_id(user_id).await?.is_some();
        
        if !user_exists {
            // New user - show language selection first
            let language_keyboard = self.create_language_selection_keyboard();
            let language_text = self.translate(&user_lang, "commands.start.language_setup", &[]);

            self.bot
                .send_message(chat_id, language_text)
                .reply_markup(language_keyboard)
                .await?;
        } else {
            // Existing user - show main menu
            let keyboard = self.create_main_keyboard(&user_lang);
            let welcome_text = self.translate(&user_lang, "commands.start.welcome", &[]);

            self.bot
                .send_message(chat_id, welcome_text)
                .reply_markup(keyboard)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
        }

        Ok(())
    }

    async fn handle_portfolio_command(&self, chat_id: ChatId, user_id: i64) -> Result<()> {
        let user_id_str = format!("user_{}", user_id);
        
        match self.convex.get_portfolio(&user_id_str).await {
            Ok(portfolio) => {
                let portfolio_text = format!(
                    "📊 **Portfolio Overview**\n\n\
                    💰 Total Value: ${}\n\
                    📈 Total P&L: {} ({}%)\n\
                    🎯 Positions: {}\n\n\
                    Use the web dashboard for detailed analytics:\n\
                    https://dashboard.solanabot.com",
                    portfolio.total_value,
                    portfolio.total_pnl,
                    portfolio.total_pnl_percentage,
                    portfolio.position_count
                );

                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("📊 Detailed View", "portfolio_detail"),
                        teloxide::types::InlineKeyboardButton::callback("🔄 Refresh", "portfolio_refresh"),
                    ],
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("💱 Quick Trade", "quick_trade"),
                    ],
                ]);

                self.bot
                    .send_message(chat_id, portfolio_text)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(e) => {
                self.bot
                    .send_message(chat_id, format!("❌ Error loading portfolio: {}", e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_trade_command(&self, chat_id: ChatId, user_id: i64, token: Option<String>) -> Result<()> {
        let token_symbol = token.unwrap_or_else(|| "SOL".to_string());
        
        // Get token mint address (simplified for example)
        let token_mint = match token_symbol.to_uppercase().as_str() {
            "SOL" => "So11111111111111111111111111111111111111112",
            _ => "So11111111111111111111111111111111111111112", // Default to SOL
        };

        // Send rich media price chart instead of just text
        match self.send_price_chart_media(chat_id.0, token_mint, &token_symbol).await {
            Ok(_) => {
                // Chart sent successfully
                println!("✅ Price chart sent for {}", token_symbol);
            }
            Err(e) => {
                // Fallback to text-based interface
                println!("⚠️ Rich media failed, using text fallback: {}", e);
                
                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("💰 Buy $10", &format!("buy_{}_{}", token_symbol, 10)),
                        teloxide::types::InlineKeyboardButton::callback("💰 Buy $50", &format!("buy_{}_{}", token_symbol, 50)),
                        teloxide::types::InlineKeyboardButton::callback("💰 Buy $100", &format!("buy_{}_{}", token_symbol, 100)),
                    ],
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("📉 Sell 25%", &format!("sell_{}_25", token_symbol)),
                        teloxide::types::InlineKeyboardButton::callback("📉 Sell 50%", &format!("sell_{}_50", token_symbol)),
                        teloxide::types::InlineKeyboardButton::callback("📉 Sell 100%", &format!("sell_{}_100", token_symbol)),
                    ],
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("📊 Price Chart", &format!("chart_{}", token_symbol)),
                        teloxide::types::InlineKeyboardButton::callback("🧠 AI Signal", &format!("signal_{}", token_symbol)),
                    ],
                ]);

                // Get current price
                let price_info = match self.get_token_price_info(&token_symbol).await {
                    Ok(info) => format!("Current Price: ${:.6}", info["price"].as_f64().unwrap_or(0.0)),
                    Err(_) => "Price unavailable".to_string(),
                };

                let trade_text = format!(
                    "💱 **Quick Trade: {}**\n\n\
                    {}\n\n\
                    Select your trading action:",
                    token_symbol, price_info
                );

                self.bot
                    .send_message(chat_id, trade_text)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .reply_markup(keyboard)
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_dca_command(&self, chat_id: ChatId, user_id: i64) -> Result<()> {
        let user_id_str = format!("user_{}", user_id);
        
        match self.convex.get_user_dca_strategies(&user_id_str).await {
            Ok(strategies) => {
                let strategies_text = if strategies.is_empty() {
                    "🤖 **DCA Strategies**\n\nNo active strategies found.\n\nDCA (Dollar Cost Averaging) helps reduce volatility by investing fixed amounts regularly."
                } else {
                    let strategy_list = strategies
                        .iter()
                        .take(5)
                        .map(|s| {
                            format!(
                                "• {} → {}: ${} every {}",
                                s["fromSymbol"].as_str().unwrap_or("?"),
                                s["toSymbol"].as_str().unwrap_or("?"),
                                s["amount"].as_str().unwrap_or("0"),
                                s["frequency"].as_str().unwrap_or("?")
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    &format!("🤖 **Active DCA Strategies**\n\n{}", strategy_list)
                };

                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("➕ New Strategy", "dca_new"),
                        teloxide::types::InlineKeyboardButton::callback("📊 Performance", "dca_stats"),
                    ],
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("⏸️ Pause All", "dca_pause"),
                        teloxide::types::InlineKeyboardButton::callback("▶️ Resume All", "dca_resume"),
                    ],
                ]);

                self.bot
                    .send_message(chat_id, strategies_text)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(e) => {
                self.bot
                    .send_message(chat_id, format!("❌ Error loading DCA strategies: {}", e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_signals_command(&self, chat_id: ChatId, user_id: i64) -> Result<()> {
        match self.convex.get_latest_signals(5).await {
            Ok(signals) => {
                let signals_text = if signals.is_empty() {
                    "🧠 **AI Trading Signals**\n\nNo recent signals available.\n\nAI analyzes market data to provide trading recommendations."
                } else {
                    let signal_list = signals
                        .iter()
                        .map(|s| {
                            let signal_emoji = match s.signal_type.as_str() {
                                "bullish" => "🟢",
                                "bearish" => "🔴",
                                _ => "🟡",
                            };
                            
                            format!(
                                "{} **{}**: {} ({:.0}% confidence)\n   {}",
                                signal_emoji,
                                s.token_mint,
                                s.signal_type.to_uppercase(),
                                s.confidence * 100.0,
                                s.reasoning
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n\n");

                    &format!("🧠 **Latest AI Signals**\n\n{}", signal_list)
                };

                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("🔄 Refresh", "signals_refresh"),
                        teloxide::types::InlineKeyboardButton::callback("⚙️ Settings", "signals_settings"),
                    ],
                ]);

                self.bot
                    .send_message(chat_id, signals_text)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(e) => {
                self.bot
                    .send_message(chat_id, format!("❌ Error loading AI signals: {}", e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_alerts_command(&self, chat_id: ChatId, user_id: i64) -> Result<()> {
        let user_id_str = format!("user_{}", user_id);
        
        match self.convex.get_user_alerts(&user_id_str).await {
            Ok(alerts) => {
                let alerts_text = if alerts.is_empty() {
                    "🔔 **Price Alerts**\n\nNo active alerts found.\n\nSet up price alerts to get notified when tokens reach your target prices."
                } else {
                    let alert_list = alerts
                        .iter()
                        .take(5)
                        .map(|a| {
                            format!(
                                "• {} {} ${}",
                                a["tokenSymbol"].as_str().unwrap_or("?"),
                                a["condition"].as_str().unwrap_or("?"),
                                a["targetPrice"].as_f64().unwrap_or(0.0)
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    &format!("🔔 **Active Alerts**\n\n{}", alert_list)
                };

                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        teloxide::types::InlineKeyboardButton::callback("➕ New Alert", "alert_new"),
                        teloxide::types::InlineKeyboardButton::callback("📊 Alert History", "alert_history"),
                    ],
                ]);

                self.bot
                    .send_message(chat_id, alerts_text)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(e) => {
                self.bot
                    .send_message(chat_id, format!("❌ Error loading alerts: {}", e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_wallet_command(&self, chat_id: ChatId, user_id: i64) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                teloxide::types::InlineKeyboardButton::callback("🔗 Connect Wallet", "wallet_connect"),
                teloxide::types::InlineKeyboardButton::callback("💰 Balances", "wallet_balances"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🔄 Sync Balances", "wallet_sync"),
                teloxide::types::InlineKeyboardButton::callback("📊 Transactions", "wallet_history"),
            ],
        ]);

        let wallet_text = "💳 **Wallet Management**\n\n\
            Connect your Solana wallet to start trading:\n\
            • Phantom Wallet\n\
            • Hardware Wallets (Ledger/Trezor)\n\
            • WalletConnect\n\n\
            Your keys remain secure - we never store private keys.";

        self.bot
            .send_message(chat_id, wallet_text)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn handle_help_command(&self, chat_id: ChatId) -> Result<()> {
        let help_text = "🤖 **Solana Trading Bot Help**\n\n\
            **Commands:**\n\
            /start - Initialize bot\n\
            /portfolio - View portfolio\n\
            /trade [token] - Quick trade\n\
            /dca - DCA strategies\n\
            /alerts - Price alerts\n\
            /signals - AI signals\n\
            /wallet - Wallet management\n\n\
            **Inline Queries:**\n\
            Type @SolanaBot followed by:\n\
            • `portfolio` - Portfolio summary\n\
            • `dca` - DCA strategies\n\
            • `trending` - Trending tokens\n\
            • Token symbol for quick info\n\n\
            **Support:**\n\
            📧 support@solanabot.com\n\
            🌐 docs.solanabot.com";

        self.bot
            .send_message(chat_id, help_text)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }

    // Helper Methods

    async fn ensure_user_exists(&self, telegram_id: i64, username: &str) -> Result<()> {
        if self.convex.get_user_by_telegram_id(telegram_id).await?.is_none() {
            self.convex.create_or_update_user(telegram_id, username).await?;
        }
        Ok(())
    }

    async fn get_token_price_info(&self, token_symbol: &str) -> Result<Value> {
        // This would need a symbol-to-mint mapping
        let mint = match token_symbol.to_uppercase().as_str() {
            "SOL" => "So11111111111111111111111111111111111111112",
            _ => return Err(anyhow::anyhow!("Unknown token: {}", token_symbol)),
        };

        self.convex.get_token_price(mint).await
    }

    async fn handle_text_message(&self, msg: &Message, text: &str) -> Result<()> {
        // Handle natural language queries
        let chat_id = msg.chat.id;

        if text.to_lowercase().contains("price") {
            // Extract token symbol and show price
            self.handle_price_query(chat_id, text).await?;
        } else if text.to_lowercase().contains("buy") || text.to_lowercase().contains("sell") {
            // Parse trading intent
            self.handle_trading_intent(chat_id, text).await?;
        } else {
            // General help
            self.bot
                .send_message(chat_id, "Use /help to see available commands or try inline queries with @SolanaBot")
                .await?;
        }

        Ok(())
    }

    async fn handle_price_query(&self, chat_id: ChatId, text: &str) -> Result<()> {
        // Simple token extraction - in production, use NLP
        let token = if text.to_uppercase().contains("SOL") {
            "SOL"
        } else {
            "SOL" // Default
        };

        match self.get_token_price_info(token).await {
            Ok(price_info) => {
                let price_text = format!(
                    "💰 **{} Price**\n\n\
                    Current: ${:.6}\n\
                    24h Change: +2.34%\n\n\
                    Use /trade {} for quick trading",
                    token,
                    price_info["price"].as_f64().unwrap_or(0.0),
                    token
                );

                self.bot
                    .send_message(chat_id, price_text)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            Err(e) => {
                self.bot
                    .send_message(chat_id, format!("❌ Could not fetch price for {}: {}", token, e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_trading_intent(&self, chat_id: ChatId, text: &str) -> Result<()> {
        self.bot
            .send_message(chat_id, "🤖 I detected trading intent! Use /trade [token] for the trading interface.")
            .await?;

        Ok(())
    }

    // Inline Query Results

    async fn create_portfolio_inline_results(&self, user_id: i64) -> Result<Vec<teloxide::types::InlineQueryResult>> {
        // Implementation would create inline query results for portfolio
        Ok(Vec::new()) // Simplified for brevity
    }

    async fn create_dca_inline_results(&self, user_id: i64) -> Result<Vec<teloxide::types::InlineQueryResult>> {
        Ok(Vec::new()) // Simplified for brevity
    }

    async fn create_trending_inline_results(&self) -> Result<Vec<teloxide::types::InlineQueryResult>> {
        Ok(Vec::new()) // Simplified for brevity
    }

    async fn create_token_search_results(&self, query: &str) -> Result<Vec<teloxide::types::InlineQueryResult>> {
        Ok(Vec::new()) // Simplified for brevity
    }

    // Rich Media Methods

    async fn send_price_chart_media(&self, chat_id: i64, token_mint: &str, symbol: &str) -> Result<()> {
        use teloxide::types::{InputFile, InputMedia, InputMediaPhoto};
        
        // Generate price chart via Convex
        let chart_result: serde_json::Value = self.convex.action(
            "actions/media_generator:generatePriceChart",
            json!({
                "tokenMint": token_mint,
                "symbol": symbol,
                "interval": "1h",
                "period": 168,
                "chartType": "candlestick",
                "indicators": ["sma20", "rsi"],
                "theme": "dark"
            })
        ).await?;

        // Decode base64 image
        let image_base64 = chart_result["imageBase64"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No image data in response"))?;
        let image_data = base64::decode(image_base64)
            .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

        // Get current price for caption
        let price_info = self.get_token_price_info(symbol).await?;
        let current_price = price_info["price"].as_f64().unwrap_or(0.0);
        let price_change = price_info.get("priceChange24h").and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        let change_emoji = if price_change >= 0.0 { "📈" } else { "📉" };
        let change_sign = if price_change >= 0.0 { "+" } else { "" };

        let caption = format!(
            "📊 **{} Price Chart**\n\n\
            💰 **Current Price:** ${:.6}\n\
            {} **24h Change:** {}{}%\n\
            📊 **Chart:** 1H Candlesticks with SMA20 & RSI\n\n\
            Use the buttons below to customize your view.",
            symbol, current_price, change_emoji, change_sign, price_change
        );

        // Create interactive keyboard
        let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![
            vec![
                teloxide::types::InlineKeyboardButton::callback("1H", &format!("chart_{}_{}", token_mint, "1h")),
                teloxide::types::InlineKeyboardButton::callback("4H", &format!("chart_{}_{}", token_mint, "4h")),
                teloxide::types::InlineKeyboardButton::callback("1D", &format!("chart_{}_{}", token_mint, "1d")),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("📈 Line", &format!("chart_type_{}_line", token_mint)),
                teloxide::types::InlineKeyboardButton::callback("🕯️ Candles", &format!("chart_type_{}_candlestick", token_mint)),
                teloxide::types::InlineKeyboardButton::callback("📊 Area", &format!("chart_type_{}_area", token_mint)),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("💱 Quick Trade", &format!("trade_{}", token_mint)),
                teloxide::types::InlineKeyboardButton::callback("🧠 AI Analysis", &format!("analysis_{}", token_mint)),
            ],
        ]);

        // Send photo with caption and keyboard
        let input_file = InputFile::memory(image_data);
        self.bot
            .send_photo(teloxide::types::ChatId(chat_id), input_file)
            .caption(caption)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn send_portfolio_overview_media(&self, chat_id: i64, user_id: i64) -> Result<()> {
        use teloxide::types::InputFile;

        let user_id_str = format!("user_{}", user_id);

        // Generate portfolio image via Convex
        let portfolio_result: serde_json::Value = self.convex.action(
            "actions/media_generator:generatePortfolioImage", 
            json!({
                "userId": user_id_str,
                "theme": "dark",
                "includeChart": true
            })
        ).await?;

        // Get portfolio data for caption
        let portfolio = self.convex.get_portfolio(&user_id_str).await?;

        // Decode image
        let image_base64 = portfolio_result["imageBase64"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No image data in response"))?;
        let image_data = base64::decode(image_base64)
            .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

        let pnl_emoji = if portfolio.total_pnl.starts_with('-') { "📉" } else { "📈" };
        let pnl_sign = if portfolio.total_pnl.starts_with('-') { "" } else { "+" };

        let caption = format!(
            "💼 **Portfolio Overview**\n\n\
            💰 **Total Value:** ${}\n\
            {} **P&L:** {}{} ({}%)\n\
            🎯 **Positions:** {}\n\n\
            📅 **Updated:** {}\n\n\
            Tap buttons below for detailed analysis.",
            portfolio.total_value,
            pnl_emoji, pnl_sign, portfolio.total_pnl, portfolio.total_pnl_percentage,
            portfolio.position_count,
            chrono::Utc::now().format("%H:%M UTC")
        );

        // Create keyboard
        let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![
            vec![
                teloxide::types::InlineKeyboardButton::callback("📊 Details", &format!("portfolio_detail_{}", user_id)),
                teloxide::types::InlineKeyboardButton::callback("🔄 Refresh", &format!("portfolio_refresh_{}", user_id)),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("💱 Rebalance", &format!("rebalance_{}", user_id)),
                teloxide::types::InlineKeyboardButton::callback("🤖 AI Tips", &format!("ai_tips_{}", user_id)),
            ],
        ]);

        // Send photo
        let input_file = InputFile::memory(image_data);
        self.bot
            .send_photo(teloxide::types::ChatId(chat_id), input_file)
            .caption(caption)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn send_trading_signal_media(&self, chat_id: i64, token_mint: &str) -> Result<()> {
        use teloxide::types::InputFile;

        // Get latest trading signal
        let signals = self.convex.get_latest_signals(1).await?;
        if signals.is_empty() {
            return Err(anyhow::anyhow!("No signals available"));
        }

        let signal = &signals[0];

        // Generate signal card via Convex
        let signal_result: serde_json::Value = self.convex.action(
            "actions/media_generator:generateSignalCard",
            json!({
                "signalId": "signal_id", // Would use actual signal ID
                "theme": "dark",
                "includeChart": true
            })
        ).await?;

        // Decode image
        let image_base64 = signal_result["imageBase64"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No image data in response"))?;
        let image_data = base64::decode(image_base64)
            .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

        let action_emoji = match signal.action.as_str() {
            "buy" => "🟢",
            "sell" => "🔴",
            "hold" => "🟡",
            _ => "⚪",
        };

        let caption = format!(
            "{} **{} Trading Signal**\n\n\
            🎯 **Action:** {}\n\
            📊 **Confidence:** {}%\n\
            ⚡ **Strength:** {}/100\n\
            ⏰ **Timeframe:** {}\n\
            ⚠️ **Risk:** {}\n\n\
            💭 **Reasoning:** {}",
            action_emoji,
            signal.token_mint, // Would have symbol
            signal.action.to_uppercase(),
            signal.confidence,
            signal.strength,
            signal.timeframe,
            signal.risk_level.to_uppercase(),
            signal.reasoning.chars().take(150).collect::<String>()
        );

        // Create action-specific keyboard
        let mut keyboard_rows = vec![];

        if signal.action == "buy" {
            keyboard_rows.push(vec![
                teloxide::types::InlineKeyboardButton::callback("💰 Buy $10", &format!("quick_buy_{}_10", token_mint)),
                teloxide::types::InlineKeyboardButton::callback("💰 Buy $50", &format!("quick_buy_{}_50", token_mint)),
                teloxide::types::InlineKeyboardButton::callback("💰 Buy $100", &format!("quick_buy_{}_100", token_mint)),
            ]);
        }

        keyboard_rows.push(vec![
            teloxide::types::InlineKeyboardButton::callback("📊 Analysis", &format!("analysis_{}", token_mint)),
            teloxide::types::InlineKeyboardButton::callback("📈 Chart", &format!("chart_{}", token_mint)),
        ]);

        keyboard_rows.push(vec![
            teloxide::types::InlineKeyboardButton::callback("🔔 Set Alert", &format!("alert_{}", token_mint)),
            teloxide::types::InlineKeyboardButton::callback("❌ Dismiss", "dismiss_signal"),
        ]);

        let keyboard = teloxide::types::InlineKeyboardMarkup::new(keyboard_rows);

        // Send photo
        let input_file = InputFile::memory(image_data);
        self.bot
            .send_photo(teloxide::types::ChatId(chat_id), input_file)
            .caption(caption)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn send_market_overview_media(&self, chat_id: i64, category: &str) -> Result<()> {
        use teloxide::types::InputFile;

        // Generate market overview via Convex
        let overview_result: serde_json::Value = self.convex.action(
            "actions/media_generator:generateMarketOverview",
            json!({
                "category": category,
                "limit": 10,
                "theme": "dark"
            })
        ).await?;

        // Decode image
        let image_base64 = overview_result["imageBase64"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No image data in response"))?;
        let image_data = base64::decode(image_base64)
            .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

        let caption = format!(
            "📊 **Market {}**\n\n\
            📅 **Updated:** {}\n\n\
            Use buttons below to explore different market views.",
            category.chars().next().unwrap().to_uppercase().to_string() + &category[1..],
            chrono::Utc::now().format("%H:%M UTC")
        );

        // Create keyboard
        let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![
            vec![
                teloxide::types::InlineKeyboardButton::callback("📈 Trending", "market_trending"),
                teloxide::types::InlineKeyboardButton::callback("🚀 Movers", "market_movers"),
                teloxide::types::InlineKeyboardButton::callback("📊 Volume", "market_volume"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🔍 Search", "token_search"),
                teloxide::types::InlineKeyboardButton::callback("💡 AI Picks", "ai_picks"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🔄 Refresh", &format!("market_refresh_{}", category)),
            ],
        ]);

        // Send photo
        let input_file = InputFile::memory(image_data);
        self.bot
            .send_photo(teloxide::types::ChatId(chat_id), input_file)
            .caption(caption)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    // Internationalization Helper Methods

    async fn get_user_language(&self, user_id: i64) -> Result<String> {
        match self.convex.get_user_by_telegram_id(user_id).await? {
            Some(user) => {
                // Check if user has language preference
                if let Some(settings) = user.settings.as_object() {
                    if let Some(lang) = settings.get("language") {
                        if let Some(lang_str) = lang.as_str() {
                            return Ok(lang_str.to_string());
                        }
                    }
                }
                Ok("en".to_string()) // Default to English
            }
            None => Ok("en".to_string()), // Default for new users
        }
    }

    fn translate(&self, lang: &str, key: &str, params: &[(&str, &str)]) -> String {
        // Simple translation function - in production would use full i18n system
        let translations = self.get_translations();
        
        if let Some(lang_translations) = translations.get(lang) {
            if let Some(translation) = self.get_nested_translation(lang_translations, key) {
                return self.replace_params(translation, params);
            }
        }
        
        // Fallback to English
        if let Some(en_translations) = translations.get("en") {
            if let Some(translation) = self.get_nested_translation(en_translations, key) {
                return self.replace_params(translation, params);
            }
        }
        
        // Return key if translation not found
        format!("[{}]", key)
    }

    fn get_translations(&self) -> std::collections::HashMap<&str, std::collections::HashMap<&str, &str>> {
        let mut translations = std::collections::HashMap::new();
        
        // English translations
        let mut en = std::collections::HashMap::new();
        en.insert("commands.start.welcome", "🚀 Welcome to Solana Trading Bot!\n\nYour AI-powered companion for Solana trading with:\n• Real-time portfolio tracking\n• Advanced DCA strategies\n• AI trading signals\n• Price alerts & notifications\n\nChoose an option below to get started:");
        en.insert("commands.start.language_setup", "Please select your preferred language:");
        en.insert("commands.start.user_created", "Welcome! Your account has been created. You can now start trading!");
        en.insert("commands.portfolio.title", "📊 Portfolio Overview");
        en.insert("commands.portfolio.total_value", "💰 Total Value: ${{value}}");
        en.insert("commands.portfolio.total_pnl", "📈 Total P&L: {{sign}}${{amount}} ({{percentage}}%)");
        en.insert("commands.portfolio.positions", "🎯 Positions: {{count}}");
        en.insert("commands.portfolio.no_portfolio", "No portfolio data available. Connect a wallet to get started!");
        en.insert("commands.trade.title", "💱 Quick Trade: {{symbol}}");
        en.insert("commands.trade.current_price", "💰 Current Price: ${{price}}");
        en.insert("commands.trade.select_action", "Select your trading action:");
        en.insert("buttons.portfolio", "📊 Portfolio");
        en.insert("buttons.trade", "💱 Trade");
        en.insert("buttons.dca", "🤖 DCA");
        en.insert("buttons.alerts", "🔔 Alerts");
        en.insert("buttons.signals", "🧠 AI Signals");
        en.insert("buttons.wallet", "💳 Wallet");
        en.insert("buttons.settings", "⚙️ Settings");
        en.insert("buttons.help", "❓ Help");
        en.insert("buttons.refresh", "🔄 Refresh");
        en.insert("buttons.back", "⬅️ Back");
        
        // Spanish translations
        let mut es = std::collections::HashMap::new();
        es.insert("commands.start.welcome", "🚀 ¡Bienvenido a Solana Trading Bot!\n\nTu compañero impulsado por IA para trading de Solana con:\n• Seguimiento de portafolio en tiempo real\n• Estrategias DCA avanzadas\n• Señales de trading AI\n• Alertas de precio y notificaciones\n\nElige una opción para comenzar:");
        es.insert("commands.start.language_setup", "Por favor selecciona tu idioma preferido:");
        es.insert("commands.start.user_created", "¡Bienvenido! Tu cuenta ha sido creada. ¡Ya puedes comenzar a hacer trading!");
        es.insert("commands.portfolio.title", "📊 Resumen del Portafolio");
        es.insert("commands.portfolio.total_value", "💰 Valor Total: ${{value}}");
        es.insert("commands.portfolio.total_pnl", "📈 P&L Total: {{sign}}${{amount}} ({{percentage}}%)");
        es.insert("commands.portfolio.positions", "🎯 Posiciones: {{count}}");
        es.insert("commands.portfolio.no_portfolio", "No hay datos de portafolio disponibles. ¡Conecta una billetera para empezar!");
        es.insert("commands.trade.title", "💱 Trade Rápido: {{symbol}}");
        es.insert("commands.trade.current_price", "💰 Precio Actual: ${{price}}");
        es.insert("commands.trade.select_action", "Selecciona tu acción de trading:");
        es.insert("buttons.portfolio", "📊 Portafolio");
        es.insert("buttons.trade", "💱 Trade");
        es.insert("buttons.dca", "🤖 DCA");
        es.insert("buttons.alerts", "🔔 Alertas");
        es.insert("buttons.signals", "🧠 Señales IA");
        es.insert("buttons.wallet", "💳 Billetera");
        es.insert("buttons.settings", "⚙️ Configuración");
        es.insert("buttons.help", "❓ Ayuda");
        es.insert("buttons.refresh", "🔄 Actualizar");
        es.insert("buttons.back", "⬅️ Atrás");
        
        translations.insert("en", en);
        translations.insert("es", es);
        translations
    }

    fn get_nested_translation(&self, translations: &std::collections::HashMap<&str, &str>, key: &str) -> Option<&str> {
        translations.get(key).copied()
    }

    fn replace_params(&self, text: &str, params: &[(&str, &str)]) -> String {
        let mut result = text.to_string();
        for (param, value) in params {
            result = result.replace(&format!("{{{{{}}}}}", param), value);
        }
        result
    }

    fn create_language_selection_keyboard(&self) -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                teloxide::types::InlineKeyboardButton::callback("🇺🇸 English", "lang_en"),
                teloxide::types::InlineKeyboardButton::callback("🇪🇸 Español", "lang_es"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🇫🇷 Français", "lang_fr"),
                teloxide::types::InlineKeyboardButton::callback("🇩🇪 Deutsch", "lang_de"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🇮🇹 Italiano", "lang_it"),
                teloxide::types::InlineKeyboardButton::callback("🇧🇷 Português", "lang_pt"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🇷🇺 Русский", "lang_ru"),
                teloxide::types::InlineKeyboardButton::callback("🇨🇳 中文", "lang_zh"),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback("🇯🇵 日本語", "lang_ja"),
                teloxide::types::InlineKeyboardButton::callback("🇰🇷 한국어", "lang_ko"),
            ],
        ])
    }

    fn create_main_keyboard(&self, lang: &str) -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.portfolio", &[]),
                    "portfolio"
                ),
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.trade", &[]),
                    "trade"
                ),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.dca", &[]),
                    "dca"
                ),
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.alerts", &[]),
                    "alerts"
                ),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.signals", &[]),
                    "signals"
                ),
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.wallet", &[]),
                    "wallet"
                ),
            ],
            vec![
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.settings", &[]),
                    "settings"
                ),
                teloxide::types::InlineKeyboardButton::callback(
                    &self.translate(lang, "buttons.help", &[]),
                    "help"
                ),
            ],
        ])
    }

    async fn handle_language_selection(&self, chat_id: ChatId, user_id: i64, language_code: &str) -> Result<()> {
        // Update user language preference
        let user_id_str = format!("user_{}", user_id);
        
        // Create or update user with language preference
        let username = "user"; // Would get from Telegram user info
        let _user_id = self.convex.create_or_update_user(user_id, username).await?;
        
        // Update user settings with language
        let settings = json!({
            "defaultSlippage": 1.0,
            "riskTolerance": "medium",
            "notifications": true,
            "language": language_code
        });

        // Update user settings (simplified - would use proper mutation)
        // self.convex.mutation("mutations/users:updateSettings", json!({
        //     "userId": user_id_str,
        //     "settings": settings
        // })).await?;

        // Show welcome message in selected language
        let keyboard = self.create_main_keyboard(language_code);
        let welcome_text = self.translate(language_code, "commands.start.welcome", &[]);
        let success_text = self.translate(language_code, "commands.start.user_created", &[]);

        self.bot
            .send_message(chat_id, format!("{}\n\n{}", success_text, welcome_text))
            .reply_markup(keyboard)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }

    async fn format_currency(&self, lang: &str, amount: f64) -> String {
        match lang {
            "es" => format!("${:.2}", amount), // Could use locale-specific formatting
            "fr" => format!("{:.2} $", amount),
            "de" => format!("{:.2} $", amount),
            _ => format!("${:.2}", amount), // Default USD format
        }
    }

    async fn format_percentage(&self, lang: &str, percentage: f64) -> String {
        match lang {
            "es" | "fr" | "de" => format!("{:.2}%", percentage),
            _ => format!("{:.2}%", percentage),
        }
    }
}