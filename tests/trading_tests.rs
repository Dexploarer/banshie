use crate::trading::types::{TradeResult, TradeType, Position, Balance};
use chrono::Utc;
use std::collections::BTreeMap;

#[test]
fn test_trade_result_creation() {
    let trade = TradeResult {
        tx_signature: "test_signature".to_string(),
        tokens_received: 1000.0,
        tokens_sold: 0.0,
        sol_received: 0.0,
        amount_sol: 1.0,
        price: 0.001,
        rebate_earned: 0.01,
        pnl_percentage: 0.0,
        timestamp: Utc::now(),
        trade_type: TradeType::Buy,
    };
    
    assert_eq!(trade.tx_signature, "test_signature");
    assert_eq!(trade.tokens_received, 1000.0);
    assert_eq!(trade.amount_sol, 1.0);
    assert_eq!(trade.trade_type, TradeType::Buy);
}

#[test]
fn test_position_equality() {
    let pos1 = Position {
        token: "BONK".to_string(),
        symbol: "BONK".to_string(),
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
        amount: 1000.0,
        value_usd: 100.0,
        pnl_percentage: 10.0,
        average_buy_price: 0.09,
        current_price: 0.1,
        sort_key: 1,
        last_updated: Utc::now(),
    };
    
    let pos2 = Position {
        token: "BONK".to_string(),
        symbol: "BONK".to_string(),
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
        amount: 2000.0, // Different amount
        value_usd: 200.0,
        pnl_percentage: 20.0,
        average_buy_price: 0.09,
        current_price: 0.1,
        sort_key: 2,
        last_updated: Utc::now(),
    };
    
    // Positions are equal if they have the same mint
    assert_eq!(pos1, pos2);
}

#[test]
fn test_position_ordering() {
    let pos1 = Position {
        token: "BONK".to_string(),
        symbol: "BONK".to_string(),
        mint: "mint1".to_string(),
        amount: 1000.0,
        value_usd: 100.0,
        pnl_percentage: 10.0,
        average_buy_price: 0.09,
        current_price: 0.1,
        sort_key: 1,
        last_updated: Utc::now(),
    };
    
    let pos2 = Position {
        token: "WIF".to_string(),
        symbol: "WIF".to_string(),
        mint: "mint2".to_string(),
        amount: 100.0,
        value_usd: 200.0, // Higher value
        pnl_percentage: 20.0,
        average_buy_price: 1.8,
        current_price: 2.0,
        sort_key: 2,
        last_updated: Utc::now(),
    };
    
    // Positions are ordered by value_usd
    assert!(pos1 < pos2);
}

#[test]
fn test_balance_creation() {
    let mut token_balances = BTreeMap::new();
    token_balances.insert(
        "BONK".to_string(),
        crate::trading::types::TokenBalance {
            mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
            symbol: "BONK".to_string(),
            amount: 1_000_000.0,
            value_usd: 50.0,
            price_per_token: 0.00005,
        },
    );
    
    let balance = Balance {
        sol: 10.5,
        usdc: 100.0,
        total_usd_value: 650.0, // 10.5 * 50 (SOL price) + 100 + 50
        token_balances,
        last_updated: Utc::now(),
    };
    
    assert_eq!(balance.sol, 10.5);
    assert_eq!(balance.usdc, 100.0);
    assert_eq!(balance.total_usd_value, 650.0);
    assert!(balance.token_balances.contains_key("BONK"));
}

#[test]
fn test_trade_type() {
    assert_eq!(TradeType::Buy, TradeType::Buy);
    assert_ne!(TradeType::Buy, TradeType::Sell);
    assert_ne!(TradeType::Sell, TradeType::Swap);
}

#[test]
fn test_portfolio_operations() {
    use crate::trading::types::Portfolio;
    
    let mut portfolio = Portfolio::new();
    
    let position = Position {
        token: "BONK".to_string(),
        symbol: "BONK".to_string(),
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
        amount: 1000.0,
        value_usd: 100.0,
        pnl_percentage: 10.0,
        average_buy_price: 0.09,
        current_price: 0.1,
        sort_key: 1,
        last_updated: Utc::now(),
    };
    
    // Add position
    portfolio.add_position(position.clone());
    assert_eq!(portfolio.positions.len(), 1);
    assert!(portfolio.positions.contains_key(&position.mint));
    
    // Update position
    let mut updated_position = position.clone();
    updated_position.amount = 2000.0;
    updated_position.value_usd = 200.0;
    portfolio.update_position(updated_position);
    
    let pos = portfolio.positions.get(&position.mint).unwrap();
    assert_eq!(pos.amount, 2000.0);
    assert_eq!(pos.value_usd, 200.0);
    
    // Remove position
    portfolio.remove_position(&position.mint);
    assert_eq!(portfolio.positions.len(), 0);
}

#[test]
fn test_portfolio_calculations() {
    use crate::trading::types::Portfolio;
    
    let mut portfolio = Portfolio::new();
    
    // Add multiple positions
    portfolio.add_position(Position {
        token: "BONK".to_string(),
        symbol: "BONK".to_string(),
        mint: "mint1".to_string(),
        amount: 1000.0,
        value_usd: 100.0,
        pnl_percentage: 10.0,
        average_buy_price: 0.09,
        current_price: 0.1,
        sort_key: 1,
        last_updated: Utc::now(),
    });
    
    portfolio.add_position(Position {
        token: "WIF".to_string(),
        symbol: "WIF".to_string(),
        mint: "mint2".to_string(),
        amount: 50.0,
        value_usd: 150.0,
        pnl_percentage: -5.0,
        average_buy_price: 3.2,
        current_price: 3.0,
        sort_key: 2,
        last_updated: Utc::now(),
    });
    
    // Calculate totals
    portfolio.calculate_totals();
    
    assert_eq!(portfolio.total_value_usd, 250.0);
    // Weighted average PnL: (100 * 10 + 150 * -5) / 250 = (1000 - 750) / 250 = 1.0
    assert_eq!(portfolio.total_pnl_percentage, 1.0);
}