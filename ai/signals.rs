use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use super::groq::{GroqAnalyzer, MarketAnalysis};
use crate::market::aggregator::MarketDataAggregator;
use crate::market::types::{TokenMarketData, TrendingToken, MarketTrend};
use crate::utils::formatting::{format_market_cap, format_volume};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub token_address: String,
    pub symbol: String,
    pub signal_type: SignalType,
    pub strength: SignalStrength,
    pub confidence: f64,
    pub entry_price: f64,
    pub target_price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub risk_reward_ratio: f64,
    pub reasoning: String,
    pub technical_indicators: TechnicalIndicators,
    pub market_conditions: MarketConditions,
    pub ai_insights: Option<MarketAnalysis>,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalType {
    Buy,
    Sell,
    StrongBuy,
    StrongSell,
    Hold,
    Accumulate,
    Distribute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalStrength {
    VeryStrong,
    Strong,
    Moderate,
    Weak,
    VeryWeak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub price_momentum: f64,
    pub volume_trend: VolumeTrend,
    pub liquidity_score: f64,
    pub volatility: f64,
    pub buy_sell_ratio: f64,
    pub holder_trend: HolderTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeTrend {
    Increasing,
    Stable,
    Decreasing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HolderTrend {
    Accumulation,
    Distribution,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub overall_sentiment: String,
    pub trending_rank: Option<u32>,
    pub sector_performance: String,
    pub correlation_with_sol: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPerformance {
    pub signal_id: String,
    pub hit_target: bool,
    pub hit_stop_loss: bool,
    pub max_profit_percent: f64,
    pub max_drawdown_percent: f64,
    pub duration_hours: f64,
}

/// AI-powered signal generator combining technical analysis with LLM insights
pub struct SignalGenerator {
    market_aggregator: Arc<MarketDataAggregator>,
    ai_analyzer: Arc<GroqAnalyzer>,
    signal_cache: Arc<RwLock<SignalCache>>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
}

struct SignalCache {
    active_signals: HashMap<String, TradingSignal>,
    historical_signals: Vec<TradingSignal>,
    last_update: DateTime<Utc>,
}

struct PerformanceTracker {
    signal_performance: HashMap<String, SignalPerformance>,
    success_rate: f64,
    average_return: f64,
    total_signals: u32,
}

impl SignalGenerator {
    pub fn new(
        market_aggregator: Arc<MarketDataAggregator>,
        ai_analyzer: Arc<GroqAnalyzer>,
    ) -> Self {
        Self {
            market_aggregator,
            ai_analyzer,
            signal_cache: Arc::new(RwLock::new(SignalCache {
                active_signals: HashMap::new(),
                historical_signals: Vec::new(),
                last_update: Utc::now(),
            })),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker {
                signal_performance: HashMap::new(),
                success_rate: 0.0,
                average_return: 0.0,
                total_signals: 0,
            })),
        }
    }

    /// Generate trading signals for trending tokens
    pub async fn generate_signals(&self, limit: usize) -> Result<Vec<TradingSignal>> {
        info!("Generating AI-powered trading signals");
        
        // Get trending tokens and market data
        let trending = self.market_aggregator.get_trending(limit * 2).await?;
        let market_trends = self.market_aggregator.get_market_trends().await?;
        
        let mut signals = Vec::new();
        
        for token in trending.iter().take(limit) {
            match self.analyze_token_for_signal(&token.token_data, &market_trends).await {
                Ok(signal) => {
                    if signal.confidence >= 60.0 {  // Only include high-confidence signals
                        signals.push(signal);
                    }
                }
                Err(e) => {
                    warn!("Failed to generate signal for {}: {}", token.token_data.symbol, e);
                }
            }
        }
        
        // Update cache
        let mut cache = self.signal_cache.write().await;
        for signal in &signals {
            cache.active_signals.insert(signal.token_address.clone(), signal.clone());
        }
        cache.last_update = Utc::now();
        
        info!("Generated {} trading signals", signals.len());
        Ok(signals)
    }

    /// Analyze a specific token for signal generation
    async fn analyze_token_for_signal(
        &self,
        token: &TokenMarketData,
        market_trends: &MarketTrend,
    ) -> Result<TradingSignal> {
        debug!("Analyzing {} for signal generation", token.symbol);
        
        // Get AI analysis
        let ai_insights = match self.ai_analyzer.analyze_token(&token.symbol).await {
            Ok(analysis) => Some(analysis),
            Err(e) => {
                warn!("AI analysis failed for {}: {}", token.symbol, e);
                None
            }
        };
        
        // Calculate technical indicators
        let technical = self.calculate_technical_indicators(token);
        
        // Determine signal type and strength
        let (signal_type, strength, confidence) = self.determine_signal(
            token,
            &technical,
            ai_insights.as_ref(),
        );
        
        // Calculate targets and stop loss
        let (target_price, stop_loss) = self.calculate_price_targets(
            token.price_usd,
            &signal_type,
            &strength,
            technical.volatility,
        );
        
        let risk_reward_ratio = if let (Some(target), Some(stop)) = (target_price, stop_loss) {
            (target - token.price_usd).abs() / (token.price_usd - stop).abs()
        } else {
            0.0
        };
        
        // Generate reasoning
        let reasoning = self.generate_reasoning(
            token,
            &signal_type,
            &technical,
            ai_insights.as_ref(),
        );
        
        // Determine market conditions
        let market_conditions = MarketConditions {
            overall_sentiment: market_trends.market_sentiment.dominant_trend.clone(),
            trending_rank: market_trends.trending_tokens
                .iter()
                .position(|t| t.token_data.address == token.address)
                .map(|p| (p + 1) as u32),
            sector_performance: "DeFi".to_string(), // Would need sector classification
            correlation_with_sol: 0.65, // Would need correlation calculation
        };
        
        Ok(TradingSignal {
            token_address: token.address.clone(),
            symbol: token.symbol.clone(),
            signal_type,
            strength,
            confidence,
            entry_price: token.price_usd,
            target_price,
            stop_loss,
            risk_reward_ratio,
            reasoning,
            technical_indicators: technical,
            market_conditions,
            ai_insights,
            generated_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(4),
        })
    }

    /// Calculate technical indicators for a token
    fn calculate_technical_indicators(&self, token: &TokenMarketData) -> TechnicalIndicators {
        // Price momentum based on 24h change
        let price_momentum = token.price_change_24h;
        
        // Volume trend analysis
        let volume_trend = if token.volume_24h_change > 20.0 {
            VolumeTrend::Increasing
        } else if token.volume_24h_change < -20.0 {
            VolumeTrend::Decreasing
        } else {
            VolumeTrend::Stable
        };
        
        // Liquidity score (0-100)
        let liquidity_score = (token.liquidity_usd / 1_000_000.0).min(100.0);
        
        // Simplified volatility calculation
        let volatility = token.price_change_24h.abs() / 100.0;
        
        // Buy/sell ratio from DEX data
        let buy_sell_ratio = if token.dex_data.sell_count_24h > 0 {
            token.dex_data.buy_count_24h as f64 / token.dex_data.sell_count_24h as f64
        } else {
            1.0
        };
        
        // Holder trend based on unique wallets
        let holder_trend = if token.dex_data.unique_wallets_24h > 100 {
            HolderTrend::Accumulation
        } else if token.dex_data.unique_wallets_24h < 50 {
            HolderTrend::Distribution
        } else {
            HolderTrend::Neutral
        };
        
        TechnicalIndicators {
            price_momentum,
            volume_trend,
            liquidity_score,
            volatility,
            buy_sell_ratio,
            holder_trend,
        }
    }

    /// Determine signal type, strength, and confidence
    fn determine_signal(
        &self,
        token: &TokenMarketData,
        technical: &TechnicalIndicators,
        ai_insights: Option<&MarketAnalysis>,
    ) -> (SignalType, SignalStrength, f64) {
        let mut score = 0.0;
        let mut factors = 0;
        
        // Price momentum factor
        if technical.price_momentum > 50.0 {
            score += 20.0;
        } else if technical.price_momentum > 20.0 {
            score += 10.0;
        } else if technical.price_momentum < -50.0 {
            score -= 20.0;
        } else if technical.price_momentum < -20.0 {
            score -= 10.0;
        }
        factors += 1;
        
        // Volume trend factor
        match technical.volume_trend {
            VolumeTrend::Increasing => score += 15.0,
            VolumeTrend::Stable => score += 0.0,
            VolumeTrend::Decreasing => score -= 10.0,
        }
        factors += 1;
        
        // Liquidity factor
        if technical.liquidity_score > 80.0 {
            score += 15.0;
        } else if technical.liquidity_score > 50.0 {
            score += 10.0;
        } else if technical.liquidity_score < 20.0 {
            score -= 15.0;
        }
        factors += 1;
        
        // Buy/sell ratio factor
        if technical.buy_sell_ratio > 1.5 {
            score += 15.0;
        } else if technical.buy_sell_ratio > 1.2 {
            score += 10.0;
        } else if technical.buy_sell_ratio < 0.8 {
            score -= 10.0;
        } else if technical.buy_sell_ratio < 0.5 {
            score -= 15.0;
        }
        factors += 1;
        
        // Holder trend factor
        match technical.holder_trend {
            HolderTrend::Accumulation => score += 10.0,
            HolderTrend::Neutral => score += 0.0,
            HolderTrend::Distribution => score -= 10.0,
        }
        factors += 1;
        
        // AI insights factor
        if let Some(ai) = ai_insights {
            let ai_score = match ai.signal.as_str() {
                "BUY" => 20.0,
                "SELL" => -20.0,
                _ => 0.0,
            };
            score += ai_score * (ai.confidence / 100.0);
            factors += 1;
        }
        
        // Market cap factor
        if token.market_cap > 10_000_000.0 {
            score += 10.0;
        } else if token.market_cap < 100_000.0 {
            score -= 10.0;
        }
        factors += 1;
        
        // Normalize score to -100 to 100
        let normalized_score = (score / factors as f64) * 2.0;
        
        // Determine signal type
        let signal_type = if normalized_score >= 40.0 {
            SignalType::StrongBuy
        } else if normalized_score >= 20.0 {
            SignalType::Buy
        } else if normalized_score >= 10.0 {
            SignalType::Accumulate
        } else if normalized_score <= -40.0 {
            SignalType::StrongSell
        } else if normalized_score <= -20.0 {
            SignalType::Sell
        } else if normalized_score <= -10.0 {
            SignalType::Distribute
        } else {
            SignalType::Hold
        };
        
        // Determine strength
        let strength = match normalized_score.abs() {
            s if s >= 60.0 => SignalStrength::VeryStrong,
            s if s >= 40.0 => SignalStrength::Strong,
            s if s >= 20.0 => SignalStrength::Moderate,
            s if s >= 10.0 => SignalStrength::Weak,
            _ => SignalStrength::VeryWeak,
        };
        
        // Calculate confidence (0-100)
        let confidence = 50.0 + (normalized_score.abs() / 2.0);
        
        (signal_type, strength, confidence.min(95.0))
    }

    /// Calculate price targets and stop loss
    fn calculate_price_targets(
        &self,
        current_price: f64,
        signal_type: &SignalType,
        strength: &SignalStrength,
        volatility: f64,
    ) -> (Option<f64>, Option<f64>) {
        let multiplier = match strength {
            SignalStrength::VeryStrong => 3.0,
            SignalStrength::Strong => 2.5,
            SignalStrength::Moderate => 2.0,
            SignalStrength::Weak => 1.5,
            SignalStrength::VeryWeak => 1.0,
        };
        
        let base_move = volatility * multiplier;
        
        match signal_type {
            SignalType::StrongBuy | SignalType::Buy | SignalType::Accumulate => {
                let target = current_price * (1.0 + base_move);
                let stop = current_price * (1.0 - base_move * 0.5);
                (Some(target), Some(stop))
            }
            SignalType::StrongSell | SignalType::Sell | SignalType::Distribute => {
                let target = current_price * (1.0 - base_move);
                let stop = current_price * (1.0 + base_move * 0.5);
                (Some(target), Some(stop))
            }
            SignalType::Hold => (None, None),
        }
    }

    /// Generate human-readable reasoning for the signal
    fn generate_reasoning(
        &self,
        token: &TokenMarketData,
        signal_type: &SignalType,
        technical: &TechnicalIndicators,
        ai_insights: Option<&MarketAnalysis>,
    ) -> String {
        let mut reasons = Vec::new();
        
        // Price action reasoning
        if technical.price_momentum > 20.0 {
            reasons.push(format!("Strong price momentum (+{:.1}%)", technical.price_momentum));
        } else if technical.price_momentum < -20.0 {
            reasons.push(format!("Negative price momentum ({:.1}%)", technical.price_momentum));
        }
        
        // Volume reasoning
        match technical.volume_trend {
            VolumeTrend::Increasing => reasons.push("Increasing volume indicates growing interest".to_string()),
            VolumeTrend::Decreasing => reasons.push("Decreasing volume suggests waning interest".to_string()),
            _ => {}
        }
        
        // Liquidity reasoning
        if technical.liquidity_score > 70.0 {
            reasons.push(format!("High liquidity (${:.0})", token.liquidity_usd));
        } else if technical.liquidity_score < 30.0 {
            reasons.push("Low liquidity poses risk".to_string());
        }
        
        // Buy/sell ratio reasoning
        if technical.buy_sell_ratio > 1.3 {
            reasons.push(format!("Strong buying pressure (ratio: {:.2})", technical.buy_sell_ratio));
        } else if technical.buy_sell_ratio < 0.7 {
            reasons.push(format!("Heavy selling pressure (ratio: {:.2})", technical.buy_sell_ratio));
        }
        
        // AI insights reasoning
        if let Some(ai) = ai_insights {
            if ai.confidence > 70.0 {
                reasons.push(format!("AI analysis: {} ({:.0}% confidence)", ai.signal, ai.confidence));
            }
        }
        
        // Market cap reasoning
        if token.market_cap > 10_000_000.0 {
            reasons.push(format!("Established market cap ({})", format_market_cap(token.market_cap)));
        } else if token.market_cap < 500_000.0 {
            reasons.push("Low market cap - high risk/reward".to_string());
        }
        
        let signal_action = match signal_type {
            SignalType::StrongBuy => "Strong Buy",
            SignalType::Buy => "Buy",
            SignalType::Accumulate => "Accumulate",
            SignalType::Hold => "Hold",
            SignalType::Distribute => "Distribute",
            SignalType::Sell => "Sell",
            SignalType::StrongSell => "Strong Sell",
        };
        
        format!(
            "{} signal: {}",
            signal_action,
            if reasons.is_empty() {
                "Based on overall market conditions".to_string()
            } else {
                reasons.join(". ")
            }
        )
    }

    /// Get active signals
    pub async fn get_active_signals(&self) -> Result<Vec<TradingSignal>> {
        let cache = self.signal_cache.read().await;
        let now = Utc::now();
        
        let active: Vec<TradingSignal> = cache
            .active_signals
            .values()
            .filter(|s| s.expires_at > now)
            .cloned()
            .collect();
        
        Ok(active)
    }

    /// Track signal performance
    pub async fn track_signal_performance(
        &self,
        signal_id: &str,
        current_price: f64,
    ) -> Result<()> {
        let cache = self.signal_cache.read().await;
        
        if let Some(signal) = cache.active_signals.get(signal_id) {
            let mut tracker = self.performance_tracker.write().await;
            
            let performance = tracker
                .signal_performance
                .entry(signal_id.to_string())
                .or_insert(SignalPerformance {
                    signal_id: signal_id.to_string(),
                    hit_target: false,
                    hit_stop_loss: false,
                    max_profit_percent: 0.0,
                    max_drawdown_percent: 0.0,
                    duration_hours: 0.0,
                });
            
            let price_change_percent = ((current_price - signal.entry_price) / signal.entry_price) * 100.0;
            
            // Update max profit/drawdown
            if price_change_percent > performance.max_profit_percent {
                performance.max_profit_percent = price_change_percent;
            }
            if price_change_percent < performance.max_drawdown_percent {
                performance.max_drawdown_percent = price_change_percent;
            }
            
            // Check if target or stop loss hit
            if let Some(target) = signal.target_price {
                if current_price >= target {
                    performance.hit_target = true;
                }
            }
            if let Some(stop) = signal.stop_loss {
                if current_price <= stop {
                    performance.hit_stop_loss = true;
                }
            }
            
            // Update duration
            let duration = Utc::now().signed_duration_since(signal.generated_at);
            performance.duration_hours = duration.num_hours() as f64;
        }
        
        Ok(())
    }

    /// Get signal performance statistics
    pub async fn get_performance_stats(&self) -> Result<(f64, f64, u32)> {
        let tracker = self.performance_tracker.read().await;
        Ok((tracker.success_rate, tracker.average_return, tracker.total_signals))
    }

    /// Format signal for display
    pub fn format_signal(signal: &TradingSignal) -> String {
        let signal_emoji = match signal.signal_type {
            SignalType::StrongBuy => "🚀",
            SignalType::Buy | SignalType::Accumulate => "📈",
            SignalType::Hold => "⏸️",
            SignalType::Sell | SignalType::Distribute => "📉",
            SignalType::StrongSell => "🔻",
        };
        
        let strength_emoji = match signal.strength {
            SignalStrength::VeryStrong => "💪💪💪",
            SignalStrength::Strong => "💪💪",
            SignalStrength::Moderate => "💪",
            SignalStrength::Weak => "👌",
            SignalStrength::VeryWeak => "🤏",
        };
        
        let mut message = format!(
            "{} {} Signal: {}\n",
            signal_emoji,
            signal.symbol,
            match signal.signal_type {
                SignalType::StrongBuy => "STRONG BUY",
                SignalType::Buy => "BUY",
                SignalType::Accumulate => "ACCUMULATE",
                SignalType::Hold => "HOLD",
                SignalType::Distribute => "DISTRIBUTE",
                SignalType::Sell => "SELL",
                SignalType::StrongSell => "STRONG SELL",
            }
        );
        
        message.push_str(&format!("📊 Strength: {}\n", strength_emoji));
        message.push_str(&format!("🎯 Confidence: {:.0}%\n", signal.confidence));
        message.push_str(&format!("💵 Entry Price: ${:.6}\n", signal.entry_price));
        
        if let Some(target) = signal.target_price {
            let target_percent = ((target - signal.entry_price) / signal.entry_price) * 100.0;
            message.push_str(&format!("🎯 Target: ${:.6} ({:+.1}%)\n", target, target_percent));
        }
        
        if let Some(stop) = signal.stop_loss {
            let stop_percent = ((stop - signal.entry_price) / signal.entry_price) * 100.0;
            message.push_str(&format!("🛑 Stop Loss: ${:.6} ({:.1}%)\n", stop, stop_percent));
        }
        
        if signal.risk_reward_ratio > 0.0 {
            message.push_str(&format!("⚖️ Risk/Reward: 1:{:.1}\n", signal.risk_reward_ratio));
        }
        
        message.push_str(&format!("\n💡 {}\n", signal.reasoning));
        
        if let Some(ai) = &signal.ai_insights {
            message.push_str(&format!("\n🤖 AI Insights:\n{}\n", ai.summary));
        }
        
        message.push_str(&format!("\n⏰ Valid until: {}", 
            signal.expires_at.format("%H:%M UTC")));
        
        message
    }
}