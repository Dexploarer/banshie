use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, ReplyKeyboardMarkup};

/// Menu creator for all bot menus
pub struct MenuCreator;

impl MenuCreator {
    /// Create the persistent main menu keyboard
    pub fn create_main_menu() -> ReplyKeyboardMarkup {
        let keyboard = ReplyKeyboardMarkup::new(vec![
            vec![
                KeyboardButton::new("💰 Balance"),
                KeyboardButton::new("📊 Portfolio"),
                KeyboardButton::new("⚡ Trade"),
            ],
            vec![
                KeyboardButton::new("💎 Rebates"),
                KeyboardButton::new("🤖 AI Analysis"),
                KeyboardButton::new("💼 Wallet"),
            ],
            vec![
                KeyboardButton::new("⚙️ Settings"),
                KeyboardButton::new("📚 Help"),
                KeyboardButton::new("📈 Charts"),
            ],
        ])
        .persistent(true)
        .resize_keyboard(true);
        
        keyboard
    }
    
    /// Create trading menu with inline keyboard
    pub fn create_trading_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("💰 Quick Buy", "trade_quick_buy"),
                InlineKeyboardButton::callback("💸 Quick Sell", "trade_quick_sell"),
            ],
            vec![
                InlineKeyboardButton::callback("🐕 Buy BONK", "quick_buy_bonk"),
                InlineKeyboardButton::callback("🐶 Buy WIF", "quick_buy_wif"),
                InlineKeyboardButton::callback("🦎 Buy GECKO", "quick_buy_gecko"),
            ],
            vec![
                InlineKeyboardButton::callback("🔍 Search Token", "trade_search"),
                InlineKeyboardButton::callback("📊 Market", "trade_market"),
            ],
            vec![
                InlineKeyboardButton::callback("⚙️ Trading Settings", "trade_settings"),
                InlineKeyboardButton::callback("📈 Chart", "trade_chart"),
            ],
            vec![
                InlineKeyboardButton::callback("🔙 Back to Main Menu", "main_menu"),
            ],
        ])
    }
    
    /// Create wallet menu with inline keyboard
    pub fn create_wallet_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("💰 Check Balance", "wallet_balance"),
                InlineKeyboardButton::callback("📥 Deposit", "wallet_deposit"),
            ],
            vec![
                InlineKeyboardButton::callback("🆕 New Wallet", "wallet_new"),
                InlineKeyboardButton::callback("📥 Import Wallet", "wallet_import"),
            ],
            vec![
                InlineKeyboardButton::callback("📤 Export Keys", "wallet_export"),
                InlineKeyboardButton::callback("🔐 Backup Guide", "wallet_backup"),
            ],
            vec![
                InlineKeyboardButton::callback("🔄 Switch Wallet", "wallet_switch"),
                InlineKeyboardButton::callback("🗑️ Remove Wallet", "wallet_remove"),
            ],
            vec![
                InlineKeyboardButton::callback("🔙 Back to Main Menu", "main_menu"),
            ],
        ])
    }
    
    /// Create analytics menu with inline keyboard
    pub fn create_analytics_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("🤖 AI Analysis SOL", "analyze_sol"),
                InlineKeyboardButton::callback("🤖 AI Analysis BTC", "analyze_btc"),
            ],
            vec![
                InlineKeyboardButton::callback("📊 Market Sentiment", "analyze_sentiment"),
                InlineKeyboardButton::callback("🔥 Trending Tokens", "analyze_trending"),
            ],
            vec![
                InlineKeyboardButton::callback("💎 Token Research", "analyze_research"),
                InlineKeyboardButton::callback("⚡ Quick Analysis", "analyze_quick"),
            ],
            vec![
                InlineKeyboardButton::callback("🔙 Back to Main Menu", "main_menu"),
            ],
        ])
    }
    
    /// Create portfolio menu with inline keyboard
    pub fn create_portfolio_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("📊 View Positions", "portfolio_positions"),
                InlineKeyboardButton::callback("📈 P&L Summary", "portfolio_pnl"),
            ],
            vec![
                InlineKeyboardButton::callback("🔄 Refresh Data", "portfolio_refresh"),
                InlineKeyboardButton::callback("📋 Trade History", "portfolio_history"),
            ],
            vec![
                InlineKeyboardButton::callback("💎 Rebate Stats", "portfolio_rebates"),
                InlineKeyboardButton::callback("📊 Performance", "portfolio_performance"),
            ],
            vec![
                InlineKeyboardButton::callback("📤 Export Report", "portfolio_export"),
                InlineKeyboardButton::callback("📧 Daily Summary", "portfolio_summary"),
            ],
            vec![
                InlineKeyboardButton::callback("🔙 Back to Main Menu", "main_menu"),
            ],
        ])
    }
    
    /// Create settings menu with inline keyboard
    pub fn create_settings_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("⚡ Trading Settings", "settings_trading"),
                InlineKeyboardButton::callback("🔔 Notifications", "settings_notifications"),
            ],
            vec![
                InlineKeyboardButton::callback("🛡️ Security", "settings_security"),
                InlineKeyboardButton::callback("🤖 AI Settings", "settings_ai"),
            ],
            vec![
                InlineKeyboardButton::callback("💎 Rebate Config", "settings_rebates"),
                InlineKeyboardButton::callback("⚙️ Advanced", "settings_advanced"),
            ],
            vec![
                InlineKeyboardButton::callback("🔙 Back to Main Menu", "main_menu"),
            ],
        ])
    }
}

// Helper functions for backwards compatibility
pub fn create_main_menu() -> ReplyKeyboardMarkup {
    MenuCreator::create_main_menu()
}

pub fn create_trading_menu() -> InlineKeyboardMarkup {
    MenuCreator::create_trading_menu()
}

pub fn create_wallet_menu() -> InlineKeyboardMarkup {
    MenuCreator::create_wallet_menu()
}

pub fn create_analytics_menu() -> InlineKeyboardMarkup {
    MenuCreator::create_analytics_menu()
}

pub fn create_portfolio_menu() -> InlineKeyboardMarkup {
    MenuCreator::create_portfolio_menu()
}

pub fn create_settings_menu() -> InlineKeyboardMarkup {
    MenuCreator::create_settings_menu()
}