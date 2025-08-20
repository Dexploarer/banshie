use chrono::{DateTime, Utc, Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use crate::errors::{BotError, Result};
use crate::db::Database;
use crate::telemetry::TelemetryService;

/// Comprehensive performance tracking system for trading activities
#[derive(Clone)]
pub struct PerformanceTracker {
    database: Arc<Database>,
    telemetry: Option<Arc<TelemetryService>>,
    performance_cache: Arc<RwLock<PerformanceCache>>,
    metrics_calculator: Arc<MetricsCalculator>,
    benchmark_data: Arc<RwLock<BenchmarkData>>,
}

/// Cache for frequently accessed performance data
#[derive(Debug, Clone)]
pub struct PerformanceCache {
    pub daily_performance: BTreeMap<NaiveDate, DailyPerformance>,
    pub weekly_performance: BTreeMap<String, WeeklyPerformance>, 
    pub monthly_performance: BTreeMap<String, MonthlyPerformance>,
    pub yearly_performance: BTreeMap<i32, YearlyPerformance>,
    pub all_time_performance: AllTimePerformance,
    pub last_updated: DateTime<Utc>,
}

/// Daily performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPerformance {
    pub date: NaiveDate,
    pub starting_value: Decimal,
    pub ending_value: Decimal,
    pub daily_return: Decimal,
    pub daily_return_percentage: f64,
    pub trades_executed: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub total_fees: Decimal,
    pub total_volume: Decimal,
    pub best_trade: Option<TradeRecord>,
    pub worst_trade: Option<TradeRecord>,
    pub tokens_traded: Vec<String>,
    pub high_water_mark: Decimal,
    pub drawdown: f64,
    pub volatility: f64,
}

/// Weekly performance aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyPerformance {
    pub week_identifier: String, // Format: "2025-W03"
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub starting_value: Decimal,
    pub ending_value: Decimal,
    pub weekly_return: Decimal,
    pub weekly_return_percentage: f64,
    pub total_trades: u32,
    pub win_rate: f64,
    pub average_daily_return: f64,
    pub best_day: Option<NaiveDate>,
    pub worst_day: Option<NaiveDate>,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
}

/// Monthly performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyPerformance {
    pub month_identifier: String, // Format: "2025-01"
    pub starting_value: Decimal,
    pub ending_value: Decimal,
    pub monthly_return: Decimal,
    pub monthly_return_percentage: f64,
    pub total_trades: u32,
    pub profitable_days: u32,
    pub losing_days: u32,
    pub max_consecutive_wins: u32,
    pub max_consecutive_losses: u32,
    pub calmar_ratio: f64,
    pub information_ratio: f64,
    pub beta: f64,
    pub alpha: f64,
}

/// Yearly performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyPerformance {
    pub year: i32,
    pub starting_value: Decimal,
    pub ending_value: Decimal,
    pub yearly_return: Decimal,
    pub yearly_return_percentage: f64,
    pub total_trades: u32,
    pub best_month: Option<String>,
    pub worst_month: Option<String>,
    pub max_drawdown: f64,
    pub max_drawdown_duration: Duration,
    pub recovery_time: Option<Duration>,
    pub risk_adjusted_return: f64,
    pub treynor_ratio: f64,
}

/// All-time performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllTimePerformance {
    pub start_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub current_value: Decimal,
    pub total_return: Decimal,
    pub total_return_percentage: f64,
    pub annualized_return: f64,
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub overall_win_rate: f64,
    pub average_win: Decimal,
    pub average_loss: Decimal,
    pub profit_factor: f64,
    pub expectancy: Decimal,
    pub max_drawdown: f64,
    pub max_drawdown_date: DateTime<Utc>,
    pub current_drawdown: f64,
    pub time_in_market: Duration,
    pub best_trade_ever: Option<TradeRecord>,
    pub worst_trade_ever: Option<TradeRecord>,
}

/// Individual trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub trade_id: String,
    pub timestamp: DateTime<Utc>,
    pub token_pair: String,
    pub trade_type: TradeType,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub quantity: Decimal,
    pub pnl: Decimal,
    pub pnl_percentage: f64,
    pub fees: Decimal,
    pub holding_period: Duration,
    pub strategy_used: String,
    pub risk_reward_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeType {
    Long,
    Short,
    Swap,
    Arbitrage,
}

/// Metrics calculator for advanced performance analytics
#[derive(Debug)]
pub struct MetricsCalculator {
    risk_free_rate: f64,
}

impl MetricsCalculator {
    /// Calculate Sharpe ratio
    pub fn calculate_sharpe_ratio(&self, returns: &[f64], period_multiplier: f64) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();
        
        if std_dev == 0.0 {
            return 0.0;
        }
        
        ((mean_return - self.risk_free_rate / period_multiplier) / std_dev) * period_multiplier.sqrt()
    }
    
    /// Calculate Sortino ratio (downside deviation)
    pub fn calculate_sortino_ratio(&self, returns: &[f64], target_return: f64, period_multiplier: f64) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < target_return)
            .map(|&r| (r - target_return).powi(2))
            .collect();
        
        if downside_returns.is_empty() {
            return f64::INFINITY; // No downside risk
        }
        
        let downside_deviation = (downside_returns.iter().sum::<f64>() / downside_returns.len() as f64).sqrt();
        
        if downside_deviation == 0.0 {
            return f64::INFINITY;
        }
        
        ((mean_return - target_return) / downside_deviation) * period_multiplier.sqrt()
    }
    
    /// Calculate Calmar ratio (return / max drawdown)
    pub fn calculate_calmar_ratio(&self, annual_return: f64, max_drawdown: f64) -> f64 {
        if max_drawdown == 0.0 {
            return f64::INFINITY;
        }
        annual_return / max_drawdown.abs()
    }
    
    /// Calculate Information ratio
    pub fn calculate_information_ratio(&self, returns: &[f64], benchmark_returns: &[f64]) -> f64 {
        if returns.len() != benchmark_returns.len() || returns.is_empty() {
            return 0.0;
        }
        
        let excess_returns: Vec<f64> = returns.iter()
            .zip(benchmark_returns.iter())
            .map(|(r, b)| r - b)
            .collect();
        
        let mean_excess = excess_returns.iter().sum::<f64>() / excess_returns.len() as f64;
        let tracking_error = self.calculate_tracking_error(&excess_returns);
        
        if tracking_error == 0.0 {
            return 0.0;
        }
        
        mean_excess / tracking_error
    }
    
    /// Calculate tracking error
    pub fn calculate_tracking_error(&self, excess_returns: &[f64]) -> f64 {
        if excess_returns.is_empty() {
            return 0.0;
        }
        
        let mean = excess_returns.iter().sum::<f64>() / excess_returns.len() as f64;
        let variance = excess_returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / excess_returns.len() as f64;
        
        variance.sqrt()
    }
    
    /// Calculate maximum drawdown
    pub fn calculate_max_drawdown(&self, values: &[Decimal]) -> (f64, usize, usize) {
        if values.is_empty() {
            return (0.0, 0, 0);
        }
        
        let mut max_drawdown = 0.0;
        let mut peak_idx = 0;
        let mut trough_idx = 0;
        let mut current_peak = values[0];
        let mut current_peak_idx = 0;
        
        for (i, &value) in values.iter().enumerate() {
            if value > current_peak {
                current_peak = value;
                current_peak_idx = i;
            }
            
            let drawdown = if current_peak > Decimal::ZERO {
                ((current_peak - value) / current_peak * Decimal::from(100)).to_f64().unwrap_or(0.0)
            } else {
                0.0
            };
            
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
                peak_idx = current_peak_idx;
                trough_idx = i;
            }
        }
        
        (max_drawdown, peak_idx, trough_idx)
    }
    
    /// Calculate beta (systematic risk)
    pub fn calculate_beta(&self, returns: &[f64], market_returns: &[f64]) -> f64 {
        if returns.len() != market_returns.len() || returns.len() < 2 {
            return 1.0; // Default beta
        }
        
        let covariance = self.calculate_covariance(returns, market_returns);
        let market_variance = self.calculate_variance(market_returns);
        
        if market_variance == 0.0 {
            return 1.0;
        }
        
        covariance / market_variance
    }
    
    /// Calculate alpha (excess return)
    pub fn calculate_alpha(&self, portfolio_return: f64, market_return: f64, beta: f64) -> f64 {
        portfolio_return - (self.risk_free_rate + beta * (market_return - self.risk_free_rate))
    }
    
    /// Calculate covariance
    fn calculate_covariance(&self, x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.is_empty() {
            return 0.0;
        }
        
        let mean_x = x.iter().sum::<f64>() / x.len() as f64;
        let mean_y = y.iter().sum::<f64>() / y.len() as f64;
        
        x.iter().zip(y.iter())
            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
            .sum::<f64>() / x.len() as f64
    }
    
    /// Calculate variance
    fn calculate_variance(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / values.len() as f64
    }
}

/// Benchmark data for comparison
#[derive(Debug, Clone)]
pub struct BenchmarkData {
    pub spy_returns: Vec<f64>,      // S&P 500 returns
    pub btc_returns: Vec<f64>,      // Bitcoin returns
    pub eth_returns: Vec<f64>,      // Ethereum returns
    pub sol_returns: Vec<f64>,      // Solana returns
    pub custom_benchmark: Option<Vec<f64>>,
    pub last_updated: DateTime<Utc>,
}

impl PerformanceTracker {
    /// Create new performance tracker
    pub fn new(
        database: Arc<Database>,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("📊 Initializing performance tracking system");
        
        Self {
            database,
            telemetry,
            performance_cache: Arc::new(RwLock::new(PerformanceCache {
                daily_performance: BTreeMap::new(),
                weekly_performance: BTreeMap::new(),
                monthly_performance: BTreeMap::new(),
                yearly_performance: BTreeMap::new(),
                all_time_performance: AllTimePerformance::default(),
                last_updated: Utc::now(),
            })),
            metrics_calculator: Arc::new(MetricsCalculator {
                risk_free_rate: 0.05, // 5% annual risk-free rate
            }),
            benchmark_data: Arc::new(RwLock::new(BenchmarkData {
                spy_returns: Vec::new(),
                btc_returns: Vec::new(),
                eth_returns: Vec::new(),
                sol_returns: Vec::new(),
                custom_benchmark: None,
                last_updated: Utc::now(),
            })),
        }
    }
    
    /// Record a new trade
    pub async fn record_trade(&self, trade: TradeRecord) -> Result<()> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_trading_span("record_trade", Some(&trade.token_pair))
        );
        
        // Store trade in database
        self.store_trade_record(&trade).await?;
        
        // Update performance metrics
        self.update_performance_metrics(&trade).await?;
        
        debug!("📊 Recorded trade: {} with P&L: {}", trade.trade_id, trade.pnl);
        
        Ok(())
    }
    
    /// Get performance for a specific date range
    pub async fn get_performance_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<PerformanceReport> {
        let cache = self.performance_cache.read().await;
        
        let daily_data: Vec<DailyPerformance> = cache.daily_performance
            .range(start_date..=end_date)
            .map(|(_, perf)| perf.clone())
            .collect();
        
        if daily_data.is_empty() {
            return Err(BotError::not_found("No performance data for specified range".to_string()).into());
        }
        
        let total_return = daily_data.last().unwrap().ending_value - daily_data.first().unwrap().starting_value;
        let total_return_percentage = if daily_data.first().unwrap().starting_value > Decimal::ZERO {
            (total_return / daily_data.first().unwrap().starting_value * Decimal::from(100)).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };
        
        let returns: Vec<f64> = daily_data.iter()
            .map(|d| d.daily_return_percentage)
            .collect();
        
        let sharpe_ratio = self.metrics_calculator.calculate_sharpe_ratio(&returns, 252.0);
        let sortino_ratio = self.metrics_calculator.calculate_sortino_ratio(&returns, 0.0, 252.0);
        
        let values: Vec<Decimal> = daily_data.iter()
            .map(|d| d.ending_value)
            .collect();
        let (max_drawdown, _, _) = self.metrics_calculator.calculate_max_drawdown(&values);
        
        Ok(PerformanceReport {
            period: format!("{} to {}", start_date, end_date),
            starting_value: daily_data.first().unwrap().starting_value,
            ending_value: daily_data.last().unwrap().ending_value,
            total_return,
            total_return_percentage,
            sharpe_ratio,
            sortino_ratio,
            max_drawdown,
            total_trades: daily_data.iter().map(|d| d.trades_executed).sum(),
            win_rate: self.calculate_win_rate(&daily_data),
            daily_performance: daily_data,
        })
    }
    
    /// Get current month performance
    pub async fn get_current_month_performance(&self) -> Result<MonthlyPerformance> {
        let cache = self.performance_cache.read().await;
        let current_month = Utc::now().format("%Y-%m").to_string();
        
        cache.monthly_performance
            .get(&current_month)
            .cloned()
            .ok_or_else(|| BotError::not_found("No performance data for current month".to_string()).into())
    }
    
    /// Get year-to-date performance
    pub async fn get_ytd_performance(&self) -> Result<YearlyPerformance> {
        let cache = self.performance_cache.read().await;
        let current_year = Utc::now().year();
        
        cache.yearly_performance
            .get(&current_year)
            .cloned()
            .ok_or_else(|| BotError::not_found("No performance data for current year".to_string()).into())
    }
    
    /// Get all-time performance statistics
    pub async fn get_all_time_performance(&self) -> AllTimePerformance {
        let cache = self.performance_cache.read().await;
        cache.all_time_performance.clone()
    }
    
    /// Compare performance against benchmarks
    pub async fn compare_with_benchmarks(&self) -> Result<BenchmarkComparison> {
        let cache = self.performance_cache.read().await;
        let benchmarks = self.benchmark_data.read().await;
        
        let portfolio_return = cache.all_time_performance.annualized_return;
        
        // Calculate returns for each benchmark
        let spy_return = self.calculate_benchmark_return(&benchmarks.spy_returns);
        let btc_return = self.calculate_benchmark_return(&benchmarks.btc_returns);
        let eth_return = self.calculate_benchmark_return(&benchmarks.eth_returns);
        let sol_return = self.calculate_benchmark_return(&benchmarks.sol_returns);
        
        Ok(BenchmarkComparison {
            portfolio_return,
            spy_return,
            btc_return,
            eth_return,
            sol_return,
            outperformance_vs_spy: portfolio_return - spy_return,
            outperformance_vs_btc: portfolio_return - btc_return,
            outperformance_vs_eth: portfolio_return - eth_return,
            outperformance_vs_sol: portfolio_return - sol_return,
        })
    }
    
    /// Generate detailed performance analytics
    pub async fn generate_analytics_report(&self) -> Result<AnalyticsReport> {
        let cache = self.performance_cache.read().await;
        let all_time = &cache.all_time_performance;
        
        // Calculate various metrics
        let risk_metrics = RiskMetrics {
            value_at_risk_95: self.calculate_var(&cache.daily_performance, 0.95),
            conditional_var_95: self.calculate_cvar(&cache.daily_performance, 0.95),
            max_drawdown: all_time.max_drawdown,
            current_drawdown: all_time.current_drawdown,
            downside_deviation: self.calculate_downside_deviation(&cache.daily_performance),
            upside_potential_ratio: self.calculate_upside_potential_ratio(&cache.daily_performance),
        };
        
        let efficiency_metrics = EfficiencyMetrics {
            profit_factor: all_time.profit_factor,
            expectancy: all_time.expectancy,
            win_rate: all_time.overall_win_rate,
            average_win_loss_ratio: if all_time.average_loss > Decimal::ZERO {
                (all_time.average_win / all_time.average_loss).to_f64().unwrap_or(0.0)
            } else {
                0.0
            },
            kelly_criterion: self.calculate_kelly_criterion(all_time.overall_win_rate, all_time.average_win, all_time.average_loss),
        };
        
        Ok(AnalyticsReport {
            generated_at: Utc::now(),
            all_time_performance: all_time.clone(),
            risk_metrics,
            efficiency_metrics,
            best_performing_month: self.find_best_month(&cache.monthly_performance),
            worst_performing_month: self.find_worst_month(&cache.monthly_performance),
            consistency_score: self.calculate_consistency_score(&cache.daily_performance),
        })
    }
    
    // Helper methods for calculations and data management
    async fn store_trade_record(&self, _trade: &TradeRecord) -> Result<()> {
        // Database storage implementation
        Ok(())
    }
    
    async fn update_performance_metrics(&self, trade: &TradeRecord) -> Result<()> {
        let mut cache = self.performance_cache.write().await;
        
        // Update all-time metrics
        cache.all_time_performance.total_trades += 1;
        if trade.pnl > Decimal::ZERO {
            cache.all_time_performance.winning_trades += 1;
        } else {
            cache.all_time_performance.losing_trades += 1;
        }
        
        // Update current value
        cache.all_time_performance.current_value += trade.pnl;
        
        // Recalculate metrics
        cache.all_time_performance.overall_win_rate = 
            cache.all_time_performance.winning_trades as f64 / cache.all_time_performance.total_trades as f64;
        
        cache.last_updated = Utc::now();
        
        Ok(())
    }
    
    fn calculate_win_rate(&self, daily_data: &[DailyPerformance]) -> f64 {
        let total_trades: u32 = daily_data.iter().map(|d| d.trades_executed).sum();
        let winning_trades: u32 = daily_data.iter().map(|d| d.winning_trades).sum();
        
        if total_trades == 0 {
            return 0.0;
        }
        
        winning_trades as f64 / total_trades as f64 * 100.0
    }
    
    fn calculate_benchmark_return(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let cumulative_return = returns.iter().fold(1.0, |acc, r| acc * (1.0 + r));
        (cumulative_return.powf(365.0 / returns.len() as f64) - 1.0) * 100.0
    }
    
    fn calculate_var(&self, daily_performance: &BTreeMap<NaiveDate, DailyPerformance>, confidence: f64) -> f64 {
        let mut returns: Vec<f64> = daily_performance.values()
            .map(|d| d.daily_return_percentage)
            .collect();
        returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((1.0 - confidence) * returns.len() as f64) as usize;
        returns.get(index).cloned().unwrap_or(0.0)
    }
    
    fn calculate_cvar(&self, daily_performance: &BTreeMap<NaiveDate, DailyPerformance>, confidence: f64) -> f64 {
        let mut returns: Vec<f64> = daily_performance.values()
            .map(|d| d.daily_return_percentage)
            .collect();
        returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let var_index = ((1.0 - confidence) * returns.len() as f64) as usize;
        let tail_returns = &returns[..=var_index];
        
        if tail_returns.is_empty() {
            return 0.0;
        }
        
        tail_returns.iter().sum::<f64>() / tail_returns.len() as f64
    }
    
    fn calculate_downside_deviation(&self, daily_performance: &BTreeMap<NaiveDate, DailyPerformance>) -> f64 {
        let negative_returns: Vec<f64> = daily_performance.values()
            .filter_map(|d| {
                if d.daily_return_percentage < 0.0 {
                    Some(d.daily_return_percentage.powi(2))
                } else {
                    None
                }
            })
            .collect();
        
        if negative_returns.is_empty() {
            return 0.0;
        }
        
        (negative_returns.iter().sum::<f64>() / negative_returns.len() as f64).sqrt()
    }
    
    fn calculate_upside_potential_ratio(&self, daily_performance: &BTreeMap<NaiveDate, DailyPerformance>) -> f64 {
        let positive_returns: Vec<f64> = daily_performance.values()
            .filter_map(|d| {
                if d.daily_return_percentage > 0.0 {
                    Some(d.daily_return_percentage)
                } else {
                    None
                }
            })
            .collect();
        
        let negative_returns: Vec<f64> = daily_performance.values()
            .filter_map(|d| {
                if d.daily_return_percentage < 0.0 {
                    Some(d.daily_return_percentage.abs())
                } else {
                    None
                }
            })
            .collect();
        
        if negative_returns.is_empty() {
            return f64::INFINITY;
        }
        
        let avg_positive = positive_returns.iter().sum::<f64>() / positive_returns.len().max(1) as f64;
        let avg_negative = negative_returns.iter().sum::<f64>() / negative_returns.len() as f64;
        
        avg_positive / avg_negative
    }
    
    fn calculate_kelly_criterion(&self, win_rate: f64, avg_win: Decimal, avg_loss: Decimal) -> f64 {
        if avg_loss == Decimal::ZERO {
            return 0.0;
        }
        
        let b = (avg_win / avg_loss).to_f64().unwrap_or(0.0);
        let p = win_rate / 100.0;
        let q = 1.0 - p;
        
        (p * b - q) / b
    }
    
    fn find_best_month(&self, monthly_performance: &BTreeMap<String, MonthlyPerformance>) -> Option<String> {
        monthly_performance.iter()
            .max_by(|a, b| a.1.monthly_return_percentage.partial_cmp(&b.1.monthly_return_percentage).unwrap())
            .map(|(month, _)| month.clone())
    }
    
    fn find_worst_month(&self, monthly_performance: &BTreeMap<String, MonthlyPerformance>) -> Option<String> {
        monthly_performance.iter()
            .min_by(|a, b| a.1.monthly_return_percentage.partial_cmp(&b.1.monthly_return_percentage).unwrap())
            .map(|(month, _)| month.clone())
    }
    
    fn calculate_consistency_score(&self, daily_performance: &BTreeMap<NaiveDate, DailyPerformance>) -> f64 {
        let returns: Vec<f64> = daily_performance.values()
            .map(|d| d.daily_return_percentage)
            .collect();
        
        if returns.is_empty() {
            return 0.0;
        }
        
        let positive_days = returns.iter().filter(|&&r| r > 0.0).count();
        let consistency_ratio = positive_days as f64 / returns.len() as f64;
        
        let variance = self.metrics_calculator.calculate_variance(&returns);
        let stability_score = 1.0 / (1.0 + variance);
        
        (consistency_ratio * 0.6 + stability_score * 0.4) * 100.0
    }
}

/// Performance report for a specific period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub period: String,
    pub starting_value: Decimal,
    pub ending_value: Decimal,
    pub total_return: Decimal,
    pub total_return_percentage: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub total_trades: u32,
    pub win_rate: f64,
    pub daily_performance: Vec<DailyPerformance>,
}

/// Benchmark comparison results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub portfolio_return: f64,
    pub spy_return: f64,
    pub btc_return: f64,
    pub eth_return: f64,
    pub sol_return: f64,
    pub outperformance_vs_spy: f64,
    pub outperformance_vs_btc: f64,
    pub outperformance_vs_eth: f64,
    pub outperformance_vs_sol: f64,
}

/// Comprehensive analytics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsReport {
    pub generated_at: DateTime<Utc>,
    pub all_time_performance: AllTimePerformance,
    pub risk_metrics: RiskMetrics,
    pub efficiency_metrics: EfficiencyMetrics,
    pub best_performing_month: Option<String>,
    pub worst_performing_month: Option<String>,
    pub consistency_score: f64,
}

/// Risk-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub value_at_risk_95: f64,
    pub conditional_var_95: f64,
    pub max_drawdown: f64,
    pub current_drawdown: f64,
    pub downside_deviation: f64,
    pub upside_potential_ratio: f64,
}

/// Trading efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyMetrics {
    pub profit_factor: f64,
    pub expectancy: Decimal,
    pub win_rate: f64,
    pub average_win_loss_ratio: f64,
    pub kelly_criterion: f64,
}

impl Default for AllTimePerformance {
    fn default() -> Self {
        Self {
            start_date: Utc::now(),
            initial_capital: Decimal::from(10000),
            current_value: Decimal::from(10000),
            total_return: Decimal::ZERO,
            total_return_percentage: 0.0,
            annualized_return: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            overall_win_rate: 0.0,
            average_win: Decimal::ZERO,
            average_loss: Decimal::ZERO,
            profit_factor: 0.0,
            expectancy: Decimal::ZERO,
            max_drawdown: 0.0,
            max_drawdown_date: Utc::now(),
            current_drawdown: 0.0,
            time_in_market: Duration::zero(),
            best_trade_ever: None,
            worst_trade_ever: None,
        }
    }
}