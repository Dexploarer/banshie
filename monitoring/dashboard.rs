use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::{info, error};

use super::{
    metrics::{MetricsCollector, MetricsSummary},
    health::{HealthCheck, SystemHealth},
    telemetry::{TelemetryService, TelemetryStats},
};

/// Dashboard server configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub metrics_path: String,
    pub health_path: String,
    pub dashboard_path: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            enable_cors: true,
            metrics_path: "/metrics".to_string(),
            health_path: "/health".to_string(),
            dashboard_path: "/dashboard".to_string(),
        }
    }
}

/// Dashboard server state
#[derive(Clone)]
pub struct AppState {
    pub metrics: Arc<MetricsCollector>,
    pub health_check: Arc<HealthCheck>,
    pub telemetry: Arc<TelemetryService>,
}

/// Dashboard server
pub struct DashboardServer {
    config: DashboardConfig,
    state: AppState,
}

impl DashboardServer {
    pub fn new(
        config: DashboardConfig,
        metrics: Arc<MetricsCollector>,
        health_check: Arc<HealthCheck>,
        telemetry: Arc<TelemetryService>,
    ) -> Self {
        let state = AppState {
            metrics,
            health_check,
            telemetry,
        };
        
        Self { config, state }
    }
    
    /// Start the dashboard server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.create_router();
        
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("Dashboard server starting on http://{}", addr);
        info!("  - Metrics: http://{}{}", addr, self.config.metrics_path);
        info!("  - Health: http://{}{}", addr, self.config.health_path);
        info!("  - Dashboard: http://{}{}", addr, self.config.dashboard_path);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
    
    /// Create the router with all routes
    fn create_router(&self) -> Router {
        let mut router = Router::new()
            .route(&self.config.metrics_path, get(metrics_handler))
            .route(&self.config.health_path, get(health_handler))
            .route(&self.config.dashboard_path, get(dashboard_handler))
            .route("/api/metrics", get(api_metrics_handler))
            .route("/api/health", get(api_health_handler))
            .route("/api/telemetry", get(api_telemetry_handler))
            .route("/api/dashboard-data", get(dashboard_data_handler))
            .with_state(self.state.clone());
        
        if self.config.enable_cors {
            router = router.layer(CorsLayer::permissive());
        }
        
        router.layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
        )
    }
}

/// Prometheus metrics endpoint
async fn metrics_handler(State(state): State<AppState>) -> Result<String, StatusCode> {
    let metrics = state.metrics.gather();
    
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    
    match encoder.encode_to_string(&metrics) {
        Ok(result) => Ok(result),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Health check endpoint
async fn health_handler(State(state): State<AppState>) -> Result<Json<SystemHealth>, StatusCode> {
    let health = state.health_check.get_health().await;
    Ok(Json(health))
}

/// API metrics endpoint
async fn api_metrics_handler(State(state): State<AppState>) -> Result<Json<MetricsSummary>, StatusCode> {
    let summary = state.metrics.get_summary().await;
    Ok(Json(summary))
}

/// API health endpoint (detailed)
async fn api_health_handler(State(state): State<AppState>) -> Result<Json<SystemHealth>, StatusCode> {
    let health = state.health_check.check_all().await;
    Ok(Json(health))
}

/// API telemetry endpoint
async fn api_telemetry_handler(State(state): State<AppState>) -> Result<Json<TelemetryStats>, StatusCode> {
    let stats = state.telemetry.get_telemetry_stats().await;
    Ok(Json(stats))
}

/// Dashboard data endpoint
async fn dashboard_data_handler(State(state): State<AppState>) -> Result<Json<DashboardData>, StatusCode> {
    let metrics = state.metrics.get_summary().await;
    let health = state.health_check.get_health().await;
    let telemetry = state.telemetry.get_telemetry_stats().await;
    
    let data = DashboardData {
        metrics,
        health,
        telemetry,
        timestamp: chrono::Utc::now(),
    };
    
    Ok(Json(data))
}

/// Main dashboard HTML page
async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// Combined dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub metrics: MetricsSummary,
    pub health: SystemHealth,
    pub telemetry: TelemetryStats,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Metrics dashboard utility
pub struct MetricsDashboard;

impl MetricsDashboard {
    /// Generate HTML dashboard
    pub fn generate_html(data: &DashboardData) -> String {
        format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Solana Trading Bot - Dashboard</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <style>
                    {}
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>🚀 Solana Trading Bot Dashboard</h1>
                    
                    <div class="grid">
                        <div class="card">
                            <h2>📊 System Health</h2>
                            <div class="status status-{}">
                                Status: {:?}
                            </div>
                            <p>Uptime: {} seconds</p>
                            <p>Components: {} total</p>
                        </div>
                        
                        <div class="card">
                            <h2>💹 Trading Metrics</h2>
                            <p>Total Trades: {}</p>
                            <p>Successful: {}</p>
                            <p>Failed: {}</p>
                            <p>Volume: {} SOL</p>
                        </div>
                        
                        <div class="card">
                            <h2>🔍 Telemetry</h2>
                            <p>Total Spans: {}</p>
                            <p>Active Spans: {}</p>
                            <p>Avg Duration: {:.2}ms</p>
                        </div>
                        
                        <div class="card">
                            <h2>🤖 Bot Performance</h2>
                            <p>Commands: {}</p>
                            <p>API Calls: {}</p>
                            <p>Cache Hit Rate: {:.1}%</p>
                            <p>Errors: {}</p>
                        </div>
                    </div>
                    
                    <div class="components">
                        <h2>🔧 Component Status</h2>
                        <div class="component-grid">
                            {}
                        </div>
                    </div>
                    
                    <div class="footer">
                        <p>Last updated: {} | Version: {}</p>
                    </div>
                </div>
                
                <script>
                    // Auto-refresh every 30 seconds
                    setTimeout(() => location.reload(), 30000);
                </script>
            </body>
            </html>
            "#,
            DASHBOARD_CSS,
            match data.health.status {
                crate::monitoring::health::HealthStatus::Healthy => "healthy",
                crate::monitoring::health::HealthStatus::Degraded => "degraded",
                crate::monitoring::health::HealthStatus::Unhealthy => "unhealthy",
                crate::monitoring::health::HealthStatus::Unknown => "unknown",
            },
            data.health.status,
            data.health.uptime_seconds,
            data.health.components.len(),
            data.metrics.total_trades,
            data.metrics.successful_trades,
            data.metrics.failed_trades,
            data.metrics.total_volume_sol,
            data.telemetry.total_spans,
            data.telemetry.active_spans,
            data.telemetry.average_duration_ms,
            data.metrics.total_commands,
            data.metrics.total_api_calls,
            data.metrics.cache_hit_rate * 100.0,
            data.metrics.total_errors,
            Self::generate_component_cards(&data.health),
            data.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            data.health.version
        )
    }
    
    /// Generate component status cards
    fn generate_component_cards(health: &SystemHealth) -> String {
        let mut cards = String::new();
        
        for (component, result) in &health.components {
            let status_class = match result.status {
                crate::monitoring::health::HealthStatus::Healthy => "healthy",
                crate::monitoring::health::HealthStatus::Degraded => "degraded",
                crate::monitoring::health::HealthStatus::Unhealthy => "unhealthy",
                crate::monitoring::health::HealthStatus::Unknown => "unknown",
            };
            
            cards.push_str(&format!(
                r#"
                <div class="component-card">
                    <h3>{}</h3>
                    <div class="status status-{}">{:?}</div>
                    <p>{}</p>
                    <small>Duration: {}ms</small>
                </div>
                "#,
                component,
                status_class,
                result.status,
                result.message,
                result.duration_ms
            ));
        }
        
        cards
    }
}

/// Dashboard CSS styles
const DASHBOARD_CSS: &str = r#"
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        margin: 0;
        padding: 20px;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: #333;
        min-height: 100vh;
    }
    
    .container {
        max-width: 1200px;
        margin: 0 auto;
        background: white;
        border-radius: 12px;
        padding: 30px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.1);
    }
    
    h1 {
        text-align: center;
        color: #2c3e50;
        margin-bottom: 30px;
    }
    
    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
        gap: 20px;
        margin-bottom: 30px;
    }
    
    .card {
        background: #f8f9fa;
        border-radius: 8px;
        padding: 20px;
        border-left: 4px solid #667eea;
    }
    
    .card h2 {
        margin-top: 0;
        color: #2c3e50;
        font-size: 1.2em;
    }
    
    .status {
        padding: 8px 12px;
        border-radius: 4px;
        font-weight: bold;
        text-align: center;
        margin: 10px 0;
    }
    
    .status-healthy { background: #d4edda; color: #155724; }
    .status-degraded { background: #fff3cd; color: #856404; }
    .status-unhealthy { background: #f8d7da; color: #721c24; }
    .status-unknown { background: #e2e3e5; color: #383d41; }
    
    .component-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 15px;
    }
    
    .component-card {
        background: #f8f9fa;
        border-radius: 6px;
        padding: 15px;
        border: 1px solid #e9ecef;
    }
    
    .component-card h3 {
        margin-top: 0;
        margin-bottom: 10px;
        color: #495057;
        font-size: 1em;
    }
    
    .footer {
        text-align: center;
        margin-top: 30px;
        padding-top: 20px;
        border-top: 1px solid #e9ecef;
        color: #6c757d;
    }
    
    @media (max-width: 768px) {
        .container { padding: 15px; }
        .grid { grid-template-columns: 1fr; }
    }
"#;

/// Static dashboard HTML
const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Solana Trading Bot - Dashboard</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #333;
            min-height: 100vh;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 12px;
            padding: 30px;
            box-shadow: 0 8px 32px rgba(0,0,0,0.1);
        }
        
        h1 {
            text-align: center;
            color: #2c3e50;
            margin-bottom: 30px;
        }
        
        .loading {
            text-align: center;
            padding: 50px;
            color: #6c757d;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .card {
            background: #f8f9fa;
            border-radius: 8px;
            padding: 20px;
            border-left: 4px solid #667eea;
        }
        
        .status {
            padding: 8px 12px;
            border-radius: 4px;
            font-weight: bold;
            text-align: center;
            margin: 10px 0;
        }
        
        .status-healthy { background: #d4edda; color: #155724; }
        .status-degraded { background: #fff3cd; color: #856404; }
        .status-unhealthy { background: #f8d7da; color: #721c24; }
        .status-unknown { background: #e2e3e5; color: #383d41; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Solana Trading Bot Dashboard</h1>
        <div id="content" class="loading">
            <div>Loading dashboard data...</div>
        </div>
    </div>
    
    <script>
        async function loadDashboard() {
            try {
                const response = await fetch('/api/dashboard-data');
                const data = await response.json();
                
                document.getElementById('content').innerHTML = `
                    <div class="grid">
                        <div class="card">
                            <h2>📊 System Health</h2>
                            <div class="status status-${data.health.status.toLowerCase()}">
                                Status: ${data.health.status}
                            </div>
                            <p>Uptime: ${data.health.uptime_seconds} seconds</p>
                            <p>Components: ${Object.keys(data.health.components).length} total</p>
                        </div>
                        
                        <div class="card">
                            <h2>💹 Trading Metrics</h2>
                            <p>Total Trades: ${data.metrics.total_trades}</p>
                            <p>Successful: ${data.metrics.successful_trades}</p>
                            <p>Failed: ${data.metrics.failed_trades}</p>
                            <p>Volume: ${data.metrics.total_volume_sol} SOL</p>
                        </div>
                        
                        <div class="card">
                            <h2>🔍 Telemetry</h2>
                            <p>Total Spans: ${data.telemetry.total_spans}</p>
                            <p>Active Spans: ${data.telemetry.active_spans}</p>
                            <p>Avg Duration: ${data.telemetry.average_duration_ms.toFixed(2)}ms</p>
                        </div>
                        
                        <div class="card">
                            <h2>🤖 Bot Performance</h2>
                            <p>Commands: ${data.metrics.total_commands}</p>
                            <p>API Calls: ${data.metrics.total_api_calls}</p>
                            <p>Cache Hit Rate: ${(data.metrics.cache_hit_rate * 100).toFixed(1)}%</p>
                            <p>Errors: ${data.metrics.total_errors}</p>
                        </div>
                    </div>
                    
                    <div class="footer">
                        <p>Last updated: ${new Date(data.timestamp).toLocaleString()} | Version: ${data.health.version}</p>
                    </div>
                `;
            } catch (error) {
                document.getElementById('content').innerHTML = `
                    <div class="loading">
                        <div>Error loading dashboard: ${error.message}</div>
                    </div>
                `;
            }
        }
        
        // Load dashboard data
        loadDashboard();
        
        // Auto-refresh every 30 seconds
        setInterval(loadDashboard, 30000);
    </script>
</body>
</html>
"#;