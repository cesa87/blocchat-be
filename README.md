# BlocChat Backend

Rust + Actix-web backend for BlocChat Phase 2: Payments

## Features

- **Payment Tracking**: Record and track token transfers on Base network
- **Transaction History**: Query transaction history by conversation
- **Status Updates**: Monitor transaction confirmations
- **RESTful API**: Clean API for frontend integration

## Tech Stack

- **Rust** with **Actix-web** for high-performance API
- **PostgreSQL** for transaction storage
- **Ethers-rs** for blockchain interactions
- **SQLx** for type-safe database queries

## Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Node.js (for frontend)

## Quick Start

### Option 1: Docker Compose (Recommended for Local Development)

```bash
# Set Base RPC URL
export BASE_RPC_URL="your-base-rpc-url"

# Start PostgreSQL + Backend
docker-compose up -d

# View logs
docker-compose logs -f backend

# Stop services
docker-compose down
```

The backend will be available at `http://localhost:8080`

### Option 2: Manual Setup

#### 1. Install Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PostgreSQL (macOS)
brew install postgresql@14
brew services start postgresql@14
```

#### 2. Database Setup

```bash
# Create database
createdb blocchat

# Run migrations
psql -d blocchat -f migrations/001_create_transactions.sql
```

#### 3. Environment Configuration

```bash
cp .env.example .env
# Edit .env with your configuration
```

#### 4. Build and Run

```bash
# Development
cargo run

# Production build
cargo build --release
./target/release/blocchat-backend
```

## API Endpoints

### Health Check
```
GET /api/health
```

### Create Transaction
```
POST /api/payments/transactions
Content-Type: application/json

{
  "tx_hash": "0x...",
  "from_address": "0x...",
  "to_address": "0x...",
  "amount": "1000000000000000000",
  "token_address": null,
  "chain_id": 8453,
  "conversation_id": "xmtp-conversation-id",
  "message_id": "xmtp-message-id"
}
```

### Get Transaction
```
GET /api/payments/transactions/{tx_hash}
```

### Get Conversation Transactions
```
GET /api/payments/conversations/{conversation_id}/transactions
```

## Deployment to AWS EC2

### Option 1: Docker Deployment (Recommended)

#### 1. Launch EC2 Instance

```bash
# Ubuntu 22.04 LTS
# t3.small or larger recommended
# Security Group: Allow 8080 (from ALB), 22 (SSH)
```

#### 2. Install Docker on EC2

```bash
ssh -i your-key.pem ubuntu@your-ec2-ip

# Update system
sudo apt update && sudo apt upgrade -y

# Install Docker
sudo apt install docker.io docker-compose -y
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker ubuntu

# Log out and back in for group changes
exit
ssh -i your-key.pem ubuntu@your-ec2-ip
```

#### 3. Deploy with Docker Compose

```bash
# Clone repository
git clone your-repo
cd blocchat-backend

# Set environment variables
export BASE_RPC_URL="your-base-rpc-url"

# Update CORS in docker-compose.yml for your CloudFront domain
# CORS_ORIGINS: https://your-cloudfront-domain.cloudfront.net

# Start services
docker-compose up -d

# Check logs
docker-compose logs -f

# Auto-restart on reboot
sudo nano /etc/systemd/system/blocchat-docker.service
```

Systemd service file for Docker:
```ini
[Unit]
Description=BlocChat Docker Compose
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/ubuntu/blocchat-backend
ExecStart=/usr/bin/docker-compose up -d
ExecStop=/usr/bin/docker-compose down

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable blocchat-docker
sudo systemctl start blocchat-docker
```

### Option 2: Native Deployment

#### 1. Launch EC2 Instance

```bash
# Ubuntu 22.04 LTS
# t3.small or larger
# Security Group: Allow 8080, 22
```

#### 2. Install Dependencies on EC2

```bash
ssh -i your-key.pem ubuntu@your-ec2-ip

# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install PostgreSQL
sudo apt install postgresql postgresql-contrib -y
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database
sudo -u postgres createdb blocchat
sudo -u postgres psql -c "CREATE USER blocchat WITH ENCRYPTED PASSWORD 'your-password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE blocchat TO blocchat;"
```

#### 3. Deploy Application

```bash
# Clone or copy your code
git clone your-repo
cd blocchat-backend

# Build
cargo build --release

# Run migrations
psql -h localhost -U blocchat -d blocchat -f migrations/001_create_transactions.sql

# Create systemd service
sudo nano /etc/systemd/system/blocchat-backend.service
```

Systemd service file:
```ini
[Unit]
Description=BlocChat Backend
After=network.target postgresql.service

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/blocchat-backend
Environment="DATABASE_URL=postgresql://blocchat:your-password@localhost:5432/blocchat"
Environment="BASE_RPC_URL=your-base-rpc-url"
Environment="CORS_ORIGINS=https://your-cloudfront-domain.cloudfront.net"
Environment="RUST_LOG=info"
ExecStart=/home/ubuntu/blocchat-backend/target/release/blocchat-backend
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
# Start service
sudo systemctl daemon-reload
sudo systemctl enable blocchat-backend
sudo systemctl start blocchat-backend
sudo systemctl status blocchat-backend
```

### ALB + CloudFront Configuration

#### 1. Set up Application Load Balancer

1. Create Target Group:
   - Protocol: HTTP
   - Port: 8080
   - Target type: Instance
   - Health check path: `/api/health`
   - Health check interval: 30s
   - Add EC2 instance as target

2. Create ALB:
   - Scheme: Internet-facing
   - Listeners: HTTP (80) and HTTPS (443)
   - Forward to target group

3. Configure Security Groups:
   - ALB SG: Allow 80, 443 from 0.0.0.0/0
   - EC2 SG: Allow 8080 from ALB SG only

#### 2. Update CloudFront Distribution

1. Add ALB as origin:
   - Origin domain: `your-alb.region.elb.amazonaws.com`
   - Protocol: HTTPS (if using HTTPS listener)
   - Origin path: empty

2. Add cache behavior:
   - Path pattern: `/api/*`
   - Origin: ALB origin
   - Viewer protocol: Redirect HTTP to HTTPS
   - Allowed methods: GET, HEAD, OPTIONS, PUT, POST, PATCH, DELETE
   - Cache policy: CachingDisabled (or custom with low TTL)
   - Origin request policy: AllViewer

3. Update CORS:
   - Ensure backend CORS_ORIGINS includes CloudFront domain
   - Response headers policy: CORS-with-preflight-and-SecurityHeadersPolicy

## Environment Variables

See `.env.example` for all available configuration options.

Key variables:
- `DATABASE_URL`: PostgreSQL connection string
- `BASE_RPC_URL`: Base network RPC endpoint
- `CORS_ALLOWED_ORIGINS`: Comma-separated list of allowed origins (CloudFront domain)

## Development

```bash
# Run with auto-reload (install cargo-watch)
cargo install cargo-watch
cargo watch -x run

# Run tests
cargo test

# Check code
cargo clippy
cargo fmt
```

## License

MIT
