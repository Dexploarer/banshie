use teloxide::{prelude::*, types::Message};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use std::sync::Arc;
use tracing::{info, error, debug};

use crate::{
    portfolio::{PortfolioFetcher, PortfolioAnalyzer},
    wallet::WalletManager,
    middleware::rate_limiter::{UserRateLimiter, RateLimitConfig},
    errors::BotError,
};

/// Portfolio command handler with real data
pub struct PortfolioHandler;

impl PortfolioHandler {
    /// Handle /portfolio command - Show real portfolio data
    pub async fn handle_portfolio(
        bot: Bot,
        msg: Message,
        args: String,
        wallet_manager: Arc<WalletManager>,
        rate_limiter: Arc<UserRateLimiter>,
    ) -> ResponseResult<()> {
        let user_id = msg.from()
            .map(|u| u.id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        // Check rate limits
        if let Err(e) = rate_limiter.check_rate_limit_with_config(&user_id, &RateLimitConfig::for_portfolio()).await {
            bot.send_message(msg.chat.id, format!("⏰ {}", e))
                .await?;
            return Ok(());
        }
        
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.is_empty() {
            Self::show_portfolio_overview(bot, msg, wallet_manager, &user_id).await?;
        } else {
            match parts[0] {
                "holdings" => Self::show_detailed_holdings(bot, msg, wallet_manager, &user_id).await?,
                "performance" => Self::show_performance_analysis(bot, msg, wallet_manager, &user_id).await?,
                "analytics" => Self::show_portfolio_analytics(bot, msg, wallet_manager, &user_id).await?,
                "refresh" => Self::refresh_portfolio_data(bot, msg, wallet_manager, &user_id).await?,
                _ => {
                    bot.send_message(msg.chat.id, 
                        "❌ Unknown portfolio command. Use `/portfolio` to see options.")
                        .await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Show portfolio overview with real data
    async fn show_portfolio_overview(
        bot: Bot,
        msg: Message,
        wallet_manager: Arc<WalletManager>,
        user_id: &str,
    ) -> ResponseResult<()> {
        // Get user's active wallet
        let wallet = match wallet_manager.get_user_wallet(user_id).await {
            Ok(Some(wallet)) => wallet,
            Ok(None) => {
                bot.send_message(msg.chat.id, 
                    "❌ No active wallet found. Use `/wallet connect` to connect a wallet.")
                    .await?;
                return Ok(());
            }
            Err(e) => {
                error!("Failed to get user wallet: {}", e);
                bot.send_message(msg.chat.id, 
                    "❌ Failed to access wallet. Please try again.")
                    .await?;
                return Ok(());
            }
        };
        
        // Show loading message
        let loading_msg = bot.send_message(msg.chat.id, "🔄 Fetching real portfolio data...")
            .await?;
        
        // Initialize portfolio fetcher with real RPC endpoint
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let portfolio_fetcher = PortfolioFetcher::new(rpc_url);
        
        // Fetch real portfolio data
        match portfolio_fetcher.get_portfolio_summary(&wallet.public_key).await {
            Ok(summary) => {
                // Delete loading message
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                
                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        InlineKeyboardButton::callback("📊 Detailed Holdings", "portfolio_holdings"),
                        InlineKeyboardButton::callback("📈 Performance", "portfolio_performance"),
                    ],
                    vec![
                        InlineKeyboardButton::callback("🔍 Analytics", "portfolio_analytics"),
                        InlineKeyboardButton::callback("🔄 Refresh", "portfolio_refresh"),
                    ],
                    vec![
                        InlineKeyboardButton::callback("💱 Quick Swap", "quick_swap"),
                        InlineKeyboardButton::callback("📤 Send Tokens", "send_tokens"),
                    ],
                ]);
                
                let performance_emoji = if summary.performance_24h >= 0.0 { "📈" } else { "📉" };
                let performance_color = if summary.performance_24h >= 0.0 { "🟢" } else { "🔴" };
                
                let message = format!(
                    "💼 **Your Portfolio**\n\n\
                    💰 **Total Value:** ${:.2}\n\
                    📊 **Holdings:** {} tokens\n\
                    {} **24h Performance:** {}{:.2}%\n\n\
                    **🔝 Top Holdings:**\n",
                    summary.total_value_usd,
                    summary.total_holdings,
                    performance_emoji,
                    performance_color,
                    summary.performance_24h
                );
                
                let mut holdings_text = message;
                for (i, holding) in summary.top_holdings.iter().take(5).enumerate() {
                    holdings_text.push_str(&format!(
                        "{}. **{}** - {:.4} tokens (${:.2} - {:.1}%)\n",
                        i + 1,
                        holding.symbol,
                        holding.balance,
                        holding.value_usd,
                        holding.percentage
                    ));
                }
                
                if summary.total_value_usd > 0.0 {
                    holdings_text.push_str(&format!(
                        "\n💡 Wallet: `{}`\n\
                        ⏰ Last updated: Just now",
                        &wallet.public_key[..8]
                    ));
                } else {
                    holdings_text.push_str("\n💡 No tokens found in this wallet");
                }
                
                bot.send_message(msg.chat.id, holdings_text)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(e) => {
                // Delete loading message
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                
                error!("Failed to fetch portfolio: {}", e);
                bot.send_message(msg.chat.id, 
                    format!("❌ Failed to fetch portfolio data: {}\n\n\
                    This could be due to:\n\
                    • RPC endpoint issues\n\
                    • Network connectivity\n\
                    • Invalid wallet address\n\n\
                    Please try again later.", e))
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Show detailed holdings
    async fn show_detailed_holdings(
        bot: Bot,
        msg: Message,
        wallet_manager: Arc<WalletManager>,
        user_id: &str,
    ) -> ResponseResult<()> {
        let wallet = wallet_manager.get_user_wallet(user_id).await?
            .ok_or_else(|| BotError::validation("No active wallet"))?;
        
        let loading_msg = bot.send_message(msg.chat.id, "🔄 Loading detailed holdings...")
            .await?;
        
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let portfolio_fetcher = PortfolioFetcher::new(rpc_url);
        
        match portfolio_fetcher.fetch_portfolio(&wallet.public_key).await {
            Ok(portfolio) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                
                if portfolio.holdings.is_empty() {
                    bot.send_message(msg.chat.id, 
                        "💰 **No tokens found**\n\n\
                        This wallet doesn't contain any tokens.\n\
                        Send some SOL or tokens to start trading!")
                        .await?;
                    return Ok(());
                }
                
                let mut message = format!(
                    "📊 **Detailed Holdings** ({} tokens)\n\n",
                    portfolio.holdings.len()
                );
                
                for (i, holding) in portfolio.holdings.iter().enumerate() {
                    let verified_badge = if holding.is_verified { "✅" } else { "⚠️" };
                    let value_display = if holding.value_usd > 0.01 {
                        format!("${:.2}", holding.value_usd)
                    } else {
                        format!("${:.6}", holding.value_usd)
                    };
                    
                    message.push_str(&format!(
                        "{}. {} **{}** {}\n\
                           💰 {:.6} tokens\n\
                           💵 {} (${:.4} per token)\n\
                           🔗 `{}`\n\n",
                        i + 1,
                        verified_badge,
                        holding.symbol,
                        holding.name,
                        holding.balance,
                        value_display,
                        holding.price_usd,
                        &holding.mint_address[..8]
                    ));
                    
                    // Split into multiple messages if too long
                    if message.len() > 3500 {
                        bot.send_message(msg.chat.id, message).await?;
                        message = String::new();
                    }
                }
                
                if !message.is_empty() {
                    bot.send_message(msg.chat.id, message).await?;
                }
            }
            Err(e) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                error!("Failed to fetch detailed holdings: {}", e);
                bot.send_message(msg.chat.id, 
                    format!("❌ Failed to load holdings: {}", e))
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Show performance analysis
    async fn show_performance_analysis(
        bot: Bot,
        msg: Message,
        wallet_manager: Arc<WalletManager>,
        user_id: &str,
    ) -> ResponseResult<()> {
        let wallet = wallet_manager.get_user_wallet(user_id).await?
            .ok_or_else(|| BotError::validation("No active wallet"))?;
        
        let loading_msg = bot.send_message(msg.chat.id, "📈 Analyzing performance...")
            .await?;
        
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let portfolio_fetcher = PortfolioFetcher::new(rpc_url);
        let analyzer = PortfolioAnalyzer;
        
        match portfolio_fetcher.fetch_portfolio(&wallet.public_key).await {
            Ok(portfolio) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                
                let analysis = analyzer.analyze_portfolio(&portfolio);
                
                let message = format!(
                    "📈 **Portfolio Performance Analysis**\n\n\
                    💰 **Total Value:** ${:.2}\n\
                    📊 **24h Change:** {:.2}%\n\n\
                    **🎯 Diversification Score:** {:.1}/100\n\
                    **🔄 Effective Holdings:** {:.1}\n\
                    **⚖️ Largest Position:** {:.1}%\n\
                    **🏆 Top 5 Concentration:** {:.1}%\n\n\
                    **🎭 Risk Assessment:**\n\
                    • Overall Risk: {:?}\n\
                    • Volatility Score: {:.1}/100\n\
                    • Verified Tokens: {:.1}%\n\
                    • Small Cap Exposure: {:.1}%\n\n\
                    **📊 Allocation Breakdown:**\n\
                    • SOL: {:.1}%\n\
                    • Stablecoins: {:.1}%\n\
                    • DeFi: {:.1}%\n\
                    • Meme: {:.1}%\n\
                    • Other: {:.1}%",
                    analysis.total_value_usd,
                    portfolio.performance.pnl_24h_percentage,
                    analysis.diversification.diversification_score,
                    analysis.diversification.effective_holdings,
                    analysis.diversification.largest_position_percentage,
                    analysis.diversification.top_5_concentration,
                    analysis.risk_metrics.risk_level,
                    analysis.risk_metrics.volatility_score,
                    analysis.risk_metrics.verified_percentage,
                    analysis.risk_metrics.small_cap_exposure,
                    analysis.allocation.sol_percentage,
                    analysis.allocation.stablecoin_percentage,
                    analysis.allocation.defi_percentage,
                    analysis.allocation.meme_percentage,
                    analysis.allocation.other_percentage
                );
                
                bot.send_message(msg.chat.id, message).await?;
                
                // Show recommendations
                if !analysis.recommendations.is_empty() {
                    let mut recommendations_text = "💡 **Recommendations:**\n\n".to_string();
                    
                    for (i, rec) in analysis.recommendations.iter().take(3).enumerate() {
                        let priority_emoji = match rec.priority {
                            crate::portfolio::analyzer::RecommendationPriority::Critical => "🚨",
                            crate::portfolio::analyzer::RecommendationPriority::High => "⚠️",
                            crate::portfolio::analyzer::RecommendationPriority::Medium => "💡",
                            crate::portfolio::analyzer::RecommendationPriority::Low => "ℹ️",
                        };
                        
                        recommendations_text.push_str(&format!(
                            "{}. {} **{}**\n   {}\n",
                            i + 1,
                            priority_emoji,
                            rec.title,
                            rec.description
                        ));
                        
                        if let Some(action) = &rec.action {
                            recommendations_text.push_str(&format!("   🎯 Action: {}\n", action));
                        }
                        recommendations_text.push('\n');
                    }
                    
                    bot.send_message(msg.chat.id, recommendations_text).await?;
                }
            }
            Err(e) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                error!("Failed to analyze performance: {}", e);
                bot.send_message(msg.chat.id, 
                    format!("❌ Performance analysis failed: {}", e))
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Show portfolio analytics
    async fn show_portfolio_analytics(
        bot: Bot,
        msg: Message,
        wallet_manager: Arc<WalletManager>,
        user_id: &str,
    ) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, 
            "🔍 **Portfolio Analytics**\n\n\
            Advanced analytics features:\n\
            • Historical performance tracking\n\
            • Risk-adjusted returns\n\
            • Correlation analysis\n\
            • Rebalancing suggestions\n\n\
            📊 Use the web dashboard for detailed analytics:\n\
            http://127.0.0.1:3000/dashboard")
            .await?;
        
        Ok(())
    }
    
    /// Refresh portfolio data
    async fn refresh_portfolio_data(
        bot: Bot,
        msg: Message,
        wallet_manager: Arc<WalletManager>,
        user_id: &str,
    ) -> ResponseResult<()> {
        bot.send_message(msg.chat.id, "🔄 Refreshing portfolio data...").await?;
        
        // Re-run the portfolio overview with fresh data
        Self::show_portfolio_overview(bot, msg, wallet_manager, user_id).await
    }
}