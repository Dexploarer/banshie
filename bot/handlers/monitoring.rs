use teloxide::{prelude::*, types::Message};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use std::sync::Arc;
use tracing::{info, error};

use crate::monitoring::{MonitoringIntegration, HealthStatus};

/// Monitoring command handler for bot
pub struct MonitoringHandler;

impl MonitoringHandler {
    /// Handle /monitor command - Show monitoring dashboard
    pub async fn handle_monitor(
        bot: Bot,
        msg: Message,
        args: String,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.is_empty() {
            // Show monitoring menu
            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![
                    InlineKeyboardButton::callback("📊 System Health", "mon_health"),
                    InlineKeyboardButton::callback("📈 Metrics", "mon_metrics"),
                ],
                vec![
                    InlineKeyboardButton::callback("🔍 Telemetry", "mon_telemetry"),
                    InlineKeyboardButton::callback("🚨 Alerts", "mon_alerts"),
                ],
                vec![
                    InlineKeyboardButton::callback("⚙️ Settings", "mon_settings"),
                    InlineKeyboardButton::url("🌐 Dashboard", "http://127.0.0.1:3000/dashboard"),
                ],
            ]);
            
            let message = r#"📊 *Monitoring & Observability*

Monitor system health, performance metrics, and alerts\.

*System Overview:*
• Health checks for all components
• Real\-time performance metrics
• Distributed tracing
• Automated alerting

*Available Commands:*
`/monitor health` \- System health status
`/monitor metrics` \- Performance metrics
`/monitor alerts` \- Active alerts
`/monitor trace <operation>` \- Trace operation

*Dashboard:* http://127\.0\.0\.1:3000/dashboard

Select an option below:"#;
            
            bot.send_message(msg.chat.id, message)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .reply_markup(keyboard)
                .await?;
            
            return Ok(());
        }
        
        match parts[0] {
            "health" => {
                Self::show_health_status(bot, msg, monitoring).await?;
            }
            "metrics" => {
                Self::show_metrics_summary(bot, msg, monitoring).await?;
            }
            "alerts" => {
                Self::show_active_alerts(bot, msg, monitoring).await?;
            }
            "telemetry" => {
                Self::show_telemetry_stats(bot, msg, monitoring).await?;
            }
            "trace" => {
                if parts.len() < 2 {
                    bot.send_message(msg.chat.id, 
                        "❌ Usage: `/monitor trace <operation_name>`")
                        .await?;
                    return Ok(());
                }
                Self::start_trace_operation(bot, msg, parts[1], monitoring).await?;
            }
            _ => {
                bot.send_message(msg.chat.id, 
                    "❌ Unknown monitoring command. Use `/monitor` to see options.")
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Show system health status
    async fn show_health_status(
        bot: Bot,
        msg: Message,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        let status = monitoring.get_status().await;
        
        let overall_emoji = match status.health.status {
            HealthStatus::Healthy => "✅",
            HealthStatus::Degraded => "⚠️",
            HealthStatus::Unhealthy => "❌",
            HealthStatus::Unknown => "❓",
        };
        
        let mut message = format!(
            "{} **System Health Status**\n\n\
            **Overall Status:** {:?}\n\
            **Uptime:** {} seconds\n\
            **Version:** {}\n\
            **Components:** {}\n\n\
            **Component Details:**\n",
            overall_emoji,
            status.health.status,
            status.health.uptime_seconds,
            status.health.version,
            status.health.components.len()
        );
        
        for (component, result) in &status.health.components {
            let emoji = match result.status {
                HealthStatus::Healthy => "✅",
                HealthStatus::Degraded => "⚠️",
                HealthStatus::Unhealthy => "❌",
                HealthStatus::Unknown => "❓",
            };
            
            message.push_str(&format!(
                "{} **{}:** {:?} ({}ms)\n",
                emoji,
                component,
                result.status,
                result.duration_ms
            ));
        }
        
        message.push_str(&format!(
            "\n**Active Alerts:** {}\n\
            **Dashboard Running:** {}",
            status.active_alerts_count,
            if status.dashboard_running { "✅ Yes" } else { "❌ No" }
        ));
        
        bot.send_message(msg.chat.id, message).await?;
        Ok(())
    }
    
    /// Show metrics summary
    async fn show_metrics_summary(
        bot: Bot,
        msg: Message,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        let status = monitoring.get_status().await;
        let metrics = &status.metrics_summary;
        
        let success_rate = if metrics.total_trades > 0.0 {
            (metrics.successful_trades / metrics.total_trades) * 100.0
        } else {
            0.0
        };
        
        let message = format!(
            "📈 **Performance Metrics**\n\n\
            **Trading Performance:**\n\
            • Total Trades: {}\n\
            • Successful: {} ({:.1}%)\n\
            • Failed: {}\n\
            • Volume: {} SOL\n\n\
            **System Performance:**\n\
            • Commands Processed: {}\n\
            • API Calls: {}\n\
            • Cache Hit Rate: {:.1}%\n\
            • Total Errors: {}\n\n\
            **Telemetry:**\n\
            • Total Spans: {}\n\
            • Active Spans: {}\n\
            • Avg Duration: {:.2}ms\n\n\
            **System:**\n\
            • Uptime: {:.1} hours\n\
            • Custom Metrics: {}",
            metrics.total_trades,
            metrics.successful_trades,
            success_rate,
            metrics.failed_trades,
            metrics.total_volume_sol,
            metrics.total_commands,
            metrics.total_api_calls,
            metrics.cache_hit_rate * 100.0,
            metrics.total_errors,
            status.telemetry_stats.total_spans,
            status.telemetry_stats.active_spans,
            status.telemetry_stats.average_duration_ms,
            metrics.uptime_seconds as f64 / 3600.0,
            metrics.custom_metrics_count
        );
        
        bot.send_message(msg.chat.id, message).await?;
        Ok(())
    }
    
    /// Show active alerts
    async fn show_active_alerts(
        bot: Bot,
        msg: Message,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        let active_alerts = monitoring.alert_manager.get_active_alerts().await;
        
        if active_alerts.is_empty() {
            bot.send_message(msg.chat.id, 
                "✅ **No Active Alerts**\n\nAll systems operating normally.")
                .await?;
            return Ok(());
        }
        
        let mut message = format!("🚨 **Active Alerts ({}):**\n\n", active_alerts.len());
        
        for (i, alert) in active_alerts.iter().take(10).enumerate() {
            let emoji = match alert.severity {
                crate::monitoring::alerts::AlertSeverity::Emergency => "🚨",
                crate::monitoring::alerts::AlertSeverity::Critical => "❌",
                crate::monitoring::alerts::AlertSeverity::Warning => "⚠️",
                crate::monitoring::alerts::AlertSeverity::Info => "ℹ️",
            };
            
            message.push_str(&format!(
                "{}. {} **{}**\n   {:?} | Value: {} | Threshold: {}\n   {}\n\n",
                i + 1,
                emoji,
                alert.title,
                alert.severity,
                alert.metric_value,
                alert.threshold,
                alert.triggered_at.format("%H:%M:%S UTC")
            ));
        }
        
        if active_alerts.len() > 10 {
            message.push_str(&format!("... and {} more alerts", active_alerts.len() - 10));
        }
        
        bot.send_message(msg.chat.id, message).await?;
        Ok(())
    }
    
    /// Show telemetry statistics
    async fn show_telemetry_stats(
        bot: Bot,
        msg: Message,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        let telemetry_stats = monitoring.telemetry.get_telemetry_stats().await;
        let active_spans = monitoring.telemetry.get_active_spans().await;
        
        let message = format!(
            "🔍 **Telemetry Statistics**\n\n\
            **Span Summary:**\n\
            • Total Spans: {}\n\
            • Active Spans: {}\n\
            • Completed Spans: {}\n\
            • Unique Operations: {}\n\
            • Average Duration: {:.2}ms\n\n\
            **Active Operations:**\n",
            telemetry_stats.total_spans,
            telemetry_stats.active_spans,
            telemetry_stats.completed_spans,
            telemetry_stats.unique_operations,
            telemetry_stats.average_duration_ms
        );
        
        let mut operations_message = message;
        
        for (i, span) in active_spans.iter().take(5).enumerate() {
            let duration = chrono::Utc::now()
                .signed_duration_since(span.start_time)
                .num_milliseconds();
            
            operations_message.push_str(&format!(
                "{}. **{}** ({}ms)\n   Trace: {}...\n",
                i + 1,
                span.operation_name,
                duration,
                &span.trace_id[..8]
            ));
        }
        
        if active_spans.len() > 5 {
            operations_message.push_str(&format!("... and {} more operations", active_spans.len() - 5));
        }
        
        if active_spans.is_empty() {
            operations_message.push_str("No active operations");
        }
        
        bot.send_message(msg.chat.id, operations_message).await?;
        Ok(())
    }
    
    /// Start a trace operation
    async fn start_trace_operation(
        bot: Bot,
        msg: Message,
        operation: &str,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        let span_id = monitoring.start_span(operation).await;
        
        bot.send_message(msg.chat.id, 
            format!("🔍 **Trace Started**\n\n\
            Operation: {}\n\
            Span ID: {}\n\
            Status: Active\n\n\
            Use `/monitor telemetry` to view active traces.", 
            operation, &span_id[..8]))
            .await?;
        
        // Simulate some work for demo
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        monitoring.finish_span(&span_id).await;
        
        bot.send_message(msg.chat.id, 
            format!("✅ **Trace Completed**\n\n\
            Operation: {}\n\
            Span ID: {}\n\
            Status: Completed", 
            operation, &span_id[..8]))
            .await?;
        
        Ok(())
    }
    
    /// Handle monitoring callback queries
    pub async fn handle_monitoring_callback(
        bot: Bot,
        callback_query: teloxide::types::CallbackQuery,
        monitoring: Arc<MonitoringIntegration>,
    ) -> ResponseResult<()> {
        if let Some(data) = &callback_query.data {
            if let Some(msg) = &callback_query.message {
                match data.as_str() {
                    "mon_health" => {
                        Self::show_health_status(bot, msg.clone(), monitoring).await?;
                    }
                    "mon_metrics" => {
                        Self::show_metrics_summary(bot, msg.clone(), monitoring).await?;
                    }
                    "mon_telemetry" => {
                        Self::show_telemetry_stats(bot, msg.clone(), monitoring).await?;
                    }
                    "mon_alerts" => {
                        Self::show_active_alerts(bot, msg.clone(), monitoring).await?;
                    }
                    "mon_settings" => {
                        bot.send_message(msg.chat.id, 
                            "⚙️ **Monitoring Settings**\n\n\
                            Dashboard: http://127.0.0.1:3000\n\
                            Metrics: http://127.0.0.1:3000/metrics\n\
                            Health: http://127.0.0.1:3000/health\n\n\
                            Use the web dashboard for detailed configuration.")
                            .await?;
                    }
                    _ => {}
                }
            }
        }
        
        // Answer the callback query
        bot.answer_callback_query(callback_query.id).await?;
        Ok(())
    }
}