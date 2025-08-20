use anyhow::Result;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use tracing::{info, debug};

use super::types::*;
use crate::errors::BotError;

/// Generates Solana Blinks for various actions
pub struct BlinkGenerator {
    base_url: String,
    network: SolanaNetwork,
}

impl BlinkGenerator {
    pub fn new(base_url: String, network: SolanaNetwork) -> Self {
        Self {
            base_url,
            network,
        }
    }
    
    /// Create a token swap blink
    pub fn create_swap_blink(
        &self,
        from_token: String,
        from_symbol: String,
        to_token: String,
        to_symbol: String,
        amount: f64,
        slippage: f64,
        creator_wallet: String,
    ) -> Result<SolanaBlink> {
        let blink = SolanaBlink {
            blink_id: SolanaBlink::generate_id(),
            blink_type: BlinkType::TokenSwap,
            title: format!("Swap {} {} for {}", amount, from_symbol, to_symbol),
            description: format!(
                "Swap {:.4} {} for {} with {:.1}% slippage tolerance",
                amount, from_symbol, to_symbol, slippage
            ),
            icon_url: Some("https://example.com/swap-icon.png".to_string()),
            action: BlinkAction {
                action_type: ActionType::Swap {
                    from_token: from_token.clone(),
                    to_token: to_token.clone(),
                    amount,
                },
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("from_token".to_string(), from_token);
                    params.insert("to_token".to_string(), to_token);
                    params.insert("amount".to_string(), amount.to_string());
                    params.insert("slippage".to_string(), slippage.to_string());
                    params.insert("dex".to_string(), "jupiter".to_string());
                    params
                },
                transaction_template: None,
                estimated_fee: Some(0.005), // 0.005 SOL
                requires_signature: true,
                multi_step: false,
                steps: vec![
                    ActionStep {
                        step_number: 1,
                        name: "Approve & Swap".to_string(),
                        description: format!("Swap {} for {}", from_symbol, to_symbol),
                        transaction: None,
                        validation: Some(StepValidation {
                            validation_type: ValidationType::BalanceCheck,
                            expected_result: "sufficient_balance".to_string(),
                            error_message: "Insufficient balance for swap".to_string(),
                        }),
                    },
                ],
            },
            metadata: BlinkMetadata {
                version: "1.0.0".to_string(),
                protocol: "jupiter-v6".to_string(),
                network: self.network.clone(),
                tags: vec!["swap".to_string(), "defi".to_string(), from_symbol, to_symbol],
                category: "DeFi".to_string(),
                language: "en".to_string(),
                custom_fields: HashMap::new(),
            },
            security: BlinkSecurity {
                verified: true,
                audit_status: AuditStatus::Verified,
                risk_level: RiskLevel::Low,
                warnings: vec![],
                requires_approval: false,
                max_uses: None,
                allowed_wallets: None,
                blocked_regions: None,
            },
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(24)),
            creator: BlinkCreator {
                wallet_address: creator_wallet,
                username: None,
                verified: false,
                reputation_score: None,
                created_count: 0,
            },
            analytics: BlinkAnalytics {
                views: 0,
                clicks: 0,
                executions: 0,
                success_rate: 0.0,
                average_execution_time: 0.0,
                total_volume: 0.0,
                unique_users: 0,
                referrals: HashMap::new(),
            },
            social_preview: self.create_social_preview(
                &format!("Swap {} for {}", from_symbol, to_symbol),
                &format!("One-click swap {:.4} {} for {} on Solana", amount, from_symbol, to_symbol),
            ),
        };
        
        Ok(blink)
    }
    
    /// Create a token transfer blink
    pub fn create_transfer_blink(
        &self,
        token: String,
        token_symbol: String,
        recipient: String,
        amount: f64,
        memo: Option<String>,
        creator_wallet: String,
    ) -> Result<SolanaBlink> {
        let blink = SolanaBlink {
            blink_id: SolanaBlink::generate_id(),
            blink_type: BlinkType::TokenTransfer,
            title: format!("Send {} {}", amount, token_symbol),
            description: format!(
                "Transfer {:.4} {} to {}{}",
                amount,
                token_symbol,
                &recipient[..8],
                memo.as_ref().map(|m| format!(" - {}", m)).unwrap_or_default()
            ),
            icon_url: Some("https://example.com/transfer-icon.png".to_string()),
            action: BlinkAction {
                action_type: ActionType::Transfer {
                    token: token.clone(),
                    recipient: recipient.clone(),
                    amount,
                },
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("token".to_string(), token);
                    params.insert("recipient".to_string(), recipient);
                    params.insert("amount".to_string(), amount.to_string());
                    if let Some(m) = memo {
                        params.insert("memo".to_string(), m);
                    }
                    params
                },
                transaction_template: None,
                estimated_fee: Some(0.000005), // Basic transfer fee
                requires_signature: true,
                multi_step: false,
                steps: vec![
                    ActionStep {
                        step_number: 1,
                        name: "Transfer".to_string(),
                        description: format!("Send {} {}", amount, token_symbol),
                        transaction: None,
                        validation: Some(StepValidation {
                            validation_type: ValidationType::BalanceCheck,
                            expected_result: "sufficient_balance".to_string(),
                            error_message: "Insufficient balance for transfer".to_string(),
                        }),
                    },
                ],
            },
            metadata: BlinkMetadata {
                version: "1.0.0".to_string(),
                protocol: "spl-token".to_string(),
                network: self.network.clone(),
                tags: vec!["transfer".to_string(), "payment".to_string(), token_symbol],
                category: "Payment".to_string(),
                language: "en".to_string(),
                custom_fields: HashMap::new(),
            },
            security: BlinkSecurity {
                verified: true,
                audit_status: AuditStatus::Verified,
                risk_level: RiskLevel::Low,
                warnings: vec![],
                requires_approval: false,
                max_uses: Some(1), // Single use for transfers
                allowed_wallets: None,
                blocked_regions: None,
            },
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(1)), // Short expiry for transfers
            creator: BlinkCreator {
                wallet_address: creator_wallet,
                username: None,
                verified: false,
                reputation_score: None,
                created_count: 0,
            },
            analytics: BlinkAnalytics {
                views: 0,
                clicks: 0,
                executions: 0,
                success_rate: 0.0,
                average_execution_time: 0.0,
                total_volume: 0.0,
                unique_users: 0,
                referrals: HashMap::new(),
            },
            social_preview: self.create_social_preview(
                &format!("Send {} {}", amount, token_symbol),
                &format!("Click to receive {:.4} {} on Solana", amount, token_symbol),
            ),
        };
        
        Ok(blink)
    }
    
    /// Create an NFT mint blink
    pub fn create_nft_mint_blink(
        &self,
        collection_address: String,
        collection_name: String,
        mint_price: f64,
        max_supply: Option<u32>,
        creator_wallet: String,
    ) -> Result<SolanaBlink> {
        let blink = SolanaBlink {
            blink_id: SolanaBlink::generate_id(),
            blink_type: BlinkType::NFTMint,
            title: format!("Mint {}", collection_name),
            description: format!(
                "Mint an NFT from {} collection for {} SOL{}",
                collection_name,
                mint_price,
                max_supply.map(|s| format!(" (Supply: {})", s)).unwrap_or_default()
            ),
            icon_url: Some("https://example.com/nft-icon.png".to_string()),
            action: BlinkAction {
                action_type: ActionType::Mint {
                    collection: collection_address.clone(),
                    price: mint_price,
                },
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("collection".to_string(), collection_address);
                    params.insert("price".to_string(), mint_price.to_string());
                    if let Some(supply) = max_supply {
                        params.insert("max_supply".to_string(), supply.to_string());
                    }
                    params
                },
                transaction_template: None,
                estimated_fee: Some(0.01 + mint_price), // Mint fee + price
                requires_signature: true,
                multi_step: false,
                steps: vec![
                    ActionStep {
                        step_number: 1,
                        name: "Mint NFT".to_string(),
                        description: format!("Mint from {} collection", collection_name),
                        transaction: None,
                        validation: Some(StepValidation {
                            validation_type: ValidationType::BalanceCheck,
                            expected_result: "sufficient_sol".to_string(),
                            error_message: format!("Need {} SOL to mint", mint_price + 0.01),
                        }),
                    },
                ],
            },
            metadata: BlinkMetadata {
                version: "1.0.0".to_string(),
                protocol: "metaplex".to_string(),
                network: self.network.clone(),
                tags: vec!["nft".to_string(), "mint".to_string(), collection_name.clone()],
                category: "NFT".to_string(),
                language: "en".to_string(),
                custom_fields: HashMap::new(),
            },
            security: BlinkSecurity {
                verified: false,
                audit_status: AuditStatus::NotAudited,
                risk_level: RiskLevel::Medium,
                warnings: vec!["Verify collection authenticity before minting".to_string()],
                requires_approval: false,
                max_uses: None,
                allowed_wallets: None,
                blocked_regions: None,
            },
            created_at: Utc::now(),
            expires_at: None, // No expiry for mints
            creator: BlinkCreator {
                wallet_address: creator_wallet,
                username: None,
                verified: false,
                reputation_score: None,
                created_count: 0,
            },
            analytics: BlinkAnalytics {
                views: 0,
                clicks: 0,
                executions: 0,
                success_rate: 0.0,
                average_execution_time: 0.0,
                total_volume: 0.0,
                unique_users: 0,
                referrals: HashMap::new(),
            },
            social_preview: self.create_social_preview(
                &format!("Mint {} NFT", collection_name),
                &format!("Mint now for {} SOL - Limited supply!", mint_price),
            ),
        };
        
        Ok(blink)
    }
    
    /// Create a staking blink
    pub fn create_staking_blink(
        &self,
        validator_address: String,
        validator_name: String,
        amount: f64,
        apy: f64,
        creator_wallet: String,
    ) -> Result<SolanaBlink> {
        let blink = SolanaBlink {
            blink_id: SolanaBlink::generate_id(),
            blink_type: BlinkType::Staking,
            title: format!("Stake {} SOL", amount),
            description: format!(
                "Stake {} SOL with {} validator ({:.2}% APY)",
                amount, validator_name, apy
            ),
            icon_url: Some("https://example.com/staking-icon.png".to_string()),
            action: BlinkAction {
                action_type: ActionType::Stake {
                    validator: validator_address.clone(),
                    amount,
                },
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("validator".to_string(), validator_address);
                    params.insert("amount".to_string(), amount.to_string());
                    params.insert("apy".to_string(), apy.to_string());
                    params
                },
                transaction_template: None,
                estimated_fee: Some(0.002), // Staking transaction fee
                requires_signature: true,
                multi_step: true,
                steps: vec![
                    ActionStep {
                        step_number: 1,
                        name: "Create Stake Account".to_string(),
                        description: "Create a new stake account".to_string(),
                        transaction: None,
                        validation: None,
                    },
                    ActionStep {
                        step_number: 2,
                        name: "Delegate Stake".to_string(),
                        description: format!("Delegate to {}", validator_name),
                        transaction: None,
                        validation: Some(StepValidation {
                            validation_type: ValidationType::ProgramState,
                            expected_result: "stake_activated".to_string(),
                            error_message: "Failed to activate stake".to_string(),
                        }),
                    },
                ],
            },
            metadata: BlinkMetadata {
                version: "1.0.0".to_string(),
                protocol: "stake-program".to_string(),
                network: self.network.clone(),
                tags: vec!["staking".to_string(), "defi".to_string(), "yield".to_string()],
                category: "Staking".to_string(),
                language: "en".to_string(),
                custom_fields: HashMap::new(),
            },
            security: BlinkSecurity {
                verified: true,
                audit_status: AuditStatus::Verified,
                risk_level: RiskLevel::Low,
                warnings: vec!["Staking has a cooldown period for unstaking".to_string()],
                requires_approval: false,
                max_uses: None,
                allowed_wallets: None,
                blocked_regions: None,
            },
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(7)),
            creator: BlinkCreator {
                wallet_address: creator_wallet,
                username: None,
                verified: false,
                reputation_score: None,
                created_count: 0,
            },
            analytics: BlinkAnalytics {
                views: 0,
                clicks: 0,
                executions: 0,
                success_rate: 0.0,
                average_execution_time: 0.0,
                total_volume: 0.0,
                unique_users: 0,
                referrals: HashMap::new(),
            },
            social_preview: self.create_social_preview(
                &format!("Stake {} SOL", amount),
                &format!("Earn {:.2}% APY by staking with {}", apy, validator_name),
            ),
        };
        
        Ok(blink)
    }
    
    /// Create social preview metadata
    fn create_social_preview(&self, title: &str, description: &str) -> SocialPreview {
        SocialPreview {
            title: title.to_string(),
            description: description.to_string(),
            image_url: Some("https://example.com/blink-preview.png".to_string()),
            twitter_card: TwitterCard {
                card_type: "summary_large_image".to_string(),
                site: Some("@SolanaTradeBot".to_string()),
                creator: Some("@SolanaTradeBot".to_string()),
                image_alt: Some(title.to_string()),
            },
            open_graph: OpenGraphData {
                og_type: "website".to_string(),
                og_url: self.base_url.clone(),
                og_title: title.to_string(),
                og_description: description.to_string(),
                og_image: Some("https://example.com/blink-preview.png".to_string()),
                og_site_name: "Solana Trading Bot".to_string(),
            },
        }
    }
    
    /// Generate a payment request blink
    pub fn create_payment_blink(
        &self,
        amount: f64,
        token: String,
        token_symbol: String,
        recipient: String,
        description: String,
        invoice_id: Option<String>,
    ) -> Result<SolanaBlink> {
        let blink = SolanaBlink {
            blink_id: invoice_id.unwrap_or_else(SolanaBlink::generate_id),
            blink_type: BlinkType::Payment,
            title: format!("Pay {} {}", amount, token_symbol),
            description: description.clone(),
            icon_url: Some("https://example.com/payment-icon.png".to_string()),
            action: BlinkAction {
                action_type: ActionType::Transfer {
                    token: token.clone(),
                    recipient: recipient.clone(),
                    amount,
                },
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("token".to_string(), token);
                    params.insert("recipient".to_string(), recipient.clone());
                    params.insert("amount".to_string(), amount.to_string());
                    params.insert("description".to_string(), description);
                    params
                },
                transaction_template: None,
                estimated_fee: Some(0.000005),
                requires_signature: true,
                multi_step: false,
                steps: vec![
                    ActionStep {
                        step_number: 1,
                        name: "Send Payment".to_string(),
                        description: format!("Pay {} {}", amount, token_symbol),
                        transaction: None,
                        validation: Some(StepValidation {
                            validation_type: ValidationType::BalanceCheck,
                            expected_result: "sufficient_balance".to_string(),
                            error_message: format!("Insufficient {} balance", token_symbol),
                        }),
                    },
                ],
            },
            metadata: BlinkMetadata {
                version: "1.0.0".to_string(),
                protocol: "spl-token".to_string(),
                network: self.network.clone(),
                tags: vec!["payment".to_string(), "invoice".to_string()],
                category: "Payment".to_string(),
                language: "en".to_string(),
                custom_fields: HashMap::new(),
            },
            security: BlinkSecurity {
                verified: true,
                audit_status: AuditStatus::Verified,
                risk_level: RiskLevel::Low,
                warnings: vec![],
                requires_approval: false,
                max_uses: Some(1),
                allowed_wallets: None,
                blocked_regions: None,
            },
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(24)),
            creator: BlinkCreator {
                wallet_address: recipient,
                username: None,
                verified: false,
                reputation_score: None,
                created_count: 0,
            },
            analytics: BlinkAnalytics {
                views: 0,
                clicks: 0,
                executions: 0,
                success_rate: 0.0,
                average_execution_time: 0.0,
                total_volume: 0.0,
                unique_users: 0,
                referrals: HashMap::new(),
            },
            social_preview: self.create_social_preview(
                &format!("Payment Request: {} {}", amount, token_symbol),
                &format!("Click to pay {} {}", amount, token_symbol),
            ),
        };
        
        Ok(blink)
    }
}