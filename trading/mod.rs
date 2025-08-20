mod executor;
mod backrun;
mod dex;
mod types;
mod token_resolver;
mod token_2022;
mod token_creator;
mod leaderboard;
mod copy_trading;
mod copy_monitor;
mod swaps;
mod signer;
mod dca;
mod dca_scheduler;
mod dca_risk_strategies;
mod orders;
mod trailing_stops;

pub use executor::{TradingEngine, TradingEngineHandle, TradingMessage};
pub use types::{TradeResult, Balance, Position, TokenRestrictions};
pub use token_resolver::TokenResolver;
pub use token_2022::{Token2022Manager, Token2022Info, ExtensionType, TransferFeeConfig, InterestBearingConfig, TokenMetadata};
pub use token_creator::{TokenCreator, TokenCreationConfig, TokenCreationResult, TokenPreset};
pub use leaderboard::{LeaderboardManager, LeaderboardEntry, LeaderboardPeriod, LeaderboardMetric, TraderStats, Trade, TradeType, TradeStatus, Badge};
pub use copy_trading::{CopyTradingManager, CopyTradingConfig, MasterTrader, CopyTradeExecution, CopyTradeType, CopyTradeStatus, TradingStyle};
pub use copy_monitor::{CopyTradingMonitor, BlockchainTradeMonitor};
pub use swaps::{JupiterSwapClient, SwapRequest, SwapResult, JupiterQuote, TokenInfo};
pub use signer::{TransactionSigner, SigningOptions, SigningRequest, SigningResult};
pub use dca::{
    DCAEngine, 
    DCAStrategy, 
    DCAInterval, 
    DCAStrategyType, 
    DCAStatus,
    RiskParameters,
    AdvancedDCAConfig,
    DCAExecution,
    DCAPerformance,
    ExecutionReason,
    MarketConditions,
    GridLevel
};
pub use dca_scheduler::{
    DCAScheduler,
    ScheduledExecution,
    ExecutionType,
    ScheduleConfig,
    ScheduleType,
    MarketEvent,
    PriceCondition,
    PriceConditionType,
    TechnicalIndicator,
    IndicatorType,
    IndicatorCondition,
    ExecutionCondition,
    ConditionType,
    TimeWindow,
    NotificationConfig,
    NotificationChannel,
    TimezoneManager,
    MarketHoursManager,
    ExecutionStats,
    ExecutionRecord
};
pub use dca_risk_strategies::{
    RiskBasedDCAManager,
    RiskModel,
    RiskModelType,
    RiskParameters as DCARiskParameters,
    PricePoint,
    RiskModelMetrics,
    MarketRegimeDetector,
    MarketRegime,
    VolatilityCalculator,
    VolatilityMetrics,
    CorrelationAnalyzer,
    RiskAdjustedRecommendation,
    RiskFactor,
    RiskFactorType,
    RiskSeverity,
    HedgingSuggestion,
    HedgeType
};
pub use orders::{
    OrderManager,
    Order,
    OrderType,
    OrderSide,
    TimeInForce,
    OrderStatus,
    TriggerConditions,
    PriceCondition,
    PriceConditionType,
    MovingAverageType,
    PriceSource,
    VolumeCondition,
    VolumeConditionType,
    TimeCondition,
    TimeConditionType,
    TechnicalCondition,
    TechnicalIndicator,
    IndicatorCondition,
    ConditionLogic,
    ExecutionConfig,
    RetryConfig,
    RetryCondition,
    GasOptimization,
    PriorityFeeStrategy,
    OrderRiskManagement,
    PositionSizingRules,
    CorrelationLimits,
    PartialFillConfig,
    OrderMetadata,
    OrderExecution,
    ExecutionType,
    TriggerReason,
    MarketConditions as OrderMarketConditions,
    NetworkCongestion,
    PriceMonitor,
    PricePoint as OrderPricePoint
};
pub use trailing_stops::{
    TrailingStopManager,
    TrailingStopState,
    TrailingStrategy,
    PositionSide,
    TrailingStopStatus,
    TrailingPerformanceMetrics,
    TrailingRiskControls,
    PriceTracker,
    PriceCandle,
    VolatilityMetrics,
    TechnicalLevels,
    SupportResistanceLevel,
    TrendDirection,
    TimeCurveType
};