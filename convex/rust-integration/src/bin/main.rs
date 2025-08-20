use convex_integration::{ConvexConfig, ConvexIntegrationService};
use std::env;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Load configuration from environment
    let config = ConvexConfig {
        convex_url: env::var("CONVEX_URL")
            .unwrap_or_else(|_| "https://your-convex-app.convex.site".to_string()),
        convex_site_url: env::var("CONVEX_SITE_URL")
            .unwrap_or_else(|_| "https://your-convex-app.convex.cloud".to_string()),
        telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
            .unwrap_or_default(),
        webhook_port: env::var("WEBHOOK_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080),
        webhook_path: env::var("WEBHOOK_PATH")
            .unwrap_or_else(|_| "/webhook".to_string()),
    };

    println!("🚀 Starting Convex Integration Service");
    println!("📡 Convex URL: {}", config.convex_url);
    println!("🤖 Telegram Bot: {}", if config.telegram_bot_token.is_empty() { "Disabled" } else { "Enabled" });
    println!("🌐 Webhook Server: http://localhost:{}{}", config.webhook_port, config.webhook_path);

    // Create and start the integration service
    let service = ConvexIntegrationService::new(config).await?;

    // Perform health check before starting
    match service.health_check().await {
        Ok(_) => println!("✅ Health check passed"),
        Err(e) => {
            eprintln!("❌ Health check failed: {}", e);
            eprintln!("Make sure Convex is running and accessible");
            return Err(e);
        }
    }

    // Start the service (this will run indefinitely)
    service.start().await?;

    Ok(())
}