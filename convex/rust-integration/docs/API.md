# Rust Integration API Documentation

## Overview

The Rust Integration Library provides a type-safe interface for interacting with the Convex backend from Rust applications. This library is specifically designed for the Solana Trading Bot project and provides seamless integration between Rust services and the Convex backend.

## Architecture

The library consists of four main modules:

- **ConvexClient**: HTTP client for direct Convex API communication
- **TelegramConvexBridge**: Integration layer for Telegram bot functionality
- **TradingService**: High-level trading operations wrapper
- **WebhookServer**: HTTP server for receiving Convex webhooks

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
convex-rust-integration = { path = "path/to/convex/rust-integration" }
tokio = { version = "1.0", features = ["full"] }
```

## Configuration

### Environment Variables

```bash
CONVEX_URL=https://your-convex-app.convex.site
CONVEX_SITE_URL=https://your-convex-app.convex.cloud
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
WEBHOOK_PORT=8080
WEBHOOK_PATH=/webhook
```

### ConvexConfig

```rust
use convex_integration::ConvexConfig;

let config = ConvexConfig {
    convex_url: "https://your-convex-app.convex.site".to_string(),
    convex_site_url: "https://your-convex-app.convex.cloud".to_string(),
    telegram_bot_token: "your_bot_token".to_string(),
    webhook_port: 8080,
    webhook_path: "/webhook".to_string(),
};
```

## ConvexClient API

### Core Methods

#### `ConvexClient::new() -> Result<ConvexClient>`

Creates a new Convex client with configuration from environment variables.

```rust
use convex_integration::ConvexClient;

let client = ConvexClient::new()?;
```

#### `query<T>(&self, function_name: &str, args: Value) -> Result<T>`

Execute a Convex query function.

```rust
use serde_json::json;

let user: Option<UserProfile> = client.query(
    "queries/users:getUserByTelegramId", 
    json!({"telegramId": 123456789})
).await?;
```

#### `mutation<T>(&self, function_name: &str, args: Value) -> Result<T>`

Execute a Convex mutation function.

```rust
let user_id: String = client.mutation(
    "mutations/users:createOrUpdateUser",
    json!({
        "telegramId": 123456789,
        "username": "trader_bot",
        "isPremium": false
    })
).await?;
```

#### `action<T>(&self, function_name: &str, args: Value) -> Result<T>`

Execute a Convex action function.

```rust
let prices: Value = client.action(
    "actions/prices:updateTokenPrices",
    json!({"tokens": ["So11111111111111111111111111111111111111112"]})
).await?;
```

### User Management

#### `get_user_by_telegram_id(&self, telegram_id: i64) -> Result<Option<UserProfile>>`

Retrieve user profile by Telegram ID.

```rust
let user = client.get_user_by_telegram_id(123456789).await?;
match user {
    Some(profile) => println!("User: {}", profile.username),
    None => println!("User not found"),
}
```

#### `create_or_update_user(&self, telegram_id: i64, username: &str) -> Result<String>`

Create or update a user profile.

```rust
let user_id = client.create_or_update_user(123456789, "new_trader").await?;
println!("User ID: {}", user_id);
```

### Portfolio Management

#### `get_portfolio(&self, user_id: &str) -> Result<PortfolioSummary>`

Get user portfolio summary.

```rust
let portfolio = client.get_portfolio("user_123456789").await?;
println!("Total Value: {}", portfolio.total_value);
println!("P&L: {}", portfolio.total_pnl);
```

#### `sync_wallet_balances(&self, user_id: &str, wallet_address: &str) -> Result<Value>`

Synchronize wallet balances with on-chain data.

```rust
let result = client.sync_wallet_balances(
    "user_123456789", 
    "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"
).await?;
```

### Trading Operations

#### `place_order(&self, order: OrderRequest) -> Result<String>`

Place a trading order.

```rust
use convex_integration::OrderRequest;

let order = OrderRequest {
    user_id: "user_123456789".to_string(),
    order_type: "market".to_string(),
    token_mint: "So11111111111111111111111111111111111111112".to_string(),
    side: "buy".to_string(),
    amount: "1.0".to_string(),
    price: None,
    slippage: Some(1.0),
};

let order_id = client.place_order(order).await?;
println!("Order placed: {}", order_id);
```

#### `get_order_status(&self, order_id: &str) -> Result<Value>`

Check order execution status.

```rust
let status = client.get_order_status("order_123").await?;
println!("Order status: {}", status);
```

### AI Trading Signals

#### `get_latest_signals(&self, limit: u32) -> Result<Vec<TradingSignal>>`

Retrieve latest AI trading signals.

```rust
let signals = client.get_latest_signals(10).await?;
for signal in signals {
    println!("Signal: {} - {} ({}%)", 
        signal.token_mint, 
        signal.signal_type, 
        signal.confidence * 100.0
    );
}
```

#### `generate_signal(&self, token_mint: &str) -> Result<TradingSignal>`

Generate a new trading signal for a token.

```rust
let signal = client.generate_signal("So11111111111111111111111111111111111111112").await?;
println!("New signal: {} with {}% confidence", signal.signal_type, signal.confidence * 100.0);
```

### Price Data

#### `get_token_price(&self, token_mint: &str) -> Result<Value>`

Get current token price data.

```rust
let price_data = client.get_token_price("So11111111111111111111111111111111111111112").await?;
println!("SOL Price: {}", price_data);
```

#### `update_prices(&self, tokens: Vec<&str>) -> Result<Value>`

Update price cache for multiple tokens.

```rust
let result = client.update_prices(vec![
    "So11111111111111111111111111111111111111112",
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
]).await?;
```

### DCA Strategies

#### `get_user_dca_strategies(&self, user_id: &str) -> Result<Vec<Value>>`

Get user's DCA strategies.

```rust
let strategies = client.get_user_dca_strategies("user_123456789").await?;
println!("Active strategies: {}", strategies.len());
```

#### `create_dca_strategy(&self, user_id: &str, token_mint: &str, amount: f64, frequency: &str) -> Result<String>`

Create a new DCA strategy.

```rust
let strategy_id = client.create_dca_strategy(
    "user_123456789",
    "So11111111111111111111111111111111111111112",
    10.0,
    "daily"
).await?;
println!("DCA strategy created: {}", strategy_id);
```

### Alerts

#### `create_price_alert(&self, user_id: &str, token_mint: &str, target_price: f64, condition: &str) -> Result<String>`

Create a price alert.

```rust
let alert_id = client.create_price_alert(
    "user_123456789",
    "So11111111111111111111111111111111111111112",
    100.0,
    "above"
).await?;
println!("Alert created: {}", alert_id);
```

#### `get_user_alerts(&self, user_id: &str) -> Result<Vec<Value>>`

Get user's active alerts.

```rust
let alerts = client.get_user_alerts("user_123456789").await?;
println!("Active alerts: {}", alerts.len());
```

### Analytics

#### `calculate_indicators(&self, token_mint: &str) -> Result<Value>`

Calculate technical indicators for a token.

```rust
let indicators = client.calculate_indicators("So11111111111111111111111111111111111111112").await?;
println!("Indicators: {}", indicators);
```

### Utility Methods

#### `health_check(&self) -> Result<bool>`

Check if Convex backend is healthy.

```rust
let is_healthy = client.health_check().await?;
if is_healthy {
    println!("✅ Convex backend is healthy");
} else {
    println!("❌ Convex backend is unhealthy");
}
```

#### `retry_with_backoff<F, T, E>(&self, f: F, max_retries: u32) -> Result<T>`

Execute a function with exponential backoff retry logic.

```rust
let result = client.retry_with_backoff(|| {
    // Your operation here
    client.get_token_price("So11111111111111111111111111111111111111112")
}, 3).await?;
```

### Convenience Methods

#### `get_or_create_user_portfolio(&self, telegram_id: i64, username: &str) -> Result<(String, PortfolioSummary)>`

Get user portfolio, creating user if needed.

```rust
let (user_id, portfolio) = client.get_or_create_user_portfolio(123456789, "new_trader").await?;
println!("User: {} | Portfolio Value: {}", user_id, portfolio.total_value);
```

#### `execute_trade_with_retry(&self, order: OrderRequest) -> Result<String>`

Execute a trade with automatic retry logic.

```rust
let order_id = client.execute_trade_with_retry(order_request).await?;
println!("Trade executed: {}", order_id);
```

#### `get_token_data(&self, token_mint: &str) -> Result<Value>`

Get comprehensive token data (price + indicators).

```rust
let data = client.get_token_data("So11111111111111111111111111111111111111112").await?;
println!("Token data: {}", data);
```

## ConvexIntegrationService

### Full Service Setup

```rust
use convex_integration::{ConvexIntegrationService, ConvexConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ConvexConfig {
        convex_url: "https://your-convex-app.convex.site".to_string(),
        convex_site_url: "https://your-convex-app.convex.cloud".to_string(),
        telegram_bot_token: "your_bot_token".to_string(),
        webhook_port: 8080,
        webhook_path: "/webhook".to_string(),
    };

    let service = ConvexIntegrationService::new(config).await?;
    
    // Health check before starting
    service.health_check().await?;
    
    // Start all services (Telegram bot, webhook server)
    service.start().await?;
    
    Ok(())
}
```

### Service Components

- **Webhook Server**: Receives real-time updates from Convex
- **Telegram Bot**: Handles user interactions and commands
- **Background Tasks**: Processes trading signals and alerts

## Data Types

### UserProfile

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub telegram_id: i64,
    pub username: String,
    pub is_premium: bool,
    pub settings: Value,
}
```

### PortfolioSummary

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub total_value: String,
    pub total_pnl: String,
    pub total_pnl_percentage: String,
    pub position_count: u32,
}
```

### TradingSignal

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct TradingSignal {
    pub token_mint: String,
    pub signal_type: String,    // "buy", "sell", "hold"
    pub strength: f64,          // 0.0 to 1.0
    pub confidence: f64,        // 0.0 to 1.0
    pub reasoning: String,
    pub timestamp: i64,
}
```

### OrderRequest

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub user_id: String,
    pub order_type: String,     // "market", "limit"
    pub token_mint: String,
    pub side: String,           // "buy", "sell"
    pub amount: String,
    pub price: Option<String>,
    pub slippage: Option<f64>,
}
```

## Error Handling

All methods return `Result<T, anyhow::Error>`. Common error scenarios:

- **Network errors**: Connection timeouts, HTTP errors
- **Convex errors**: Backend validation failures, rate limits
- **Serialization errors**: Invalid JSON responses
- **Authentication errors**: Invalid credentials or tokens

```rust
match client.get_portfolio("invalid_user").await {
    Ok(portfolio) => println!("Portfolio: {:?}", portfolio),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Rate Limits

The Convex backend implements rate limiting:

- **Queries**: 100 requests/minute per user
- **Mutations**: 60 requests/minute per user  
- **Actions**: 30 requests/minute per user

Use the `retry_with_backoff` method for automatic retry handling.

## Testing

Run tests with:

```bash
cargo test
```

Integration tests require a running Convex instance:

```bash
# Set environment variables
export CONVEX_URL="https://test-convex-app.convex.site"
export CONVEX_SITE_URL="https://test-convex-app.convex.cloud"

# Run integration tests
cargo test --features integration
```

## Examples

### Basic Trading Bot

```rust
use convex_integration::{ConvexClient, OrderRequest};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ConvexClient::new()?;
    
    loop {
        // Check for trading signals
        let signals = client.get_latest_signals(5).await?;
        
        for signal in signals {
            if signal.confidence > 0.8 {
                println!("High confidence signal: {} - {}", 
                    signal.token_mint, signal.signal_type);
                
                // Execute trade based on signal
                if signal.signal_type == "buy" {
                    let order = OrderRequest {
                        user_id: "user_123456789".to_string(),
                        order_type: "market".to_string(),
                        token_mint: signal.token_mint,
                        side: "buy".to_string(),
                        amount: "10.0".to_string(),
                        price: None,
                        slippage: Some(1.0),
                    };
                    
                    let order_id = client.execute_trade_with_retry(order).await?;
                    println!("Trade executed: {}", order_id);
                }
            }
        }
        
        sleep(Duration::from_secs(60)).await;
    }
}
```

### Portfolio Monitoring

```rust
use convex_integration::ConvexClient;

async fn monitor_portfolio(user_id: &str) -> anyhow::Result<()> {
    let client = ConvexClient::new()?;
    
    let portfolio = client.get_portfolio(user_id).await?;
    
    println!("📊 Portfolio Summary");
    println!("Total Value: ${}", portfolio.total_value);
    println!("P&L: ${} ({}%)", portfolio.total_pnl, portfolio.total_pnl_percentage);
    println!("Positions: {}", portfolio.position_count);
    
    // Check for alerts
    let alerts = client.get_user_alerts(user_id).await?;
    if !alerts.is_empty() {
        println!("🚨 Active Alerts: {}", alerts.len());
    }
    
    Ok(())
}
```

## Security Notes

- Never commit API keys or tokens to version control
- Use environment variables for all sensitive configuration
- Implement proper input validation for user data
- Monitor rate limits and implement backoff strategies
- Use TLS for all network communications

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Support

For issues and questions:
- Check the existing issues in the repository
- Review the Convex backend documentation
- Contact the development team

---

This documentation covers the complete Rust Integration API for the Convex-based Solana Trading Bot. The library provides a robust, type-safe interface for all backend operations while handling common concerns like retry logic, error handling, and rate limiting.