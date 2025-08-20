use anyhow::Result;
use qrcode::{QrCode, render::svg};
use std::collections::HashMap;
use tracing::{info, debug};
use urlencoding;

use super::types::*;

/// Handles sharing of Solana Blinks across different platforms
pub struct BlinkSharing {
    base_url: String,
    tracking_enabled: bool,
}

impl BlinkSharing {
    pub fn new(base_url: String, tracking_enabled: bool) -> Self {
        Self {
            base_url,
            tracking_enabled,
        }
    }
    
    /// Generate a shareable URL for a blink
    pub fn generate_share_url(
        &self,
        blink: &SolanaBlink,
        platform: SharePlatform,
        custom_params: Option<HashMap<String, String>>,
    ) -> String {
        let blink_url = blink.to_url(&self.base_url);
        
        // Add tracking parameters if enabled
        let tracked_url = if self.tracking_enabled {
            self.add_tracking_params(&blink_url, &platform, custom_params)
        } else {
            blink_url
        };
        
        // Generate platform-specific share URL
        match platform {
            SharePlatform::Twitter => self.generate_twitter_share(&tracked_url, blink),
            SharePlatform::Telegram => self.generate_telegram_share(&tracked_url, blink),
            SharePlatform::Discord => self.generate_discord_share(&tracked_url, blink),
            SharePlatform::WhatsApp => self.generate_whatsapp_share(&tracked_url, blink),
            SharePlatform::Email => self.generate_email_share(&tracked_url, blink),
            SharePlatform::SMS => self.generate_sms_share(&tracked_url, blink),
            SharePlatform::QRCode => tracked_url, // QR code uses direct URL
            SharePlatform::Direct => tracked_url,
        }
    }
    
    /// Add tracking parameters to URL
    fn add_tracking_params(
        &self,
        url: &str,
        platform: &SharePlatform,
        custom_params: Option<HashMap<String, String>>,
    ) -> String {
        let mut params = vec![
            format!("utm_source={:?}", platform).to_lowercase(),
            format!("utm_medium=blink"),
            format!("utm_campaign=share"),
        ];
        
        if let Some(custom) = custom_params {
            for (key, value) in custom {
                params.push(format!("{}={}", key, urlencoding::encode(&value)));
            }
        }
        
        if url.contains('?') {
            format!("{}&{}", url, params.join("&"))
        } else {
            format!("{}?{}", url, params.join("&"))
        }
    }
    
    /// Generate Twitter share URL
    fn generate_twitter_share(&self, url: &str, blink: &SolanaBlink) -> String {
        let text = format!(
            "{} - {} #Solana #DeFi",
            blink.title,
            blink.description
        );
        
        format!(
            "https://twitter.com/intent/tweet?text={}&url={}",
            urlencoding::encode(&text),
            urlencoding::encode(url)
        )
    }
    
    /// Generate Telegram share URL
    fn generate_telegram_share(&self, url: &str, blink: &SolanaBlink) -> String {
        let text = format!("{}\n\n{}", blink.title, blink.description);
        
        format!(
            "https://t.me/share/url?url={}&text={}",
            urlencoding::encode(url),
            urlencoding::encode(&text)
        )
    }
    
    /// Generate Discord share message
    fn generate_discord_share(&self, url: &str, blink: &SolanaBlink) -> String {
        format!(
            "**{}**\n{}\n\n🔗 {}",
            blink.title,
            blink.description,
            url
        )
    }
    
    /// Generate WhatsApp share URL
    fn generate_whatsapp_share(&self, url: &str, blink: &SolanaBlink) -> String {
        let text = format!(
            "*{}*\n{}\n\n{}",
            blink.title,
            blink.description,
            url
        );
        
        format!(
            "https://wa.me/?text={}",
            urlencoding::encode(&text)
        )
    }
    
    /// Generate email share URL
    fn generate_email_share(&self, url: &str, blink: &SolanaBlink) -> String {
        let subject = format!("Solana Blink: {}", blink.title);
        let body = format!(
            "{}\n\n{}\n\nClick here to execute: {}",
            blink.title,
            blink.description,
            url
        );
        
        format!(
            "mailto:?subject={}&body={}",
            urlencoding::encode(&subject),
            urlencoding::encode(&body)
        )
    }
    
    /// Generate SMS share text
    fn generate_sms_share(&self, url: &str, blink: &SolanaBlink) -> String {
        format!(
            "sms:?body={}",
            urlencoding::encode(&format!(
                "{} - {} {}",
                blink.title,
                &blink.description[..50.min(blink.description.len())],
                url
            ))
        )
    }
    
    /// Generate QR code for a blink
    pub fn generate_qr_code(&self, blink: &SolanaBlink) -> Result<String> {
        let url = blink.to_url(&self.base_url);
        
        let code = QrCode::new(&url)?;
        let svg = code.render::<svg::Color>()
            .min_dimensions(200, 200)
            .max_dimensions(400, 400)
            .build();
        
        Ok(svg)
    }
    
    /// Generate a shareable card/image for social media
    pub fn generate_social_card(&self, blink: &SolanaBlink) -> ShareCard {
        ShareCard {
            title: blink.title.clone(),
            description: blink.description.clone(),
            image_url: blink.icon_url.clone(),
            blink_type: format!("{:?}", blink.blink_type),
            network: format!("{:?}", blink.metadata.network),
            expires_in: blink.expires_at.map(|e| {
                let duration = e.signed_duration_since(chrono::Utc::now());
                if duration.num_days() > 0 {
                    format!("{} days", duration.num_days())
                } else if duration.num_hours() > 0 {
                    format!("{} hours", duration.num_hours())
                } else {
                    format!("{} minutes", duration.num_minutes())
                }
            }),
            action_preview: self.format_action_preview(&blink.action),
            security_badge: self.get_security_badge(&blink.security),
        }
    }
    
    /// Format action preview for display
    fn format_action_preview(&self, action: &BlinkAction) -> String {
        match &action.action_type {
            ActionType::Swap { from_token, to_token, amount } => {
                format!("Swap {} tokens for {}", amount, to_token)
            }
            ActionType::Transfer { token, recipient, amount } => {
                format!("Send {} tokens to {}", amount, &recipient[..8])
            }
            ActionType::Mint { collection, price } => {
                format!("Mint NFT for {} SOL", price)
            }
            ActionType::Stake { validator, amount } => {
                format!("Stake {} SOL", amount)
            }
            ActionType::Vote { proposal_id, choice } => {
                format!("Vote {} on proposal", choice)
            }
            ActionType::Custom { .. } => {
                "Execute custom action".to_string()
            }
        }
    }
    
    /// Get security badge for display
    fn get_security_badge(&self, security: &BlinkSecurity) -> String {
        if security.verified {
            "✅ Verified".to_string()
        } else if security.audit_status == AuditStatus::Audited {
            "🔍 Audited".to_string()
        } else {
            match security.risk_level {
                RiskLevel::Low => "🟢 Low Risk".to_string(),
                RiskLevel::Medium => "🟡 Medium Risk".to_string(),
                RiskLevel::High => "🟠 High Risk".to_string(),
                RiskLevel::Critical => "🔴 Critical Risk".to_string(),
            }
        }
    }
    
    /// Generate shareable text for different platforms
    pub fn generate_share_text(
        &self,
        blink: &SolanaBlink,
        platform: SharePlatform,
    ) -> String {
        match platform {
            SharePlatform::Twitter => {
                format!(
                    "🔗 {} - One-click Solana action!\n\n{}\n\n#Solana #Blinks #DeFi",
                    blink.title,
                    if blink.description.len() > 100 {
                        format!("{}...", &blink.description[..100])
                    } else {
                        blink.description.clone()
                    }
                )
            }
            SharePlatform::Telegram | SharePlatform::WhatsApp => {
                format!(
                    "🔗 *{}*\n\n{}\n\n⚡ Execute with one click!\n🔒 {}",
                    blink.title,
                    blink.description,
                    self.get_security_badge(&blink.security)
                )
            }
            SharePlatform::Discord => {
                format!(
                    "**__{}__**\n\n{}\n\n**Action:** {}\n**Security:** {}\n**Network:** {:?}",
                    blink.title,
                    blink.description,
                    self.format_action_preview(&blink.action),
                    self.get_security_badge(&blink.security),
                    blink.metadata.network
                )
            }
            _ => format!("{} - {}", blink.title, blink.description),
        }
    }
    
    /// Create a shortened URL for better sharing
    pub async fn create_short_url(&self, blink: &SolanaBlink) -> Result<String> {
        // In production, this would use a URL shortening service
        // For now, return a simulated short URL
        let short_id = &blink.blink_id[..8];
        Ok(format!("{}/b/{}", self.base_url, short_id))
    }
}

/// Shareable card representation
#[derive(Debug, Clone)]
pub struct ShareCard {
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
    pub blink_type: String,
    pub network: String,
    pub expires_in: Option<String>,
    pub action_preview: String,
    pub security_badge: String,
}

/// Analytics tracker for shared blinks
pub struct ShareAnalytics {
    shares_by_platform: HashMap<SharePlatform, u64>,
    clicks_by_platform: HashMap<SharePlatform, u64>,
    conversions_by_platform: HashMap<SharePlatform, u64>,
}

impl ShareAnalytics {
    pub fn new() -> Self {
        Self {
            shares_by_platform: HashMap::new(),
            clicks_by_platform: HashMap::new(),
            conversions_by_platform: HashMap::new(),
        }
    }
    
    /// Track a share event
    pub fn track_share(&mut self, platform: SharePlatform) {
        *self.shares_by_platform.entry(platform).or_insert(0) += 1;
    }
    
    /// Track a click event
    pub fn track_click(&mut self, platform: SharePlatform) {
        *self.clicks_by_platform.entry(platform).or_insert(0) += 1;
    }
    
    /// Track a conversion (execution)
    pub fn track_conversion(&mut self, platform: SharePlatform) {
        *self.conversions_by_platform.entry(platform).or_insert(0) += 1;
    }
    
    /// Get analytics summary
    pub fn get_summary(&self) -> AnalyticsSummary {
        AnalyticsSummary {
            total_shares: self.shares_by_platform.values().sum(),
            total_clicks: self.clicks_by_platform.values().sum(),
            total_conversions: self.conversions_by_platform.values().sum(),
            conversion_rate: if self.clicks_by_platform.values().sum::<u64>() > 0 {
                (self.conversions_by_platform.values().sum::<u64>() as f64 / 
                 self.clicks_by_platform.values().sum::<u64>() as f64) * 100.0
            } else {
                0.0
            },
            top_platform: self.shares_by_platform
                .iter()
                .max_by_key(|(_, v)| *v)
                .map(|(k, _)| k.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalyticsSummary {
    pub total_shares: u64,
    pub total_clicks: u64,
    pub total_conversions: u64,
    pub conversion_rate: f64,
    pub top_platform: Option<SharePlatform>,
}