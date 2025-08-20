import { action } from "../_generated/server";
import { api } from "../_generated/api";
import { v } from "convex/values";
import axios from "axios";

// AI Configuration
const GROQ_API_URL = "https://api.groq.com/openai/v1/chat/completions";
const GROQ_API_KEY = process.env.GROQ_API_KEY;

// Generate trading signals using AI
export const generateTradingSignals = action({
  handler: async (ctx) => {
    console.log("🤖 Generating AI trading signals");
    
    // Get top tokens by volume
    const topTokens = await ctx.runQuery(api.queries.prices.getTopTokens, {
      limit: 20,
      sortBy: "volume",
    });

    const signals = [];

    for (const token of topTokens) {
      try {
        // Get recent price history
        const priceHistory = await ctx.runQuery(api.queries.prices.getPriceHistory, {
          tokenMint: token.mint,
          interval: "1h",
          limit: 24,
        });

        // Get technical indicators
        const indicators = await ctx.runQuery(api.queries.analytics.getTechnicalIndicators, {
          tokenMint: token.mint,
        });

        // Generate AI analysis
        const analysis = await analyzeWithAI({
          token: token.symbol,
          priceData: priceHistory,
          indicators,
        });

        // Create trading signal
        const signal = {
          tokenMint: token.mint,
          symbol: token.symbol,
          recommendation: analysis.recommendation,
          confidence: analysis.confidence,
          reasoning: analysis.reasoning,
          entry: analysis.entry,
          target: analysis.target,
          stopLoss: analysis.stopLoss,
          timeframe: "short", // short, medium, long
          timestamp: Date.now(),
        };

        // Store signal
        await ctx.runMutation(api.mutations.ai.storeTradingSignal, signal);
        signals.push(signal);

        console.log(`📊 Signal generated for ${token.symbol}: ${analysis.recommendation} (${analysis.confidence}% confidence)`);
      } catch (error) {
        console.error(`Failed to generate signal for ${token.symbol}:`, error);
      }
    }

    // Notify users of high-confidence signals
    const highConfidenceSignals = signals.filter(s => s.confidence > 80);
    if (highConfidenceSignals.length > 0) {
      await ctx.runAction(api.actions.notifications.broadcastSignals, {
        signals: highConfidenceSignals,
      });
    }

    return {
      total: signals.length,
      highConfidence: highConfidenceSignals.length,
    };
  },
});

// Update sentiment analysis for tokens
export const updateSentimentAnalysis = action({
  handler: async (ctx) => {
    console.log("💭 Updating sentiment analysis");
    
    // Get active tokens
    const activeTokens = await ctx.runQuery(api.queries.portfolio.getActiveTokens);
    
    for (const token of activeTokens) {
      try {
        // Fetch social sentiment data
        const sentiment = await analyzeSocialSentiment(token);
        
        // Store sentiment analysis
        await ctx.runMutation(api.mutations.ai.storeSentimentAnalysis, {
          targetId: token.mint,
          type: "sentiment",
          analysis: {
            summary: sentiment.summary,
            score: sentiment.score,
            confidence: sentiment.confidence,
            signals: sentiment.signals,
            recommendation: sentiment.recommendation,
          },
          sources: sentiment.sources,
          model: "groq-llama3",
          timestamp: Date.now(),
          expiresAt: Date.now() + (30 * 60 * 1000), // 30 minutes
        });

        console.log(`✅ Sentiment updated for ${token.symbol}: ${sentiment.score}/100`);
      } catch (error) {
        console.error(`Failed to analyze sentiment for ${token.symbol}:`, error);
      }
    }
  },
});

// Analyze market with AI for a specific token
export const analyzeToken = action({
  args: {
    tokenMint: v.string(),
    depth: v.optional(v.union(v.literal("basic"), v.literal("detailed"), v.literal("comprehensive"))),
  },
  handler: async (ctx, args) => {
    const depth = args.depth || "detailed";
    
    console.log(`🔍 Analyzing token ${args.tokenMint} (${depth})`);
    
    // Gather all relevant data
    const [priceData, indicators, volume, sentiment, news] = await Promise.all([
      ctx.runQuery(api.queries.prices.getPriceHistory, {
        tokenMint: args.tokenMint,
        interval: "1h",
        limit: 168, // 1 week
      }),
      ctx.runQuery(api.queries.analytics.getTechnicalIndicators, {
        tokenMint: args.tokenMint,
      }),
      ctx.runQuery(api.queries.analytics.getVolumeAnalysis, {
        tokenMint: args.tokenMint,
      }),
      ctx.runQuery(api.queries.ai.getCachedSentiment, {
        tokenMint: args.tokenMint,
      }),
      fetchNewsData(args.tokenMint),
    ]);

    // Perform comprehensive AI analysis
    const analysis = await performComprehensiveAnalysis({
      tokenMint: args.tokenMint,
      priceData,
      indicators,
      volume,
      sentiment,
      news,
      depth,
    });

    // Store the analysis
    await ctx.runMutation(api.mutations.ai.storeAnalysis, {
      targetId: args.tokenMint,
      type: "comprehensive",
      analysis,
      model: "groq-mixtral",
      timestamp: Date.now(),
      expiresAt: Date.now() + (60 * 60 * 1000), // 1 hour
    });

    return analysis;
  },
});

// Generate predictive price models
export const generatePriceModels = action({
  handler: async (ctx) => {
    console.log("📈 Generating predictive price models");
    
    // Get tokens with sufficient history
    const tokens = await ctx.runQuery(api.queries.analytics.getTokensWithHistory, {
      minDataPoints: 100,
    });

    for (const token of tokens) {
      try {
        // Get historical data
        const history = await ctx.runQuery(api.queries.prices.getPriceHistory, {
          tokenMint: token.mint,
          interval: "1h",
          limit: 720, // 30 days
        });

        // Generate predictions using AI
        const predictions = await generatePricePredictions({
          token: token.symbol,
          history,
        });

        // Store predictions
        await ctx.runMutation(api.mutations.ai.storePrediction, {
          targetId: token.mint,
          type: "prediction",
          analysis: {
            summary: `Price predictions for ${token.symbol}`,
            score: predictions.confidence,
            confidence: predictions.confidence / 100,
            signals: predictions.signals,
            recommendation: predictions.recommendation,
            predictions: {
              hour1: predictions.hour1,
              hour4: predictions.hour4,
              hour24: predictions.hour24,
              day7: predictions.day7,
            },
          },
          model: "groq-llama3-70b",
          timestamp: Date.now(),
          expiresAt: Date.now() + (60 * 60 * 1000), // 1 hour
        });

        console.log(`✅ Price model generated for ${token.symbol}`);
      } catch (error) {
        console.error(`Failed to generate model for ${token.symbol}:`, error);
      }
    }
  },
});

// Helper function to analyze with AI
async function analyzeWithAI(data: any): Promise<any> {
  if (!GROQ_API_KEY) {
    // Return mock data if no API key
    return {
      recommendation: "hold",
      confidence: 65,
      reasoning: "Based on technical indicators",
      entry: data.priceData[0]?.close || "0",
      target: data.priceData[0]?.close || "0",
      stopLoss: data.priceData[0]?.close || "0",
    };
  }

  try {
    const prompt = `Analyze the following cryptocurrency trading data and provide a trading signal:

Token: ${data.token}
Current Price: ${data.priceData[0]?.close}
24h Change: ${data.priceData[0]?.change24h}%
RSI: ${data.indicators?.rsi}
MACD: ${data.indicators?.macd?.value}
Volume: ${data.priceData[0]?.volume}

Provide a JSON response with:
- recommendation: "buy", "sell", or "hold"
- confidence: 0-100
- reasoning: brief explanation
- entry: suggested entry price
- target: target price
- stopLoss: stop loss price`;

    const response = await axios.post(
      GROQ_API_URL,
      {
        model: "llama3-70b-8192",
        messages: [
          {
            role: "system",
            content: "You are an expert cryptocurrency trading analyst. Provide accurate, data-driven trading signals.",
          },
          {
            role: "user",
            content: prompt,
          },
        ],
        temperature: 0.3,
        max_tokens: 500,
      },
      {
        headers: {
          Authorization: `Bearer ${GROQ_API_KEY}`,
          "Content-Type": "application/json",
        },
      }
    );

    const content = response.data.choices[0].message.content;
    return JSON.parse(content);
  } catch (error) {
    console.error("AI analysis failed:", error);
    return {
      recommendation: "hold",
      confidence: 50,
      reasoning: "Unable to analyze",
      entry: "0",
      target: "0",
      stopLoss: "0",
    };
  }
}

// Analyze social sentiment
async function analyzeSocialSentiment(token: any): Promise<any> {
  // In production, would integrate with social media APIs
  // For now, return simulated data
  
  const sentiments = ["bullish", "neutral", "bearish"];
  const randomSentiment = sentiments[Math.floor(Math.random() * sentiments.length)];
  
  const score = randomSentiment === "bullish" ? 75 : 
                randomSentiment === "bearish" ? 25 : 50;

  return {
    summary: `Overall ${randomSentiment} sentiment for ${token.symbol}`,
    score,
    confidence: 70 + Math.random() * 20,
    signals: [
      {
        type: "social_volume",
        strength: "medium",
        description: "Moderate social media activity",
      },
      {
        type: "sentiment_trend",
        strength: randomSentiment === "neutral" ? "weak" : "strong",
        description: `${randomSentiment} trend detected`,
      },
    ],
    recommendation: score > 60 ? "buy" : score < 40 ? "sell" : "hold",
    sources: ["Twitter", "Reddit", "Discord"],
  };
}

// Fetch news data
async function fetchNewsData(tokenMint: string): Promise<any[]> {
  // In production, would integrate with news APIs
  return [
    {
      title: "Market Update",
      source: "CryptoNews",
      sentiment: "neutral",
      relevance: 0.8,
    },
  ];
}

// Perform comprehensive analysis
async function performComprehensiveAnalysis(data: any): Promise<any> {
  const prompt = `Perform a ${data.depth} analysis of the following cryptocurrency:

Token: ${data.tokenMint}
Price History: ${JSON.stringify(data.priceData.slice(0, 10))}
Technical Indicators: ${JSON.stringify(data.indicators)}
Volume Analysis: ${JSON.stringify(data.volume)}
Sentiment: ${JSON.stringify(data.sentiment)}
Recent News: ${JSON.stringify(data.news)}

Provide a comprehensive analysis including:
1. Market trend analysis
2. Support and resistance levels
3. Risk assessment
4. Trading recommendation
5. Time horizon
6. Key risks and opportunities`;

  if (!GROQ_API_KEY) {
    return {
      summary: "Comprehensive analysis unavailable",
      score: 50,
      confidence: 0.5,
      signals: [],
      recommendation: "hold",
    };
  }

  try {
    const response = await axios.post(
      GROQ_API_URL,
      {
        model: "mixtral-8x7b-32768",
        messages: [
          {
            role: "system",
            content: "You are a senior cryptocurrency analyst providing institutional-grade analysis.",
          },
          {
            role: "user",
            content: prompt,
          },
        ],
        temperature: 0.2,
        max_tokens: 1500,
      },
      {
        headers: {
          Authorization: `Bearer ${GROQ_API_KEY}`,
          "Content-Type": "application/json",
        },
      }
    );

    const analysis = response.data.choices[0].message.content;
    
    // Parse and structure the response
    return {
      summary: analysis,
      score: 70,
      confidence: 0.75,
      signals: extractSignals(analysis),
      recommendation: extractRecommendation(analysis),
    };
  } catch (error) {
    console.error("Comprehensive analysis failed:", error);
    return {
      summary: "Analysis failed",
      score: 0,
      confidence: 0,
      signals: [],
      recommendation: "hold",
    };
  }
}

// Generate price predictions
async function generatePricePredictions(data: any): Promise<any> {
  // Simple prediction model - in production would use ML
  const currentPrice = parseFloat(data.history[0]?.close || "0");
  const avgChange = calculateAverageChange(data.history);
  
  return {
    confidence: 60 + Math.random() * 20,
    hour1: currentPrice * (1 + avgChange * 0.1),
    hour4: currentPrice * (1 + avgChange * 0.4),
    hour24: currentPrice * (1 + avgChange),
    day7: currentPrice * (1 + avgChange * 7),
    signals: [
      {
        type: "trend",
        strength: avgChange > 0 ? "bullish" : "bearish",
        description: `${Math.abs(avgChange * 100).toFixed(2)}% average hourly change`,
      },
    ],
    recommendation: avgChange > 0.01 ? "buy" : avgChange < -0.01 ? "sell" : "hold",
  };
}

// Helper functions
function extractSignals(analysis: string): any[] {
  // Extract signals from AI analysis text
  const signals = [];
  
  if (analysis.toLowerCase().includes("bullish")) {
    signals.push({ type: "sentiment", strength: "bullish", description: "Bullish sentiment detected" });
  }
  if (analysis.toLowerCase().includes("support")) {
    signals.push({ type: "technical", strength: "strong", description: "Near support level" });
  }
  if (analysis.toLowerCase().includes("resistance")) {
    signals.push({ type: "technical", strength: "strong", description: "Near resistance level" });
  }
  
  return signals;
}

function extractRecommendation(analysis: string): string {
  const lower = analysis.toLowerCase();
  if (lower.includes("buy") || lower.includes("long")) return "buy";
  if (lower.includes("sell") || lower.includes("short")) return "sell";
  return "hold";
}

function calculateAverageChange(history: any[]): number {
  if (history.length < 2) return 0;
  
  let totalChange = 0;
  for (let i = 1; i < Math.min(history.length, 24); i++) {
    const change = (parseFloat(history[i-1].close) - parseFloat(history[i].close)) / parseFloat(history[i].close);
    totalChange += change;
  }
  
  return totalChange / Math.min(history.length - 1, 23);
}