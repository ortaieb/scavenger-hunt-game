# Scavenger Hunt Game Server

A Rust-based REST API server for managing scavenger hunt challenges with real-time location validation, image proof verification, and comprehensive audit logging.

## Features

- **JWT Authentication**: Dual-token system for users and participants
- **Challenge Management**: Create, start, and manage scavenger hunt challenges
- **Geolocation Validation**: GPS-based waypoint check-ins with configurable radius
- **Image Proof Verification**: Integration with external image validation service
- **Role-based Authorization**: Granular permissions for different user types
- **Real-time Tracking**: Participant progress monitoring with audit logs
- **PostgreSQL Integration**: Robust data persistence with migrations

## Prerequisites

### Required Software

- **Rust** (1.70+): Install from [rustup.rs](https://rustup.rs/)
- **Docker** & **Docker Compose**: Container runtime
- **Git**: Version control

### System Dependencies

```bash
# macOS
brew install docker docker-compose

# Ubuntu/Debian
sudo apt-get install docker.io docker-compose

# Arch Linux
sudo pacman -S docker docker-compose

# Start Docker service
sudo systemctl start docker
sudo systemctl enable docker
```

## Database Setup

### 1. PostgreSQL with Docker

Create a `docker-compose.dev.yml` file for development:

```yaml
version: '3.8'
services:
  postgres:
    image: postgres:15
    container_name: scavenger_postgres
    environment:
      POSTGRES_DB: scavenger_hunt
      POSTGRES_USER: scavenger_user
      POSTGRES_PASSWORD: secure_password
    ports:
      - "5432:5432"
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    restart: unless-stopped
```

### 2. Initialize Database

Create `init.sql` file:

```sql
-- Create test database
CREATE DATABASE scavenger_test;
GRANT ALL PRIVILEGES ON DATABASE scavenger_test TO scavenger_user;

-- Grant schema permissions
GRANT ALL ON SCHEMA public TO scavenger_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO scavenger_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO scavenger_user;
```

### 3. Start Database

```bash
# Create data directory
mkdir -p data/postgres

# Start PostgreSQL container
docker-compose -f docker-compose.dev.yml up -d postgres

# Check container status
docker ps

# View logs
docker-compose -f docker-compose.dev.yml logs postgres
```

### 4. Verify Connection

```bash
# Connect using Docker
docker exec -it scavenger_postgres psql -U scavenger_user -d scavenger_hunt

# Or using local psql client
psql -h localhost -U scavenger_user -d scavenger_hunt
```

### 5. Stop Database

```bash
# Stop container
docker-compose -f docker-compose.dev.yml down

# Stop and remove volumes
docker-compose -f docker-compose.dev.yml down -v
```

## Project Setup

### 1. Clone and Build

```bash
git clone <repository-url>
cd game-server
cargo build --release
```

### 2. Environment Configuration

```bash
# Copy environment template
cp .env.example .env

# Edit configuration
nano .env
```

### Required Environment Variables

```bash
# Database Configuration
DATABASE_URL=postgresql://scavenger_user:secure_password@localhost:5432/scavenger_hunt

# JWT Configuration (Generate secure random key)
JWT_SECRET=your-super-secret-jwt-key-minimum-32-characters-long-random-value

# Server Configuration
HOST=127.0.0.1
PORT=3000

# External Services
IMAGE_CHECKER_URL=http://localhost:8080
IMAGE_BASE_DIR=/var/images

# Logging
RUST_LOG=info
```

### 3. Generate JWT Secret

```bash
# Generate secure random key
openssl rand -hex 32
```

### 4. Database Migrations

```bash
# Run migrations
cargo run --bin migrate

# Or start server (auto-runs migrations)
cargo run
```

## Development Setup

### 1. Install Development Tools

```bash
# Install cargo-watch for auto-reload
cargo install cargo-watch

# Install sqlx-cli for database operations
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### 2. Database Operations

```bash
# Create migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run

# Revert migration
sqlx migrate revert

# Generate query metadata (for offline compilation)
cargo sqlx prepare
```

### 3. Development Server

```bash
# Auto-reload on changes
cargo watch -x run

# Manual start
cargo run
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Integration Tests

```bash
# Set test database
export DATABASE_URL=postgresql://scavenger_user:secure_password@localhost:5432/scavenger_test

# Run integration tests
cargo test --test '*'
```

### API Testing

```bash
# Health check
curl http://localhost:3000/health

# User registration
curl -X POST http://localhost:3000/authentication/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "test@example.com",
    "password": "password123",
    "nickname": "TestUser",
    "roles": ["user.verified", "challenge.participant"]
  }'
```

## Production Deployment

### 1. Build Release Binary

```bash
cargo build --release
```

### 2. Environment Setup

```bash
# Production environment
export DATABASE_URL=postgresql://user:pass@prod-db:5432/scavenger_hunt
export JWT_SECRET=<secure-production-key>
export HOST=0.0.0.0
export PORT=3000
export RUST_LOG=warn
```

### 3. Database Configuration

```bash
# Ensure database exists and is accessible
psql $DATABASE_URL -c "SELECT 1;"

# Run migrations
./target/release/scavenger-hunt-game-server migrate
```

### 4. Start Server

```bash
./target/release/scavenger-hunt-game-server
```

### 5. Health Monitoring

```bash
# Health endpoint
curl http://your-server:3000/health

# Expected response
{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00Z",
  "database": "healthy"
}
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/scavenger-hunt-game-server .
EXPOSE 3000
CMD ["./scavenger-hunt-game-server"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  db:
    image: postgres:15
    environment:
      POSTGRES_DB: scavenger_hunt
      POSTGRES_USER: scavenger_user
      POSTGRES_PASSWORD: secure_password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  app:
    build: .
    environment:
      DATABASE_URL: postgresql://scavenger_user:secure_password@db:5432/scavenger_hunt
      JWT_SECRET: your-secure-jwt-secret
      HOST: 0.0.0.0
      PORT: 3000
    ports:
      - "3000:3000"
    depends_on:
      - db

volumes:
  postgres_data:
```

## API Endpoints

### Authentication
- `POST /authentication/register` - User registration
- `POST /authentication/login` - User login
- `POST /challenge/authentication` - Participant token creation

### Challenges
- `POST /challenges` - Create challenge (manager role)
- `GET /challenges/{id}` - Get challenge details
- `POST /challenges/start` - Start challenge (moderator)
- `POST /challenges/{id}/invite/{user_id}` - Invite participant

### Waypoints
- `POST /challenges/waypoints/{id}/checkin` - Location check-in
- `POST /challenges/waypoints/{id}/proof` - Submit image proof

### System
- `GET /health` - Health check

## Troubleshooting

### Common Issues

#### Database Connection Errors
```bash
# Check PostgreSQL container is running
docker ps | grep postgres

# Start database if not running
docker-compose -f docker-compose.dev.yml up -d postgres

# Check connection
psql $DATABASE_URL -c "SELECT 1;"

# Check user permissions via Docker
docker exec -it scavenger_postgres psql -U postgres -c "\du"

# View container logs
docker-compose -f docker-compose.dev.yml logs postgres
```

#### Compilation Errors
```bash
# SQLx offline mode
export SQLX_OFFLINE=true
cargo build

# Generate query cache
cargo sqlx prepare
```

#### Port Already in Use
```bash
# Find process using port
lsof -i :3000

# Kill process
kill -9 <PID>
```

#### JWT Token Issues
```bash
# Verify JWT secret length (minimum 32 characters)
echo $JWT_SECRET | wc -c

# Generate new secret
openssl rand -hex 32
```

### Log Analysis

```bash
# Increase log level
export RUST_LOG=debug

# Filter logs
cargo run 2>&1 | grep ERROR

# JSON structured logs (production)
export RUST_LOG=info,scavenger_hunt_game_server=debug
```

## Security Considerations

### Production Checklist

- [ ] Use strong JWT secret (32+ random characters)
- [ ] Enable HTTPS/TLS in reverse proxy
- [ ] Configure database connection pooling
- [ ] Set up database backups
- [ ] Monitor failed authentication attempts
- [ ] Configure CORS appropriately
- [ ] Use environment variables for secrets
- [ ] Enable database connection encryption
- [ ] Set up log rotation
- [ ] Configure firewall rules

### Database Security

```sql
-- Revoke public schema access
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO scavenger_user;

-- Create read-only user for monitoring
CREATE USER monitoring WITH PASSWORD 'monitor_password';
GRANT CONNECT ON DATABASE scavenger_hunt TO monitoring;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO monitoring;
```

## Performance Tuning

### Database Optimization

```sql
-- Add indexes for frequently queried columns
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_challenges_moderator ON challenges(challenge_moderator);
CREATE INDEX idx_participants_challenge ON challenge_participants(challenge_id);
CREATE INDEX idx_audit_log_event_time ON audit_log(event_time DESC);
```

### Application Configuration

```bash
# Increase connection pool size
export DB_MAX_CONNECTIONS=20

# Adjust server workers
export TOKIO_WORKER_THREADS=4
```

## License

[License information]

## Contributing

[Contributing guidelines]

## Support

For issues and questions:
- Create GitHub issues for bugs
- Check logs with `RUST_LOG=debug`
- Review database connection settings
- Verify environment variables are set correctly