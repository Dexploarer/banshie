import { action } from "../_generated/server";
import { api } from "../_generated/api";
import { v } from "convex/values";

// OpenAI client for generating embeddings
const OPENAI_API_KEY = process.env.OPENAI_API_KEY!;

interface EmbeddingResponse {
  object: string;
  data: Array<{
    object: string;
    embedding: number[];
    index: number;
  }>;
  model: string;
  usage: {
    prompt_tokens: number;
    total_tokens: number;
  };
}

// Generate embeddings using OpenAI
export const generateEmbedding = action({
  args: {
    text: v.string(),
    model: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const model = args.model || "text-embedding-3-small";
    
    console.log(`🔍 Generating embedding for text: ${args.text.slice(0, 100)}...`);
    
    try {
      const response = await fetch("https://api.openai.com/v1/embeddings", {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${OPENAI_API_KEY}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: model,
          input: args.text,
          encoding_format: "float",
        }),
      });

      if (!response.ok) {
        throw new Error(`OpenAI API error: ${response.status} ${response.statusText}`);
      }

      const data: EmbeddingResponse = await response.json();
      const embedding = data.data[0].embedding;

      console.log(`✅ Generated embedding with ${embedding.length} dimensions`);
      
      return {
        embedding,
        model,
        tokens: data.usage.total_tokens,
      };
    } catch (error) {
      console.error("❌ Error generating embedding:", error);
      throw new Error(`Failed to generate embedding: ${error}`);
    }
  },
});

// Semantic search across AI analysis
export const searchSimilarAnalysis = action({
  args: {
    query: v.string(),
    targetId: v.optional(v.string()),
    analysisType: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    console.log(`🔍 Searching similar analysis for: "${args.query}"`);
    
    // Generate embedding for query
    const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
      text: args.query,
    });
    
    // Build filter conditions
    const filter: any = {};
    if (args.targetId) filter.targetId = args.targetId;
    if (args.analysisType) filter.type = args.analysisType;
    
    // Perform vector search
    const results = await ctx.vectorSearch("aiAnalysis", "by_analysis_embedding", {
      vector: embeddingResult.embedding,
      limit,
      filter: (q) => {
        let query = q;
        if (args.targetId) query = query.eq("targetId", args.targetId);
        if (args.analysisType) query = query.eq("type", args.analysisType);
        return query;
      },
    });

    console.log(`✅ Found ${results.length} similar analysis results`);

    return {
      query: args.query,
      results: results.map(result => ({
        id: result._id,
        score: result._score,
        analysis: result.analysis,
        targetId: result.targetId,
        type: result.type,
        timestamp: result.timestamp,
      })),
      embedding: embeddingResult.embedding,
    };
  },
});

// Find similar trading signals
export const findSimilarSignals = action({
  args: {
    description: v.string(),
    tokenMint: v.optional(v.string()),
    signalType: v.optional(v.string()),
    action: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    console.log(`🔍 Finding similar signals for: "${args.description}"`);
    
    // Generate embedding for description
    const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
      text: args.description,
    });
    
    // Perform vector search
    const results = await ctx.vectorSearch("tradingSignals", "by_signal_embedding", {
      vector: embeddingResult.embedding,
      limit,
      filter: (q) => {
        let query = q;
        if (args.tokenMint) query = query.eq("tokenMint", args.tokenMint);
        if (args.signalType) query = query.eq("signalType", args.signalType);
        if (args.action) query = query.eq("action", args.action);
        return query;
      },
    });

    console.log(`✅ Found ${results.length} similar trading signals`);

    return {
      description: args.description,
      results: results.map(result => ({
        id: result._id,
        score: result._score,
        tokenMint: result.tokenMint,
        symbol: result.symbol,
        signalType: result.signalType,
        action: result.action,
        strength: result.strength,
        confidence: result.confidence,
        reasoning: result.reasoning,
        performance: result.performance,
        createdAt: result.createdAt,
      })),
    };
  },
});

// Search knowledge base semantically
export const searchKnowledgeBase = action({
  args: {
    query: v.string(),
    category: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    console.log(`🔍 Searching knowledge base for: "${args.query}"`);
    
    // Generate embedding for query
    const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
      text: args.query,
    });
    
    // Perform vector search
    const results = await ctx.vectorSearch("knowledgeBase", "by_content_embedding", {
      vector: embeddingResult.embedding,
      limit,
      filter: (q) => {
        let query = q;
        if (args.category) query = query.eq("category", args.category);
        return query;
      },
    });

    console.log(`✅ Found ${results.length} knowledge base results`);

    return {
      query: args.query,
      results: results.map(result => ({
        id: result._id,
        score: result._score,
        category: result.category,
        title: result.title,
        content: result.content,
        metadata: result.metadata,
        createdAt: result.createdAt,
      })),
    };
  },
});

// Find similar market events
export const findSimilarEvents = action({
  args: {
    description: v.string(),
    eventType: v.optional(v.string()),
    severity: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    console.log(`🔍 Finding similar market events for: "${args.description}"`);
    
    // Generate embedding for description
    const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
      text: args.description,
    });
    
    // Perform vector search
    const results = await ctx.vectorSearch("marketEvents", "by_content_embedding", {
      vector: embeddingResult.embedding,
      limit,
      filter: (q) => {
        let query = q;
        if (args.eventType) query = query.eq("type", args.eventType);
        if (args.severity) query = query.eq("severity", args.severity);
        return query;
      },
    });

    console.log(`✅ Found ${results.length} similar market events`);

    return {
      description: args.description,
      results: results.map(result => ({
        id: result._id,
        score: result._score,
        type: result.type,
        severity: result.severity,
        title: result.title,
        description: result.description,
        affectedTokens: result.affectedTokens,
        impact: result.impact,
        timestamp: result.timestamp,
      })),
    };
  },
});

// Find similar user queries (for improving responses)
export const findSimilarQueries = action({
  args: {
    query: v.string(),
    userId: v.optional(v.id("users")),
    queryType: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    
    console.log(`🔍 Finding similar queries for: "${args.query}"`);
    
    // Generate embedding for query
    const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
      text: args.query,
    });
    
    // Perform vector search
    const results = await ctx.vectorSearch("userQueries", "by_query_embedding", {
      vector: embeddingResult.embedding,
      limit,
      filter: (q) => {
        let query = q;
        if (args.userId) query = query.eq("userId", args.userId);
        if (args.queryType) query = query.eq("queryType", args.queryType);
        return query;
      },
    });

    console.log(`✅ Found ${results.length} similar user queries`);

    return {
      query: args.query,
      results: results.map(result => ({
        id: result._id,
        score: result._score,
        query: result.query,
        queryType: result.queryType,
        intent: result.intent,
        tokenMints: result.tokenMints,
        results: result.results,
        satisfaction: result.satisfaction,
        timestamp: result.timestamp,
      })),
    };
  },
});

// Find similar conversations for context
export const findSimilarConversations = action({
  args: {
    content: v.string(),
    userId: v.optional(v.id("users")),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 5;
    
    console.log(`🔍 Finding similar conversations for: "${args.content.slice(0, 50)}..."`);
    
    // Generate embedding for content
    const embeddingResult = await ctx.runAction(api.actions.vector_search.generateEmbedding, {
      text: args.content,
    });
    
    // Perform vector search
    const results = await ctx.vectorSearch("conversations", "by_conversation_embedding", {
      vector: embeddingResult.embedding,
      limit,
      filter: (q) => {
        let query = q;
        if (args.userId) query = query.eq("userId", args.userId);
        return query;
      },
    });

    console.log(`✅ Found ${results.length} similar conversations`);

    return {
      content: args.content,
      results: results.map(result => ({
        id: result._id,
        score: result._score,
        sessionId: result.sessionId,
        summary: result.summary,
        messageCount: result.messageCount,
        startedAt: result.startedAt,
        lastMessageAt: result.lastMessageAt,
      })),
    };
  },
});

// Comprehensive AI-powered semantic search
export const intelligentSearch = action({
  args: {
    query: v.string(),
    userId: v.optional(v.id("users")),
    categories: v.optional(v.array(v.string())), // ["analysis", "signals", "knowledge", "events"]
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 20;
    const categories = args.categories || ["analysis", "signals", "knowledge", "events"];
    
    console.log(`🧠 Performing intelligent search for: "${args.query}"`);
    
    const results: any = {
      query: args.query,
      categories: {},
    };

    // Search across different categories in parallel
    const searchPromises = [];

    if (categories.includes("analysis")) {
      searchPromises.push(
        ctx.runAction(api.actions.vector_search.searchSimilarAnalysis, {
          query: args.query,
          limit: Math.ceil(limit / categories.length),
        }).then(result => ({ category: "analysis", data: result }))
      );
    }

    if (categories.includes("signals")) {
      searchPromises.push(
        ctx.runAction(api.actions.vector_search.findSimilarSignals, {
          description: args.query,
          limit: Math.ceil(limit / categories.length),
        }).then(result => ({ category: "signals", data: result }))
      );
    }

    if (categories.includes("knowledge")) {
      searchPromises.push(
        ctx.runAction(api.actions.vector_search.searchKnowledgeBase, {
          query: args.query,
          limit: Math.ceil(limit / categories.length),
        }).then(result => ({ category: "knowledge", data: result }))
      );
    }

    if (categories.includes("events")) {
      searchPromises.push(
        ctx.runAction(api.actions.vector_search.findSimilarEvents, {
          description: args.query,
          limit: Math.ceil(limit / categories.length),
        }).then(result => ({ category: "events", data: result }))
      );
    }

    // Execute all searches in parallel
    const searchResults = await Promise.all(searchPromises);

    // Organize results by category
    for (const result of searchResults) {
      results.categories[result.category] = result.data;
    }

    // Store the user query for learning
    if (args.userId) {
      await ctx.runMutation(api.mutations.ai.storeUserQuery, {
        userId: args.userId,
        query: args.query,
        queryType: "intelligent_search",
        results: searchResults.map(r => ({
          type: r.category,
          id: "search_result",
          relevanceScore: r.data.results?.[0]?.score || 0,
        })),
      });
    }

    console.log(`✅ Intelligent search completed across ${categories.length} categories`);

    return results;
  },
});

// Get AI context for a specific token
export const getTokenContext = action({
  args: {
    tokenMint: v.string(),
    context: v.optional(v.string()), // Additional context to search for
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 15;
    
    console.log(`🪙 Getting AI context for token: ${args.tokenMint}`);
    
    // Get token info first
    const tokenInfo = await ctx.runQuery(api.queries.prices.getTokenPrice, {
      mint: args.tokenMint,
    });

    if (!tokenInfo) {
      throw new Error(`Token not found: ${args.tokenMint}`);
    }

    const searchQuery = args.context 
      ? `${tokenInfo.symbol} ${tokenInfo.name} ${args.context}` 
      : `${tokenInfo.symbol} ${tokenInfo.name} trading analysis`;

    // Search across all categories for this token
    const [analysis, signals, knowledge, events] = await Promise.all([
      ctx.runAction(api.actions.vector_search.searchSimilarAnalysis, {
        query: searchQuery,
        targetId: args.tokenMint,
        limit: 5,
      }),
      ctx.runAction(api.actions.vector_search.findSimilarSignals, {
        description: searchQuery,
        tokenMint: args.tokenMint,
        limit: 5,
      }),
      ctx.runAction(api.actions.vector_search.searchKnowledgeBase, {
        query: searchQuery,
        limit: 3,
      }),
      ctx.runAction(api.actions.vector_search.findSimilarEvents, {
        description: searchQuery,
        limit: 2,
      }),
    ]);

    console.log(`✅ Retrieved comprehensive context for ${tokenInfo.symbol}`);

    return {
      token: tokenInfo,
      context: {
        analysis: analysis.results,
        signals: signals.results,
        knowledge: knowledge.results,
        events: events.results,
      },
      searchQuery,
      totalResults: analysis.results.length + signals.results.length + knowledge.results.length + events.results.length,
    };
  },
});

// Batch process embeddings for existing data
export const batchGenerateEmbeddings = action({
  args: {
    table: v.string(),
    field: v.string(),
    embeddingField: v.string(),
    limit: v.optional(v.number()),
    offset: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 10;
    const offset = args.offset || 0;
    
    console.log(`🔄 Batch generating embeddings for ${args.table}.${args.field}`);
    
    // This would need to be implemented based on the specific table structure
    // For now, return a placeholder
    
    return {
      processed: 0,
      total: 0,
      message: "Batch embedding generation requires table-specific implementation",
    };
  },
});