import { query } from "../_generated/server";
import { v } from "convex/values";

// Get latest AI analysis for a target
export const getLatestAnalysis = query({
  args: {
    targetId: v.string(),
    type: v.optional(v.union(
      v.literal("sentiment"),
      v.literal("technical"),
      v.literal("fundamental"),
      v.literal("prediction")
    )),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 5;
    
    let query = ctx.db
      .query("aiAnalysis")
      .withIndex("by_target", (q) => q.eq("targetId", args.targetId));

    if (args.type) {
      query = ctx.db
        .query("aiAnalysis")
        .withIndex("by_target_type", (q) => 
          q.eq("targetId", args.targetId).eq("type", args.type)
        );
    }

    const analysis = await query
      .order("desc")
      .take(limit);

    // Filter out expired analysis
    const now = Date.now();
    const validAnalysis = analysis.filter(a => !a.expiresAt || a.expiresAt > now);

    return validAnalysis.map(a => ({
      id: a._id,
      targetId: a.targetId,
      type: a.type,
      analysis: a.analysis,
      sources: a.sources,
      model: a.model,
      timestamp: a.timestamp,
      expiresAt: a.expiresAt,
    }));
  },
});

// Get latest trading signals
export const getLatestSignals = query({
  args: {
    tokenMint: v.optional(v.string()),
    action: v.optional(v.string()),
    signalType: v.optional(v.string()),
    minConfidence: v.optional(v.number()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    const minConfidence = args.minConfidence || 0;
    
    let query = ctx.db.query("tradingSignals");
    
    if (args.tokenMint) {
      query = query.withIndex("by_token", (q) => q.eq("tokenMint", args.tokenMint));
    } else if (args.action) {
      query = query.withIndex("by_action", (q) => q.eq("action", args.action));
    } else if (args.signalType) {
      query = query.withIndex("by_type", (q) => q.eq("signalType", args.signalType));
    } else {
      query = query.withIndex("by_created");
    }

    const signals = await query
      .order("desc")
      .filter((q) => 
        q.and(
          q.gte(q.field("validUntil"), Date.now()),
          q.gte(q.field("confidence"), minConfidence)
        )
      )
      .take(limit);

    return signals.map(s => ({
      id: s._id,
      tokenMint: s.tokenMint,
      symbol: s.symbol,
      signalType: s.signalType,
      action: s.action,
      strength: s.strength,
      confidence: s.confidence,
      reasoning: s.reasoning,
      technicalFactors: s.technicalFactors,
      fundamentalFactors: s.fundamentalFactors,
      sentimentFactors: s.sentimentFactors,
      priceTarget: s.priceTarget,
      stopLoss: s.stopLoss,
      timeframe: s.timeframe,
      riskLevel: s.riskLevel,
      validUntil: s.validUntil,
      createdAt: s.createdAt,
      performance: s.performance,
    }));
  },
});

// Get signal performance statistics
export const getSignalPerformance = query({
  args: {
    tokenMint: v.optional(v.string()),
    signalType: v.optional(v.string()),
    timeframe: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 100;
    
    let query = ctx.db.query("tradingSignals");
    
    if (args.tokenMint) {
      query = query.withIndex("by_token", (q) => q.eq("tokenMint", args.tokenMint));
    } else if (args.signalType) {
      query = query.withIndex("by_type", (q) => q.eq("signalType", args.signalType));
    } else {
      query = query.withIndex("by_created");
    }

    const signals = await query
      .order("desc")
      .filter((q) => q.neq(q.field("performance"), undefined))
      .take(limit);

    let totalSignals = 0;
    let executedSignals = 0;
    let profitableSignals = 0;
    let totalReturn = 0;
    let maxReturn = -Infinity;
    let minReturn = Infinity;

    const performanceByTimeframe: { [key: string]: any } = {};
    const performanceByType: { [key: string]: any } = {};

    for (const signal of signals) {
      if (!signal.performance?.executed) continue;

      totalSignals++;
      executedSignals++;

      const returnPct = signal.performance.returnPct || 0;
      totalReturn += returnPct;

      if (returnPct > 0) profitableSignals++;
      if (returnPct > maxReturn) maxReturn = returnPct;
      if (returnPct < minReturn) minReturn = returnPct;

      // Group by timeframe
      const timeframe = signal.timeframe;
      if (!performanceByTimeframe[timeframe]) {
        performanceByTimeframe[timeframe] = { count: 0, totalReturn: 0, profitable: 0 };
      }
      performanceByTimeframe[timeframe].count++;
      performanceByTimeframe[timeframe].totalReturn += returnPct;
      if (returnPct > 0) performanceByTimeframe[timeframe].profitable++;

      // Group by signal type
      const type = signal.signalType;
      if (!performanceByType[type]) {
        performanceByType[type] = { count: 0, totalReturn: 0, profitable: 0 };
      }
      performanceByType[type].count++;
      performanceByType[type].totalReturn += returnPct;
      if (returnPct > 0) performanceByType[type].profitable++;
    }

    const winRate = executedSignals > 0 ? (profitableSignals / executedSignals) * 100 : 0;
    const avgReturn = executedSignals > 0 ? totalReturn / executedSignals : 0;

    return {
      overall: {
        totalSignals: signals.length,
        executedSignals,
        winRate: Math.round(winRate * 100) / 100,
        avgReturn: Math.round(avgReturn * 100) / 100,
        totalReturn: Math.round(totalReturn * 100) / 100,
        maxReturn: maxReturn === -Infinity ? 0 : Math.round(maxReturn * 100) / 100,
        minReturn: minReturn === Infinity ? 0 : Math.round(minReturn * 100) / 100,
      },
      byTimeframe: Object.entries(performanceByTimeframe).map(([timeframe, stats]: [string, any]) => ({
        timeframe,
        count: stats.count,
        winRate: Math.round((stats.profitable / stats.count) * 10000) / 100,
        avgReturn: Math.round((stats.totalReturn / stats.count) * 100) / 100,
      })),
      byType: Object.entries(performanceByType).map(([type, stats]: [string, any]) => ({
        signalType: type,
        count: stats.count,
        winRate: Math.round((stats.profitable / stats.count) * 10000) / 100,
        avgReturn: Math.round((stats.totalReturn / stats.count) * 100) / 100,
      })),
    };
  },
});

// Get knowledge base entries
export const getKnowledgeEntries = query({
  args: {
    category: v.optional(v.string()),
    tags: v.optional(v.array(v.string())),
    tokenMints: v.optional(v.array(v.string())),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 20;
    
    let query = ctx.db.query("knowledgeBase");
    
    if (args.category) {
      query = query.withIndex("by_category", (q) => q.eq("category", args.category));
    } else {
      query = query.withIndex("by_updated");
    }

    let entries = await query
      .order("desc")
      .take(limit * 2); // Take more to allow for filtering

    // Filter by tags if specified
    if (args.tags && args.tags.length > 0) {
      entries = entries.filter(entry => 
        args.tags!.some(tag => entry.metadata.tags.includes(tag))
      );
    }

    // Filter by token mints if specified
    if (args.tokenMints && args.tokenMints.length > 0) {
      entries = entries.filter(entry => 
        entry.metadata.tokenMints &&
        args.tokenMints!.some(mint => entry.metadata.tokenMints!.includes(mint))
      );
    }

    // Take only the requested limit after filtering
    entries = entries.slice(0, limit);

    return entries.map(entry => ({
      id: entry._id,
      category: entry.category,
      title: entry.title,
      content: entry.content,
      metadata: entry.metadata,
      createdAt: entry.createdAt,
      updatedAt: entry.updatedAt,
    }));
  },
});

// Get user query history
export const getUserQueryHistory = query({
  args: {
    userId: v.id("users"),
    queryType: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 50;
    
    let query = ctx.db
      .query("userQueries")
      .withIndex("by_user_timestamp", (q) => q.eq("userId", args.userId))
      .order("desc");

    if (args.queryType) {
      query = ctx.db
        .query("userQueries")
        .withIndex("by_user", (q) => q.eq("userId", args.userId))
        .filter((q) => q.eq(q.field("queryType"), args.queryType))
        .order("desc");
    }

    const queries = await query.take(limit);

    return queries.map(q => ({
      id: q._id,
      query: q.query,
      queryType: q.queryType,
      intent: q.intent,
      tokenMints: q.tokenMints,
      results: q.results,
      satisfaction: q.satisfaction,
      timestamp: q.timestamp,
    }));
  },
});

// Get user conversations
export const getUserConversations = query({
  args: {
    userId: v.id("users"),
    sessionId: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    let query = ctx.db
      .query("conversations")
      .withIndex("by_user_last_message", (q) => q.eq("userId", args.userId))
      .order("desc");

    if (args.sessionId) {
      query = ctx.db
        .query("conversations")
        .withIndex("by_session", (q) => q.eq("sessionId", args.sessionId));
    }

    const conversations = await query.take(limit);

    return conversations.map(conv => ({
      id: conv._id,
      sessionId: conv.sessionId,
      messages: conv.messages,
      summary: conv.summary,
      startedAt: conv.startedAt,
      lastMessageAt: conv.lastMessageAt,
      messageCount: conv.messageCount,
    }));
  },
});

// Get market events
export const getMarketEvents = query({
  args: {
    type: v.optional(v.union(
      v.literal("listing"),
      v.literal("delisting"),
      v.literal("hack"),
      v.literal("update"),
      v.literal("partnership"),
      v.literal("regulation")
    )),
    severity: v.optional(v.union(
      v.literal("info"),
      v.literal("low"),
      v.literal("medium"),
      v.literal("high"),
      v.literal("critical")
    )),
    affectedToken: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 20;
    
    let query = ctx.db.query("marketEvents");
    
    if (args.type && args.severity) {
      query = query.withIndex("by_type_severity", (q) => 
        q.eq("type", args.type).eq("severity", args.severity)
      );
    } else if (args.type) {
      query = query.withIndex("by_type", (q) => q.eq("type", args.type));
    } else if (args.severity) {
      query = query.withIndex("by_severity", (q) => q.eq("severity", args.severity));
    } else {
      query = query.withIndex("by_timestamp");
    }

    let events = await query
      .order("desc")
      .take(limit * 2); // Take more to allow for token filtering

    // Filter by affected token if specified
    if (args.affectedToken) {
      events = events.filter(event => 
        event.affectedTokens.includes(args.affectedToken!)
      );
      events = events.slice(0, limit);
    }

    return events.slice(0, limit).map(event => ({
      id: event._id,
      type: event.type,
      severity: event.severity,
      title: event.title,
      description: event.description,
      affectedTokens: event.affectedTokens,
      source: event.source,
      impact: event.impact,
      timestamp: event.timestamp,
    }));
  },
});

// Get AI analysis statistics
export const getAnalysisStats = query({
  args: {
    targetId: v.optional(v.string()),
    timeframe: v.optional(v.number()), // milliseconds back from now
  },
  handler: async (ctx, args) => {
    const timeframe = args.timeframe || 7 * 24 * 60 * 60 * 1000; // 7 days default
    const since = Date.now() - timeframe;
    
    let query = ctx.db.query("aiAnalysis")
      .withIndex("by_timestamp", (q) => q.gte("timestamp", since));
    
    if (args.targetId) {
      query = ctx.db.query("aiAnalysis")
        .withIndex("by_target", (q) => q.eq("targetId", args.targetId))
        .filter((q) => q.gte(q.field("timestamp"), since));
    }

    const analysis = await query.collect();
    
    const stats = {
      total: analysis.length,
      byType: {} as { [key: string]: number },
      byRecommendation: {} as { [key: string]: number },
      avgConfidence: 0,
      avgScore: 0,
      recentTrend: {} as { [key: string]: number },
    };

    let totalConfidence = 0;
    let totalScore = 0;
    
    for (const item of analysis) {
      // Count by type
      stats.byType[item.type] = (stats.byType[item.type] || 0) + 1;
      
      // Count by recommendation
      const rec = item.analysis.recommendation;
      stats.byRecommendation[rec] = (stats.byRecommendation[rec] || 0) + 1;
      
      // Calculate averages
      totalConfidence += item.analysis.confidence;
      totalScore += item.analysis.score;
    }
    
    if (analysis.length > 0) {
      stats.avgConfidence = Math.round((totalConfidence / analysis.length) * 1000) / 1000;
      stats.avgScore = Math.round((totalScore / analysis.length) * 100) / 100;
    }

    // Calculate recent trend (last 24h vs previous 24h)
    const oneDayAgo = Date.now() - (24 * 60 * 60 * 1000);
    const recent = analysis.filter(a => a.timestamp >= oneDayAgo);
    const previous = analysis.filter(a => a.timestamp < oneDayAgo && a.timestamp >= (oneDayAgo - (24 * 60 * 60 * 1000)));
    
    stats.recentTrend = {
      recent: recent.length,
      previous: previous.length,
      change: recent.length - previous.length,
      changePercent: previous.length > 0 ? Math.round(((recent.length - previous.length) / previous.length) * 10000) / 100 : 0,
    };

    return stats;
  },
});

// Get comprehensive AI context for a token
export const getTokenAIContext = query({
  args: {
    tokenMint: v.string(),
    includeExpired: v.optional(v.boolean()),
  },
  handler: async (ctx, args) => {
    const includeExpired = args.includeExpired || false;
    const now = Date.now();
    
    // Get all analysis for this token
    let analysis = await ctx.db
      .query("aiAnalysis")
      .withIndex("by_target", (q) => q.eq("targetId", args.tokenMint))
      .order("desc")
      .take(20);

    if (!includeExpired) {
      analysis = analysis.filter(a => !a.expiresAt || a.expiresAt > now);
    }

    // Get trading signals
    const signals = await ctx.db
      .query("tradingSignals")
      .withIndex("by_token", (q) => q.eq("tokenMint", args.tokenMint))
      .order("desc")
      .filter((q) => includeExpired ? q.gt(q.field("createdAt"), 0) : q.gte(q.field("validUntil"), now))
      .take(10);

    // Get related market events
    const events = await ctx.db
      .query("marketEvents")
      .withIndex("by_timestamp")
      .order("desc")
      .filter((q) => q.gt(q.field("timestamp"), now - (30 * 24 * 60 * 60 * 1000))) // Last 30 days
      .take(50);

    const relevantEvents = events.filter(event => 
      event.affectedTokens.includes(args.tokenMint)
    );

    return {
      tokenMint: args.tokenMint,
      analysis: analysis.map(a => ({
        id: a._id,
        type: a.type,
        analysis: a.analysis,
        timestamp: a.timestamp,
        expiresAt: a.expiresAt,
        sources: a.sources,
        model: a.model,
      })),
      signals: signals.map(s => ({
        id: s._id,
        signalType: s.signalType,
        action: s.action,
        strength: s.strength,
        confidence: s.confidence,
        reasoning: s.reasoning,
        timeframe: s.timeframe,
        riskLevel: s.riskLevel,
        priceTarget: s.priceTarget,
        stopLoss: s.stopLoss,
        validUntil: s.validUntil,
        createdAt: s.createdAt,
        performance: s.performance,
      })),
      events: relevantEvents.map(e => ({
        id: e._id,
        type: e.type,
        severity: e.severity,
        title: e.title,
        description: e.description,
        impact: e.impact,
        timestamp: e.timestamp,
        source: e.source,
      })),
      summary: {
        totalAnalysis: analysis.length,
        totalSignals: signals.length,
        totalEvents: relevantEvents.length,
        lastAnalysis: analysis[0]?.timestamp,
        lastSignal: signals[0]?.createdAt,
        lastEvent: relevantEvents[0]?.timestamp,
      },
    };
  },
});