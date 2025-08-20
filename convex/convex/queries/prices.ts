import { query } from "../_generated/server";
import { v } from "convex/values";

// Watch real-time price updates for multiple tokens
export const watchPrices = query({
  args: {
    tokens: v.array(v.string()), // Array of token mints
    includeMetrics: v.optional(v.boolean()),
  },
  handler: async (ctx, args) => {
    // Get latest price for each token
    const prices = await Promise.all(
      args.tokens.map(async (tokenMint) => {
        const price = await ctx.db
          .query("priceFeeds")
          .withIndex("by_token", (q) => q.eq("tokenMint", tokenMint))
          .order("desc")
          .first();
        
        return price || null;
      })
    );

    // Filter out nulls and format response
    const validPrices = prices.filter(p => p !== null);

    return {
      prices: validPrices.map(p => ({
        tokenMint: p!.tokenMint,
        symbol: p!.symbol,
        name: p!.name,
        price: p!.price,
        prices: p!.prices,
        change24h: p!.changes.price24h,
        volume24h: args.includeMetrics ? p!.metrics.volume24h : undefined,
        marketCap: args.includeMetrics ? p!.metrics.marketCap : undefined,
        lastUpdate: p!.timestamp,
      })),
      timestamp: Date.now(),
    };
  },
});

// Get price history for charting
export const getPriceHistory = query({
  args: {
    tokenMint: v.string(),
    interval: v.union(
      v.literal("1m"),
      v.literal("5m"),
      v.literal("15m"),
      v.literal("1h"),
      v.literal("4h"),
      v.literal("1d")
    ),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 100;
    
    // Get historical prices
    const prices = await ctx.db
      .query("priceFeeds")
      .withIndex("by_token", (q) => q.eq("tokenMint", args.tokenMint))
      .order("desc")
      .take(limit)
      .collect();

    if (prices.length === 0) {
      return { data: [], interval: args.interval };
    }

    // Group by interval (simplified - in production would aggregate properly)
    const intervalMs = getIntervalMs(args.interval);
    const grouped = new Map<number, typeof prices[0]>();

    prices.forEach(price => {
      const bucket = Math.floor(price.timestamp / intervalMs) * intervalMs;
      if (!grouped.has(bucket) || price.timestamp > grouped.get(bucket)!.timestamp) {
        grouped.set(bucket, price);
      }
    });

    // Convert to OHLCV format
    const ohlcv = Array.from(grouped.entries())
      .sort((a, b) => a[0] - b[0])
      .map(([timestamp, price]) => ({
        timestamp,
        open: price.price,
        high: price.price,
        low: price.price,
        close: price.price,
        volume: price.metrics.volume24h,
      }));

    return {
      data: ohlcv,
      interval: args.interval,
      token: {
        mint: prices[0].tokenMint,
        symbol: prices[0].symbol,
        name: prices[0].name,
      },
    };
  },
});

// Get market overview
export const getMarketOverview = query({
  args: {
    category: v.optional(v.union(
      v.literal("all"),
      v.literal("defi"),
      v.literal("meme"),
      v.literal("gaming"),
      v.literal("ai")
    )),
    sortBy: v.optional(v.union(
      v.literal("volume"),
      v.literal("change"),
      v.literal("marketCap")
    )),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 20;
    const sortBy = args.sortBy || "volume";

    // Get latest prices for all tokens
    let prices = await ctx.db
      .query("priceFeeds")
      .order("desc")
      .take(1000) // Get many, then dedupe
      .collect();

    // Deduplicate by token (keep latest)
    const latestByToken = new Map();
    prices.forEach(p => {
      if (!latestByToken.has(p.tokenMint) || 
          p.timestamp > latestByToken.get(p.tokenMint).timestamp) {
        latestByToken.set(p.tokenMint, p);
      }
    });

    prices = Array.from(latestByToken.values());

    // Sort based on criteria
    switch (sortBy) {
      case "volume":
        prices.sort((a, b) => 
          parseFloat(b.metrics.volume24h) - parseFloat(a.metrics.volume24h)
        );
        break;
      case "change":
        prices.sort((a, b) => b.changes.price24h - a.changes.price24h);
        break;
      case "marketCap":
        prices.sort((a, b) => 
          parseFloat(b.metrics.marketCap) - parseFloat(a.metrics.marketCap)
        );
        break;
    }

    // Get top tokens
    const topTokens = prices.slice(0, limit);

    // Calculate market stats
    const totalVolume = prices.reduce((sum, p) => 
      sum + parseFloat(p.metrics.volume24h), 0
    );
    
    const avgChange = prices.reduce((sum, p) => 
      sum + p.changes.price24h, 0
    ) / prices.length;

    const gainers = prices.filter(p => p.changes.price24h > 0).length;
    const losers = prices.filter(p => p.changes.price24h < 0).length;

    return {
      tokens: topTokens.map(formatToken),
      stats: {
        totalVolume: totalVolume.toFixed(2),
        averageChange24h: avgChange.toFixed(2),
        gainers,
        losers,
        unchanged: prices.length - gainers - losers,
      },
      timestamp: Date.now(),
    };
  },
});

// Get trending tokens
export const getTrending = query({
  args: {
    timeframe: v.union(v.literal("1h"), v.literal("24h"), v.literal("7d")),
    metric: v.union(
      v.literal("volume"),
      v.literal("price"),
      v.literal("mentions")
    ),
  },
  handler: async (ctx, args) => {
    // Get recent prices
    const prices = await ctx.db
      .query("priceFeeds")
      .order("desc")
      .take(500)
      .collect();

    // Deduplicate and get latest for each token
    const latestByToken = new Map();
    prices.forEach(p => {
      if (!latestByToken.has(p.tokenMint) || 
          p.timestamp > latestByToken.get(p.tokenMint).timestamp) {
        latestByToken.set(p.tokenMint, p);
      }
    });

    const tokens = Array.from(latestByToken.values());

    // Sort by metric
    let sorted;
    switch (args.metric) {
      case "volume":
        sorted = tokens.sort((a, b) => 
          parseFloat(b.metrics.volumeChange24h) - parseFloat(a.metrics.volumeChange24h)
        );
        break;
      case "price":
        const changeKey = args.timeframe === "1h" ? "price1h" : 
                         args.timeframe === "7d" ? "price7d" : "price24h";
        sorted = tokens.sort((a, b) => 
          b.changes[changeKey] - a.changes[changeKey]
        );
        break;
      default:
        sorted = tokens;
    }

    // Get top 10 trending
    const trending = sorted.slice(0, 10);

    return {
      trending: trending.map(t => ({
        ...formatToken(t),
        trendScore: calculateTrendScore(t, args.metric, args.timeframe),
      })),
      timeframe: args.timeframe,
      metric: args.metric,
    };
  },
});

// Search tokens
export const searchTokens = query({
  args: {
    query: v.string(),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    const searchTerm = args.query.toLowerCase();

    // Get all unique tokens from recent price feeds
    const prices = await ctx.db
      .query("priceFeeds")
      .order("desc")
      .take(1000)
      .collect();

    // Deduplicate and search
    const matches = new Map();
    
    prices.forEach(p => {
      const symbolMatch = p.symbol.toLowerCase().includes(searchTerm);
      const nameMatch = p.name.toLowerCase().includes(searchTerm);
      const mintMatch = p.tokenMint.toLowerCase().startsWith(searchTerm);
      
      if ((symbolMatch || nameMatch || mintMatch) && !matches.has(p.tokenMint)) {
        matches.set(p.tokenMint, {
          ...p,
          relevance: symbolMatch ? 3 : nameMatch ? 2 : 1,
        });
      }
    });

    // Sort by relevance and volume
    const results = Array.from(matches.values())
      .sort((a, b) => {
        if (a.relevance !== b.relevance) {
          return b.relevance - a.relevance;
        }
        return parseFloat(b.metrics.volume24h) - parseFloat(a.metrics.volume24h);
      })
      .slice(0, limit);

    return {
      results: results.map(formatToken),
      query: args.query,
      count: results.length,
    };
  },
});

// Get price alerts for monitoring
export const getPriceAlerts = query({
  args: {
    tokenMint: v.string(),
    userId: v.id("users"),
  },
  handler: async (ctx, args) => {
    // Get current price
    const currentPrice = await ctx.db
      .query("priceFeeds")
      .withIndex("by_token", (q) => q.eq("tokenMint", args.tokenMint))
      .order("desc")
      .first();

    if (!currentPrice) {
      return { alerts: [], currentPrice: null };
    }

    // Get user's alerts for this token
    const alerts = await ctx.db
      .query("alerts")
      .withIndex("by_user_active", (q) => 
        q.eq("userId", args.userId).eq("isActive", true)
      )
      .filter((q) => 
        q.eq(q.field("condition.target"), args.tokenMint)
      )
      .collect();

    // Check which alerts would trigger
    const price = parseFloat(currentPrice.price);
    const triggered = alerts.map(alert => {
      const threshold = parseFloat(alert.condition.value);
      let wouldTrigger = false;

      switch (alert.condition.operator) {
        case "above":
          wouldTrigger = price > threshold;
          break;
        case "below":
          wouldTrigger = price < threshold;
          break;
        case "equals":
          wouldTrigger = Math.abs(price - threshold) < 0.001;
          break;
      }

      return {
        ...alert,
        wouldTrigger,
        currentPrice: price,
        threshold,
        distance: Math.abs(price - threshold),
        distancePercent: ((Math.abs(price - threshold) / price) * 100).toFixed(2),
      };
    });

    return {
      alerts: triggered,
      currentPrice: currentPrice.price,
      symbol: currentPrice.symbol,
    };
  },
});

// Helper functions
function getIntervalMs(interval: string): number {
  const intervals: Record<string, number> = {
    "1m": 60 * 1000,
    "5m": 5 * 60 * 1000,
    "15m": 15 * 60 * 1000,
    "1h": 60 * 60 * 1000,
    "4h": 4 * 60 * 60 * 1000,
    "1d": 24 * 60 * 60 * 1000,
  };
  return intervals[interval] || intervals["1h"];
}

function formatToken(price: any) {
  return {
    mint: price.tokenMint,
    symbol: price.symbol,
    name: price.name,
    price: price.price,
    change24h: price.changes.price24h,
    volume24h: price.metrics.volume24h,
    marketCap: price.metrics.marketCap,
    lastUpdate: price.timestamp,
  };
}

function calculateTrendScore(token: any, metric: string, timeframe: string): number {
  // Simple trend score calculation
  let score = 0;
  
  if (metric === "volume") {
    score = token.metrics.volumeChange24h * 10;
  } else if (metric === "price") {
    const change = timeframe === "1h" ? token.changes.price1h :
                   timeframe === "7d" ? token.changes.price7d :
                   token.changes.price24h;
    score = change;
  }
  
  // Boost for high volume
  const volume = parseFloat(token.metrics.volume24h);
  if (volume > 1000000) score *= 1.5;
  if (volume > 10000000) score *= 2;
  
  return Math.min(100, Math.max(0, score));
}