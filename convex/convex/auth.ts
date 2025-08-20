import { mutation, query } from "./_generated/server";
import { v } from "convex/values";
import crypto from "crypto";

// Authentication configuration
const SESSION_DURATION = 7 * 24 * 60 * 60 * 1000; // 7 days
const API_KEY_LENGTH = 32;

// Authenticate user with Telegram
export const authenticateWithTelegram = mutation({
  args: {
    telegramId: v.number(),
    username: v.string(),
    firstName: v.optional(v.string()),
    lastName: v.optional(v.string()),
    photoUrl: v.optional(v.string()),
    authDate: v.number(),
    hash: v.string(),
  },
  handler: async (ctx, args) => {
    // Verify Telegram auth hash
    if (!verifyTelegramAuth(args)) {
      throw new Error("Invalid Telegram authentication");
    }

    // Check if auth is not too old (5 minutes)
    const now = Date.now() / 1000;
    if (now - args.authDate > 300) {
      throw new Error("Authentication expired");
    }

    // Find or create user
    let user = await ctx.db
      .query("users")
      .withIndex("by_telegram", (q) => q.eq("telegramId", args.telegramId))
      .first();

    if (!user) {
      // Create new user
      const userId = await ctx.db.insert("users", {
        telegramId: args.telegramId,
        username: args.username,
        email: undefined,
        isActive: true,
        isPremium: false,
        settings: {
          defaultSlippage: 1.0,
          maxPositionSize: "100",
          autoCompound: false,
          riskLevel: "moderate",
          notifications: {
            trades: true,
            alerts: true,
            daily: false,
          },
        },
        limits: {
          dailyTrades: 10,
          maxOpenPositions: 20,
          maxOrderValue: "1000",
        },
        stats: {
          totalTrades: 0,
          successRate: 0,
          totalVolume: "0",
          totalPnL: "0",
        },
        createdAt: Date.now(),
        lastActive: Date.now(),
      });

      user = await ctx.db.get(userId);
    } else {
      // Update last active
      await ctx.db.patch(user._id, {
        lastActive: Date.now(),
      });
    }

    // Create session
    const sessionToken = generateSessionToken();
    const sessionId = await ctx.db.insert("sessions", {
      userId: user!._id,
      token: sessionToken,
      type: "telegram",
      device: undefined,
      isActive: true,
      lastActivity: Date.now(),
      expiresAt: Date.now() + SESSION_DURATION,
      createdAt: Date.now(),
    });

    return {
      user: {
        id: user!._id,
        username: user!.username,
        isPremium: user!.isPremium,
      },
      session: {
        id: sessionId,
        token: sessionToken,
        expiresAt: Date.now() + SESSION_DURATION,
      },
    };
  },
});

// Authenticate with API key
export const authenticateWithApiKey = mutation({
  args: {
    apiKey: v.string(),
  },
  handler: async (ctx, args) => {
    // Hash the API key for lookup
    const hashedKey = hashApiKey(args.apiKey);

    // Find user with this API key
    const user = await ctx.db
      .query("users")
      .filter((q) => q.eq(q.field("apiKeyHash"), hashedKey))
      .first();

    if (!user) {
      throw new Error("Invalid API key");
    }

    if (!user.isActive) {
      throw new Error("Account is inactive");
    }

    // Update last active
    await ctx.db.patch(user._id, {
      lastActive: Date.now(),
    });

    // Create API session
    const sessionToken = generateSessionToken();
    const sessionId = await ctx.db.insert("sessions", {
      userId: user._id,
      token: sessionToken,
      type: "api",
      device: undefined,
      isActive: true,
      lastActivity: Date.now(),
      expiresAt: Date.now() + SESSION_DURATION,
      createdAt: Date.now(),
    });

    return {
      user: {
        id: user._id,
        username: user.username,
        isPremium: user.isPremium,
      },
      session: {
        id: sessionId,
        token: sessionToken,
        expiresAt: Date.now() + SESSION_DURATION,
      },
    };
  },
});

// Validate session
export const validateSession = query({
  args: {
    sessionToken: v.string(),
  },
  handler: async (ctx, args) => {
    const session = await ctx.db
      .query("sessions")
      .withIndex("by_token", (q) => q.eq("token", args.sessionToken))
      .first();

    if (!session) {
      return { valid: false, reason: "Session not found" };
    }

    if (!session.isActive) {
      return { valid: false, reason: "Session inactive" };
    }

    if (Date.now() > session.expiresAt) {
      // Mark as inactive
      await ctx.db.patch(session._id, { isActive: false });
      return { valid: false, reason: "Session expired" };
    }

    // Get user
    const user = await ctx.db.get(session.userId);
    if (!user || !user.isActive) {
      return { valid: false, reason: "User not found or inactive" };
    }

    // Update last activity
    await ctx.db.patch(session._id, {
      lastActivity: Date.now(),
    });

    return {
      valid: true,
      user: {
        id: user._id,
        username: user.username,
        isPremium: user.isPremium,
        limits: user.limits,
      },
      session: {
        id: session._id,
        type: session.type,
        expiresAt: session.expiresAt,
      },
    };
  },
});

// Logout
export const logout = mutation({
  args: {
    sessionToken: v.string(),
  },
  handler: async (ctx, args) => {
    const session = await ctx.db
      .query("sessions")
      .withIndex("by_token", (q) => q.eq("token", args.sessionToken))
      .first();

    if (!session) {
      throw new Error("Session not found");
    }

    // Mark session as inactive
    await ctx.db.patch(session._id, {
      isActive: false,
      lastActivity: Date.now(),
    });

    return { success: true };
  },
});

// Generate API key for user
export const generateApiKey = mutation({
  args: {
    userId: v.id("users"),
    sessionToken: v.string(),
  },
  handler: async (ctx, args) => {
    // Validate session
    const session = await ctx.db
      .query("sessions")
      .withIndex("by_token", (q) => q.eq("token", args.sessionToken))
      .first();

    if (!session || session.userId !== args.userId) {
      throw new Error("Unauthorized");
    }

    // Generate new API key
    const apiKey = generateApiKeyString();
    const hashedKey = hashApiKey(apiKey);

    // Store hashed key
    await ctx.db.patch(args.userId, {
      apiKeyHash: hashedKey,
      apiKeyCreatedAt: Date.now(),
    });

    // Return the plain API key (only shown once)
    return {
      apiKey,
      warning: "Save this API key securely. It won't be shown again.",
    };
  },
});

// Revoke API key
export const revokeApiKey = mutation({
  args: {
    userId: v.id("users"),
    sessionToken: v.string(),
  },
  handler: async (ctx, args) => {
    // Validate session
    const session = await ctx.db
      .query("sessions")
      .withIndex("by_token", (q) => q.eq("token", args.sessionToken))
      .first();

    if (!session || session.userId !== args.userId) {
      throw new Error("Unauthorized");
    }

    // Remove API key
    await ctx.db.patch(args.userId, {
      apiKeyHash: undefined,
      apiKeyCreatedAt: undefined,
    });

    // Invalidate all API sessions for this user
    const apiSessions = await ctx.db
      .query("sessions")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .filter((q) => q.eq(q.field("type"), "api"))
      .collect();

    for (const session of apiSessions) {
      await ctx.db.patch(session._id, { isActive: false });
    }

    return { success: true };
  },
});

// Get active sessions
export const getActiveSessions = query({
  args: {
    userId: v.id("users"),
    sessionToken: v.string(),
  },
  handler: async (ctx, args) => {
    // Validate current session
    const currentSession = await ctx.db
      .query("sessions")
      .withIndex("by_token", (q) => q.eq("token", args.sessionToken))
      .first();

    if (!currentSession || currentSession.userId !== args.userId) {
      throw new Error("Unauthorized");
    }

    // Get all active sessions
    const sessions = await ctx.db
      .query("sessions")
      .withIndex("by_user_active", (q) => 
        q.eq("userId", args.userId).eq("isActive", true)
      )
      .collect();

    return sessions.map(s => ({
      id: s._id,
      type: s.type,
      device: s.device,
      lastActivity: s.lastActivity,
      createdAt: s.createdAt,
      expiresAt: s.expiresAt,
      isCurrent: s._id === currentSession._id,
    }));
  },
});

// Terminate session
export const terminateSession = mutation({
  args: {
    sessionId: v.id("sessions"),
    sessionToken: v.string(),
  },
  handler: async (ctx, args) => {
    // Validate current session
    const currentSession = await ctx.db
      .query("sessions")
      .withIndex("by_token", (q) => q.eq("token", args.sessionToken))
      .first();

    if (!currentSession) {
      throw new Error("Unauthorized");
    }

    // Get target session
    const targetSession = await ctx.db.get(args.sessionId);
    if (!targetSession || targetSession.userId !== currentSession.userId) {
      throw new Error("Session not found or unauthorized");
    }

    // Terminate session
    await ctx.db.patch(args.sessionId, {
      isActive: false,
      lastActivity: Date.now(),
    });

    return { success: true };
  },
});

// Helper functions
function verifyTelegramAuth(params: any): boolean {
  // In production, verify the hash using Telegram bot token
  const botToken = process.env.TELEGRAM_BOT_TOKEN;
  if (!botToken) return false;

  const secret = crypto.createHash("sha256")
    .update(botToken)
    .digest();

  const checkString = Object.keys(params)
    .filter(key => key !== "hash")
    .sort()
    .map(key => `${key}=${params[key]}`)
    .join("\n");

  const hash = crypto.createHmac("sha256", secret)
    .update(checkString)
    .digest("hex");

  return hash === params.hash;
}

function generateSessionToken(): string {
  return crypto.randomBytes(32).toString("hex");
}

function generateApiKeyString(): string {
  const prefix = "sk_live_";
  const key = crypto.randomBytes(API_KEY_LENGTH).toString("hex");
  return `${prefix}${key}`;
}

function hashApiKey(apiKey: string): string {
  return crypto.createHash("sha256")
    .update(apiKey)
    .digest("hex");
}

// Middleware helper for protected queries/mutations
export function requireAuth(handler: any) {
  return async (ctx: any, args: any) => {
    if (!args.sessionToken) {
      throw new Error("Authentication required");
    }

    const validation = await ctx.runQuery("auth:validateSession", {
      sessionToken: args.sessionToken,
    });

    if (!validation.valid) {
      throw new Error(`Authentication failed: ${validation.reason}`);
    }

    // Add user to context
    const authenticatedArgs = {
      ...args,
      _user: validation.user,
      _session: validation.session,
    };

    return handler(ctx, authenticatedArgs);
  };
}