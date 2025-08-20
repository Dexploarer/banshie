# Development Setup Guide

## Overview

This guide provides step-by-step instructions for setting up a complete development environment for the Solana Trading Bot project. The development setup includes all components needed for local development and testing.

## Prerequisites

### System Requirements

- **Operating System**: Windows 10/11, macOS 10.15+, or Ubuntu 18.04+
- **Node.js**: Version 18.0 or higher
- **Rust**: Latest stable version
- **Git**: Version 2.25 or higher
- **Memory**: At least 8GB RAM
- **Storage**: At least 10GB free space

### Required Accounts

- [ ] GitHub account for code access
- [ ] Convex account (free tier available)
- [ ] Telegram Bot API token
- [ ] OpenAI API key (optional, for AI features)

## Part 1: Environment Setup

### 1.1 Install Node.js and npm

#### Windows
```powershell
# Using winget
winget install OpenJS.NodeJS

# Or download from https://nodejs.org/
```

#### macOS
```bash
# Using Homebrew
brew install node

# Or download from https://nodejs.org/
```

#### Ubuntu/Linux
```bash
# Using NodeSource repository
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Verify installation
node --version
npm --version
```

### 1.2 Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 1.3 Install Development Tools

```bash
# Install Convex CLI globally
npm install -g convex

# Install Vercel CLI (optional)
npm install -g vercel

# Install useful development tools
cargo install cargo-watch
cargo install cargo-edit
```

## Part 2: Project Setup

### 2.1 Clone Repository

```bash
# Clone the project
git clone https://github.com/your-org/solana-trading-bot.git
cd solana-trading-bot

# Create development branch
git checkout -b feature/your-feature-name
```

### 2.2 Project Structure

Understand the project structure:

```
solana-trading-bot/
├── convex/                     # Backend logic
│   ├── src/                    # Convex functions
│   ├── rust-integration/       # Rust integration library
│   ├── dashboard/              # React frontend
│   ├── docs/                   # API documentation
│   └── package.json
├── docs/                       # Project documentation
│   ├── deployment/
│   ├── api/
│   └── guides/
└── README.md
```

## Part 3: Convex Backend Setup

### 3.1 Initialize Convex Project

```bash
cd convex/

# Install dependencies
npm install

# Login to Convex (creates account if needed)
npx convex login

# Initialize development deployment
npx convex dev --configure
```

### 3.2 Environment Variables

Create development environment file:

```bash
# convex/.env.local
OPENAI_API_KEY=sk-your-openai-key-here
SOLANA_RPC_URL=https://api.devnet.solana.com
JUPITER_API_BASE=https://quote-api.jup.ag
TELEGRAM_BOT_TOKEN=your-dev-bot-token
WEBHOOK_SECRET=dev-webhook-secret-123
LOG_LEVEL=debug
```

### 3.3 Database Schema Setup

The schema will be automatically deployed when you start the development server:

```bash
# Start Convex development server
npx convex dev
```

This will:
- Deploy all Convex functions
- Set up database tables and indexes
- Generate TypeScript types
- Start real-time sync

### 3.4 Verify Backend Setup

Test the backend is working:

```bash
# In a new terminal window
cd convex/

# Run health check
npx convex run queries/system:healthCheck

# Check deployed functions
npx convex functions list
```

## Part 4: React Dashboard Setup

### 4.1 Install Dependencies

```bash
cd dashboard/

# Install npm dependencies
npm install

# Install additional development tools
npm install -D @types/node @types/react @types/react-dom
```

### 4.2 Development Environment

Create environment file:

```bash
# dashboard/.env.local
NEXT_PUBLIC_CONVEX_URL=https://your-dev-app.convex.site
NEXT_PUBLIC_CONVEX_SITE_URL=https://your-dev-app.convex.cloud
NEXT_PUBLIC_APP_ENV=development
NEXT_PUBLIC_ENABLE_ANALYTICS=false
```

### 4.3 Start Development Server

```bash
# Start React development server
npm run dev

# The dashboard will be available at http://localhost:3000
```

### 4.4 Verify Dashboard Setup

- Open http://localhost:3000 in your browser
- Check that the dashboard loads without errors
- Verify real-time data sync is working
- Test basic navigation and components

## Part 5: Rust Integration Setup

### 5.1 Build Rust Project

```bash
cd convex/rust-integration/

# Build the project
cargo build

# Run tests
cargo test

# Install development dependencies
cargo install cargo-watch
```

### 5.2 Environment Configuration

Create development environment file:

```bash
# convex/rust-integration/.env
CONVEX_URL=https://your-dev-app.convex.site
CONVEX_SITE_URL=https://your-dev-app.convex.cloud
TELEGRAM_BOT_TOKEN=your-dev-bot-token
WEBHOOK_PORT=8080
WEBHOOK_PATH=/webhook
RUST_LOG=debug
```

### 5.3 Start Integration Service

```bash
# Start the Rust integration service with auto-reload
cargo watch -x run

# Or run normally
cargo run
```

### 5.4 Verify Rust Integration

Test the service is working:

```bash
# Test health endpoint
curl http://localhost:8080/health

# Check service logs
# The service should connect to Convex and Telegram APIs
```

## Part 6: Development Workflow

### 6.1 Running All Services

Use multiple terminal windows or tabs:

**Terminal 1 - Convex Backend:**
```bash
cd convex/
npx convex dev
```

**Terminal 2 - React Dashboard:**
```bash
cd dashboard/
npm run dev
```

**Terminal 3 - Rust Service:**
```bash
cd convex/rust-integration/
cargo watch -x run
```

### 6.2 Using Scripts

Create convenience scripts for development:

```bash
# package.json scripts (in root directory)
{
  "scripts": {
    "dev": "npm-run-all --parallel dev:*",
    "dev:convex": "cd convex && npx convex dev",
    "dev:dashboard": "cd dashboard && npm run dev",
    "dev:rust": "cd convex/rust-integration && cargo watch -x run",
    "build:all": "npm run build:convex && npm run build:dashboard && npm run build:rust",
    "build:convex": "cd convex && npx convex deploy --dry-run",
    "build:dashboard": "cd dashboard && npm run build",
    "build:rust": "cd convex/rust-integration && cargo build --release",
    "test:all": "npm run test:convex && npm run test:dashboard && npm run test:rust",
    "test:convex": "cd convex && npm test",
    "test:dashboard": "cd dashboard && npm test",
    "test:rust": "cd convex/rust-integration && cargo test",
    "lint:all": "npm run lint:convex && npm run lint:dashboard && npm run lint:rust",
    "lint:convex": "cd convex && npm run lint",
    "lint:dashboard": "cd dashboard && npm run lint",
    "lint:rust": "cd convex/rust-integration && cargo clippy"
  }
}
```

## Part 7: Testing Setup

### 7.1 Backend Testing

```bash
cd convex/

# Install testing dependencies
npm install -D jest @types/jest ts-jest

# Run tests
npm test

# Run tests in watch mode
npm run test:watch
```

### 7.2 Frontend Testing

```bash
cd dashboard/

# Install testing libraries
npm install -D @testing-library/react @testing-library/jest-dom @testing-library/user-event

# Run tests
npm test

# Run tests in watch mode
npm run test:watch
```

### 7.3 Rust Testing

```bash
cd convex/rust-integration/

# Run unit tests
cargo test

# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

## Part 8: Development Tools Configuration

### 8.1 VS Code Configuration

Create `.vscode/settings.json`:

```json
{
  "typescript.preferences.importModuleSpecifier": "relative",
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll.eslint": true
  },
  "rust-analyzer.checkOnSave.command": "clippy",
  "files.associations": {
    "*.ts": "typescript",
    "*.tsx": "typescriptreact"
  }
}
```

Create `.vscode/extensions.json`:

```json
{
  "recommendations": [
    "ms-vscode.vscode-typescript-next",
    "bradlc.vscode-tailwindcss",
    "rust-lang.rust-analyzer",
    "ms-vscode.vscode-json"
  ]
}
```

### 8.2 Git Configuration

Create `.gitignore`:

```gitignore
# Dependencies
node_modules/
target/

# Environment files
.env
.env.local
.env.development
.env.production

# Build outputs
.next/
dist/
build/

# IDE
.vscode/settings.json
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Coverage
coverage/
*.lcov

# Convex
.convex/
```

### 8.3 EditorConfig

Create `.editorconfig`:

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true
indent_style = space
indent_size = 2

[*.rs]
indent_size = 4

[*.md]
trim_trailing_whitespace = false
```

## Part 9: Database Development

### 9.1 Schema Migrations

Convex handles schema migrations automatically, but you can track changes:

```typescript
// convex/schema.ts
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  users: defineTable({
    telegramId: v.number(),
    username: v.string(),
    // Add new fields here for migrations
  }).index("by_telegram_id", ["telegramId"]),
  
  // Add new tables here
});
```

### 9.2 Seed Data

Create development seed data:

```typescript
// convex/seed.ts
import { mutation } from "./_generated/server";

export const seedDevelopmentData = mutation({
  args: {},
  handler: async (ctx) => {
    // Create test users
    await ctx.db.insert("users", {
      telegramId: 123456789,
      username: "test_user",
      isPremium: false,
      settings: {
        defaultSlippage: 1.0,
        riskTolerance: "medium",
      },
    });

    // Add more seed data as needed
    console.log("Development data seeded");
  },
});
```

### 9.3 Database Management

Useful database commands:

```bash
# Clear all data (development only)
npx convex run mutations/admin:clearAllData

# Seed development data
npx convex run seed:seedDevelopmentData

# Export data for backup
npx convex export --output backup.json

# Import data from backup
npx convex import backup.json
```

## Part 10: Debugging and Logging

### 10.1 Convex Debugging

Enable detailed logging:

```typescript
// convex/utils/logger.ts
export function debugLog(message: string, data?: any) {
  if (process.env.LOG_LEVEL === "debug") {
    console.log(`[DEBUG] ${message}`, data);
  }
}
```

### 10.2 React Development Tools

Install browser extensions:
- React Developer Tools
- Redux DevTools (if using Redux)

### 10.3 Rust Debugging

Configure debugging in Rust:

```rust
// src/main.rs
use log::{debug, info, warn, error};

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("debug")
    ).init();
    
    info!("Starting development server...");
    // Your application code
}
```

## Part 11: Common Development Tasks

### 11.1 Adding New API Endpoints

1. Define the function in `convex/src/`
2. Export it in the appropriate module
3. Update TypeScript types
4. Add tests
5. Update documentation

### 11.2 Adding New React Components

1. Create component in `dashboard/src/components/`
2. Add TypeScript interfaces
3. Implement component logic
4. Add tests
5. Update storybook (if used)

### 11.3 Extending Rust Integration

1. Add new module in `convex/rust-integration/src/`
2. Update `lib.rs` exports
3. Add integration tests
4. Update API documentation

## Part 12: Troubleshooting

### 12.1 Common Issues

**Convex connection issues:**
```bash
# Check network connectivity
curl https://api.convex.dev/health

# Verify authentication
npx convex auth status

# Clear cache and restart
rm -rf node_modules/.convex
npx convex dev
```

**React build issues:**
```bash
# Clear Next.js cache
rm -rf .next

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install

# Check TypeScript errors
npm run type-check
```

**Rust compilation issues:**
```bash
# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Check for issues
cargo check
cargo clippy
```

### 12.2 Performance Issues

**Slow development server:**
- Increase Node.js memory: `export NODE_OPTIONS="--max-old-space-size=4096"`
- Use faster file system (SSD recommended)
- Close unnecessary applications

**High memory usage:**
- Monitor with `htop` or Activity Monitor
- Restart development servers periodically
- Use `--max-old-space-size` flag for Node.js

## Part 13: Development Best Practices

### 13.1 Code Style

- Use TypeScript for all new code
- Follow ESLint and Prettier configurations
- Use meaningful variable and function names
- Add JSDoc comments for public APIs

### 13.2 Testing

- Write unit tests for new functions
- Add integration tests for API endpoints
- Test error conditions and edge cases
- Aim for >80% code coverage

### 13.3 Version Control

- Create feature branches for new work
- Write descriptive commit messages
- Squash commits before merging
- Keep pull requests focused and small

## Part 14: Development Checklist

### Initial Setup
- [ ] Node.js and npm installed
- [ ] Rust toolchain installed
- [ ] Repository cloned
- [ ] Convex account created
- [ ] Environment variables configured
- [ ] All services start without errors

### Daily Development
- [ ] Pull latest changes from main branch
- [ ] Run tests before starting work
- [ ] Start all development servers
- [ ] Check for TypeScript errors
- [ ] Verify database connectivity

### Before Committing
- [ ] All tests pass
- [ ] No TypeScript errors
- [ ] Code formatted with Prettier
- [ ] ESLint warnings addressed
- [ ] Documentation updated if needed

## Support

For development issues:

1. Check this guide first
2. Search existing GitHub issues
3. Check the troubleshooting section
4. Ask in team chat or create a GitHub issue

---

This development setup guide should get you up and running with the complete Solana Trading Bot development environment. Follow the checklist to ensure everything is configured correctly.