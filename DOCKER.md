# Docker Deployment Guide for RLM Server

This guide covers how to build, deploy, and manage the RLM server using Docker and Docker Compose in different environments.

## Quick Start

### 1. Basic Docker Run

```bash
# Build the image
docker build -t rlm-server .

# Run the server
docker run -d \
  --name rlm-server \
  -p 8080:8080 \
  -e RLM_LLM_API_KEY=your-openai-api-key \
  rlm-server
```

### 2. Docker Compose (Recommended)

```bash
# Create environment file
echo "OPENAI_API_KEY=your-openai-api-key" > .env

# Start the server
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f rlm-server
```

## Docker Images

### Production Image (`Dockerfile`)

Optimized for production deployment:
- Multi-stage build for minimal size
- Non-root user for security
- Health checks included
- Debian slim base for stability

```bash
# Build production image
docker build -t rlm-server:prod .

# Or use the build script
./scripts/docker-build.sh production
```

### Development Image (`Dockerfile.dev`)

Optimized for development:
- Includes development tools (cargo-watch, etc.)
- Volume mounts for hot reloading
- Debug symbols included
- Larger but more convenient for development

```bash
# Build development image
docker build -f Dockerfile.dev -t rlm-server:dev .

# Or use the build script
./scripts/docker-build.sh development
```

## Docker Compose Configurations

### 1. Development (`docker-compose.dev.yml`)

For local development with:
- Hot reloading with cargo-watch
- Debug logging
- Development services (Redis, PostgreSQL, etc.)
- Relaxed security settings

```bash
ENVIRONMENT=development docker-compose -f docker-compose.dev.yml up -d
```

### 2. Production (`docker-compose.prod.yml`)

For production deployment with:
- Resource limits and reservations
- Security hardening
- Load balancing with Nginx
- Monitoring stack (Prometheus, Grafana, Loki)
- Multi-replica support

```bash
ENVIRONMENT=production docker-compose -f docker-compose.prod.yml up -d
```

### 3. Standard (`docker-compose.yml`)

Balanced configuration for staging or simple deployments.

## Environment Variables

### Required Variables

```bash
# LLM Provider (Required)
RLM_LLM_API_KEY=your-openai-api-key

# Optional but recommended
RLM_LLM_PROVIDER=openai
RLM_LLM_MODEL=gpt-4
```

### Server Configuration

```bash
# Server settings
RLM_SERVER_HOST=0.0.0.0
RLM_SERVER_PORT=8080

# Processing limits
RLM_MAX_CONTEXT_SIZE=10000000
RLM_MAX_RECURSIVE_DEPTH=10
RLM_MAX_ITERATIONS=50

# Logging
RLM_LOG_LEVEL=info
RLM_LOG_FORMAT=json
```

### Production Security

```bash
# API Security (Production)
RLM_REQUIRE_API_KEY=true
RLM_API_KEYS=key1,key2,key3

# Monitoring credentials
GRAFANA_PASSWORD=secure-password
GRAFANA_SECRET_KEY=secure-secret
```

## Build Scripts

### Using the Build Script

The `scripts/docker-build.sh` script provides convenient commands:

```bash
# Build production image
./scripts/docker-build.sh production

# Build development image
./scripts/docker-build.sh development

# Build and push to registry
REGISTRY=docker.io/myorg PUSH=true ./scripts/docker-build.sh registry

# Test the built image
./scripts/docker-build.sh test

# Analyze image size and security
./scripts/docker-build.sh analyze

# Clean up Docker resources
./scripts/docker-build.sh cleanup
```

## Docker Compose Utilities

### Using the Compose Utils Script

The `scripts/docker-compose-utils.sh` script provides management commands:

```bash
# Start services
./scripts/docker-compose-utils.sh start

# Start with monitoring profile
PROFILE=monitoring ./scripts/docker-compose-utils.sh start

# Show status and health
./scripts/docker-compose-utils.sh status
./scripts/docker-compose-utils.sh health

# View logs
./scripts/docker-compose-utils.sh logs
FOLLOW=true ./scripts/docker-compose-utils.sh logs rlm-server

# Scale services
./scripts/docker-compose-utils.sh scale rlm-server 3

# Execute commands in containers
./scripts/docker-compose-utils.sh exec rlm-server bash

# Update services
./scripts/docker-compose-utils.sh update

# Backup and restore data
./scripts/docker-compose-utils.sh backup
./scripts/docker-compose-utils.sh restore ./backups/20240119_143022
```

## Production Deployment

### 1. Resource Requirements

**Minimum Requirements:**
- CPU: 2 cores
- Memory: 4GB RAM
- Storage: 10GB (for logs and data)

**Recommended for Production:**
- CPU: 4+ cores
- Memory: 8GB+ RAM
- Storage: 50GB+ SSD

### 2. Security Hardening

```yaml
# docker-compose.prod.yml security settings
security_opt:
  - no-new-privileges:true
tmpfs:
  - /tmp:rw,noexec,nosuid,size=256m

# Use secrets for sensitive data
secrets:
  openai_api_key:
    external: true
  rlm_api_keys:
    external: true
```

### 3. Monitoring Stack

The production compose file includes:

- **Prometheus**: Metrics collection
- **Grafana**: Visualization and alerting
- **Loki**: Log aggregation
- **Promtail**: Log shipping

Access URLs:
- Grafana: http://localhost:3000
- Prometheus: http://localhost:9090

### 4. Load Balancing

Nginx reverse proxy provides:
- Load balancing across multiple replicas
- SSL termination
- Rate limiting
- Request routing

### 5. Health Checks

Built-in health checks monitor:
- Application responsiveness (`/health` endpoint)
- Backend provider connectivity
- System resource usage

## Storage and Persistence

### Volume Mounts

```yaml
volumes:
  # Application logs
  - ./logs:/app/logs

  # Configuration
  - ./config.prod.yaml:/app/config.yaml:ro

  # Data persistence (if needed)
  - ./data:/app/data
```

### Backup Strategy

```bash
# Automated backup
./scripts/docker-compose-utils.sh backup

# Manual volume backup
docker run --rm \
  -v rlm_data:/source:ro \
  -v $(pwd)/backup:/backup \
  alpine:latest \
  tar czf /backup/rlm-data-$(date +%Y%m%d_%H%M%S).tar.gz -C /source .
```

## Troubleshooting

### Common Issues

1. **Container fails to start**
   ```bash
   # Check logs
   docker-compose logs rlm-server

   # Check configuration
   docker-compose config
   ```

2. **Health check failures**
   ```bash
   # Test health endpoint manually
   curl -f http://localhost:8080/health

   # Check container health
   docker inspect rlm-server --format='{{.State.Health.Status}}'
   ```

3. **Performance issues**
   ```bash
   # Monitor resource usage
   ./scripts/docker-compose-utils.sh resources

   # Check metrics
   curl http://localhost:8080/metrics
   ```

4. **Permission issues**
   ```bash
   # Fix volume permissions
   sudo chown -R 1001:1001 ./logs ./data
   ```

### Debug Mode

Enable debug logging:

```bash
# For specific service
docker-compose exec rlm-server \
  RLM_LOG_LEVEL=debug rlm-server

# For entire stack
ENVIRONMENT=development docker-compose up -d
```

### Log Analysis

```bash
# Follow all logs
docker-compose logs -f

# Filter specific service
docker-compose logs -f rlm-server

# Search logs
docker-compose logs | grep ERROR

# Export logs
docker-compose logs --no-color > rlm-server.log
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Deploy
on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build Docker image
        run: ./scripts/docker-build.sh production
      - name: Test image
        run: ./scripts/docker-build.sh test
      - name: Push to registry
        env:
          REGISTRY: ${{ secrets.DOCKER_REGISTRY }}
          PUSH: true
        run: ./scripts/docker-build.sh registry
```

### Deployment Automation

```bash
# Production deployment script
#!/bin/bash
cd /opt/rlm-server
git pull origin main
ENVIRONMENT=production ./scripts/docker-compose-utils.sh update
./scripts/docker-compose-utils.sh health
```

## Best Practices

### 1. Image Optimization

- Use multi-stage builds
- Minimize layer count
- Use .dockerignore effectively
- Regular security scanning

### 2. Configuration Management

- Use environment variables for configuration
- Mount config files as read-only
- Keep secrets out of images
- Use Docker secrets in swarm mode

### 3. Monitoring and Observability

- Enable comprehensive health checks
- Use structured logging (JSON)
- Monitor resource usage
- Set up alerting for critical issues

### 4. Security

- Run as non-root user
- Use minimal base images
- Regular security updates
- Network segmentation
- API key rotation

### 5. Backup and Recovery

- Regular automated backups
- Test restore procedures
- Document recovery processes
- Keep backups secure and encrypted

This Docker deployment guide provides everything needed to run RLM server in containerized environments, from development to production scale.