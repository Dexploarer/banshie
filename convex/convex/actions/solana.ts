import { action } from "../_generated/server";
import { v } from "convex/values";
import { api } from "../_generated/api";
import axios from "axios";

// Jupiter API configuration
const JUPITER_API = "https://quote-api.jup.ag/v6";
const JUPITER_PRICE_API = "https://api.jup.ag/price/v2";
const RPC_ENDPOINT = process.env.SOLANA_RPC_URL || "https://api.mainnet-beta.solana.com";

// Execute a trade on Solana
export const executeTrade = action({
  args: {
    orderId: v.id("orders"),
  },
  handler: async (ctx, args) => {
    try {
      // Get order details
      const order = await ctx.runQuery(api.queries.orders.getOrder, {
        orderId: args.orderId,
      });

      if (!order) {
        throw new Error("Order not found");
      }

      // Update status to executing
      await ctx.runMutation(api.mutations.trading.updateOrderStatus, {
        orderId: args.orderId,
        status: "executing",
      });

      // Get Jupiter quote
      const quote = await getJupiterQuote({
        inputMint: order.tokenIn.mint,
        outputMint: order.tokenOut.mint,
        amount: order.tokenIn.amount,
        slippageBps: Math.floor(order.pricing.slippage * 100),
      });

      // Build swap transaction
      const swapResult = await buildSwapTransaction({
        quoteResponse: quote,
        userPublicKey: order.wallet.publicKey,
        wrapAndUnwrapSol: true,
        feeAccount: process.env.FEE_ACCOUNT,
      });

      // Here we would sign and send the transaction
      // For now, simulate success
      const txSignature = `sim_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

      // Update order with success
      await ctx.runMutation(api.mutations.trading.updateOrderStatus, {
        orderId: args.orderId,
        status: "completed",
        execution: {
          txSignature,
          blockHeight: 250000000 + Math.floor(Math.random() * 1000),
          gasUsed: "5000",
          actualPrice: quote.price,
          actualOut: quote.outAmount,
        },
      });

      // Record the trade
      await ctx.runMutation(api.mutations.trading.recordTrade, {
        userId: order.userId,
        walletId: order.walletId,
        orderId: args.orderId,
        type: order.type,
        side: order.side,
        tokenIn: {
          ...order.tokenIn,
          price: quote.price,
          value: (parseFloat(order.tokenIn.amount) * parseFloat(quote.price)).toString(),
        },
        tokenOut: {
          ...order.tokenOut,
          amount: quote.outAmount,
          price: quote.price,
          value: quote.outAmount,
        },
        execution: {
          dex: quote.routePlan[0]?.swapInfo?.label || "Jupiter",
          txSignature,
          blockHeight: 250000000,
          slot: 250000000,
          gasUsed: "5000",
          gasCost: "0.00005",
        },
        fees: {
          network: "0.00005",
          dex: quote.routePlan[0]?.swapInfo?.feeAmount || "0",
          platform: "0",
          total: "0.00005",
        },
      });

      return {
        success: true,
        txSignature,
        actualOut: quote.outAmount,
      };
    } catch (error: any) {
      // Update order with failure
      await ctx.runMutation(api.mutations.trading.updateOrderStatus, {
        orderId: args.orderId,
        status: "failed",
        error: {
          code: "EXECUTION_FAILED",
          message: error.message || "Failed to execute trade",
          details: error.stack,
        },
      });

      throw error;
    }
  },
});

// Sync wallet balance from blockchain
export const syncWalletBalance = action({
  args: {
    walletId: v.id("wallets"),
  },
  handler: async (ctx, args) => {
    // Get wallet details
    const wallet = await ctx.runQuery(api.queries.wallets.getWallet, {
      walletId: args.walletId,
    });

    if (!wallet) {
      throw new Error("Wallet not found");
    }

    try {
      // Get SOL balance
      const balanceResponse = await axios.post(RPC_ENDPOINT, {
        jsonrpc: "2.0",
        id: 1,
        method: "getBalance",
        params: [wallet.address],
      });

      const solBalance = (balanceResponse.data.result.value / 1e9).toString();

      // Get token accounts
      const tokenAccountsResponse = await axios.post(RPC_ENDPOINT, {
        jsonrpc: "2.0",
        id: 1,
        method: "getTokenAccountsByOwner",
        params: [
          wallet.address,
          { programId: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" },
          { encoding: "jsonParsed" },
        ],
      });

      const tokenAccounts = tokenAccountsResponse.data.result.value || [];
      
      // Parse token balances
      const tokens = tokenAccounts.map((account: any) => {
        const info = account.account.data.parsed.info;
        return {
          mint: info.mint,
          symbol: "UNKNOWN", // Would need to look up
          amount: info.tokenAmount.uiAmountString,
          usdValue: "0", // Would calculate
        };
      });

      // Calculate total USD value
      const solPrice = await getSolPrice();
      const solUsdValue = parseFloat(solBalance) * solPrice;
      const totalUsdValue = solUsdValue; // + token values

      // Update wallet balance
      await ctx.runMutation(api.mutations.wallets.updateBalance, {
        walletId: args.walletId,
        balance: {
          sol: solBalance,
          usd: totalUsdValue.toString(),
          tokens,
          lastUpdated: Date.now(),
        },
      });

      // Update positions for each token
      for (const token of tokens) {
        if (parseFloat(token.amount) > 0) {
          await ctx.runMutation(api.mutations.trading.updatePosition, {
            userId: wallet.userId,
            walletId: args.walletId,
            tokenMint: token.mint,
            symbol: token.symbol,
            name: token.symbol,
            amount: token.amount,
            price: "0", // Would get actual price
            action: "update",
          });
        }
      }

      return {
        success: true,
        sol: solBalance,
        tokens: tokens.length,
        usdValue: totalUsdValue.toString(),
      };
    } catch (error: any) {
      console.error("Failed to sync wallet:", error);
      throw new Error(`Wallet sync failed: ${error.message}`);
    }
  },
});

// Fetch and update price feeds
export const updatePriceFeeds = action({
  args: {
    tokens: v.array(v.string()),
  },
  handler: async (ctx, args) => {
    try {
      // Batch request to Jupiter Price API
      const ids = args.tokens.join(",");
      const response = await axios.get(`${JUPITER_PRICE_API}?ids=${ids}`);
      
      const prices = response.data.data || {};
      const updates = [];

      for (const [mint, data] of Object.entries(prices) as any) {
        const priceData = {
          tokenMint: mint,
          symbol: data.symbol || "UNKNOWN",
          name: data.name || data.symbol || "Unknown Token",
          price: data.price?.toString() || "0",
          prices: {
            usd: data.price?.toString() || "0",
            sol: (data.price / (await getSolPrice())).toString(),
          },
          metrics: {
            volume24h: data.volume24h?.toString() || "0",
            volumeChange24h: data.volumeChange24h || 0,
            marketCap: data.marketCap?.toString() || "0",
            fdv: data.fdv?.toString(),
            circulatingSupply: data.circulatingSupply?.toString(),
            totalSupply: data.totalSupply?.toString(),
          },
          changes: {
            price1h: data.priceChange1h || 0,
            price24h: data.priceChange24h || 0,
            price7d: data.priceChange7d || 0,
            price30d: data.priceChange30d || 0,
          },
          source: {
            primary: "Jupiter",
            lastUpdate: Date.now(),
            confidence: 0.95,
          },
          timestamp: Date.now(),
        };

        // Store in database
        await ctx.runMutation(api.mutations.prices.upsertPriceFeed, priceData);
        updates.push(mint);
      }

      return {
        success: true,
        updated: updates.length,
        tokens: updates,
      };
    } catch (error: any) {
      console.error("Failed to update prices:", error);
      throw new Error(`Price update failed: ${error.message}`);
    }
  },
});

// Monitor blockchain for events
export const monitorBlockchain = action({
  args: {
    walletAddresses: v.array(v.string()),
    startSlot: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    try {
      // Get recent transactions for wallets
      const transactions = [];
      
      for (const address of args.walletAddresses) {
        const response = await axios.post(RPC_ENDPOINT, {
          jsonrpc: "2.0",
          id: 1,
          method: "getSignaturesForAddress",
          params: [
            address,
            {
              limit: 10,
              commitment: "confirmed",
            },
          ],
        });

        const signatures = response.data.result || [];
        
        for (const sig of signatures) {
          // Get transaction details
          const txResponse = await axios.post(RPC_ENDPOINT, {
            jsonrpc: "2.0",
            id: 1,
            method: "getTransaction",
            params: [
              sig.signature,
              {
                encoding: "jsonParsed",
                commitment: "confirmed",
              },
            ],
          });

          const tx = txResponse.data.result;
          if (tx) {
            transactions.push({
              signature: sig.signature,
              slot: tx.slot,
              blockTime: tx.blockTime,
              fee: tx.meta.fee,
              status: tx.meta.err ? "failed" : "success",
            });
          }
        }
      }

      // Process and store relevant transactions
      for (const tx of transactions) {
        // Would parse and store trade-related transactions
        console.log("Found transaction:", tx.signature);
      }

      return {
        success: true,
        transactions: transactions.length,
      };
    } catch (error: any) {
      console.error("Blockchain monitoring error:", error);
      throw new Error(`Monitoring failed: ${error.message}`);
    }
  },
});

// Helper functions
async function getJupiterQuote(params: {
  inputMint: string;
  outputMint: string;
  amount: string;
  slippageBps: number;
}) {
  try {
    const response = await axios.get(`${JUPITER_API}/quote`, {
      params: {
        inputMint: params.inputMint,
        outputMint: params.outputMint,
        amount: params.amount,
        slippageBps: params.slippageBps,
        swapMode: "ExactIn",
        onlyDirectRoutes: false,
        asLegacyTransaction: false,
      },
    });

    return {
      ...response.data,
      price: (parseFloat(response.data.outAmount) / parseFloat(params.amount)).toString(),
    };
  } catch (error: any) {
    throw new Error(`Failed to get quote: ${error.message}`);
  }
}

async function buildSwapTransaction(params: {
  quoteResponse: any;
  userPublicKey: string;
  wrapAndUnwrapSol: boolean;
  feeAccount?: string;
}) {
  try {
    const response = await axios.post(`${JUPITER_API}/swap`, {
      quoteResponse: params.quoteResponse,
      userPublicKey: params.userPublicKey,
      wrapAndUnwrapSol: params.wrapAndUnwrapSol,
      computeUnitPriceMicroLamports: 50000,
      dynamicComputeUnitLimit: true,
      feeAccount: params.feeAccount,
    });

    return response.data;
  } catch (error: any) {
    throw new Error(`Failed to build swap: ${error.message}`);
  }
}

async function getSolPrice(): Promise<number> {
  try {
    const response = await axios.get(
      `${JUPITER_PRICE_API}?ids=So11111111111111111111111111111111111111112`
    );
    return response.data.data?.So11111111111111111111111111111111111111112?.price || 100;
  } catch {
    return 100; // Default fallback
  }
}