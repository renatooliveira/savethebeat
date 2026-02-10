# Deployment Guide

## Overview

This guide covers deploying savethebeat to production. The application requires:
- Rust runtime environment
- PostgreSQL database
- HTTPS endpoint (for Slack webhooks)
- Environment variables configuration

## Deployment Options

### Option 1: Fly.io (Recommended)

**Pros:** Excellent Rust support, built-in PostgreSQL, free tier, automatic HTTPS, global deployment

**Steps:**

1. **Install flyctl:**
```bash
# macOS
brew install flyctl

# Linux
curl -L https://fly.io/install.sh | sh

# Login
flyctl auth login
```

2. **Create Fly.toml:**
```toml
app = "savethebeat"
primary_region = "iad"

[build]
  builder = "paketobuildpacks/builder:base"
  buildpacks = ["gcr.io/paketo-buildpacks/rust"]

[env]
  PORT = "8080"
  RUST_LOG = "info,savethebeat=debug"
  RUST_LOG_FORMAT = "json"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = false
  auto_start_machines = true
  min_machines_running = 1

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_gb = 1
```

3. **Create PostgreSQL database:**
```bash
flyctl postgres create --name savethebeat-db
flyctl postgres attach savethebeat-db
```

4. **Set secrets (environment variables):**
```bash
flyctl secrets set \
  SPOTIFY_CLIENT_ID="your_client_id" \
  SPOTIFY_CLIENT_SECRET="your_client_secret" \
  SPOTIFY_REDIRECT_URI="https://savethebeat.fly.dev/spotify/callback" \
  BASE_URL="https://savethebeat.fly.dev" \
  SLACK_SIGNING_SECRET="your_slack_secret" \
  SLACK_BOT_TOKEN="xoxb-your-bot-token"
```

5. **Deploy:**
```bash
flyctl launch --no-deploy  # Initialize (skip first deploy)
flyctl deploy              # Deploy application
```

6. **Run migrations:**
```bash
flyctl ssh console
cd /app
DATABASE_URL=$DATABASE_URL ./target/release/savethebeat
# Or run migrations separately if needed
```

7. **Update Slack webhook URL:**
   - Go to Slack App settings
   - Event Subscriptions → Request URL: `https://savethebeat.fly.dev/slack/events`

8. **Monitor:**
```bash
flyctl logs
flyctl status
```

---

### Option 2: Railway

**Pros:** Very simple deployment, PostgreSQL included, good free tier

**Steps:**

1. **Install Railway CLI:**
```bash
npm install -g @railway/cli
railway login
```

2. **Initialize project:**
```bash
railway init
railway link  # Link to Railway project
```

3. **Add PostgreSQL:**
```bash
railway add --database postgres
```

4. **Set environment variables:**
   - Go to Railway dashboard
   - Add variables:
     - `SPOTIFY_CLIENT_ID`
     - `SPOTIFY_CLIENT_SECRET`
     - `SPOTIFY_REDIRECT_URI` (will be `https://your-app.up.railway.app/spotify/callback`)
     - `BASE_URL` (will be `https://your-app.up.railway.app`)
     - `SLACK_SIGNING_SECRET`
     - `SLACK_BOT_TOKEN`
     - `PORT=8080`
     - `RUST_LOG=info,savethebeat=debug`

5. **Deploy:**
```bash
railway up
```

6. **Run migrations:**
```bash
railway run sqlx migrate run
```

---

### Option 3: Docker (Self-Hosted)

**Pros:** Maximum control, works on any VPS (DigitalOcean, Linode, AWS, etc.)

**Steps:**

1. **Create Dockerfile:**
```dockerfile
# Build stage
FROM rust:1.93 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY .sqlx ./.sqlx

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/savethebeat /app/savethebeat
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

CMD ["/app/savethebeat"]
```

2. **Create docker-compose.yml:**
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:14-alpine
    environment:
      POSTGRES_USER: savethebeat
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: savethebeat
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    restart: unless-stopped

  app:
    build: .
    environment:
      DATABASE_URL: postgresql://savethebeat:${DB_PASSWORD}@postgres:5432/savethebeat
      PORT: 8080
      RUST_LOG: info,savethebeat=debug
      RUST_LOG_FORMAT: json
      SPOTIFY_CLIENT_ID: ${SPOTIFY_CLIENT_ID}
      SPOTIFY_CLIENT_SECRET: ${SPOTIFY_CLIENT_SECRET}
      SPOTIFY_REDIRECT_URI: ${SPOTIFY_REDIRECT_URI}
      BASE_URL: ${BASE_URL}
      SLACK_SIGNING_SECRET: ${SLACK_SIGNING_SECRET}
      SLACK_BOT_TOKEN: ${SLACK_BOT_TOKEN}
    ports:
      - "8080:8080"
    depends_on:
      - postgres
    restart: unless-stopped

volumes:
  postgres_data:
```

3. **Create .env.production:**
```bash
DB_PASSWORD=your_secure_password
SPOTIFY_CLIENT_ID=your_client_id
SPOTIFY_CLIENT_SECRET=your_client_secret
SPOTIFY_REDIRECT_URI=https://your-domain.com/spotify/callback
BASE_URL=https://your-domain.com
SLACK_SIGNING_SECRET=your_slack_secret
SLACK_BOT_TOKEN=xoxb-your-bot-token
```

4. **Deploy:**
```bash
# Build and start
docker-compose --env-file .env.production up -d

# Run migrations
docker-compose exec app /app/savethebeat
# Or use sqlx-cli if installed in container
```

5. **Setup reverse proxy (Nginx):**
```nginx
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

---

### Option 4: Render

**Pros:** Simple web UI, managed PostgreSQL, free SSL

**Steps:**

1. **Connect GitHub repo to Render**

2. **Create PostgreSQL database:**
   - Dashboard → New → PostgreSQL
   - Note the internal connection string

3. **Create Web Service:**
   - Dashboard → New → Web Service
   - Connect your GitHub repo
   - Settings:
     - **Name:** savethebeat
     - **Environment:** Rust
     - **Build Command:** `cargo build --release`
     - **Start Command:** `./target/release/savethebeat`
     - **Environment Variables:** Add all required vars

4. **Run migrations:**
   - Use Render Shell or deploy a migration script

---

## Post-Deployment Checklist

### 1. Update Spotify App Settings
- [ ] Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
- [ ] Update redirect URI to: `https://your-domain.com/spotify/callback`

### 2. Update Slack App Settings
- [ ] Go to [Slack API Apps](https://api.slack.com/apps)
- [ ] Event Subscriptions → Request URL: `https://your-domain.com/slack/events`
- [ ] Verify endpoint (Slack will send challenge)

### 3. Run Database Migrations
```bash
# If using fly.io
flyctl ssh console -C "cd /app && DATABASE_URL=\$DATABASE_URL sqlx migrate run"

# If using Railway
railway run sqlx migrate run

# If using Docker
docker-compose exec app sqlx migrate run
```

### 4. Verify Deployment
```bash
# Health check
curl https://your-domain.com/health

# Should return:
# {"status":"ok","version":"0.1.0"}
```

### 5. Test OAuth Flow
- Visit: `https://your-domain.com/spotify/connect?slack_workspace_id=TEST&slack_user_id=TEST`
- Complete Spotify authorization
- Verify success page displays

### 6. Test Slack Integration
- Post Spotify link in Slack channel
- Mention the bot: `@savethebeat`
- Verify bot adds reaction (✅/♻️/❌)
- Check logs for any errors

---

## Environment Variables Reference

| Variable | Required | Example | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | `postgresql://user:pass@host:5432/db` | PostgreSQL connection string |
| `PORT` | No (default 3000) | `8080` | Server port |
| `HOST` | No (default 0.0.0.0) | `0.0.0.0` | Server host |
| `RUST_LOG` | No | `info,savethebeat=debug` | Log level |
| `RUST_LOG_FORMAT` | No | `json` | Log format (json/pretty) |
| `SPOTIFY_CLIENT_ID` | Yes | `abc123...` | Spotify app client ID |
| `SPOTIFY_CLIENT_SECRET` | Yes | `xyz789...` | Spotify app client secret |
| `SPOTIFY_REDIRECT_URI` | Yes | `https://domain.com/spotify/callback` | OAuth redirect URI |
| `BASE_URL` | Yes | `https://domain.com` | Application base URL |
| `SLACK_SIGNING_SECRET` | No* | `abc123...` | Slack signing secret |
| `SLACK_BOT_TOKEN` | No* | `xoxb-...` | Slack bot token |

*Required for Slack integration

---

## Monitoring & Logging

### View Logs

**Fly.io:**
```bash
flyctl logs
flyctl logs -a savethebeat
```

**Railway:**
```bash
railway logs
```

**Docker:**
```bash
docker-compose logs -f app
```

### Structured Logging

Set `RUST_LOG_FORMAT=json` for structured logs suitable for log aggregation services (DataDog, Papertrail, etc.)

Example JSON log:
```json
{
  "timestamp": "2026-02-10T00:00:00Z",
  "level": "INFO",
  "target": "savethebeat",
  "message": "Starting server on 0.0.0.0:8080"
}
```

---

## Scaling Considerations

### Current Architecture (MVP)
- Single instance deployment
- In-memory OAuth state store
- Suitable for small teams (<100 users)

### Future Scaling (Phase 4+)
- **Horizontal scaling:** Add Redis for OAuth state
- **Database:** Connection pooling (already implemented)
- **Rate limiting:** Implement per-user limits
- **Monitoring:** Add metrics (Prometheus, Grafana)
- **Alerting:** Set up error notifications

---

## Backup & Recovery

### Database Backups

**Fly.io:**
```bash
flyctl postgres backup create savethebeat-db
flyctl postgres backup list savethebeat-db
```

**Railway:**
- Automatic daily backups included
- Manual backups via dashboard

**Docker/Self-hosted:**
```bash
# Backup
docker-compose exec postgres pg_dump -U savethebeat savethebeat > backup.sql

# Restore
docker-compose exec -T postgres psql -U savethebeat savethebeat < backup.sql
```

### Application State
- OAuth state: Ephemeral, no backup needed
- Database: Regular backups critical (user_auth, save_action_log tables)

---

## Troubleshooting

### Issue: Slack webhook fails verification
- Check `SLACK_SIGNING_SECRET` is correct
- Verify server time is synchronized (for timestamp validation)
- Check logs for signature mismatch errors

### Issue: Spotify OAuth fails
- Verify `SPOTIFY_REDIRECT_URI` matches exactly in:
  - Environment variables
  - Spotify Developer Dashboard
- Check `SPOTIFY_CLIENT_ID` and `SPOTIFY_CLIENT_SECRET`

### Issue: Database connection fails
- Verify `DATABASE_URL` format
- Check PostgreSQL is running
- Verify network connectivity
- Check migrations have run: `sqlx migrate info`

### Issue: 502 Bad Gateway
- Check application is running: `curl http://localhost:8080/health`
- Verify port configuration
- Check reverse proxy configuration

---

## Cost Estimates

### Fly.io
- **Free tier:** 3 shared-cpu VMs, 3GB storage (sufficient for small teams)
- **Paid:** ~$5-10/month for dedicated resources

### Railway
- **Free tier:** $5 credit/month (sufficient for small teams)
- **Paid:** Usage-based, ~$5-15/month

### Render
- **Free tier:** Available (with limitations)
- **Paid:** $7/month for web service + $7/month for PostgreSQL

### Self-hosted VPS
- **DigitalOcean/Linode:** $5-10/month for basic droplet
- **Domain + SSL:** $10-15/year

---

## Security Checklist

- [ ] HTTPS enabled (required for Slack webhooks)
- [ ] Environment variables secured (not in git)
- [ ] Database credentials rotated regularly
- [ ] Firewall configured (only ports 80, 443, 22 open)
- [ ] Regular security updates applied
- [ ] Logs monitored for suspicious activity
- [ ] Backup strategy implemented
- [ ] Rate limiting configured (Phase 4)

---

## Support

For deployment issues:
- Check logs first
- Review [TESTING.md](./TESTING.md) for troubleshooting
- File issue: https://github.com/renatooliveira/savethebeat/issues
