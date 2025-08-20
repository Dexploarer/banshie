use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, Error as WsError},
    WebSocketStream,
    MaybeTlsStream,
};
use tracing::{info, debug, warn, error};
use url::Url;

use crate::errors::{BotError, Result};
use crate::telemetry::TelemetryService;

/// WebSocket client for real-time data streaming
#[derive(Clone)]
pub struct WebSocketClient {
    config: Arc<WebSocketConfig>,
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    message_handlers: Arc<RwLock<HashMap<String, Arc<dyn MessageHandler>>>>,
    error_handlers: Arc<RwLock<Vec<Arc<dyn ErrorHandler>>>>,
    telemetry: Option<Arc<TelemetryService>>,
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub endpoints: HashMap<String, String>,
    pub reconnect_strategy: ReconnectStrategy,
    pub heartbeat_interval: Duration,
    pub message_timeout: Duration,
    pub max_message_size: usize,
    pub compression: bool,
    pub tls_config: Option<TlsConfig>,
}

/// TLS configuration for secure connections
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub verify_cert: bool,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub ca_cert: Option<String>,
}

/// Reconnection strategy
#[derive(Debug, Clone)]
pub struct ReconnectStrategy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_backoff: bool,
    pub jitter: bool,
}

/// Connection state for each WebSocket
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub endpoint: String,
    pub status: ConnectionStatus,
    pub subscriptions: Vec<SubscriptionRequest>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub reconnect_attempts: u32,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Connection status
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Reconnecting,
    Failed(String),
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    Subscribe(SubscriptionRequest),
    Unsubscribe { id: String },
    Ping { timestamp: i64 },
    Pong { timestamp: i64 },
    Data(StreamData),
    Error { code: u16, message: String },
    Auth { token: String },
    Custom(serde_json::Value),
}

/// Subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub id: String,
    pub subscription_type: SubscriptionType,
    pub params: HashMap<String, serde_json::Value>,
    pub filters: Option<Vec<Filter>>,
}

/// Subscription types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionType {
    Price { symbols: Vec<String> },
    OrderBook { symbols: Vec<String>, depth: u32 },
    Trades { symbols: Vec<String> },
    Portfolio { account: String },
    Orders { account: String },
    Positions { account: String },
    MarketData { market: String },
    News { sources: Vec<String> },
    Alerts { types: Vec<String> },
    Custom(String),
}

/// Data filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    In,
}

/// Stream data wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    pub subscription_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sequence: u64,
    pub data: serde_json::Value,
}

/// Message handler trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle_message(&self, message: StreamData) -> Result<()>;
    fn subscription_id(&self) -> String;
}

/// Error handler trait
#[async_trait]
pub trait ErrorHandler: Send + Sync {
    async fn handle_error(&self, error: WebSocketError) -> Result<()>;
    async fn handle_disconnection(&self, endpoint: String) -> Result<()>;
}

/// WebSocket error types
#[derive(Debug, Clone)]
pub enum WebSocketError {
    ConnectionFailed(String),
    MessageParseError(String),
    SubscriptionError(String),
    HeartbeatTimeout,
    RateLimitExceeded,
    AuthenticationFailed,
    UnexpectedClose(u16, String),
}

impl WebSocketClient {
    /// Create new WebSocket client
    pub fn new(
        config: WebSocketConfig,
        telemetry: Option<Arc<TelemetryService>>,
    ) -> Self {
        info!("🔌 Initializing WebSocket client");
        
        Self {
            config: Arc::new(config),
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            error_handlers: Arc::new(RwLock::new(Vec::new())),
            telemetry,
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Connect to WebSocket endpoint
    pub async fn connect(&self, name: &str, endpoint: &str) -> Result<()> {
        let _span = self.telemetry.as_ref().map(|t| 
            t.create_span("websocket_connect")
        );
        
        info!("🔌 Connecting to WebSocket: {} at {}", name, endpoint);
        
        // Parse URL
        let url = Url::parse(endpoint)
            .map_err(|e| BotError::config(format!("Invalid WebSocket URL: {}", e)))?;
        
        // Create connection state
        let state = ConnectionState {
            endpoint: endpoint.to_string(),
            status: ConnectionStatus::Connecting,
            subscriptions: Vec::new(),
            last_heartbeat: chrono::Utc::now(),
            reconnect_attempts: 0,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
        };
        
        // Store connection state
        {
            let mut connections = self.connections.write().await;
            connections.insert(name.to_string(), state);
        }
        
        // Spawn connection handler
        let client = self.clone();
        let name = name.to_string();
        let endpoint = endpoint.to_string();
        
        tokio::spawn(async move {
            if let Err(e) = client.handle_connection(&name, &endpoint).await {
                error!("🔌 WebSocket connection error for {}: {}", name, e);
                client.handle_connection_error(&name, e).await;
            }
        });
        
        Ok(())
    }
    
    /// Handle WebSocket connection
    async fn handle_connection(&self, name: &str, endpoint: &str) -> Result<()> {
        let url = Url::parse(endpoint)?;
        
        // Connect with retry logic
        let ws_stream = self.connect_with_retry(&url).await?;
        
        // Update connection status
        {
            let mut connections = self.connections.write().await;
            if let Some(state) = connections.get_mut(name) {
                state.status = ConnectionStatus::Connected;
                state.reconnect_attempts = 0;
            }
        }
        
        info!("🔌 WebSocket connected: {}", name);
        
        // Split stream
        let (mut write, mut read) = ws_stream.split();
        
        // Create channels for communication
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        
        // Store shutdown sender
        {
            let mut shutdown = self.shutdown_tx.write().await;
            *shutdown = Some(shutdown_tx);
        }
        
        // Spawn heartbeat task
        let client = self.clone();
        let name_clone = name.to_string();
        let tx_clone = tx.clone();
        
        tokio::spawn(async move {
            let mut heartbeat = interval(client.config.heartbeat_interval);
            
            loop {
                tokio::select! {
                    _ = heartbeat.tick() => {
                        let ping = Message::Ping(vec![]);
                        if tx_clone.send(ping).await.is_err() {
                            break;
                        }
                        
                        // Update heartbeat timestamp
                        let mut connections = client.connections.write().await;
                        if let Some(state) = connections.get_mut(&name_clone) {
                            state.last_heartbeat = chrono::Utc::now();
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });
        
        // Spawn write task
        let client = self.clone();
        let name_clone = name.to_string();
        
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg.clone()).await {
                    error!("🔌 Failed to send WebSocket message: {}", e);
                    break;
                }
                
                // Update stats
                let mut connections = client.connections.write().await;
                if let Some(state) = connections.get_mut(&name_clone) {
                    state.messages_sent += 1;
                    state.bytes_sent += msg.len() as u64;
                }
            }
        });
        
        // Handle incoming messages
        while let Some(result) = read.next().await {
            match result {
                Ok(msg) => {
                    // Update stats
                    {
                        let mut connections = self.connections.write().await;
                        if let Some(state) = connections.get_mut(name) {
                            state.messages_received += 1;
                            state.bytes_received += msg.len() as u64;
                        }
                    }
                    
                    // Process message
                    if let Err(e) = self.process_message(name, msg).await {
                        warn!("🔌 Failed to process WebSocket message: {}", e);
                    }
                },
                Err(e) => {
                    error!("🔌 WebSocket error: {}", e);
                    break;
                }
            }
        }
        
        // Handle disconnection
        self.handle_disconnection(name).await?;
        
        Ok(())
    }
    
    /// Connect with retry logic
    async fn connect_with_retry(&self, url: &Url) -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>> {
        let mut attempts = 0;
        let mut delay = self.config.reconnect_strategy.initial_delay;
        
        loop {
            match connect_async(url.clone()).await {
                Ok((ws_stream, _)) => return Ok(ws_stream),
                Err(e) => {
                    attempts += 1;
                    
                    if attempts >= self.config.reconnect_strategy.max_attempts {
                        return Err(BotError::external_api(
                            format!("Failed to connect after {} attempts: {}", attempts, e)
                        ).into());
                    }
                    
                    warn!("🔌 WebSocket connection attempt {} failed: {}", attempts, e);
                    
                    // Apply jitter if configured
                    let mut actual_delay = delay;
                    if self.config.reconnect_strategy.jitter {
                        let jitter = rand::random::<f64>() * 0.3; // 0-30% jitter
                        actual_delay = delay.mul_f64(1.0 + jitter);
                    }
                    
                    sleep(actual_delay).await;
                    
                    // Update delay with backoff
                    if self.config.reconnect_strategy.exponential_backoff {
                        delay = (delay * 2).min(self.config.reconnect_strategy.max_delay);
                    }
                }
            }
        }
    }
    
    /// Process incoming WebSocket message
    async fn process_message(&self, name: &str, msg: Message) -> Result<()> {
        match msg {
            Message::Text(text) => {
                debug!("🔌 Received text message from {}: {}", name, text);
                
                // Parse message
                let ws_msg: WebSocketMessage = serde_json::from_str(&text)
                    .map_err(|e| BotError::parsing(format!("Invalid WebSocket message: {}", e)))?;
                
                match ws_msg {
                    WebSocketMessage::Data(data) => {
                        self.handle_data_message(data).await?;
                    },
                    WebSocketMessage::Pong { timestamp } => {
                        debug!("🔌 Received pong: {}", timestamp);
                    },
                    WebSocketMessage::Error { code, message } => {
                        warn!("🔌 WebSocket error {}: {}", code, message);
                        self.handle_error_message(code, message).await?;
                    },
                    _ => {
                        debug!("🔌 Unhandled message type");
                    }
                }
            },
            Message::Binary(data) => {
                debug!("🔌 Received binary message from {}: {} bytes", name, data.len());
                // Handle binary data if needed
            },
            Message::Ping(data) => {
                debug!("🔌 Received ping from {}", name);
                // Pong is usually sent automatically by the library
            },
            Message::Pong(_) => {
                debug!("🔌 Received pong from {}", name);
            },
            Message::Close(frame) => {
                if let Some(frame) = frame {
                    warn!("🔌 WebSocket closing: {} - {}", frame.code, frame.reason);
                } else {
                    warn!("🔌 WebSocket closing without frame");
                }
            },
            Message::Frame(_) => {
                // Raw frame, usually not handled directly
            }
        }
        
        Ok(())
    }
    
    /// Handle data message
    async fn handle_data_message(&self, data: StreamData) -> Result<()> {
        let handlers = self.message_handlers.read().await;
        
        if let Some(handler) = handlers.get(&data.subscription_id) {
            handler.handle_message(data).await?;
        } else {
            debug!("🔌 No handler for subscription: {}", data.subscription_id);
        }
        
        Ok(())
    }
    
    /// Handle error message
    async fn handle_error_message(&self, code: u16, message: String) -> Result<()> {
        let error = match code {
            429 => WebSocketError::RateLimitExceeded,
            401 | 403 => WebSocketError::AuthenticationFailed,
            _ => WebSocketError::SubscriptionError(message),
        };
        
        let handlers = self.error_handlers.read().await;
        for handler in handlers.iter() {
            handler.handle_error(error.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Handle disconnection
    async fn handle_disconnection(&self, name: &str) -> Result<()> {
        warn!("🔌 WebSocket disconnected: {}", name);
        
        // Update connection status
        {
            let mut connections = self.connections.write().await;
            if let Some(state) = connections.get_mut(name) {
                state.status = ConnectionStatus::Disconnected;
            }
        }
        
        // Notify error handlers
        let handlers = self.error_handlers.read().await;
        for handler in handlers.iter() {
            handler.handle_disconnection(name.to_string()).await?;
        }
        
        // Attempt reconnection if configured
        if self.should_reconnect(name).await {
            self.reconnect(name).await?;
        }
        
        Ok(())
    }
    
    /// Handle connection error
    async fn handle_connection_error(&self, name: &str, error: BotError) {
        error!("🔌 Connection error for {}: {}", name, error);
        
        // Update connection status
        let mut connections = self.connections.write().await;
        if let Some(state) = connections.get_mut(name) {
            state.status = ConnectionStatus::Failed(error.to_string());
        }
    }
    
    /// Check if should reconnect
    async fn should_reconnect(&self, name: &str) -> bool {
        let connections = self.connections.read().await;
        
        if let Some(state) = connections.get(name) {
            state.reconnect_attempts < self.config.reconnect_strategy.max_attempts
        } else {
            false
        }
    }
    
    /// Reconnect to WebSocket
    async fn reconnect(&self, name: &str) -> Result<()> {
        info!("🔌 Attempting to reconnect WebSocket: {}", name);
        
        // Get endpoint
        let endpoint = {
            let connections = self.connections.read().await;
            connections.get(name)
                .map(|s| s.endpoint.clone())
                .ok_or_else(|| BotError::not_found(format!("Connection {} not found", name)))?
        };
        
        // Update reconnect attempts
        {
            let mut connections = self.connections.write().await;
            if let Some(state) = connections.get_mut(name) {
                state.status = ConnectionStatus::Reconnecting;
                state.reconnect_attempts += 1;
            }
        }
        
        // Reconnect
        self.connect(name, &endpoint).await
    }
    
    /// Subscribe to data stream
    pub async fn subscribe(&self, connection: &str, request: SubscriptionRequest) -> Result<()> {
        info!("🔌 Subscribing to: {:?}", request.subscription_type);
        
        // Store subscription
        {
            let mut connections = self.connections.write().await;
            if let Some(state) = connections.get_mut(connection) {
                state.subscriptions.push(request.clone());
            }
        }
        
        // Send subscription message
        let msg = WebSocketMessage::Subscribe(request);
        self.send_message(connection, msg).await
    }
    
    /// Unsubscribe from data stream
    pub async fn unsubscribe(&self, connection: &str, subscription_id: &str) -> Result<()> {
        info!("🔌 Unsubscribing from: {}", subscription_id);
        
        // Remove subscription
        {
            let mut connections = self.connections.write().await;
            if let Some(state) = connections.get_mut(connection) {
                state.subscriptions.retain(|s| s.id != subscription_id);
            }
        }
        
        // Send unsubscribe message
        let msg = WebSocketMessage::Unsubscribe {
            id: subscription_id.to_string(),
        };
        self.send_message(connection, msg).await
    }
    
    /// Send message to WebSocket
    async fn send_message(&self, _connection: &str, msg: WebSocketMessage) -> Result<()> {
        let json = serde_json::to_string(&msg)
            .map_err(|e| BotError::serialization(e))?;
        
        // In production, would send through the actual WebSocket connection
        debug!("🔌 Sending message: {}", json);
        
        Ok(())
    }
    
    /// Register message handler
    pub async fn register_handler(&self, handler: Arc<dyn MessageHandler>) {
        let subscription_id = handler.subscription_id();
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(subscription_id, handler);
    }
    
    /// Register error handler
    pub async fn register_error_handler(&self, handler: Arc<dyn ErrorHandler>) {
        let mut handlers = self.error_handlers.write().await;
        handlers.push(handler);
    }
    
    /// Get connection status
    pub async fn get_status(&self, name: &str) -> Option<ConnectionStatus> {
        let connections = self.connections.read().await;
        connections.get(name).map(|s| s.status.clone())
    }
    
    /// Get all connection stats
    pub async fn get_stats(&self) -> HashMap<String, ConnectionState> {
        let connections = self.connections.read().await;
        connections.clone()
    }
    
    /// Shutdown all connections
    pub async fn shutdown(&self) -> Result<()> {
        info!("🔌 Shutting down WebSocket client");
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(()).await;
        }
        
        // Update all connections to disconnected
        let mut connections = self.connections.write().await;
        for (_, state) in connections.iter_mut() {
            state.status = ConnectionStatus::Disconnected;
        }
        
        Ok(())
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            endpoints: HashMap::new(),
            reconnect_strategy: ReconnectStrategy::default(),
            heartbeat_interval: Duration::from_secs(30),
            message_timeout: Duration::from_secs(60),
            max_message_size: 10 * 1024 * 1024, // 10MB
            compression: true,
            tls_config: None,
        }
    }
}

impl Default for ReconnectStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            exponential_backoff: true,
            jitter: true,
        }
    }
}