# Production Deployment Guide

## Overview

This guide covers deploying the Solana Trading Bot to production with high availability, security, and scalability. The system consists of three main components:

- **Convex Backend**: Serverless backend with real-time data sync
- **React Dashboard**: User interface for portfolio management
- **Rust Integration Service**: Telegram bot and webhook processing

## Prerequisites

Before starting deployment, ensure you have:

- [ ] Domain name and SSL certificate
- [ ] Convex Pro account
- [ ] Vercel/Netlify account for frontend hosting
- [ ] VPS or cloud server for Rust service
- [ ] Telegram Bot API token
- [ ] Environment variables configured
- [ ] Database backups ready

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   React App     │    │  Convex Backend │    │  Rust Service   │
│   (Vercel)      │◄──►│   (Convex)      │◄──►│    (VPS)        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                        │                        │
         ▼                        ▼                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│     Users       │    │   Database      │    │  Telegram API   │
│   (Browser)     │    │   (Convex)      │    │   (External)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Part 1: Convex Backend Deployment

### 1.1 Environment Setup

Create production environment configuration:

```bash
# Install Convex CLI
npm install -g convex

# Navigate to convex directory
cd convex/

# Login to Convex
convex login

# Create production deployment
convex deploy --prod
```

### 1.2 Environment Variables

Configure production environment variables in Convex dashboard:

```bash
# Required Environment Variables
OPENAI_API_KEY=sk-...
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
JUPITER_API_BASE=https://quote-api.jup.ag
TELEGRAM_BOT_TOKEN=your_bot_token
WEBHOOK_SECRET=your_webhook_secret

# Optional Configuration
LOG_LEVEL=info
RATE_LIMIT_REQUESTS_PER_MINUTE=100
MAX_CONCURRENT_TRADES=10
DEFAULT_SLIPPAGE=1.0
```

### 1.3 Database Schema Verification

Verify all tables and indexes are properly deployed:

```bash
# Check deployment status
convex status

# Verify table schema
convex dashboard

# Run post-deployment tests
npm run test:integration
```

### 1.4 API Rate Limits

Configure production rate limits:

```typescript
// convex/constants.ts
export const RATE_LIMITS = {
  QUERIES_PER_MINUTE: 1000,
  MUTATIONS_PER_MINUTE: 500,
  ACTIONS_PER_MINUTE: 200,
  MAX_CONCURRENT_USERS: 10000,
};
```

## Part 2: React Dashboard Deployment

### 2.1 Build Configuration

Configure production build settings:

```bash
cd dashboard/

# Install dependencies
npm install

# Build for production
npm run build

# Test production build locally
npm start
```

### 2.2 Vercel Deployment

Deploy to Vercel:

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy to production
vercel --prod

# Set environment variables
vercel env add NEXT_PUBLIC_CONVEX_URL
vercel env add NEXT_PUBLIC_CONVEX_SITE_URL
vercel env add NEXT_PUBLIC_APP_ENV production
```

### 2.3 Environment Configuration

Create production environment file:

```env
# .env.production
NEXT_PUBLIC_CONVEX_URL=https://your-app.convex.site
NEXT_PUBLIC_CONVEX_SITE_URL=https://your-app.convex.cloud
NEXT_PUBLIC_APP_ENV=production
NEXT_PUBLIC_ENABLE_ANALYTICS=true
NEXT_PUBLIC_SENTRY_DSN=your_sentry_dsn
```

### 2.4 Performance Optimization

Optimize for production performance:

```javascript
// next.config.js
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  poweredByHeader: false,
  compress: true,
  images: {
    formats: ['image/webp', 'image/avif'],
    minimumCacheTTL: 86400,
  },
  experimental: {
    optimizeCss: true,
    optimizePackageImports: ['lucide-react', '@radix-ui/react-icons'],
  },
};

module.exports = nextConfig;
```

## Part 3: Rust Integration Service

### 3.1 Server Setup

Set up a production server for the Rust service:

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install system dependencies
sudo apt install -y build-essential pkg-config libssl-dev
```

### 3.2 Application Deployment

Build and deploy the Rust application:

```bash
# Clone repository
git clone https://github.com/your-org/solana-trading-bot.git
cd solana-trading-bot/convex/rust-integration/

# Build release binary
cargo build --release

# Create deployment directory
sudo mkdir -p /opt/trading-bot
sudo cp target/release/integration-service /opt/trading-bot/
```

### 3.3 Service Configuration

Create systemd service:

```bash
# /etc/systemd/system/trading-bot.service
[Unit]
Description=Solana Trading Bot Integration Service
After=network.target

[Service]
Type=simple
User=trading-bot
WorkingDirectory=/opt/trading-bot
ExecStart=/opt/trading-bot/integration-service
Restart=always
RestartSec=10
Environment=RUST_LOG=info
EnvironmentFile=/etc/trading-bot/environment

[Install]
WantedBy=multi-user.target
```

### 3.4 Environment Configuration

Create environment file:

```bash
# /etc/trading-bot/environment
CONVEX_URL=https://your-app.convex.site
CONVEX_SITE_URL=https://your-app.convex.cloud
TELEGRAM_BOT_TOKEN=your_bot_token
WEBHOOK_PORT=8080
WEBHOOK_PATH=/webhook
RUST_LOG=info
```

### 3.5 Service Management

Start and enable the service:

```bash
# Create user for service
sudo useradd --system --create-home trading-bot

# Set permissions
sudo chown -R trading-bot:trading-bot /opt/trading-bot
sudo chmod 600 /etc/trading-bot/environment

# Enable and start service
sudo systemctl enable trading-bot
sudo systemctl start trading-bot

# Check status
sudo systemctl status trading-bot
```

## Part 4: Monitoring and Logging

### 4.1 Application Monitoring

Set up monitoring for all components:

```bash
# Install monitoring tools
sudo apt install -y htop iotop nethogs

# Create monitoring script
cat > /opt/trading-bot/monitor.sh << 'EOF'
#!/bin/bash
# Monitor system resources and application health

LOG_FILE="/var/log/trading-bot/monitor.log"
DATE=$(date '+%Y-%m-%d %H:%M:%S')

# Check service status
if systemctl is-active --quiet trading-bot; then
    echo "[$DATE] Service: RUNNING" >> $LOG_FILE
else
    echo "[$DATE] Service: STOPPED - Restarting..." >> $LOG_FILE
    systemctl restart trading-bot
fi

# Check memory usage
MEMORY_USAGE=$(free | grep Mem | awk '{printf "%.2f", $3/$2 * 100}')
echo "[$DATE] Memory Usage: ${MEMORY_USAGE}%" >> $LOG_FILE

# Check disk space
DISK_USAGE=$(df -h / | awk 'NR==2 {print $5}')
echo "[$DATE] Disk Usage: ${DISK_USAGE}" >> $LOG_FILE
EOF

chmod +x /opt/trading-bot/monitor.sh
```

### 4.2 Log Management

Configure log rotation:

```bash
# /etc/logrotate.d/trading-bot
/var/log/trading-bot/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 trading-bot trading-bot
    postrotate
        systemctl reload trading-bot > /dev/null 2>&1 || true
    endrotate
}
```

### 4.3 Health Check Endpoints

Implement health checks in Rust service:

```rust
// src/health.rs
use warp::Filter;
use serde_json::json;

pub fn health_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().timestamp(),
                "version": env!("CARGO_PKG_VERSION")
            }))
        });

    let readiness = warp::path("ready")
        .and(warp::get())
        .and_then(readiness_check);

    health.or(readiness)
}

async fn readiness_check() -> Result<impl warp::Reply, warp::Rejection> {
    // Check Convex connectivity
    // Check Telegram API connectivity
    // Return appropriate status
    Ok(warp::reply::json(&json!({
        "status": "ready",
        "checks": {
            "convex": "healthy",
            "telegram": "healthy"
        }
    })))
}
```

## Part 5: Security Configuration

### 5.1 Firewall Setup

Configure UFW firewall:

```bash
# Enable firewall
sudo ufw --force enable

# Allow SSH
sudo ufw allow 22/tcp

# Allow webhook port
sudo ufw allow 8080/tcp

# Allow HTTPS
sudo ufw allow 443/tcp

# Deny all other inbound traffic
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Check status
sudo ufw status verbose
```

### 5.2 SSL/TLS Configuration

Set up SSL certificates:

```bash
# Install Certbot
sudo apt install -y certbot

# Get SSL certificate
sudo certbot certonly --standalone \
  -d your-webhook-domain.com \
  --email your-email@example.com \
  --agree-tos \
  --no-eff-email

# Set up auto-renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

### 5.3 Secrets Management

Secure environment variables:

```bash
# Create secrets directory
sudo mkdir -p /etc/trading-bot/secrets
sudo chmod 700 /etc/trading-bot/secrets

# Store sensitive data
echo "your_telegram_token" | sudo tee /etc/trading-bot/secrets/telegram_token
echo "your_webhook_secret" | sudo tee /etc/trading-bot/secrets/webhook_secret

# Set restrictive permissions
sudo chmod 600 /etc/trading-bot/secrets/*
sudo chown trading-bot:trading-bot /etc/trading-bot/secrets/*
```

## Part 6: Backup and Recovery

### 6.1 Database Backup

Convex handles database backups automatically, but create additional backups:

```bash
# Create backup script
cat > /opt/trading-bot/backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR="/var/backups/trading-bot"
DATE=$(date '+%Y%m%d_%H%M%S')

mkdir -p $BACKUP_DIR

# Export Convex data (implement based on Convex CLI capabilities)
convex export --output "$BACKUP_DIR/convex_backup_$DATE.json"

# Backup configuration files
tar -czf "$BACKUP_DIR/config_backup_$DATE.tar.gz" /etc/trading-bot/

# Clean old backups (keep 30 days)
find $BACKUP_DIR -type f -mtime +30 -delete

echo "Backup completed: $DATE"
EOF

chmod +x /opt/trading-bot/backup.sh
```

### 6.2 Automated Backups

Schedule regular backups:

```bash
# Add to crontab
sudo crontab -e
# Add: 0 2 * * * /opt/trading-bot/backup.sh >> /var/log/trading-bot/backup.log 2>&1
```

## Part 7: Performance Tuning

### 7.1 System Optimization

Optimize server performance:

```bash
# /etc/sysctl.d/99-trading-bot.conf
net.core.somaxconn = 65536
net.ipv4.tcp_max_syn_backlog = 65536
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_tw_reuse = 1
```

### 7.2 Application Tuning

Configure Rust application for performance:

```bash
# /etc/trading-bot/environment additions
TOKIO_WORKER_THREADS=4
MAX_BLOCKING_THREADS=16
STACK_SIZE=2097152
```

## Part 8: Testing Production Deployment

### 8.1 Smoke Tests

Run basic functionality tests:

```bash
# Test Convex API
curl -X POST https://your-app.convex.site/api/query \
  -H "Content-Type: application/json" \
  -d '{"path":"queries/system:healthCheck","args":{},"format":"json"}'

# Test React dashboard
curl -I https://your-dashboard-domain.com

# Test Rust service
curl http://your-server:8080/health
```

### 8.2 Load Testing

Perform load testing:

```bash
# Install Apache Bench
sudo apt install -y apache2-utils

# Test API endpoints
ab -n 1000 -c 10 https://your-app.convex.site/api/query

# Monitor during testing
htop
iotop
```

## Part 9: Deployment Checklist

### Pre-Deployment
- [ ] All environment variables configured
- [ ] SSL certificates obtained and configured
- [ ] Firewall rules configured
- [ ] Monitoring systems set up
- [ ] Backup procedures tested
- [ ] Load testing completed
- [ ] Security audit performed

### Deployment
- [ ] Convex backend deployed
- [ ] React dashboard deployed
- [ ] Rust service deployed
- [ ] All services started and healthy
- [ ] DNS records configured
- [ ] SSL certificates verified

### Post-Deployment
- [ ] Health checks passing
- [ ] Monitoring alerts configured
- [ ] Log aggregation working
- [ ] Backup schedules active
- [ ] Performance metrics collected
- [ ] End-to-end testing completed

## Part 10: Maintenance Procedures

### 10.1 Regular Maintenance

Schedule regular maintenance tasks:

```bash
# Weekly maintenance script
cat > /opt/trading-bot/maintenance.sh << 'EOF'
#!/bin/bash
echo "Starting weekly maintenance..."

# Update system packages
sudo apt update && sudo apt upgrade -y

# Clean up logs older than 30 days
find /var/log/trading-bot -name "*.log" -mtime +30 -delete

# Restart services if needed
sudo systemctl daemon-reload
sudo systemctl restart trading-bot

# Check disk space
df -h

echo "Maintenance completed"
EOF
```

### 10.2 Monitoring Alerts

Set up alerting for critical issues:

```bash
# Install monitoring agent
# Configure alerts for:
# - Service downtime
# - High memory usage (>80%)
# - High disk usage (>85%)
# - Failed health checks
# - High error rates
```

### 10.3 Update Procedures

Document update procedures:

1. **Test updates in staging environment**
2. **Create backup before updates**
3. **Deploy during low-traffic periods**
4. **Monitor system health after updates**
5. **Rollback procedure if issues occur**

## Troubleshooting

### Common Issues

1. **Service won't start**
   - Check environment variables
   - Verify file permissions
   - Check system logs: `journalctl -u trading-bot`

2. **High memory usage**
   - Monitor with `htop`
   - Check for memory leaks
   - Consider increasing server resources

3. **API errors**
   - Check Convex backend status
   - Verify network connectivity
   - Review application logs

4. **Telegram bot not responding**
   - Verify bot token
   - Check webhook configuration
   - Test Telegram API connectivity

### Emergency Procedures

1. **Service Down**
   ```bash
   sudo systemctl restart trading-bot
   sudo systemctl status trading-bot
   ```

2. **High Load**
   ```bash
   # Check system resources
   htop
   iotop
   
   # Scale resources if needed
   # Implement rate limiting
   ```

3. **Database Issues**
   ```bash
   # Check Convex dashboard
   # Review error logs
   # Contact Convex support if needed
   ```

## Support and Documentation

- **Internal Documentation**: `/opt/trading-bot/docs/`
- **Log Files**: `/var/log/trading-bot/`
- **Configuration**: `/etc/trading-bot/`
- **Service Status**: `systemctl status trading-bot`

For additional support, contact the development team or refer to the troubleshooting guide.

---

This production deployment guide ensures your Solana Trading Bot runs reliably in a production environment with proper monitoring, security, and maintenance procedures.