use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Solana Blink - A shareable, metadata-rich link for Solana interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaBlink {
    pub blink_id: String,
    pub blink_type: BlinkType,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub action: BlinkAction,
    pub metadata: BlinkMetadata,
    pub security: BlinkSecurity,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub creator: BlinkCreator,
    pub analytics: BlinkAnalytics,
    pub social_preview: SocialPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlinkType {
    TokenSwap,
    TokenTransfer,
    NFTMint,
    NFTPurchase,
    Staking,
    Governance,
    DeFiAction,
    Payment,
    Donation,
    Airdrop,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkAction {
    pub action_type: ActionType,
    pub parameters: HashMap<String, String>,
    pub transaction_template: Option<TransactionTemplate>,
    pub estimated_fee: Option<f64>,
    pub requires_signature: bool,
    pub multi_step: bool,
    pub steps: Vec<ActionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    Swap {
        from_token: String,
        to_token: String,
        amount: f64,
    },
    Transfer {
        token: String,
        recipient: String,
        amount: f64,
    },
    Mint {
        collection: String,
        price: f64,
    },
    Stake {
        validator: String,
        amount: f64,
    },
    Vote {
        proposal_id: String,
        choice: String,
    },
    Custom {
        program_id: String,
        instruction_data: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub step_number: u8,
    pub name: String,
    pub description: String,
    pub transaction: Option<TransactionTemplate>,
    pub validation: Option<StepValidation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepValidation {
    pub validation_type: ValidationType,
    pub expected_result: String,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    BalanceCheck,
    TokenOwnership,
    ProgramState,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTemplate {
    pub program_id: String,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
    pub compute_units: Option<u32>,
    pub priority_fee: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkMetadata {
    pub version: String,
    pub protocol: String,
    pub network: SolanaNetwork,
    pub tags: Vec<String>,
    pub category: String,
    pub language: String,
    pub custom_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SolanaNetwork {
    Mainnet,
    Devnet,
    Testnet,
    Localnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkSecurity {
    pub verified: bool,
    pub audit_status: AuditStatus,
    pub risk_level: RiskLevel,
    pub warnings: Vec<String>,
    pub requires_approval: bool,
    pub max_uses: Option<u32>,
    pub allowed_wallets: Option<Vec<String>>,
    pub blocked_regions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditStatus {
    NotAudited,
    InProgress,
    Audited,
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkCreator {
    pub wallet_address: String,
    pub username: Option<String>,
    pub verified: bool,
    pub reputation_score: Option<f64>,
    pub created_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkAnalytics {
    pub views: u64,
    pub clicks: u64,
    pub executions: u64,
    pub success_rate: f64,
    pub average_execution_time: f64,
    pub total_volume: f64,
    pub unique_users: u32,
    pub referrals: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPreview {
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
    pub twitter_card: TwitterCard,
    pub open_graph: OpenGraphData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterCard {
    pub card_type: String,
    pub site: Option<String>,
    pub creator: Option<String>,
    pub image_alt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenGraphData {
    pub og_type: String,
    pub og_url: String,
    pub og_title: String,
    pub og_description: String,
    pub og_image: Option<String>,
    pub og_site_name: String,
}

/// Blink execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkExecutionResult {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub gas_used: Option<u64>,
    pub outputs: HashMap<String, String>,
}

/// Blink share configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkShareConfig {
    pub platform: SharePlatform,
    pub custom_message: Option<String>,
    pub include_preview: bool,
    pub track_clicks: bool,
    pub utm_params: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SharePlatform {
    Twitter,
    Telegram,
    Discord,
    WhatsApp,
    Email,
    SMS,
    QRCode,
    Direct,
}

/// Blink validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkValidation {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Compressed Blink for URL sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedBlink {
    pub id: String,
    pub v: u8, // version
    pub t: String, // type
    pub a: String, // action (base64 encoded)
    pub s: Option<String>, // signature
}

impl SolanaBlink {
    /// Generate a unique ID for the blink
    pub fn generate_id() -> String {
        format!("blink_{}", uuid::Uuid::new_v4())
    }
    
    /// Create a shareable URL for the blink
    pub fn to_url(&self, base_url: &str) -> String {
        let compressed = self.compress();
        format!("{}/blink/{}", base_url, compressed.to_base64())
    }
    
    /// Compress the blink for URL sharing
    pub fn compress(&self) -> CompressedBlink {
        CompressedBlink {
            id: self.blink_id.clone(),
            v: 1,
            t: format!("{:?}", self.blink_type),
            a: base64::encode(serde_json::to_string(&self.action).unwrap_or_default()),
            s: None, // Would add signature in production
        }
    }
    
    /// Validate the blink
    pub fn validate(&self) -> BlinkValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        
        // Check required fields
        if self.title.is_empty() {
            errors.push(ValidationError {
                field: "title".to_string(),
                message: "Title is required".to_string(),
                severity: ErrorSeverity::High,
            });
        }
        
        if self.description.len() > 500 {
            warnings.push(ValidationWarning {
                field: "description".to_string(),
                message: "Description is too long for optimal sharing".to_string(),
            });
        }
        
        // Check expiration
        if let Some(expires) = self.expires_at {
            if expires < Utc::now() {
                errors.push(ValidationError {
                    field: "expires_at".to_string(),
                    message: "Blink has expired".to_string(),
                    severity: ErrorSeverity::Critical,
                });
            }
        }
        
        // Add suggestions
        if self.icon_url.is_none() {
            suggestions.push("Add an icon for better visibility".to_string());
        }
        
        if self.social_preview.image_url.is_none() {
            suggestions.push("Add a preview image for social sharing".to_string());
        }
        
        BlinkValidation {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }
}

impl CompressedBlink {
    /// Convert to base64 for URL
    pub fn to_base64(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        base64::encode_config(json, base64::URL_SAFE_NO_PAD)
    }
    
    /// Parse from base64 URL parameter
    pub fn from_base64(data: &str) -> Result<Self, String> {
        let decoded = base64::decode_config(data, base64::URL_SAFE_NO_PAD)
            .map_err(|e| format!("Failed to decode base64: {}", e))?;
        
        let json = String::from_utf8(decoded)
            .map_err(|e| format!("Invalid UTF-8: {}", e))?;
        
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }
}