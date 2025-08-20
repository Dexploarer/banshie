import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  // User management
  users: defineTable({
    telegramId: v.number(),
    username: v.string(),
    email: v.optional(v.string()),
    isActive: v.boolean(),
    isPremium: v.boolean(),
    settings: v.object({
      defaultSlippage: v.number(),
      maxPositionSize: v.string(),
      autoCompound: v.boolean(),
      riskLevel: v.union(v.literal("conservative"), v.literal("moderate"), v.literal("aggressive")),
      notifications: v.object({
        trades: v.boolean(),
        alerts: v.boolean(),
        daily: v.boolean(),
      }),
    }),
    limits: v.object({
      dailyTrades: v.number(),
      maxOpenPositions: v.number(),
      maxOrderValue: v.string(),
    }),
    stats: v.object({
      totalTrades: v.number(),
      successRate: v.number(),
      totalVolume: v.string(),
      totalPnL: v.string(),
    }),
    createdAt: v.number(),
    lastActive: v.number(),
  })
    .index("by_telegram", ["telegramId"])
    .index("by_username", ["username"])
    .index("by_activity", ["lastActive"])
    .index("by_premium", ["isPremium"]),

  // Wallet management
  wallets: defineTable({
    userId: v.id("users"),
    address: v.string(),
    publicKey: v.string(),
    type: v.union(
      v.literal("hot"),
      v.literal("imported"),
      v.literal("ledger"),
      v.literal("trezor")
    ),
    label: v.optional(v.string()),
    isActive: v.boolean(),
    isDefault: v.boolean(),
    balance: v.object({
      sol: v.string(),
      usd: v.string(),
      tokens: v.array(v.object({
        mint: v.string(),
        symbol: v.string(),
        amount: v.string(),
        usdValue: v.string(),
      })),
      lastUpdated: v.number(),
    }),
    performance: v.object({
      totalDeposited: v.string(),
      totalWithdrawn: v.string(),
      realizedPnL: v.string(),
      unrealizedPnL: v.string(),
    }),
    createdAt: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_address", ["address"])
    .index("by_user_active", ["userId", "isActive"]),

  // Trading positions
  positions: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    tokenMint: v.string(),
    symbol: v.string(),
    name: v.string(),
    amount: v.string(),
    decimals: v.number(),
    averagePrice: v.string(),
    currentPrice: v.string(),
    marketValue: v.string(),
    costBasis: v.string(),
    pnl: v.object({
      amount: v.string(),
      percentage: v.number(),
      isProfit: v.boolean(),
    }),
    metadata: v.object({
      logoUri: v.optional(v.string()),
      coingeckoId: v.optional(v.string()),
      website: v.optional(v.string()),
      twitter: v.optional(v.string()),
    }),
    analytics: v.object({
      priceChange24h: v.number(),
      volume24h: v.string(),
      marketCap: v.string(),
      holdTime: v.number(), // in seconds
    }),
    openedAt: v.number(),
    lastUpdated: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_wallet", ["walletId"])
    .index("by_token", ["tokenMint"])
    .index("by_user_token", ["userId", "tokenMint"])
    .index("by_pnl", ["pnl.percentage"]),

  // Order management
  orders: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    type: v.union(
      v.literal("market"),
      v.literal("limit"),
      v.literal("stop_loss"),
      v.literal("take_profit"),
      v.literal("trailing_stop"),
      v.literal("dca"),
      v.literal("twap"),
      v.literal("iceberg")
    ),
    side: v.union(v.literal("buy"), v.literal("sell")),
    status: v.union(
      v.literal("pending"),
      v.literal("submitted"),
      v.literal("executing"),
      v.literal("partial"),
      v.literal("completed"),
      v.literal("failed"),
      v.literal("cancelled")
    ),
    tokenIn: v.object({
      mint: v.string(),
      symbol: v.string(),
      amount: v.string(),
      decimals: v.number(),
    }),
    tokenOut: v.object({
      mint: v.string(),
      symbol: v.string(),
      amount: v.string(),
      decimals: v.number(),
    }),
    pricing: v.object({
      expectedPrice: v.string(),
      executionPrice: v.optional(v.string()),
      slippage: v.number(),
      priceImpact: v.optional(v.number()),
      fee: v.optional(v.string()),
    }),
    routing: v.optional(v.object({
      dex: v.string(),
      route: v.array(v.string()),
      estimatedGas: v.string(),
    })),
    conditions: v.optional(v.object({
      triggerPrice: v.optional(v.string()),
      limitPrice: v.optional(v.string()),
      stopPrice: v.optional(v.string()),
      timeInForce: v.optional(v.string()),
      expiresAt: v.optional(v.number()),
    })),
    execution: v.optional(v.object({
      txSignature: v.string(),
      blockHeight: v.number(),
      gasUsed: v.string(),
      attempts: v.number(),
    })),
    error: v.optional(v.object({
      code: v.string(),
      message: v.string(),
      details: v.optional(v.string()),
    })),
    metadata: v.any(),
    createdAt: v.number(),
    updatedAt: v.number(),
    executedAt: v.optional(v.number()),
  })
    .index("by_user", ["userId"])
    .index("by_user_status", ["userId", "status"])
    .index("by_wallet", ["walletId"])
    .index("by_created", ["createdAt"])
    .index("by_type", ["type"])
    .index("by_status", ["status"]),

  // DCA strategies
  dcaStrategies: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    name: v.string(),
    description: v.optional(v.string()),
    isActive: v.boolean(),
    isPaused: v.boolean(),
    config: v.object({
      tokenIn: v.object({
        mint: v.string(),
        symbol: v.string(),
      }),
      tokenOut: v.object({
        mint: v.string(),
        symbol: v.string(),
      }),
      amount: v.string(),
      frequency: v.object({
        type: v.union(
          v.literal("interval"),
          v.literal("cron"),
          v.literal("dynamic")
        ),
        value: v.string(), // "1h", "*/30 * * * *", etc.
      }),
      conditions: v.optional(v.object({
        minPrice: v.optional(v.string()),
        maxPrice: v.optional(v.string()),
        onlyBuyDips: v.boolean(),
        dipThreshold: v.optional(v.number()),
      })),
      limits: v.object({
        maxInvestment: v.optional(v.string()),
        maxExecutions: v.optional(v.number()),
        endDate: v.optional(v.number()),
      }),
      advanced: v.object({
        valueAveraging: v.boolean(),
        rebalancing: v.boolean(),
        compounding: v.boolean(),
      }),
    }),
    stats: v.object({
      totalExecutions: v.number(),
      totalInvested: v.string(),
      totalReceived: v.string(),
      averagePrice: v.string(),
      currentValue: v.string(),
      pnl: v.object({
        amount: v.string(),
        percentage: v.number(),
      }),
      lastExecution: v.optional(v.number()),
      nextExecution: v.optional(v.number()),
    }),
    executions: v.array(v.object({
      orderId: v.id("orders"),
      timestamp: v.number(),
      amountIn: v.string(),
      amountOut: v.string(),
      price: v.string(),
      txSignature: v.optional(v.string()),
      status: v.string(),
    })),
    createdAt: v.number(),
    updatedAt: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_user_active", ["userId", "isActive"])
    .index("by_next_execution", ["stats.nextExecution"])
    .index("by_wallet", ["walletId"]),

  // Price feeds
  priceFeeds: defineTable({
    tokenMint: v.string(),
    symbol: v.string(),
    name: v.string(),
    price: v.string(),
    prices: v.object({
      usd: v.string(),
      sol: v.string(),
      btc: v.optional(v.string()),
      eth: v.optional(v.string()),
    }),
    metrics: v.object({
      volume24h: v.string(),
      volumeChange24h: v.number(),
      marketCap: v.string(),
      fdv: v.optional(v.string()),
      circulatingSupply: v.optional(v.string()),
      totalSupply: v.optional(v.string()),
    }),
    changes: v.object({
      price1h: v.number(),
      price24h: v.number(),
      price7d: v.number(),
      price30d: v.number(),
    }),
    technical: v.optional(v.object({
      rsi: v.number(),
      macd: v.object({
        value: v.number(),
        signal: v.number(),
        histogram: v.number(),
      }),
      ma20: v.string(),
      ma50: v.string(),
      ma200: v.string(),
      support: v.string(),
      resistance: v.string(),
    })),
    source: v.object({
      primary: v.string(),
      secondary: v.optional(v.string()),
      lastUpdate: v.number(),
      confidence: v.number(),
    }),
    timestamp: v.number(),
  })
    .index("by_token", ["tokenMint"])
    .index("by_symbol", ["symbol"])
    .index("by_timestamp", ["timestamp"])
    .index("by_volume", ["metrics.volume24h"]),

  // Alerts
  alerts: defineTable({
    userId: v.id("users"),
    name: v.string(),
    type: v.union(
      v.literal("price"),
      v.literal("volume"),
      v.literal("position"),
      v.literal("technical"),
      v.literal("news"),
      v.literal("whale")
    ),
    isActive: v.boolean(),
    condition: v.object({
      target: v.string(), // token mint or position id
      metric: v.string(), // "price", "volume", "pnl", etc.
      operator: v.union(
        v.literal("above"),
        v.literal("below"),
        v.literal("equals"),
        v.literal("change")
      ),
      value: v.string(),
      timeframe: v.optional(v.string()),
    }),
    actions: v.array(v.union(
      v.literal("notify"),
      v.literal("execute_trade"),
      v.literal("pause_strategy"),
      v.literal("email"),
      v.literal("webhook")
    )),
    notification: v.object({
      channels: v.array(v.string()),
      message: v.optional(v.string()),
      cooldown: v.number(), // seconds
    }),
    stats: v.object({
      triggered: v.boolean(),
      triggerCount: v.number(),
      lastTriggered: v.optional(v.number()),
      nextCheck: v.number(),
    }),
    metadata: v.any(),
    createdAt: v.number(),
    expiresAt: v.optional(v.number()),
  })
    .index("by_user", ["userId"])
    .index("by_user_active", ["userId", "isActive"])
    .index("by_type", ["type"])
    .index("by_next_check", ["stats.nextCheck"]),

  // Trading history
  trades: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    orderId: v.id("orders"),
    type: v.string(),
    side: v.union(v.literal("buy"), v.literal("sell")),
    tokenIn: v.object({
      mint: v.string(),
      symbol: v.string(),
      amount: v.string(),
      price: v.string(),
      value: v.string(),
    }),
    tokenOut: v.object({
      mint: v.string(),
      symbol: v.string(),
      amount: v.string(),
      price: v.string(),
      value: v.string(),
    }),
    execution: v.object({
      dex: v.string(),
      txSignature: v.string(),
      blockHeight: v.number(),
      slot: v.number(),
      gasUsed: v.string(),
      gasCost: v.string(),
    }),
    fees: v.object({
      network: v.string(),
      dex: v.string(),
      platform: v.string(),
      total: v.string(),
    }),
    pnl: v.optional(v.object({
      realized: v.string(),
      percentage: v.number(),
      holdTime: v.number(),
    })),
    metadata: v.any(),
    timestamp: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_wallet", ["walletId"])
    .index("by_timestamp", ["timestamp"])
    .index("by_token_in", ["tokenIn.mint"])
    .index("by_token_out", ["tokenOut.mint"]),

  // Sessions
  sessions: defineTable({
    userId: v.id("users"),
    token: v.string(),
    type: v.union(
      v.literal("telegram"),
      v.literal("web"),
      v.literal("api")
    ),
    device: v.optional(v.object({
      userAgent: v.string(),
      ip: v.string(),
      platform: v.string(),
    })),
    isActive: v.boolean(),
    lastActivity: v.number(),
    expiresAt: v.number(),
    createdAt: v.number(),
  })
    .index("by_token", ["token"])
    .index("by_user", ["userId"])
    .index("by_user_active", ["userId", "isActive"])
    .index("by_expiry", ["expiresAt"]),

  // AI Analysis Cache with Vector Search
  aiAnalysis: defineTable({
    targetId: v.string(), // token mint or market id
    type: v.union(
      v.literal("sentiment"),
      v.literal("technical"),
      v.literal("fundamental"),
      v.literal("prediction")
    ),
    analysis: v.object({
      summary: v.string(),
      score: v.number(), // -100 to 100
      confidence: v.number(), // 0 to 1
      signals: v.array(v.object({
        type: v.string(),
        strength: v.string(),
        description: v.string(),
      })),
      recommendation: v.union(
        v.literal("strong_buy"),
        v.literal("buy"),
        v.literal("hold"),
        v.literal("sell"),
        v.literal("strong_sell")
      ),
    }),
    embedding: v.optional(v.array(v.number())), // Vector embedding for semantic search
    sources: v.array(v.string()),
    model: v.string(),
    timestamp: v.number(),
    expiresAt: v.number(),
  })
    .index("by_target", ["targetId"])
    .index("by_type", ["type"])
    .index("by_timestamp", ["timestamp"])
    .index("by_target_type", ["targetId", "type"])
    .vectorIndex("by_analysis_embedding", {
      vectorField: "embedding",
      dimensions: 1536, // OpenAI embedding dimensions
      filterFields: ["targetId", "type", "timestamp"],
    }),

  // Trading Signals with Vector Search
  tradingSignals: defineTable({
    tokenMint: v.string(),
    symbol: v.string(),
    signalType: v.string(), // "momentum", "reversal", "breakout", "support", "resistance"
    action: v.string(), // "buy", "sell", "hold"
    strength: v.number(), // 0 to 100
    confidence: v.number(), // 0 to 100
    reasoning: v.string(),
    technicalFactors: v.array(v.string()),
    fundamentalFactors: v.array(v.string()),
    sentimentFactors: v.array(v.string()),
    priceTarget: v.optional(v.number()),
    stopLoss: v.optional(v.number()),
    timeframe: v.string(), // "short", "medium", "long"
    riskLevel: v.string(), // "low", "medium", "high"
    embedding: v.optional(v.array(v.number())), // For finding similar signals
    validUntil: v.number(),
    createdAt: v.number(),
    performance: v.optional(v.object({
      executed: v.boolean(),
      outcome: v.optional(v.string()), // "profit", "loss", "neutral"
      returnPct: v.optional(v.number()),
      updatedAt: v.number(),
    })),
  })
    .index("by_token", ["tokenMint"])
    .index("by_symbol", ["symbol"])
    .index("by_type", ["signalType"])
    .index("by_action", ["action"])
    .index("by_strength", ["strength"])
    .index("by_confidence", ["confidence"])
    .index("by_created", ["createdAt"])
    .index("by_valid_until", ["validUntil"])
    .index("by_token_action", ["tokenMint", "action"])
    .vectorIndex("by_signal_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["tokenMint", "signalType", "action", "createdAt"],
    }),

  // Knowledge Base for AI Context
  knowledgeBase: defineTable({
    category: v.string(), // "token", "project", "defi", "analysis", "strategy"
    title: v.string(),
    content: v.string(),
    metadata: v.object({
      tokenMints: v.optional(v.array(v.string())),
      tags: v.array(v.string()),
      source: v.optional(v.string()),
      confidence: v.optional(v.number()),
      lastVerified: v.optional(v.number()),
    }),
    embedding: v.array(v.number()), // Vector embedding for semantic search
    createdAt: v.number(),
    updatedAt: v.number(),
  })
    .index("by_category", ["category"])
    .index("by_created", ["createdAt"])
    .index("by_updated", ["updatedAt"])
    .vectorIndex("by_content_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["category", "createdAt"],
    }),

  // User Queries for Semantic Search History
  userQueries: defineTable({
    userId: v.id("users"),
    query: v.string(),
    queryType: v.string(), // "general", "trading", "analysis", "price"
    intent: v.optional(v.string()), // "buy", "sell", "analyze", "learn"
    tokenMints: v.optional(v.array(v.string())), // Extracted token mentions
    embedding: v.array(v.number()), // Query embedding for similarity search
    results: v.optional(v.array(v.object({
      type: v.string(),
      id: v.string(),
      relevanceScore: v.number(),
    }))),
    satisfaction: v.optional(v.number()), // User feedback 1-5
    timestamp: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_type", ["queryType"])
    .index("by_timestamp", ["timestamp"])
    .index("by_user_timestamp", ["userId", "timestamp"])
    .vectorIndex("by_query_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["userId", "queryType", "timestamp"],
    }),

  // Chat Conversations for Context
  conversations: defineTable({
    userId: v.id("users"),
    sessionId: v.string(),
    messages: v.array(v.object({
      role: v.string(), // "user", "assistant"
      content: v.string(),
      timestamp: v.number(),
      metadata: v.optional(v.object({
        tokenMints: v.optional(v.array(v.string())),
        intent: v.optional(v.string()),
        confidence: v.optional(v.number()),
      })),
    })),
    summary: v.optional(v.string()),
    embedding: v.optional(v.array(v.number())), // Conversation summary embedding
    startedAt: v.number(),
    lastMessageAt: v.number(),
    messageCount: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_session", ["sessionId"])
    .index("by_started", ["startedAt"])
    .index("by_last_message", ["lastMessageAt"])
    .index("by_user_last_message", ["userId", "lastMessageAt"])
    .vectorIndex("by_conversation_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["userId", "startedAt"],
    }),

  // Market Events with Vector Search
  marketEvents: defineTable({
    type: v.union(
      v.literal("listing"),
      v.literal("delisting"),
      v.literal("hack"),
      v.literal("update"),
      v.literal("partnership"),
      v.literal("regulation")
    ),
    severity: v.union(
      v.literal("info"),
      v.literal("low"),
      v.literal("medium"),
      v.literal("high"),
      v.literal("critical")
    ),
    title: v.string(),
    description: v.string(),
    affectedTokens: v.array(v.string()),
    source: v.object({
      name: v.string(),
      url: v.optional(v.string()),
      verified: v.boolean(),
    }),
    impact: v.object({
      priceChange: v.optional(v.number()),
      volumeChange: v.optional(v.number()),
      sentiment: v.optional(v.string()),
    }),
    embedding: v.optional(v.array(v.number())), // Vector embedding for semantic search
    timestamp: v.number(),
  })
    .index("by_type", ["type"])
    .index("by_severity", ["severity"])
    .index("by_timestamp", ["timestamp"])
    .index("by_type_severity", ["type", "severity"])
    .vectorIndex("by_content_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["type", "severity", "timestamp"],
    }),
});