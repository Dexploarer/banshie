use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{Utc, Duration};
use tracing::{info, warn, error, debug};

use super::types::*;
use super::providers::{goplus::GoPlusProvider, rugcheck::RugCheckProvider};
use crate::errors::BotError;

/// Cache entry for security analysis
struct CachedAnalysis {
    analysis: SecurityAnalysis,
    cached_at: chrono::DateTime<Utc>,
}

/// Comprehensive LARP (Liquidity And Rug Pull) checker
pub struct LarpChecker {
    goplus_provider: GoPlusProvider,
    rugcheck_provider: RugCheckProvider,
    cache: Arc<RwLock<HashMap<String, CachedAnalysis>>>,
    cache_ttl: Duration,
}

impl LarpChecker {
    pub fn new(goplus_api_key: Option<String>) -> Self {
        Self {
            goplus_provider: GoPlusProvider::new(goplus_api_key),
            rugcheck_provider: RugCheckProvider::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::minutes(5),
        }
    }
    
    /// Perform comprehensive security analysis on a token
    pub async fn analyze_token(&self, token_address: &str) -> Result<SecurityAnalysis> {
        info!("Starting LARP analysis for token: {}", token_address);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(token_address) {
                let age = Utc::now().signed_duration_since(cached.cached_at);
                if age < self.cache_ttl {
                    debug!("Returning cached analysis for {}", token_address);
                    return Ok(cached.analysis.clone());
                }
            }
        }
        
        // Try multiple providers and combine results
        let mut combined_analysis = None;
        let mut data_sources = Vec::new();
        
        // Try GoPlus first (primary provider)
        match self.goplus_provider.check_token_security(token_address).await {
            Ok(analysis) => {
                info!("GoPlus analysis successful for {}", token_address);
                combined_analysis = Some(analysis);
                data_sources.push("GoPlus Security".to_string());
            }
            Err(e) => {
                warn!("GoPlus analysis failed for {}: {}", token_address, e);
            }
        }
        
        // Try RugCheck as backup or additional validation
        match self.rugcheck_provider.check_token(token_address).await {
            Ok(rugcheck_analysis) => {
                info!("RugCheck analysis successful for {}", token_address);
                data_sources.push("RugCheck".to_string());
                
                if let Some(ref mut analysis) = combined_analysis {
                    // Merge results - take the more conservative score
                    analysis.risk_score = analysis.risk_score.min(rugcheck_analysis.risk_score);
                    
                    // Combine warnings
                    for warning in rugcheck_analysis.warnings {
                        if !analysis.warnings.iter().any(|w| w.message == warning.message) {
                            analysis.warnings.push(warning);
                        }
                    }
                    
                    // Combine passed checks
                    for check in rugcheck_analysis.passed_checks {
                        if !analysis.passed_checks.contains(&check) {
                            analysis.passed_checks.push(check);
                        }
                    }
                    
                    // Update data sources
                    analysis.data_sources = data_sources.clone();
                } else {
                    combined_analysis = Some(rugcheck_analysis);
                }
            }
            Err(e) => {
                warn!("RugCheck analysis failed for {}: {}", token_address, e);
            }
        }
        
        // If no providers succeeded, return error
        let mut final_analysis = combined_analysis
            .ok_or_else(|| BotError::external_api("All security providers failed"))?;
        
        // Add additional analysis
        self.perform_additional_checks(&mut final_analysis).await;
        
        // Generate final recommendations
        final_analysis.recommendations = self.generate_recommendations(&final_analysis);
        
        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                token_address.to_string(),
                CachedAnalysis {
                    analysis: final_analysis.clone(),
                    cached_at: Utc::now(),
                },
            );
        }
        
        info!(
            "LARP analysis complete for {}: Score {}/100, Risk Level: {:?}",
            token_address, final_analysis.risk_score, final_analysis.risk_level
        );
        
        Ok(final_analysis)
    }
    
    /// Perform additional security checks
    async fn perform_additional_checks(&self, analysis: &mut SecurityAnalysis) {
        // Check for common scam patterns
        
        // 1. Check if liquidity is too low
        if analysis.liquidity_usd < 5000.0 && analysis.liquidity_usd > 0.0 {
            if !analysis.warnings.iter().any(|w| w.category == WarningCategory::Liquidity) {
                analysis.warnings.push(SecurityWarning {
                    severity: WarningSeverity::High,
                    category: WarningCategory::Liquidity,
                    message: format!("Very low liquidity: ${:.2}", analysis.liquidity_usd),
                    details: Some("Low liquidity increases risk of price manipulation".to_string()),
                });
                analysis.risk_score = analysis.risk_score.saturating_sub(15);
            }
        }
        
        // 2. Check token age
        if analysis.token_age_hours < 24.0 && analysis.token_age_hours > 0.0 {
            analysis.warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                category: WarningCategory::Age,
                message: "Brand new token (< 24 hours)".to_string(),
                details: Some("New tokens have higher risk of rug pulls".to_string()),
            });
            analysis.risk_score = analysis.risk_score.saturating_sub(10);
        }
        
        // 3. Check holder concentration
        let top_10_percent: f64 = analysis.top_holders
            .iter()
            .take(10)
            .map(|h| h.percentage)
            .sum();
        
        if top_10_percent > 70.0 {
            analysis.warnings.push(SecurityWarning {
                severity: WarningSeverity::High,
                category: WarningCategory::Distribution,
                message: format!("Top 10 holders own {:.1}% of supply", top_10_percent),
                details: Some("High concentration increases manipulation risk".to_string()),
            });
            analysis.risk_score = analysis.risk_score.saturating_sub(20);
        }
        
        // 4. Check for suspicious patterns
        if analysis.holder_count < 50 && analysis.holder_count > 0 {
            analysis.warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                category: WarningCategory::Distribution,
                message: format!("Only {} holders", analysis.holder_count),
                details: Some("Very few holders suggests limited adoption".to_string()),
            });
            analysis.risk_score = analysis.risk_score.saturating_sub(10);
        }
        
        // Update risk level
        analysis.risk_level = SecurityAnalysis::calculate_risk_level(analysis.risk_score);
    }
    
    /// Generate recommendations based on analysis
    fn generate_recommendations(&self, analysis: &SecurityAnalysis) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Base recommendation on risk level
        recommendations.push(analysis.generate_recommendation());
        
        // Specific recommendations based on warnings
        if analysis.is_honeypot {
            recommendations.push("⛔ DO NOT BUY - This is a honeypot".to_string());
            return recommendations;
        }
        
        if analysis.liquidity_usd < 10000.0 {
            recommendations.push("💧 Use very small position due to low liquidity".to_string());
        }
        
        if analysis.token_age_hours < 168.0 { // Less than 1 week
            recommendations.push("⏰ Wait for token to mature before large positions".to_string());
        }
        
        if let Some(freeze) = &analysis.freeze_authority {
            if !freeze.is_empty() {
                recommendations.push("🔒 Be aware: Freeze authority could halt trading".to_string());
            }
        }
        
        if let Some(mint) = &analysis.mint_authority {
            if !mint.is_empty() {
                recommendations.push("🏭 Caution: New tokens can be minted".to_string());
            }
        }
        
        // Risk-based recommendations
        match analysis.risk_level {
            RiskLevel::VeryLow | RiskLevel::Low => {
                recommendations.push("✅ Appears safe for normal trading".to_string());
                recommendations.push("💡 Still use proper risk management".to_string());
            }
            RiskLevel::Medium => {
                recommendations.push("⚠️ Trade with caution".to_string());
                recommendations.push("📊 Consider using stop-loss orders".to_string());
                recommendations.push("💰 Limit position to 1-2% of portfolio".to_string());
            }
            RiskLevel::High => {
                recommendations.push("🚨 High risk - only for experienced traders".to_string());
                recommendations.push("🛡️ Use tight stop-loss if trading".to_string());
                recommendations.push("💵 Maximum 0.5% of portfolio recommended".to_string());
            }
            RiskLevel::VeryHigh => {
                recommendations.push("🚫 Extremely risky - consider avoiding".to_string());
                recommendations.push("🔍 Do extensive research before any trade".to_string());
            }
        }
        
        recommendations
    }
    
    /// Format analysis for display
    pub fn format_analysis(&self, analysis: &SecurityAnalysis) -> String {
        let mut output = format!(
            "🛡️ **Security Analysis**\n\n\
            Token: `{}`\n\
            Symbol: {}\n\
            Name: {}\n\n\
            **Risk Score: {}/100** {}\n\
            **Risk Level: {:?}**\n\n",
            analysis.token_address,
            analysis.token_symbol,
            analysis.token_name,
            analysis.risk_score,
            analysis.get_risk_emoji(),
            analysis.risk_level
        );
        
        // Passed checks
        if !analysis.passed_checks.is_empty() {
            output.push_str("✅ **Passed Checks:**\n");
            for check in &analysis.passed_checks {
                output.push_str(&format!("• {}\n", check));
            }
            output.push('\n');
        }
        
        // Failed checks
        if !analysis.failed_checks.is_empty() {
            output.push_str("❌ **Failed Checks:**\n");
            for check in &analysis.failed_checks {
                output.push_str(&format!("• {}\n", check));
            }
            output.push('\n');
        }
        
        // Warnings
        if !analysis.warnings.is_empty() {
            output.push_str("⚠️ **Warnings:**\n");
            for warning in &analysis.warnings {
                let severity_emoji = match warning.severity {
                    WarningSeverity::Critical => "🔴",
                    WarningSeverity::High => "🟠",
                    WarningSeverity::Medium => "🟡",
                    WarningSeverity::Low => "🟢",
                };
                output.push_str(&format!("{} {}\n", severity_emoji, warning.message));
                if let Some(details) = &warning.details {
                    output.push_str(&format!("   {}\n", details));
                }
            }
            output.push('\n');
        }
        
        // Token details
        output.push_str(&format!(
            "📊 **Token Details:**\n\
            • Holders: {}\n\
            • Liquidity: ${:.2}\n\
            • Volume 24h: ${:.2}\n\
            • Age: {:.1} hours\n",
            analysis.holder_count,
            analysis.liquidity_usd,
            analysis.volume_24h,
            analysis.token_age_hours
        ));
        
        if analysis.freeze_authority.is_some() {
            output.push_str("• ⚠️ Freeze Authority: Enabled\n");
        }
        if analysis.mint_authority.is_some() {
            output.push_str("• ⚠️ Mint Authority: Enabled\n");
        }
        output.push('\n');
        
        // Recommendations
        output.push_str("💡 **Recommendations:**\n");
        for rec in &analysis.recommendations {
            output.push_str(&format!("• {}\n", rec));
        }
        
        // Data sources
        output.push_str(&format!(
            "\n📌 *Data from: {}*\n",
            analysis.data_sources.join(", ")
        ));
        
        output
    }
    
    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Security analysis cache cleared");
    }
}