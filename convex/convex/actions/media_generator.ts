import { action } from "../_generated/server";
import { api } from "../_generated/api";
import { v } from "convex/values";

// Chart.js and Canvas API for server-side chart generation
const CHART_WIDTH = 800;
const CHART_HEIGHT = 600;

// Generate price chart image
export const generatePriceChart = action({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    interval: v.optional(v.string()), // "1m", "5m", "15m", "1h", "4h", "1d"
    period: v.optional(v.number()), // number of data points
    chartType: v.optional(v.string()), // "line", "candlestick", "area"
    indicators: v.optional(v.array(v.string())), // ["sma20", "rsi", "macd"]
    theme: v.optional(v.string()), // "dark", "light"
  },
  handler: async (ctx, args) => {
    const interval = args.interval || "1h";
    const period = args.period || 168; // 7 days of hourly data
    const chartType = args.chartType || "candlestick";
    const indicators = args.indicators || [];
    const theme = args.theme || "dark";
    
    console.log(`📊 Generating ${chartType} chart for ${args.symbol} (${interval}, ${period} points)`);
    
    try {
      // Get price history
      const priceHistory = await ctx.runQuery(api.queries.prices.getPriceHistory, {
        tokenMint: args.tokenMint,
        interval: interval,
        limit: period,
      });

      if (!priceHistory?.data || priceHistory.data.length === 0) {
        throw new Error("No price data available");
      }

      // Get technical indicators if requested
      let technicalData = null;
      if (indicators.length > 0) {
        technicalData = await ctx.runAction(api.actions.analytics.calculateTokenIndicators, {
          tokenMint: args.tokenMint,
        });
      }

      // Generate chart configuration
      const chartConfig = createChartConfig(
        priceHistory.data,
        technicalData,
        chartType,
        indicators,
        theme,
        args.symbol
      );

      // Generate chart image using Chart.js (server-side)
      const chartBuffer = await generateChartImage(chartConfig);
      
      // Convert to base64 for easy transmission
      const base64Image = chartBuffer.toString('base64');
      
      console.log(`✅ Generated ${chartType} chart for ${args.symbol}`);

      return {
        symbol: args.symbol,
        chartType: chartType,
        interval: interval,
        period: period,
        indicators: indicators,
        theme: theme,
        imageBase64: base64Image,
        imageSize: {
          width: CHART_WIDTH,
          height: CHART_HEIGHT,
        },
        dataPoints: priceHistory.data.length,
        timestamp: Date.now(),
      };
    } catch (error) {
      console.error(`❌ Error generating chart for ${args.symbol}:`, error);
      throw new Error(`Chart generation failed: ${error}`);
    }
  },
});

// Generate portfolio overview image
export const generatePortfolioImage = action({
  args: {
    userId: v.id("users"),
    theme: v.optional(v.string()),
    includeChart: v.optional(v.boolean()),
  },
  handler: async (ctx, args) => {
    const theme = args.theme || "dark";
    const includeChart = args.includeChart || true;
    
    console.log(`📊 Generating portfolio image for user ${args.userId}`);
    
    try {
      // Get portfolio data
      const portfolio = await ctx.runQuery(api.queries.portfolio.getPortfolio, {
        userId: args.userId,
      });

      if (!portfolio) {
        throw new Error("Portfolio not found");
      }

      // Create portfolio visualization
      const portfolioConfig = createPortfolioConfig(portfolio, theme, includeChart);
      
      // Generate image
      const imageBuffer = await generatePortfolioImage(portfolioConfig);
      const base64Image = imageBuffer.toString('base64');
      
      console.log(`✅ Generated portfolio image for user ${args.userId}`);

      return {
        userId: args.userId,
        theme: theme,
        imageBase64: base64Image,
        imageSize: {
          width: CHART_WIDTH,
          height: CHART_HEIGHT,
        },
        portfolioValue: portfolio.summary.totalValue,
        positionCount: portfolio.summary.positionCount,
        timestamp: Date.now(),
      };
    } catch (error) {
      console.error(`❌ Error generating portfolio image:`, error);
      throw new Error(`Portfolio image generation failed: ${error}`);
    }
  },
});

// Generate technical analysis summary image
export const generateTechnicalAnalysisImage = action({
  args: {
    tokenMint: v.string(),
    symbol: v.string(),
    analysisType: v.optional(v.string()), // "overview", "detailed", "signals"
    theme: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const analysisType = args.analysisType || "overview";
    const theme = args.theme || "dark";
    
    console.log(`📊 Generating technical analysis image for ${args.symbol}`);
    
    try {
      // Get technical indicators
      const technicalData = await ctx.runAction(api.actions.analytics.calculateTokenIndicators, {
        tokenMint: args.tokenMint,
      });

      // Get current price
      const priceData = await ctx.runQuery(api.queries.prices.getTokenPrice, {
        mint: args.tokenMint,
      });

      // Get AI analysis if available
      const aiAnalysis = await ctx.runQuery(api.queries.ai.getLatestAnalysis, {
        targetId: args.tokenMint,
        type: "technical",
        limit: 1,
      });

      // Create analysis visualization
      const analysisConfig = createAnalysisConfig(
        args.symbol,
        priceData,
        technicalData,
        aiAnalysis[0],
        analysisType,
        theme
      );

      // Generate image
      const imageBuffer = await generateAnalysisImage(analysisConfig);
      const base64Image = imageBuffer.toString('base64');
      
      console.log(`✅ Generated technical analysis image for ${args.symbol}`);

      return {
        symbol: args.symbol,
        analysisType: analysisType,
        theme: theme,
        imageBase64: base64Image,
        imageSize: {
          width: CHART_WIDTH,
          height: CHART_HEIGHT,
        },
        indicators: Object.keys(technicalData || {}),
        timestamp: Date.now(),
      };
    } catch (error) {
      console.error(`❌ Error generating technical analysis image:`, error);
      throw new Error(`Technical analysis image generation failed: ${error}`);
    }
  },
});

// Generate trading signal card
export const generateSignalCard = action({
  args: {
    signalId: v.id("tradingSignals"),
    theme: v.optional(v.string()),
    includeChart: v.optional(v.boolean()),
  },
  handler: async (ctx, args) => {
    const theme = args.theme || "dark";
    const includeChart = args.includeChart || true;
    
    console.log(`📊 Generating signal card for ${args.signalId}`);
    
    try {
      // Get signal data
      const signal = await ctx.db.get(args.signalId);
      if (!signal) {
        throw new Error("Signal not found");
      }

      // Get mini price chart if requested
      let priceData = null;
      if (includeChart) {
        priceData = await ctx.runQuery(api.queries.prices.getPriceHistory, {
          tokenMint: signal.tokenMint,
          interval: "1h",
          limit: 24, // 24 hours
        });
      }

      // Create signal card configuration
      const cardConfig = createSignalCardConfig(signal, priceData, theme);
      
      // Generate image
      const imageBuffer = await generateSignalCardImage(cardConfig);
      const base64Image = imageBuffer.toString('base64');
      
      console.log(`✅ Generated signal card for ${signal.symbol}`);

      return {
        signalId: args.signalId,
        symbol: signal.symbol,
        action: signal.action,
        confidence: signal.confidence,
        theme: theme,
        imageBase64: base64Image,
        imageSize: {
          width: 600,
          height: 400,
        },
        timestamp: Date.now(),
      };
    } catch (error) {
      console.error(`❌ Error generating signal card:`, error);
      throw new Error(`Signal card generation failed: ${error}`);
    }
  },
});

// Generate market overview infographic
export const generateMarketOverview = action({
  args: {
    category: v.optional(v.string()), // "trending", "movers", "volume"
    limit: v.optional(v.number()),
    theme: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const category = args.category || "trending";
    const limit = args.limit || 10;
    const theme = args.theme || "dark";
    
    console.log(`📊 Generating market overview (${category})`);
    
    try {
      // Get market data
      const marketData = await ctx.runQuery(api.queries.prices.getMarketOverview, {
        category: category,
        limit: limit,
      });

      if (!marketData || marketData.length === 0) {
        throw new Error("No market data available");
      }

      // Create market overview configuration
      const overviewConfig = createMarketOverviewConfig(marketData, category, theme);
      
      // Generate image
      const imageBuffer = await generateMarketOverviewImage(overviewConfig);
      const base64Image = imageBuffer.toString('base64');
      
      console.log(`✅ Generated market overview (${category})`);

      return {
        category: category,
        tokenCount: marketData.length,
        theme: theme,
        imageBase64: base64Image,
        imageSize: {
          width: CHART_WIDTH,
          height: CHART_HEIGHT,
        },
        timestamp: Date.now(),
      };
    } catch (error) {
      console.error(`❌ Error generating market overview:`, error);
      throw new Error(`Market overview generation failed: ${error}`);
    }
  },
});

// Helper functions for chart configuration

function createChartConfig(priceData: any[], technicalData: any, chartType: string, indicators: string[], theme: string, symbol: string) {
  const colors = theme === "dark" 
    ? {
        background: "#1a1a1a",
        text: "#ffffff",
        grid: "#333333",
        up: "#00ff88",
        down: "#ff4444",
        volume: "#666666",
      }
    : {
        background: "#ffffff",
        text: "#000000",
        grid: "#e0e0e0",
        up: "#00cc66",
        down: "#cc3333",
        volume: "#999999",
      };

  const config = {
    type: chartType === "candlestick" ? "candlestick" : "line",
    data: {
      labels: priceData.map(d => new Date(d.timestamp).toLocaleDateString()),
      datasets: []
    },
    options: {
      responsive: true,
      plugins: {
        title: {
          display: true,
          text: `${symbol} Price Chart`,
          color: colors.text,
          font: { size: 18, weight: 'bold' }
        },
        legend: {
          display: true,
          labels: { color: colors.text }
        }
      },
      scales: {
        x: {
          grid: { color: colors.grid },
          ticks: { color: colors.text }
        },
        y: {
          grid: { color: colors.grid },
          ticks: { color: colors.text }
        }
      },
      backgroundColor: colors.background,
    }
  };

  // Add price data
  if (chartType === "candlestick") {
    config.data.datasets.push({
      label: "Price",
      data: priceData.map(d => ({
        x: d.timestamp,
        o: parseFloat(d.open),
        h: parseFloat(d.high),
        l: parseFloat(d.low),
        c: parseFloat(d.close),
      })),
      borderColor: colors.up,
      backgroundColor: colors.up,
    });
  } else {
    config.data.datasets.push({
      label: "Price",
      data: priceData.map(d => parseFloat(d.close)),
      borderColor: colors.up,
      backgroundColor: `${colors.up}20`,
      fill: chartType === "area",
    });
  }

  // Add technical indicators
  if (technicalData && indicators.length > 0) {
    indicators.forEach(indicator => {
      if (technicalData[indicator]) {
        config.data.datasets.push(createIndicatorDataset(technicalData[indicator], indicator, colors));
      }
    });
  }

  return config;
}

function createIndicatorDataset(indicatorData: any, indicator: string, colors: any) {
  const indicatorColors: { [key: string]: string } = {
    sma20: "#ffaa00",
    sma50: "#00aaff",
    sma200: "#aa00ff",
    ema12: "#ff6600",
    ema26: "#0066ff",
    rsi: "#ff0066",
  };

  return {
    label: indicator.toUpperCase(),
    data: Array.isArray(indicatorData) ? indicatorData : [indicatorData],
    borderColor: indicatorColors[indicator] || colors.text,
    backgroundColor: "transparent",
    borderWidth: 2,
    pointRadius: 0,
  };
}

function createPortfolioConfig(portfolio: any, theme: string, includeChart: boolean) {
  const colors = theme === "dark" 
    ? {
        background: "#1a1a1a",
        text: "#ffffff",
        positive: "#00ff88",
        negative: "#ff4444",
        neutral: "#ffaa00",
      }
    : {
        background: "#ffffff",
        text: "#000000",
        positive: "#00cc66",
        negative: "#cc3333",
        neutral: "#cc8800",
      };

  return {
    portfolio,
    theme,
    colors,
    includeChart,
    title: "Portfolio Overview",
    timestamp: new Date().toISOString(),
  };
}

function createAnalysisConfig(symbol: string, priceData: any, technicalData: any, aiAnalysis: any, analysisType: string, theme: string) {
  const colors = theme === "dark" 
    ? {
        background: "#1a1a1a",
        text: "#ffffff",
        accent: "#00aaff",
        positive: "#00ff88",
        negative: "#ff4444",
      }
    : {
        background: "#ffffff",
        text: "#000000",
        accent: "#0088cc",
        positive: "#00cc66",
        negative: "#cc3333",
      };

  return {
    symbol,
    priceData,
    technicalData,
    aiAnalysis,
    analysisType,
    theme,
    colors,
    title: `${symbol} Technical Analysis`,
    timestamp: new Date().toISOString(),
  };
}

function createSignalCardConfig(signal: any, priceData: any, theme: string) {
  const colors = theme === "dark" 
    ? {
        background: "#1a1a1a",
        text: "#ffffff",
        accent: getActionColor(signal.action, true),
      }
    : {
        background: "#ffffff",
        text: "#000000",
        accent: getActionColor(signal.action, false),
      };

  return {
    signal,
    priceData,
    theme,
    colors,
    timestamp: new Date().toISOString(),
  };
}

function createMarketOverviewConfig(marketData: any[], category: string, theme: string) {
  const colors = theme === "dark" 
    ? {
        background: "#1a1a1a",
        text: "#ffffff",
        positive: "#00ff88",
        negative: "#ff4444",
        accent: "#00aaff",
      }
    : {
        background: "#ffffff",
        text: "#000000",
        positive: "#00cc66",
        negative: "#cc3333",
        accent: "#0088cc",
      };

  return {
    marketData,
    category,
    theme,
    colors,
    title: `Market ${category.charAt(0).toUpperCase() + category.slice(1)}`,
    timestamp: new Date().toISOString(),
  };
}

function getActionColor(action: string, isDark: boolean): string {
  const colors = {
    buy: isDark ? "#00ff88" : "#00cc66",
    sell: isDark ? "#ff4444" : "#cc3333",
    hold: isDark ? "#ffaa00" : "#cc8800",
  };
  
  return colors[action as keyof typeof colors] || (isDark ? "#ffffff" : "#000000");
}

// Mock image generation functions (in production, use Canvas API or similar)
async function generateChartImage(config: any): Promise<Buffer> {
  // This would use a library like Chart.js with node-canvas
  // For now, return a placeholder
  console.log("🎨 Generating chart image with config:", JSON.stringify(config, null, 2));
  
  // Create a simple SVG as placeholder
  const svg = createPlaceholderSVG("Price Chart", config.options.plugins.title.text, CHART_WIDTH, CHART_HEIGHT);
  return Buffer.from(svg);
}

async function generatePortfolioImage(config: any): Promise<Buffer> {
  console.log("🎨 Generating portfolio image");
  const svg = createPlaceholderSVG("Portfolio", config.title, CHART_WIDTH, CHART_HEIGHT);
  return Buffer.from(svg);
}

async function generateAnalysisImage(config: any): Promise<Buffer> {
  console.log("🎨 Generating analysis image");
  const svg = createPlaceholderSVG("Analysis", config.title, CHART_WIDTH, CHART_HEIGHT);
  return Buffer.from(svg);
}

async function generateSignalCardImage(config: any): Promise<Buffer> {
  console.log("🎨 Generating signal card");
  const svg = createPlaceholderSVG("Signal", `${config.signal.symbol} ${config.signal.action.toUpperCase()}`, 600, 400);
  return Buffer.from(svg);
}

async function generateMarketOverviewImage(config: any): Promise<Buffer> {
  console.log("🎨 Generating market overview");
  const svg = createPlaceholderSVG("Market", config.title, CHART_WIDTH, CHART_HEIGHT);
  return Buffer.from(svg);
}

function createPlaceholderSVG(type: string, title: string, width: number, height: number): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="#1a1a1a"/>
  <text x="50%" y="30%" dominant-baseline="central" text-anchor="middle" fill="#ffffff" font-size="24" font-family="Arial, sans-serif">
    ${type}
  </text>
  <text x="50%" y="50%" dominant-baseline="central" text-anchor="middle" fill="#00aaff" font-size="32" font-family="Arial, sans-serif" font-weight="bold">
    ${title}
  </text>
  <text x="50%" y="70%" dominant-baseline="central" text-anchor="middle" fill="#666666" font-size="16" font-family="Arial, sans-serif">
    Generated by Solana Trading Bot
  </text>
  <text x="50%" y="80%" dominant-baseline="central" text-anchor="middle" fill="#666666" font-size="12" font-family="Arial, sans-serif">
    ${new Date().toLocaleString()}
  </text>
</svg>`;
}