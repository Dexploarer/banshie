use teloxide::{prelude::*, types::{Message, CallbackQuery}};
use std::sync::Arc;
use tracing::{info, error};

use crate::{
    trading::TradingEngineHandle,
    wallet::WalletManager,
    db::Database,
    errors::Result,
    utils::validation::{Validator, ValidatedAmount, ValidatedPercentage, ValidatedTokenSymbol, ValidatedUserId},
};

/// Handler for trading-related operations
pub struct TradingHandler;

impl TradingHandler {
    /// Execute a quick trade from callback
    pub async fn execute_quick_trade(
        bot: &Bot,
        q: &CallbackQuery,
        token: &str,
        amount: f64,
        is_buy: bool,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
    ) -> ResponseResult<()> {
        if let Some(msg) = &q.message {
            // Validate and sanitize user ID
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
            
            // Validate token symbol
            let validated_token = match ValidatedTokenSymbol::new(token) {
                Ok(t) => t,
                Err(e) => {
                    error!("Invalid token symbol {}: {}", token, e);
                    bot.send_message(msg.chat.id, "❌ Invalid token symbol")
                        .await?;
                    return Ok(());
                }
            };
            
            // Validate trade amount
            let validated_amount = match ValidatedAmount::new(amount, crate::constants::MAX_TRADE_SOL) {
                Ok(a) => a,
                Err(e) => {
                    error!("Invalid trade amount {}: {}", amount, e);
                    bot.send_message(msg.chat.id, format!("❌ {}", e))
                        .await?;
                    return Ok(());
                }
            };
            
            let user_wallet = match wallet_manager.get_user_wallet(user_id.as_str()).await {
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
            
            if is_buy {
                match trading_engine.buy_with_rebate(user_wallet.clone(), validated_token.as_str().to_string(), validated_amount.value()).await {
                    Ok(result) => {
                        let message = format!(
                            "✅ Quick buy executed\\!\n{} {} for {} SOL\nRebate: {:.6} SOL\n\n[View on Solscan](https://solscan\\.io/tx/{})",
                            result.tokens_received, validated_token.as_str(), validated_amount.value(), result.rebate_earned, result.tx_signature
                        );
                        bot.send_message(msg.chat.id, message)
                            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("❌ Trade failed: {}", e))
                            .await?;
                    }
                }
            } else {
                // Quick sell logic would go here
                bot.send_message(msg.chat.id, "💸 Quick sell feature coming soon!")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle buy command
    pub async fn handle_buy(
        bot: Bot,
        msg: Message,
        args: String,
        trading_engine: TradingEngineHandle,
        db: Arc<Database>,
        wallet_manager: Arc<WalletManager>,
        user_id: String,
    ) -> ResponseResult<()> {
        // Validate user ID
        let validated_user_id = match ValidatedUserId::new(&user_id) {
            Ok(id) => id,
            Err(e) => {
                error!("Invalid user ID {}: {}", user_id, e);
                bot.send_message(msg.chat.id, "❌ Invalid user session")
                    .await?;
                return Ok(());
            }
        };
        
        // Check if user has a wallet configured
        let user_wallet = match wallet_manager.get_user_wallet(validated_user_id.as_str()).await {
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
        
        // Sanitize and validate command arguments
        let sanitized_args = match Validator::sanitize_command_args(&args) {
            Ok(args) => args,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ {}", e))
                    .await?;
                return Ok(());
            }
        };
        
        let parts: Vec<&str> = sanitized_args.split_whitespace().collect();
        if parts.len() != 2 {
            bot.send_message(
                msg.chat.id, 
                "Usage: /buy <token> <amount_sol>\\nExample: /buy BONK 0\\.1"
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
            return Ok(());
        }
        
        // Validate token symbol
        let validated_token = match ValidatedTokenSymbol::new(parts[0]) {
            Ok(t) => t,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ {}", e))
                    .await?;
                return Ok(());
            }
        };
        
        // Parse and validate amount
        let amount: f64 = match parts[1].parse() {
            Ok(a) => a,
            Err(_) => {
                bot.send_message(msg.chat.id, "❌ Invalid amount format")
                    .await?;
                return Ok(());
            }
        };
        
        let validated_amount = match ValidatedAmount::new(amount, crate::constants::MAX_TRADE_SOL) {
            Ok(a) => a,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ {}", e))
                    .await?;
                return Ok(());
            }
        };
        
        bot.send_message(msg.chat.id, format!("⏳ Buying {} with {} SOL...", validated_token.as_str(), validated_amount.value()))
            .await?;
        
        match trading_engine.buy_with_rebate(user_wallet.clone(), validated_token.as_str().to_string(), validated_amount.value()).await {
            Ok(result) => {
                let message = format!(
                    "✅ *Buy Order Executed*\\n\\n\
                    Token: {}\\n\
                    Amount: {} SOL\\n\
                    Received: {:.2} tokens\\n\
                    Price: ${:.8}\\n\
                    Rebate Earned: {:.6} SOL\\n\\n\
                    [View Transaction](https://solscan\\.io/tx/{})",
                    validated_token.as_str(),
                    validated_amount.value(),
                    result.tokens_received,
                    result.price,
                    result.rebate_earned,
                    result.tx_signature
                );
                
                bot.send_message(msg.chat.id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
                
                // Record trade in database
                let _ = db.record_trade(
                    validated_user_id.as_str(),
                    validated_token.as_str(),
                    validated_amount.value(),
                    result.tokens_received,
                    result.rebate_earned,
                    &result.tx_signature,
                ).await;
            }
            Err(e) => {
                error!("Trade failed: {}", e);
                bot.send_message(msg.chat.id, format!("❌ Trade failed: {}", e))
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle sell command
    pub async fn handle_sell(
        bot: Bot,
        msg: Message,
        args: String,
        trading_engine: TradingEngineHandle,
        db: Arc<Database>,
        wallet_manager: Arc<WalletManager>,
        user_id: String,
    ) -> ResponseResult<()> {
        // Validate user ID
        let validated_user_id = match ValidatedUserId::new(&user_id) {
            Ok(id) => id,
            Err(e) => {
                error!("Invalid user ID {}: {}", user_id, e);
                bot.send_message(msg.chat.id, "❌ Invalid user session")
                    .await?;
                return Ok(());
            }
        };
        
        // Check if user has a wallet configured
        let user_wallet = match wallet_manager.get_user_wallet(validated_user_id.as_str()).await {
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
        
        // Sanitize and validate command arguments
        let sanitized_args = match Validator::sanitize_command_args(&args) {
            Ok(args) => args,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ {}", e))
                    .await?;
                return Ok(());
            }
        };
        
        let parts: Vec<&str> = sanitized_args.split_whitespace().collect();
        if parts.len() != 2 {
            bot.send_message(
                msg.chat.id,
                "Usage: /sell <token> <percentage>\\nExample: /sell BONK 50"
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
            return Ok(());
        }
        
        // Validate token symbol
        let validated_token = match ValidatedTokenSymbol::new(parts[0]) {
            Ok(t) => t,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ {}", e))
                    .await?;
                return Ok(());
            }
        };
        
        // Parse and validate percentage
        let percentage: f64 = match parts[1].parse() {
            Ok(p) => p,
            Err(_) => {
                bot.send_message(msg.chat.id, "❌ Invalid percentage format")
                    .await?;
                return Ok(());
            }
        };
        
        let validated_percentage = match ValidatedPercentage::new(percentage) {
            Ok(p) => p,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ {}", e))
                    .await?;
                return Ok(());
            }
        };
        
        bot.send_message(msg.chat.id, format!("⏳ Selling {}% of {}...", validated_percentage.value(), validated_token.as_str()))
            .await?;
        
        match trading_engine.sell_with_rebate(user_wallet.clone(), validated_token.as_str().to_string(), validated_percentage.value()).await {
            Ok(result) => {
                let pnl_emoji = if result.pnl_percentage >= 0.0 { "📈" } else { "📉" };
                let pnl_sign = if result.pnl_percentage >= 0.0 { "+" } else { "" };
                
                let message = format!(
                    "✅ *Sell Order Executed*\\n\\n\
                    Token: {}\\n\
                    Sold: {}%\\n\
                    Received: {:.4} SOL\\n\
                    Price: ${:.8}\\n\
                    Rebate Earned: {:.6} SOL\\n\
                    {} P&L: {}{:.2}%\\n\\n\
                    [View Transaction](https://solscan\\.io/tx/{})",
                    validated_token.as_str(),
                    validated_percentage.value(),
                    result.sol_received,
                    result.price,
                    result.rebate_earned,
                    pnl_emoji,
                    pnl_sign,
                    result.pnl_percentage,
                    result.tx_signature
                );
                
                bot.send_message(msg.chat.id, message)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
                
                // Record trade in database
                let _ = db.record_trade(
                    validated_user_id.as_str(),
                    validated_token.as_str(),
                    -result.sol_received,
                    -result.tokens_sold,
                    result.rebate_earned,
                    &result.tx_signature,
                ).await;
            }
            Err(e) => {
                error!("Sell failed: {}", e);
                bot.send_message(msg.chat.id, format!("❌ Sell failed: {}", e))
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle portfolio command
    pub async fn handle_portfolio(
        bot: Bot,
        msg: Message,
        trading_engine: TradingEngineHandle,
        wallet_manager: Arc<WalletManager>,
        user_id: String,
    ) -> ResponseResult<()> {
        // Validate user ID
        let validated_user_id = match ValidatedUserId::new(&user_id) {
            Ok(id) => id,
            Err(e) => {
                error!("Invalid user ID {}: {}", user_id, e);
                bot.send_message(msg.chat.id, "❌ Invalid user session")
                    .await?;
                return Ok(());
            }
        };
        
        // Check if user has a wallet configured
        let user_wallet = match wallet_manager.get_user_wallet(validated_user_id.as_str()).await {
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
        
        match trading_engine.get_positions(user_wallet.clone()).await {
            Ok(positions) => {
                if positions.is_empty() {
                    bot.send_message(
                        msg.chat.id,
                        "📊 *Portfolio Empty*\\n\\nYou don't have any token positions\\.\n\nStart trading to build your portfolio\\!"
                    )
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
                } else {
                    let mut message = String::from("📊 *Your Portfolio*\\n\\n");
                    
                    for position in positions.iter() {
                        let pnl_emoji = if position.pnl_percentage >= 0.0 { "📈" } else { "📉" };
                        let pnl_sign = if position.pnl_percentage >= 0.0 { "\\+" } else { "" };
                        
                        message.push_str(&format!(
                            "💎 **{}**\\n\
                            Amount: {:.2}\\n\
                            Value: ${:.2}\\n\
                            {} P&L: {}{:.2}%\\n\\n",
                            position.token_symbol,
                            position.amount,
                            position.current_value_usd,
                            pnl_emoji,
                            pnl_sign,
                            position.pnl_percentage
                        ));
                    }
                    
                    message.push_str("_Portfolio updated in real\\-time_");
                    
                    bot.send_message(msg.chat.id, message)
                        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                        .await?;
                }
            }
            Err(e) => {
                error!("Failed to get positions: {}", e);
                bot.send_message(msg.chat.id, "❌ Failed to fetch portfolio")
                    .await?;
            }
        }
        
        Ok(())
    }
}