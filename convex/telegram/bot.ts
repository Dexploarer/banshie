import { ConvexHttpClient } from "convex/browser";
import { api } from "../convex/_generated/api";
import TelegramBot from "node-telegram-bot-api";

// Initialize Convex client
const convex = new ConvexHttpClient(process.env.CONVEX_URL!);

// Initialize Telegram bot
const bot = new TelegramBot(process.env.TELEGRAM_BOT_TOKEN!, { polling: true });

// Bot configuration
interface BotUser {
  id: string;
  sessionToken: string;
  username: string;
  isPremium: boolean;
}

const activeSessions = new Map<number, BotUser>();

// ============================================
// INLINE QUERIES - Search and Quick Actions
// ============================================

// Handle inline queries for token search
bot.on('inline_query', async (query) => {
  const queryText = query.query.toLowerCase();
  const results: any[] = [];

  try {
    if (queryText.length < 2) {
      // Show trending tokens if no query
      const trending = await convex.query(api.queries.prices.getTrending, {
        timeframe: "24h",
        metric: "volume",
      });

      trending.trending.forEach((token, index) => {
        results.push({
          type: 'article',
          id: `trending_${token.mint}`,
          title: `📈 ${token.symbol} - $${token.price}`,
          description: `${token.change24h > 0 ? '🟢' : '🔴'} ${token.change24h.toFixed(2)}% | Vol: $${formatNumber(token.volume24h)}`,
          input_message_content: {
            message_text: formatTokenInfo(token),
            parse_mode: 'HTML'
          },
          reply_markup: {
            inline_keyboard: [[
              { text: "📊 Chart", callback_data: `chart_${token.mint}` },
              { text: "💰 Trade", callback_data: `trade_${token.mint}` }
            ]]
          }
        });
      });
    } else {
      // Search tokens
      const searchResults = await convex.query(api.queries.prices.searchTokens, {
        query: queryText,
        limit: 10,
      });

      searchResults.results.forEach((token, index) => {
        const changeEmoji = token.change24h > 0 ? '🟢' : token.change24h < 0 ? '🔴' : '⚪';
        
        results.push({
          type: 'article',
          id: `search_${token.mint}`,
          title: `${token.symbol} - $${token.price}`,
          description: `${changeEmoji} ${token.change24h.toFixed(2)}% | ${token.name}`,
          input_message_content: {
            message_text: formatTokenInfo(token),
            parse_mode: 'HTML'
          },
          reply_markup: {
            inline_keyboard: [
              [
                { text: "📊 Chart", callback_data: `chart_${token.mint}` },
                { text: "💰 Buy", callback_data: `buy_${token.mint}` },
                { text: "📉 Sell", callback_data: `sell_${token.mint}` }
              ],
              [
                { text: "🔔 Alert", callback_data: `alert_${token.mint}` },
                { text: "🤖 DCA", callback_data: `dca_${token.mint}` }
              ]
            ]
          }
        });
      });
    }

    // Add quick actions
    if (queryText.startsWith('/portfolio')) {
      results.unshift({
        type: 'article',
        id: 'portfolio_quick',
        title: '💼 My Portfolio',
        description: 'View your current portfolio balance and positions',
        input_message_content: {
          message_text: '💼 <b>Loading Portfolio...</b>',
          parse_mode: 'HTML'
        }
      });
    }

    if (queryText.startsWith('/dca')) {
      results.unshift({
        type: 'article',
        id: 'dca_quick',
        title: '🤖 DCA Strategies',
        description: 'Manage your Dollar Cost Averaging strategies',
        input_message_content: {
          message_text: '🤖 <b>DCA Strategies</b>\n\nManage your automated investing...',
          parse_mode: 'HTML'
        }
      });
    }

    await bot.answerInlineQuery(query.id, results, {
      cache_time: 30,
      is_personal: true
    });

  } catch (error) {
    console.error('Inline query error:', error);
    await bot.answerInlineQuery(query.id, [{
      type: 'article',
      id: 'error',
      title: 'Error',
      description: 'Unable to process query',
      input_message_content: {
        message_text: '❌ Error processing request. Please try again.'
      }
    }]);
  }
});

// ============================================
// CUSTOM KEYBOARDS & CALLBACK HANDLERS
// ============================================

// Main menu keyboard
function getMainKeyboard(isPremium: boolean = false): any {
  const keyboard = [
    [
      { text: "💼 Portfolio", callback_data: "menu_portfolio" },
      { text: "💰 Trade", callback_data: "menu_trade" }
    ],
    [
      { text: "🤖 DCA", callback_data: "menu_dca" },
      { text: "🔔 Alerts", callback_data: "menu_alerts" }
    ],
    [
      { text: "📊 Market", callback_data: "menu_market" },
      { text: "📈 Analytics", callback_data: "menu_analytics" }
    ]
  ];

  if (isPremium) {
    keyboard.push([
      { text: "🎯 AI Signals", callback_data: "menu_ai_signals" },
      { text: "⚙️ Settings", callback_data: "menu_settings" }
    ]);
  } else {
    keyboard.push([
      { text: "⭐ Upgrade to Pro", callback_data: "menu_upgrade" }
    ]);
  }

  return { inline_keyboard: keyboard };
}

// Trading keyboard for a specific token
function getTradingKeyboard(tokenMint: string): any {
  return {
    inline_keyboard: [
      [
        { text: "💰 Quick Buy $10", callback_data: `quickbuy_${tokenMint}_10` },
        { text: "💰 Quick Buy $50", callback_data: `quickbuy_${tokenMint}_50` }
      ],
      [
        { text: "💰 Quick Buy $100", callback_data: `quickbuy_${tokenMint}_100` },
        { text: "💱 Custom Amount", callback_data: `custombuy_${tokenMint}` }
      ],
      [
        { text: "📊 Chart", callback_data: `chart_${tokenMint}` },
        { text: "🔔 Set Alert", callback_data: `alert_${tokenMint}` }
      ],
      [
        { text: "🤖 Start DCA", callback_data: `dca_setup_${tokenMint}` }
      ],
      [
        { text: "◀️ Back", callback_data: "menu_main" }
      ]
    ]
  };
}

// Portfolio keyboard
function getPortfolioKeyboard(): any {
  return {
    inline_keyboard: [
      [
        { text: "💎 Positions", callback_data: "portfolio_positions" },
        { text: "📊 Performance", callback_data: "portfolio_performance" }
      ],
      [
        { text: "📈 Top Gainers", callback_data: "portfolio_gainers" },
        { text: "📉 Top Losers", callback_data: "portfolio_losers" }
      ],
      [
        { text: "📱 Export", callback_data: "portfolio_export" },
        { text: "🔄 Refresh", callback_data: "portfolio_refresh" }
      ],
      [
        { text: "◀️ Main Menu", callback_data: "menu_main" }
      ]
    ]
  };
}

// Handle callback queries
bot.on('callback_query', async (callbackQuery) => {
  const message = callbackQuery.message;
  const data = callbackQuery.data;
  const userId = callbackQuery.from.id;

  if (!message) return;

  try {
    // Get user session
    const userSession = activeSessions.get(userId);
    if (!userSession) {
      await bot.answerCallbackQuery(callbackQuery.id, {
        text: "Please authenticate first using /start",
        show_alert: true
      });
      return;
    }

    await bot.answerCallbackQuery(callbackQuery.id);

    // Route callback based on action
    if (data.startsWith('menu_')) {
      await handleMenuCallback(message, data, userSession);
    } else if (data.startsWith('portfolio_')) {
      await handlePortfolioCallback(message, data, userSession);
    } else if (data.startsWith('trade_') || data.startsWith('buy_') || data.startsWith('sell_')) {
      await handleTradeCallback(message, data, userSession);
    } else if (data.startsWith('quickbuy_')) {
      await handleQuickBuy(message, data, userSession);
    } else if (data.startsWith('chart_')) {
      await handleChartCallback(message, data, userSession);
    } else if (data.startsWith('alert_')) {
      await handleAlertCallback(message, data, userSession);
    } else if (data.startsWith('dca_')) {
      await handleDCACallback(message, data, userSession);
    }

  } catch (error) {
    console.error('Callback query error:', error);
    await bot.sendMessage(message.chat.id, "❌ An error occurred processing your request.");
  }
});

// ============================================
// MENU HANDLERS
// ============================================

async function handleMenuCallback(message: any, data: string, userSession: BotUser) {
  const action = data.replace('menu_', '');

  switch (action) {
    case 'main':
      await bot.editMessageText("🤖 <b>Solana Trading Bot</b>\n\nWhat would you like to do?", {
        chat_id: message.chat.id,
        message_id: message.message_id,
        parse_mode: 'HTML',
        reply_markup: getMainKeyboard(userSession.isPremium)
      });
      break;

    case 'portfolio':
      const portfolio = await convex.query(api.queries.portfolio.getPortfolio, {
        userId: userSession.id as any,
      });
      
      const portfolioText = formatPortfolioSummary(portfolio);
      
      await bot.editMessageText(portfolioText, {
        chat_id: message.chat.id,
        message_id: message.message_id,
        parse_mode: 'HTML',
        reply_markup: getPortfolioKeyboard()
      });
      break;

    case 'trade':
      const trending = await convex.query(api.queries.prices.getTrending, {
        timeframe: "24h",
        metric: "volume",
      });

      let tradeText = "💰 <b>Trading</b>\n\n<b>🔥 Trending Tokens:</b>\n";
      trending.trending.slice(0, 5).forEach(token => {
        const emoji = token.change24h > 0 ? '🟢' : '🔴';
        tradeText += `${emoji} <b>${token.symbol}</b> $${token.price} (${token.change24h.toFixed(2)}%)\n`;
      });

      await bot.editMessageText(tradeText, {
        chat_id: message.chat.id,
        message_id: message.message_id,
        parse_mode: 'HTML',
        reply_markup: {
          inline_keyboard: [
            ...trending.trending.slice(0, 3).map(token => [
              { text: `💰 ${token.symbol}`, callback_data: `trade_${token.mint}` }
            ]),
            [{ text: "🔍 Search Token", switch_inline_query_current_chat: "" }],
            [{ text: "◀️ Main Menu", callback_data: "menu_main" }]
          ]
        }
      });
      break;

    case 'dca':
      await showDCAStrategies(message, userSession);
      break;

    case 'alerts':
      await showAlerts(message, userSession);
      break;

    case 'market':
      await showMarketOverview(message);
      break;

    case 'ai_signals':
      if (userSession.isPremium) {
        await showAISignals(message, userSession);
      } else {
        await showUpgradeMessage(message);
      }
      break;
  }
}

// ============================================
// PORTFOLIO HANDLERS
// ============================================

async function handlePortfolioCallback(message: any, data: string, userSession: BotUser) {
  const action = data.replace('portfolio_', '');

  switch (action) {
    case 'positions':
      const positions = await convex.query(api.queries.portfolio.getPortfolio, {
        userId: userSession.id as any,
      });

      let positionsText = "💎 <b>Your Positions</b>\n\n";
      
      if (positions.positions.length === 0) {
        positionsText += "No positions found. Start trading to see your portfolio here!";
      } else {
        positions.positions.forEach(pos => {
          const pnlEmoji = pos.pnl.isProfit ? '🟢' : '🔴';
          positionsText += `${pnlEmoji} <b>${pos.symbol}</b>\n`;
          positionsText += `   Amount: ${formatNumber(pos.amount)}\n`;
          positionsText += `   Value: $${formatNumber(pos.marketValue)}\n`;
          positionsText += `   P&L: ${pnlEmoji} ${pos.pnl.percentage.toFixed(2)}%\n\n`;
        });
      }

      await bot.editMessageText(positionsText, {
        chat_id: message.chat.id,
        message_id: message.message_id,
        parse_mode: 'HTML',
        reply_markup: getPortfolioKeyboard()
      });
      break;

    case 'performance':
      const performance = await convex.query(api.queries.portfolio.getTopPerformers, {
        userId: userSession.id as any,
        limit: 5,
      });

      let perfText = "📊 <b>Portfolio Performance</b>\n\n";
      
      if (performance.gainers.length > 0) {
        perfText += "🟢 <b>Top Gainers:</b>\n";
        performance.gainers.forEach(pos => {
          perfText += `  ${pos.symbol}: +${pos.pnl.percentage.toFixed(2)}%\n`;
        });
        perfText += "\n";
      }

      if (performance.losers.length > 0) {
        perfText += "🔴 <b>Top Losers:</b>\n";
        performance.losers.forEach(pos => {
          perfText += `  ${pos.symbol}: ${pos.pnl.percentage.toFixed(2)}%\n`;
        });
      }

      await bot.editMessageText(perfText, {
        chat_id: message.chat.id,
        message_id: message.message_id,
        parse_mode: 'HTML',
        reply_markup: getPortfolioKeyboard()
      });
      break;

    case 'refresh':
      // Trigger wallet sync
      await bot.editMessageText("🔄 <b>Refreshing Portfolio...</b>\n\nSyncing with blockchain...", {
        chat_id: message.chat.id,
        message_id: message.message_id,
        parse_mode: 'HTML'
      });

      // Get user's wallets and sync them
      const wallets = await convex.query(api.queries.wallets.getUserWallets, {
        userId: userSession.id as any,
      });

      for (const wallet of wallets) {
        await convex.action(api.actions.solana.syncWalletBalance, {
          walletId: wallet._id,
        });
      }

      // Show updated portfolio
      setTimeout(async () => {
        const updatedPortfolio = await convex.query(api.queries.portfolio.getPortfolio, {
          userId: userSession.id as any,
        });
        
        await bot.editMessageText(formatPortfolioSummary(updatedPortfolio), {
          chat_id: message.chat.id,
          message_id: message.message_id,
          parse_mode: 'HTML',
          reply_markup: getPortfolioKeyboard()
        });
      }, 3000);
      break;
  }
}

// ============================================
// TRADING HANDLERS
// ============================================

async function handleQuickBuy(message: any, data: string, userSession: BotUser) {
  const [, tokenMint, amount] = data.split('_');
  
  // Show confirmation
  const token = await convex.query(api.queries.prices.getTokenInfo, {
    tokenMint,
  });

  const confirmText = `💰 <b>Quick Buy Confirmation</b>\n\n` +
    `Token: <b>${token.symbol}</b>\n` +
    `Amount: <b>$${amount}</b>\n` +
    `Price: <b>$${token.price}</b>\n\n` +
    `Proceed with purchase?`;

  await bot.editMessageText(confirmText, {
    chat_id: message.chat.id,
    message_id: message.message_id,
    parse_mode: 'HTML',
    reply_markup: {
      inline_keyboard: [
        [
          { text: "✅ Confirm", callback_data: `confirm_buy_${tokenMint}_${amount}` },
          { text: "❌ Cancel", callback_data: `trade_${tokenMint}` }
        ]
      ]
    }
  });
}

// ============================================
// AUTHENTICATION
// ============================================

bot.onText(/\/start/, async (msg) => {
  const chatId = msg.chat.id;
  const user = msg.from;

  if (!user) return;

  try {
    // Authenticate with Convex
    const auth = await convex.mutation(api.mutations.auth.authenticateWithTelegram, {
      telegramId: user.id,
      username: user.username || user.first_name || 'Unknown',
      firstName: user.first_name,
      lastName: user.last_name,
      authDate: Math.floor(Date.now() / 1000),
      hash: "mock_hash", // In production, verify with real hash
    });

    // Store session
    activeSessions.set(user.id, {
      id: auth.user.id,
      sessionToken: auth.session.token,
      username: auth.user.username,
      isPremium: auth.user.isPremium,
    });

    const welcomeText = `🤖 <b>Welcome to Solana Trading Bot!</b>\n\n` +
      `👋 Hello ${user.first_name}!\n` +
      `🎯 Your account is ${auth.user.isPremium ? 'Premium ⭐' : 'Free'}\n\n` +
      `<b>Quick Start:</b>\n` +
      `• Type @${process.env.BOT_USERNAME} in any chat for quick token search\n` +
      `• Use the menu below to navigate\n` +
      `• Check out trending tokens and start trading!\n\n` +
      `Ready to start trading? 🚀`;

    await bot.sendMessage(chatId, welcomeText, {
      parse_mode: 'HTML',
      reply_markup: getMainKeyboard(auth.user.isPremium)
    });

  } catch (error) {
    console.error('Authentication error:', error);
    await bot.sendMessage(chatId, "❌ Authentication failed. Please try again.");
  }
});

// ============================================
// HELPER FUNCTIONS
// ============================================

function formatTokenInfo(token: any): string {
  const changeEmoji = token.change24h > 0 ? '🟢' : token.change24h < 0 ? '🔴' : '⚪';
  
  return `${changeEmoji} <b>${token.symbol}</b> - $${token.price}\n` +
    `📊 24h Change: ${token.change24h.toFixed(2)}%\n` +
    `💰 Volume: $${formatNumber(token.volume24h)}\n` +
    `📈 Market Cap: $${formatNumber(token.marketCap)}`;
}

function formatPortfolioSummary(portfolio: any): string {
  return `💼 <b>Portfolio Summary</b>\n\n` +
    `💰 Total Value: <b>$${formatNumber(portfolio.summary.totalValue)}</b>\n` +
    `📊 Total P&L: ${portfolio.summary.totalPnL.startsWith('-') ? '🔴' : '🟢'} <b>$${portfolio.summary.totalPnL}</b>\n` +
    `📈 P&L %: <b>${portfolio.summary.totalPnLPercentage}%</b>\n` +
    `💎 Positions: <b>${portfolio.summary.positionCount}</b>\n` +
    `🏦 Wallets: <b>${portfolio.wallets.length}</b>\n\n` +
    `<i>Last updated: ${new Date(portfolio.summary.lastUpdated).toLocaleTimeString()}</i>`;
}

function formatNumber(num: string | number): string {
  const n = typeof num === 'string' ? parseFloat(num) : num;
  if (n >= 1e9) return (n / 1e9).toFixed(2) + 'B';
  if (n >= 1e6) return (n / 1e6).toFixed(2) + 'M';
  if (n >= 1e3) return (n / 1e3).toFixed(2) + 'K';
  return n.toFixed(2);
}

async function showDCAStrategies(message: any, userSession: BotUser) {
  // Implementation for DCA strategies view
}

async function showAlerts(message: any, userSession: BotUser) {
  // Implementation for alerts view
}

async function showMarketOverview(message: any) {
  // Implementation for market overview
}

async function showAISignals(message: any, userSession: BotUser) {
  // Implementation for AI signals
}

async function showUpgradeMessage(message: any) {
  // Implementation for upgrade message
}

console.log("🤖 Telegram bot started with Convex integration!");

export { bot };