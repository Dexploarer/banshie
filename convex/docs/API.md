# Solana Trading Bot - Convex API Reference

## Overview

This document provides comprehensive API documentation for the Convex backend of the Solana Trading Bot. All functions are organized into queries (read-only), mutations (state-changing), and actions (external integrations).

## Table of Contents

1. [Authentication](#authentication)
2. [Queries](#queries)
3. [Mutations](#mutations)
4. [Actions](#actions)
5. [Error Handling](#error-handling)
6. [Rate Limiting](#rate-limiting)
7. [Examples](#examples)

## Authentication

The Solana Trading Bot uses session-based authentication with Telegram integration.

### Session Management

```typescript
// Create session
const session = await ctx.runMutation(api.mutations.auth.createSession, {
  telegramId: 123456789,
  username: "trader_user"
});

// Verify session
const user = await ctx.runQuery(api.queries.auth.verifySession, {
  sessionToken: "session_token_here"
});
```

## Queries

Queries are read-only operations that fetch data from the database.

### Portfolio Queries

#### `queries/portfolio:getPortfolio`

Retrieves comprehensive portfolio information for a user.

**Parameters:**
- `userId` (Id<"users">): The user's unique identifier

**Returns:**
```typescript
{
  summary: {
    totalValue: string;           // Total portfolio value in USD
    totalPnL: string;            // Total profit/loss amount
    totalPnLPercentage: string;  // P&L as percentage
    positionCount: number;       // Number of active positions
  };
  positions: Array<{
    _id: Id<"positions">;
    symbol: string;              // Token symbol (e.g., "SOL")
    name: string;                // Token full name
    amount: string;              // Amount of tokens held
    averagePrice: string;        // Average purchase price
    currentPrice: string;        // Current market price
    marketValue: string;         // Current market value
    costBasis: string;          // Total amount invested
    pnl: {
      amount: string;           // P&L amount in USD
      percentage: number;       // P&L percentage
      isProfit: boolean;        // Whether position is profitable
    };
    allocation: number;         // Portfolio allocation percentage
  }>;
  wallets: Array<{
    _id: Id<"wallets">;
    address: string;            // Wallet public address
    isActive: boolean;          // Whether wallet is active
    balance: {
      sol: string;              // SOL balance
      usd: string;              // USD value of balance
    };
  }>;
}
```

**Example:**
```typescript
const portfolio = await ctx.runQuery(api.queries.portfolio.getPortfolio, {
  userId: "user_123456789"
});

console.log(`Total Value: $${portfolio.summary.totalValue}`);
console.log(`Positions: ${portfolio.positions.length}`);
```

#### `queries/portfolio:getPositionHistory`

Retrieves historical performance data for a specific position.

**Parameters:**
- `positionId` (Id<"positions">): Position identifier
- `timeframe` (optional, string): "1d", "7d", "30d", "1y" (default: "30d")
- `interval` (optional, string): "1h", "4h", "1d" (default: "1d")

**Returns:**
```typescript
{
  positionId: Id<"positions">;
  timeframe: string;
  data: Array<{
    timestamp: number;          // Unix timestamp
    value: number;              // Position value at timestamp
    price: number;              // Token price at timestamp
    pnl: number;                // P&L at timestamp
  }>;
  summary: {
    totalReturn: number;        // Total return percentage
    maxDrawdown: number;        // Maximum drawdown percentage
    volatility: number;         // Price volatility
    sharpeRatio: number;        // Risk-adjusted return
  };
}
```

### Price Data Queries

#### `queries/prices:getTokenPrice`

Retrieves current price information for a specific token.

**Parameters:**
- `mint` (string): Token mint address

**Returns:**
```typescript
{
  mint: string;                 // Token mint address
  symbol: string;               // Token symbol
  name: string;                 // Token full name
  price: number;                // Current price in USD
  priceChange24h: number;       // 24h price change percentage
  volume24h: number;            // 24h trading volume
  marketCap: number;            // Market capitalization
  supply: number;               // Circulating supply
  lastUpdated: number;          // Last update timestamp
}
```

**Example:**
```typescript
const solPrice = await ctx.runQuery(api.queries.prices.getTokenPrice, {
  mint: "So11111111111111111111111111111111111111112"
});

console.log(`SOL Price: $${solPrice.price}`);
console.log(`24h Change: ${solPrice.priceChange24h}%`);
```

#### `queries/prices:getPriceHistory`

Retrieves historical price data for charting and analysis.

**Parameters:**
- `tokenMint` (string): Token mint address
- `interval` (string): "1m", "5m", "15m", "1h", "4h", "1d"
- `limit` (optional, number): Maximum number of data points (default: 100)
- `startTime` (optional, number): Start timestamp
- `endTime` (optional, number): End timestamp

**Returns:**
```typescript
{
  tokenMint: string;
  symbol: string;
  interval: string;
  data: Array<{
    timestamp: number;          // Unix timestamp
    open: number;               // Opening price
    high: number;               // Highest price
    low: number;                // Lowest price
    close: number;              // Closing price
    volume: number;             // Trading volume
  }>;
}
```

#### `queries/prices:getMarketOverview`

Retrieves market overview with top tokens by various criteria.

**Parameters:**
- `category` (string): "trending", "gainers", "losers", "volume"
- `limit` (optional, number): Number of tokens to return (default: 20)
- `timeframe` (optional, string): "1h", "24h", "7d" (default: "24h")

**Returns:**
```typescript
{
  category: string;
  timeframe: string;
  tokens: Array<{
    mint: string;
    symbol: string;
    name: string;
    price: number;
    change: number;             // Price change percentage
    volume: number;             // Trading volume
    marketCap: number;
    rank: number;               // Market rank
  }>;
  lastUpdated: number;
}
```

### AI Analysis Queries

#### `queries/ai:getLatestAnalysis`

Retrieves the most recent AI analysis for a token.

**Parameters:**
- `targetId` (string): Token mint address or market identifier
- `type` (optional, union): "sentiment" | "technical" | "fundamental" | "prediction"
- `limit` (optional, number): Number of analyses to return (default: 5)

**Returns:**
```typescript
Array<{
  id: Id<"aiAnalysis">;
  targetId: string;
  type: string;
  analysis: {
    summary: string;            // Analysis summary
    score: number;              // Score from -100 to 100
    confidence: number;         // Confidence level 0-1
    signals: Array<{
      type: string;             // Signal type
      strength: string;         // "weak" | "medium" | "strong"
      description: string;      // Signal description
    }>;
    recommendation: string;     // "strong_buy" | "buy" | "hold" | "sell" | "strong_sell"
  };
  sources: string[];           // Data sources used
  model: string;               // AI model used
  timestamp: number;           // Analysis timestamp
  expiresAt: number;           // Expiration timestamp
}>
```

**Example:**
```typescript
const analysis = await ctx.runQuery(api.queries.ai.getLatestAnalysis, {
  targetId: "So11111111111111111111111111111111111111112",
  type: "technical",
  limit: 1
});

if (analysis.length > 0) {
  console.log(`Analysis: ${analysis[0].analysis.summary}`);
  console.log(`Score: ${analysis[0].analysis.score}/100`);
  console.log(`Recommendation: ${analysis[0].analysis.recommendation}`);
}
```

#### `queries/ai:getLatestSignals`

Retrieves recent AI trading signals.

**Parameters:**
- `tokenMint` (optional, string): Filter by specific token
- `action` (optional, string): "buy" | "sell" | "hold"
- `signalType` (optional, string): Signal type filter
- `minConfidence` (optional, number): Minimum confidence threshold (0-100)
- `limit` (optional, number): Number of signals to return (default: 10)

**Returns:**
```typescript
Array<{
  id: Id<"tradingSignals">;
  tokenMint: string;
  symbol: string;
  signalType: string;          // "momentum" | "reversal" | "breakout" | etc.
  action: string;              // "buy" | "sell" | "hold"
  strength: number;            // Signal strength 0-100
  confidence: number;          // Confidence level 0-100
  reasoning: string;           // Explanation of the signal
  technicalFactors: string[];  // Technical analysis factors
  fundamentalFactors: string[]; // Fundamental analysis factors
  sentimentFactors: string[];  // Sentiment analysis factors
  priceTarget: number;         // Target price (optional)
  stopLoss: number;            // Stop loss price (optional)
  timeframe: string;           // "short" | "medium" | "long"
  riskLevel: string;           // "low" | "medium" | "high"
  validUntil: number;          // Signal expiration timestamp
  createdAt: number;           // Signal creation timestamp
  performance: {               // Performance tracking (optional)
    executed: boolean;
    outcome: string;           // "profit" | "loss" | "neutral"
    returnPct: number;         // Return percentage
    updatedAt: number;
  };
}>
```

### Trading Queries

#### `queries/trading:getOrderHistory`

Retrieves user's trading order history.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `status` (optional, string): Filter by order status
- `tokenMint` (optional, string): Filter by token
- `limit` (optional, number): Number of orders to return (default: 50)
- `offset` (optional, number): Pagination offset (default: 0)

**Returns:**
```typescript
{
  orders: Array<{
    _id: Id<"orders">;
    type: string;               // Order type
    side: string;               // "buy" | "sell"
    status: string;             // Order status
    tokenIn: {
      mint: string;
      symbol: string;
      amount: string;
    };
    tokenOut: {
      mint: string;
      symbol: string;
      amount: string;
    };
    pricing: {
      expectedPrice: string;
      executionPrice: string;
      slippage: number;
      fee: string;
    };
    execution: {
      txSignature: string;
      blockHeight: number;
      gasUsed: string;
    };
    createdAt: number;
    executedAt: number;
  }>;
  pagination: {
    total: number;
    limit: number;
    offset: number;
    hasMore: boolean;
  };
}
```

#### `queries/trading:getOrderStatus`

Retrieves the current status of a specific order.

**Parameters:**
- `orderId` (Id<"orders">): Order identifier

**Returns:**
```typescript
{
  _id: Id<"orders">;
  status: string;              // "pending" | "submitted" | "executing" | "completed" | "failed" | "cancelled"
  statusDetails: {
    message: string;            // Status description
    lastUpdated: number;        // Last status update
    attempts: number;           // Execution attempts
  };
  execution: {
    txSignature: string;        // Transaction signature (if executed)
    blockHeight: number;        // Block height (if executed)
    actualSlippage: number;     // Actual slippage experienced
    executionTime: number;      // Time taken to execute
  };
  error: {                     // Error details (if failed)
    code: string;
    message: string;
    details: string;
  };
}
```

### DCA Strategy Queries

#### `queries/dca:getUserStrategies`

Retrieves user's DCA strategies.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `isActive` (optional, boolean): Filter by active status
- `tokenMint` (optional, string): Filter by target token

**Returns:**
```typescript
Array<{
  _id: Id<"dcaStrategies">;
  name: string;                // Strategy name
  description: string;         // Strategy description
  isActive: boolean;           // Whether strategy is active
  isPaused: boolean;           // Whether strategy is paused
  config: {
    tokenIn: {
      mint: string;
      symbol: string;
    };
    tokenOut: {
      mint: string;
      symbol: string;
    };
    amount: string;            // Amount to invest per execution
    frequency: {
      type: string;            // "interval" | "cron" | "dynamic"
      value: string;           // Frequency specification
    };
    conditions: {
      minPrice: string;        // Minimum price condition
      maxPrice: string;        // Maximum price condition
      onlyBuyDips: boolean;    // Only execute on price dips
      dipThreshold: number;    // Dip percentage threshold
    };
    limits: {
      maxInvestment: string;   // Maximum total investment
      maxExecutions: number;   // Maximum number of executions
      endDate: number;         // Strategy end date
    };
  };
  stats: {
    totalExecutions: number;   // Number of executions completed
    totalInvested: string;     // Total amount invested
    totalReceived: string;     // Total tokens received
    averagePrice: string;      // Average purchase price
    currentValue: string;      // Current value of holdings
    pnl: {
      amount: string;          // P&L amount
      percentage: number;      // P&L percentage
    };
    lastExecution: number;     // Last execution timestamp
    nextExecution: number;     // Next scheduled execution
  };
  createdAt: number;
  updatedAt: number;
}>
```

#### `queries/dca:getStrategyPerformance`

Retrieves detailed performance metrics for a DCA strategy.

**Parameters:**
- `strategyId` (Id<"dcaStrategies">): Strategy identifier
- `timeframe` (optional, string): "7d", "30d", "90d", "1y", "all" (default: "30d")

**Returns:**
```typescript
{
  strategyId: Id<"dcaStrategies">;
  timeframe: string;
  performance: {
    totalReturn: number;        // Total return percentage
    annualizedReturn: number;   // Annualized return percentage
    volatility: number;         // Return volatility
    sharpeRatio: number;        // Sharpe ratio
    maxDrawdown: number;        // Maximum drawdown percentage
    winRate: number;           // Percentage of profitable periods
  };
  executions: Array<{
    timestamp: number;          // Execution timestamp
    amountInvested: string;     // Amount invested
    tokensReceived: string;     // Tokens received
    price: string;              // Execution price
    value: string;              // Current value
    pnl: number;                // P&L at time of execution
  }>;
  benchmarkComparison: {
    benchmarkReturn: number;    // Benchmark return (buy-and-hold)
    outperformance: number;     // Outperformance vs benchmark
    correlation: number;        // Correlation with benchmark
  };
}
```

### Alert Queries

#### `queries/alerts:getUserAlerts`

Retrieves user's price alerts and notifications.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `isActive` (optional, boolean): Filter by active status
- `alertType` (optional, string): Filter by alert type

**Returns:**
```typescript
Array<{
  _id: Id<"alerts">;
  name: string;                // Alert name
  type: string;                // "price" | "volume" | "position" | "technical" | "news"
  isActive: boolean;           // Whether alert is active
  condition: {
    target: string;            // Target token or position
    metric: string;            // Metric being monitored
    operator: string;          // "above" | "below" | "equals" | "change"
    value: string;             // Threshold value
    timeframe: string;         // Time window for condition
  };
  actions: string[];           // Actions to take when triggered
  notification: {
    channels: string[];        // Notification channels
    message: string;           // Custom message
    cooldown: number;          // Cooldown between notifications
  };
  stats: {
    triggered: boolean;        // Whether alert has been triggered
    triggerCount: number;      // Number of times triggered
    lastTriggered: number;     // Last trigger timestamp
    nextCheck: number;         // Next check timestamp
  };
  createdAt: number;
  expiresAt: number;          // Alert expiration
}>
```

## Mutations

Mutations modify the database state and should be used for all write operations.

### User Management Mutations

#### `mutations/users:createOrUpdateUser`

Creates a new user or updates an existing one.

**Parameters:**
- `telegramId` (number): Telegram user ID
- `username` (string): Telegram username
- `isPremium` (optional, boolean): Premium status (default: false)
- `settings` (optional, object): User settings

**Returns:**
```typescript
Id<"users"> // User ID
```

**Example:**
```typescript
const userId = await ctx.runMutation(api.mutations.users.createOrUpdateUser, {
  telegramId: 123456789,
  username: "crypto_trader",
  isPremium: false,
  settings: {
    defaultSlippage: 1.0,
    riskTolerance: "medium",
    notifications: true,
    language: "en"
  }
});
```

#### `mutations/users:updateSettings`

Updates user settings.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `settings` (object): Settings to update

**Returns:**
```typescript
Id<"users"> // User ID
```

### Trading Mutations

#### `mutations/trading:placeTrade`

Places a new trading order.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `walletId` (Id<"wallets">): Wallet identifier
- `type` (string): Order type ("market" | "limit" | "stop_loss" | "take_profit")
- `side` (string): Order side ("buy" | "sell")
- `tokenIn` (object): Input token details
- `tokenOut` (object): Output token details
- `pricing` (object): Pricing parameters
- `conditions` (optional, object): Conditional order parameters

**Returns:**
```typescript
Id<"orders"> // Order ID
```

**Example:**
```typescript
const orderId = await ctx.runMutation(api.mutations.trading.placeTrade, {
  userId: "user_123456789",
  walletId: "wallet_abc123",
  type: "market",
  side: "buy",
  tokenIn: {
    mint: "So11111111111111111111111111111111111111112",
    symbol: "SOL",
    amount: "1000000000", // 1 SOL in lamports
    decimals: 9
  },
  tokenOut: {
    mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    symbol: "USDC",
    amount: "0", // To be calculated
    decimals: 6
  },
  pricing: {
    expectedPrice: "150.00",
    slippage: 0.01, // 1%
    priceImpact: 0.005
  }
});
```

#### `mutations/trading:cancelOrder`

Cancels a pending order.

**Parameters:**
- `orderId` (Id<"orders">): Order identifier
- `userId` (Id<"users">): User identifier (for authorization)

**Returns:**
```typescript
{
  success: boolean;
  message: string;
  orderId: Id<"orders">;
}
```

#### `mutations/trading:updateOrderStatus`

Updates the status of an order (typically called by actions).

**Parameters:**
- `orderId` (Id<"orders">): Order identifier
- `status` (string): New status
- `statusDetails` (optional, object): Status details
- `execution` (optional, object): Execution details
- `error` (optional, object): Error details

**Returns:**
```typescript
Id<"orders"> // Order ID
```

### DCA Strategy Mutations

#### `mutations/dca:createStrategy`

Creates a new DCA strategy.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `walletId` (Id<"wallets">): Wallet identifier
- `name` (string): Strategy name
- `description` (optional, string): Strategy description
- `config` (object): Strategy configuration

**Returns:**
```typescript
Id<"dcaStrategies"> // Strategy ID
```

**Example:**
```typescript
const strategyId = await ctx.runMutation(api.mutations.dca.createStrategy, {
  userId: "user_123456789",
  walletId: "wallet_abc123",
  name: "SOL Weekly DCA",
  description: "Weekly DCA into SOL with $100",
  config: {
    tokenIn: {
      mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      symbol: "USDC"
    },
    tokenOut: {
      mint: "So11111111111111111111111111111111111111112",
      symbol: "SOL"
    },
    amount: "100.00",
    frequency: {
      type: "interval",
      value: "7d" // Weekly
    },
    conditions: {
      minPrice: "0",
      maxPrice: "1000",
      onlyBuyDips: false,
      dipThreshold: 0
    },
    limits: {
      maxInvestment: "5000.00",
      maxExecutions: 50,
      endDate: Date.now() + (365 * 24 * 60 * 60 * 1000) // 1 year
    }
  }
});
```

#### `mutations/dca:updateStrategy`

Updates an existing DCA strategy.

**Parameters:**
- `strategyId` (Id<"dcaStrategies">): Strategy identifier
- `userId` (Id<"users">): User identifier (for authorization)
- `updates` (object): Fields to update

**Returns:**
```typescript
Id<"dcaStrategies"> // Strategy ID
```

#### `mutations/dca:pauseStrategy`

Pauses a DCA strategy.

**Parameters:**
- `strategyId` (Id<"dcaStrategies">): Strategy identifier
- `userId` (Id<"users">): User identifier (for authorization)

**Returns:**
```typescript
{
  success: boolean;
  message: string;
  strategyId: Id<"dcaStrategies">;
}
```

#### `mutations/dca:resumeStrategy`

Resumes a paused DCA strategy.

**Parameters:**
- `strategyId` (Id<"dcaStrategies">): Strategy identifier
- `userId` (Id<"users">): User identifier (for authorization)

**Returns:**
```typescript
{
  success: boolean;
  message: string;
  strategyId: Id<"dcaStrategies">;
  nextExecution: number; // Next scheduled execution timestamp
}
```

### Alert Mutations

#### `mutations/alerts:createAlert`

Creates a new price alert.

**Parameters:**
- `userId` (Id<"users">): User identifier
- `name` (string): Alert name
- `type` (string): Alert type
- `condition` (object): Alert condition
- `actions` (string[]): Actions to take
- `notification` (object): Notification settings
- `expiresAt` (optional, number): Expiration timestamp

**Returns:**
```typescript
Id<"alerts"> // Alert ID
```

**Example:**
```typescript
const alertId = await ctx.runMutation(api.mutations.alerts.createAlert, {
  userId: "user_123456789",
  name: "SOL Price Alert - $200",
  type: "price",
  condition: {
    target: "So11111111111111111111111111111111111111112",
    metric: "price",
    operator: "above",
    value: "200.00",
    timeframe: "1m"
  },
  actions: ["notify", "execute_trade"],
  notification: {
    channels: ["telegram", "email"],
    message: "🚀 SOL has reached $200!",
    cooldown: 3600 // 1 hour
  },
  expiresAt: Date.now() + (30 * 24 * 60 * 60 * 1000) // 30 days
});
```

### AI Analysis Mutations

#### `mutations/ai:storeAIAnalysis`

Stores AI analysis results.

**Parameters:**
- `targetId` (string): Target identifier (token mint)
- `type` (string): Analysis type
- `analysis` (object): Analysis results
- `embedding` (optional, number[]): Vector embedding
- `sources` (string[]): Data sources
- `model` (string): AI model used
- `expiresAt` (optional, number): Expiration timestamp

**Returns:**
```typescript
Id<"aiAnalysis"> // Analysis ID
```

#### `mutations/ai:storeTradingSignal`

Stores a trading signal.

**Parameters:**
- `tokenMint` (string): Token mint address
- `symbol` (string): Token symbol
- `signalType` (string): Signal type
- `action` (string): Trading action
- `strength` (number): Signal strength (0-100)
- `confidence` (number): Confidence level (0-100)
- `reasoning` (string): Signal reasoning
- `technicalFactors` (string[]): Technical factors
- `fundamentalFactors` (string[]): Fundamental factors
- `sentimentFactors` (string[]): Sentiment factors
- `priceTarget` (optional, number): Target price
- `stopLoss` (optional, number): Stop loss price
- `timeframe` (string): Signal timeframe
- `riskLevel` (string): Risk level
- `embedding` (optional, number[]): Vector embedding
- `validUntil` (number): Signal expiration

**Returns:**
```typescript
Id<"tradingSignals"> // Signal ID
```

## Actions

Actions handle external integrations and complex business logic.

### Trading Actions

#### `actions/solana:executeTrade`

Executes a trade on Solana using Jupiter DEX.

**Parameters:**
- `orderId` (Id<"orders">): Order identifier
- `walletAddress` (string): Wallet public key
- `priorityFee` (optional, number): Priority fee in lamports

**Returns:**
```typescript
{
  success: boolean;
  txSignature: string;      // Transaction signature
  blockHeight: number;      // Block height
  actualSlippage: number;   // Actual slippage experienced
  gasUsed: string;          // Gas consumed
  executionTime: number;    // Execution time in ms
  error: string;            // Error message if failed
}
```

#### `actions/solana:simulateTrade`

Simulates a trade to estimate results.

**Parameters:**
- `tokenIn` (object): Input token details
- `tokenOut` (object): Output token details
- `amount` (string): Trade amount
- `slippage` (number): Slippage tolerance

**Returns:**
```typescript
{
  estimatedOutput: string;   // Estimated output amount
  priceImpact: number;       // Price impact percentage
  fee: string;               // Estimated fees
  route: object[];           // Routing information
  worstCaseOutput: string;   // Worst case output with slippage
}
```

### AI Actions

#### `actions/ai:generateTradingSignals`

Generates AI trading signals for a token.

**Parameters:**
- `tokenMint` (string): Token mint address
- `analysisDepth` (optional, string): "basic" | "standard" | "deep"
- `timeframe` (optional, string): Signal timeframe

**Returns:**
```typescript
{
  signals: Array<{
    signalId: Id<"tradingSignals">;
    action: string;
    confidence: number;
    reasoning: string;
    priceTarget: number;
    stopLoss: number;
    timeframe: string;
    riskLevel: string;
  }>;
  analysisMetadata: {
    model: string;
    dataQuality: string;
    processingTime: number;
    sources: string[];
  };
}
```

#### `actions/ai:analyzeSentiment`

Performs sentiment analysis for a token.

**Parameters:**
- `tokenMint` (string): Token mint address
- `symbol` (string): Token symbol
- `sources` (optional, string[]): Data sources to analyze
- `timeframe` (optional, string): Analysis timeframe

**Returns:**
```typescript
{
  analysisId: Id<"aiAnalysis">;
  sentiment: {
    score: number;            // Sentiment score -100 to 100
    confidence: number;       // Confidence level 0-1
    summary: string;          // Analysis summary
    signals: Array<{
      type: string;
      strength: string;
      description: string;
    }>;
    recommendation: string;   // Trading recommendation
  };
  sources: Array<{
    name: string;
    dataPoints: number;
    avgSentiment: number;
  }>;
  dataQuality: string;
  timeframe: string;
}
```

### Vector Search Actions

#### `actions/vector_search:searchSimilarAnalysis`

Searches for similar AI analyses using vector similarity.

**Parameters:**
- `query` (string): Search query
- `targetId` (optional, string): Filter by target
- `analysisType` (optional, string): Filter by analysis type
- `limit` (optional, number): Number of results

**Returns:**
```typescript
{
  query: string;
  results: Array<{
    id: Id<"aiAnalysis">;
    score: number;            // Similarity score 0-1
    analysis: object;         // Analysis data
    targetId: string;
    type: string;
    timestamp: number;
  }>;
  embedding: number[];       // Query embedding
}
```

#### `actions/vector_search:findSimilarSignals`

Finds similar trading signals using vector search.

**Parameters:**
- `description` (string): Signal description to match
- `tokenMint` (optional, string): Filter by token
- `signalType` (optional, string): Filter by signal type
- `action` (optional, string): Filter by action
- `limit` (optional, number): Number of results

**Returns:**
```typescript
{
  description: string;
  results: Array<{
    id: Id<"tradingSignals">;
    score: number;            // Similarity score 0-1
    tokenMint: string;
    symbol: string;
    signalType: string;
    action: string;
    confidence: number;
    reasoning: string;
    performance: object;      // Historical performance
    createdAt: number;
  }>;
}
```

### Media Generation Actions

#### `actions/media_generator:generatePriceChart`

Generates a price chart image.

**Parameters:**
- `tokenMint` (string): Token mint address
- `symbol` (string): Token symbol
- `interval` (optional, string): Chart interval
- `period` (optional, number): Number of data points
- `chartType` (optional, string): Chart type
- `indicators` (optional, string[]): Technical indicators
- `theme` (optional, string): Chart theme

**Returns:**
```typescript
{
  symbol: string;
  chartType: string;
  interval: string;
  period: number;
  indicators: string[];
  theme: string;
  imageBase64: string;       // Base64 encoded image
  imageSize: {
    width: number;
    height: number;
  };
  dataPoints: number;
  timestamp: number;
}
```

## Error Handling

All API functions follow consistent error handling patterns:

### Error Response Format

```typescript
{
  error: {
    code: string;              // Error code
    message: string;           // Human-readable message
    details: string;           // Additional details
    timestamp: number;         // Error timestamp
    traceId: string;          // Trace ID for debugging
  };
}
```

### Common Error Codes

- `INVALID_PARAMS`: Invalid parameters provided
- `UNAUTHORIZED`: User not authenticated or authorized
- `NOT_FOUND`: Requested resource not found
- `RATE_LIMITED`: Rate limit exceeded
- `EXTERNAL_API_ERROR`: External API integration error
- `INSUFFICIENT_BALANCE`: Insufficient wallet balance
- `NETWORK_ERROR`: Blockchain network error
- `VALIDATION_ERROR`: Data validation failed
- `INTERNAL_ERROR`: Internal server error

### Error Handling Example

```typescript
try {
  const portfolio = await ctx.runQuery(api.queries.portfolio.getPortfolio, {
    userId: "invalid_user_id"
  });
} catch (error) {
  if (error.code === "NOT_FOUND") {
    console.log("Portfolio not found for user");
  } else if (error.code === "UNAUTHORIZED") {
    console.log("User not authorized to view portfolio");
  } else {
    console.log("Unexpected error:", error.message);
  }
}
```

## Rate Limiting

API endpoints have rate limits to ensure fair usage:

### Rate Limit Headers

All responses include rate limit headers:
- `X-RateLimit-Limit`: Maximum requests per window
- `X-RateLimit-Remaining`: Remaining requests in current window
- `X-RateLimit-Reset`: Window reset time (Unix timestamp)

### Rate Limits by Endpoint Type

- **Queries**: 100 requests per minute per user
- **Mutations**: 50 requests per minute per user
- **Actions**: 20 requests per minute per user
- **AI Actions**: 10 requests per minute per user

### Rate Limit Exceeded Response

```typescript
{
  error: {
    code: "RATE_LIMITED",
    message: "Rate limit exceeded",
    details: "Maximum 100 requests per minute allowed",
    retryAfter: 60 // Seconds until retry allowed
  }
}
```

## Examples

### Complete Trading Workflow

```typescript
// 1. Create user account
const userId = await ctx.runMutation(api.mutations.users.createOrUpdateUser, {
  telegramId: 123456789,
  username: "crypto_trader"
});

// 2. Get market analysis
const signals = await ctx.runQuery(api.queries.ai.getLatestSignals, {
  tokenMint: "So11111111111111111111111111111111111111112",
  minConfidence: 70,
  limit: 1
});

// 3. Place trade based on signal
if (signals.length > 0 && signals[0].action === "buy") {
  const orderId = await ctx.runMutation(api.mutations.trading.placeTrade, {
    userId: userId,
    walletId: "wallet_abc123",
    type: "market",
    side: "buy",
    tokenIn: {
      mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      symbol: "USDC",
      amount: "100000000", // $100 USDC
      decimals: 6
    },
    tokenOut: {
      mint: "So11111111111111111111111111111111111111112",
      symbol: "SOL",
      amount: "0",
      decimals: 9
    },
    pricing: {
      expectedPrice: signals[0].priceTarget?.toString() || "150.00",
      slippage: 0.01
    }
  });

  // 4. Monitor order execution
  const result = await ctx.runAction(api.actions.solana.executeTrade, {
    orderId: orderId,
    walletAddress: "wallet_public_key_here"
  });

  console.log(`Trade executed: ${result.txSignature}`);
}
```

### DCA Strategy Setup

```typescript
// 1. Create DCA strategy
const strategyId = await ctx.runMutation(api.mutations.dca.createStrategy, {
  userId: "user_123456789",
  walletId: "wallet_abc123",
  name: "BTC Weekly DCA",
  config: {
    tokenIn: {
      mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      symbol: "USDC"
    },
    tokenOut: {
      mint: "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E", // BTC
      symbol: "BTC"
    },
    amount: "50.00",
    frequency: {
      type: "interval",
      value: "7d"
    },
    limits: {
      maxInvestment: "2600.00", // 1 year of $50/week
      endDate: Date.now() + (365 * 24 * 60 * 60 * 1000)
    }
  }
});

// 2. Monitor strategy performance
const performance = await ctx.runQuery(api.queries.dca.getStrategyPerformance, {
  strategyId: strategyId,
  timeframe: "30d"
});

console.log(`Strategy return: ${performance.performance.totalReturn}%`);
console.log(`Sharpe ratio: ${performance.performance.sharpeRatio}`);
```

### AI Analysis Integration

```typescript
// 1. Generate comprehensive analysis
const analysis = await ctx.runAction(api.actions.ai.analyzeSentiment, {
  tokenMint: "So11111111111111111111111111111111111111112",
  symbol: "SOL",
  sources: ["twitter", "reddit", "news"],
  timeframe: "24h"
});

// 2. Search for similar historical analyses
const similarAnalyses = await ctx.runAction(api.actions.vector_search.searchSimilarAnalysis, {
  query: `SOL sentiment analysis bullish technical breakout`,
  targetId: "So11111111111111111111111111111111111111112",
  limit: 5
});

// 3. Generate trading signals based on analysis
const signals = await ctx.runAction(api.actions.ai.generateTradingSignals, {
  tokenMint: "So11111111111111111111111111111111111111112",
  analysisDepth: "deep"
});

console.log(`Generated ${signals.signals.length} trading signals`);
console.log(`Top signal: ${signals.signals[0].action} with ${signals.signals[0].confidence}% confidence`);
```

This comprehensive API documentation provides developers with all the information needed to integrate with the Solana Trading Bot's Convex backend. Each function includes detailed parameter descriptions, return types, examples, and error handling guidance.