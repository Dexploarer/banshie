use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};
use crate::errors::{BotError, Result};
use crate::trading::{TokenCreator, TokenCreationConfig, TokenCreationResult, TokenPreset};

/// API request for creating a new Token-2022 token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub preset: Option<TokenPreset>,
    pub config: Option<TokenCreationConfig>,
    pub creator_wallet: String,
}

/// API response for token creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub success: bool,
    pub result: Option<TokenCreationResult>,
    pub error: Option<String>,
    pub estimated_cost_sol: f64,
    pub transaction_to_sign: Option<String>,
}

/// API request for getting token presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPresetsRequest {
    pub include_estimated_costs: bool,
}

/// API response for token presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPresetsResponse {
    pub presets: Vec<PresetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetInfo {
    pub preset_type: TokenPreset,
    pub config: TokenCreationConfig,
    pub estimated_cost_sol: f64,
    pub features: Vec<String>,
    pub revenue_potential: f64,
}

/// API request for validating token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateConfigRequest {
    pub config: TokenCreationConfig,
}

/// API response for config validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateConfigResponse {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub estimated_cost_sol: f64,
    pub features: Vec<String>,
}

/// Token Creator API Handler
pub struct TokenCreatorAPI {
    token_creator: Arc<TokenCreator>,
}

impl TokenCreatorAPI {
    pub fn new(token_creator: Arc<TokenCreator>) -> Self {
        Self { token_creator }
    }
    
    /// Get all available token presets with costs and features
    pub async fn get_presets(&self, request: GetPresetsRequest) -> Result<GetPresetsResponse> {
        info!("Fetching token creation presets");
        
        let presets = self.token_creator.get_all_presets();
        let mut preset_infos = Vec::new();
        
        for (preset_type, config) in presets {
            let estimated_cost = if request.include_estimated_costs {
                self.token_creator.estimate_creation_cost(config)?
            } else {
                0.0
            };
            
            let preview = self.token_creator.preview_token_features(config);
            
            preset_infos.push(PresetInfo {
                preset_type,
                config: config.clone(),
                estimated_cost_sol: estimated_cost,
                features: preview.features,
                revenue_potential: preview.monthly_revenue_potential,
            });
        }
        
        Ok(GetPresetsResponse {
            presets: preset_infos,
        })
    }
    
    /// Validate a token configuration
    pub async fn validate_config(&self, request: ValidateConfigRequest) -> Result<ValidateConfigResponse> {
        info!("Validating token configuration: {}", request.config.symbol);
        
        let validation_result = self.token_creator.validate_config(&request.config);
        let is_valid = validation_result.is_ok();
        
        let errors = if let Err(ref e) = validation_result {
            vec![e.to_string()]
        } else {
            vec![]
        };
        
        let estimated_cost = self.token_creator.estimate_creation_cost(&request.config)?;
        let preview = self.token_creator.preview_token_features(&request.config);
        
        Ok(ValidateConfigResponse {
            is_valid,
            errors,
            warnings: preview.warnings,
            recommendations: preview.recommendations,
            estimated_cost_sol: estimated_cost,
            features: preview.features,
        })
    }
    
    /// Create a new Token-2022 token
    pub async fn create_token(&self, request: CreateTokenRequest) -> Result<CreateTokenResponse> {
        info!("Creating new Token-2022 token for creator: {}", request.creator_wallet);
        
        // Get configuration from preset or use provided config
        let config = if let Some(preset) = request.preset {
            let mut preset_config = self.token_creator.get_preset(preset)
                .ok_or_else(|| BotError::validation("Invalid preset specified".to_string()))?;
            
            // Override creator address
            preset_config.creator_address = request.creator_wallet.parse()
                .map_err(|_| BotError::validation("Invalid creator wallet address".to_string()))?;
            
            preset_config
        } else if let Some(config) = request.config {
            config
        } else {
            return Err(BotError::validation("Either preset or config must be provided".to_string()));
        };
        
        // Validate configuration
        self.token_creator.validate_config(&config)?;
        
        // Estimate cost
        let estimated_cost = self.token_creator.estimate_creation_cost(&config)?;
        
        // For demo purposes, simulate token creation
        // In production, this would create actual transactions
        match self.simulate_token_creation(&config).await {
            Ok(result) => {
                info!("Token creation successful: {}", result.mint_address);
                Ok(CreateTokenResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                    estimated_cost_sol: estimated_cost,
                    transaction_to_sign: Some("SIMULATED_TRANSACTION_DATA".to_string()),
                })
            }
            Err(e) => {
                error!("Token creation failed: {}", e);
                Ok(CreateTokenResponse {
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                    estimated_cost_sol: estimated_cost,
                    transaction_to_sign: None,
                })
            }
        }
    }
    
    /// Get token creation guide for creators
    pub async fn get_creation_guide(&self) -> Result<TokenCreationGuide> {
        Ok(TokenCreationGuide {
            steps: vec![
                CreationStep {
                    step_number: 1,
                    title: "Choose Token Type".to_string(),
                    description: "Select a preset or configure custom token features".to_string(),
                    options: vec![
                        "Basic Token - Simple token with no special features".to_string(),
                        "Creator Token - With transfer fees for revenue".to_string(),
                        "Community Token - Interest-bearing with governance".to_string(),
                        "Utility Token - Advanced features with hooks".to_string(),
                        "Meme Token - Optimized for viral spread".to_string(),
                        "Staking Token - Interest rewards for holders".to_string(),
                    ],
                },
                CreationStep {
                    step_number: 2,
                    title: "Configure Token Details".to_string(),
                    description: "Set name, symbol, supply, and metadata".to_string(),
                    options: vec![
                        "Token name (1-100 characters)".to_string(),
                        "Token symbol (1-20 characters)".to_string(),
                        "Decimal places (0-9)".to_string(),
                        "Initial supply".to_string(),
                        "Description and metadata".to_string(),
                    ],
                },
                CreationStep {
                    step_number: 3,
                    title: "Set Revenue Features".to_string(),
                    description: "Configure transfer fees and interest rates".to_string(),
                    options: vec![
                        "Transfer fee percentage (0-100%)".to_string(),
                        "Maximum transfer fee cap".to_string(),
                        "Interest rate for holders".to_string(),
                        "Creator royalty percentage".to_string(),
                    ],
                },
                CreationStep {
                    step_number: 4,
                    title: "Review and Create".to_string(),
                    description: "Verify configuration and sign transaction".to_string(),
                    options: vec![
                        "Review all settings".to_string(),
                        "Check estimated creation cost".to_string(),
                        "Sign transaction with your wallet".to_string(),
                        "Wait for confirmation".to_string(),
                    ],
                },
            ],
            tips: vec![
                "Transfer fees generate ongoing revenue from token trades".to_string(),
                "Interest-bearing tokens encourage long-term holding".to_string(),
                "Rich metadata improves token discoverability".to_string(),
                "Consider your community when setting fee percentages".to_string(),
                "Test with small amounts before full deployment".to_string(),
            ],
            examples: vec![
                TokenExample {
                    name: "Content Creator Token".to_string(),
                    description: "2% transfer fee, rich metadata, memo transfers".to_string(),
                    monthly_revenue: 500.0,
                    use_case: "YouTuber with 100K subscribers".to_string(),
                },
                TokenExample {
                    name: "Community Governance Token".to_string(),
                    description: "5% APY interest, governance features, multisig".to_string(),
                    monthly_revenue: 0.0,
                    use_case: "DAO with community rewards".to_string(),
                },
                TokenExample {
                    name: "Gaming Utility Token".to_string(),
                    description: "Transfer hooks, metadata, burn mechanics".to_string(),
                    monthly_revenue: 2000.0,
                    use_case: "Play-to-earn game with 10K players".to_string(),
                },
            ],
        })
    }
    
    // Helper methods
    
    async fn simulate_token_creation(&self, config: &TokenCreationConfig) -> Result<TokenCreationResult> {
        // In a real implementation, this would:
        // 1. Generate a keypair for the mint
        // 2. Create and send the Token-2022 creation transaction
        // 3. Wait for confirmation
        // 4. Return the actual transaction signature and mint address
        
        // For now, simulate the creation
        use solana_sdk::pubkey::Pubkey;
        use crate::trading::{CreatorInfo, RevenueStream};
        
        let mint_address = Pubkey::new_unique();
        let mut revenue_streams = Vec::new();
        
        if config.enable_transfer_fees {
            revenue_streams.push(RevenueStream {
                source: "Transfer Fees".to_string(),
                description: format!("{}% fee on all transfers", 
                    config.transfer_fee_basis_points.unwrap_or(0) as f64 / 100.0),
                estimated_monthly_amount: 250.0, // Simplified estimate
            });
        }
        
        if config.enable_interest_bearing {
            revenue_streams.push(RevenueStream {
                source: "Interest Generation".to_string(),
                description: format!("{}% APY for token holders", 
                    config.interest_rate_basis_points.unwrap_or(0) as f64 / 100.0),
                estimated_monthly_amount: 100.0,
            });
        }
        
        Ok(TokenCreationResult {
            mint_address,
            transaction_signature: format!("SIM{}", rand::random::<u64>()),
            explorer_url: format!("https://solscan.io/token/{}", mint_address),
            creation_cost_sol: self.token_creator.estimate_creation_cost(config)?,
            enabled_extensions: vec![], // Would be populated with actual extensions
            creator_info: CreatorInfo {
                creator_address: config.creator_address,
                royalty_percentage: config.creator_royalty_percentage,
                estimated_monthly_revenue: revenue_streams.iter().map(|r| r.estimated_monthly_amount).sum(),
                revenue_streams,
            },
        })
    }
}

/// Token creation guide structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCreationGuide {
    pub steps: Vec<CreationStep>,
    pub tips: Vec<String>,
    pub examples: Vec<TokenExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationStep {
    pub step_number: u32,
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenExample {
    pub name: String,
    pub description: String,
    pub monthly_revenue: f64,
    pub use_case: String,
}