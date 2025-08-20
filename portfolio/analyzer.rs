use anyhow::Result;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, debug};

use super::types::*;

/// Analyzes portfolio data and provides insights
pub struct PortfolioAnalyzer;

impl PortfolioAnalyzer {
    /// Analyze portfolio for insights and recommendations
    pub fn analyze_portfolio(&self, portfolio: &Portfolio) -> PortfolioAnalysis {
        let diversification = self.analyze_diversification(&portfolio.holdings);
        let risk_metrics = self.calculate_risk_metrics(&portfolio.holdings);
        let allocation = self.analyze_allocation(&portfolio.holdings);
        let recommendations = self.generate_recommendations(portfolio);
        
        PortfolioAnalysis {
            wallet_address: portfolio.wallet_address.clone(),
            total_value_usd: portfolio.total_value_usd,
            diversification,
            risk_metrics,
            allocation,
            recommendations,
            analysis_timestamp: Utc::now(),
        }
    }
    
    /// Analyze portfolio diversification
    fn analyze_diversification(&self, holdings: &[TokenHolding]) -> DiversificationMetrics {
        let total_value = holdings.iter().map(|h| h.value_usd).sum::<f64>();
        
        if total_value == 0.0 {
            return DiversificationMetrics {
                herfindahl_index: 0.0,
                effective_holdings: 0.0,
                largest_position_percentage: 0.0,
                top_5_concentration: 0.0,
                diversification_score: 0.0,
                is_well_diversified: false,
            };
        }
        
        // Calculate Herfindahl-Hirschman Index
        let hhi = holdings
            .iter()
            .map(|h| {
                let weight = h.value_usd / total_value;
                weight * weight
            })
            .sum::<f64>();
        
        // Effective number of holdings (1/HHI)
        let effective_holdings = if hhi > 0.0 { 1.0 / hhi } else { 0.0 };
        
        // Largest position percentage
        let largest_position_percentage = holdings
            .iter()
            .map(|h| (h.value_usd / total_value) * 100.0)
            .fold(0.0, f64::max);
        
        // Top 5 concentration
        let mut sorted_holdings = holdings.to_vec();
        sorted_holdings.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap());
        let top_5_concentration = sorted_holdings
            .iter()
            .take(5)
            .map(|h| h.value_usd / total_value)
            .sum::<f64>() * 100.0;
        
        // Diversification score (0-100)
        let diversification_score = match effective_holdings {
            x if x >= 10.0 => 100.0,
            x if x >= 5.0 => 70.0 + (x - 5.0) * 6.0,
            x if x >= 3.0 => 40.0 + (x - 3.0) * 15.0,
            x if x >= 1.0 => x * 20.0,
            _ => 0.0,
        };
        
        let is_well_diversified = effective_holdings >= 5.0 && largest_position_percentage < 50.0;
        
        DiversificationMetrics {
            herfindahl_index: hhi,
            effective_holdings,
            largest_position_percentage,
            top_5_concentration,
            diversification_score,
            is_well_diversified,
        }
    }
    
    /// Calculate risk metrics
    fn calculate_risk_metrics(&self, holdings: &[TokenHolding]) -> RiskMetrics {
        let total_value = holdings.iter().map(|h| h.value_usd).sum::<f64>();
        
        // Calculate volatility score based on token characteristics
        let mut volatility_score = 0.0;
        let mut verified_percentage = 0.0;
        let mut small_cap_exposure = 0.0;
        
        for holding in holdings {
            let weight = if total_value > 0.0 { holding.value_usd / total_value } else { 0.0 };
            
            // Verified tokens are less risky
            if holding.is_verified {
                verified_percentage += weight * 100.0;
            }
            
            // Small cap tokens (value < $1000) are riskier
            if holding.value_usd < 1000.0 {
                small_cap_exposure += weight * 100.0;
                volatility_score += weight * 0.8; // High volatility
            } else if holding.value_usd < 10000.0 {
                volatility_score += weight * 0.5; // Medium volatility
            } else {
                volatility_score += weight * 0.2; // Low volatility
            }
        }
        
        // Overall risk score (0-100, higher = riskier)
        let overall_risk_score = (volatility_score * 100.0)
            + if verified_percentage < 50.0 { 20.0 } else { 0.0 }
            + if small_cap_exposure > 50.0 { 30.0 } else { 0.0 };
        
        let risk_level = match overall_risk_score {
            x if x >= 80.0 => RiskLevel::VeryHigh,
            x if x >= 60.0 => RiskLevel::High,
            x if x >= 40.0 => RiskLevel::Medium,
            x if x >= 20.0 => RiskLevel::Low,
            _ => RiskLevel::VeryLow,
        };
        
        RiskMetrics {
            overall_risk_score,
            risk_level,
            volatility_score: volatility_score * 100.0,
            verified_percentage,
            small_cap_exposure,
            concentration_risk: if holdings.len() > 0 {
                100.0 / holdings.len() as f64
            } else {
                100.0
            },
        }
    }
    
    /// Analyze allocation breakdown
    fn analyze_allocation(&self, holdings: &[TokenHolding]) -> AllocationBreakdown {
        let total_value = holdings.iter().map(|h| h.value_usd).sum::<f64>();
        
        let mut sol_allocation = 0.0;
        let mut stablecoin_allocation = 0.0;
        let mut defi_allocation = 0.0;
        let mut meme_allocation = 0.0;
        let mut other_allocation = 0.0;
        
        for holding in holdings {
            let percentage = if total_value > 0.0 {
                (holding.value_usd / total_value) * 100.0
            } else {
                0.0
            };
            
            match holding.symbol.as_str() {
                "SOL" => sol_allocation += percentage,
                "USDC" | "USDT" | "DAI" | "FRAX" => stablecoin_allocation += percentage,
                "JUP" | "RAY" | "SRM" | "ORCA" | "MNGO" => defi_allocation += percentage,
                "BONK" | "WIF" | "PEPE" | "SHIB" => meme_allocation += percentage,
                _ => other_allocation += percentage,
            }
        }
        
        AllocationBreakdown {
            sol_percentage: sol_allocation,
            stablecoin_percentage: stablecoin_allocation,
            defi_percentage: defi_allocation,
            meme_percentage: meme_allocation,
            other_percentage: other_allocation,
        }
    }
    
    /// Generate recommendations based on analysis
    fn generate_recommendations(&self, portfolio: &Portfolio) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();
        
        // Diversification recommendations
        if portfolio.holdings.len() < 3 {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Diversification,
                priority: RecommendationPriority::High,
                title: "Increase Diversification".to_string(),
                description: "Consider adding more tokens to reduce concentration risk. Aim for at least 5-10 different holdings.".to_string(),
                action: Some("Add 2-3 more token positions".to_string()),
            });
        }
        
        // Check for over-concentration
        if let Some(largest) = portfolio.holdings.iter().max_by(|a, b| a.value_usd.partial_cmp(&b.value_usd).unwrap()) {
            let concentration = (largest.value_usd / portfolio.total_value_usd) * 100.0;
            if concentration > 70.0 {
                recommendations.push(Recommendation {
                    category: RecommendationCategory::RiskManagement,
                    priority: RecommendationPriority::High,
                    title: "Reduce Concentration Risk".to_string(),
                    description: format!("{} represents {:.1}% of your portfolio. Consider reducing this position.", largest.symbol, concentration),
                    action: Some(format!("Sell some {} to rebalance", largest.symbol)),
                });
            }
        }
        
        // Stablecoin recommendations
        let stablecoin_value = portfolio.holdings
            .iter()
            .filter(|h| matches!(h.symbol.as_str(), "USDC" | "USDT" | "DAI"))
            .map(|h| h.value_usd)
            .sum::<f64>();
        
        let stablecoin_percentage = (stablecoin_value / portfolio.total_value_usd) * 100.0;
        
        if stablecoin_percentage < 10.0 && portfolio.total_value_usd > 1000.0 {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Allocation,
                priority: RecommendationPriority::Medium,
                title: "Consider Adding Stablecoins".to_string(),
                description: "Having 10-20% in stablecoins can provide stability and dry powder for opportunities.".to_string(),
                action: Some("Add USDC position".to_string()),
            });
        }
        
        // Performance recommendations
        if portfolio.performance.pnl_24h_percentage < -10.0 {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Performance,
                priority: RecommendationPriority::Medium,
                title: "Portfolio Down 24h".to_string(),
                description: "Your portfolio is down significantly today. Consider reviewing your positions.".to_string(),
                action: Some("Review worst performers".to_string()),
            });
        }
        
        // Small holding cleanup
        let small_holdings = portfolio.holdings
            .iter()
            .filter(|h| h.value_usd < 10.0 && h.value_usd > 0.0)
            .count();
        
        if small_holdings > 5 {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Optimization,
                priority: RecommendationPriority::Low,
                title: "Clean Up Small Holdings".to_string(),
                description: format!("You have {} small positions worth less than $10. Consider consolidating.", small_holdings),
                action: Some("Sell dust positions".to_string()),
            });
        }
        
        recommendations
    }
}

/// Portfolio analysis result
#[derive(Debug, Clone)]
pub struct PortfolioAnalysis {
    pub wallet_address: String,
    pub total_value_usd: f64,
    pub diversification: DiversificationMetrics,
    pub risk_metrics: RiskMetrics,
    pub allocation: AllocationBreakdown,
    pub recommendations: Vec<Recommendation>,
    pub analysis_timestamp: DateTime<Utc>,
}

/// Diversification metrics
#[derive(Debug, Clone)]
pub struct DiversificationMetrics {
    pub herfindahl_index: f64,
    pub effective_holdings: f64,
    pub largest_position_percentage: f64,
    pub top_5_concentration: f64,
    pub diversification_score: f64,
    pub is_well_diversified: bool,
}

/// Risk assessment metrics
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub overall_risk_score: f64,
    pub risk_level: RiskLevel,
    pub volatility_score: f64,
    pub verified_percentage: f64,
    pub small_cap_exposure: f64,
    pub concentration_risk: f64,
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Allocation breakdown by category
#[derive(Debug, Clone)]
pub struct AllocationBreakdown {
    pub sol_percentage: f64,
    pub stablecoin_percentage: f64,
    pub defi_percentage: f64,
    pub meme_percentage: f64,
    pub other_percentage: f64,
}

/// Recommendation for portfolio improvement
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub category: RecommendationCategory,
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RecommendationCategory {
    Diversification,
    RiskManagement,
    Allocation,
    Performance,
    Optimization,
}

#[derive(Debug, Clone)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}