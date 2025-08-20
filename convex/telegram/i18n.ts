// Internationalization support for Solana Trading Bot

export type SupportedLanguage = 'en' | 'es' | 'fr' | 'de' | 'it' | 'pt' | 'ru' | 'zh' | 'ja' | 'ko';

export interface Translation {
  [key: string]: string | Translation;
}

export interface Translations {
  [key: string]: Translation;
}

// English translations (base)
const en: Translations = {
  // Common
  common: {
    yes: "Yes",
    no: "No",
    cancel: "Cancel",
    back: "Back",
    next: "Next",
    done: "Done",
    loading: "Loading...",
    error: "Error",
    success: "Success",
    warning: "Warning",
    confirm: "Confirm",
    refresh: "Refresh",
    settings: "Settings",
    help: "Help",
    close: "Close",
  },

  // Commands
  commands: {
    start: {
      welcome: "🚀 Welcome to Solana Trading Bot!\n\nYour AI-powered companion for Solana trading with:\n• Real-time portfolio tracking\n• Advanced DCA strategies\n• AI trading signals\n• Price alerts & notifications\n\nChoose an option below to get started:",
      language_setup: "Please select your preferred language:",
      user_created: "Welcome! Your account has been created. You can now start trading!",
    },
    portfolio: {
      title: "📊 Portfolio Overview",
      total_value: "💰 Total Value: ${{value}}",
      total_pnl: "📈 Total P&L: {{sign}}${{amount}} ({{percentage}}%)",
      positions: "🎯 Positions: {{count}}",
      no_portfolio: "No portfolio data available. Connect a wallet to get started!",
      last_updated: "📅 Last updated: {{time}}",
    },
    trade: {
      title: "💱 Quick Trade: {{symbol}}",
      current_price: "💰 Current Price: ${{price}}",
      price_change: "{{emoji}} 24h Change: {{sign}}{{change}}%",
      select_action: "Select your trading action:",
      buy: "Buy",
      sell: "Sell",
      chart: "Price Chart",
      analysis: "AI Analysis",
    },
    dca: {
      title: "🤖 DCA Strategies",
      no_strategies: "No active strategies found.\n\nDCA (Dollar Cost Averaging) helps reduce volatility by investing fixed amounts regularly.",
      active_strategies: "🤖 Active DCA Strategies",
      new_strategy: "➕ New Strategy",
      performance: "📊 Performance",
      pause_all: "⏸️ Pause All",
      resume_all: "▶️ Resume All",
    },
    alerts: {
      title: "🔔 Price Alerts",
      no_alerts: "No active alerts found.\n\nSet up price alerts to get notified when tokens reach your target prices.",
      active_alerts: "🔔 Active Alerts",
      new_alert: "➕ New Alert",
      alert_history: "📊 Alert History",
    },
    signals: {
      title: "🧠 AI Trading Signals",
      no_signals: "No recent signals available.\n\nAI analyzes market data to provide trading recommendations.",
      latest_signals: "🧠 Latest AI Signals",
      refresh: "🔄 Refresh",
      settings: "⚙️ Settings",
    },
    wallet: {
      title: "💳 Wallet Management",
      description: "Connect your Solana wallet to start trading:\n• Phantom Wallet\n• Hardware Wallets (Ledger/Trezor)\n• WalletConnect\n\nYour keys remain secure - we never store private keys.",
      connect: "🔗 Connect Wallet",
      balances: "💰 Balances",
      sync: "🔄 Sync Balances",
      history: "📊 Transactions",
    },
    help: {
      title: "🤖 Solana Trading Bot Help",
      commands: "**Commands:**\n/start - Initialize bot\n/portfolio - View portfolio\n/trade [token] - Quick trade\n/dca - DCA strategies\n/alerts - Price alerts\n/signals - AI signals\n/wallet - Wallet management",
      inline_queries: "**Inline Queries:**\nType @SolanaBot followed by:\n• `portfolio` - Portfolio summary\n• `dca` - DCA strategies\n• `trending` - Trending tokens\n• Token symbol for quick info",
      support: "**Support:**\n📧 support@solanabot.com\n🌐 docs.solanabot.com",
    },
  },

  // Trading
  trading: {
    order: {
      placed: "✅ Order placed: {{orderId}}",
      failed: "❌ Order failed: {{error}}",
      executing: "⏳ Executing order...",
      completed: "✅ Order completed! Transaction: {{txSignature}}",
      cancelled: "❌ Order cancelled",
    },
    price: {
      title: "💰 {{symbol}} Price",
      current: "Current: ${{price}}",
      change_24h: "24h Change: {{sign}}{{change}}%",
      volume: "Volume: ${{volume}}M",
      market_cap: "Market Cap: ${{marketCap}}M",
    },
    signal: {
      action: "🎯 Action: {{action}}",
      confidence: "📊 Confidence: {{confidence}}%",
      strength: "⚡ Strength: {{strength}}/100",
      timeframe: "⏰ Timeframe: {{timeframe}}",
      risk: "⚠️ Risk: {{risk}}",
      reasoning: "💭 Reasoning: {{reasoning}}",
      valid_until: "🕐 Valid Until: {{time}}",
    },
  },

  // Errors
  errors: {
    general: "❌ An error occurred. Please try again.",
    network: "❌ Network error. Please check your connection.",
    api: "❌ API error. Please try again later.",
    token_not_found: "❌ Token not found: {{symbol}}",
    insufficient_balance: "❌ Insufficient balance",
    invalid_amount: "❌ Invalid amount",
    wallet_not_connected: "❌ Please connect your wallet first",
    permission_denied: "❌ Permission denied",
    rate_limited: "❌ Too many requests. Please wait {{seconds}} seconds.",
  },

  // Success messages
  success: {
    wallet_connected: "✅ Wallet connected successfully",
    alert_created: "✅ Alert created successfully",
    dca_created: "✅ DCA strategy created successfully",
    settings_updated: "✅ Settings updated successfully",
    order_placed: "✅ Order placed successfully",
  },

  // Buttons
  buttons: {
    buy: "💰 Buy",
    sell: "📉 Sell",
    hold: "⏸️ Hold",
    trade: "💱 Trade",
    chart: "📊 Chart",
    analysis: "🧠 Analysis",
    alerts: "🔔 Alerts",
    portfolio: "📊 Portfolio",
    dca: "🤖 DCA",
    wallet: "💳 Wallet",
    settings: "⚙️ Settings",
    help: "❓ Help",
    refresh: "🔄 Refresh",
    back: "⬅️ Back",
    cancel: "❌ Cancel",
    confirm: "✅ Confirm",
    edit: "✏️ Edit",
    delete: "🗑️ Delete",
    pause: "⏸️ Pause",
    resume: "▶️ Resume",
    stop: "⏹️ Stop",
  },

  // Time formats
  time: {
    just_now: "just now",
    minutes_ago: "{{minutes}} minutes ago",
    hours_ago: "{{hours}} hours ago",
    days_ago: "{{days}} days ago",
    weeks_ago: "{{weeks}} weeks ago",
  },

  // Numbers and currencies
  format: {
    currency: "${{amount}}",
    percentage: "{{value}}%",
    large_number: "{{value}}{{unit}}",
  },
};

// Spanish translations
const es: Translations = {
  common: {
    yes: "Sí",
    no: "No",
    cancel: "Cancelar",
    back: "Atrás",
    next: "Siguiente",
    done: "Hecho",
    loading: "Cargando...",
    error: "Error",
    success: "Éxito",
    warning: "Advertencia",
    confirm: "Confirmar",
    refresh: "Actualizar",
    settings: "Configuración",
    help: "Ayuda",
    close: "Cerrar",
  },
  commands: {
    start: {
      welcome: "🚀 ¡Bienvenido a Solana Trading Bot!\n\nTu compañero impulsado por IA para trading de Solana con:\n• Seguimiento de portafolio en tiempo real\n• Estrategias DCA avanzadas\n• Señales de trading AI\n• Alertas de precio y notificaciones\n\nElige una opción para comenzar:",
      language_setup: "Por favor selecciona tu idioma preferido:",
      user_created: "¡Bienvenido! Tu cuenta ha sido creada. ¡Ya puedes comenzar a hacer trading!",
    },
    portfolio: {
      title: "📊 Resumen del Portafolio",
      total_value: "💰 Valor Total: ${{value}}",
      total_pnl: "📈 P&L Total: {{sign}}${{amount}} ({{percentage}}%)",
      positions: "🎯 Posiciones: {{count}}",
      no_portfolio: "No hay datos de portafolio disponibles. ¡Conecta una billetera para empezar!",
      last_updated: "📅 Última actualización: {{time}}",
    },
    // ... more Spanish translations
  },
  // ... more sections
};

// French translations
const fr: Translations = {
  common: {
    yes: "Oui",
    no: "Non",
    cancel: "Annuler",
    back: "Retour",
    next: "Suivant",
    done: "Terminé",
    loading: "Chargement...",
    error: "Erreur",
    success: "Succès",
    warning: "Attention",
    confirm: "Confirmer",
    refresh: "Actualiser",
    settings: "Paramètres",
    help: "Aide",
    close: "Fermer",
  },
  commands: {
    start: {
      welcome: "🚀 Bienvenue sur Solana Trading Bot !\n\nVotre compagnon IA pour le trading Solana avec :\n• Suivi de portefeuille en temps réel\n• Stratégies DCA avancées\n• Signaux de trading IA\n• Alertes de prix et notifications\n\nChoisissez une option pour commencer :",
      language_setup: "Veuillez sélectionner votre langue préférée :",
      user_created: "Bienvenue ! Votre compte a été créé. Vous pouvez maintenant commencer à trader !",
    },
    // ... more French translations
  },
  // ... more sections
};

// German translations
const de: Translations = {
  common: {
    yes: "Ja",
    no: "Nein",
    cancel: "Abbrechen",
    back: "Zurück",
    next: "Weiter",
    done: "Fertig",
    loading: "Lädt...",
    error: "Fehler",
    success: "Erfolg",
    warning: "Warnung",
    confirm: "Bestätigen",
    refresh: "Aktualisieren",
    settings: "Einstellungen",
    help: "Hilfe",
    close: "Schließen",
  },
  // ... more sections
};

// Add more languages...
const it: Translations = { /* Italian */ };
const pt: Translations = { /* Portuguese */ };
const ru: Translations = { /* Russian */ };
const zh: Translations = { /* Chinese */ };
const ja: Translations = { /* Japanese */ };
const ko: Translations = { /* Korean */ };

// Language dictionary
export const translations: Record<SupportedLanguage, Translations> = {
  en,
  es,
  fr,
  de,
  it,
  pt,
  ru,
  zh,
  ja,
  ko,
};

// Language names for selection
export const languageNames: Record<SupportedLanguage, string> = {
  en: "🇺🇸 English",
  es: "🇪🇸 Español",
  fr: "🇫🇷 Français",
  de: "🇩🇪 Deutsch",
  it: "🇮🇹 Italiano",
  pt: "🇧🇷 Português",
  ru: "🇷🇺 Русский",
  zh: "🇨🇳 中文",
  ja: "🇯🇵 日本語",
  ko: "🇰🇷 한국어",
};

// Translation function
export function t(
  lang: SupportedLanguage,
  key: string,
  params: Record<string, string | number> = {}
): string {
  const langTranslations = translations[lang] || translations.en;
  
  // Navigate through nested keys (e.g., "commands.start.welcome")
  const keys = key.split('.');
  let value: any = langTranslations;
  
  for (const k of keys) {
    if (value && typeof value === 'object' && k in value) {
      value = value[k];
    } else {
      // Fallback to English if key not found
      value = translations.en;
      for (const fallbackKey of keys) {
        if (value && typeof value === 'object' && fallbackKey in value) {
          value = value[fallbackKey];
        } else {
          return `[${key}]`; // Return key if translation not found
        }
      }
      break;
    }
  }
  
  if (typeof value !== 'string') {
    return `[${key}]`;
  }
  
  // Replace parameters
  let result = value;
  for (const [param, val] of Object.entries(params)) {
    result = result.replace(new RegExp(`{{${param}}}`, 'g'), String(val));
  }
  
  return result;
}

// Detect user language from Telegram locale
export function detectLanguage(telegramLangCode?: string): SupportedLanguage {
  if (!telegramLangCode) return 'en';
  
  const langCode = telegramLangCode.split('-')[0].toLowerCase();
  
  // Map common language codes
  const langMapping: Record<string, SupportedLanguage> = {
    'en': 'en',
    'es': 'es',
    'fr': 'fr',
    'de': 'de',
    'it': 'it',
    'pt': 'pt',
    'ru': 'ru',
    'zh': 'zh',
    'ja': 'ja',
    'ko': 'ko',
  };
  
  return langMapping[langCode] || 'en';
}

// Format numbers based on language
export function formatNumber(
  lang: SupportedLanguage,
  value: number,
  type: 'currency' | 'percentage' | 'number' = 'number'
): string {
  const locales: Record<SupportedLanguage, string> = {
    en: 'en-US',
    es: 'es-ES',
    fr: 'fr-FR',
    de: 'de-DE',
    it: 'it-IT',
    pt: 'pt-BR',
    ru: 'ru-RU',
    zh: 'zh-CN',
    ja: 'ja-JP',
    ko: 'ko-KR',
  };

  const locale = locales[lang] || 'en-US';

  switch (type) {
    case 'currency':
      return new Intl.NumberFormat(locale, {
        style: 'currency',
        currency: 'USD',
        minimumFractionDigits: 2,
        maximumFractionDigits: 6,
      }).format(value);
      
    case 'percentage':
      return new Intl.NumberFormat(locale, {
        style: 'percent',
        minimumFractionDigits: 1,
        maximumFractionDigits: 2,
      }).format(value / 100);
      
    default:
      return new Intl.NumberFormat(locale).format(value);
  }
}

// Format relative time based on language
export function formatRelativeTime(
  lang: SupportedLanguage,
  timestamp: number
): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);
  const weeks = Math.floor(diff / 604800000);

  if (minutes < 1) {
    return t(lang, 'time.just_now');
  } else if (minutes < 60) {
    return t(lang, 'time.minutes_ago', { minutes: minutes.toString() });
  } else if (hours < 24) {
    return t(lang, 'time.hours_ago', { hours: hours.toString() });
  } else if (days < 7) {
    return t(lang, 'time.days_ago', { days: days.toString() });
  } else {
    return t(lang, 'time.weeks_ago', { weeks: weeks.toString() });
  }
}

// Get user's language from database or detect from Telegram
export async function getUserLanguage(
  convex: any,
  userId: string,
  telegramLangCode?: string
): Promise<SupportedLanguage> {
  try {
    const user = await convex.query("queries/users:getUser", { userId });
    if (user && user.settings && user.settings.language) {
      return user.settings.language as SupportedLanguage;
    }
  } catch (error) {
    console.log('Could not get user language from database:', error);
  }
  
  return detectLanguage(telegramLangCode);
}

// Update user's language preference
export async function updateUserLanguage(
  convex: any,
  userId: string,
  language: SupportedLanguage
): Promise<void> {
  try {
    await convex.mutation("mutations/users:updateSettings", {
      userId,
      settings: {
        language,
      },
    });
  } catch (error) {
    console.error('Failed to update user language:', error);
    throw error;
  }
}

// Create language selection keyboard
export function createLanguageKeyboard(): any {
  const keyboard = [];
  const languages = Object.entries(languageNames);
  
  // Create rows of 2 languages each
  for (let i = 0; i < languages.length; i += 2) {
    const row = [];
    row.push({
      text: languages[i][1],
      callback_data: `lang_${languages[i][0]}`,
    });
    
    if (i + 1 < languages.length) {
      row.push({
        text: languages[i + 1][1],
        callback_data: `lang_${languages[i + 1][0]}`,
      });
    }
    
    keyboard.push(row);
  }
  
  return { inline_keyboard: keyboard };
}

// Middleware for handling language in commands
export class I18nMiddleware {
  private convex: any;

  constructor(convex: any) {
    this.convex = convex;
  }

  async handleMessage(ctx: any, next: () => Promise<void>) {
    // Get user language
    const userId = ctx.from?.id?.toString();
    const telegramLangCode = ctx.from?.language_code;
    
    if (userId) {
      const lang = await getUserLanguage(this.convex, userId, telegramLangCode);
      
      // Add translation function to context
      ctx.t = (key: string, params?: Record<string, string | number>) => 
        t(lang, key, params);
      
      // Add formatting functions
      ctx.formatNumber = (value: number, type?: 'currency' | 'percentage' | 'number') => 
        formatNumber(lang, value, type);
      
      ctx.formatTime = (timestamp: number) => 
        formatRelativeTime(lang, timestamp);
      
      // Store current language
      ctx.lang = lang;
    } else {
      // Fallback to English
      ctx.t = (key: string, params?: Record<string, string | number>) => 
        t('en', key, params);
      ctx.formatNumber = (value: number, type?: 'currency' | 'percentage' | 'number') => 
        formatNumber('en', value, type);
      ctx.formatTime = (timestamp: number) => 
        formatRelativeTime('en', timestamp);
      ctx.lang = 'en';
    }
    
    await next();
  }

  async handleLanguageChange(ctx: any) {
    const languageCode = ctx.callbackQuery?.data?.replace('lang_', '') as SupportedLanguage;
    const userId = ctx.from?.id?.toString();
    
    if (!userId || !languageCode || !(languageCode in translations)) {
      return;
    }
    
    try {
      await updateUserLanguage(this.convex, userId, languageCode);
      
      const newLang = languageCode;
      ctx.lang = newLang;
      
      await ctx.answerCallbackQuery({
        text: t(newLang, 'success.settings_updated'),
      });
      
      await ctx.editMessageText(
        t(newLang, 'commands.start.welcome'),
        {
          reply_markup: this.createMainKeyboard(newLang),
          parse_mode: 'Markdown',
        }
      );
    } catch (error) {
      console.error('Error updating language:', error);
      await ctx.answerCallbackQuery({
        text: t(ctx.lang || 'en', 'errors.general'),
      });
    }
  }

  private createMainKeyboard(lang: SupportedLanguage): any {
    return {
      inline_keyboard: [
        [
          { text: t(lang, 'buttons.portfolio'), callback_data: 'portfolio' },
          { text: t(lang, 'buttons.trade'), callback_data: 'trade' },
        ],
        [
          { text: t(lang, 'buttons.dca'), callback_data: 'dca' },
          { text: t(lang, 'buttons.alerts'), callback_data: 'alerts' },
        ],
        [
          { text: t(lang, 'buttons.signals'), callback_data: 'signals' },
          { text: t(lang, 'buttons.wallet'), callback_data: 'wallet' },
        ],
        [
          { text: t(lang, 'buttons.settings'), callback_data: 'settings' },
          { text: t(lang, 'buttons.help'), callback_data: 'help' },
        ],
      ],
    };
  }
}