use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use crate::constants::KNOWN_TOKENS;
use crate::errors::{TradingError, BotError};

pub struct TokenResolver;

impl TokenResolver {
    /// Resolve token symbol or mint address to a valid mint address
    pub fn resolve(token: &str) -> Result<String> {
        // First check if it's a known token symbol
        for (symbol, mint) in KNOWN_TOKENS {
            if token.to_uppercase() == *symbol {
                return Ok(mint.to_string());
            }
        }
        
        // If not found, assume it's a mint address
        if token.len() == 44 {
            // Validate it's a valid base58 pubkey
            Pubkey::from_str(token)
                .map_err(|_| BotError::Trading(TradingError::TokenNotFound(token.to_string()).to_string()))?;
            Ok(token.to_string())
        } else {
            Err(BotError::Trading(TradingError::TokenNotFound(
                format!("{} - please provide a valid token symbol or mint address", token)
            ).to_string()).into())
        }
    }
    
    /// Get token symbol from mint address
    pub fn get_symbol(mint: &str) -> String {
        for (symbol, known_mint) in KNOWN_TOKENS {
            if mint == *known_mint {
                return symbol.to_string();
            }
        }
        // Return shortened mint address if not found
        format!("{}...{}", &mint[..4], &mint[mint.len()-4..])
    }
    
    /// Check if a token is a stablecoin
    pub fn is_stablecoin(token: &str) -> bool {
        matches!(token.to_uppercase().as_str(), "USDC" | "USDT" | "DAI" | "BUSD")
    }
    
    /// Get all known tokens
    pub fn get_known_tokens() -> Vec<(String, String)> {
        KNOWN_TOKENS.iter()
            .map(|(symbol, mint)| (symbol.to_string(), mint.to_string()))
            .collect()
    }
}