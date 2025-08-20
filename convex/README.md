# Convex Backend Integration for Solana Trading Bot

## Overview

Convex provides a reactive, TypeScript-first backend that will replace our current database layer and provide real-time updates across all connected clients. This architecture perfectly suits our trading bot's need for instant price updates, order notifications, and portfolio synchronization.

## Architecture Benefits for Trading Bot

### 1. Real-Time Price Updates
- **Reactive Queries**: Price changes automatically propagate to all connected clients
- **WebSocket Management**: Built-in, no manual connection handling needed
- **Subscription Model**: Clients automatically receive updates when data changes

### 2. Type-Safe Trading Operations
- **End-to-End TypeScript**: From database schema to API calls
- **Automatic Validation**: Type checking prevents invalid trades
- **AI-Friendly**: LLMs can generate accurate code using TypeScript

### 3. Transaction Guarantees
- **ACID Compliance**: Critical for financial operations
- **Serializable Isolation**: Prevents race conditions in trading
- **Automatic Rollback**: Failed trades don't corrupt state

## Integration Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend Apps                        │
│  (Telegram Bot, Web Dashboard, Mobile App)                   │
└─────────────────┬───────────────────────────────────────────┘
                  │ WebSocket (Automatic)
┌─────────────────▼───────────────────────────────────────────┐
│                      Convex Backend                          │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              TypeScript Functions                     │   │
│  │  • Queries (getPrices, getPortfolio, getOrders)     │   │
│  │  • Mutations (placeTrade, updatePosition)           │   │
│  │  • Actions (callSolanaRPC, executeSwap)             │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Reactive Database                        │   │
│  │  • Users, Wallets, Positions                        │   │
│  │  • Orders, Trades, Transactions                     │   │
│  │  • Prices, Market Data, Alerts                      │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Built-in Features                        │   │
│  │  • Authentication & Sessions                         │   │
│  │  • File Storage (Charts, Reports)                   │   │
│  │  • Cron Jobs (DCA, Rebalancing)                    │   │
│  │  • Vector Search (AI Embeddings)                    │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────┬───────────────────────────────────────────┘
                  │ HTTP/RPC Calls
┌─────────────────▼───────────────────────────────────────────┐
│                   External Services                          │
│  • Solana RPC (Transaction Execution)                        │
│  • Jupiter API (Swap Routing)                                │
│  • Helius/Triton (Enhanced RPC)                              │
│  • Price Oracles (Pyth, Chainlink)                           │
└───────────────────────────────────────────────────────────┘
```

## Database Schema Design

### Core Tables

```typescript
// schema.ts
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  users: defineTable({
    telegramId: v.number(),
    username: v.string(),
    settings: v.object({
      defaultSlippage: v.number(),
      autoCompound: v.boolean(),
      riskLevel: v.string(),
    }),
    createdAt: v.number(),
    lastActive: v.number(),
  })
    .index("by_telegram", ["telegramId"])
    .index("by_activity", ["lastActive"]),

  wallets: defineTable({
    userId: v.id("users"),
    address: v.string(),
    type: v.union(v.literal("hot"), v.literal("ledger"), v.literal("trezor")),
    isActive: v.boolean(),
    balance: v.object({
      sol: v.string(),
      usd: v.string(),
      lastUpdated: v.number(),
    }),
  })
    .index("by_user", ["userId"])
    .index("by_address", ["address"]),

  positions: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    tokenMint: v.string(),
    symbol: v.string(),
    amount: v.string(),
    averagePrice: v.string(),
    currentPrice: v.string(),
    pnl: v.object({
      amount: v.string(),
      percentage: v.number(),
    }),
    metadata: v.any(),
  })
    .index("by_user", ["userId"])
    .index("by_wallet", ["walletId"])
    .index("by_token", ["tokenMint"]),

  orders: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    type: v.union(
      v.literal("market"),
      v.literal("limit"),
      v.literal("stop_loss"),
      v.literal("take_profit"),
      v.literal("dca")
    ),
    status: v.union(
      v.literal("pending"),
      v.literal("executing"),
      v.literal("completed"),
      v.literal("failed"),
      v.literal("cancelled")
    ),
    tokenIn: v.string(),
    tokenOut: v.string(),
    amountIn: v.string(),
    expectedOut: v.string(),
    actualOut: v.optional(v.string()),
    slippage: v.number(),
    txSignature: v.optional(v.string()),
    error: v.optional(v.string()),
    createdAt: v.number(),
    executedAt: v.optional(v.number()),
  })
    .index("by_user_status", ["userId", "status"])
    .index("by_created", ["createdAt"]),

  priceFeeds: defineTable({
    tokenMint: v.string(),
    symbol: v.string(),
    price: v.string(),
    volume24h: v.string(),
    change24h: v.number(),
    marketCap: v.string(),
    source: v.string(),
    timestamp: v.number(),
  })
    .index("by_token", ["tokenMint"])
    .index("by_symbol", ["symbol"])
    .index("by_timestamp", ["timestamp"]),

  alerts: defineTable({
    userId: v.id("users"),
    type: v.string(),
    condition: v.object({
      token: v.string(),
      metric: v.string(),
      operator: v.string(),
      value: v.string(),
    }),
    isActive: v.boolean(),
    triggered: v.boolean(),
    lastTriggered: v.optional(v.number()),
  })
    .index("by_user_active", ["userId", "isActive"]),

  dcaStrategies: defineTable({
    userId: v.id("users"),
    walletId: v.id("wallets"),
    name: v.string(),
    tokenIn: v.string(),
    tokenOut: v.string(),
    amount: v.string(),
    frequency: v.string(), // cron expression
    isActive: v.boolean(),
    nextExecution: v.number(),
    executions: v.array(v.object({
      timestamp: v.number(),
      amountIn: v.string(),
      amountOut: v.string(),
      txSignature: v.string(),
    })),
  })
    .index("by_user", ["userId"])
    .index("by_next_execution", ["nextExecution", "isActive"]),
});
```

## Key Convex Functions

### Queries (Real-time Data)

```typescript
// queries/portfolio.ts
import { query } from "./_generated/server";
import { v } from "convex/values";

export const getPortfolio = query({
  args: { userId: v.id("users") },
  handler: async (ctx, args) => {
    const positions = await ctx.db
      .query("positions")
      .withIndex("by_user", (q) => q.eq("userId", args.userId))
      .collect();

    // Convex automatically tracks this query's dependencies
    // When any position changes, this query reruns
    // And updates are pushed to all subscribed clients
    
    const totalValue = positions.reduce((sum, pos) => {
      const value = parseFloat(pos.amount) * parseFloat(pos.currentPrice);
      return sum + value;
    }, 0);

    return {
      positions,
      totalValue,
      totalPnL: calculateTotalPnL(positions),
    };
  },
});

export const watchPrices = query({
  args: { 
    tokens: v.array(v.string()),
    interval: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const prices = await ctx.db
      .query("priceFeeds")
      .filter((q) => 
        args.tokens.some(token => q.eq(q.field("tokenMint"), token))
      )
      .order("desc")
      .take(args.tokens.length)
      .collect();

    // This query will automatically update whenever
    // new prices are inserted into priceFeeds table
    return prices;
  },
});
```

### Mutations (Transactional Updates)

```typescript
// mutations/trading.ts
import { mutation } from "./_generated/server";
import { v } from "convex/values";

export const placeTrade = mutation({
  args: {
    userId: v.id("users"),
    walletId: v.id("wallets"),
    tokenIn: v.string(),
    tokenOut: v.string(),
    amount: v.string(),
    slippage: v.number(),
  },
  handler: async (ctx, args) => {
    // All database operations in a mutation run in a transaction
    // If any operation fails, everything is rolled back
    
    // Create order record
    const orderId = await ctx.db.insert("orders", {
      userId: args.userId,
      walletId: args.walletId,
      type: "market",
      status: "pending",
      tokenIn: args.tokenIn,
      tokenOut: args.tokenOut,
      amountIn: args.amount,
      expectedOut: "0", // Will be updated
      slippage: args.slippage,
      createdAt: Date.now(),
    });

    // Update user's last activity
    await ctx.db.patch(args.userId, {
      lastActive: Date.now(),
    });

    // Schedule the actual blockchain execution
    await ctx.scheduler.runAfter(0, "actions/executeTrade", {
      orderId,
      ...args,
    });

    return orderId;
  },
});

export const updatePosition = mutation({
  args: {
    positionId: v.id("positions"),
    currentPrice: v.string(),
  },
  handler: async (ctx, args) => {
    const position = await ctx.db.get(args.positionId);
    if (!position) throw new Error("Position not found");

    const pnl = calculatePnL(
      position.amount,
      position.averagePrice,
      args.currentPrice
    );

    await ctx.db.patch(args.positionId, {
      currentPrice: args.currentPrice,
      pnl,
    });

    // All clients watching this position will instantly update
    return { success: true };
  },
});
```

### Actions (External API Calls)

```typescript
// actions/solana.ts
import { action } from "./_generated/server";
import { v } from "convex/values";
import { Connection, PublicKey, Transaction } from "@solana/web3.js";

export const executeTrade = action({
  args: {
    orderId: v.id("orders"),
    walletId: v.id("wallets"),
    tokenIn: v.string(),
    tokenOut: v.string(),
    amount: v.string(),
    slippage: v.number(),
  },
  handler: async (ctx, args) => {
    // Actions can make external API calls
    // This is where we integrate with Solana
    
    try {
      // Get Jupiter quote
      const quote = await getJupiterQuote({
        inputMint: args.tokenIn,
        outputMint: args.tokenOut,
        amount: args.amount,
        slippage: args.slippage,
      });

      // Build transaction
      const { swapTransaction } = await getJupiterSwap({
        quoteResponse: quote,
        userPublicKey: walletPublicKey,
      });

      // Execute on Solana
      const connection = new Connection(RPC_ENDPOINT);
      const txid = await connection.sendTransaction(swapTransaction);

      // Update order in database
      await ctx.runMutation("mutations/trading:completeOrder", {
        orderId: args.orderId,
        txSignature: txid,
        actualOut: quote.outAmount,
      });

      return { success: true, txid };
    } catch (error) {
      // Update order with error
      await ctx.runMutation("mutations/trading:failOrder", {
        orderId: args.orderId,
        error: error.message,
      });
      
      throw error;
    }
  },
});

export const syncWalletBalance = action({
  args: { walletId: v.id("wallets") },
  handler: async (ctx, args) => {
    const wallet = await ctx.runQuery("queries/wallets:get", { 
      walletId: args.walletId 
    });

    const connection = new Connection(RPC_ENDPOINT);
    const balance = await connection.getBalance(
      new PublicKey(wallet.address)
    );

    // Get token accounts
    const tokenAccounts = await getTokenAccounts(wallet.address);

    // Update wallet balance
    await ctx.runMutation("mutations/wallets:updateBalance", {
      walletId: args.walletId,
      balance: {
        sol: (balance / 1e9).toString(),
        usd: calculateUSDValue(balance, tokenAccounts),
        lastUpdated: Date.now(),
      },
    });

    // Update positions
    for (const account of tokenAccounts) {
      await ctx.runMutation("mutations/positions:sync", {
        walletId: args.walletId,
        tokenMint: account.mint,
        amount: account.amount,
      });
    }
  },
});
```

### Cron Jobs (Scheduled Tasks)

```typescript
// crons.ts
import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();

// Update prices every minute
crons.interval(
  "update prices",
  { minutes: 1 },
  internal.actions.prices.updateAllPrices
);

// Execute DCA strategies
crons.interval(
  "execute DCA",
  { minutes: 5 },
  internal.actions.dca.executeScheduledDCA
);

// Clean up old data
crons.daily(
  "cleanup",
  { hourUTC: 3, minuteUTC: 0 },
  internal.actions.maintenance.cleanupOldData
);

export default crons;
```

## Migration Strategy

### Phase 1: Setup Convex (Week 1)
1. Initialize Convex project
2. Define complete schema
3. Set up authentication
4. Create basic queries/mutations

### Phase 2: Data Migration (Week 2)
1. Export existing SQLite/PostgreSQL data
2. Transform to Convex document format
3. Import historical data
4. Verify data integrity

### Phase 3: Function Implementation (Week 3-4)
1. Port existing database queries to Convex
2. Implement real-time subscriptions
3. Create action functions for Solana integration
4. Set up cron jobs for automation

### Phase 4: Client Integration (Week 5)
1. Update Telegram bot to use Convex client
2. Implement real-time updates in web dashboard
3. Add WebSocket subscriptions for price feeds
4. Test end-to-end flows

### Phase 5: Production Deployment (Week 6)
1. Deploy to Convex cloud or self-host
2. Configure monitoring and alerts
3. Implement backup strategies
4. Performance optimization

## Performance Benefits

### Real-time Updates
- **Latency**: <50ms for data propagation
- **Subscriptions**: Automatic, no polling needed
- **Consistency**: Guaranteed across all clients

### Scalability
- **Horizontal Scaling**: Automatic with Convex cloud
- **Caching**: Built-in query result caching
- **Optimization**: Automatic query dependency tracking

### Developer Experience
- **Type Safety**: End-to-end TypeScript
- **Hot Reload**: Instant function updates
- **Debugging**: Built-in dashboard and logs
- **AI Generation**: LLMs can easily generate Convex code

## Integration with Existing Rust Code

Our Rust-based services will continue to handle:
- Low-level Solana program interactions
- High-frequency trading algorithms
- MEV and arbitrage strategies
- Hardware wallet communication

Convex will handle:
- Data persistence and real-time sync
- User management and authentication
- API layer for frontends
- Scheduling and automation
- File storage and caching

Communication between Rust services and Convex:
```
Rust Service → HTTP Action → Convex Mutation → Database
                                             ↓
                                   Real-time Updates → All Clients
```

## Security Considerations

1. **Authentication**: Built-in auth with Convex Auth
2. **Rate Limiting**: Automatic function-level limits
3. **Validation**: TypeScript types enforce data integrity
4. **Encryption**: TLS for all connections
5. **Isolation**: Sandboxed function execution
6. **Audit Logs**: Built-in for compliance

## Monitoring and Observability

Convex provides:
- Function execution logs
- Performance metrics
- Error tracking
- Usage analytics
- Real-time dashboard

Integration with existing tools:
- Export metrics to Prometheus
- Send logs to DataDog/Splunk
- Alerts via PagerDuty/Slack

## Cost Analysis

### Convex Cloud Pricing (2025)
- **Free Tier**: 1M function calls, 1GB storage
- **Pro**: $25/month base + usage
- **Scale**: Custom pricing for high volume

### Self-Hosted Costs
- Infrastructure: ~$100-500/month
- Maintenance: Internal team time
- Benefits: Full control, compliance

## Conclusion

Convex provides the perfect backend for our Solana trading bot:
- **Real-time**: Instant updates across all clients
- **Type-safe**: Reduced bugs, better AI code generation
- **Scalable**: Handles growth automatically
- **Developer-friendly**: Fast iteration and deployment

The reactive nature of Convex eliminates complex state management, making our trading bot more reliable and easier to maintain.