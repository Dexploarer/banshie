import { action } from "../_generated/server";
import { api } from "../_generated/api";
import { v } from "convex/values";

const OPENAI_API_KEY = process.env.OPENAI_API_KEY!;
const GROQ_API_KEY = process.env.GROQ_API_KEY!;

interface OpenAIResponse {
  choices: Array<{
    message: {
      content: string;
      role: string;
    };
    finish_reason: string;
  }>;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

// Enhanced sentiment analysis with multiple data sources
export const analyzeSentiment = action({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    sources: v.optional(v.array(v.string())), // ["twitter", "reddit", "news", "onchain"]
    timeframe: v.optional(v.string()), // "1h", "24h", "7d"
  },
  handler: async (ctx, args) => {
    const sources = args.sources || ["twitter", "reddit", "news"];
    const timeframe = args.timeframe || "24h";
    
    console.log(`🧠 Analyzing sentiment for ${args.symbol} across ${sources.join(", ")}`);
    
    try {
      // Gather data from multiple sources
      const sentimentData = await gatherSentimentData(args.symbol, sources, timeframe);
      
      // Generate embedding for the sentiment analysis
      const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
        text: `${args.symbol} sentiment analysis: ${sentimentData.summary}`,
      });
      
      // Analyze sentiment using AI
      const analysis = await analyzeSentimentWithAI(args.symbol, sentimentData);
      
      // Store the analysis with embedding
      const analysisId = await ctx.runMutation(api.mutations.ai.storeAIAnalysis, {
        targetId: args.tokenMint,
        type: "sentiment",
        analysis: {
          summary: analysis.summary,
          score: analysis.sentiment_score,
          confidence: analysis.confidence,
          signals: analysis.signals.map((signal: any) => ({
            type: signal.type,
            strength: signal.strength,
            description: signal.description,
          })),
          recommendation: analysis.recommendation as any,
        },
        embedding: embeddingResult.embedding,
        sources: sources,
        model: "gpt-4-turbo-preview",
        expiresAt: Date.now() + (4 * 60 * 60 * 1000), // 4 hours
      });

      console.log(`✅ Sentiment analysis completed for ${args.symbol}`);

      return {
        analysisId,
        sentiment: analysis,
        sources: sentimentData.sources,
        dataPoints: sentimentData.totalDataPoints,
        timeframe,
      };
    } catch (error) {
      console.error(`❌ Error in sentiment analysis for ${args.symbol}:`, error);
      throw new Error(`Sentiment analysis failed: ${error}`);
    }
  },
});

// Create predictive price models using multiple factors
export const createPriceModel = action({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    timeHorizon: v.optional(v.string()), // "1d", "7d", "30d"
    modelType: v.optional(v.string()), // "technical", "fundamental", "hybrid"
  },
  handler: async (ctx, args) => {
    const timeHorizon = args.timeHorizon || "7d";
    const modelType = args.modelType || "hybrid";
    
    console.log(`📈 Creating price model for ${args.symbol} (${timeHorizon}, ${modelType})`);
    
    try {
      // Gather comprehensive data
      const modelData = await gatherModelData(ctx, args.tokenMint, args.symbol, timeHorizon);
      
      // Generate price prediction
      const prediction = await generatePricePrediction(args.symbol, modelData, timeHorizon, modelType);
      
      // Create embedding for the prediction
      const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
        text: `${args.symbol} price prediction: ${prediction.reasoning}`,
      });
      
      // Store the prediction
      const analysisId = await ctx.runMutation(api.mutations.ai.storeAIAnalysis, {
        targetId: args.tokenMint,
        type: "prediction",
        analysis: {
          summary: prediction.summary,
          score: prediction.confidence_score,
          confidence: prediction.confidence / 100,
          signals: prediction.key_factors.map((factor: string) => ({
            type: "prediction_factor",
            strength: "medium",
            description: factor,
          })),
          recommendation: prediction.recommendation as any,
        },
        embedding: embeddingResult.embedding,
        sources: ["technical_analysis", "market_data", "sentiment", "onchain"],
        model: "gpt-4-turbo-preview",
        expiresAt: Date.now() + getPredictionExpiry(timeHorizon),
      });

      console.log(`✅ Price model created for ${args.symbol}`);

      return {
        analysisId,
        prediction: {
          currentPrice: prediction.current_price,
          targetPrice: prediction.target_price,
          priceChange: prediction.price_change_percent,
          timeHorizon: timeHorizon,
          confidence: prediction.confidence,
          reasoning: prediction.reasoning,
          keyFactors: prediction.key_factors,
          risks: prediction.risks,
          catalysts: prediction.catalysts,
        },
        modelType,
        dataQuality: modelData.quality,
      };
    } catch (error) {
      console.error(`❌ Error creating price model for ${args.symbol}:`, error);
      throw new Error(`Price model creation failed: ${error}`);
    }
  },
});

// Generate automated trading signals with confidence scoring
export const generateTradingSignal = action({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    signalType: v.optional(v.string()), // "momentum", "reversal", "breakout"
    riskTolerance: v.optional(v.string()), // "low", "medium", "high"
  },
  handler: async (ctx, args) => {
    const signalType = args.signalType || "hybrid";
    const riskTolerance = args.riskTolerance || "medium";
    
    console.log(`🎯 Generating trading signal for ${args.symbol} (${signalType}, ${riskTolerance} risk)`);
    
    try {
      // Get comprehensive analysis data
      const signalData = await gatherSignalData(ctx, args.tokenMint, args.symbol);
      
      // Generate trading signal using AI
      const signal = await generateSignalWithAI(args.symbol, signalData, signalType, riskTolerance);
      
      // Create embedding for signal similarity search
      const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
        text: `${args.symbol} ${signal.action} signal: ${signal.reasoning}`,
      });
      
      // Store the trading signal
      const signalId = await ctx.runMutation(api.mutations.ai.storeTradingSignal, {
        tokenMint: args.tokenMint,
        symbol: args.symbol,
        signalType: signal.signal_type,
        action: signal.action,
        strength: signal.strength,
        confidence: signal.confidence,
        reasoning: signal.reasoning,
        technicalFactors: signal.technical_factors,
        fundamentalFactors: signal.fundamental_factors,
        sentimentFactors: signal.sentiment_factors,
        priceTarget: signal.price_target,
        stopLoss: signal.stop_loss,
        timeframe: signal.timeframe,
        riskLevel: signal.risk_level,
        embedding: embeddingResult.embedding,
        validUntil: Date.now() + getSignalValidDuration(signal.timeframe),
      });

      console.log(`✅ Trading signal generated for ${args.symbol}: ${signal.action} (${signal.confidence}% confidence)`);

      return {
        signalId,
        signal: {
          action: signal.action,
          strength: signal.strength,
          confidence: signal.confidence,
          reasoning: signal.reasoning,
          priceTarget: signal.price_target,
          stopLoss: signal.stop_loss,
          timeframe: signal.timeframe,
          riskLevel: signal.risk_level,
          technicalFactors: signal.technical_factors,
          fundamentalFactors: signal.fundamental_factors,
          sentimentFactors: signal.sentiment_factors,
        },
        dataQuality: signalData.quality,
        riskAssessment: signal.risk_assessment,
      };
    } catch (error) {
      console.error(`❌ Error generating trading signal for ${args.symbol}:`, error);
      throw new Error(`Signal generation failed: ${error}`);
    }
  },
});

// Comprehensive market analysis combining all AI features
export const comprehensiveAnalysis = action({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    analysisDepth: v.optional(v.string()), // "basic", "standard", "deep"
  },
  handler: async (ctx, args) => {
    const depth = args.analysisDepth || "standard";
    
    console.log(`🔬 Running comprehensive analysis for ${args.symbol} (${depth})`);
    
    try {
      const startTime = Date.now();
      
      // Run all analysis types in parallel
      const analysisPromises = [
        ctx.runAction(api.actions.ai_enhanced.analyzeSentiment, {
          tokenMint: args.tokenMint,
          symbol: args.symbol,
        }),
        ctx.runAction(api.actions.ai_enhanced.createPriceModel, {
          tokenMint: args.tokenMint,
          symbol: args.symbol,
          timeHorizon: depth === "deep" ? "30d" : "7d",
        }),
        ctx.runAction(api.actions.ai_enhanced.generateTradingSignal, {
          tokenMint: args.tokenMint,
          symbol: args.symbol,
        }),
      ];

      if (depth === "deep") {
        // Add additional analysis for deep mode
        analysisPromises.push(
          ctx.runAction(api.actions.ai_enhanced.createPriceModel, {
            tokenMint: args.tokenMint,
            symbol: args.symbol,
            timeHorizon: "1d",
          })
        );
      }

      const [sentimentResult, priceModelResult, signalResult, ...additionalResults] = await Promise.all(analysisPromises);
      
      // Generate overall assessment
      const overallAssessment = await generateOverallAssessment(
        args.symbol,
        sentimentResult.sentiment,
        priceModelResult.prediction,
        signalResult.signal
      );

      // Create summary embedding
      const summaryText = `${args.symbol} comprehensive analysis: ${overallAssessment.summary}`;
      const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
        text: summaryText,
      });

      const analysisTime = Date.now() - startTime;

      console.log(`✅ Comprehensive analysis completed for ${args.symbol} in ${analysisTime}ms`);

      return {
        symbol: args.symbol,
        analysisDepth: depth,
        timestamp: Date.now(),
        executionTime: analysisTime,
        results: {
          sentiment: sentimentResult,
          priceModel: priceModelResult,
          tradingSignal: signalResult,
          additional: additionalResults,
        },
        overallAssessment,
        embedding: embeddingResult.embedding,
      };
    } catch (error) {
      console.error(`❌ Error in comprehensive analysis for ${args.symbol}:`, error);
      throw new Error(`Comprehensive analysis failed: ${error}`);
    }
  },
});

// Helper functions

async function gatherSentimentData(symbol: string, sources: string[], timeframe: string) {
  // This would integrate with various APIs (Twitter, Reddit, news APIs)
  // For now, return mock data structure
  return {
    summary: `Market sentiment for ${symbol} over ${timeframe}`,
    sources: sources.map(source => ({
      name: source,
      dataPoints: Math.floor(Math.random() * 1000),
      avgSentiment: -1 + Math.random() * 2, // -1 to 1
    })),
    totalDataPoints: Math.floor(Math.random() * 5000),
    quality: "high",
  };
}

async function gatherModelData(ctx: any, tokenMint: string, symbol: string, timeHorizon: string) {
  // Get price history, technical indicators, and market data
  const [priceHistory, technicalData, marketData] = await Promise.all([
    ctx.runQuery(api.queries.prices.getPriceHistory, {
      tokenMint,
      interval: "1h",
      limit: 168, // 7 days of hourly data
    }),
    ctx.runAction(api.actions.analytics.calculateTokenIndicators, {
      tokenMint,
    }),
    ctx.runQuery(api.queries.prices.getTokenPrice, {
      mint: tokenMint,
    }),
  ]);

  return {
    priceHistory,
    technical: technicalData,
    market: marketData,
    quality: "high",
    timeHorizon,
  };
}

async function gatherSignalData(ctx: any, tokenMint: string, symbol: string) {
  // Get comprehensive data for signal generation
  const [aiContext, technicalData, recentSignals] = await Promise.all([
    ctx.runQuery(api.queries.ai.getTokenAIContext, { tokenMint }),
    ctx.runAction(api.actions.analytics.calculateTokenIndicators, { tokenMint }),
    ctx.runQuery(api.queries.ai.getLatestSignals, { tokenMint, limit: 5 }),
  ]);

  return {
    context: aiContext,
    technical: technicalData,
    recentSignals,
    quality: "high",
  };
}

async function analyzeSentimentWithAI(symbol: string, sentimentData: any): Promise<any> {
  const prompt = `Analyze the sentiment for ${symbol} based on the following data:
${JSON.stringify(sentimentData, null, 2)}

Provide a comprehensive sentiment analysis with:
1. Overall sentiment score (-100 to 100)
2. Confidence level (0-100)
3. Key signals and their strength
4. Trading recommendation (strong_buy, buy, hold, sell, strong_sell)
5. Summary of findings

Return as JSON with this structure:
{
  "sentiment_score": number,
  "confidence": number,
  "signals": [{"type": string, "strength": string, "description": string}],
  "recommendation": string,
  "summary": string
}`;

  return await callAIModel(prompt, "gpt-4-turbo-preview");
}

async function generatePricePrediction(symbol: string, modelData: any, timeHorizon: string, modelType: string): Promise<any> {
  const prompt = `Create a price prediction model for ${symbol} with ${timeHorizon} horizon using ${modelType} analysis.

Data available:
${JSON.stringify(modelData, null, 2)}

Provide:
1. Current price analysis
2. Target price prediction
3. Percentage change expectation
4. Confidence level (0-100)
5. Key factors influencing the prediction
6. Risk factors
7. Potential catalysts
8. Detailed reasoning

Return as JSON:
{
  "current_price": number,
  "target_price": number,
  "price_change_percent": number,
  "confidence": number,
  "confidence_score": number,
  "key_factors": string[],
  "risks": string[],
  "catalysts": string[],
  "reasoning": string,
  "summary": string,
  "recommendation": string
}`;

  return await callAIModel(prompt, "gpt-4-turbo-preview");
}

async function generateSignalWithAI(symbol: string, signalData: any, signalType: string, riskTolerance: string): Promise<any> {
  const prompt = `Generate a trading signal for ${symbol} based on comprehensive analysis.

Signal Type: ${signalType}
Risk Tolerance: ${riskTolerance}

Data:
${JSON.stringify(signalData, null, 2)}

Generate:
1. Trading action (buy/sell/hold)
2. Signal strength (0-100)
3. Confidence level (0-100)
4. Entry/exit strategy
5. Risk management levels
6. Timeframe for the signal
7. Technical, fundamental, and sentiment factors

Return as JSON:
{
  "action": string,
  "signal_type": string,
  "strength": number,
  "confidence": number,
  "reasoning": string,
  "price_target": number,
  "stop_loss": number,
  "timeframe": string,
  "risk_level": string,
  "technical_factors": string[],
  "fundamental_factors": string[],
  "sentiment_factors": string[],
  "risk_assessment": {
    "potential_upside": number,
    "potential_downside": number,
    "risk_reward_ratio": number
  }
}`;

  return await callAIModel(prompt, "gpt-4-turbo-preview");
}

async function generateOverallAssessment(symbol: string, sentiment: any, prediction: any, signal: any): Promise<any> {
  const prompt = `Provide an overall investment assessment for ${symbol} based on:

Sentiment Analysis: ${JSON.stringify(sentiment, null, 2)}
Price Prediction: ${JSON.stringify(prediction, null, 2)}
Trading Signal: ${JSON.stringify(signal, null, 2)}

Create a comprehensive assessment with:
1. Overall recommendation and confidence
2. Key strengths and weaknesses
3. Risk-reward analysis
4. Strategic positioning advice
5. Market conditions impact

Return as JSON:
{
  "overall_score": number,
  "recommendation": string,
  "confidence": number,
  "strengths": string[],
  "weaknesses": string[],
  "risk_reward": {
    "risk_score": number,
    "reward_potential": number,
    "risk_reward_ratio": number
  },
  "strategic_advice": string,
  "summary": string
}`;

  return await callAIModel(prompt, "gpt-4-turbo-preview");
}

async function callAIModel(prompt: string, model: string): Promise<any> {
  try {
    const response = await fetch("https://api.openai.com/v1/chat/completions", {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${OPENAI_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model,
        messages: [
          {
            role: "system",
            content: "You are an expert AI trading analyst. Always respond with valid JSON and provide detailed, actionable insights.",
          },
          {
            role: "user",
            content: prompt,
          },
        ],
        temperature: 0.3,
        response_format: { type: "json_object" },
      }),
    });

    if (!response.ok) {
      throw new Error(`OpenAI API error: ${response.status}`);
    }

    const data: OpenAIResponse = await response.json();
    const content = data.choices[0].message.content;
    
    return JSON.parse(content);
  } catch (error) {
    console.error("AI model call failed:", error);
    throw error;
  }
}

function getPredictionExpiry(timeHorizon: string): number {
  const multipliers: { [key: string]: number } = {
    "1d": 1 * 24 * 60 * 60 * 1000,
    "7d": 3 * 24 * 60 * 60 * 1000,
    "30d": 7 * 24 * 60 * 60 * 1000,
  };
  
  return multipliers[timeHorizon] || 3 * 24 * 60 * 60 * 1000;
}

function getSignalValidDuration(timeframe: string): number {
  const durations: { [key: string]: number } = {
    "short": 4 * 60 * 60 * 1000, // 4 hours
    "medium": 24 * 60 * 60 * 1000, // 1 day
    "long": 7 * 24 * 60 * 60 * 1000, // 7 days
  };
  
  return durations[timeframe] || 24 * 60 * 60 * 1000;
}