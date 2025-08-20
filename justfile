# Solana Trading Bot - Development Commands

# Show available commands
default:
    @just --list

# Development commands
dev:
    cargo run

# Build the project
build:
    cargo build

# Build for production
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Format code
fmt:
    cargo fmt

# Check code without building
check:
    cargo check

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Fix clippy warnings where possible
lint-fix:
    cargo clippy --fix --allow-dirty

# Clean build artifacts
clean:
    cargo clean

# Run full CI checks (format, lint, test, build)
ci: fmt lint test build

# Database commands
db-migrate:
    # sqlx migrate run
    echo "Database migrations temporarily disabled"

db-reset:
    # sqlx database reset
    echo "Database reset temporarily disabled"

# Docker commands
docker-build:
    docker build -t solana-trading-bot .

docker-run:
    docker-compose up

docker-dev:
    docker-compose up --build

# Frontend commands
frontend-dev:
    cd mini-app && npm run dev

frontend-build:
    cd mini-app && npm run build

frontend-install:
    cd mini-app && npm install

# Full development setup
setup: frontend-install
    cargo build
    echo "Setup complete! Run 'just dev' to start the backend and 'just frontend-dev' for the frontend"

# Start both backend and frontend
dev-full:
    #!/usr/bin/env bash
    echo "Starting backend..."
    cargo run &
    BACKEND_PID=$!
    echo "Starting frontend..."
    cd mini-app && npm run dev &
    FRONTEND_PID=$!
    echo "Backend PID: $BACKEND_PID, Frontend PID: $FRONTEND_PID"
    echo "Press Ctrl+C to stop both services"
    wait

# Deployment commands
deploy-railway:
    railway up

deploy-docker:
    docker-compose -f docker-compose.prod.yml up --build

# Security audit
audit:
    cargo audit

# Update dependencies
update:
    cargo update
    cd mini-app && npm update

# Generate documentation
docs:
    cargo doc --open

# Profile the application
profile:
    cargo build --release
    perf record --call-graph=dwarf target/release/solana-trading-bot
    perf report

# Benchmark
bench:
    cargo bench