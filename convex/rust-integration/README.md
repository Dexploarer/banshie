# Convex-Rust Integration

This library provides integration between Rust services and the Convex backend for the Solana Trading Bot project.

## Features

- **HTTP Client for Convex**: Direct communication with Convex queries, mutations, and actions
- **Telegram Bot Integration**: Telegram bot that integrates with Convex for real-time trading
- **Trading Service**: Jupiter DEX integration with Convex order management
- **Webhook Server**: Receive webhooks from Convex for event-driven processing
- **Type Safety**: Full TypeScript-like type safety for Convex operations

## Architecture

```
┌─────────────────┐    HTTP API     ┌─────────────────┐
│   Rust Service  │ ◄──────────────► │  Convex Backend │
│                 │                 │                 │
│ ┌─────────────┐ │                 │ ┌─────────────┐ │
│ │ ConvexClient│ │                 │ │   Queries   │ │
│ ├─────────────┤ │                 │ ├─────────────┤ │
│ │TelegramBridge│ │                 │ │  Mutations  │ │
│ ├─────────────┤ │                 │ ├─────────────┤ │
│ │TradingService│ │   Webhooks      │ │   Actions   │ │
│ ├─────────────┤ │ ◄──────────────► │ ├─────────────┤ │
│ │WebhookServer│ │                 │ │    Crons    │ │
│ └─────────────┘ │                 │ └─────────────┘ │
└─────────────────┘                 └─────────────────┘
        │                                   │
        ▼                                   ▼
┌─────────────────┐                 ┌─────────────────┐
│  Telegram API   │                 │   Jupiter API   │
└─────────────────┘                 └─────────────────┘
```

## Quick Start

### 1. Environment Setup

Create a `.env` file:

```env
# Convex Configuration
CONVEX_URL=https://your-convex-app.convex.site
CONVEX_SITE_URL=https://your-convex-app.convex.cloud

# Telegram Bot
TELEGRAM_BOT_TOKEN=your_telegram_bot_token

# Webhook Server
WEBHOOK_PORT=8080
WEBHOOK_PATH=/webhook

# Logging
RUST_LOG=info
```

### 2. Build and Run

```bash
# Build the project
cargo build --release

# Run the integration service
cargo run --bin integration-service

# Or run with specific features
cargo run --bin integration-service --features="telegram,webhooks"
```

### 3. Use as a Library

```rust
use convex_integration::{ConvexClient, ConvexConfig, ConvexIntegrationService};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create Convex client
    let client = Arc::new(ConvexClient::new()?);
    
    // Get user portfolio
    let portfolio = client.get_portfolio("user_123").await?;
    println!("Portfolio value: ${}", portfolio.total_value);
    
    // Place a trade
    let order = OrderRequest {
        user_id: "user_123".to_string(),
        order_type: "market".to_string(),
        token_mint: "So11111111111111111111111111111111111111112".to_string(),
        side: "buy".to_string(),
        amount: "100.0".to_string(),
        price: None,
        slippage: Some(0.01),
    };
    
    let order_id = client.place_order(order).await?;
    println!("Order placed: {}", order_id);
    
    Ok(())
}
```

## API Reference

### ConvexClient

The main client for interacting with Convex backend.

```rust
impl ConvexClient {
    // Core methods
    pub async fn query<T>(&self, function_name: &str, args: Value) -> Result<T>
    pub async fn mutation<T>(&self, function_name: &str, args: Value) -> Result<T>
    pub async fn action<T>(&self, function_name: &str, args: Value) -> Result<T>
    
    // User management
    pub async fn get_user_by_telegram_id(&self, telegram_id: i64) -> Result<Option<UserProfile>>
    pub async fn create_or_update_user(&self, telegram_id: i64, username: &str) -> Result<String>
    
    // Portfolio
    pub async fn get_portfolio(&self, user_id: &str) -> Result<PortfolioSummary>
    pub async fn sync_wallet_balances(&self, user_id: &str, wallet_address: &str) -> Result<Value>
    
    // Trading
    pub async fn place_order(&self, order: OrderRequest) -> Result<String>
    pub async fn get_order_status(&self, order_id: &str) -> Result<Value>
    
    // AI Signals
    pub async fn get_latest_signals(&self, limit: u32) -> Result<Vec<TradingSignal>>
    pub async fn generate_signal(&self, token_mint: &str) -> Result<TradingSignal>
    
    // Price Data
    pub async fn get_token_price(&self, token_mint: &str) -> Result<Value>
    pub async fn update_prices(&self, tokens: Vec<&str>) -> Result<Value>
    
    // DCA Strategies
    pub async fn get_user_dca_strategies(&self, user_id: &str) -> Result<Vec<Value>>
    pub async fn create_dca_strategy(&self, user_id: &str, token_mint: &str, amount: f64, frequency: &str) -> Result<String>
    
    // Alerts
    pub async fn create_price_alert(&self, user_id: &str, token_mint: &str, target_price: f64, condition: &str) -> Result<String>
    pub async fn get_user_alerts(&self, user_id: &str) -> Result<Vec<Value>>
    
    // Analytics
    pub async fn calculate_indicators(&self, token_mint: &str) -> Result<Value>
    
    // Utility
    pub async fn health_check(&self) -> Result<bool>
    pub async fn retry_with_backoff<F, T, E>(&self, f: F, max_retries: u32) -> Result<T>
}
```

### TradingService

High-level trading operations with Jupiter integration.

```rust
impl TradingService {
    // Market orders
    pub async fn execute_market_buy(&self, user_id: &str, input_token_mint: &str, output_token_mint: &str, input_amount: u64, slippage_bps: Option<u16>, wallet_address: &str) -> Result<String>
    pub async fn execute_market_sell(&self, user_id: &str, input_token_mint: &str, output_token_mint: &str, input_amount: u64, slippage_bps: Option<u16>, wallet_address: &str) -> Result<String>
    
    // Analysis
    pub async fn get_best_route(&self, input_mint: &str, output_mint: &str, amount: u64) -> Result<QuoteResponse>
    pub async fn calculate_price_impact(&self, input_mint: &str, output_mint: &str, amount: u64) -> Result<f64>
    
    // DCA
    pub async fn execute_dca_order(&self, strategy_id: &str, user_id: &str, wallet_address: &str) -> Result<String>
    
    // Monitoring
    pub async fn monitor_order_execution(&self, order_id: &str) -> Result<String>
    
    // Validation
    pub fn validate_trade_params(&self, input_mint: &str, output_mint: &str, amount: u64, slippage_bps: Option<u16>) -> Result<()>
    
    // Data
    pub async fn get_supported_tokens(&self) -> Result<Vec<Value>>
    pub async fn calculate_trade_fees(&self, quote: &QuoteResponse) -> Result<Value>
}
```

### TelegramConvexBridge

Telegram bot integration with Convex backend.

```rust
impl TelegramConvexBridge {
    // Message handling
    pub async fn handle_message(&self, msg: Message) -> Result<()>
    pub async fn handle_command(&self, msg: &Message, command: Command) -> Result<()>
    pub async fn handle_inline_query(&self, query: InlineQuery) -> Result<()>
    
    // Command handlers
    async fn handle_start_command(&self, chat_id: ChatId, user_id: i64) -> Result<()>
    async fn handle_portfolio_command(&self, chat_id: ChatId, user_id: i64) -> Result<()>
    async fn handle_trade_command(&self, chat_id: ChatId, user_id: i64, token: Option<String>) -> Result<()>
    async fn handle_dca_command(&self, chat_id: ChatId, user_id: i64) -> Result<()>
    async fn handle_signals_command(&self, chat_id: ChatId, user_id: i64) -> Result<()>
    async fn handle_alerts_command(&self, chat_id: ChatId, user_id: i64) -> Result<()>
    async fn handle_wallet_command(&self, chat_id: ChatId, user_id: i64) -> Result<()>
    async fn handle_help_command(&self, chat_id: ChatId) -> Result<()>
}
```

### WebhookServer

HTTP server for receiving Convex webhooks.

```rust
impl WebhookServer {
    pub fn new(port: u16, path: String, convex: Arc<ConvexClient>) -> Self
    pub async fn start(self) -> Result<()>
}

// Supported webhook events
- order.completed
- order.failed
- dca.executed
- alert.triggered
- price.updated
- user.created
- wallet.connected
- ai.signal
```

## Integration Examples

### 1. Basic Portfolio Check

```rust
use convex_integration::ConvexClient;

let client = ConvexClient::new()?;
let portfolio = client.get_portfolio("user_123").await?;
println!("Total value: ${}", portfolio.total_value);
```

### 2. Execute a Trade

```rust
use convex_integration::{ConvexClient, TradingService};

let client = Arc::new(ConvexClient::new()?);
let trading_service = TradingService::new(client);

let order_id = trading_service.execute_market_buy(
    "user_123",
    "So11111111111111111111111111111111111111112", // SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    1_000_000_000, // 1 SOL
    Some(100), // 1% slippage
    "wallet_address"
).await?;

println!("Order placed: {}", order_id);
```

### 3. Set Up DCA Strategy

```rust
let strategy_id = client.create_dca_strategy(
    "user_123",
    "So11111111111111111111111111111111111111112", // SOL
    50.0, // $50
    "daily"
).await?;

println!("DCA strategy created: {}", strategy_id);
```

### 4. Handle Telegram Commands

```rust
use convex_integration::TelegramConvexBridge;
use teloxide::prelude::*;

let bot = Bot::new("your_bot_token");
let client = Arc::new(ConvexClient::new()?);
let bridge = TelegramConvexBridge::new(bot, client);

// In your message handler
if let Some(text) = msg.text() {
    if text.starts_with('/') {
        let command = Command::parse(text, "YourBot")?;
        bridge.handle_command(&msg, command).await?;
    }
}
```

### 5. Process Webhooks

```rust
use convex_integration::WebhookServer;

let webhook_server = WebhookServer::new(
    8080,
    "/webhook".to_string(),
    client
);

// Start server (runs indefinitely)
webhook_server.start().await?;
```

## Error Handling

All methods return `anyhow::Result<T>` for consistent error handling:

```rust
match client.get_portfolio("user_123").await {
    Ok(portfolio) => println!("Portfolio: ${}", portfolio.total_value),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

Run tests with:

```bash
# Unit tests
cargo test

# Integration tests (requires running Convex instance)
cargo test --features integration-tests

# Test specific module
cargo test convex_client::tests
```

## Configuration

### Environment Variables

- `CONVEX_URL`: Your Convex deployment URL
- `CONVEX_SITE_URL`: Your Convex site URL  
- `TELEGRAM_BOT_TOKEN`: Telegram bot token
- `WEBHOOK_PORT`: Port for webhook server (default: 8080)
- `WEBHOOK_PATH`: Path for webhook endpoint (default: /webhook)
- `RUST_LOG`: Log level (debug, info, warn, error)

### Convex Setup

Make sure your Convex backend has the corresponding functions:

```typescript
// convex/queries/portfolio.ts
export const getPortfolio = query({ ... });

// convex/mutations/trading.ts  
export const placeTrade = mutation({ ... });

// convex/actions/solana.ts
export const executeTrade = action({ ... });
```

## Production Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/integration-service /usr/local/bin/
CMD ["integration-service"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: convex-integration
spec:
  replicas: 2
  selector:
    matchLabels:
      app: convex-integration
  template:
    metadata:
      labels:
        app: convex-integration
    spec:
      containers:
      - name: integration-service
        image: your-registry/convex-integration:latest
        env:
        - name: CONVEX_URL
          value: "https://your-app.convex.site"
        - name: TELEGRAM_BOT_TOKEN
          valueFrom:
            secretKeyRef:
              name: telegram-secret
              key: token
        ports:
        - containerPort: 8080
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Run `cargo test` and `cargo clippy`
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.