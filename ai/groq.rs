use crate::errors::{BotError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub summary: String,
    pub signal: String,
    pub confidence: f64,
    pub key_factors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroqRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: u32,
    top_p: f64,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroqResponse {
    id: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: Message,
    finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

pub struct GroqAnalyzer {
    api_key: String,
    client: Client,
}

impl GroqAnalyzer {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
    
    pub async fn analyze_token(&self, token: &str) -> Result<MarketAnalysis> {
        let system_prompt = r#"You are a cryptocurrency market analyst specializing in Solana tokens.
        Analyze tokens based on available market data and sentiment.
        Provide clear, actionable insights.
        Response format: Summary|Signal|Confidence|Factor1,Factor2,Factor3"#;
        
        let user_prompt = format!(
            "Analyze {} token for trading. Consider market trends, volume, and sentiment. \
            Provide: 1) Market summary (50 words), 2) Trading signal (BUY/HOLD/SELL), \
            3) Confidence percentage (0-100), 4) Three key factors affecting the token. \
            Format response as: Summary|Signal|Confidence|Factor1,Factor2,Factor3",
            token
        );
        
        debug!("Analyzing token: {}", token);
        
        let request = GroqRequest {
            model: "llama-3.1-70b-instruct".to_string(), // Updated to Llama 3.1
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: 200,
            top_p: 0.9,
            stream: false,
        };
        
        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(BotError::external_api(format!("Groq API error: {}", error_text)));
        }
        
        let groq_response: GroqResponse = response.json().await?;
        
        let content = groq_response.choices
            .first()
            .map(|c| &c.message.content)
            .ok_or_else(|| BotError::external_api("No response from Groq"))?;
        
        let analysis = self.parse_analysis(content)?;
        
        info!(
            "Analysis complete for {}: Signal={}, Confidence={}%",
            token, analysis.signal, analysis.confidence
        );
        
        Ok(analysis)
    }
    
    pub async fn analyze_market_conditions(&self) -> Result<String> {
        let request = GroqRequest {
            model: "llama-3.1-70b-instruct".to_string(), // Updated to Llama 3.1
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a crypto market analyst. Provide brief market updates.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "Provide a brief Solana market update in 50 words.".to_string(),
                },
            ],
            temperature: 0.5,
            max_tokens: 100,
            top_p: 0.9,
            stream: false,
        };
        
        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        
        let groq_response: GroqResponse = response.json().await?;
        
        Ok(groq_response.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "Market conditions normal.".to_string()))
    }
    
    fn parse_analysis(&self, content: &str) -> Result<MarketAnalysis> {
        let parts: Vec<&str> = content.split('|').collect();
        
        if parts.len() < 4 {
            return Ok(MarketAnalysis {
                summary: content.to_string(),
                signal: "HOLD".to_string(),
                confidence: 50.0,
                key_factors: vec!["Market volatility".to_string()],
            });
        }
        
        let summary = parts[0].trim().to_string();
        let signal = parts[1].trim().to_uppercase();
        let confidence = parts[2]
            .trim()
            .replace('%', "")
            .parse::<f64>()
            .unwrap_or(50.0)
            .clamp(0.0, 100.0);
        
        let factors: Vec<String> = parts.get(3)
            .map(|f| f.split(',')
                .map(|s| s.trim().to_string())
                .collect())
            .unwrap_or_else(|| vec!["Market conditions".to_string()]);
        
        let valid_signal = match signal.as_str() {
            "BUY" | "SELL" | "HOLD" => signal,
            _ => "HOLD".to_string(),
        };
        
        Ok(MarketAnalysis {
            summary,
            signal: valid_signal,
            confidence,
            key_factors: factors,
        })
    }
}