# savethebeat

Slack ↔ Spotify integration bot

## Development

### Prerequisites
- Rust 1.93+ (edition 2024)

### Setup
1. Copy environment variables:
   ```bash
   cp .env.example .env
   ```

2. Run the service:
   ```bash
   cargo run
   ```

3. Test the health endpoint:
   ```bash
   curl http://localhost:3000/health
   ```

### CI Commands

Format code:
```bash
cargo fmt
```

Run linter:
```bash
cargo clippy
```

Run tests:
```bash
cargo test
```

Run all checks:
```bash
cargo fmt && cargo clippy && cargo test
```

## Environment Variables

See `.env.example` for configuration options.
