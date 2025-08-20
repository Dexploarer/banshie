import { action } from "../_generated/server";
import { api } from "../_generated/api";
import { v } from "convex/values";

// Calculate technical indicators for all tracked tokens
export const calculateTechnicalIndicators = action({
  handler: async (ctx) => {
    console.log("📊 Calculating technical indicators");
    
    // Get all tokens with sufficient price history
    const tokens = await ctx.runQuery(api.queries.analytics.getTokensWithHistory, {
      minDataPoints: 50,
    });

    const results = [];

    for (const token of tokens) {
      try {
        // Get price history (last 200 data points for better indicator accuracy)
        const priceHistory = await ctx.runQuery(api.queries.prices.getPriceHistory, {
          tokenMint: token.mint,
          interval: "1h",
          limit: 200,
        });

        if (priceHistory.data.length < 20) {
          console.log(`Skipping ${token.symbol}: insufficient data`);
          continue;
        }

        // Calculate all indicators
        const indicators = await calculateAllIndicators(priceHistory.data);

        // Store indicators in database
        await ctx.runMutation(api.mutations.analytics.storeTechnicalIndicators, {
          tokenMint: token.mint,
          indicators,
          timestamp: Date.now(),
        });

        results.push({
          token: token.symbol,
          indicators,
        });

        console.log(`✅ Indicators calculated for ${token.symbol}`);
      } catch (error) {
        console.error(`Failed to calculate indicators for ${token.symbol}:`, error);
      }
    }

    return {
      processed: results.length,
      tokens: results.map(r => r.token),
    };
  },
});

// Calculate indicators for a specific token
export const calculateTokenIndicators = action({
  args: {
    tokenMint: v.string(),
    periods: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const periods = args.periods || 100;
    
    console.log(`📊 Calculating indicators for token ${args.tokenMint}`);
    
    // Get price history
    const priceHistory = await ctx.runQuery(api.queries.prices.getPriceHistory, {
      tokenMint: args.tokenMint,
      interval: "1h",
      limit: periods,
    });

    if (priceHistory.data.length < 20) {
      throw new Error("Insufficient price data for technical analysis");
    }

    // Calculate indicators
    const indicators = await calculateAllIndicators(priceHistory.data);

    // Store in database
    await ctx.runMutation(api.mutations.analytics.storeTechnicalIndicators, {
      tokenMint: args.tokenMint,
      indicators,
      timestamp: Date.now(),
    });

    return indicators;
  },
});

// Main function to calculate all technical indicators
async function calculateAllIndicators(priceData: any[]): Promise<any> {
  const prices = priceData.map(d => parseFloat(d.close));
  const volumes = priceData.map(d => parseFloat(d.volume));
  const highs = priceData.map(d => parseFloat(d.high));
  const lows = priceData.map(d => parseFloat(d.low));

  // Reverse arrays to have oldest first for calculations
  prices.reverse();
  volumes.reverse();
  highs.reverse();
  lows.reverse();

  const indicators = {
    // Trend Indicators
    sma: {
      sma20: calculateSMA(prices, 20),
      sma50: calculateSMA(prices, 50),
      sma200: calculateSMA(prices, 200),
    },
    ema: {
      ema12: calculateEMA(prices, 12),
      ema26: calculateEMA(prices, 26),
      ema50: calculateEMA(prices, 50),
    },
    macd: calculateMACD(prices),
    
    // Momentum Indicators
    rsi: calculateRSI(prices, 14),
    stochastic: calculateStochastic(highs, lows, prices, 14),
    williams: calculateWilliamsR(highs, lows, prices, 14),
    
    // Volatility Indicators
    bollingerBands: calculateBollingerBands(prices, 20, 2),
    atr: calculateATR(highs, lows, prices, 14),
    
    // Volume Indicators
    volumeSMA: calculateSMA(volumes, 20),
    obv: calculateOBV(prices, volumes),
    
    // Support/Resistance
    pivotPoints: calculatePivotPoints(highs.slice(-1)[0], lows.slice(-1)[0], prices.slice(-1)[0]),
    
    // Custom Composite Indicators
    trendStrength: calculateTrendStrength(prices),
    volatilityScore: calculateVolatilityScore(prices),
    momentumScore: calculateMomentumScore(prices),
    
    // Signal Summary
    signals: generateTradingSignals(prices, highs, lows, volumes),
  };

  return indicators;
}

// Simple Moving Average
function calculateSMA(prices: number[], period: number): number | null {
  if (prices.length < period) return null;
  
  const slice = prices.slice(-period);
  const sum = slice.reduce((a, b) => a + b, 0);
  return sum / period;
}

// Exponential Moving Average
function calculateEMA(prices: number[], period: number): number | null {
  if (prices.length < period) return null;
  
  const multiplier = 2 / (period + 1);
  let ema = prices[0];
  
  for (let i = 1; i < prices.length; i++) {
    ema = (prices[i] * multiplier) + (ema * (1 - multiplier));
  }
  
  return ema;
}

// MACD (Moving Average Convergence Divergence)
function calculateMACD(prices: number[]): any {
  const ema12 = calculateEMA(prices, 12);
  const ema26 = calculateEMA(prices, 26);
  
  if (!ema12 || !ema26) return null;
  
  const macdLine = ema12 - ema26;
  
  // Calculate signal line (9-period EMA of MACD)
  const macdHistory = [];
  for (let i = 26; i <= prices.length; i++) {
    const slice = prices.slice(0, i);
    const ema12_slice = calculateEMA(slice, 12);
    const ema26_slice = calculateEMA(slice, 26);
    if (ema12_slice && ema26_slice) {
      macdHistory.push(ema12_slice - ema26_slice);
    }
  }
  
  const signalLine = calculateEMA(macdHistory, 9) || 0;
  const histogram = macdLine - signalLine;
  
  return {
    macd: macdLine,
    signal: signalLine,
    histogram,
  };
}

// RSI (Relative Strength Index)
function calculateRSI(prices: number[], period: number): number | null {
  if (prices.length < period + 1) return null;
  
  const changes = [];
  for (let i = 1; i < prices.length; i++) {
    changes.push(prices[i] - prices[i - 1]);
  }
  
  const gains = changes.map(c => c > 0 ? c : 0);
  const losses = changes.map(c => c < 0 ? Math.abs(c) : 0);
  
  const avgGain = gains.slice(-period).reduce((a, b) => a + b, 0) / period;
  const avgLoss = losses.slice(-period).reduce((a, b) => a + b, 0) / period;
  
  if (avgLoss === 0) return 100;
  
  const rs = avgGain / avgLoss;
  return 100 - (100 / (1 + rs));
}

// Stochastic Oscillator
function calculateStochastic(highs: number[], lows: number[], closes: number[], period: number): any {
  if (highs.length < period) return null;
  
  const highestHigh = Math.max(...highs.slice(-period));
  const lowestLow = Math.min(...lows.slice(-period));
  const currentClose = closes.slice(-1)[0];
  
  const k = ((currentClose - lowestLow) / (highestHigh - lowestLow)) * 100;
  
  // %D is typically a 3-period SMA of %K
  const kValues = [];
  for (let i = period - 1; i < highs.length; i++) {
    const periodHigh = Math.max(...highs.slice(i - period + 1, i + 1));
    const periodLow = Math.min(...lows.slice(i - period + 1, i + 1));
    const kValue = ((closes[i] - periodLow) / (periodHigh - periodLow)) * 100;
    kValues.push(kValue);
  }
  
  const d = kValues.slice(-3).reduce((a, b) => a + b, 0) / 3;
  
  return { k, d };
}

// Williams %R
function calculateWilliamsR(highs: number[], lows: number[], closes: number[], period: number): number | null {
  if (highs.length < period) return null;
  
  const highestHigh = Math.max(...highs.slice(-period));
  const lowestLow = Math.min(...lows.slice(-period));
  const currentClose = closes.slice(-1)[0];
  
  return ((highestHigh - currentClose) / (highestHigh - lowestLow)) * -100;
}

// Bollinger Bands
function calculateBollingerBands(prices: number[], period: number, stdDev: number): any {
  if (prices.length < period) return null;
  
  const sma = calculateSMA(prices, period);
  if (!sma) return null;
  
  const slice = prices.slice(-period);
  const variance = slice.reduce((acc, price) => acc + Math.pow(price - sma, 2), 0) / period;
  const standardDeviation = Math.sqrt(variance);
  
  return {
    upper: sma + (standardDeviation * stdDev),
    middle: sma,
    lower: sma - (standardDeviation * stdDev),
    bandwidth: ((sma + (standardDeviation * stdDev)) - (sma - (standardDeviation * stdDev))) / sma,
  };
}

// Average True Range (ATR)
function calculateATR(highs: number[], lows: number[], closes: number[], period: number): number | null {
  if (highs.length < period + 1) return null;
  
  const trueRanges = [];
  for (let i = 1; i < highs.length; i++) {
    const tr1 = highs[i] - lows[i];
    const tr2 = Math.abs(highs[i] - closes[i - 1]);
    const tr3 = Math.abs(lows[i] - closes[i - 1]);
    trueRanges.push(Math.max(tr1, tr2, tr3));
  }
  
  return calculateSMA(trueRanges, period);
}

// On-Balance Volume (OBV)
function calculateOBV(prices: number[], volumes: number[]): number {
  let obv = 0;
  
  for (let i = 1; i < prices.length; i++) {
    if (prices[i] > prices[i - 1]) {
      obv += volumes[i];
    } else if (prices[i] < prices[i - 1]) {
      obv -= volumes[i];
    }
    // If prices are equal, OBV remains unchanged
  }
  
  return obv;
}

// Pivot Points
function calculatePivotPoints(high: number, low: number, close: number): any {
  const pivot = (high + low + close) / 3;
  
  return {
    pivot,
    resistance1: (2 * pivot) - low,
    resistance2: pivot + (high - low),
    resistance3: high + (2 * (pivot - low)),
    support1: (2 * pivot) - high,
    support2: pivot - (high - low),
    support3: low - (2 * (high - pivot)),
  };
}

// Custom Composite Indicators

function calculateTrendStrength(prices: number[]): number {
  if (prices.length < 20) return 0;
  
  const sma20 = calculateSMA(prices, 20) || 0;
  const sma50 = calculateSMA(prices, 50) || 0;
  const currentPrice = prices.slice(-1)[0];
  
  let score = 0;
  
  // Price vs SMAs
  if (currentPrice > sma20) score += 25;
  if (currentPrice > sma50) score += 25;
  if (sma20 > sma50) score += 25;
  
  // Recent trend
  const recentSlope = (prices.slice(-1)[0] - prices.slice(-10)[0]) / 10;
  if (recentSlope > 0) score += 25;
  
  return score;
}

function calculateVolatilityScore(prices: number[]): number {
  if (prices.length < 20) return 0;
  
  const returns = [];
  for (let i = 1; i < prices.length; i++) {
    returns.push((prices[i] - prices[i - 1]) / prices[i - 1]);
  }
  
  const mean = returns.reduce((a, b) => a + b, 0) / returns.length;
  const variance = returns.reduce((acc, ret) => acc + Math.pow(ret - mean, 2), 0) / returns.length;
  const volatility = Math.sqrt(variance) * Math.sqrt(252); // Annualized
  
  // Convert to 0-100 scale
  return Math.min(100, volatility * 100);
}

function calculateMomentumScore(prices: number[]): number {
  if (prices.length < 10) return 50;
  
  const rsi = calculateRSI(prices, 14) || 50;
  const priceChange = ((prices.slice(-1)[0] - prices.slice(-10)[0]) / prices.slice(-10)[0]) * 100;
  
  // Combine RSI and price momentum
  const momentumScore = (rsi + Math.max(-50, Math.min(50, priceChange * 2)) + 50) / 2;
  
  return Math.max(0, Math.min(100, momentumScore));
}

// Generate trading signals based on technical indicators
function generateTradingSignals(prices: number[], highs: number[], lows: number[], volumes: number[]): any {
  const signals = {
    overall: 'neutral' as 'bullish' | 'bearish' | 'neutral',
    strength: 0,
    signals: [] as any[],
  };

  const rsi = calculateRSI(prices, 14);
  const macd = calculateMACD(prices);
  const bb = calculateBollingerBands(prices, 20, 2);
  const sma20 = calculateSMA(prices, 20);
  const sma50 = calculateSMA(prices, 50);
  const currentPrice = prices.slice(-1)[0];

  let bullishSignals = 0;
  let bearishSignals = 0;

  // RSI Signals
  if (rsi) {
    if (rsi < 30) {
      signals.signals.push({ type: 'RSI', signal: 'oversold', strength: 'strong' });
      bullishSignals += 2;
    } else if (rsi > 70) {
      signals.signals.push({ type: 'RSI', signal: 'overbought', strength: 'strong' });
      bearishSignals += 2;
    }
  }

  // MACD Signals
  if (macd) {
    if (macd.macd > macd.signal && macd.histogram > 0) {
      signals.signals.push({ type: 'MACD', signal: 'bullish_crossover', strength: 'medium' });
      bullishSignals += 1;
    } else if (macd.macd < macd.signal && macd.histogram < 0) {
      signals.signals.push({ type: 'MACD', signal: 'bearish_crossover', strength: 'medium' });
      bearishSignals += 1;
    }
  }

  // Moving Average Signals
  if (sma20 && sma50) {
    if (currentPrice > sma20 && sma20 > sma50) {
      signals.signals.push({ type: 'MA', signal: 'uptrend', strength: 'medium' });
      bullishSignals += 1;
    } else if (currentPrice < sma20 && sma20 < sma50) {
      signals.signals.push({ type: 'MA', signal: 'downtrend', strength: 'medium' });
      bearishSignals += 1;
    }
  }

  // Bollinger Bands Signals
  if (bb) {
    if (currentPrice < bb.lower) {
      signals.signals.push({ type: 'BB', signal: 'oversold', strength: 'medium' });
      bullishSignals += 1;
    } else if (currentPrice > bb.upper) {
      signals.signals.push({ type: 'BB', signal: 'overbought', strength: 'medium' });
      bearishSignals += 1;
    }
  }

  // Determine overall signal
  const netSignal = bullishSignals - bearishSignals;
  if (netSignal > 1) {
    signals.overall = 'bullish';
    signals.strength = Math.min(100, netSignal * 20);
  } else if (netSignal < -1) {
    signals.overall = 'bearish';
    signals.strength = Math.min(100, Math.abs(netSignal) * 20);
  } else {
    signals.overall = 'neutral';
    signals.strength = 0;
  }

  return signals;
}