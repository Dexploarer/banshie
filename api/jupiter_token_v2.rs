use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::{DateTime, Utc, Duration};

use crate::errors::{BotError, Result};
use crate::api::jupiter_auth::{JupiterAuthManager, ApiTierLevel};

/// Jupiter Token API V2 client with enhanced token analytics
#[derive(Clone)]
pub struct JupiterTokenV2Client {
    client: Client,
    auth_manager: Arc<JupiterAuthManager>,
    base_url: String,
    token_cache: Arc<RwLock<TokenCache>>,
}

/// Enhanced token data with V2 features and organic scoring
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenDataV2 {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    pub tags: Vec<String>,
    
    // V2 Enhanced features
    #[serde(rename = "organicScore")]
    pub organic_score: Option<f64>,
    #[serde(rename = "socialScore")]
    pub social_score: Option<f64>,
    #[serde(rename = "liquidityScore")]
    pub liquidity_score: Option<f64>,
    #[serde(rename = "communityScore")]
    pub community_score: Option<f64>,
    
    // Verification and trust metrics
    pub verified: bool,
    #[serde(rename = "strictList")]
    pub strict_list: Option<bool>,
    #[serde(rename = "communityValidated")]
    pub community_validated: Option<bool>,
    
    // Trading data
    #[serde(rename = "dailyVolume")]
    pub daily_volume: Option<u64>,
    #[serde(rename = "weeklyVolume")]
    pub weekly_volume: Option<u64>,
    #[serde(rename = "monthlyVolume")]
    pub monthly_volume: Option<u64>,
    #[serde(rename = "marketCap")]
    pub market_cap: Option<u64>,
    #[serde(rename = "fullyDilutedMarketCap")]
    pub fully_diluted_market_cap: Option<u64>,
    
    // Token authority information
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: Option<String>,
    #[serde(rename = "mintAuthority")]
    pub mint_authority: Option<String>,
    #[serde(rename = "updateAuthority")]
    pub update_authority: Option<String>,
    
    // Supply information
    #[serde(rename = "totalSupply")]
    pub total_supply: Option<u64>,
    #[serde(rename = "circulatingSupply")]
    pub circulating_supply: Option<u64>,
    
    // Social and external links
    #[serde(rename = "extensions")]
    pub extensions: Option<TokenExtensions>,
    
    // Risk assessment
    #[serde(rename = "riskLevel")]
    pub risk_level: Option<RiskLevel>,
    #[serde(rename = "riskFactors")]
    pub risk_factors: Option<Vec<RiskFactor>>,
    
    // Metadata
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<DateTime<Utc>>,
}

/// Token extensions with social links and metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenExtensions {
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub discord: Option<String>,
    pub telegram: Option<String>,
    pub github: Option<String>,
    pub medium: Option<String>,
    pub coinmarketcap: Option<String>,
    pub coingecko: Option<String>,
    pub description: Option<String>,
}

/// Risk assessment levels
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// Specific risk factors
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskFactor {
    HighConcentration,
    RecentlyCreated,
    LowLiquidity,
    NoWebsite,
    UnverifiedToken,
    MintAuthority,
    FreezeAuthority,
    HighVolatility,
    LowTradingVolume,
    PumpAndDump,
    RugPullRisk,
}

/// Token list response with enhanced filtering
#[derive(Debug, Deserialize)]
pub struct TokenListResponse {
    pub tokens: Vec<TokenDataV2>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
    pub total_count: Option<u64>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Token search request with advanced filters
#[derive(Debug, Serialize)]
pub struct TokenSearchRequest {
    pub query: Option<String>,           // Search by name, symbol, or address
    pub tags: Option<Vec<String>>,       // Filter by tags
    pub verified_only: Option<bool>,     // Only verified tokens
    pub min_daily_volume: Option<u64>,   // Minimum daily volume
    pub min_liquidity: Option<u64>,      // Minimum liquidity
    pub min_organic_score: Option<f64>,  // Minimum organic score (0.0-1.0)
    pub risk_level: Option<RiskLevel>,   // Maximum risk level
    pub sort_by: Option<SortBy>,         // Sorting criteria
    pub order: Option<SortOrder>,        // Sort order
    pub page: Option<u32>,               // Page number (1-based)
    pub page_size: Option<u32>,          // Results per page (max 1000)
}

/// Sorting options for token search
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    Name,
    Symbol,
    DailyVolume,
    MarketCap,
    OrganicScore,
    SocialScore,
    LiquidityScore,
    CreatedAt,
    LastUpdated,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Token analytics with comprehensive metrics
#[derive(Debug, Clone)]
pub struct TokenAnalytics {
    pub token_address: String,
    pub basic_info: TokenDataV2,
    pub trading_metrics: TradingMetrics,
    pub social_metrics: SocialMetrics,
    pub risk_assessment: RiskAssessment,
    pub price_performance: PricePerformance,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TradingMetrics {
    pub volume_24h: f64,
    pub volume_7d: f64,
    pub volume_30d: f64,
    pub unique_traders_24h: Option<u64>,
    pub transactions_24h: Option<u64>,
    pub avg_trade_size: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub market_cap_rank: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SocialMetrics {
    pub twitter_followers: Option<u64>,
    pub discord_members: Option<u64>,
    pub telegram_members: Option<u64>,
    pub github_stars: Option<u64>,
    pub social_sentiment: Option<f64>, // -1.0 to 1.0
    pub mention_frequency: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub overall_risk_score: f64, // 0.0 to 1.0 (higher = riskier)
    pub liquidity_risk: f64,
    pub centralization_risk: f64,
    pub smart_contract_risk: f64,
    pub market_risk: f64,
    pub regulatory_risk: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PricePerformance {
    pub price_change_1h: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub price_change_7d: Option<f64>,
    pub price_change_30d: Option<f64>,
    pub volatility_30d: Option<f64>,
    pub all_time_high: Option<f64>,
    pub all_time_low: Option<f64>,
    pub ath_change_percent: Option<f64>,
}

/// Token watchlist for tracking favorite tokens
#[derive(Debug, Clone, Serialize)]
pub struct TokenWatchlist {
    pub user_id: i64,
    pub tokens: Vec<WatchlistToken>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchlistToken {
    pub address: String,
    pub symbol: String,
    pub added_at: DateTime<Utc>,
    pub alert_price_above: Option<f64>,
    pub alert_price_below: Option<f64>,
    pub notes: Option<String>,
}

/// Token cache with intelligent invalidation
#[derive(Debug)]
struct TokenCache {
    tokens: HashMap<String, CachedToken>,
    token_list: Option<CachedTokenList>,
    last_cleanup: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    data: TokenDataV2,
    cached_at: DateTime<Utc>,
    access_count: u32,
}

#[derive(Debug, Clone)]
struct CachedTokenList {
    tokens: Vec<TokenDataV2>,
    cached_at: DateTime<Utc>,
    cache_key: String,
}

impl JupiterTokenV2Client {
    /// Create new Token API V2 client
    pub fn new(auth_manager: Arc<JupiterAuthManager>) -> Self {
        info!("🪙 Initializing Jupiter Token API V2 client");
        
        Self {
            client: Client::new(),
            auth_manager,
            base_url: "https://api.jup.ag".to_string(),
            token_cache: Arc::new(RwLock::new(TokenCache {
                tokens: HashMap::new(),
                token_list: None,
                last_cleanup: Utc::now(),
            })),
        }
    }
    
    /// Get all tokens with optional filtering
    pub async fn get_tokens(&self, search: Option<TokenSearchRequest>) -> Result<TokenListResponse> {
        // Check cache for token list
        if let Some(cached_list) = self.check_token_list_cache(&search).await {
            return Ok(cached_list);
        }
        
        let api_key_config = self.auth_manager.select_best_key("tokens").await?;
        let base_url = match &api_key_config {
            Some(config) => match config.tier {
                ApiTierLevel::Lite => "https://lite-api.jup.ag".to_string(),
                _ => "https://api.jup.ag".to_string(),
            },
            None => "https://lite-api.jup.ag".to_string(),
        };
        
        let url = format!("{}/token/v2/tokens", base_url);
        
        let mut request = self.client.get(&url);
        
        // Add search parameters if provided
        if let Some(search_params) = &search {
            if let Some(query) = &search_params.query {
                request = request.query(&[("q", query)]);
            }
            if let Some(tags) = &search_params.tags {
                request = request.query(&[("tags", &tags.join(","))]);
            }
            if let Some(verified) = search_params.verified_only {
                request = request.query(&[("verified", &verified.to_string())]);
            }
            if let Some(min_volume) = search_params.min_daily_volume {
                request = request.query(&[("minDailyVolume", &min_volume.to_string())]);
            }
            if let Some(min_score) = search_params.min_organic_score {
                request = request.query(&[("minOrganicScore", &min_score.to_string())]);
            }
            if let Some(sort_by) = &search_params.sort_by {
                request = request.query(&[("sortBy", &serde_json::to_string(sort_by).unwrap_or_default())]);
            }
            if let Some(order) = &search_params.order {
                request = request.query(&[("order", &serde_json::to_string(order).unwrap_or_default())]);
            }
            if let Some(page) = search_params.page {
                request = request.query(&[("page", &page.to_string())]);
            }
            if let Some(page_size) = search_params.page_size {
                request = request.query(&[("pageSize", &page_size.to_string())]);
            }
        }
        
        // Add authentication if available
        if let Some(config) = &api_key_config {
            request = request.header("Authorization", format!("Bearer {}", config.key));
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Token request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Token API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let token_response: TokenListResponse = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse token response: {}", e)))?;
            
        // Cache the results
        self.cache_token_list(&token_response, &search).await;
        
        // Record usage
        if let Some(config) = &api_key_config {
            let key_id = format!("key_{}", &config.key[..8]);
            self.auth_manager.record_usage(&key_id, "tokens").await;
        }
        
        info!("🪙 Retrieved {} tokens from Jupiter Token API V2", token_response.tokens.len());
        
        Ok(token_response)
    }
    
    /// Get detailed information about a specific token
    pub async fn get_token(&self, token_address: &str) -> Result<TokenDataV2> {
        // Check cache first
        if let Some(cached_token) = self.check_token_cache(token_address).await {
            return Ok(cached_token);
        }
        
        let api_key_config = self.auth_manager.select_best_key("token_detail").await?;
        let base_url = match &api_key_config {
            Some(config) => match config.tier {
                ApiTierLevel::Lite => "https://lite-api.jup.ag".to_string(),
                _ => "https://api.jup.ag".to_string(),
            },
            None => "https://lite-api.jup.ag".to_string(),
        };
        
        let url = format!("{}/token/v2/token/{}", base_url, token_address);
        
        let mut request = self.client.get(&url);
        
        // Add authentication if available
        if let Some(config) = &api_key_config {
            request = request.header("Authorization", format!("Bearer {}", config.key));
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Token detail request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Token detail API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let token_data: TokenDataV2 = response
            .json()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse token detail response: {}", e)))?;
            
        // Cache the result
        self.cache_token(&token_data).await;
        
        // Record usage
        if let Some(config) = &api_key_config {
            let key_id = format!("key_{}", &config.key[..8]);
            self.auth_manager.record_usage(&key_id, "token_detail").await;
        }
        
        debug!("🪙 Retrieved detailed info for token {}", token_address);
        
        Ok(token_data)
    }
    
    /// Get tokens filtered by organic score for trading
    pub async fn get_top_organic_tokens(&self, limit: Option<u32>) -> Result<Vec<TokenDataV2>> {
        let search_request = TokenSearchRequest {
            query: None,
            tags: Some(vec!["toporganicscore".to_string()]),
            verified_only: Some(true),
            min_daily_volume: Some(1000), // At least $1k daily volume
            min_liquidity: Some(10000),   // At least $10k liquidity
            min_organic_score: Some(0.7), // High organic score
            risk_level: Some(RiskLevel::Medium), // Max medium risk
            sort_by: Some(SortBy::OrganicScore),
            order: Some(SortOrder::Desc),
            page: Some(1),
            page_size: limit,
        };
        
        let response = self.get_tokens(Some(search_request)).await?;
        Ok(response.tokens)
    }
    
    /// Analyze token for comprehensive metrics
    pub async fn analyze_token(&self, token_address: &str) -> Result<TokenAnalytics> {
        let token_data = self.get_token(token_address).await?;
        
        // Calculate comprehensive analytics
        let trading_metrics = TradingMetrics {
            volume_24h: token_data.daily_volume.unwrap_or(0) as f64,
            volume_7d: token_data.weekly_volume.unwrap_or(0) as f64,
            volume_30d: token_data.monthly_volume.unwrap_or(0) as f64,
            unique_traders_24h: None, // Would need additional API calls
            transactions_24h: None,
            avg_trade_size: None,
            liquidity_usd: None,
            market_cap_rank: None,
        };
        
        let social_metrics = SocialMetrics {
            twitter_followers: None, // Would need social media APIs
            discord_members: None,
            telegram_members: None,
            github_stars: None,
            social_sentiment: token_data.social_score,
            mention_frequency: None,
        };
        
        let risk_assessment = self.calculate_risk_assessment(&token_data);
        
        let price_performance = PricePerformance {
            price_change_1h: None,
            price_change_24h: None,
            price_change_7d: None,
            price_change_30d: None,
            volatility_30d: None,
            all_time_high: None,
            all_time_low: None,
            ath_change_percent: None,
        };
        
        Ok(TokenAnalytics {
            token_address: token_address.to_string(),
            basic_info: token_data,
            trading_metrics,
            social_metrics,
            risk_assessment,
            price_performance,
            calculated_at: Utc::now(),
        })
    }
    
    /// Calculate comprehensive risk assessment
    fn calculate_risk_assessment(&self, token_data: &TokenDataV2) -> RiskAssessment {
        let mut overall_risk = 0.0;
        let mut recommendations = Vec::new();
        
        // Liquidity risk (30% weight)
        let liquidity_risk = if token_data.daily_volume.unwrap_or(0) < 1000 {
            recommendations.push("Low trading volume may impact liquidity".to_string());
            0.8
        } else if token_data.daily_volume.unwrap_or(0) < 10000 {
            0.5
        } else {
            0.2
        };
        
        // Centralization risk (25% weight)
        let centralization_risk = match (&token_data.mint_authority, &token_data.freeze_authority) {
            (Some(_), Some(_)) => {
                recommendations.push("Token has both mint and freeze authority".to_string());
                0.9
            },
            (Some(_), None) => {
                recommendations.push("Token has mint authority".to_string());
                0.7
            },
            (None, Some(_)) => {
                recommendations.push("Token has freeze authority".to_string());
                0.6
            },
            (None, None) => 0.1,
        };
        
        // Smart contract risk (20% weight)
        let smart_contract_risk = if token_data.verified {
            0.2
        } else {
            recommendations.push("Token is not verified".to_string());
            0.7
        };
        
        // Market risk (15% weight)
        let market_risk = 1.0 - token_data.organic_score.unwrap_or(0.5);
        
        // Regulatory risk (10% weight)
        let regulatory_risk = if token_data.tags.contains(&"security".to_string()) {
            recommendations.push("Token may be classified as a security".to_string());
            0.8
        } else {
            0.3
        };
        
        overall_risk = (liquidity_risk * 0.3) +
                      (centralization_risk * 0.25) +
                      (smart_contract_risk * 0.2) +
                      (market_risk * 0.15) +
                      (regulatory_risk * 0.1);
        
        RiskAssessment {
            overall_risk_score: overall_risk,
            liquidity_risk,
            centralization_risk,
            smart_contract_risk,
            market_risk,
            regulatory_risk,
            recommendations,
        }
    }
    
    /// Check token cache
    async fn check_token_cache(&self, token_address: &str) -> Option<TokenDataV2> {
        let mut cache = self.token_cache.write().await;
        
        if let Some(cached) = cache.tokens.get_mut(token_address) {
            let age = Utc::now().signed_duration_since(cached.cached_at);
            
            if age < Duration::minutes(5) { // 5 minute cache
                cached.access_count += 1;
                return Some(cached.data.clone());
            }
        }
        
        None
    }
    
    /// Check token list cache
    async fn check_token_list_cache(&self, search: &Option<TokenSearchRequest>) -> Option<TokenListResponse> {
        let cache = self.token_cache.read().await;
        
        if let Some(cached_list) = &cache.token_list {
            let age = Utc::now().signed_duration_since(cached_list.cached_at);
            let cache_key = self.generate_search_cache_key(search);
            
            if age < Duration::minutes(10) && cached_list.cache_key == cache_key {
                return Some(TokenListResponse {
                    tokens: cached_list.tokens.clone(),
                    time_taken: None,
                    total_count: Some(cached_list.tokens.len() as u64),
                    page: None,
                    page_size: None,
                });
            }
        }
        
        None
    }
    
    /// Cache individual token
    async fn cache_token(&self, token_data: &TokenDataV2) {
        let mut cache = self.token_cache.write().await;
        
        cache.tokens.insert(token_data.address.clone(), CachedToken {
            data: token_data.clone(),
            cached_at: Utc::now(),
            access_count: 1,
        });
    }
    
    /// Cache token list
    async fn cache_token_list(&self, response: &TokenListResponse, search: &Option<TokenSearchRequest>) {
        let mut cache = self.token_cache.write().await;
        let cache_key = self.generate_search_cache_key(search);
        
        cache.token_list = Some(CachedTokenList {
            tokens: response.tokens.clone(),
            cached_at: Utc::now(),
            cache_key,
        });
    }
    
    /// Generate cache key for search parameters
    fn generate_search_cache_key(&self, search: &Option<TokenSearchRequest>) -> String {
        match search {
            Some(s) => format!("{:?}", s),
            None => "default".to_string(),
        }
    }
}