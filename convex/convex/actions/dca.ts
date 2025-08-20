import { action } from "../_generated/server";
import { api } from "../_generated/api";
import { Id } from "../_generated/dataModel";

// Execute scheduled DCA strategies
export const executeScheduledStrategies = action({
  handler: async (ctx) => {
    console.log("🔄 Executing scheduled DCA strategies");
    
    // Get all active DCA strategies that are due
    const now = Date.now();
    const strategies = await ctx.runQuery(api.queries.dca.getDueStrategies, {
      timestamp: now,
    });

    console.log(`Found ${strategies.length} strategies due for execution`);

    const results = {
      total: strategies.length,
      executed: 0,
      failed: 0,
      skipped: 0,
    };

    // Execute each strategy
    for (const strategy of strategies) {
      try {
        // Check if conditions are met
        const shouldExecute = await checkStrategyConditions(ctx, strategy);
        
        if (!shouldExecute) {
          console.log(`Skipping strategy ${strategy._id}: conditions not met`);
          results.skipped++;
          continue;
        }

        // Execute the DCA order
        const orderId = await ctx.runMutation(api.mutations.trading.placeTrade, {
          userId: strategy.userId,
          walletId: strategy.walletId,
          type: "market",
          side: "buy",
          tokenIn: {
            mint: strategy.config.tokenIn.mint,
            symbol: strategy.config.tokenIn.symbol,
            amount: strategy.config.amount,
            decimals: 9,
          },
          tokenOut: {
            mint: strategy.config.tokenOut.mint,
            symbol: strategy.config.tokenOut.symbol,
            decimals: 9,
          },
          slippage: 1.0,
        });

        // Update strategy execution record
        await ctx.runMutation(api.mutations.dca.recordExecution, {
          strategyId: strategy._id,
          orderId,
          amount: strategy.config.amount,
          timestamp: now,
        });

        // Schedule next execution
        const nextExecution = calculateNextExecution(strategy.config.frequency, now);
        await ctx.runMutation(api.mutations.dca.updateNextExecution, {
          strategyId: strategy._id,
          nextExecution,
        });

        results.executed++;
        console.log(`✅ Executed DCA strategy ${strategy._id}`);
      } catch (error) {
        console.error(`❌ Failed to execute strategy ${strategy._id}:`, error);
        results.failed++;
        
        // Log error
        await ctx.runMutation(api.mutations.dca.logError, {
          strategyId: strategy._id,
          error: error.toString(),
          timestamp: now,
        });
      }
    }

    console.log(`DCA Execution Complete: ${results.executed} executed, ${results.failed} failed, ${results.skipped} skipped`);
    
    return results;
  },
});

// Process value averaging DCA strategies
export const processValueAveraging = action({
  handler: async (ctx) => {
    console.log("📊 Processing value averaging strategies");
    
    // Get all value averaging strategies
    const strategies = await ctx.runQuery(api.queries.dca.getValueAveragingStrategies);
    
    for (const strategy of strategies) {
      try {
        // Calculate target value based on schedule
        const targetValue = calculateTargetValue(strategy);
        
        // Get current position value
        const currentValue = await getCurrentPositionValue(ctx, strategy);
        
        // Calculate amount to invest
        const investmentAmount = targetValue - currentValue;
        
        if (investmentAmount <= 0) {
          console.log(`Strategy ${strategy._id}: Already at or above target value`);
          continue;
        }

        // Adjust for market conditions
        const adjustedAmount = adjustForMarketConditions(
          investmentAmount,
          strategy.config.conditions
        );

        // Execute trade
        await ctx.runMutation(api.mutations.trading.placeTrade, {
          userId: strategy.userId,
          walletId: strategy.walletId,
          type: "market",
          side: "buy",
          tokenIn: {
            mint: strategy.config.tokenIn.mint,
            symbol: strategy.config.tokenIn.symbol,
            amount: adjustedAmount.toString(),
            decimals: 9,
          },
          tokenOut: {
            mint: strategy.config.tokenOut.mint,
            symbol: strategy.config.tokenOut.symbol,
            decimals: 9,
          },
          slippage: 1.5,
        });

        console.log(`✅ Value averaging executed for strategy ${strategy._id}: $${adjustedAmount}`);
      } catch (error) {
        console.error(`Failed to process value averaging for ${strategy._id}:`, error);
      }
    }
  },
});

// Weekend DCA boost - execute all pending strategies
export const weekendBoost = action({
  handler: async (ctx) => {
    console.log("🚀 Weekend DCA Boost - Executing all pending strategies");
    
    // Get all active strategies
    const strategies = await ctx.runQuery(api.queries.dca.getActiveStrategies);
    
    const boostConfig = {
      multiplier: 1.5, // Invest 50% more on weekends
      maxStrategies: 50, // Limit to prevent overload
    };
    
    let executed = 0;
    
    for (const strategy of strategies.slice(0, boostConfig.maxStrategies)) {
      try {
        // Check weekend boost eligibility
        if (!strategy.config.advanced.weekendBoost) {
          continue;
        }

        const boostedAmount = (
          parseFloat(strategy.config.amount) * boostConfig.multiplier
        ).toString();

        // Execute boosted trade
        await ctx.runMutation(api.mutations.trading.placeTrade, {
          userId: strategy.userId,
          walletId: strategy.walletId,
          type: "market",
          side: "buy",
          tokenIn: {
            mint: strategy.config.tokenIn.mint,
            symbol: strategy.config.tokenIn.symbol,
            amount: boostedAmount,
            decimals: 9,
          },
          tokenOut: {
            mint: strategy.config.tokenOut.mint,
            symbol: strategy.config.tokenOut.symbol,
            decimals: 9,
          },
          slippage: 1.0,
        });

        executed++;
      } catch (error) {
        console.error(`Weekend boost failed for strategy ${strategy._id}:`, error);
      }
    }

    console.log(`✅ Weekend boost complete: ${executed} strategies executed`);
    return { executed };
  },
});

// Helper functions
async function checkStrategyConditions(ctx: any, strategy: any): Promise<boolean> {
  // Check if strategy has conditions
  if (!strategy.config.conditions) {
    return true;
  }

  const conditions = strategy.config.conditions;

  // Check price conditions
  if (conditions.minPrice || conditions.maxPrice) {
    const currentPrice = await ctx.runQuery(api.queries.prices.getCurrentPrice, {
      tokenMint: strategy.config.tokenOut.mint,
    });

    if (conditions.minPrice && parseFloat(currentPrice) < parseFloat(conditions.minPrice)) {
      return false;
    }

    if (conditions.maxPrice && parseFloat(currentPrice) > parseFloat(conditions.maxPrice)) {
      return false;
    }
  }

  // Check buy-the-dip condition
  if (conditions.onlyBuyDips) {
    const priceChange = await ctx.runQuery(api.queries.prices.getPriceChange24h, {
      tokenMint: strategy.config.tokenOut.mint,
    });

    const dipThreshold = conditions.dipThreshold || -5; // Default 5% dip
    if (priceChange > dipThreshold) {
      return false; // Not a dip
    }
  }

  // Check strategy limits
  if (strategy.config.limits.maxExecutions) {
    if (strategy.stats.totalExecutions >= strategy.config.limits.maxExecutions) {
      return false;
    }
  }

  if (strategy.config.limits.maxInvestment) {
    if (parseFloat(strategy.stats.totalInvested) >= parseFloat(strategy.config.limits.maxInvestment)) {
      return false;
    }
  }

  if (strategy.config.limits.endDate) {
    if (Date.now() > strategy.config.limits.endDate) {
      return false;
    }
  }

  return true;
}

function calculateNextExecution(frequency: any, currentTime: number): number {
  switch (frequency.type) {
    case "interval":
      // Parse interval like "1h", "30m", "1d"
      const interval = parseInterval(frequency.value);
      return currentTime + interval;
    
    case "cron":
      // Parse cron expression and find next execution
      return getNextCronExecution(frequency.value, currentTime);
    
    case "dynamic":
      // Dynamic scheduling based on market conditions
      return calculateDynamicExecution(frequency.value, currentTime);
    
    default:
      // Default to 1 hour
      return currentTime + (60 * 60 * 1000);
  }
}

function parseInterval(interval: string): number {
  const unit = interval.slice(-1);
  const value = parseInt(interval.slice(0, -1));
  
  switch (unit) {
    case 'm': return value * 60 * 1000;
    case 'h': return value * 60 * 60 * 1000;
    case 'd': return value * 24 * 60 * 60 * 1000;
    case 'w': return value * 7 * 24 * 60 * 60 * 1000;
    default: return 60 * 60 * 1000; // Default 1 hour
  }
}

function getNextCronExecution(cronExpression: string, currentTime: number): number {
  // Simplified cron parsing - in production would use a library
  // For now, return next hour
  return currentTime + (60 * 60 * 1000);
}

function calculateDynamicExecution(config: string, currentTime: number): number {
  // Dynamic scheduling based on volatility, volume, etc.
  // For now, return 2 hours
  return currentTime + (2 * 60 * 60 * 1000);
}

function calculateTargetValue(strategy: any): number {
  const executionNumber = strategy.stats.totalExecutions + 1;
  const baseAmount = parseFloat(strategy.config.amount);
  
  // Linear value averaging: target = base * execution number
  return baseAmount * executionNumber;
}

async function getCurrentPositionValue(ctx: any, strategy: any): Promise<number> {
  const position = await ctx.runQuery(api.queries.portfolio.getPosition, {
    userId: strategy.userId,
    tokenMint: strategy.config.tokenOut.mint,
  });

  return position ? parseFloat(position.marketValue) : 0;
}

function adjustForMarketConditions(amount: number, conditions: any): number {
  // Adjust investment amount based on market conditions
  // This is a simplified version
  
  if (conditions?.volatilityAdjustment) {
    // Reduce amount in high volatility
    return amount * 0.8;
  }
  
  return amount;
}