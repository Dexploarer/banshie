//! Convex-Rust Integration Library
//! 
//! This library provides integration between Rust services and Convex backend,
//! specifically designed for the Solana Trading Bot project.

pub mod convex_client;
pub mod telegram_integration;
pub mod trading_service;
pub mod webhook_server;

pub use convex_client::ConvexClient;
pub use telegram_integration::TelegramConvexBridge;

use anyhow::Result;
use std::sync::Arc;

/// Configuration for the Convex integration
#[derive(Clone)]
pub struct ConvexConfig {
    pub convex_url: String,
    pub convex_site_url: String,
    pub telegram_bot_token: String,
    pub webhook_port: u16,
    pub webhook_path: String,
}

impl Default for ConvexConfig {
    fn default() -> Self {
        Self {
            convex_url: "https://your-convex-app.convex.site".to_string(),
            convex_site_url: "https://your-convex-app.convex.cloud".to_string(),
            telegram_bot_token: String::new(),
            webhook_port: 8080,
            webhook_path: "/webhook".to_string(),
        }
    }
}

/// Main integration service that coordinates all components
pub struct ConvexIntegrationService {
    pub convex_client: Arc<ConvexClient>,
    pub telegram_bridge: Option<TelegramConvexBridge>,
    pub config: ConvexConfig,
}

impl ConvexIntegrationService {
    /// Create a new integration service
    pub async fn new(config: ConvexConfig) -> Result<Self> {
        let convex_client = Arc::new(ConvexClient::new()?);
        
        let telegram_bridge = if !config.telegram_bot_token.is_empty() {
            let bot = teloxide::Bot::new(&config.telegram_bot_token);
            Some(TelegramConvexBridge::new(bot, convex_client.clone()))
        } else {
            None
        };

        Ok(Self {
            convex_client,
            telegram_bridge,
            config,
        })
    }

    /// Start all services
    pub async fn start(&self) -> Result<()> {
        // Start webhook server for Convex -> Rust communication
        let webhook_server = webhook_server::WebhookServer::new(
            self.config.webhook_port,
            self.config.webhook_path.clone(),
            self.convex_client.clone(),
        );

        tokio::spawn(async move {
            if let Err(e) = webhook_server.start().await {
                eprintln!("Webhook server error: {}", e);
            }
        });

        // Start Telegram bot if configured
        if let Some(telegram_bridge) = &self.telegram_bridge {
            let bridge = telegram_bridge.clone();
            tokio::spawn(async move {
                if let Err(e) = start_telegram_bot(bridge).await {
                    eprintln!("Telegram bot error: {}", e);
                }
            });
        }

        // Keep the service running
        tokio::signal::ctrl_c().await?;
        println!("Shutting down Convex integration service...");

        Ok(())
    }

    /// Health check for all components
    pub async fn health_check(&self) -> Result<()> {
        // Check Convex connection
        if !self.convex_client.health_check().await? {
            return Err(anyhow::anyhow!("Convex health check failed"));
        }

        println!("✅ All components healthy");
        Ok(())
    }
}

async fn start_telegram_bot(bridge: TelegramConvexBridge) -> Result<()> {
    use teloxide::{prelude::*, update_listeners::webhooks};

    let bot = bridge.bot.clone();
    
    // Use polling for simplicity - in production, consider webhooks
    let mut dispatcher = Dispatcher::builder(bot, move |update: Update| {
        let bridge = bridge.clone();
        async move {
            match update {
                Update::Message(msg) => {
                    if let Err(e) = bridge.handle_message(msg).await {
                        eprintln!("Error handling message: {}", e);
                    }
                }
                Update::InlineQuery(query) => {
                    if let Err(e) = bridge.handle_inline_query(query).await {
                        eprintln!("Error handling inline query: {}", e);
                    }
                }
                _ => {}
            }

            teloxide::respond(())
        }
    })
    .build();

    dispatcher.dispatch().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_service_creation() {
        let config = ConvexConfig::default();
        let service = ConvexIntegrationService::new(config).await;
        assert!(service.is_ok());
    }
}