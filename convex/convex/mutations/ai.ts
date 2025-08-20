import { mutation } from "../_generated/server";
import { v } from "convex/values";

// Store AI analysis with embedding
export const storeAIAnalysis = mutation({
  args: {
    targetId: v.string(),
    type: v.union(
      v.literal("sentiment"),
      v.literal("technical"),
      v.literal("fundamental"),
      v.literal("prediction")
    ),
    analysis: v.object({
      summary: v.string(),
      score: v.number(),
      confidence: v.number(),
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
    embedding: v.optional(v.array(v.number())),
    sources: v.array(v.string()),
    model: v.string(),
    expiresAt: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    console.log(`💾 Storing AI analysis for ${args.targetId} (${args.type})`);
    
    const analysisId = await ctx.db.insert("aiAnalysis", {
      targetId: args.targetId,
      type: args.type,
      analysis: args.analysis,
      embedding: args.embedding,
      sources: args.sources,
      model: args.model,
      timestamp: Date.now(),
      expiresAt: args.expiresAt || Date.now() + (7 * 24 * 60 * 60 * 1000), // 7 days default
    });

    console.log(`✅ Stored AI analysis with ID: ${analysisId}`);
    return analysisId;
  },
});

// Store trading signal with embedding
export const storeTradingSignal = mutation({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    signalType: v.string(),
    action: v.string(),
    strength: v.number(),
    confidence: v.number(),
    reasoning: v.string(),
    technicalFactors: v.array(v.string()),
    fundamentalFactors: v.array(v.string()),
    sentimentFactors: v.array(v.string()),
    priceTarget: v.optional(v.number()),
    stopLoss: v.optional(v.number()),
    timeframe: v.string(),
    riskLevel: v.string(),
    embedding: v.optional(v.array(v.number())),
    validUntil: v.number(),
  },
  handler: async (ctx, args) => {
    console.log(`💾 Storing trading signal for ${args.symbol} (${args.action})`);
    
    const signalId = await ctx.db.insert("tradingSignals", {
      tokenMint: args.tokenMint,
      symbol: args.symbol,
      signalType: args.signalType,
      action: args.action,
      strength: args.strength,
      confidence: args.confidence,
      reasoning: args.reasoning,
      technicalFactors: args.technicalFactors,
      fundamentalFactors: args.fundamentalFactors,
      sentimentFactors: args.sentimentFactors,
      priceTarget: args.priceTarget,
      stopLoss: args.stopLoss,
      timeframe: args.timeframe,
      riskLevel: args.riskLevel,
      embedding: args.embedding,
      validUntil: args.validUntil,
      createdAt: Date.now(),
    });

    console.log(`✅ Stored trading signal with ID: ${signalId}`);
    return signalId;
  },
});

// Update signal performance
export const updateSignalPerformance = mutation({
  args: {
    signalId: v.id("tradingSignals"),
    performance: v.object({
      executed: v.boolean(),
      outcome: v.optional(v.string()),
      returnPct: v.optional(v.number()),
      updatedAt: v.number(),
    }),
  },
  handler: async (ctx, args) => {
    console.log(`📊 Updating signal performance for ${args.signalId}`);
    
    const signal = await ctx.db.get(args.signalId);
    if (!signal) {
      throw new Error("Signal not found");
    }

    await ctx.db.patch(args.signalId, {
      performance: args.performance,
    });

    console.log(`✅ Updated signal performance: ${args.performance.outcome} (${args.performance.returnPct}%)`);
    return args.signalId;
  },
});

// Store knowledge base entry with embedding
export const storeKnowledgeEntry = mutation({
  args: {
    category: v.string(),
    title: v.string(),
    content: v.string(),
    metadata: v.object({
      tokenMints: v.optional(v.array(v.string())),
      tags: v.array(v.string()),
      source: v.optional(v.string()),
      confidence: v.optional(v.number()),
      lastVerified: v.optional(v.number()),
    }),
    embedding: v.array(v.number()),
  },
  handler: async (ctx, args) => {
    console.log(`💾 Storing knowledge entry: ${args.title}`);
    
    const entryId = await ctx.db.insert("knowledgeBase", {
      category: args.category,
      title: args.title,
      content: args.content,
      metadata: args.metadata,
      embedding: args.embedding,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    });

    console.log(`✅ Stored knowledge entry with ID: ${entryId}`);
    return entryId;
  },
});

// Store user query with embedding
export const storeUserQuery = mutation({
  args: {
    userId: v.id("users"),
    query: v.string(),
    queryType: v.string(),
    intent: v.optional(v.string()),
    tokenMints: v.optional(v.array(v.string())),
    embedding: v.optional(v.array(v.number())),
    results: v.optional(v.array(v.object({
      type: v.string(),
      id: v.string(),
      relevanceScore: v.number(),
    }))),
  },
  handler: async (ctx, args) => {
    console.log(`💾 Storing user query from ${args.userId}: ${args.query}`);
    
    const queryId = await ctx.db.insert("userQueries", {
      userId: args.userId,
      query: args.query,
      queryType: args.queryType,
      intent: args.intent,
      tokenMints: args.tokenMints,
      embedding: args.embedding || [], // Will be populated by action if not provided
      results: args.results,
      timestamp: Date.now(),
    });

    console.log(`✅ Stored user query with ID: ${queryId}`);
    return queryId;
  },
});

// Update query satisfaction feedback
export const updateQuerySatisfaction = mutation({
  args: {
    queryId: v.id("userQueries"),
    satisfaction: v.number(), // 1-5 rating
  },
  handler: async (ctx, args) => {
    console.log(`📊 Updating query satisfaction for ${args.queryId}: ${args.satisfaction}/5`);
    
    const query = await ctx.db.get(args.queryId);
    if (!query) {
      throw new Error("Query not found");
    }

    await ctx.db.patch(args.queryId, {
      satisfaction: args.satisfaction,
    });

    console.log(`✅ Updated query satisfaction`);
    return args.queryId;
  },
});

// Store or update conversation with embedding
export const storeConversation = mutation({
  args: {
    userId: v.id("users"),
    sessionId: v.string(),
    messages: v.array(v.object({
      role: v.string(),
      content: v.string(),
      timestamp: v.number(),
      metadata: v.optional(v.object({
        tokenMints: v.optional(v.array(v.string())),
        intent: v.optional(v.string()),
        confidence: v.optional(v.number()),
      })),
    })),
    summary: v.optional(v.string()),
    embedding: v.optional(v.array(v.number())),
  },
  handler: async (ctx, args) => {
    console.log(`💾 Storing conversation for user ${args.userId}, session ${args.sessionId}`);
    
    // Check if conversation already exists
    const existing = await ctx.db
      .query("conversations")
      .withIndex("by_session", (q) => q.eq("sessionId", args.sessionId))
      .first();

    const lastMessageAt = Math.max(...args.messages.map(m => m.timestamp));

    if (existing) {
      // Update existing conversation
      await ctx.db.patch(existing._id, {
        messages: args.messages,
        summary: args.summary,
        embedding: args.embedding,
        lastMessageAt,
        messageCount: args.messages.length,
      });
      
      console.log(`✅ Updated existing conversation: ${existing._id}`);
      return existing._id;
    } else {
      // Create new conversation
      const conversationId = await ctx.db.insert("conversations", {
        userId: args.userId,
        sessionId: args.sessionId,
        messages: args.messages,
        summary: args.summary,
        embedding: args.embedding,
        startedAt: Math.min(...args.messages.map(m => m.timestamp)),
        lastMessageAt,
        messageCount: args.messages.length,
      });

      console.log(`✅ Created new conversation: ${conversationId}`);
      return conversationId;
    }
  },
});

// Store market event with embedding
export const storeMarketEvent = mutation({
  args: {
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
    embedding: v.optional(v.array(v.number())),
  },
  handler: async (ctx, args) => {
    console.log(`💾 Storing market event: ${args.title}`);
    
    const eventId = await ctx.db.insert("marketEvents", {
      type: args.type,
      severity: args.severity,
      title: args.title,
      description: args.description,
      affectedTokens: args.affectedTokens,
      source: args.source,
      impact: args.impact,
      embedding: args.embedding,
      timestamp: Date.now(),
    });

    console.log(`✅ Stored market event with ID: ${eventId}`);
    return eventId;
  },
});

// Update analysis embedding (for existing records)
export const updateAnalysisEmbedding = mutation({
  args: {
    analysisId: v.id("aiAnalysis"),
    embedding: v.array(v.number()),
  },
  handler: async (ctx, args) => {
    console.log(`🔄 Updating embedding for analysis ${args.analysisId}`);
    
    const analysis = await ctx.db.get(args.analysisId);
    if (!analysis) {
      throw new Error("Analysis not found");
    }

    await ctx.db.patch(args.analysisId, {
      embedding: args.embedding,
    });

    console.log(`✅ Updated analysis embedding`);
    return args.analysisId;
  },
});

// Update signal embedding (for existing records)
export const updateSignalEmbedding = mutation({
  args: {
    signalId: v.id("tradingSignals"),
    embedding: v.array(v.number()),
  },
  handler: async (ctx, args) => {
    console.log(`🔄 Updating embedding for signal ${args.signalId}`);
    
    const signal = await ctx.db.get(args.signalId);
    if (!signal) {
      throw new Error("Signal not found");
    }

    await ctx.db.patch(args.signalId, {
      embedding: args.embedding,
    });

    console.log(`✅ Updated signal embedding`);
    return args.signalId;
  },
});

// Clean up expired analysis
export const cleanupExpiredAnalysis = mutation({
  args: {},
  handler: async (ctx, args) => {
    console.log(`🧹 Cleaning up expired AI analysis`);
    
    const now = Date.now();
    const expired = await ctx.db
      .query("aiAnalysis")
      .filter((q) => q.lt(q.field("expiresAt"), now))
      .collect();

    let deletedCount = 0;
    for (const analysis of expired) {
      await ctx.db.delete(analysis._id);
      deletedCount++;
    }

    console.log(`✅ Deleted ${deletedCount} expired analysis records`);
    return { deletedCount };
  },
});

// Clean up old user queries (keep last 1000 per user)
export const cleanupOldQueries = mutation({
  args: {
    userId: v.id("users"),
    keepCount: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const keepCount = args.keepCount || 1000;
    
    console.log(`🧹 Cleaning up old queries for user ${args.userId}`);
    
    const allQueries = await ctx.db
      .query("userQueries")
      .withIndex("by_user_timestamp", (q) => q.eq("userId", args.userId))
      .order("desc")
      .collect();

    if (allQueries.length <= keepCount) {
      console.log(`✅ No cleanup needed (${allQueries.length} queries)`);
      return { deletedCount: 0 };
    }

    const toDelete = allQueries.slice(keepCount);
    let deletedCount = 0;

    for (const query of toDelete) {
      await ctx.db.delete(query._id);
      deletedCount++;
    }

    console.log(`✅ Deleted ${deletedCount} old query records`);
    return { deletedCount };
  },
});

// Batch update embeddings for existing records without embeddings
export const batchUpdateMissingEmbeddings = mutation({
  args: {
    table: v.string(),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    console.log(`🔄 Finding records without embeddings in ${args.table}`);
    
    // This is a placeholder - actual implementation would depend on table structure
    // Each table would need its own specific implementation
    
    return {
      found: 0,
      updated: 0,
      message: `Batch embedding update for ${args.table} requires specific implementation`,
    };
  },
});