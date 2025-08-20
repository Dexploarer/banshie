use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,
    
    #[command(description = "Manage your wallets")]
    Wallet,
    
    #[command(description = "Generate new wallet")]
    NewWallet,
    
    #[command(description = "Import existing wallet")]
    Import,
    
    #[command(description = "Show deposit address")]
    Deposit,
    
    #[command(description = "View your balance")]
    Balance,
    
    #[command(description = "Buy token: /buy <token> <amount_sol>")]
    Buy(String),
    
    #[command(description = "Sell token: /sell <token> <percentage>")]
    Sell(String),
    
    #[command(description = "View earned rebates")]
    Rebates,
    
    #[command(description = "Get AI market analysis")]
    Analyze(String),
    
    #[command(description = "View active positions")]
    Portfolio,
    
    #[command(description = "Export wallet (secure)")]
    Export,
    
    #[command(description = "Backup instructions")]
    Backup,
    
    #[command(description = "Bot settings")]
    Settings,
    
    #[command(description = "Get help")]
    Help,
    
    #[command(description = "Confirm action")]
    Confirm,
    
    #[command(description = "Cancel current operation")]
    Cancel,
    
    // MVP Trading Features
    #[command(description = "Quick snipe new tokens: /snipe <token_address>")]
    Snipe(String),
    
    #[command(description = "Copy trader: /copy <wallet_address>")]
    Copy(String),
    
    #[command(description = "Stop copying trader: /unfollow <wallet_address>")]
    Unfollow(String),
    
    #[command(description = "Check if token is LARP/scam: /larp <token_address>")]
    Larp(String),
    
    #[command(description = "Get trending tokens")]
    Trending,
    
    #[command(description = "Launch new token")]
    Launch,
    
    #[command(description = "Create Solana Blink: /blink <action>")]
    Blink(String),
    
    #[command(description = "Set trading alerts: /alert <token> <price>")]
    Alert(String),
    
    #[command(description = "View top traders leaderboard")]
    Leaderboard,
    
    #[command(description = "Get AI trading signals")]
    Signals,
    
    #[command(description = "Pump.fun integration: /pump <action>")]
    Pump(String),
    
    #[command(description = "Quick buy with SOL: /qbuy <amount_sol>")]
    QuickBuy(String),
    
    #[command(description = "Quick sell percentage: /qsell <percentage>")]
    QuickSell(String),
    
    #[command(description = "Set stop loss: /stop <token> <percentage>")]
    StopLoss(String),
}