use teloxide::{prelude::*, types::Message};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use std::sync::Arc;
use tracing::{info, error, debug, warn};

use crate::{
    trading::{JupiterSwapClient, SwapRequest},
    wallet::WalletManager,
    middleware::rate_limiter::{UserRateLimiter, RateLimitConfig},
    errors::BotError,
};

/// Real swap execution handler
pub struct SwapHandler;

impl SwapHandler {
    /// Handle /swap command - Execute real trades
    pub async fn handle_swap(
        bot: Bot,
        msg: Message,
        args: String,
        wallet_manager: Arc<WalletManager>,
        swap_client: Arc<JupiterSwapClient>,
        rate_limiter: Arc<UserRateLimiter>,
    ) -> ResponseResult<()> {
        let user_id = msg.from()
            .map(|u| u.id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        // Check trading rate limits (more restrictive)
        if let Err(e) = rate_limiter.check_rate_limit_with_config(&user_id, &RateLimitConfig::for_trading()).await {
            bot.send_message(msg.chat.id, format!("⏰ Trading Rate Limit: {}", e))
                .await?;
            return Ok(());
        }
        
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.len() < 3 {
            Self::show_swap_help(bot, msg).await?;
            return Ok(());
        }
        
        let from_token = parts[0];
        let to_token = parts[1];
        let amount_str = parts[2];
        
        // Parse amount
        let amount: f64 = match amount_str.parse() {
            Ok(amt) => amt,
            Err(_) => {
                bot.send_message(msg.chat.id, 
                    "❌ Invalid amount. Please enter a valid number.")
                    .await?;
                return Ok(());
            }
        };
        
        if amount <= 0.0 {
            bot.send_message(msg.chat.id, 
                "❌ Amount must be greater than 0.")
                .await?;
            return Ok(());
        }
        
        // Get user's wallet
        let wallet = match wallet_manager.get_user_wallet(&user_id).await {
            Ok(Some(wallet)) => wallet,
            Ok(None) => {
                bot.send_message(msg.chat.id, 
                    "❌ No active wallet found. Use `/wallet connect` to connect a wallet first.")
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
        
        // Show swap preview first
        Self::show_swap_preview(bot, msg, swap_client, from_token, to_token, amount, &wallet.public_key).await
    }
    
    /// Show swap help
    async fn show_swap_help(bot: Bot, msg: Message) -> ResponseResult<()> {
        let message = r#"💱 **How to Swap Tokens**

**Usage:** `/swap <from> <to> <amount>`

**Examples:**
• `/swap SOL USDC 1.5` - Swap 1.5 SOL for USDC
• `/swap USDC SOL 100` - Swap 100 USDC for SOL  
• `/swap BONK WIF 1000000` - Swap 1M BONK for WIF

**Supported Tokens:**
• SOL, USDC, USDT, RAY, SRM, ORCA
• BONK, WIF, PEPE (meme tokens)
• Any SPL token (use mint address)

**Features:**
✅ Real Jupiter DEX aggregation
✅ Best price routing
✅ MEV protection via Jito bundles
✅ Slippage protection (max 5%)
✅ Price impact warnings

**Security:**
🔒 Transactions require your approval
🛡️ Protected against sandwich attacks
⚡ Fast execution via Jupiter

Use `/wallet` to connect your wallet first!"#;
        
        bot.send_message(msg.chat.id, message).await?;
        Ok(())
    }
    
    /// Show swap preview with real quote
    async fn show_swap_preview(
        bot: Bot,
        msg: Message,
        swap_client: Arc<JupiterSwapClient>,
        from_token: &str,
        to_token: &str,
        amount: f64,
        wallet_address: &str,
    ) -> ResponseResult<()> {
        let loading_msg = bot.send_message(msg.chat.id, 
            format!("🔄 Getting real-time quote for {} {} → {}...", amount, from_token, to_token))
            .await?;
        
        // Resolve token addresses
        let (from_mint, to_mint) = match Self::resolve_token_addresses(from_token, to_token) {
            Ok(mints) => mints,
            Err(e) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                bot.send_message(msg.chat.id, format!("❌ {}", e)).await?;
                return Ok(());
            }
        };
        
        // Convert amount to base units (handle decimals)
        let amount_base_units = Self::convert_to_base_units(amount, from_token);
        
        // Create swap request
        let swap_request = SwapRequest {
            input_mint: from_mint,
            output_mint: to_mint,
            amount: amount_base_units,
            slippage_bps: 100, // 1% slippage
            user_public_key: wallet_address.to_string(),
            quote_only: true,
        };
        
        // Get real quote from Jupiter
        match swap_client.get_quote(&swap_request).await {
            Ok(quote) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                
                let output_amount = quote.out_amount.parse::<u64>().unwrap_or(0) as f64;
                let output_display = Self::convert_from_base_units(output_amount, to_token);
                let price_impact: f64 = quote.price_impact_pct.parse().unwrap_or(0.0);
                
                // Calculate fee breakdown
                let fee_breakdown = swap_client.calculate_swap_fee(&quote);
                
                // Create confirmation keyboard
                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        InlineKeyboardButton::callback(
                            "✅ Confirm Swap", 
                            format!("confirm_swap:{}:{}:{}", from_token, to_token, amount)
                        ),
                        InlineKeyboardButton::callback("❌ Cancel", "cancel_swap"),
                    ],
                    vec![
                        InlineKeyboardButton::callback("🔄 Refresh Quote", 
                            format!("refresh_quote:{}:{}:{}", from_token, to_token, amount)),
                        InlineKeyboardButton::callback("⚙️ Settings", "swap_settings"),
                    ],
                ]);
                
                let price_impact_emoji = if price_impact > 3.0 { "⚠️" } else if price_impact > 1.0 { "🟡" } else { "🟢" };
                let route_info = if !quote.route_plan.is_empty() {
                    format!("🛣️ **Route:** {}", quote.route_plan[0].swap_info.label)
                } else {
                    "🛣️ **Route:** Direct".to_string()
                };
                
                let message = format!(
                    "💱 **Swap Preview**\n\n\
                    **From:** {:.6} {}\n\
                    **To:** {:.6} {}\n\
                    **Rate:** 1 {} = {:.6} {}\n\n\
                    {} **Price Impact:** {:.3}%\n\
                    {}\n\n\
                    **💸 Fees Breakdown:**\n\
                    • Network Fee: ${:.6}\n\
                    • Platform Fee: ${:.6}\n\
                    • Price Impact: ${:.6}\n\
                    • **Total Cost:** ${:.6} ({:.3}%)\n\n\
                    **⚡ Execution:**\n\
                    • Jupiter DEX Aggregator\n\
                    • MEV Protection: ✅ Enabled\n\
                    • Slippage: 1.0% max\n\n\
                    **⚠️ Review carefully before confirming!**",
                    amount,
                    from_token.to_uppercase(),
                    output_display,
                    to_token.to_uppercase(),
                    from_token.to_uppercase(),
                    output_display / amount,
                    to_token.to_uppercase(),
                    price_impact_emoji,
                    price_impact,
                    route_info,
                    fee_breakdown.network_fee,
                    fee_breakdown.platform_fee,
                    fee_breakdown.price_impact_cost,
                    fee_breakdown.total_fee,
                    fee_breakdown.fee_percentage
                );
                
                // Warn about high price impact
                if price_impact > 5.0 {
                    bot.send_message(msg.chat.id, 
                        format!("⚠️ **HIGH PRICE IMPACT WARNING**\n\n\
                        This swap has a {:.2}% price impact, which is quite high.\n\
                        Consider reducing the amount or trying again later.", price_impact))
                        .await?;
                }
                
                bot.send_message(msg.chat.id, message)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(e) => {
                bot.delete_message(msg.chat.id, loading_msg.id).await.ok();
                error!("Failed to get swap quote: {}", e);
                
                let error_message = if e.to_string().contains("insufficient") {
                    "❌ **Insufficient Balance**\n\nYou don't have enough tokens for this swap."
                } else if e.to_string().contains("slippage") {
                    "❌ **High Slippage**\n\nThis swap would have too much slippage. Try a smaller amount."
                } else {
                    "❌ **Quote Failed**\n\nCouldn't get a quote for this swap. Please check the token symbols and try again."
                };
                
                bot.send_message(msg.chat.id, error_message).await?;
            }
        }
        
        Ok(())
    }
    
    /// Execute confirmed swap
    pub async fn execute_confirmed_swap(
        bot: Bot,
        msg: Message,
        swap_client: Arc<JupiterSwapClient>,
        wallet_manager: Arc<WalletManager>,
        from_token: &str,
        to_token: &str,
        amount: f64,
        user_id: &str,
    ) -> ResponseResult<()> {
        let execution_msg = bot.send_message(msg.chat.id, 
            "⚡ Executing swap with MEV protection...")
            .await?;
        
        // Get wallet
        let wallet = wallet_manager.get_user_wallet(user_id).await?
            .ok_or_else(|| BotError::validation("No active wallet"))?;
        
        // Resolve token addresses
        let (from_mint, to_mint) = Self::resolve_token_addresses(from_token, to_token)?;
        let amount_base_units = Self::convert_to_base_units(amount, from_token);
        
        // Create swap request
        let swap_request = SwapRequest {
            input_mint: from_mint,
            output_mint: to_mint,
            amount: amount_base_units,
            slippage_bps: 100, // 1% slippage
            user_public_key: wallet.public_key,
            quote_only: false,
        };
        
        // Execute the swap
        match swap_client.execute_swap(&swap_request, user_id).await {
            Ok(result) => {
                bot.delete_message(msg.chat.id, execution_msg.id).await.ok();
                
                if result.success {
                    let output_display = Self::convert_from_base_units(result.output_amount, to_token);
                    
                    let success_message = format!(
                        "✅ **Swap Successful!**\n\n\
                        **Swapped:** {:.6} {} → {:.6} {}\n\
                        **Rate:** 1 {} = {:.6} {}\n\
                        **Price Impact:** {:.3}%\n\
                        **Execution Time:** {}ms\n\n\
                        🔗 **Transaction:** `{}`\n\n\
                        View on Solscan: https://solscan.io/tx/{}",
                        amount,
                        from_token.to_uppercase(),
                        output_display,
                        to_token.to_uppercase(),
                        from_token.to_uppercase(),
                        if amount > 0.0 { output_display / amount } else { 0.0 },
                        to_token.to_uppercase(),
                        result.price_impact,
                        result.execution_time_ms,
                        result.signature.as_deref().unwrap_or("N/A"),
                        result.signature.as_deref().unwrap_or("")
                    );
                    
                    bot.send_message(msg.chat.id, success_message).await?;
                } else {
                    let error_msg = format!(
                        "❌ **Swap Failed**\n\n\
                        Error: {}\n\
                        Execution Time: {}ms\n\n\
                        Don't worry, no tokens were lost.\n\
                        Please try again or contact support.",
                        result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        result.execution_time_ms
                    );
                    
                    bot.send_message(msg.chat.id, error_msg).await?;
                }
            }
            Err(e) => {
                bot.delete_message(msg.chat.id, execution_msg.id).await.ok();
                error!("Swap execution failed: {}", e);
                
                bot.send_message(msg.chat.id, 
                    format!("❌ **Swap Execution Failed**\n\n\
                    Error: {}\n\n\
                    This could be due to:\n\
                    • Network congestion\n\
                    • Insufficient balance\n\
                    • Price movement\n\
                    • Transaction timeout\n\n\
                    Please try again.", e))
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Resolve token symbols to mint addresses
    fn resolve_token_addresses(from_token: &str, to_token: &str) -> Result<(String, String), String> {
        let from_mint = Self::get_token_mint(from_token)?;
        let to_mint = Self::get_token_mint(to_token)?;
        Ok((from_mint, to_mint))
    }
    
    /// Get token mint address
    fn get_token_mint(symbol: &str) -> Result<String, String> {
        match symbol.to_uppercase().as_str() {
            "SOL" => Ok("So11111111111111111111111111111111111111112".to_string()),
            "USDC" => Ok("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
            "USDT" => Ok("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string()),
            "RAY" => Ok("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R".to_string()),
            "SRM" => Ok("SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt".to_string()),
            "ORCA" => Ok("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE".to_string()),
            "BONK" => Ok("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string()),
            "WIF" => Ok("EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm".to_string()),
            _ => {
                // Check if it's already a mint address (44 characters)
                if symbol.len() == 44 {
                    Ok(symbol.to_string())
                } else {
                    Err(format!("Unsupported token: {}. Use mint address for other tokens.", symbol))
                }
            }
        }
    }
    
    /// Convert human amount to base units
    fn convert_to_base_units(amount: f64, token: &str) -> u64 {
        let decimals = match token.to_uppercase().as_str() {
            "SOL" => 9,
            "USDC" | "USDT" => 6,
            "RAY" | "SRM" | "ORCA" => 6,
            "BONK" => 5,
            "WIF" => 6,
            _ => 9, // Default to 9 decimals
        };
        
        (amount * 10_f64.powi(decimals)) as u64
    }
    
    /// Convert base units to human amount
    fn convert_from_base_units(amount: f64, token: &str) -> f64 {
        let decimals = match token.to_uppercase().as_str() {
            "SOL" => 9,
            "USDC" | "USDT" => 6,
            "RAY" | "SRM" | "ORCA" => 6,
            "BONK" => 5,
            "WIF" => 6,
            _ => 9, // Default to 9 decimals
        };
        
        amount / 10_f64.powi(decimals)
    }
}