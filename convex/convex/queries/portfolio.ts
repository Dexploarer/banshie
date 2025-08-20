import { query } from "../_generated/server";
import { v } from "convex/values";

// Get user's complete portfolio with real-time updates
export const getPortfolio = query({
  args: { userId: v.id("users") },
  handler: async (ctx, args) => {
    // Get user
    const user = await ctx.db.get(args.userId);
    if (!user) throw new Error("User not found");

    // Get all wallets
    const wallets = await ctx.db
      .query("wallets")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .filter((q) => q.eq(q.field("isActive"), true))
      .collect();

    // Get all positions
    const positions = await ctx.db
      .query("positions")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .collect();

    // Calculate totals
    let totalValue = 0;
    let totalCost = 0;
    let totalPnL = 0;

    const enrichedPositions = positions.map((position) => {
      const marketValue = parseFloat(position.marketValue);
      const costBasis = parseFloat(position.costBasis);
      const pnlAmount = parseFloat(position.pnl.amount);

      totalValue += marketValue;
      totalCost += costBasis;
      totalPnL += pnlAmount;

      return {
        ...position,
        allocation: 0, // Will be calculated after total
      };
    });

    // Calculate allocations
    enrichedPositions.forEach((position) => {
      position.allocation = totalValue > 0
        ? (parseFloat(position.marketValue) / totalValue) * 100
        : 0;
    });

    // Sort by market value
    enrichedPositions.sort((a, b) => 
      parseFloat(b.marketValue) - parseFloat(a.marketValue)
    );

    return {
      user: {
        id: user._id,
        username: user.username,
        isPremium: user.isPremium,
        stats: user.stats,
      },
      wallets: wallets.map((w) => ({
        id: w._id,
        address: w.address,
        type: w.type,
        balance: w.balance,
        performance: w.performance,
      })),
      positions: enrichedPositions,
      summary: {
        totalValue: totalValue.toFixed(2),
        totalCost: totalCost.toFixed(2),
        totalPnL: totalPnL.toFixed(2),
        totalPnLPercentage: totalCost > 0
          ? ((totalPnL / totalCost) * 100).toFixed(2)
          : "0",
        positionCount: positions.length,
        lastUpdated: Date.now(),
      },
    };
  },
});

// Get specific position details
export const getPosition = query({
  args: { 
    positionId: v.id("positions"),
    includeHistory: v.optional(v.boolean()),
  },
  handler: async (ctx, args) => {
    const position = await ctx.db.get(args.positionId);
    if (!position) throw new Error("Position not found");

    // Get recent trades for this position
    let trades = [];
    if (args.includeHistory) {
      trades = await ctx.db
        .query("trades")
        .withIndex("by_token_out", (q) => 
          q.eq("tokenOut.mint", position.tokenMint)
        )
        .order("desc")
        .take(10)
        .collect();
    }

    // Get current price feed
    const priceFeed = await ctx.db
      .query("priceFeeds")
      .withIndex("by_token", (q) => q.eq("tokenMint", position.tokenMint))
      .order("desc")
      .first();

    return {
      position,
      currentPrice: priceFeed?.price || position.currentPrice,
      priceChange24h: priceFeed?.changes.price24h || 0,
      volume24h: priceFeed?.metrics.volume24h || "0",
      trades,
      lastPriceUpdate: priceFeed?.timestamp || Date.now(),
    };
  },
});

// Watch portfolio value changes
export const watchPortfolioValue = query({
  args: { 
    userId: v.id("users"),
    interval: v.optional(v.literal("1m", "5m", "1h", "1d")),
  },
  handler: async (ctx, args) => {
    const positions = await ctx.db
      .query("positions")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .collect();

    // Calculate current total value
    const currentValue = positions.reduce((sum, pos) => 
      sum + parseFloat(pos.marketValue), 0
    );

    // Get historical snapshots (would be stored separately)
    // For now, return current value with metadata
    return {
      current: currentValue.toFixed(2),
      change24h: {
        amount: "0", // Would calculate from historical data
        percentage: 0,
      },
      high24h: currentValue.toFixed(2),
      low24h: currentValue.toFixed(2),
      positions: positions.length,
      timestamp: Date.now(),
    };
  },
});

// Get top performers
export const getTopPerformers = query({
  args: { 
    userId: v.id("users"),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit || 5;
    
    const positions = await ctx.db
      .query("positions")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .collect();

    // Sort by PnL percentage
    const sorted = positions.sort((a, b) => 
      b.pnl.percentage - a.pnl.percentage
    );

    const top = sorted.slice(0, limit);
    const bottom = sorted.slice(-limit).reverse();

    return {
      gainers: top.filter(p => p.pnl.percentage > 0),
      losers: bottom.filter(p => p.pnl.percentage < 0),
    };
  },
});

// Get wallet performance
export const getWalletPerformance = query({
  args: { walletId: v.id("wallets") },
  handler: async (ctx, args) => {
    const wallet = await ctx.db.get(args.walletId);
    if (!wallet) throw new Error("Wallet not found");

    // Get positions for this wallet
    const positions = await ctx.db
      .query("positions")
      .withIndex("by_wallet", (q) => q.eq("walletId", args.walletId))
      .collect();

    // Get recent trades
    const trades = await ctx.db
      .query("trades")
      .withIndex("by_wallet", (q) => q.eq("walletId", args.walletId))
      .order("desc")
      .take(20)
      .collect();

    // Calculate metrics
    const totalValue = positions.reduce((sum, pos) => 
      sum + parseFloat(pos.marketValue), 0
    );

    const unrealizedPnL = positions.reduce((sum, pos) => 
      sum + parseFloat(pos.pnl.amount), 0
    );

    const realizedPnL = trades
      .filter(t => t.pnl)
      .reduce((sum, t) => sum + parseFloat(t.pnl!.realized), 0);

    return {
      wallet: {
        address: wallet.address,
        type: wallet.type,
        balance: wallet.balance,
      },
      performance: {
        totalValue: totalValue.toFixed(2),
        unrealizedPnL: unrealizedPnL.toFixed(2),
        realizedPnL: realizedPnL.toFixed(2),
        totalPnL: (unrealizedPnL + realizedPnL).toFixed(2),
        winRate: calculateWinRate(trades),
        avgHoldTime: calculateAvgHoldTime(positions),
      },
      positions: positions.length,
      trades: trades.length,
    };
  },
});

// Helper functions
function calculateWinRate(trades: any[]): number {
  const completedTrades = trades.filter(t => t.pnl);
  if (completedTrades.length === 0) return 0;
  
  const wins = completedTrades.filter(t => parseFloat(t.pnl.realized) > 0);
  return (wins.length / completedTrades.length) * 100;
}

function calculateAvgHoldTime(positions: any[]): number {
  if (positions.length === 0) return 0;
  
  const totalHoldTime = positions.reduce((sum, pos) => 
    sum + pos.analytics.holdTime, 0
  );
  
  return totalHoldTime / positions.length;
}