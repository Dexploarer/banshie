use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use crate::db::Database;
use crate::errors::BotError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderStats {
    pub user_id: i64,
    pub username: String,
    pub wallet_address: String,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub total_volume_sol: f64,
    pub total_profit_sol: f64,
    pub total_profit_percent: f64,
    pub win_rate: f64,
    pub avg_profit_per_trade: f64,
    pub best_trade: Trade,
    pub worst_trade: Trade,
    pub streak_current: i32, // Positive for wins, negative for losses
    pub streak_best: i32,
    pub last_trade_time: DateTime<Utc>,
    pub rank_global: u32,
    pub rank_weekly: u32,
    pub rank_daily: u32,
    pub badges: Vec<Badge>,
    pub copy_traders_count: u32,
    pub performance_7d: f64,
    pub performance_30d: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub token_symbol: String,
    pub token_address: String,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub amount_sol: f64,
    pub profit_sol: f64,
    pub profit_percent: f64,
    pub timestamp: DateTime<Utc>,
    pub trade_type: TradeType,
    pub status: TradeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeType {
    Buy,
    Sell,
    QuickBuy,
    QuickSell,
    Snipe,
    CopyTrade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeStatus {
    Open,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Badge {
    TopTrader,
    DiamondHands,
    Sniper,
    VolumeKing,
    ProfitMaster,
    WinStreak(u32),
    EarlyAdopter,
    RiskTaker,
    Consistent,
    Whale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub user_id: i64,
    pub username: String,
    pub profit_percent: f64,
    pub total_trades: u32,
    pub win_rate: f64,
    pub volume_sol: f64,
    pub badges: Vec<Badge>,
    pub is_copyable: bool,
    pub copy_fee_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeaderboardPeriod {
    Daily,
    Weekly,
    Monthly,
    AllTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeaderboardMetric {
    Profit,
    Volume,
    WinRate,
    TradeCount,
    SharpeRatio,
}

/// Manages trading leaderboards and trader statistics
pub struct LeaderboardManager {
    db: Arc<Database>,
    cache: Arc<RwLock<LeaderboardCache>>,
    stats_cache: Arc<RwLock<HashMap<i64, TraderStats>>>,
}

struct LeaderboardCache {
    daily: Vec<LeaderboardEntry>,
    weekly: Vec<LeaderboardEntry>,
    monthly: Vec<LeaderboardEntry>,
    all_time: Vec<LeaderboardEntry>,
    last_update: DateTime<Utc>,
}

impl LeaderboardManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(LeaderboardCache {
                daily: Vec::new(),
                weekly: Vec::new(),
                monthly: Vec::new(),
                all_time: Vec::new(),
                last_update: Utc::now() - Duration::hours(1),
            })),
            stats_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get leaderboard for a specific period and metric
    pub async fn get_leaderboard(
        &self,
        period: LeaderboardPeriod,
        metric: LeaderboardMetric,
        limit: usize,
    ) -> Result<Vec<LeaderboardEntry>> {
        // Check cache
        let cache = self.cache.read().await;
        if cache.last_update > Utc::now() - Duration::minutes(5) {
            let cached = match period {
                LeaderboardPeriod::Daily => &cache.daily,
                LeaderboardPeriod::Weekly => &cache.weekly,
                LeaderboardPeriod::Monthly => &cache.monthly,
                LeaderboardPeriod::AllTime => &cache.all_time,
            };
            
            if !cached.is_empty() {
                return Ok(cached.iter().take(limit).cloned().collect());
            }
        }
        drop(cache);

        // Fetch from database
        self.update_leaderboard_cache().await?;
        
        let cache = self.cache.read().await;
        let leaderboard = match period {
            LeaderboardPeriod::Daily => &cache.daily,
            LeaderboardPeriod::Weekly => &cache.weekly,
            LeaderboardPeriod::Monthly => &cache.monthly,
            LeaderboardPeriod::AllTime => &cache.all_time,
        };
        
        // Sort by specified metric
        let mut sorted = leaderboard.clone();
        match metric {
            LeaderboardMetric::Profit => {
                sorted.sort_by(|a, b| b.profit_percent.partial_cmp(&a.profit_percent).unwrap());
            }
            LeaderboardMetric::Volume => {
                sorted.sort_by(|a, b| b.volume_sol.partial_cmp(&a.volume_sol).unwrap());
            }
            LeaderboardMetric::WinRate => {
                sorted.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap());
            }
            LeaderboardMetric::TradeCount => {
                sorted.sort_by(|a, b| b.total_trades.cmp(&a.total_trades));
            }
            LeaderboardMetric::SharpeRatio => {
                // Would need Sharpe ratio in LeaderboardEntry
                sorted.sort_by(|a, b| b.profit_percent.partial_cmp(&a.profit_percent).unwrap());
            }
        }
        
        Ok(sorted.into_iter().take(limit).collect())
    }

    /// Update leaderboard cache from database
    async fn update_leaderboard_cache(&self) -> Result<()> {
        info!("Updating leaderboard cache");
        
        // In production, these would be actual database queries
        // For now, generate sample data
        let daily = self.generate_sample_leaderboard(LeaderboardPeriod::Daily);
        let weekly = self.generate_sample_leaderboard(LeaderboardPeriod::Weekly);
        let monthly = self.generate_sample_leaderboard(LeaderboardPeriod::Monthly);
        let all_time = self.generate_sample_leaderboard(LeaderboardPeriod::AllTime);
        
        let mut cache = self.cache.write().await;
        cache.daily = daily;
        cache.weekly = weekly;
        cache.monthly = monthly;
        cache.all_time = all_time;
        cache.last_update = Utc::now();
        
        Ok(())
    }

    /// Generate sample leaderboard data
    fn generate_sample_leaderboard(&self, period: LeaderboardPeriod) -> Vec<LeaderboardEntry> {
        let base_multiplier = match period {
            LeaderboardPeriod::Daily => 1.0,
            LeaderboardPeriod::Weekly => 2.5,
            LeaderboardPeriod::Monthly => 5.0,
            LeaderboardPeriod::AllTime => 10.0,
        };
        
        vec![
            LeaderboardEntry {
                rank: 1,
                user_id: 1001,
                username: "AlphaTrader".to_string(),
                profit_percent: 127.5 * base_multiplier,
                total_trades: (23.0 * base_multiplier) as u32,
                win_rate: 78.3,
                volume_sol: 1250.0 * base_multiplier,
                badges: vec![Badge::TopTrader, Badge::WinStreak(12), Badge::ProfitMaster],
                is_copyable: true,
                copy_fee_percent: 10.0,
            },
            LeaderboardEntry {
                rank: 2,
                user_id: 1002,
                username: "DiamondHands".to_string(),
                profit_percent: 89.2 * base_multiplier,
                total_trades: (31.0 * base_multiplier) as u32,
                win_rate: 71.0,
                volume_sol: 890.5 * base_multiplier,
                badges: vec![Badge::DiamondHands, Badge::Consistent],
                is_copyable: true,
                copy_fee_percent: 8.0,
            },
            LeaderboardEntry {
                rank: 3,
                user_id: 1003,
                username: "DegenKing".to_string(),
                profit_percent: 76.8 * base_multiplier,
                total_trades: (45.0 * base_multiplier) as u32,
                win_rate: 62.2,
                volume_sol: 2100.0 * base_multiplier,
                badges: vec![Badge::VolumeKing, Badge::RiskTaker],
                is_copyable: true,
                copy_fee_percent: 7.0,
            },
            LeaderboardEntry {
                rank: 4,
                user_id: 1004,
                username: "SnipeMaster".to_string(),
                profit_percent: 65.3 * base_multiplier,
                total_trades: (18.0 * base_multiplier) as u32,
                win_rate: 83.3,
                volume_sol: 450.0 * base_multiplier,
                badges: vec![Badge::Sniper, Badge::WinStreak(8)],
                is_copyable: true,
                copy_fee_percent: 12.0,
            },
            LeaderboardEntry {
                rank: 5,
                user_id: 1005,
                username: "HODLLegend".to_string(),
                profit_percent: 58.9 * base_multiplier,
                total_trades: (12.0 * base_multiplier) as u32,
                win_rate: 75.0,
                volume_sol: 3500.0 * base_multiplier,
                badges: vec![Badge::DiamondHands, Badge::Whale],
                is_copyable: false,
                copy_fee_percent: 0.0,
            },
        ]
    }

    /// Get trader statistics
    pub async fn get_trader_stats(&self, user_id: i64) -> Result<TraderStats> {
        // Check cache
        let cache = self.stats_cache.read().await;
        if let Some(stats) = cache.get(&user_id) {
            return Ok(stats.clone());
        }
        drop(cache);

        // Fetch from database or generate sample
        let stats = self.generate_sample_trader_stats(user_id);
        
        // Update cache
        let mut cache = self.stats_cache.write().await;
        cache.insert(user_id, stats.clone());
        
        Ok(stats)
    }

    /// Generate sample trader statistics
    fn generate_sample_trader_stats(&self, user_id: i64) -> TraderStats {
        TraderStats {
            user_id,
            username: format!("Trader{}", user_id),
            wallet_address: format!("{}...{}", "Demo", user_id),
            total_trades: 47,
            winning_trades: 28,
            losing_trades: 19,
            total_volume_sol: 587.5,
            total_profit_sol: 45.2,
            total_profit_percent: 12.3,
            win_rate: 59.6,
            avg_profit_per_trade: 0.96,
            best_trade: Trade {
                token_symbol: "BONK".to_string(),
                token_address: "DemoAddr123".to_string(),
                entry_price: 0.000012,
                exit_price: Some(0.000028),
                amount_sol: 10.0,
                profit_sol: 13.3,
                profit_percent: 133.0,
                timestamp: Utc::now() - Duration::days(2),
                trade_type: TradeType::Snipe,
                status: TradeStatus::Closed,
            },
            worst_trade: Trade {
                token_symbol: "SCAM".to_string(),
                token_address: "DemoAddr456".to_string(),
                entry_price: 0.15,
                exit_price: Some(0.03),
                amount_sol: 5.0,
                profit_sol: -4.0,
                profit_percent: -80.0,
                timestamp: Utc::now() - Duration::days(5),
                trade_type: TradeType::QuickBuy,
                status: TradeStatus::Closed,
            },
            streak_current: 3,
            streak_best: 7,
            last_trade_time: Utc::now() - Duration::hours(2),
            rank_global: 47,
            rank_weekly: 23,
            rank_daily: 12,
            badges: vec![Badge::EarlyAdopter, Badge::Consistent],
            copy_traders_count: 0,
            performance_7d: 8.5,
            performance_30d: 12.3,
            sharpe_ratio: 1.45,
            max_drawdown_percent: -15.2,
        }
    }

    /// Record a new trade
    pub async fn record_trade(&self, user_id: i64, trade: Trade) -> Result<()> {
        // Update database
        // In production, this would be an actual database insert
        debug!("Recording trade for user {}: {:?}", user_id, trade);
        
        // Update cached stats
        let mut cache = self.stats_cache.write().await;
        if let Some(stats) = cache.get_mut(&user_id) {
            stats.total_trades += 1;
            
            if trade.profit_sol > 0.0 {
                stats.winning_trades += 1;
                stats.streak_current = stats.streak_current.max(0) + 1;
                stats.streak_best = stats.streak_best.max(stats.streak_current);
            } else {
                stats.losing_trades += 1;
                stats.streak_current = stats.streak_current.min(0) - 1;
            }
            
            stats.total_volume_sol += trade.amount_sol;
            stats.total_profit_sol += trade.profit_sol;
            stats.total_profit_percent = (stats.total_profit_sol / stats.total_volume_sol) * 100.0;
            stats.win_rate = (stats.winning_trades as f64 / stats.total_trades as f64) * 100.0;
            stats.avg_profit_per_trade = stats.total_profit_sol / stats.total_trades as f64;
            stats.last_trade_time = trade.timestamp;
            
            if trade.profit_sol > stats.best_trade.profit_sol {
                stats.best_trade = trade.clone();
            }
            if trade.profit_sol < stats.worst_trade.profit_sol {
                stats.worst_trade = trade.clone();
            }
        }
        
        Ok(())
    }

    /// Get top traders for copy trading
    pub async fn get_copyable_traders(&self, limit: usize) -> Result<Vec<LeaderboardEntry>> {
        let leaderboard = self.get_leaderboard(
            LeaderboardPeriod::Weekly,
            LeaderboardMetric::Profit,
            limit * 2,
        ).await?;
        
        Ok(leaderboard
            .into_iter()
            .filter(|entry| entry.is_copyable)
            .take(limit)
            .collect())
    }

    /// Format leaderboard for display
    pub fn format_leaderboard(
        &self,
        entries: &[LeaderboardEntry],
        period: LeaderboardPeriod,
        user_stats: Option<&TraderStats>,
    ) -> String {
        let period_text = match period {
            LeaderboardPeriod::Daily => "Today",
            LeaderboardPeriod::Weekly => "This Week",
            LeaderboardPeriod::Monthly => "This Month",
            LeaderboardPeriod::AllTime => "All Time",
        };
        
        let mut message = format!("🏆 **Top Traders - {}**\n\n", period_text);
        
        for entry in entries {
            let medal = match entry.rank {
                1 => "🥇",
                2 => "🥈",
                3 => "🥉",
                _ => "🎯",
            };
            
            let badges_str = entry.badges
                .iter()
                .map(|b| match b {
                    Badge::TopTrader => "👑",
                    Badge::DiamondHands => "💎",
                    Badge::Sniper => "🎯",
                    Badge::VolumeKing => "📊",
                    Badge::ProfitMaster => "💰",
                    Badge::WinStreak(_) => "🔥",
                    Badge::EarlyAdopter => "🌟",
                    Badge::RiskTaker => "🎲",
                    Badge::Consistent => "📈",
                    Badge::Whale => "🐋",
                })
                .collect::<Vec<_>>()
                .join("");
            
            message.push_str(&format!(
                "{}. {} {} {} +{:.1}% ({} trades, {:.1}% WR)\n",
                entry.rank,
                medal,
                entry.username,
                badges_str,
                entry.profit_percent,
                entry.total_trades,
                entry.win_rate
            ));
            
            if entry.is_copyable {
                message.push_str(&format!(
                    "   💫 Copy available ({}% fee)\n",
                    entry.copy_fee_percent
                ));
            }
        }
        
        if let Some(stats) = user_stats {
            let rank = match period {
                LeaderboardPeriod::Daily => stats.rank_daily,
                LeaderboardPeriod::Weekly => stats.rank_weekly,
                _ => stats.rank_global,
            };
            
            message.push_str(&format!(
                "\n📍 **Your Position**\n\
                Rank: #{} (+{:.1}%, {} trades)\n\
                Win Rate: {:.1}%\n\
                Current Streak: {}\n",
                rank,
                stats.total_profit_percent,
                stats.total_trades,
                stats.win_rate,
                if stats.streak_current > 0 {
                    format!("🔥 {} wins", stats.streak_current)
                } else if stats.streak_current < 0 {
                    format!("❄️ {} losses", stats.streak_current.abs())
                } else {
                    "➖ Neutral".to_string()
                }
            ));
        }
        
        message.push_str("\n💡 Use `/copy <username>` to follow top traders");
        
        message
    }
}