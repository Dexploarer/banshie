use crate::utils::Validator;
use crate::errors::{BotError, TradingError, WalletError};

#[test]
fn test_validate_pubkey() {
    // Valid Solana address
    let valid_address = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
    assert!(Validator::validate_pubkey(valid_address).is_ok());
    
    // Invalid addresses
    assert!(Validator::validate_pubkey("").is_err());
    assert!(Validator::validate_pubkey("short").is_err());
    assert!(Validator::validate_pubkey("invalid_address_xyz").is_err());
    assert!(Validator::validate_pubkey(&"x".repeat(100)).is_err());
}

#[test]
fn test_validate_trade_amount() {
    // Valid amounts
    assert!(Validator::validate_trade_amount(0.1, 10.0).is_ok());
    assert!(Validator::validate_trade_amount(1.0, 10.0).is_ok());
    assert!(Validator::validate_trade_amount(5.0, 10.0).is_ok());
    
    // Invalid amounts
    assert!(Validator::validate_trade_amount(0.0, 10.0).is_err());
    assert!(Validator::validate_trade_amount(-1.0, 10.0).is_err());
    assert!(Validator::validate_trade_amount(0.0001, 10.0).is_err()); // Below MIN_TRADE_SOL
    assert!(Validator::validate_trade_amount(15.0, 10.0).is_err()); // Above max_allowed
    assert!(Validator::validate_trade_amount(f64::NAN, 10.0).is_err());
    assert!(Validator::validate_trade_amount(f64::INFINITY, 10.0).is_err());
}

#[test]
fn test_validate_percentage() {
    // Valid percentages
    assert!(Validator::validate_percentage(1.0).is_ok());
    assert!(Validator::validate_percentage(50.0).is_ok());
    assert!(Validator::validate_percentage(100.0).is_ok());
    
    // Invalid percentages
    assert!(Validator::validate_percentage(0.0).is_err());
    assert!(Validator::validate_percentage(-10.0).is_err());
    assert!(Validator::validate_percentage(101.0).is_err());
    assert!(Validator::validate_percentage(f64::NAN).is_err());
}

#[test]
fn test_validate_slippage() {
    // Valid slippage
    assert!(Validator::validate_slippage(100).is_ok());
    assert!(Validator::validate_slippage(300).is_ok());
    assert!(Validator::validate_slippage(500).is_ok());
    
    // Invalid slippage (above MAX_SLIPPAGE_BPS)
    assert!(Validator::validate_slippage(1500).is_err());
    assert!(Validator::validate_slippage(2000).is_err());
}

#[test]
fn test_validate_signature() {
    // Valid signature (base58 encoded)
    let valid_sig = "5xMockTxHash123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    assert!(Validator::validate_signature(valid_sig).is_ok());
    
    // Invalid signatures
    assert!(Validator::validate_signature("").is_err());
    assert!(Validator::validate_signature("short").is_err());
    assert!(Validator::validate_signature(&"x".repeat(100)).is_err());
}

#[test]
fn test_validate_user_id() {
    // Valid user IDs (numeric Telegram IDs)
    assert!(Validator::validate_user_id("123456789").is_ok());
    assert!(Validator::validate_user_id("987654321").is_ok());
    
    // Invalid user IDs
    assert!(Validator::validate_user_id("").is_err());
    assert!(Validator::validate_user_id("abc123").is_err());
    assert!(Validator::validate_user_id(&"1".repeat(30)).is_err());
}

#[test]
fn test_validate_token_symbol() {
    // Valid symbols
    assert_eq!(Validator::validate_token_symbol("BONK").unwrap(), "BONK");
    assert_eq!(Validator::validate_token_symbol("wif").unwrap(), "WIF");
    assert_eq!(Validator::validate_token_symbol("SOL123").unwrap(), "SOL123");
    
    // Invalid symbols
    assert!(Validator::validate_token_symbol("").is_err());
    assert!(Validator::validate_token_symbol(&"X".repeat(30)).is_err());
    assert!(Validator::validate_token_symbol("SOL-USD").is_err()); // Contains hyphen
    assert!(Validator::validate_token_symbol("$TOKEN").is_err()); // Contains special char
}

#[test]
fn test_validate_priority_fee() {
    // Valid fees
    assert!(Validator::validate_priority_fee(1000).is_ok());
    assert!(Validator::validate_priority_fee(50000).is_ok());
    assert!(Validator::validate_priority_fee(100000).is_ok());
    
    // Invalid fees (above max)
    assert!(Validator::validate_priority_fee(20_000_000).is_err());
}

#[test]
fn test_validate_sufficient_balance() {
    // Sufficient balance
    assert!(Validator::validate_sufficient_balance(10.0, 5.0).is_ok());
    assert!(Validator::validate_sufficient_balance(100.0, 100.0).is_ok());
    
    // Insufficient balance
    assert!(Validator::validate_sufficient_balance(5.0, 10.0).is_err());
    assert!(Validator::validate_sufficient_balance(0.0, 1.0).is_err());
}

#[test]
fn test_sanitize_input() {
    use std::borrow::Cow;
    
    // Clean input (no changes)
    assert!(matches!(Validator::sanitize_input("hello world"), Cow::Borrowed(_)));
    assert!(matches!(Validator::sanitize_input("BONK123"), Cow::Borrowed(_)));
    
    // Input needing sanitization
    let sanitized = Validator::sanitize_input("hello@#$%world!");
    assert_eq!(&*sanitized, "helloworld");
    
    let sanitized = Validator::sanitize_input("  spaces  ");
    assert_eq!(&*sanitized, "spaces");
    
    // Long input gets truncated
    let long_input = "a".repeat(150);
    let sanitized = Validator::sanitize_input(&long_input);
    assert!(sanitized.len() <= 100);
}

#[test]
fn test_sanitize_command_args() {
    // Valid args
    assert!(Validator::sanitize_command_args("buy 100 BONK").is_ok());
    assert!(Validator::sanitize_command_args("wallet:address").is_ok());
    
    // Empty args
    assert!(Validator::sanitize_command_args("").is_err());
    assert!(Validator::sanitize_command_args("   ").is_err());
    
    // Too long args
    let long_args = "x".repeat(300);
    assert!(Validator::sanitize_command_args(&long_args).is_err());
}