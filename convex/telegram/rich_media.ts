import { Bot, InlineKeyboard, InputFile } from "grammy";
import { ConvexClient } from "../rust-integration/src/convex_client";

interface MediaMessage {
  chatId: number;
  imageBase64: string;
  caption: string;
  replyMarkup?: InlineKeyboard;
}

interface ChartOptions {
  tokenMint: string;
  symbol: string;
  interval?: string;
  period?: number;
  chartType?: string;
  indicators?: string[];
  theme?: string;
}

interface PortfolioOptions {
  userId: string;
  theme?: string;
  includeChart?: boolean;
}

export class RichMediaHandler {
  private bot: Bot;
  private convex: ConvexClient;

  constructor(bot: Bot, convex: ConvexClient) {
    this.bot = bot;
    this.convex = convex;
  }

  // Send price chart with interactive controls
  async sendPriceChart(chatId: number, options: ChartOptions, messageId?: number): Promise<void> {
    try {
      console.log(`📊 Sending price chart for ${options.symbol} to chat ${chatId}`);

      // Generate chart image
      const chartResult = await this.convex.action("actions/media_generator:generatePriceChart", {
        tokenMint: options.tokenMint,
        symbol: options.symbol,
        interval: options.interval || "1h",
        period: options.period || 168,
        chartType: options.chartType || "candlestick",
        indicators: options.indicators || [],
        theme: options.theme || "dark",
      });

      // Create interactive keyboard
      const keyboard = new InlineKeyboard()
        .text("1H", `chart_${options.tokenMint}_1h`).text("4H", `chart_${options.tokenMint}_4h`).text("1D", `chart_${options.tokenMint}_1d`)
        .row()
        .text("📈 Line", `chart_type_${options.tokenMint}_line`).text("🕯️ Candles", `chart_type_${options.tokenMint}_candlestick`).text("📊 Area", `chart_type_${options.tokenMint}_area`)
        .row()
        .text("+ Indicators", `indicators_${options.tokenMint}`).text("🎨 Theme", `theme_${options.tokenMint}`)
        .row()
        .text("💱 Quick Trade", `trade_${options.tokenMint}`).text("🔔 Set Alert", `alert_${options.tokenMint}`)
        .row()
        .text("🧠 AI Analysis", `analysis_${options.tokenMint}`).text("🔄 Refresh", `refresh_chart_${options.tokenMint}`);

      // Create caption with current info
      const currentPrice = await this.getCurrentPrice(options.tokenMint);
      const caption = this.createChartCaption(options.symbol, currentPrice, chartResult);

      // Convert base64 to buffer
      const imageBuffer = Buffer.from(chartResult.imageBase64, 'base64');
      const inputFile = new InputFile(imageBuffer, `${options.symbol}_chart.png`);

      if (messageId) {
        // Edit existing message
        await this.bot.api.editMessageMedia(chatId, messageId, {
          type: "photo",
          media: inputFile,
          caption: caption,
          parse_mode: "Markdown",
        }, {
          reply_markup: keyboard,
        });
      } else {
        // Send new message
        await this.bot.api.sendPhoto(chatId, inputFile, {
          caption: caption,
          parse_mode: "Markdown",
          reply_markup: keyboard,
        });
      }

      console.log(`✅ Price chart sent for ${options.symbol}`);
    } catch (error) {
      console.error(`❌ Error sending price chart:`, error);
      await this.bot.api.sendMessage(chatId, `❌ Failed to generate chart for ${options.symbol}: ${error.message}`);
    }
  }

  // Send portfolio overview with pie chart
  async sendPortfolioOverview(chatId: number, options: PortfolioOptions): Promise<void> {
    try {
      console.log(`📊 Sending portfolio overview to chat ${chatId}`);

      // Generate portfolio image
      const portfolioResult = await this.convex.action("actions/media_generator:generatePortfolioImage", {
        userId: options.userId,
        theme: options.theme || "dark",
        includeChart: options.includeChart || true,
      });

      // Get portfolio data for caption
      const portfolio = await this.convex.query("queries/portfolio:getPortfolio", {
        userId: options.userId,
      });

      // Create interactive keyboard
      const keyboard = new InlineKeyboard()
        .text("📊 Detailed View", `portfolio_detailed_${options.userId}`).text("🔄 Refresh", `portfolio_refresh_${options.userId}`)
        .row()
        .text("💱 Rebalance", `rebalance_${options.userId}`).text("📈 Performance", `performance_${options.userId}`)
        .row()
        .text("🎯 Top Movers", `top_movers_${options.userId}`).text("⚠️ Alerts", `portfolio_alerts_${options.userId}`)
        .row()
        .text("💰 Quick Trade", "quick_trade").text("🤖 AI Suggestions", `ai_suggest_${options.userId}`);

      // Create caption
      const caption = this.createPortfolioCaption(portfolio);

      // Send image
      const imageBuffer = Buffer.from(portfolioResult.imageBase64, 'base64');
      const inputFile = new InputFile(imageBuffer, 'portfolio_overview.png');

      await this.bot.api.sendPhoto(chatId, inputFile, {
        caption: caption,
        parse_mode: "Markdown",
        reply_markup: keyboard,
      });

      console.log(`✅ Portfolio overview sent`);
    } catch (error) {
      console.error(`❌ Error sending portfolio overview:`, error);
      await this.bot.api.sendMessage(chatId, `❌ Failed to generate portfolio overview: ${error.message}`);
    }
  }

  // Send technical analysis summary
  async sendTechnicalAnalysis(chatId: number, tokenMint: string, symbol: string): Promise<void> {
    try {
      console.log(`📊 Sending technical analysis for ${symbol}`);

      // Generate analysis image
      const analysisResult = await this.convex.action("actions/media_generator:generateTechnicalAnalysisImage", {
        tokenMint: tokenMint,
        symbol: symbol,
        analysisType: "overview",
        theme: "dark",
      });

      // Get AI analysis for caption
      const aiAnalysis = await this.convex.query("queries/ai:getLatestAnalysis", {
        targetId: tokenMint,
        type: "technical",
        limit: 1,
      });

      // Create keyboard
      const keyboard = new InlineKeyboard()
        .text("📈 Price Chart", `chart_${tokenMint}`).text("🧠 AI Analysis", `ai_full_${tokenMint}`)
        .row()
        .text("📊 All Indicators", `indicators_all_${tokenMint}`).text("🎯 Trading Signals", `signals_${tokenMint}`)
        .row()
        .text("⏰ Set Alerts", `alert_setup_${tokenMint}`).text("💱 Trade Now", `trade_${tokenMint}`);

      // Create caption
      const caption = this.createAnalysisCaption(symbol, aiAnalysis[0], analysisResult);

      // Send image
      const imageBuffer = Buffer.from(analysisResult.imageBase64, 'base64');
      const inputFile = new InputFile(imageBuffer, `${symbol}_analysis.png`);

      await this.bot.api.sendPhoto(chatId, inputFile, {
        caption: caption,
        parse_mode: "Markdown",
        reply_markup: keyboard,
      });

      console.log(`✅ Technical analysis sent for ${symbol}`);
    } catch (error) {
      console.error(`❌ Error sending technical analysis:`, error);
      await this.bot.api.sendMessage(chatId, `❌ Failed to generate technical analysis: ${error.message}`);
    }
  }

  // Send trading signal card
  async sendTradingSignal(chatId: number, signalId: string): Promise<void> {
    try {
      console.log(`📊 Sending trading signal card for ${signalId}`);

      // Generate signal card
      const signalResult = await this.convex.action("actions/media_generator:generateSignalCard", {
        signalId: signalId,
        theme: "dark",
        includeChart: true,
      });

      // Get signal data
      const signal = await this.convex.query("queries/ai:getLatestSignals", {
        limit: 1,
      });

      if (!signal || signal.length === 0) {
        throw new Error("Signal not found");
      }

      const signalData = signal[0];

      // Create action keyboard based on signal
      const keyboard = this.createSignalKeyboard(signalData);

      // Create caption
      const caption = this.createSignalCaption(signalData);

      // Send image
      const imageBuffer = Buffer.from(signalResult.imageBase64, 'base64');
      const inputFile = new InputFile(imageBuffer, `${signalData.symbol}_signal.png`);

      await this.bot.api.sendPhoto(chatId, inputFile, {
        caption: caption,
        parse_mode: "Markdown",
        reply_markup: keyboard,
      });

      console.log(`✅ Trading signal sent`);
    } catch (error) {
      console.error(`❌ Error sending trading signal:`, error);
      await this.bot.api.sendMessage(chatId, `❌ Failed to generate trading signal: ${error.message}`);
    }
  }

  // Send market overview
  async sendMarketOverview(chatId: number, category: string = "trending"): Promise<void> {
    try {
      console.log(`📊 Sending market overview (${category})`);

      // Generate market overview
      const overviewResult = await this.convex.action("actions/media_generator:generateMarketOverview", {
        category: category,
        limit: 10,
        theme: "dark",
      });

      // Create keyboard
      const keyboard = new InlineKeyboard()
        .text("📈 Trending", "market_trending").text("🚀 Movers", "market_movers").text("📊 Volume", "market_volume")
        .row()
        .text("🔍 Search Token", "token_search").text("💡 AI Picks", "ai_picks")
        .row()
        .text("🔄 Refresh", `market_refresh_${category}`).text("⚙️ Settings", "market_settings");

      // Create caption
      const caption = `📊 **Market ${category.charAt(0).toUpperCase() + category.slice(1)}**\n\n` +
                     `Updated: ${new Date().toLocaleTimeString()}\n\n` +
                     `Use the buttons below to explore different market views or search for specific tokens.`;

      // Send image
      const imageBuffer = Buffer.from(overviewResult.imageBase64, 'base64');
      const inputFile = new InputFile(imageBuffer, `market_${category}.png`);

      await this.bot.api.sendPhoto(chatId, inputFile, {
        caption: caption,
        parse_mode: "Markdown",
        reply_markup: keyboard,
      });

      console.log(`✅ Market overview sent`);
    } catch (error) {
      console.error(`❌ Error sending market overview:`, error);
      await this.bot.api.sendMessage(chatId, `❌ Failed to generate market overview: ${error.message}`);
    }
  }

  // Send animated price ticker (GIF-like effect)
  async sendAnimatedTicker(chatId: number, tokens: string[]): Promise<void> {
    try {
      console.log(`📊 Sending animated ticker for ${tokens.length} tokens`);

      // Create multiple frames for animation effect
      const frames: Buffer[] = [];
      
      for (let i = 0; i < 5; i++) {
        // Generate frame with different highlight
        const frame = await this.generateTickerFrame(tokens, i);
        frames.push(frame);
      }

      // Create GIF from frames (placeholder - would use actual GIF library)
      const gifBuffer = await this.createGifFromFrames(frames);
      const inputFile = new InputFile(gifBuffer, 'price_ticker.gif');

      // Create keyboard
      const keyboard = new InlineKeyboard()
        .text("📊 Full Charts", "charts_all").text("🔄 Refresh", "ticker_refresh")
        .row()
        .text("⚙️ Customize", "ticker_settings").text("🔔 Set Alerts", "alerts_multiple");

      await this.bot.api.sendAnimation(chatId, inputFile, {
        caption: "📊 **Live Price Ticker**\n\nReal-time price updates for your watchlist",
        parse_mode: "Markdown",
        reply_markup: keyboard,
      });

      console.log(`✅ Animated ticker sent`);
    } catch (error) {
      console.error(`❌ Error sending animated ticker:`, error);
      await this.bot.api.sendMessage(chatId, `❌ Failed to generate animated ticker: ${error.message}`);
    }
  }

  // Handle callback queries for interactive charts
  async handleChartCallback(callbackQuery: any): Promise<void> {
    const data = callbackQuery.data;
    const chatId = callbackQuery.message?.chat.id;
    const messageId = callbackQuery.message?.message_id;

    if (!chatId || !messageId) return;

    try {
      if (data.startsWith('chart_')) {
        // Parse chart callback data
        const parts = data.split('_');
        const tokenMint = parts[1];
        const interval = parts[2];

        // Get token info
        const tokenInfo = await this.convex.query("queries/prices:getTokenPrice", {
          mint: tokenMint,
        });

        if (!tokenInfo) return;

        // Update chart with new interval
        await this.sendPriceChart(chatId, {
          tokenMint: tokenMint,
          symbol: tokenInfo.symbol,
          interval: interval,
        }, messageId);

        await this.bot.api.answerCallbackQuery(callbackQuery.id, {
          text: `📊 Chart updated to ${interval.toUpperCase()}`,
        });
      }
      // Handle other callback types...
    } catch (error) {
      console.error('Error handling chart callback:', error);
      await this.bot.api.answerCallbackQuery(callbackQuery.id, {
        text: "❌ Failed to update chart",
      });
    }
  }

  // Helper methods

  private async getCurrentPrice(tokenMint: string): Promise<any> {
    try {
      return await this.convex.query("queries/prices:getTokenPrice", {
        mint: tokenMint,
      });
    } catch (error) {
      console.error('Error getting current price:', error);
      return null;
    }
  }

  private createChartCaption(symbol: string, priceData: any, chartResult: any): string {
    if (!priceData) {
      return `📊 **${symbol} Price Chart**\n\nChart generated with ${chartResult.dataPoints} data points`;
    }

    const priceChange = priceData.priceChange24h || 0;
    const changeEmoji = priceChange >= 0 ? "📈" : "📉";
    const changeSign = priceChange >= 0 ? "+" : "";

    return `📊 **${symbol} Price Chart**
    
💰 **Current Price:** $${priceData.price?.toFixed(6) || 'N/A'}
${changeEmoji} **24h Change:** ${changeSign}${priceChange.toFixed(2)}%
📊 **Volume:** ${priceData.volume24h ? `$${(priceData.volume24h / 1000000).toFixed(2)}M` : 'N/A'}

🕐 **Chart:** ${chartResult.interval.toUpperCase()} • ${chartResult.period} periods
📅 **Updated:** ${new Date().toLocaleTimeString()}

Use the buttons below to customize your chart view.`;
  }

  private createPortfolioCaption(portfolio: any): string {
    if (!portfolio) {
      return "📊 **Portfolio Overview**\n\nNo portfolio data available";
    }

    const { summary } = portfolio;
    const pnlEmoji = summary.totalPnL.startsWith('-') ? "📉" : "📈";
    const pnlColor = summary.totalPnL.startsWith('-') ? "" : "+";

    return `💼 **Portfolio Overview**

💰 **Total Value:** $${summary.totalValue}
${pnlEmoji} **P&L:** ${pnlColor}${summary.totalPnL} (${summary.totalPnLPercentage}%)
🎯 **Positions:** ${summary.positionCount}

📅 **Updated:** ${new Date().toLocaleTimeString()}

Tap buttons below for detailed analysis and trading options.`;
  }

  private createAnalysisCaption(symbol: string, aiAnalysis: any, analysisResult: any): string {
    let caption = `🔬 **${symbol} Technical Analysis**\n\n`;

    if (aiAnalysis) {
      const { analysis } = aiAnalysis;
      const scoreEmoji = analysis.score > 50 ? "🟢" : analysis.score < -50 ? "🔴" : "🟡";
      
      caption += `${scoreEmoji} **AI Score:** ${analysis.score}/100\n`;
      caption += `🎯 **Confidence:** ${Math.round(analysis.confidence * 100)}%\n`;
      caption += `📊 **Recommendation:** ${analysis.recommendation.toUpperCase()}\n\n`;
      caption += `💭 **Summary:** ${analysis.summary.substring(0, 150)}...\n\n`;
    }

    caption += `📈 **Indicators:** ${analysisResult.indicators.join(', ')}\n`;
    caption += `📅 **Updated:** ${new Date().toLocaleTimeString()}`;

    return caption;
  }

  private createSignalCaption(signal: any): string {
    const actionEmoji = {
      buy: "🟢",
      sell: "🔴",
      hold: "🟡"
    }[signal.action] || "⚪";

    const strengthBar = "█".repeat(Math.floor(signal.strength / 20)) + "░".repeat(5 - Math.floor(signal.strength / 20));
    const confidenceBar = "█".repeat(Math.floor(signal.confidence / 20)) + "░".repeat(5 - Math.floor(signal.confidence / 20));

    return `${actionEmoji} **${signal.symbol} Trading Signal**

🎯 **Action:** ${signal.action.toUpperCase()}
📊 **Type:** ${signal.signalType}
⚡ **Strength:** ${signal.strength}/100 ${strengthBar}
🎯 **Confidence:** ${signal.confidence}% ${confidenceBar}

💰 **Price Target:** $${signal.priceTarget?.toFixed(6) || 'N/A'}
🛡️ **Stop Loss:** $${signal.stopLoss?.toFixed(6) || 'N/A'}
⏰ **Timeframe:** ${signal.timeframe}
⚠️ **Risk:** ${signal.riskLevel.toUpperCase()}

💭 **Reasoning:** ${signal.reasoning.substring(0, 200)}...

🕐 **Valid Until:** ${new Date(signal.validUntil).toLocaleString()}`;
  }

  private createSignalKeyboard(signal: any): InlineKeyboard {
    const keyboard = new InlineKeyboard();

    if (signal.action === 'buy') {
      keyboard
        .text("💰 Buy $10", `quick_buy_${signal.tokenMint}_10`)
        .text("💰 Buy $50", `quick_buy_${signal.tokenMint}_50`)
        .text("💰 Buy $100", `quick_buy_${signal.tokenMint}_100`)
        .row();
    } else if (signal.action === 'sell') {
      keyboard
        .text("📉 Sell 25%", `quick_sell_${signal.tokenMint}_25`)
        .text("📉 Sell 50%", `quick_sell_${signal.tokenMint}_50`)
        .text("📉 Sell 100%", `quick_sell_${signal.tokenMint}_100`)
        .row();
    }

    keyboard
      .text("📊 Full Analysis", `analysis_${signal.tokenMint}`)
      .text("📈 Price Chart", `chart_${signal.tokenMint}`)
      .row()
      .text("🔔 Set Alert", `alert_${signal.tokenMint}`)
      .text("👥 Share Signal", `share_signal_${signal.id}`)
      .row()
      .text("❌ Dismiss", "dismiss_signal");

    return keyboard;
  }

  private async generateTickerFrame(tokens: string[], highlightIndex: number): Promise<Buffer> {
    // Generate a frame of the price ticker
    // This would create an image with price data, highlighting different tokens
    const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg width="600" height="100" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="#1a1a1a"/>
  <text x="300" y="50" text-anchor="middle" fill="#ffffff" font-size="18">
    Price Ticker Frame ${highlightIndex + 1}
  </text>
</svg>`;
    
    return Buffer.from(svg);
  }

  private async createGifFromFrames(frames: Buffer[]): Promise<Buffer> {
    // This would use a GIF creation library to combine frames
    // For now, return the first frame as a placeholder
    return frames[0];
  }
}