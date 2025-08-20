use crate::convex_client::ConvexClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

/// HTTP server for receiving webhooks from Convex
pub struct WebhookServer {
    port: u16,
    path: String,
    convex: Arc<ConvexClient>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub data: Value,
    pub timestamp: i64,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub success: bool,
    pub message: String,
}

impl WebhookServer {
    pub fn new(port: u16, path: String, convex: Arc<ConvexClient>) -> Self {
        Self { port, path, convex }
    }

    /// Start the webhook server
    pub async fn start(self) -> Result<()> {
        let convex = self.convex.clone();
        let webhook_path = self.path.clone();

        // Webhook endpoint
        let webhook_route = warp::post()
            .and(warp::path(&webhook_path[1..])) // Remove leading slash
            .and(warp::body::json())
            .and(with_convex(convex.clone()))
            .and_then(handle_webhook);

        // Health check endpoint
        let health_route = warp::get()
            .and(warp::path("health"))
            .and(with_convex(convex.clone()))
            .and_then(handle_health_check);

        // CORS configuration
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "authorization"])
            .allow_methods(vec!["GET", "POST", "OPTIONS"]);

        let routes = webhook_route
            .or(health_route)
            .with(cors)
            .recover(handle_rejection);

        println!("🚀 Webhook server starting on port {}", self.port);
        println!("📡 Webhook endpoint: http://localhost:{}{}", self.port, self.path);
        
        warp::serve(routes)
            .run(([0, 0, 0, 0], self.port))
            .await;

        Ok(())
    }
}

/// Warp filter to provide ConvexClient to handlers
fn with_convex(convex: Arc<ConvexClient>) -> impl Filter<Extract = (Arc<ConvexClient>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || convex.clone())
}

/// Handle webhook requests from Convex
async fn handle_webhook(payload: WebhookPayload, convex: Arc<ConvexClient>) -> Result<impl Reply, Rejection> {
    println!("📨 Received webhook: {} at {}", payload.event_type, payload.timestamp);

    let response = match payload.event_type.as_str() {
        "order.completed" => handle_order_completed(payload.data, convex).await,
        "order.failed" => handle_order_failed(payload.data, convex).await,
        "dca.executed" => handle_dca_executed(payload.data, convex).await,
        "alert.triggered" => handle_alert_triggered(payload.data, convex).await,
        "price.updated" => handle_price_updated(payload.data, convex).await,
        "user.created" => handle_user_created(payload.data, convex).await,
        "wallet.connected" => handle_wallet_connected(payload.data, convex).await,
        "ai.signal" => handle_ai_signal(payload.data, convex).await,
        _ => {
            println!("⚠️ Unknown webhook event type: {}", payload.event_type);
            Ok(WebhookResponse {
                success: false,
                message: format!("Unknown event type: {}", payload.event_type),
            })
        }
    };

    match response {
        Ok(resp) => {
            println!("✅ Webhook handled successfully: {}", resp.message);
            Ok(warp::reply::json(&resp))
        }
        Err(e) => {
            println!("❌ Webhook handler error: {}", e);
            Ok(warp::reply::json(&WebhookResponse {
                success: false,
                message: format!("Handler error: {}", e),
            }))
        }
    }
}

/// Handle health check requests
async fn handle_health_check(convex: Arc<ConvexClient>) -> Result<impl Reply, Rejection> {
    let convex_healthy = convex.health_check().await.unwrap_or(false);
    
    let health_status = serde_json::json!({
        "status": if convex_healthy { "healthy" } else { "unhealthy" },
        "convex": convex_healthy,
        "timestamp": chrono::Utc::now().timestamp()
    });

    Ok(warp::reply::json(&health_status))
}

// Event Handlers

async fn handle_order_completed(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let order_id = data["orderId"].as_str().unwrap_or("unknown");
    let tx_signature = data["transactionSignature"].as_str().unwrap_or("unknown");
    let user_id = data["userId"].as_str().unwrap_or("unknown");

    println!("💱 Order completed - ID: {}, TX: {}, User: {}", order_id, tx_signature, user_id);

    // Here you could:
    // - Send Telegram notification to user
    // - Update external analytics
    // - Trigger follow-up actions
    // - Log to external systems

    Ok(WebhookResponse {
        success: true,
        message: format!("Order {} processed successfully", order_id),
    })
}

async fn handle_order_failed(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let order_id = data["orderId"].as_str().unwrap_or("unknown");
    let error = data["error"].as_str().unwrap_or("unknown error");
    let user_id = data["userId"].as_str().unwrap_or("unknown");

    println!("❌ Order failed - ID: {}, Error: {}, User: {}", order_id, error, user_id);

    // Handle order failure:
    // - Send notification to user
    // - Retry logic if appropriate
    // - Log for debugging

    Ok(WebhookResponse {
        success: true,
        message: format!("Order failure {} handled", order_id),
    })
}

async fn handle_dca_executed(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let strategy_id = data["strategyId"].as_str().unwrap_or("unknown");
    let amount = data["amount"].as_str().unwrap_or("0");
    let token = data["token"].as_str().unwrap_or("unknown");
    let user_id = data["userId"].as_str().unwrap_or("unknown");

    println!("🤖 DCA executed - Strategy: {}, Amount: {} {}, User: {}", 
             strategy_id, amount, token, user_id);

    // Handle DCA execution:
    // - Update user statistics
    // - Send confirmation message
    // - Adjust strategy if needed

    Ok(WebhookResponse {
        success: true,
        message: format!("DCA execution {} processed", strategy_id),
    })
}

async fn handle_alert_triggered(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let alert_id = data["alertId"].as_str().unwrap_or("unknown");
    let token = data["token"].as_str().unwrap_or("unknown");
    let price = data["price"].as_f64().unwrap_or(0.0);
    let condition = data["condition"].as_str().unwrap_or("unknown");
    let user_id = data["userId"].as_str().unwrap_or("unknown");

    println!("🔔 Alert triggered - ID: {}, {} {} ${}, User: {}", 
             alert_id, token, condition, price, user_id);

    // Handle alert:
    // - Send immediate notification
    // - Execute associated actions
    // - Update alert status

    Ok(WebhookResponse {
        success: true,
        message: format!("Alert {} processed", alert_id),
    })
}

async fn handle_price_updated(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let token_mint = data["tokenMint"].as_str().unwrap_or("unknown");
    let symbol = data["symbol"].as_str().unwrap_or("unknown");
    let price = data["price"].as_f64().unwrap_or(0.0);
    let change_24h = data["change24h"].as_f64().unwrap_or(0.0);

    println!("📊 Price updated - {}: ${} ({:+.2}%)", symbol, price, change_24h);

    // Handle price update:
    // - Check for alert triggers
    // - Update external systems
    // - Cache for performance

    Ok(WebhookResponse {
        success: true,
        message: format!("Price update for {} processed", symbol),
    })
}

async fn handle_user_created(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let user_id = data["userId"].as_str().unwrap_or("unknown");
    let telegram_id = data["telegramId"].as_i64().unwrap_or(0);
    let username = data["username"].as_str().unwrap_or("unknown");

    println!("👤 User created - ID: {}, Telegram: {}, Username: {}", 
             user_id, telegram_id, username);

    // Handle new user:
    // - Send welcome message
    // - Set up default configurations
    // - Track user acquisition

    Ok(WebhookResponse {
        success: true,
        message: format!("User {} onboarded successfully", user_id),
    })
}

async fn handle_wallet_connected(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let wallet_id = data["walletId"].as_str().unwrap_or("unknown");
    let address = data["address"].as_str().unwrap_or("unknown");
    let user_id = data["userId"].as_str().unwrap_or("unknown");

    println!("💳 Wallet connected - ID: {}, Address: {}..., User: {}", 
             wallet_id, &address[..8], user_id);

    // Handle wallet connection:
    // - Sync initial balances
    // - Set up monitoring
    // - Send confirmation

    Ok(WebhookResponse {
        success: true,
        message: format!("Wallet {} connection processed", wallet_id),
    })
}

async fn handle_ai_signal(data: Value, _convex: Arc<ConvexClient>) -> Result<WebhookResponse> {
    let signal_id = data["signalId"].as_str().unwrap_or("unknown");
    let token_mint = data["tokenMint"].as_str().unwrap_or("unknown");
    let signal_type = data["signalType"].as_str().unwrap_or("unknown");
    let confidence = data["confidence"].as_f64().unwrap_or(0.0);

    println!("🧠 AI signal generated - ID: {}, Token: {}, Type: {}, Confidence: {:.0}%", 
             signal_id, token_mint, signal_type, confidence * 100.0);

    // Handle AI signal:
    // - Notify premium users
    // - Trigger automated trading if enabled
    // - Update signal history

    Ok(WebhookResponse {
        success: true,
        message: format!("AI signal {} processed", signal_id),
    })
}

/// Handle warp rejections
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "BAD_REQUEST";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        eprintln!("Unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "INTERNAL_SERVER_ERROR";
    }

    let json = warp::reply::json(&WebhookResponse {
        success: false,
        message: message.to_string(),
    });

    Ok(warp::reply::with_status(json, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event_type: "order.completed".to_string(),
            data: serde_json::json!({"orderId": "123", "status": "completed"}),
            timestamp: 1234567890,
            signature: Some("test_signature".to_string()),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: WebhookPayload = serde_json::from_str(&json).unwrap();
        
        assert_eq!(payload.event_type, deserialized.event_type);
        assert_eq!(payload.timestamp, deserialized.timestamp);
    }
}