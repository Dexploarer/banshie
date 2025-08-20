use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{info, debug, warn};

use crate::errors::Result;
use crate::websocket::realtime_client::{
    WebSocketClient, StreamData, MessageHandler, SubscriptionRequest, SubscriptionType,
};

/// Real-time price stream manager
#[derive(Clone)]
pub struct PriceStreamManager {
    ws_client: Arc<WebSocketClient>,
    price_cache: Arc<RwLock<HashMap<String, PriceData>>>,
    price_history: Arc<RwLock<HashMap<String, VecDeque<PriceUpdate>>>>,
    orderbook_cache: Arc<RwLock<HashMap<String, OrderBook>>>,
    subscribers: Arc<RwLock<HashMap<String, broadcast::Sender<PriceUpdate>>>>,
    aggregators: Arc<RwLock<Vec<Arc<dyn PriceAggregator>>>>,
}

/// Price data cache
#[derive(Debug, Clone)]
pub struct PriceData {
    pub symbol: String,
    pub current_price: Decimal,
    pub last_update: DateTime<Utc>,
    pub daily_high: Decimal,
    pub daily_low: Decimal,
    pub daily_volume: Decimal,
    pub price_change_24h: Decimal,
    pub price_change_percentage_24h: f64,
    pub market_cap: Option<Decimal>,
    pub sources: Vec<PriceSource>,
}

/// Real-time price update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub symbol: String,
    pub price: Decimal,
    pub timestamp: DateTime<Utc>,
    pub volume: Option<Decimal>,
    pub source: PriceSource,
    pub update_type: UpdateType,
    pub metadata: Option<PriceMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    Trade,
    Quote,
    Aggregate,
    Index,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceMetadata {
    pub trade_id: Option<String>,
    pub exchange: Option<String>,
    pub conditions: Option<Vec<String>>,
    pub is_dark_pool: bool,
}

/// Price source enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PriceSource {
    Jupiter,
    Pyth,
    Chainlink,
    Birdeye,
    CoinGecko,
    Binance,
    Coinbase,
    Aggregate,
}

/// Price subscription configuration
#[derive(Debug, Clone)]
pub struct PriceSubscription {
    pub symbols: Vec<String>,
    pub sources: Vec<PriceSource>,
    pub include_orderbook: bool,
    pub orderbook_depth: u32,
    pub include_trades: bool,
    pub aggregation_interval: Option<std::time::Duration>,
}

/// OHLCV data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCV {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trades: u32,
}

/// Tick data for high-frequency updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bid: Decimal,
    pub ask: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
    pub last_price: Decimal,
    pub last_size: Decimal,
}

/// Order book representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub size: Decimal,
    pub orders: u32,
}

/// Market depth analysis
#[derive(Debug, Clone)]
pub struct MarketDepth {
    pub symbol: String,
    pub bid_depth: Decimal,
    pub ask_depth: Decimal,
    pub bid_ask_spread: Decimal,
    pub spread_percentage: f64,
    pub imbalance_ratio: f64,
    pub liquidity_score: f64,
}

/// Aggregated price from multiple sources
#[derive(Debug, Clone)]
pub struct AggregatedPrice {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub mean_price: Decimal,
    pub median_price: Decimal,
    pub weighted_price: Decimal,
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub std_deviation: f64,
    pub confidence: f64,
    pub sources_count: usize,
    pub source_prices: HashMap<PriceSource, Decimal>,
}

/// Price aggregator trait
#[async_trait]
pub trait PriceAggregator: Send + Sync {
    async fn aggregate(&self, prices: &[PriceUpdate]) -> AggregatedPrice;
    fn name(&self) -> String;
}

/// WebSocket message handler for price updates
pub struct PriceMessageHandler {
    manager: Arc<PriceStreamManager>,
    subscription_id: String,
}

#[async_trait]
impl MessageHandler for PriceMessageHandler {
    async fn handle_message(&self, message: StreamData) -> Result<()> {
        // Parse price update from message
        let update: PriceUpdate = serde_json::from_value(message.data)?;
        
        // Process the update
        self.manager.process_price_update(update).await?;
        
        Ok(())
    }
    
    fn subscription_id(&self) -> String {
        self.subscription_id.clone()
    }
}

impl PriceStreamManager {
    /// Create new price stream manager
    pub fn new(ws_client: Arc<WebSocketClient>) -> Self {
        info!("📈 Initializing price stream manager");
        
        Self {
            ws_client,
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            price_history: Arc::new(RwLock::new(HashMap::new())),
            orderbook_cache: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            aggregators: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Subscribe to price updates for symbols
    pub async fn subscribe_prices(&self, subscription: PriceSubscription) -> Result<broadcast::Receiver<PriceUpdate>> {
        info!("📈 Subscribing to prices for {} symbols", subscription.symbols.len());
        
        // Create broadcast channel for this subscription
        let (tx, rx) = broadcast::channel(1000);
        
        // Store subscriber
        for symbol in &subscription.symbols {
            let mut subscribers = self.subscribers.write().await;
            subscribers.insert(symbol.clone(), tx.clone());
        }
        
        // Create WebSocket subscription
        let ws_subscription = SubscriptionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::Price {
                symbols: subscription.symbols.clone(),
            },
            params: HashMap::new(),
            filters: None,
        };
        
        // Register message handler
        let handler = Arc::new(PriceMessageHandler {
            manager: Arc::new(self.clone()),
            subscription_id: ws_subscription.id.clone(),
        });
        
        self.ws_client.register_handler(handler).await;
        
        // Subscribe via WebSocket
        self.ws_client.subscribe("price_stream", ws_subscription).await?;
        
        // If orderbook is requested, subscribe separately
        if subscription.include_orderbook {
            self.subscribe_orderbook(&subscription.symbols, subscription.orderbook_depth).await?;
        }
        
        Ok(rx)
    }
    
    /// Subscribe to order book updates
    async fn subscribe_orderbook(&self, symbols: &[String], depth: u32) -> Result<()> {
        let ws_subscription = SubscriptionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::OrderBook {
                symbols: symbols.to_vec(),
                depth,
            },
            params: HashMap::new(),
            filters: None,
        };
        
        self.ws_client.subscribe("orderbook_stream", ws_subscription).await
    }
    
    /// Process incoming price update
    async fn process_price_update(&self, update: PriceUpdate) -> Result<()> {
        debug!("📈 Processing price update for {}: {}", update.symbol, update.price);
        
        // Update price cache
        {
            let mut cache = self.price_cache.write().await;
            let price_data = cache.entry(update.symbol.clone()).or_insert_with(|| PriceData {
                symbol: update.symbol.clone(),
                current_price: update.price,
                last_update: update.timestamp,
                daily_high: update.price,
                daily_low: update.price,
                daily_volume: Decimal::ZERO,
                price_change_24h: Decimal::ZERO,
                price_change_percentage_24h: 0.0,
                market_cap: None,
                sources: vec![update.source.clone()],
            });
            
            // Update price data
            price_data.current_price = update.price;
            price_data.last_update = update.timestamp;
            
            if update.price > price_data.daily_high {
                price_data.daily_high = update.price;
            }
            if update.price < price_data.daily_low {
                price_data.daily_low = update.price;
            }
            
            if let Some(volume) = update.volume {
                price_data.daily_volume += volume;
            }
            
            if !price_data.sources.contains(&update.source) {
                price_data.sources.push(update.source.clone());
            }
        }
        
        // Add to price history
        {
            let mut history = self.price_history.write().await;
            let symbol_history = history.entry(update.symbol.clone())
                .or_insert_with(|| VecDeque::with_capacity(1000));
            
            symbol_history.push_back(update.clone());
            
            // Keep only last 1000 updates
            if symbol_history.len() > 1000 {
                symbol_history.pop_front();
            }
        }
        
        // Broadcast to subscribers
        let subscribers = self.subscribers.read().await;
        if let Some(tx) = subscribers.get(&update.symbol) {
            let _ = tx.send(update.clone()); // Ignore errors if no receivers
        }
        
        // Run aggregators if multiple sources
        self.run_aggregators(&update.symbol).await?;
        
        Ok(())
    }
    
    /// Run price aggregators
    async fn run_aggregators(&self, symbol: &str) -> Result<()> {
        let history = self.price_history.read().await;
        
        if let Some(symbol_history) = history.get(symbol) {
            // Get recent prices from different sources
            let recent_prices: Vec<PriceUpdate> = symbol_history.iter()
                .rev()
                .take(10)
                .cloned()
                .collect();
            
            if recent_prices.len() >= 2 {
                let aggregators = self.aggregators.read().await;
                
                for aggregator in aggregators.iter() {
                    let aggregated = aggregator.aggregate(&recent_prices).await;
                    debug!("📈 Aggregated price for {}: {} (confidence: {:.2}%)", 
                        symbol, aggregated.weighted_price, aggregated.confidence * 100.0);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get current price for symbol
    pub async fn get_price(&self, symbol: &str) -> Option<PriceData> {
        let cache = self.price_cache.read().await;
        cache.get(symbol).cloned()
    }
    
    /// Get price history for symbol
    pub async fn get_price_history(&self, symbol: &str) -> Vec<PriceUpdate> {
        let history = self.price_history.read().await;
        history.get(symbol)
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get order book for symbol
    pub async fn get_orderbook(&self, symbol: &str) -> Option<OrderBook> {
        let orderbooks = self.orderbook_cache.read().await;
        orderbooks.get(symbol).cloned()
    }
    
    /// Calculate market depth
    pub async fn calculate_market_depth(&self, symbol: &str) -> Option<MarketDepth> {
        let orderbook = self.get_orderbook(symbol).await?;
        
        let bid_depth: Decimal = orderbook.bids.iter()
            .map(|level| level.price * level.size)
            .sum();
        
        let ask_depth: Decimal = orderbook.asks.iter()
            .map(|level| level.price * level.size)
            .sum();
        
        let best_bid = orderbook.bids.first()?.price;
        let best_ask = orderbook.asks.first()?.price;
        let spread = best_ask - best_bid;
        let mid_price = (best_bid + best_ask) / Decimal::from(2);
        let spread_percentage = (spread / mid_price * Decimal::from(100)).to_f64().unwrap_or(0.0);
        
        let imbalance_ratio = if ask_depth > Decimal::ZERO {
            (bid_depth / ask_depth).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };
        
        let liquidity_score = ((bid_depth + ask_depth) / Decimal::from(2))
            .to_f64()
            .unwrap_or(0.0)
            .log10()
            .max(0.0)
            .min(10.0) / 10.0;
        
        Some(MarketDepth {
            symbol: symbol.to_string(),
            bid_depth,
            ask_depth,
            bid_ask_spread: spread,
            spread_percentage,
            imbalance_ratio,
            liquidity_score,
        })
    }
    
    /// Calculate VWAP (Volume Weighted Average Price)
    pub async fn calculate_vwap(&self, symbol: &str, periods: usize) -> Option<Decimal> {
        let history = self.price_history.read().await;
        let symbol_history = history.get(symbol)?;
        
        let recent_updates: Vec<PriceUpdate> = symbol_history.iter()
            .rev()
            .take(periods)
            .filter(|u| u.volume.is_some())
            .cloned()
            .collect();
        
        if recent_updates.is_empty() {
            return None;
        }
        
        let total_volume: Decimal = recent_updates.iter()
            .filter_map(|u| u.volume)
            .sum();
        
        if total_volume == Decimal::ZERO {
            return None;
        }
        
        let weighted_sum: Decimal = recent_updates.iter()
            .filter_map(|u| u.volume.map(|v| u.price * v))
            .sum();
        
        Some(weighted_sum / total_volume)
    }
    
    /// Register custom price aggregator
    pub async fn register_aggregator(&self, aggregator: Arc<dyn PriceAggregator>) {
        let mut aggregators = self.aggregators.write().await;
        aggregators.push(aggregator);
    }
    
    /// Unsubscribe from symbol updates
    pub async fn unsubscribe(&self, symbol: &str) -> Result<()> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(symbol);
        
        // Would also unsubscribe from WebSocket if no more subscribers
        
        Ok(())
    }
}

/// Default price aggregator implementation
pub struct DefaultPriceAggregator;

#[async_trait]
impl PriceAggregator for DefaultPriceAggregator {
    async fn aggregate(&self, prices: &[PriceUpdate]) -> AggregatedPrice {
        let symbol = prices.first().map(|p| p.symbol.clone()).unwrap_or_default();
        let timestamp = Utc::now();
        
        let mut price_values: Vec<Decimal> = prices.iter().map(|p| p.price).collect();
        price_values.sort();
        
        let mean_price = if !price_values.is_empty() {
            price_values.iter().sum::<Decimal>() / Decimal::from(price_values.len())
        } else {
            Decimal::ZERO
        };
        
        let median_price = if !price_values.is_empty() {
            price_values[price_values.len() / 2]
        } else {
            Decimal::ZERO
        };
        
        // Volume-weighted price
        let total_volume: Decimal = prices.iter()
            .filter_map(|p| p.volume)
            .sum();
        
        let weighted_price = if total_volume > Decimal::ZERO {
            prices.iter()
                .filter_map(|p| p.volume.map(|v| p.price * v))
                .sum::<Decimal>() / total_volume
        } else {
            mean_price
        };
        
        let min_price = price_values.first().cloned().unwrap_or(Decimal::ZERO);
        let max_price = price_values.last().cloned().unwrap_or(Decimal::ZERO);
        
        // Calculate standard deviation
        let variance = if !price_values.is_empty() {
            price_values.iter()
                .map(|p| {
                    let diff = (*p - mean_price).to_f64().unwrap_or(0.0);
                    diff * diff
                })
                .sum::<f64>() / price_values.len() as f64
        } else {
            0.0
        };
        
        let std_deviation = variance.sqrt();
        
        // Calculate confidence based on source diversity and consistency
        let sources_count = prices.iter()
            .map(|p| &p.source)
            .collect::<std::collections::HashSet<_>>()
            .len();
        
        let consistency_factor = if std_deviation > 0.0 {
            1.0 / (1.0 + std_deviation)
        } else {
            1.0
        };
        
        let diversity_factor = sources_count as f64 / 5.0; // Assume 5 sources is ideal
        let confidence = (consistency_factor * 0.6 + diversity_factor.min(1.0) * 0.4).min(1.0);
        
        // Collect source prices
        let mut source_prices = HashMap::new();
        for update in prices {
            source_prices.insert(update.source.clone(), update.price);
        }
        
        AggregatedPrice {
            symbol,
            timestamp,
            mean_price,
            median_price,
            weighted_price,
            min_price,
            max_price,
            std_deviation,
            confidence,
            sources_count,
            source_prices,
        }
    }
    
    fn name(&self) -> String {
        "DefaultPriceAggregator".to_string()
    }
}