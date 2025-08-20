mod realtime_client;
mod price_stream;
mod portfolio_stream;

pub use realtime_client::{
    WebSocketClient,
    WebSocketConfig,
    ConnectionStatus,
    ReconnectStrategy,
    WebSocketMessage,
    MessageHandler,
    SubscriptionType,
    SubscriptionRequest,
    StreamData,
    ErrorHandler,
};

pub use price_stream::{
    PriceStreamManager,
    PriceUpdate,
    PriceSubscription,
    AggregatedPrice,
    PriceSource,
    OHLCV,
    TickData,
    OrderBook,
    OrderBookLevel,
    MarketDepth,
};

pub use portfolio_stream::{
    PortfolioStreamManager,
    PortfolioUpdate,
    PositionUpdate,
    BalanceUpdate,
    PnLUpdate,
    OrderUpdate,
    TradeExecution,
    RiskMetricsUpdate,
    AlertTrigger,
};