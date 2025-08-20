use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{info, debug, warn};

use crate::errors::Result;
use crate::websocket::realtime_client::{
    WebSocketClient, StreamData, MessageHandler, SubscriptionRequest, SubscriptionType,
};

/// Real-time portfolio updates manager
#[derive(Clone)]
pub struct PortfolioStreamManager {
    ws_client: Arc<WebSocketClient>,
    portfolio_state: Arc<RwLock<PortfolioState>>,
    update_subscribers: Arc<RwLock<HashMap<String, broadcast::Sender<PortfolioUpdate>>>>,
    alert_subscribers: Arc<RwLock<Vec<broadcast::Sender<AlertTrigger>>>>,
}

/// Current portfolio state
#[derive(Debug, Clone)]
pub struct PortfolioState {
    pub account: String,
    pub total_value: Decimal,
    pub available_balance: Decimal,
    pub positions: HashMap<String, Position>,
    pub open_orders: HashMap<String, OpenOrder>,
    pub pnl: PnLSummary,
    pub risk_metrics: RiskMetrics,
    pub last_update: DateTime<Utc>,
}

/// Position information
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_percentage: f64,
    pub realized_pnl: Decimal,
    pub cost_basis: Decimal,
}

/// Open order information
#[derive(Debug, Clone)]
pub struct OpenOrder {
    pub order_id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
    TrailingStop,
}

#[derive(Debug, Clone)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// P&L summary
#[derive(Debug, Clone)]
pub struct PnLSummary {
    pub daily_pnl: Decimal,
    pub daily_pnl_percentage: f64,
    pub weekly_pnl: Decimal,
    pub weekly_pnl_percentage: f64,
    pub monthly_pnl: Decimal,
    pub monthly_pnl_percentage: f64,
    pub yearly_pnl: Decimal,
    pub yearly_pnl_percentage: f64,
    pub all_time_pnl: Decimal,
    pub all_time_pnl_percentage: f64,
}

/// Risk metrics
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub portfolio_beta: f64,
    pub portfolio_volatility: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub current_drawdown: f64,
    pub value_at_risk: f64,
    pub position_concentration: HashMap<String, f64>,
    pub correlation_risk: f64,
}

/// Portfolio update types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PortfolioUpdate {
    Position(PositionUpdate),
    Balance(BalanceUpdate),
    PnL(PnLUpdate),
    Order(OrderUpdate),
    Trade(TradeExecution),
    RiskMetrics(RiskMetricsUpdate),
    Alert(AlertTrigger),
}

/// Position update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdate {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub action: PositionAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionAction {
    Opened,
    Increased,
    Decreased,
    Closed,
    Updated,
}

/// Balance update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceUpdate {
    pub timestamp: DateTime<Utc>,
    pub available_balance: Decimal,
    pub total_equity: Decimal,
    pub margin_used: Decimal,
    pub margin_available: Decimal,
    pub change_amount: Decimal,
    pub change_reason: BalanceChangeReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BalanceChangeReason {
    Deposit,
    Withdrawal,
    Trade,
    Fee,
    Interest,
    Liquidation,
}

/// P&L update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLUpdate {
    pub timestamp: DateTime<Utc>,
    pub timeframe: PnLTimeframe,
    pub pnl_amount: Decimal,
    pub pnl_percentage: f64,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PnLTimeframe {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    AllTime,
}

/// Order update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdate {
    pub timestamp: DateTime<Utc>,
    pub order_id: String,
    pub symbol: String,
    pub order_type: String,
    pub side: String,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
    pub filled_quantity: Decimal,
    pub status: String,
    pub message: Option<String>,
}

/// Trade execution notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub timestamp: DateTime<Utc>,
    pub trade_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub pnl: Option<Decimal>,
}

/// Risk metrics update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetricsUpdate {
    pub timestamp: DateTime<Utc>,
    pub metric_type: RiskMetricType,
    pub value: f64,
    pub threshold: Option<f64>,
    pub status: RiskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskMetricType {
    Volatility,
    Drawdown,
    VaR,
    Concentration,
    Correlation,
    Leverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskStatus {
    Normal,
    Warning,
    Critical,
}

/// Alert trigger notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertTrigger {
    pub timestamp: DateTime<Utc>,
    pub alert_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub action_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    PriceAlert,
    VolumeAlert,
    PnLAlert,
    RiskAlert,
    OrderAlert,
    SystemAlert,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// WebSocket message handler for portfolio updates
pub struct PortfolioMessageHandler {
    manager: Arc<PortfolioStreamManager>,
    subscription_id: String,
}

#[async_trait]
impl MessageHandler for PortfolioMessageHandler {
    async fn handle_message(&self, message: StreamData) -> Result<()> {
        // Parse portfolio update from message
        let update: PortfolioUpdate = serde_json::from_value(message.data)?;
        
        // Process the update
        self.manager.process_portfolio_update(update).await?;
        
        Ok(())
    }
    
    fn subscription_id(&self) -> String {
        self.subscription_id.clone()
    }
}

impl PortfolioStreamManager {
    /// Create new portfolio stream manager
    pub fn new(ws_client: Arc<WebSocketClient>, account: String) -> Self {
        info!("💼 Initializing portfolio stream manager for account: {}", account);
        
        let initial_state = PortfolioState {
            account,
            total_value: Decimal::ZERO,
            available_balance: Decimal::ZERO,
            positions: HashMap::new(),
            open_orders: HashMap::new(),
            pnl: PnLSummary::default(),
            risk_metrics: RiskMetrics::default(),
            last_update: Utc::now(),
        };
        
        Self {
            ws_client,
            portfolio_state: Arc::new(RwLock::new(initial_state)),
            update_subscribers: Arc::new(RwLock::new(HashMap::new())),
            alert_subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Subscribe to portfolio updates
    pub async fn subscribe_portfolio(&self, account: &str) -> Result<broadcast::Receiver<PortfolioUpdate>> {
        info!("💼 Subscribing to portfolio updates for account: {}", account);
        
        // Create broadcast channel
        let (tx, rx) = broadcast::channel(1000);
        
        // Store subscriber
        let mut subscribers = self.update_subscribers.write().await;
        subscribers.insert(account.to_string(), tx);
        
        // Create WebSocket subscription
        let ws_subscription = SubscriptionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::Portfolio {
                account: account.to_string(),
            },
            params: HashMap::new(),
            filters: None,
        };
        
        // Register message handler
        let handler = Arc::new(PortfolioMessageHandler {
            manager: Arc::new(self.clone()),
            subscription_id: ws_subscription.id.clone(),
        });
        
        self.ws_client.register_handler(handler).await;
        
        // Subscribe via WebSocket
        self.ws_client.subscribe("portfolio_stream", ws_subscription).await?;
        
        // Also subscribe to orders and positions
        self.subscribe_orders(account).await?;
        self.subscribe_positions(account).await?;
        
        Ok(rx)
    }
    
    /// Subscribe to order updates
    async fn subscribe_orders(&self, account: &str) -> Result<()> {
        let ws_subscription = SubscriptionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::Orders {
                account: account.to_string(),
            },
            params: HashMap::new(),
            filters: None,
        };
        
        self.ws_client.subscribe("order_stream", ws_subscription).await
    }
    
    /// Subscribe to position updates
    async fn subscribe_positions(&self, account: &str) -> Result<()> {
        let ws_subscription = SubscriptionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::Positions {
                account: account.to_string(),
            },
            params: HashMap::new(),
            filters: None,
        };
        
        self.ws_client.subscribe("position_stream", ws_subscription).await
    }
    
    /// Subscribe to alerts
    pub async fn subscribe_alerts(&self) -> broadcast::Receiver<AlertTrigger> {
        let (tx, rx) = broadcast::channel(100);
        
        let mut subscribers = self.alert_subscribers.write().await;
        subscribers.push(tx);
        
        rx
    }
    
    /// Process portfolio update
    async fn process_portfolio_update(&self, update: PortfolioUpdate) -> Result<()> {
        debug!("💼 Processing portfolio update: {:?}", update);
        
        match &update {
            PortfolioUpdate::Position(pos_update) => {
                self.update_position(pos_update).await?;
            },
            PortfolioUpdate::Balance(bal_update) => {
                self.update_balance(bal_update).await?;
            },
            PortfolioUpdate::PnL(pnl_update) => {
                self.update_pnl(pnl_update).await?;
            },
            PortfolioUpdate::Order(order_update) => {
                self.update_order(order_update).await?;
            },
            PortfolioUpdate::Trade(trade) => {
                self.process_trade_execution(trade).await?;
            },
            PortfolioUpdate::RiskMetrics(risk_update) => {
                self.update_risk_metrics(risk_update).await?;
            },
            PortfolioUpdate::Alert(alert) => {
                self.process_alert(alert).await?;
            },
        }
        
        // Broadcast update to subscribers
        self.broadcast_update(update).await?;
        
        Ok(())
    }
    
    /// Update position in portfolio state
    async fn update_position(&self, update: &PositionUpdate) -> Result<()> {
        let mut state = self.portfolio_state.write().await;
        
        match update.action {
            PositionAction::Opened | PositionAction::Increased | PositionAction::Updated => {
                let position = Position {
                    symbol: update.symbol.clone(),
                    quantity: update.quantity,
                    entry_price: update.entry_price,
                    current_price: update.current_price,
                    market_value: update.quantity * update.current_price,
                    unrealized_pnl: update.unrealized_pnl,
                    unrealized_pnl_percentage: (update.unrealized_pnl / (update.quantity * update.entry_price) * Decimal::from(100))
                        .to_f64().unwrap_or(0.0),
                    realized_pnl: Decimal::ZERO, // Would be updated separately
                    cost_basis: update.quantity * update.entry_price,
                };
                
                state.positions.insert(update.symbol.clone(), position);
            },
            PositionAction::Decreased => {
                if let Some(position) = state.positions.get_mut(&update.symbol) {
                    position.quantity = update.quantity;
                    position.market_value = update.quantity * update.current_price;
                    position.unrealized_pnl = update.unrealized_pnl;
                }
            },
            PositionAction::Closed => {
                state.positions.remove(&update.symbol);
            },
        }
        
        state.last_update = Utc::now();
        
        Ok(())
    }
    
    /// Update balance
    async fn update_balance(&self, update: &BalanceUpdate) -> Result<()> {
        let mut state = self.portfolio_state.write().await;
        
        state.available_balance = update.available_balance;
        state.total_value = update.total_equity;
        state.last_update = Utc::now();
        
        Ok(())
    }
    
    /// Update P&L
    async fn update_pnl(&self, update: &PnLUpdate) -> Result<()> {
        let mut state = self.portfolio_state.write().await;
        
        match update.timeframe {
            PnLTimeframe::Daily => {
                state.pnl.daily_pnl = update.pnl_amount;
                state.pnl.daily_pnl_percentage = update.pnl_percentage;
            },
            PnLTimeframe::Weekly => {
                state.pnl.weekly_pnl = update.pnl_amount;
                state.pnl.weekly_pnl_percentage = update.pnl_percentage;
            },
            PnLTimeframe::Monthly => {
                state.pnl.monthly_pnl = update.pnl_amount;
                state.pnl.monthly_pnl_percentage = update.pnl_percentage;
            },
            PnLTimeframe::Yearly => {
                state.pnl.yearly_pnl = update.pnl_amount;
                state.pnl.yearly_pnl_percentage = update.pnl_percentage;
            },
            PnLTimeframe::AllTime => {
                state.pnl.all_time_pnl = update.pnl_amount;
                state.pnl.all_time_pnl_percentage = update.pnl_percentage;
            },
        }
        
        state.last_update = Utc::now();
        
        Ok(())
    }
    
    /// Update order
    async fn update_order(&self, _update: &OrderUpdate) -> Result<()> {
        // Implementation would update open orders
        Ok(())
    }
    
    /// Process trade execution
    async fn process_trade_execution(&self, _trade: &TradeExecution) -> Result<()> {
        // Implementation would process trade and update positions
        Ok(())
    }
    
    /// Update risk metrics
    async fn update_risk_metrics(&self, update: &RiskMetricsUpdate) -> Result<()> {
        let mut state = self.portfolio_state.write().await;
        
        match update.metric_type {
            RiskMetricType::Volatility => {
                state.risk_metrics.portfolio_volatility = update.value;
            },
            RiskMetricType::Drawdown => {
                state.risk_metrics.current_drawdown = update.value;
            },
            RiskMetricType::VaR => {
                state.risk_metrics.value_at_risk = update.value;
            },
            _ => {}
        }
        
        state.last_update = Utc::now();
        
        Ok(())
    }
    
    /// Process alert
    async fn process_alert(&self, alert: &AlertTrigger) -> Result<()> {
        warn!("💼 Alert triggered: {} - {}", alert.title, alert.message);
        
        // Broadcast to alert subscribers
        let subscribers = self.alert_subscribers.read().await;
        for tx in subscribers.iter() {
            let _ = tx.send(alert.clone());
        }
        
        Ok(())
    }
    
    /// Broadcast update to subscribers
    async fn broadcast_update(&self, update: PortfolioUpdate) -> Result<()> {
        let state = self.portfolio_state.read().await;
        let subscribers = self.update_subscribers.read().await;
        
        if let Some(tx) = subscribers.get(&state.account) {
            let _ = tx.send(update);
        }
        
        Ok(())
    }
    
    /// Get current portfolio state
    pub async fn get_portfolio_state(&self) -> PortfolioState {
        let state = self.portfolio_state.read().await;
        state.clone()
    }
    
    /// Get specific position
    pub async fn get_position(&self, symbol: &str) -> Option<Position> {
        let state = self.portfolio_state.read().await;
        state.positions.get(symbol).cloned()
    }
    
    /// Calculate portfolio metrics
    pub async fn calculate_metrics(&self) -> PortfolioMetrics {
        let state = self.portfolio_state.read().await;
        
        let total_positions = state.positions.len();
        let total_unrealized_pnl: Decimal = state.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();
        
        let total_market_value: Decimal = state.positions.values()
            .map(|p| p.market_value)
            .sum();
        
        let largest_position = state.positions.values()
            .max_by_key(|p| p.market_value)
            .cloned();
        
        let best_performer = state.positions.values()
            .max_by(|a, b| a.unrealized_pnl_percentage.partial_cmp(&b.unrealized_pnl_percentage).unwrap())
            .cloned();
        
        let worst_performer = state.positions.values()
            .min_by(|a, b| a.unrealized_pnl_percentage.partial_cmp(&b.unrealized_pnl_percentage).unwrap())
            .cloned();
        
        PortfolioMetrics {
            total_value: state.total_value,
            available_balance: state.available_balance,
            total_positions,
            total_market_value,
            total_unrealized_pnl,
            largest_position,
            best_performer,
            worst_performer,
            risk_metrics: state.risk_metrics.clone(),
        }
    }
}

/// Portfolio metrics summary
#[derive(Debug, Clone)]
pub struct PortfolioMetrics {
    pub total_value: Decimal,
    pub available_balance: Decimal,
    pub total_positions: usize,
    pub total_market_value: Decimal,
    pub total_unrealized_pnl: Decimal,
    pub largest_position: Option<Position>,
    pub best_performer: Option<Position>,
    pub worst_performer: Option<Position>,
    pub risk_metrics: RiskMetrics,
}

impl Default for PnLSummary {
    fn default() -> Self {
        Self {
            daily_pnl: Decimal::ZERO,
            daily_pnl_percentage: 0.0,
            weekly_pnl: Decimal::ZERO,
            weekly_pnl_percentage: 0.0,
            monthly_pnl: Decimal::ZERO,
            monthly_pnl_percentage: 0.0,
            yearly_pnl: Decimal::ZERO,
            yearly_pnl_percentage: 0.0,
            all_time_pnl: Decimal::ZERO,
            all_time_pnl_percentage: 0.0,
        }
    }
}

impl Default for RiskMetrics {
    fn default() -> Self {
        Self {
            portfolio_beta: 1.0,
            portfolio_volatility: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            max_drawdown: 0.0,
            current_drawdown: 0.0,
            value_at_risk: 0.0,
            position_concentration: HashMap::new(),
            correlation_risk: 0.0,
        }
    }
}