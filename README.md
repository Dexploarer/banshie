# 🚀 Solana Trading Bot - Non-Custodial Architecture

A production-ready, non-custodial Solana trading bot with MEV rebates, AI analysis, and secure wallet management.

## ✨ Features

### Core Trading
- **Non-Custodial Design**: Users maintain complete control of their private keys
- **MEV Rebates**: Automatic rebate earning through Helius RPC optimization
- **Jupiter V6 Integration**: Access to best prices across all Solana DEXs
- **AI Market Analysis**: Powered by Groq's Llama 3.1 70B model
- **Real-time Portfolio Tracking**: P&L calculations and position management

### Security & Architecture
- **Zero Private Key Storage**: Bot never has access to user private keys
- **Session-Based Trading**: Temporary encrypted sessions for convenience
- **Multi-Wallet Support**: Manage multiple wallets per user
- **HD Wallet Generation**: BIP39/BIP32 compliant wallet creation
- **Comprehensive Validation**: Input sanitization and security checks

### Refactored Clean Code
- **Custom Error Types**: Type-safe error handling throughout
- **Database Traits**: Clean abstraction for database operations
- **Constants Module**: Centralized configuration values
- **Token Resolver**: Smart token symbol and mint resolution
- **Validation Layer**: Comprehensive input validation

## 🎯 Quick Start

### Prerequisites

- Rust 1.75+
- Telegram Bot Token (from @BotFather)
- Helius API Key (free tier at helius.dev)
- Groq API Key (free tier at groq.com)
- PostgreSQL (optional - uses mock DB by default)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/solana-trading-bot
cd solana-trading-bot
```

2. Create `.env` file:
```env
# Required API Keys
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
HELIUS_API_KEY=your_helius_api_key
GROQ_API_KEY=your_groq_api_key

# Network Configuration
NETWORK=mainnet  # or devnet/testnet

# Trading Configuration
MAX_TRADE_SIZE_SOL=1.0
MIN_TRADE_SIZE_SOL=0.001
SLIPPAGE_BPS=300
PRIORITY_FEE_LAMPORTS=50000

# User Authorization (comma-separated)
ALLOWED_USERS=telegram_id_1,telegram_id_2
ADMIN_USERS=admin_telegram_id

# Feature Flags
ENABLE_BACKRUN_REBATES=true
ENABLE_AI_ANALYSIS=true
ENABLE_PAPER_TRADING=false
```

3. Build and run:
```bash
cargo build --release
cargo run --release
```

## 📱 Bot Commands

### Wallet Management (Non-Custodial)
- `/start` - Initialize bot and set up wallet
- `/wallet` - Manage wallets
- `/import` - Import existing wallet (private key never stored)
- `/export` - Export wallet credentials (secure)
- `/backup` - Create wallet backup

### Trading Commands
- `/balance` - Check wallet balance
- `/buy <token> <amount>` - Buy tokens (requires user signature)
- `/sell <token> <percentage>` - Sell tokens (requires user signature)
- `/portfolio` - View all positions
- `/analyze <token>` - Get AI market analysis
- `/rebates` - View earned MEV rebates

## 🔄 MEV Rebate System

The bot automatically earns MEV rebates through:
1. **Helius RPC Integration**: Optimized transaction routing
2. **Transaction Bundling**: Efficient block inclusion
3. **Priority Fee Optimization**: Smart fee calculation
4. **Backrun Protection**: Protection against sandwich attacks

Rebates are paid instantly in SOL to the user's wallet.

## 🏗️ Architecture

```
src/
├── bot/                  # Telegram bot interface
│   ├── commands.rs       # Command handlers
│   ├── telegram.rs       # Bot implementation
│   └── wallet_setup.rs   # Wallet setup flow
├── trading/              # Trading engine
│   ├── executor.rs       # Non-custodial trade execution
│   ├── dex.rs           # Jupiter DEX integration
│   ├── backrun.rs       # MEV rebate system
│   ├── token_resolver.rs # Token resolution
│   └── types.rs         # Trading types
├── wallet/              # Wallet management
│   ├── generator.rs     # HD wallet generation
│   └── manager.rs       # Wallet lifecycle
├── ai/                  # AI analysis
│   └── groq.rs         # Market analysis
├── db/                  # Database layer
│   ├── traits.rs       # Database interface
│   ├── mock.rs         # Mock implementation
│   └── models.rs       # Data models
├── utils/               # Utilities
│   ├── config.rs       # Configuration
│   └── validation.rs   # Input validation
├── constants.rs         # Global constants
├── errors.rs           # Error types
└── main.rs             # Application entry
```

## 🚀 Deployment

### Railway (Recommended)

1. Connect your GitHub repository
2. Add environment variables
3. Deploy with one click

### Docker

```bash
docker build -t solana-bot .
docker run --env-file .env solana-bot
```

### Local Development

```bash
# Install dependencies
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=info cargo run
```

## 🔐 Security Features

### Non-Custodial Architecture
- **Private keys never leave user's control**
- **All transactions require user signing**
- **No custodial wallet storage**
- **Encrypted session management**

### Wallet Security
- **BIP39/BIP32 HD wallet derivation**
- **Secure mnemonic generation**
- **QR code paper wallet support**
- **Automatic session expiration**

### Input Validation & Safety
- **Comprehensive input sanitization**
- **SQL injection prevention**
- **Rate limiting protection**
- **Slippage protection**
- **Maximum trade size limits**

## 🛠️ Technical Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Rust** | Core Language | Performance & Safety |
| **Solana SDK 1.18** | Blockchain | Core blockchain interaction |
| **Teloxide 0.13** | Bot Framework | Telegram integration |
| **Tokio** | Async Runtime | Concurrent operations |
| **Jupiter V6** | DEX Aggregator | Best price routing |
| **Helius RPC** | MEV Infrastructure | Rebate system |
| **Groq AI** | LLM Provider | Market analysis |
| **BIP39/HMAC** | Cryptography | Wallet generation |

## 📊 Configuration

### Network Types
- `mainnet` - Production trading
- `devnet` - Development testing
- `testnet` - Test network

### Trading Limits
- **Max trade size**: Configurable (default 1 SOL)
- **Min trade size**: 0.001 SOL
- **Slippage tolerance**: 3% default (configurable)
- **Priority fee**: 0.00005 SOL

### Constants Module
All configuration values are centralized in `src/constants.rs` for easy management.

## ✅ Refactoring Completed

### Technical Debt Eliminated
- ✅ **Custom Error Types**: Replaced anyhow with typed errors
- ✅ **Database Abstraction**: Clean trait-based database interface
- ✅ **Constants Module**: Centralized all magic numbers
- ✅ **Token Resolution**: Smart token symbol/mint resolver
- ✅ **Validation Layer**: Comprehensive input validation
- ✅ **Non-Custodial Architecture**: Complete wallet security
- ✅ **Code Organization**: Clean module structure
- ✅ **Removed Unused Code**: Purged all dead code

### Code Quality Improvements
- **Type Safety**: Strong typing throughout
- **Error Handling**: Proper Result types everywhere
- **Async/Await**: Consistent async patterns
- **Documentation**: Comprehensive inline docs
- **Testing**: Unit test structure ready

## 🚨 Important Security Notes

1. **Never share your private keys or mnemonic phrases**
2. **Always verify transaction details before signing**
3. **Start with small amounts when testing**
4. **Use paper trading mode for practice**
5. **Keep secure backups of your wallet**
6. **Enable 2FA on your Telegram account**

## 📄 License

MIT License - see LICENSE file for details

## ⚠️ Disclaimer

This bot is for educational purposes. Trading cryptocurrencies carries risk. Always DYOR and never invest more than you can afford to lose.

## 📝 Project Structure Benefits

### Clean Architecture
- **Separation of Concerns**: Each module has a single responsibility
- **Dependency Injection**: Easy testing and mocking
- **Interface-Based Design**: Database operations use traits
- **Error Propagation**: Consistent error handling

### Performance Optimizations
- **Connection Pooling**: Efficient resource usage
- **Async Operations**: Non-blocking I/O
- **Batch Processing**: Reduced RPC calls
- **Caching Strategy**: Minimize external API calls

## 🔗 Resources

- [Solana Documentation](https://docs.solana.com)
- [Jupiter Aggregator](https://jup.ag)
- [Helius RPC](https://helius.xyz)
- [Groq AI](https://groq.com)
- [Telegram Bot API](https://core.telegram.org/bots/api)
- [BIP39 Specification](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)

---

**Built with ❤️ for the Solana community**