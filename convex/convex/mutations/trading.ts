import { mutation } from "../_generated/server";
import { v } from "convex/values";
import { Id } from "../_generated/dataModel";

// Place a new trade order
export const placeTrade = mutation({
  args: {
    userId: v.id("users"),
    walletId: v.id("wallets"),
    type: v.union(
      v.literal("market"),
      v.literal("limit"),
      v.literal("stop_loss"),
      v.literal("take_profit")
    ),
    side: v.union(v.literal("buy"), v.literal("sell")),
    tokenIn: v.object({
      mint: v.string(),
      symbol: v.string(),
      amount: v.string(),
      decimals: v.number(),
    }),
    tokenOut: v.object({
      mint: v.string(),
      symbol: v.string(),
      decimals: v.number(),
    }),
    slippage: v.number(),
    conditions: v.optional(v.object({
      triggerPrice: v.optional(v.string()),
      limitPrice: v.optional(v.string()),
      timeInForce: v.optional(v.string()),
    })),
  },
  handler: async (ctx, args) => {
    // Validate user exists and is active
    const user = await ctx.db.get(args.userId);
    if (!user || !user.isActive) {
      throw new Error("User not found or inactive");
    }

    // Validate wallet belongs to user
    const wallet = await ctx.db.get(args.walletId);
    if (!wallet || wallet.userId !== args.userId) {
      throw new Error("Invalid wallet");
    }

    // Check user limits
    const todayStart = new Date().setHours(0, 0, 0, 0);
    const todayOrders = await ctx.db
      .query("orders")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .filter((q) => q.gte(q.field("createdAt"), todayStart))
      .collect();

    if (todayOrders.length >= user.limits.dailyTrades) {
      throw new Error(`Daily trade limit (${user.limits.dailyTrades}) reached`);
    }

    // Check position limits
    const openPositions = await ctx.db
      .query("positions")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .collect();

    if (args.side === "buy" && openPositions.length >= user.limits.maxOpenPositions) {
      throw new Error(`Maximum open positions (${user.limits.maxOpenPositions}) reached`);
    }

    // Create the order
    const orderId = await ctx.db.insert("orders", {
      userId: args.userId,
      walletId: args.walletId,
      type: args.type,
      side: args.side,
      status: "pending",
      tokenIn: args.tokenIn,
      tokenOut: {
        ...args.tokenOut,
        amount: "0", // Will be calculated
      },
      pricing: {
        expectedPrice: "0", // Will be calculated
        slippage: args.slippage,
      },
      conditions: args.conditions,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    });

    // Update user activity
    await ctx.db.patch(args.userId, {
      lastActive: Date.now(),
    });

    // Schedule execution (would trigger external action)
    await ctx.scheduler.runAfter(0, "actions/trading:executeTrade", {
      orderId,
    });

    return { orderId, status: "submitted" };
  },
});

// Update order status
export const updateOrderStatus = mutation({
  args: {
    orderId: v.id("orders"),
    status: v.union(
      v.literal("submitted"),
      v.literal("executing"),
      v.literal("partial"),
      v.literal("completed"),
      v.literal("failed"),
      v.literal("cancelled")
    ),
    execution: v.optional(v.object({
      txSignature: v.string(),
      blockHeight: v.number(),
      gasUsed: v.string(),
      actualPrice: v.string(),
      actualOut: v.string(),
    })),
    error: v.optional(v.object({
      code: v.string(),
      message: v.string(),
      details: v.optional(v.string()),
    })),
  },
  handler: async (ctx, args) => {
    const order = await ctx.db.get(args.orderId);
    if (!order) throw new Error("Order not found");

    // Update order
    const updates: any = {
      status: args.status,
      updatedAt: Date.now(),
    };

    if (args.execution) {
      updates.execution = {
        ...args.execution,
        attempts: (order.execution?.attempts || 0) + 1,
      };
      updates.executedAt = Date.now();
      
      // Update pricing with actual values
      updates.pricing = {
        ...order.pricing,
        executionPrice: args.execution.actualPrice,
        priceImpact: calculatePriceImpact(
          order.pricing.expectedPrice,
          args.execution.actualPrice
        ),
      };
      
      // Update token out amount
      updates.tokenOut = {
        ...order.tokenOut,
        amount: args.execution.actualOut,
      };
    }

    if (args.error) {
      updates.error = args.error;
    }

    await ctx.db.patch(args.orderId, updates);

    // If completed, update positions
    if (args.status === "completed" && args.execution) {
      await handleCompletedOrder(ctx, order, args.execution);
    }

    return { success: true };
  },
});

// Cancel an order
export const cancelOrder = mutation({
  args: {
    orderId: v.id("orders"),
    userId: v.id("users"),
    reason: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const order = await ctx.db.get(args.orderId);
    if (!order) throw new Error("Order not found");

    // Verify ownership
    if (order.userId !== args.userId) {
      throw new Error("Unauthorized");
    }

    // Check if order can be cancelled
    if (!["pending", "submitted"].includes(order.status)) {
      throw new Error(`Cannot cancel order with status: ${order.status}`);
    }

    // Update order status
    await ctx.db.patch(args.orderId, {
      status: "cancelled",
      updatedAt: Date.now(),
      error: args.reason ? {
        code: "USER_CANCELLED",
        message: args.reason,
      } : undefined,
    });

    return { success: true };
  },
});

// Update or create position after trade
export const updatePosition = mutation({
  args: {
    userId: v.id("users"),
    walletId: v.id("wallets"),
    tokenMint: v.string(),
    symbol: v.string(),
    name: v.string(),
    amount: v.string(),
    price: v.string(),
    action: v.union(v.literal("add"), v.literal("remove"), v.literal("update")),
    metadata: v.optional(v.any()),
  },
  handler: async (ctx, args) => {
    // Find existing position
    const existing = await ctx.db
      .query("positions")
      .withIndex("by_user_token", (q) => 
        q.eq("userId", args.userId).eq("tokenMint", args.tokenMint)
      )
      .first();

    if (args.action === "add" || args.action === "update") {
      if (existing) {
        // Update existing position
        const currentAmount = parseFloat(existing.amount);
        const currentAvgPrice = parseFloat(existing.averagePrice);
        const newAmount = parseFloat(args.amount);
        const newPrice = parseFloat(args.price);

        let updatedAmount, updatedAvgPrice;

        if (args.action === "add") {
          // Calculate new average price
          updatedAmount = currentAmount + newAmount;
          updatedAvgPrice = 
            ((currentAmount * currentAvgPrice) + (newAmount * newPrice)) / updatedAmount;
        } else {
          // Just update current price
          updatedAmount = currentAmount;
          updatedAvgPrice = currentAvgPrice;
        }

        const marketValue = updatedAmount * newPrice;
        const costBasis = updatedAmount * updatedAvgPrice;
        const pnlAmount = marketValue - costBasis;
        const pnlPercentage = (pnlAmount / costBasis) * 100;

        await ctx.db.patch(existing._id, {
          amount: updatedAmount.toString(),
          averagePrice: updatedAvgPrice.toString(),
          currentPrice: args.price,
          marketValue: marketValue.toString(),
          costBasis: costBasis.toString(),
          pnl: {
            amount: pnlAmount.toString(),
            percentage: pnlPercentage,
            isProfit: pnlAmount > 0,
          },
          lastUpdated: Date.now(),
        });

        return { positionId: existing._id, action: "updated" };
      } else {
        // Create new position
        const amount = parseFloat(args.amount);
        const price = parseFloat(args.price);
        const marketValue = amount * price;
        const costBasis = marketValue;

        const positionId = await ctx.db.insert("positions", {
          userId: args.userId,
          walletId: args.walletId,
          tokenMint: args.tokenMint,
          symbol: args.symbol,
          name: args.name,
          amount: args.amount,
          decimals: 9, // Default, should be passed
          averagePrice: args.price,
          currentPrice: args.price,
          marketValue: marketValue.toString(),
          costBasis: costBasis.toString(),
          pnl: {
            amount: "0",
            percentage: 0,
            isProfit: false,
          },
          metadata: args.metadata || {},
          analytics: {
            priceChange24h: 0,
            volume24h: "0",
            marketCap: "0",
            holdTime: 0,
          },
          openedAt: Date.now(),
          lastUpdated: Date.now(),
        });

        return { positionId, action: "created" };
      }
    } else if (args.action === "remove" && existing) {
      const currentAmount = parseFloat(existing.amount);
      const removeAmount = parseFloat(args.amount);
      
      if (removeAmount >= currentAmount) {
        // Close position
        await ctx.db.delete(existing._id);
        return { positionId: existing._id, action: "closed" };
      } else {
        // Partial close
        const remainingAmount = currentAmount - removeAmount;
        const price = parseFloat(args.price);
        const marketValue = remainingAmount * price;
        const costBasis = remainingAmount * parseFloat(existing.averagePrice);
        const pnlAmount = marketValue - costBasis;
        const pnlPercentage = (pnlAmount / costBasis) * 100;

        await ctx.db.patch(existing._id, {
          amount: remainingAmount.toString(),
          currentPrice: args.price,
          marketValue: marketValue.toString(),
          costBasis: costBasis.toString(),
          pnl: {
            amount: pnlAmount.toString(),
            percentage: pnlPercentage,
            isProfit: pnlAmount > 0,
          },
          lastUpdated: Date.now(),
        });

        return { positionId: existing._id, action: "reduced" };
      }
    }

    throw new Error("Invalid position update");
  },
});

// Record completed trade
export const recordTrade = mutation({
  args: {
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
  },
  handler: async (ctx, args) => {
    // Record the trade
    const tradeId = await ctx.db.insert("trades", {
      ...args,
      metadata: {},
      timestamp: Date.now(),
    });

    // Update user stats
    const user = await ctx.db.get(args.userId);
    if (user) {
      const totalTrades = user.stats.totalTrades + 1;
      const totalVolume = (
        parseFloat(user.stats.totalVolume) + 
        parseFloat(args.tokenIn.value)
      ).toString();
      
      let totalPnL = user.stats.totalPnL;
      let successRate = user.stats.successRate;
      
      if (args.pnl) {
        totalPnL = (
          parseFloat(totalPnL) + 
          parseFloat(args.pnl.realized)
        ).toString();
        
        // Update success rate
        const successful = parseFloat(args.pnl.realized) > 0;
        successRate = ((successRate * (totalTrades - 1)) + (successful ? 1 : 0)) / totalTrades;
      }

      await ctx.db.patch(args.userId, {
        stats: {
          totalTrades,
          successRate,
          totalVolume,
          totalPnL,
        },
      });
    }

    // Update wallet performance
    const wallet = await ctx.db.get(args.walletId);
    if (wallet && args.pnl) {
      await ctx.db.patch(args.walletId, {
        performance: {
          ...wallet.performance,
          realizedPnL: (
            parseFloat(wallet.performance.realizedPnL) + 
            parseFloat(args.pnl.realized)
          ).toString(),
        },
      });
    }

    return { tradeId };
  },
});

// Helper functions
function calculatePriceImpact(expected: string, actual: string): number {
  const expectedPrice = parseFloat(expected);
  const actualPrice = parseFloat(actual);
  
  if (expectedPrice === 0) return 0;
  
  return ((actualPrice - expectedPrice) / expectedPrice) * 100;
}

async function handleCompletedOrder(ctx: any, order: any, execution: any) {
  // Update position based on order side
  if (order.side === "buy") {
    await ctx.runMutation("mutations/trading:updatePosition", {
      userId: order.userId,
      walletId: order.walletId,
      tokenMint: order.tokenOut.mint,
      symbol: order.tokenOut.symbol,
      name: order.tokenOut.symbol, // Would get full name
      amount: execution.actualOut,
      price: execution.actualPrice,
      action: "add",
    });
  } else {
    // Selling - reduce position
    await ctx.runMutation("mutations/trading:updatePosition", {
      userId: order.userId,
      walletId: order.walletId,
      tokenMint: order.tokenIn.mint,
      symbol: order.tokenIn.symbol,
      name: order.tokenIn.symbol,
      amount: order.tokenIn.amount,
      price: execution.actualPrice,
      action: "remove",
    });
  }
}