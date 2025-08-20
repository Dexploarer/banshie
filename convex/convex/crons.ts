import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();

// ============================================
// PRICE UPDATES - Critical for real-time data
// ============================================

// Update major token prices every minute
crons.interval(
  "update_major_prices",
  { minutes: 1 },
  internal.actions.prices.updateMajorTokenPrices
);

// Update all tracked prices every 5 minutes
crons.interval(
  "update_all_prices",
  { minutes: 5 },
  internal.actions.prices.updateAllTrackedPrices
);

// ============================================
// DCA EXECUTION - Core automation feature
// ============================================

// Check and execute DCA strategies every 5 minutes
crons.interval(
  "execute_dca_strategies",
  { minutes: 5 },
  internal.actions.dca.executeScheduledStrategies
);

// Process value averaging strategies hourly
crons.hourly(
  "value_averaging_dca",
  { minuteUTC: 0 },
  internal.actions.dca.processValueAveraging
);

// ============================================
// PORTFOLIO MANAGEMENT
// ============================================

// Sync wallet balances every 15 minutes
crons.interval(
  "sync_wallets",
  { minutes: 15 },
  internal.actions.portfolio.syncAllWalletBalances
);

// Calculate portfolio metrics hourly
crons.hourly(
  "calculate_metrics",
  { minuteUTC: 30 },
  internal.actions.portfolio.calculatePerformanceMetrics
);

// Portfolio rebalancing check (daily)
crons.daily(
  "check_rebalancing",
  { hourUTC: 14, minuteUTC: 0 }, // 2 PM UTC
  internal.actions.portfolio.checkRebalancingNeeds
);

// ============================================
// ALERTS & MONITORING
// ============================================

// Check price alerts every minute
crons.interval(
  "check_price_alerts",
  { minutes: 1 },
  internal.actions.alerts.checkPriceAlerts
);

// Monitor market events every 5 minutes
crons.interval(
  "monitor_market_events",
  { minutes: 5 },
  internal.actions.alerts.monitorMarketEvents
);

// Check stop-loss and take-profit orders
crons.interval(
  "check_conditional_orders",
  { minutes: 1 },
  internal.actions.orders.checkConditionalOrders
);

// ============================================
// AI & ANALYTICS
// ============================================

// Update technical indicators every 15 minutes
crons.interval(
  "calculate_indicators",
  { minutes: 15 },
  internal.actions.analytics.calculateTechnicalIndicators
);

// Generate AI trading signals hourly
crons.hourly(
  "generate_signals",
  { minuteUTC: 15 },
  internal.actions.ai.generateTradingSignals
);

// Update sentiment analysis every 30 minutes
crons.interval(
  "update_sentiment",
  { minutes: 30 },
  internal.actions.ai.updateSentimentAnalysis
);

// ============================================
// REPORTING & NOTIFICATIONS
// ============================================

// Send daily summary reports
crons.daily(
  "daily_reports",
  { hourUTC: 12, minuteUTC: 0 }, // Noon UTC
  internal.actions.reporting.sendDailyReports
);

// Weekly performance reports
crons.weekly(
  "weekly_reports",
  { dayOfWeek: 1, hourUTC: 14, minuteUTC: 0 }, // Monday 2 PM UTC
  internal.actions.reporting.sendWeeklyReports
);

// ============================================
// MAINTENANCE & CLEANUP
// ============================================

// Clean up old price data (keep 30 days)
crons.daily(
  "cleanup_prices",
  { hourUTC: 3, minuteUTC: 0 },
  internal.actions.maintenance.cleanupOldPrices
);

// Archive completed orders (older than 90 days)
crons.weekly(
  "archive_orders",
  { dayOfWeek: 0, hourUTC: 4, minuteUTC: 0 }, // Sunday 4 AM UTC
  internal.actions.maintenance.archiveOldOrders
);

// Clean expired sessions
crons.daily(
  "cleanup_sessions",
  { hourUTC: 2, minuteUTC: 30 },
  internal.actions.maintenance.cleanupExpiredSessions
);

// Optimize database indexes
crons.weekly(
  "optimize_database",
  { dayOfWeek: 0, hourUTC: 5, minuteUTC: 0 }, // Sunday 5 AM UTC
  internal.actions.maintenance.optimizeDatabase
);

// ============================================
// RISK MANAGEMENT
// ============================================

// Check risk limits every 10 minutes
crons.interval(
  "check_risk_limits",
  { minutes: 10 },
  internal.actions.risk.checkUserRiskLimits
);

// Monitor position exposure hourly
crons.hourly(
  "monitor_exposure",
  { minuteUTC: 45 },
  internal.actions.risk.monitorPositionExposure
);

// ============================================
// SPECIAL SCHEDULES
// ============================================

// Market open actions (considering crypto is 24/7, but for traditional market correlation)
crons.daily(
  "market_open_actions",
  { hourUTC: 13, minuteUTC: 30 }, // 9:30 AM EST
  internal.actions.market.handleMarketOpen
);

// Market close actions
crons.daily(
  "market_close_actions",
  { hourUTC: 20, minuteUTC: 0 }, // 4:00 PM EST
  internal.actions.market.handleMarketClose
);

// Weekend DCA boost (execute pending strategies)
crons.weekly(
  "weekend_dca",
  { dayOfWeek: 6, hourUTC: 10, minuteUTC: 0 }, // Saturday 10 AM UTC
  internal.actions.dca.weekendBoost
);

// Month-end portfolio snapshot
crons.monthly(
  "monthly_snapshot",
  { dayOfMonth: "last", hourUTC: 23, minuteUTC: 45 },
  internal.actions.portfolio.createMonthlySnapshot
);

export default crons;