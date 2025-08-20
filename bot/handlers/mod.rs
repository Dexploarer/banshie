pub mod callback;
pub mod command;
pub mod menu;
pub mod text;
pub mod trading;
pub mod wallet;
pub mod blinks;
pub mod monitoring;
pub mod portfolio;

pub use callback::CallbackHandler;
pub use command::CommandHandler;
pub use menu::MenuCreator;
pub use text::TextMessageHandler;
pub use trading::TradingHandler;
pub use wallet::WalletHandler;
pub use blinks::BlinksHandler;
pub use monitoring::MonitoringHandler;
pub use portfolio::PortfolioHandler;

// Re-export specific menu functions for convenience
pub use menu::{create_main_menu, create_trading_menu, create_wallet_menu, 
               create_analytics_menu, create_portfolio_menu, create_settings_menu};