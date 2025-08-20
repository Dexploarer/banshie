use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use crate::errors::{BotError, Result};
use crate::api::jupiter_price_v3::{JupiterPriceV3Client, PriceDataV3};
use crate::trading::dca::{DCAStrategy, MarketConditions, ExecutionReason};
use crate::telemetry::TelemetryService;

/// Risk-based DCA strategy manager
#[derive(Clone)]
pub struct RiskBasedDCAManager {
    price_client: Arc<JupiterPriceV3Client>,
    telemetry: Option<Arc<TelemetryService>>,
    risk_models: Arc<RwLock<HashMap<String, RiskModel>>>,
    market_regime_detector: Arc<MarketRegimeDetector>,
    volatility_calculator: Arc<VolatilityCalculator>,
    correlation_analyzer: Arc<CorrelationAnalyzer>,
}

/// Risk model for a specific token/strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskModel {
    pub token_mint: String,
    pub model_type: RiskModelType,
    pub parameters: RiskParameters,
    pub historical_data: VecDeque<PricePoint>,
    pub last_updated: DateTime<Utc>,
    pub confidence_score: f64, // 0.0 to 1.0
    pub performance_metrics: RiskModelMetrics,
}

/// Types of risk models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskModelType {
    /// Volatility-based risk adjustment
    VolatilityAdjusted {
        lookback_periods: u32,
        volatility_threshold: f64,
        adjustment_factor: f64,
    },
    /// Value at Risk (VaR) based
    ValueAtRisk {
        confidence_level: f64,
        time_horizon_days: u32,
        max_loss_percentage: f64,
    },
    /// Kelly Criterion optimization
    KellyCriterion {
        win_rate: f64,
        avg_win: f64,
        avg_loss: f64,
        risk_free_rate: f64,
    },
    /// Black-Litterman model
    BlackLitterman {
        market_cap_weights: HashMap<String, f64>,
        confidence_levels: HashMap<String, f64>,
        tau: f64, // Uncertainty parameter
    },
    /// Monte Carlo simulation
    MonteCarlo {
        simulations: u32,
        confidence_interval: f64,
        price_drift: f64,
        volatility: f64,
    },
    /// Market regime adaptive
    RegimeAdaptive {
        bull_multiplier: f64,
        bear_multiplier: f64,
        sideways_multiplier: f64,
        regime_detection_periods: u32,
    },
    /// Correlation-based risk parity
    RiskParity {
        correlation_threshold: f64,
        rebalance_threshold: f64,
        risk_budget: HashMap<String, f64>,
    },
}

/// Risk parameters for calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParameters {
    pub max_position_size: Decimal,
    pub max_drawdown: f64,
    pub volatility_ceiling: f64,
    pub correlation_limit: f64,
    pub liquidity_threshold: Decimal,
    pub stop_loss_percentage: Option<f64>,
    pub take_profit_percentage: Option<f64>,
    pub rebalance_frequency_days: u32,
}

/// Historical price point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub volume: Option<u64>,
    pub returns: Option<f64>, // Period return
    pub volatility: Option<f64>, // Rolling volatility
}

/// Risk model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskModelMetrics {
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub max_drawdown: f64,
    pub volatility: f64,
    pub var_95: Option<f64>, // Value at Risk at 95% confidence
    pub cvar_95: Option<f64>, // Conditional VaR at 95% confidence
    pub calmar_ratio: Option<f64>,
    pub information_ratio: Option<f64>,
    pub tracking_error: Option<f64>,
}

/// Market regime detector
#[derive(Debug)]
pub struct MarketRegimeDetector {
    price_history: RwLock<HashMap<String, VecDeque<PricePoint>>>,
    regime_cache: RwLock<HashMap<String, MarketRegime>>,
}

/// Market regime types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketRegime {
    Bull {
        strength: f64,
        duration_days: u32,
        trend_slope: f64,
    },
    Bear {
        strength: f64,
        duration_days: u32,
        trend_slope: f64,
    },
    Sideways {
        volatility: f64,
        range_bound: (Decimal, Decimal),
    },
    Transition {
        from_regime: Box<MarketRegime>,
        confidence: f64,
    },
}

/// Volatility calculator with multiple models
#[derive(Debug)]
pub struct VolatilityCalculator {
    calculation_cache: RwLock<HashMap<String, VolatilityMetrics>>,
}

/// Volatility calculation results
#[derive(Debug, Clone)]
pub struct VolatilityMetrics {
    pub historical_volatility: f64,
    pub realized_volatility: f64,
    pub implied_volatility: Option<f64>,
    pub garch_forecast: Option<f64>,
    pub ewma_volatility: f64, // Exponentially weighted moving average
    pub parkinson_volatility: Option<f64>, // High-low estimator
    pub garman_klass_volatility: Option<f64>, // OHLC estimator
}

/// Correlation analyzer
#[derive(Debug)]
pub struct CorrelationAnalyzer {
    correlation_matrix: RwLock<HashMap<(String, String), f64>>,
    rolling_correlations: RwLock<HashMap<(String, String), VecDeque<f64>>>,
}

/// Risk-adjusted DCA execution recommendation
#[derive(Debug, Clone)]
pub struct RiskAdjustedRecommendation {
    pub strategy_id: String,
    pub recommended_amount: Decimal,
    pub confidence_level: f64,
    pub risk_score: f64, // 0.0 = low risk, 1.0 = high risk
    pub execution_reason: ExecutionReason,
    pub market_conditions: MarketConditions,
    pub risk_factors: Vec<RiskFactor>,
    pub hedging_suggestions: Vec<HedgingSuggestion>,
}

/// Individual risk factors
#[derive(Debug, Clone, Serialize)]
pub struct RiskFactor {
    pub factor_type: RiskFactorType,
    pub severity: RiskSeverity,
    pub description: String,
    pub mitigation_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum RiskFactorType {
    HighVolatility,
    LowLiquidity,
    HighCorrelation,
    MarketRegimeChange,
    ConcentrationRisk,
    DrawdownRisk,
    TailRisk,
    CounterpartyRisk,
}

#[derive(Debug, Clone, Serialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Hedging suggestions
#[derive(Debug, Clone, Serialize)]
pub struct HedgingSuggestion {
    pub hedge_type: HedgeType,
    pub description: String,
    pub estimated_cost: Option<Decimal>,
    pub effectiveness: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize)]
pub enum HedgeType {
    PositionSizing,
    Diversification,
    StopLoss,
    TakeProfit,
    OptionsHedge,
    FuturesHedge,
    CorrelationHedge,
}

impl RiskBasedDCAManager {
    /// Create new risk-based DCA manager
    pub fn new(
        price_client: Arc<JupiterPriceV3Client>,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("🎯 Initializing risk-based DCA manager");
        
        Self {
            price_client,
            telemetry,
            risk_models: Arc::new(RwLock::new(HashMap::new())),
            market_regime_detector: Arc::new(MarketRegimeDetector {
                price_history: RwLock::new(HashMap::new()),
                regime_cache: RwLock::new(HashMap::new()),
            }),
            volatility_calculator: Arc::new(VolatilityCalculator {
                calculation_cache: RwLock::new(HashMap::new()),
            }),
            correlation_analyzer: Arc::new(CorrelationAnalyzer {
                correlation_matrix: RwLock::new(HashMap::new()),
                rolling_correlations: RwLock::new(HashMap::new()),
            }),
        }
    }
    
    /// Create a risk model for a token
    pub async fn create_risk_model(
        &self,
        token_mint: String,
        model_type: RiskModelType,
    ) -> Result<RiskModel> {
        // Fetch historical price data
        let historical_data = self.fetch_historical_data(&token_mint, 100).await?;
        
        let risk_model = RiskModel {
            token_mint: token_mint.clone(),
            model_type,
            parameters: RiskParameters::default(),
            historical_data,
            last_updated: Utc::now(),
            confidence_score: 0.5, // Initial confidence
            performance_metrics: RiskModelMetrics::default(),
        };
        
        // Calculate initial metrics
        let updated_model = self.update_risk_model_metrics(risk_model).await?;
        
        // Store the model
        let mut models = self.risk_models.write().await;
        models.insert(token_mint.clone(), updated_model.clone());
        
        info!("🎯 Created risk model for token {}", token_mint);
        
        Ok(updated_model)
    }
    
    /// Get risk-adjusted DCA recommendation
    pub async fn get_risk_adjusted_recommendation(
        &self,
        strategy: &DCAStrategy,
    ) -> Result<RiskAdjustedRecommendation> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_trading_span("risk_adjusted_dca", None)
        );
        
        // Get or create risk model
        let risk_model = self.get_or_create_risk_model(&strategy.output_token).await?;
        
        // Detect current market regime
        let market_regime = self.detect_market_regime(&strategy.output_token).await?;
        
        // Calculate current volatility
        let volatility_metrics = self.calculate_volatility(&strategy.output_token).await?;
        
        // Get market conditions
        let market_conditions = self.get_current_market_conditions(&strategy.output_token).await?;
        
        // Calculate risk-adjusted amount
        let base_amount = strategy.amount_per_execution;
        let risk_adjusted_amount = self.calculate_risk_adjusted_amount(
            base_amount,
            &risk_model,
            &market_regime,
            &volatility_metrics,
            &market_conditions,
        ).await?;
        
        // Calculate risk score
        let risk_score = self.calculate_overall_risk_score(
            &risk_model,
            &market_regime,
            &volatility_metrics,
        ).await?;
        
        // Identify risk factors
        let risk_factors = self.identify_risk_factors(
            &risk_model,
            &market_regime,
            &volatility_metrics,
            &market_conditions,
        ).await?;
        
        // Generate hedging suggestions
        let hedging_suggestions = self.generate_hedging_suggestions(
            &risk_factors,
            &strategy,
        ).await?;
        
        // Determine execution reason
        let execution_reason = self.determine_risk_based_execution_reason(
            &market_regime,
            &risk_factors,
        );
        
        let recommendation = RiskAdjustedRecommendation {
            strategy_id: strategy.strategy_id.clone(),
            recommended_amount: risk_adjusted_amount,
            confidence_level: risk_model.confidence_score,
            risk_score,
            execution_reason,
            market_conditions,
            risk_factors,
            hedging_suggestions,
        };
        
        debug!("🎯 Generated risk-adjusted recommendation for strategy {}: {} {} (risk score: {:.2})", 
            strategy.strategy_id, 
            recommendation.recommended_amount,
            strategy.output_token,
            risk_score);
        
        Ok(recommendation)
    }
    
    /// Calculate risk-adjusted position size
    async fn calculate_risk_adjusted_amount(
        &self,
        base_amount: Decimal,
        risk_model: &RiskModel,
        market_regime: &MarketRegime,
        volatility_metrics: &VolatilityMetrics,
        _market_conditions: &MarketConditions,
    ) -> Result<Decimal> {
        let mut adjustment_factor = Decimal::ONE;
        
        match &risk_model.model_type {
            RiskModelType::VolatilityAdjusted { volatility_threshold, adjustment_factor: factor, .. } => {
                if volatility_metrics.historical_volatility > *volatility_threshold {
                    // Reduce position size in high volatility
                    adjustment_factor = Decimal::from_f64_retain(1.0 - factor).unwrap_or(Decimal::ONE);
                }
            },
            
            RiskModelType::ValueAtRisk { max_loss_percentage, .. } => {
                // Adjust based on VaR calculation
                if let Some(var_95) = risk_model.performance_metrics.var_95 {
                    if var_95.abs() > *max_loss_percentage {
                        adjustment_factor = Decimal::from_f64_retain(
                            max_loss_percentage / var_95.abs()
                        ).unwrap_or(Decimal::ONE);
                    }
                }
            },
            
            RiskModelType::KellyCriterion { win_rate, avg_win, avg_loss, .. } => {
                // Kelly formula: f = (bp - q) / b
                // where b = odds, p = win probability, q = loss probability
                let odds = avg_win / avg_loss;
                let kelly_fraction = (odds * win_rate - (1.0 - win_rate)) / odds;
                
                // Apply fractional Kelly (usually 25% of full Kelly)
                let fractional_kelly = kelly_fraction * 0.25;
                adjustment_factor = Decimal::from_f64_retain(fractional_kelly.max(0.1).min(2.0))
                    .unwrap_or(Decimal::ONE);
            },
            
            RiskModelType::RegimeAdaptive { bull_multiplier, bear_multiplier, sideways_multiplier, .. } => {
                match market_regime {
                    MarketRegime::Bull { .. } => {
                        adjustment_factor = Decimal::from_f64_retain(*bull_multiplier).unwrap_or(Decimal::ONE);
                    },
                    MarketRegime::Bear { .. } => {
                        adjustment_factor = Decimal::from_f64_retain(*bear_multiplier).unwrap_or(Decimal::ONE);
                    },
                    MarketRegime::Sideways { .. } => {
                        adjustment_factor = Decimal::from_f64_retain(*sideways_multiplier).unwrap_or(Decimal::ONE);
                    },
                    MarketRegime::Transition { .. } => {
                        adjustment_factor = Decimal::from_f64_retain(0.5).unwrap(); // Conservative during transitions
                    },
                }
            },
            
            _ => {
                // Default volatility adjustment
                if volatility_metrics.historical_volatility > 0.5 {
                    adjustment_factor = Decimal::from_f64_retain(0.7).unwrap();
                }
            }
        }
        
        // Apply maximum position size limit
        let adjusted_amount = base_amount * adjustment_factor;
        let max_amount = risk_model.parameters.max_position_size;
        
        Ok(adjusted_amount.min(max_amount))
    }
    
    /// Calculate overall risk score
    async fn calculate_overall_risk_score(
        &self,
        risk_model: &RiskModel,
        market_regime: &MarketRegime,
        volatility_metrics: &VolatilityMetrics,
    ) -> Result<f64> {
        let mut risk_components = Vec::new();
        
        // Volatility risk (0.3 weight)
        let volatility_risk = (volatility_metrics.historical_volatility / 2.0).min(1.0);
        risk_components.push((volatility_risk, 0.3));
        
        // Market regime risk (0.2 weight)
        let regime_risk = match market_regime {
            MarketRegime::Bull { .. } => 0.2,
            MarketRegime::Bear { .. } => 0.8,
            MarketRegime::Sideways { volatility, .. } => volatility / 2.0,
            MarketRegime::Transition { .. } => 0.6,
        };
        risk_components.push((regime_risk, 0.2));
        
        // Drawdown risk (0.2 weight)
        let drawdown_risk = risk_model.performance_metrics.max_drawdown / 100.0;
        risk_components.push((drawdown_risk, 0.2));
        
        // VaR risk (0.15 weight)
        let var_risk = risk_model.performance_metrics.var_95
            .map(|var| (var.abs() / 50.0).min(1.0))
            .unwrap_or(0.5);
        risk_components.push((var_risk, 0.15));
        
        // Liquidity risk (0.15 weight)
        let liquidity_risk = 0.3; // Placeholder - would calculate from market data
        risk_components.push((liquidity_risk, 0.15));
        
        // Calculate weighted average
        let total_risk = risk_components.iter()
            .map(|(risk, weight)| risk * weight)
            .sum::<f64>();
        
        Ok(total_risk.max(0.0).min(1.0))
    }
    
    /// Identify current risk factors
    async fn identify_risk_factors(
        &self,
        risk_model: &RiskModel,
        market_regime: &MarketRegime,
        volatility_metrics: &VolatilityMetrics,
        market_conditions: &MarketConditions,
    ) -> Result<Vec<RiskFactor>> {
        let mut risk_factors = Vec::new();
        
        // High volatility risk
        if volatility_metrics.historical_volatility > 0.5 {
            risk_factors.push(RiskFactor {
                factor_type: RiskFactorType::HighVolatility,
                severity: if volatility_metrics.historical_volatility > 1.0 {
                    RiskSeverity::High
                } else {
                    RiskSeverity::Medium
                },
                description: format!("Historical volatility at {:.1}% is elevated", 
                    volatility_metrics.historical_volatility * 100.0),
                mitigation_strategy: Some("Consider reducing position size".to_string()),
            });
        }
        
        // Low liquidity risk
        if let Some(volume) = market_conditions.volume_24h {
            if volume < 100000 {
                risk_factors.push(RiskFactor {
                    factor_type: RiskFactorType::LowLiquidity,
                    severity: RiskSeverity::Medium,
                    description: format!("24h volume of ${} is low", volume),
                    mitigation_strategy: Some("Use smaller position sizes and wider slippage tolerances".to_string()),
                });
            }
        }
        
        // Market regime risk
        match market_regime {
            MarketRegime::Bear { strength, .. } => {
                if *strength > 0.7 {
                    risk_factors.push(RiskFactor {
                        factor_type: RiskFactorType::MarketRegimeChange,
                        severity: RiskSeverity::High,
                        description: "Strong bear market detected".to_string(),
                        mitigation_strategy: Some("Consider defensive positioning".to_string()),
                    });
                }
            },
            MarketRegime::Transition { .. } => {
                risk_factors.push(RiskFactor {
                    factor_type: RiskFactorType::MarketRegimeChange,
                    severity: RiskSeverity::Medium,
                    description: "Market regime transition detected".to_string(),
                    mitigation_strategy: Some("Exercise caution during regime changes".to_string()),
                });
            },
            _ => {}
        }
        
        // Drawdown risk
        if risk_model.performance_metrics.max_drawdown > 30.0 {
            risk_factors.push(RiskFactor {
                factor_type: RiskFactorType::DrawdownRisk,
                severity: RiskSeverity::High,
                description: format!("Maximum drawdown of {:.1}% is concerning", 
                    risk_model.performance_metrics.max_drawdown),
                mitigation_strategy: Some("Consider implementing stop-loss mechanisms".to_string()),
            });
        }
        
        Ok(risk_factors)
    }
    
    /// Generate hedging suggestions based on risk factors
    async fn generate_hedging_suggestions(
        &self,
        risk_factors: &[RiskFactor],
        _strategy: &DCAStrategy,
    ) -> Result<Vec<HedgingSuggestion>> {
        let mut suggestions = Vec::new();
        
        for risk_factor in risk_factors {
            match risk_factor.factor_type {
                RiskFactorType::HighVolatility => {
                    suggestions.push(HedgingSuggestion {
                        hedge_type: HedgeType::PositionSizing,
                        description: "Reduce position sizes during high volatility periods".to_string(),
                        estimated_cost: None,
                        effectiveness: 0.7,
                    });
                },
                
                RiskFactorType::LowLiquidity => {
                    suggestions.push(HedgingSuggestion {
                        hedge_type: HedgeType::Diversification,
                        description: "Diversify across multiple liquid assets".to_string(),
                        estimated_cost: None,
                        effectiveness: 0.6,
                    });
                },
                
                RiskFactorType::DrawdownRisk => {
                    suggestions.push(HedgingSuggestion {
                        hedge_type: HedgeType::StopLoss,
                        description: "Implement dynamic stop-loss orders".to_string(),
                        estimated_cost: Some(Decimal::from_str("0.005").unwrap()), // 0.5% estimated cost
                        effectiveness: 0.8,
                    });
                },
                
                _ => {}
            }
        }
        
        Ok(suggestions)
    }
    
    /// Helper methods for data fetching and calculations
    async fn fetch_historical_data(&self, token_mint: &str, periods: u32) -> Result<VecDeque<PricePoint>> {
        // This would fetch actual historical data from Jupiter Price API
        // For now, we'll return placeholder data
        let mut data = VecDeque::new();
        let base_price = Decimal::from(100);
        
        for i in 0..periods {
            data.push_back(PricePoint {
                timestamp: Utc::now() - Duration::hours(i as i64),
                price: base_price + Decimal::from(i % 10),
                volume: Some(1000000),
                returns: Some(0.01),
                volatility: Some(0.2),
            });
        }
        
        Ok(data)
    }
    
    async fn get_or_create_risk_model(&self, token_mint: &str) -> Result<RiskModel> {
        let models = self.risk_models.read().await;
        if let Some(model) = models.get(token_mint) {
            return Ok(model.clone());
        }
        drop(models);
        
        // Create default volatility-adjusted model
        self.create_risk_model(
            token_mint.to_string(),
            RiskModelType::VolatilityAdjusted {
                lookback_periods: 30,
                volatility_threshold: 0.5,
                adjustment_factor: 0.3,
            },
        ).await
    }
    
    async fn detect_market_regime(&self, _token_mint: &str) -> Result<MarketRegime> {
        // Placeholder implementation
        Ok(MarketRegime::Sideways {
            volatility: 0.3,
            range_bound: (Decimal::from(90), Decimal::from(110)),
        })
    }
    
    async fn calculate_volatility(&self, _token_mint: &str) -> Result<VolatilityMetrics> {
        // Placeholder implementation
        Ok(VolatilityMetrics {
            historical_volatility: 0.25,
            realized_volatility: 0.23,
            implied_volatility: None,
            garch_forecast: None,
            ewma_volatility: 0.24,
            parkinson_volatility: None,
            garman_klass_volatility: None,
        })
    }
    
    async fn get_current_market_conditions(&self, token_mint: &str) -> Result<MarketConditions> {
        let prices = self.price_client
            .get_prices(vec![token_mint.to_string()])
            .await?;
            
        let price_data = prices.prices
            .get(token_mint)
            .ok_or_else(|| BotError::trading(format!("Price data not found for token {}", token_mint)))?;
        
        Ok(MarketConditions {
            token_price: Decimal::from_f64_retain(price_data.usd_price)
                .unwrap_or(Decimal::ZERO),
            volume_24h: price_data.volume_24h,
            volatility: None,
            rsi: None,
            fear_greed_index: None,
            social_sentiment: None,
            market_cap_rank: None,
        })
    }
    
    async fn update_risk_model_metrics(&self, mut risk_model: RiskModel) -> Result<RiskModel> {
        // Calculate performance metrics from historical data
        let returns: Vec<f64> = risk_model.historical_data
            .iter()
            .filter_map(|p| p.returns)
            .collect();
        
        if !returns.is_empty() {
            let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / returns.len() as f64;
            let std_dev = variance.sqrt();
            
            risk_model.performance_metrics = RiskModelMetrics {
                sharpe_ratio: Some(mean_return / std_dev),
                sortino_ratio: None, // Would calculate downside deviation
                max_drawdown: self.calculate_max_drawdown(&returns),
                volatility: std_dev,
                var_95: Some(self.calculate_var(&returns, 0.95)),
                cvar_95: None, // Would calculate conditional VaR
                calmar_ratio: None,
                information_ratio: None,
                tracking_error: None,
            };
        }
        
        Ok(risk_model)
    }
    
    fn calculate_max_drawdown(&self, returns: &[f64]) -> f64 {
        let mut peak = 1.0;
        let mut max_dd = 0.0;
        let mut cum_return = 1.0;
        
        for &ret in returns {
            cum_return *= 1.0 + ret;
            if cum_return > peak {
                peak = cum_return;
            }
            let drawdown = (peak - cum_return) / peak;
            if drawdown > max_dd {
                max_dd = drawdown;
            }
        }
        
        max_dd * 100.0 // Convert to percentage
    }
    
    fn calculate_var(&self, returns: &[f64], confidence: f64) -> f64 {
        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((1.0 - confidence) * sorted_returns.len() as f64) as usize;
        sorted_returns[index.min(sorted_returns.len() - 1)] * 100.0 // Convert to percentage
    }
    
    fn determine_risk_based_execution_reason(
        &self,
        market_regime: &MarketRegime,
        risk_factors: &[RiskFactor],
    ) -> ExecutionReason {
        // Determine execution reason based on risk analysis
        if risk_factors.iter().any(|rf| matches!(rf.severity, RiskSeverity::Critical)) {
            ExecutionReason::ManualTrigger // Require manual approval for critical risk
        } else {
            match market_regime {
                MarketRegime::Bear { .. } => ExecutionReason::PriceDip,
                MarketRegime::Bull { .. } => ExecutionReason::MomentumSignal,
                _ => ExecutionReason::ScheduledInterval,
            }
        }
    }
}

impl Default for RiskParameters {
    fn default() -> Self {
        Self {
            max_position_size: Decimal::from(10000), // $10k default
            max_drawdown: 20.0, // 20%
            volatility_ceiling: 1.0, // 100%
            correlation_limit: 0.8, // 80%
            liquidity_threshold: Decimal::from(100000), // $100k
            stop_loss_percentage: Some(15.0), // 15%
            take_profit_percentage: Some(50.0), // 50%
            rebalance_frequency_days: 30,
        }
    }
}

impl Default for RiskModelMetrics {
    fn default() -> Self {
        Self {
            sharpe_ratio: None,
            sortino_ratio: None,
            max_drawdown: 0.0,
            volatility: 0.0,
            var_95: None,
            cvar_95: None,
            calmar_ratio: None,
            information_ratio: None,
            tracking_error: None,
        }
    }
}