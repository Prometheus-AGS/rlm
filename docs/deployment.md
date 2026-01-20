# RLM Server Deployment Guide

This guide provides comprehensive instructions for deploying the RLM (Recursive Language Model) OpenAI-compatible server in various environments.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Configuration](#configuration)
- [Local Development Deployment](#local-development-deployment)
- [Docker Deployment](#docker-deployment)
- [Production Deployment](#production-deployment)
- [Cloud Deployment](#cloud-deployment)
- [Load Balancing and High Availability](#load-balancing-and-high-availability)
- [Monitoring and Observability](#monitoring-and-observability)
- [Security Considerations](#security-considerations)
- [Troubleshooting](#troubleshooting)
- [Maintenance and Updates](#maintenance-and-updates)

## Overview

The RLM server is a high-performance OpenAI-compatible HTTP API server that implements the RLM (Recursive Language Model) architecture for handling arbitrarily long context inputs. It provides:

- OpenAI-compatible REST API endpoints
- Server-Sent Events (SSE) streaming for real-time responses
- Multi-backend LLM provider support
- Sandboxed Rhai REPL environment
- Comprehensive metrics and observability
- Production-ready performance and reliability

## Prerequisites

### System Requirements

**Minimum Requirements:**
- CPU: 2 cores (4+ recommended for production)
- Memory: 4GB RAM (8GB+ recommended for production)
- Storage: 10GB available space
- Network: Stable internet connection for LLM providers

**Recommended Production:**
- CPU: 8+ cores
- Memory: 16GB+ RAM
- Storage: 50GB+ SSD storage
- Network: High bandwidth, low latency connection

### Software Dependencies

- **Rust**: Version 1.75 or later
- **Docker**: Version 20.10+ (for containerized deployment)
- **Git**: For source code access
- **OpenSSL**: For HTTPS/TLS support

### LLM Provider Access

Configure at least one LLM provider:
- **OpenAI**: API key with GPT-4 access
- **Anthropic**: API key with Claude access
- **Azure OpenAI**: Endpoint and API key
- **Local Models**: Compatible API endpoint

## Configuration

### Environment Variables

Create a `.env` file or set environment variables:

```bash
# LLM Provider Configuration
OPENAI_API_KEY=sk-your-openai-api-key
ANTHROPIC_API_KEY=your-anthropic-api-key
AZURE_OPENAI_ENDPOINT=https://your-resource.openai.azure.com/
AZURE_OPENAI_API_KEY=your-azure-api-key

# Server Configuration
RLM_HOST=0.0.0.0
RLM_PORT=8080
RLM_LOG_LEVEL=info
RLM_MAX_CONCURRENT_REQUESTS=100
RLM_REQUEST_TIMEOUT=300

# REPL Configuration
RLM_REPL_TIMEOUT=30
RLM_REPL_MAX_MEMORY=128
RLM_REPL_MAX_INSTRUCTIONS=1000000

# Security Configuration
RLM_CORS_ORIGINS=*
RLM_API_KEY_REQUIRED=false
RLM_ALLOWED_ORIGINS=http://localhost:3000,https://yourdomain.com

# Metrics and Monitoring
RLM_METRICS_ENABLED=true
RLM_PROMETHEUS_PORT=9090
RLM_HEALTH_CHECK_INTERVAL=30
```

### Configuration File

Alternatively, use a YAML configuration file (`config.yaml`):

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  max_concurrent_requests: 100
  request_timeout: 300
  cors_origins: "*"

llm_providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
    model: "gpt-4"
    timeout: 120
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    base_url: "https://api.anthropic.com"
    model: "claude-3-sonnet-20240229"
    timeout: 120

repl:
  backend: "rhai"
  timeout: 30
  max_memory: 134217728  # 128MB
  max_instructions: 1000000

security:
  api_key_required: false
  allowed_origins:
    - "http://localhost:3000"
    - "https://yourdomain.com"
  rate_limiting:
    requests_per_minute: 60
    burst_size: 10

monitoring:
  metrics_enabled: true
  prometheus_port: 9090
  health_check_interval: 30
  logging:
    level: "info"
    format: "json"
```

## Local Development Deployment

### Direct Binary Execution

1. **Clone and Build:**
   ```bash
   git clone https://github.com/your-org/rlm.git
   cd rlm
   cargo build --release --package rlm-server
   ```

2. **Configure Environment:**
   ```bash
   cp .env.example .env
   # Edit .env with your LLM provider credentials
   ```

3. **Run the Server:**
   ```bash
   cargo run --bin rlm-server -- --config config.yaml --port 8080
   ```

4. **Verify Deployment:**
   ```bash
   curl http://localhost:8080/health
   curl http://localhost:8080/v1/models
   ```

### Development with Hot Reload

For development with automatic reloading:

```bash
cargo install cargo-watch
cargo watch -x 'run --bin rlm-server'
```

## Docker Deployment

### Single Container

1. **Build Docker Image:**
   ```bash
   docker build -t rlm-server:latest .
   ```

2. **Run Container:**
   ```bash
   docker run -d \
     --name rlm-server \
     -p 8080:8080 \
     -p 9090:9090 \
     -e OPENAI_API_KEY=your-api-key \
     -e RLM_LOG_LEVEL=info \
     rlm-server:latest
   ```

3. **With Configuration File:**
   ```bash
   docker run -d \
     --name rlm-server \
     -p 8080:8080 \
     -v $(pwd)/config.yaml:/app/config.yaml \
     -v $(pwd)/.env:/app/.env \
     rlm-server:latest --config /app/config.yaml
   ```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  rlm-server:
    build: .
    container_name: rlm-server
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RLM_LOG_LEVEL=info
      - RLM_METRICS_ENABLED=true
    volumes:
      - ./config.yaml:/app/config.yaml:ro
      - ./logs:/app/logs
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  prometheus:
    image: prom/prometheus:latest
    container_name: rlm-prometheus
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/usr/share/prometheus/console_libraries'
      - '--web.console.templates=/usr/share/prometheus/consoles'
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    container_name: rlm-grafana
    ports:
      - "3001:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning:ro
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    restart: unless-stopped

volumes:
  prometheus_data:
  grafana_data:
```

Deploy with:
```bash
docker-compose up -d
```

## Production Deployment

### Systemd Service (Linux)

1. **Create Service File** (`/etc/systemd/system/rlm-server.service`):
   ```ini
   [Unit]
   Description=RLM OpenAI-Compatible Server
   After=network.target
   StartLimitIntervalSec=0

   [Service]
   Type=simple
   Restart=always
   RestartSec=5
   User=rlm
   Group=rlm
   WorkingDirectory=/opt/rlm-server
   ExecStart=/opt/rlm-server/bin/rlm-server --config /opt/rlm-server/config.yaml
   EnvironmentFile=/opt/rlm-server/.env
   StandardOutput=journal
   StandardError=journal
   SyslogIdentifier=rlm-server

   # Security settings
   NoNewPrivileges=true
   PrivateTmp=true
   ProtectSystem=strict
   ProtectHome=true
   ReadWritePaths=/opt/rlm-server/logs

   [Install]
   WantedBy=multi-user.target
   ```

2. **Setup Service:**
   ```bash
   # Create user and directories
   sudo useradd --system --home /opt/rlm-server --shell /bin/false rlm
   sudo mkdir -p /opt/rlm-server/{bin,logs}
   sudo chown -R rlm:rlm /opt/rlm-server

   # Install binary
   sudo cp target/release/rlm-server /opt/rlm-server/bin/
   sudo cp config.yaml /opt/rlm-server/
   sudo cp .env /opt/rlm-server/

   # Enable and start service
   sudo systemctl enable rlm-server
   sudo systemctl start rlm-server
   sudo systemctl status rlm-server
   ```

### Reverse Proxy with Nginx

1. **Install Nginx:**
   ```bash
   sudo apt update
   sudo apt install nginx certbot python3-certbot-nginx
   ```

2. **Configure Nginx** (`/etc/nginx/sites-available/rlm-server`):
   ```nginx
   upstream rlm_backend {
       server 127.0.0.1:8080;
       keepalive 32;
   }

   server {
       listen 80;
       server_name yourdomain.com www.yourdomain.com;

       # Redirect HTTP to HTTPS
       return 301 https://$server_name$request_uri;
   }

   server {
       listen 443 ssl http2;
       server_name yourdomain.com www.yourdomain.com;

       # SSL Configuration
       ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
       ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;
       ssl_session_timeout 1d;
       ssl_session_cache shared:SSL:50m;
       ssl_stapling on;
       ssl_stapling_verify on;

       # Security Headers
       add_header X-Frame-Options DENY;
       add_header X-Content-Type-Options nosniff;
       add_header X-XSS-Protection "1; mode=block";
       add_header Strict-Transport-Security "max-age=31536000" always;

       # Rate Limiting
       limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
       limit_req zone=api burst=20 nodelay;

       # Main API endpoints
       location / {
           proxy_pass http://rlm_backend;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection 'upgrade';
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
           proxy_cache_bypass $http_upgrade;
           proxy_read_timeout 300s;
           proxy_connect_timeout 75s;
       }

       # SSE Streaming endpoints
       location /v1/chat/completions {
           proxy_pass http://rlm_backend;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection 'upgrade';
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
           proxy_cache_bypass $http_upgrade;
           proxy_buffering off;
           proxy_read_timeout 300s;
           proxy_connect_timeout 75s;
       }

       # Health check endpoint
       location /health {
           proxy_pass http://rlm_backend;
           access_log off;
       }

       # Metrics endpoint (restrict access)
       location /metrics {
           proxy_pass http://127.0.0.1:9090/metrics;
           allow 127.0.0.1;
           allow 10.0.0.0/8;
           deny all;
       }
   }
   ```

3. **Enable and Configure SSL:**
   ```bash
   sudo ln -s /etc/nginx/sites-available/rlm-server /etc/nginx/sites-enabled/
   sudo nginx -t
   sudo systemctl reload nginx
   sudo certbot --nginx -d yourdomain.com -d www.yourdomain.com
   ```

## Cloud Deployment

### AWS EC2 Deployment

1. **Launch EC2 Instance:**
   - Instance Type: t3.large or larger
   - AMI: Ubuntu 22.04 LTS
   - Security Group: Allow ports 22, 80, 443, 8080

2. **Setup Script:**
   ```bash
   #!/bin/bash
   # EC2 User Data Script

   # Update system
   apt update && apt upgrade -y

   # Install dependencies
   apt install -y curl git build-essential pkg-config libssl-dev nginx

   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source ~/.cargo/env

   # Create RLM user
   useradd --system --home /opt/rlm-server --shell /bin/false rlm
   mkdir -p /opt/rlm-server
   chown rlm:rlm /opt/rlm-server

   # Clone and build RLM
   git clone https://github.com/your-org/rlm.git /tmp/rlm
   cd /tmp/rlm
   cargo build --release --package rlm-server
   cp target/release/rlm-server /opt/rlm-server/
   cp config.default.yaml /opt/rlm-server/config.yaml
   chown -R rlm:rlm /opt/rlm-server

   # Setup systemd service
   cat > /etc/systemd/system/rlm-server.service << 'EOF'
   [Unit]
   Description=RLM Server
   After=network.target

   [Service]
   Type=simple
   User=rlm
   Group=rlm
   WorkingDirectory=/opt/rlm-server
   ExecStart=/opt/rlm-server/rlm-server --config /opt/rlm-server/config.yaml
   Restart=always

   [Install]
   WantedBy=multi-user.target
   EOF

   systemctl enable rlm-server
   systemctl start rlm-server
   ```

### AWS ECS Deployment

1. **Task Definition** (`ecs-task-definition.json`):
   ```json
   {
     "family": "rlm-server",
     "networkMode": "awsvpc",
     "requiresCompatibilities": ["FARGATE"],
     "cpu": "1024",
     "memory": "2048",
     "executionRoleArn": "arn:aws:iam::ACCOUNT:role/ecsTaskExecutionRole",
     "taskRoleArn": "arn:aws:iam::ACCOUNT:role/ecsTaskRole",
     "containerDefinitions": [
       {
         "name": "rlm-server",
         "image": "your-ecr-repo/rlm-server:latest",
         "portMappings": [
           {
             "containerPort": 8080,
             "protocol": "tcp"
           }
         ],
         "environment": [
           {
             "name": "RLM_HOST",
             "value": "0.0.0.0"
           },
           {
             "name": "RLM_PORT",
             "value": "8080"
           }
         ],
         "secrets": [
           {
             "name": "OPENAI_API_KEY",
             "valueFrom": "arn:aws:secretsmanager:region:account:secret:rlm/openai-api-key"
           }
         ],
         "logConfiguration": {
           "logDriver": "awslogs",
           "options": {
             "awslogs-group": "/aws/ecs/rlm-server",
             "awslogs-region": "us-east-1",
             "awslogs-stream-prefix": "ecs"
           }
         },
         "healthCheck": {
           "command": ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"],
           "interval": 30,
           "timeout": 10,
           "retries": 3,
           "startPeriod": 60
         }
       }
     ]
   }
   ```

2. **Deploy with AWS CLI:**
   ```bash
   # Register task definition
   aws ecs register-task-definition --cli-input-json file://ecs-task-definition.json

   # Create service
   aws ecs create-service \
     --cluster rlm-cluster \
     --service-name rlm-server \
     --task-definition rlm-server:1 \
     --desired-count 2 \
     --launch-type FARGATE \
     --network-configuration "awsvpcConfiguration={subnets=[subnet-12345,subnet-67890],securityGroups=[sg-abcdef],assignPublicIp=ENABLED}" \
     --load-balancers targetGroupArn=arn:aws:elasticloadbalancing:region:account:targetgroup/rlm-tg/1234567890,containerName=rlm-server,containerPort=8080
   ```

### Kubernetes Deployment

1. **Deployment Manifest** (`k8s-deployment.yaml`):
   ```yaml
   apiVersion: apps/v1
   kind: Deployment
   metadata:
     name: rlm-server
     labels:
       app: rlm-server
   spec:
     replicas: 3
     selector:
       matchLabels:
         app: rlm-server
     template:
       metadata:
         labels:
           app: rlm-server
       spec:
         containers:
         - name: rlm-server
           image: your-registry/rlm-server:latest
           ports:
           - containerPort: 8080
           env:
           - name: RLM_HOST
             value: "0.0.0.0"
           - name: RLM_PORT
             value: "8080"
           - name: OPENAI_API_KEY
             valueFrom:
               secretKeyRef:
                 name: rlm-secrets
                 key: openai-api-key
           resources:
             requests:
               memory: "1Gi"
               cpu: "500m"
             limits:
               memory: "2Gi"
               cpu: "1000m"
           livenessProbe:
             httpGet:
               path: /health
               port: 8080
             initialDelaySeconds: 30
             periodSeconds: 30
           readinessProbe:
             httpGet:
               path: /health
               port: 8080
             initialDelaySeconds: 5
             periodSeconds: 10
   ---
   apiVersion: v1
   kind: Service
   metadata:
     name: rlm-server-service
   spec:
     selector:
       app: rlm-server
     ports:
     - protocol: TCP
       port: 80
       targetPort: 8080
     type: LoadBalancer
   ---
   apiVersion: networking.k8s.io/v1
   kind: Ingress
   metadata:
     name: rlm-server-ingress
     annotations:
       kubernetes.io/ingress.class: nginx
       cert-manager.io/cluster-issuer: letsencrypt-prod
   spec:
     tls:
     - hosts:
       - api.yourdomain.com
       secretName: rlm-server-tls
     rules:
     - host: api.yourdomain.com
       http:
         paths:
         - path: /
           pathType: Prefix
           backend:
             service:
               name: rlm-server-service
               port:
                 number: 80
   ```

2. **Deploy to Kubernetes:**
   ```bash
   kubectl apply -f k8s-deployment.yaml
   kubectl get pods -l app=rlm-server
   kubectl logs -l app=rlm-server --tail=100
   ```

## Load Balancing and High Availability

### HAProxy Configuration

```haproxy
global
    daemon
    maxconn 4096

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms
    option httplog

frontend rlm_frontend
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/yourdomain.com.pem
    redirect scheme https if !{ ssl_fc }
    default_backend rlm_backend

backend rlm_backend
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200

    server rlm1 10.0.1.10:8080 check
    server rlm2 10.0.1.11:8080 check
    server rlm3 10.0.1.12:8080 check
```

### Auto-scaling Configuration

For Kubernetes HPA:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rlm-server-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rlm-server
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## Monitoring and Observability

### Prometheus Configuration

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'rlm-server'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

rule_files:
  - "rlm_alerts.yml"
```

### Grafana Dashboard

Import dashboard JSON or create panels for:
- Request rate and latency
- Error rates by endpoint
- REPL execution metrics
- Memory and CPU usage
- LLM provider response times
- Concurrent connections

### Alert Rules

Create `rlm_alerts.yml`:

```yaml
groups:
- name: rlm_server_alerts
  rules:
  - alert: RLMServerDown
    expr: up{job="rlm-server"} == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "RLM Server is down"
      description: "RLM Server has been down for more than 1 minute."

  - alert: HighErrorRate
    expr: rate(rlm_requests_total{status=~"5.."}[5m]) > 0.1
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "High error rate on RLM Server"
      description: "Error rate is {{ $value }} errors per second."

  - alert: HighLatency
    expr: histogram_quantile(0.95, rate(rlm_request_duration_seconds_bucket[5m])) > 30
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High latency on RLM Server"
      description: "95th percentile latency is {{ $value }} seconds."
```

### Logging Configuration

Configure structured logging in production:

```yaml
logging:
  level: "info"
  format: "json"
  output: "stdout"
  fields:
    service: "rlm-server"
    version: "1.0.0"
  filters:
    - module: "hyper"
      level: "warn"
    - module: "tokio"
      level: "warn"
```

## Security Considerations

### API Security

1. **API Key Authentication:**
   ```yaml
   security:
     api_key_required: true
     api_keys:
       - "rlm-api-key-1"
       - "rlm-api-key-2"
   ```

2. **Rate Limiting:**
   ```yaml
   security:
     rate_limiting:
       requests_per_minute: 60
       burst_size: 10
       per_ip_limit: 100
   ```

3. **CORS Configuration:**
   ```yaml
   security:
     cors:
       allowed_origins:
         - "https://yourdomain.com"
         - "https://app.yourdomain.com"
       allowed_methods: ["GET", "POST", "OPTIONS"]
       allowed_headers: ["Authorization", "Content-Type"]
   ```

### Network Security

1. **TLS/SSL Configuration:**
   - Use TLS 1.2 or higher
   - Strong cipher suites
   - HSTS headers
   - Certificate pinning

2. **Firewall Rules:**
   ```bash
   # UFW configuration
   sudo ufw default deny incoming
   sudo ufw default allow outgoing
   sudo ufw allow ssh
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   sudo ufw enable
   ```

### REPL Security

The Rhai REPL is sandboxed with:
- No file system access
- No network access
- Limited memory allocation
- Execution timeout limits
- No dangerous function access

### Secrets Management

Use environment-specific secret management:

**Docker Secrets:**
```bash
echo "sk-your-api-key" | docker secret create openai_api_key -
```

**Kubernetes Secrets:**
```bash
kubectl create secret generic rlm-secrets \
  --from-literal=openai-api-key=sk-your-api-key
```

**AWS Secrets Manager:**
```bash
aws secretsmanager create-secret \
  --name "rlm/openai-api-key" \
  --secret-string "sk-your-api-key"
```

## Troubleshooting

### Common Issues

1. **Server Won't Start:**
   ```bash
   # Check configuration
   ./rlm-server --config config.yaml --validate

   # Check port availability
   netstat -tulpn | grep 8080

   # Check logs
   journalctl -u rlm-server -f
   ```

2. **High Memory Usage:**
   ```bash
   # Monitor memory
   top -p $(pgrep rlm-server)

   # Adjust REPL limits
   RLM_REPL_MAX_MEMORY=67108864  # 64MB
   ```

3. **Slow Response Times:**
   ```bash
   # Check LLM provider latency
   curl -w "@curl-format.txt" -s -o /dev/null \
     https://api.openai.com/v1/models \
     -H "Authorization: Bearer $OPENAI_API_KEY"

   # Check server metrics
   curl http://localhost:9090/metrics | grep latency
   ```

### Debug Mode

Enable debug logging:

```bash
RLM_LOG_LEVEL=debug ./rlm-server
```

Or via configuration:
```yaml
logging:
  level: "debug"
  modules:
    rlm_core: "trace"
    rlm_server: "debug"
```

### Health Checks

The server provides multiple health check endpoints:

- `/health` - Basic health check
- `/health/ready` - Readiness check (all components initialized)
- `/health/live` - Liveness check (server responsive)
- `/metrics` - Prometheus metrics

### Log Analysis

Common log patterns to monitor:

```bash
# Error patterns
grep "ERROR" /var/log/rlm-server.log

# Performance patterns
grep "slow_request" /var/log/rlm-server.log

# REPL execution issues
grep "repl_timeout" /var/log/rlm-server.log
```

## Maintenance and Updates

### Rolling Updates

1. **Docker Compose:**
   ```bash
   # Build new image
   docker-compose build rlm-server

   # Rolling update
   docker-compose up -d --no-deps rlm-server
   ```

2. **Kubernetes:**
   ```bash
   # Update deployment image
   kubectl set image deployment/rlm-server rlm-server=your-registry/rlm-server:v1.1.0

   # Monitor rollout
   kubectl rollout status deployment/rlm-server
   ```

3. **Systemd Service:**
   ```bash
   # Stop service
   sudo systemctl stop rlm-server

   # Update binary
   sudo cp target/release/rlm-server /opt/rlm-server/bin/

   # Start service
   sudo systemctl start rlm-server
   ```

### Backup and Recovery

1. **Configuration Backup:**
   ```bash
   # Backup configuration
   tar -czf rlm-config-$(date +%Y%m%d).tar.gz \
     config.yaml .env prometheus.yml
   ```

2. **Log Rotation:**
   ```logrotate
   /var/log/rlm-server/*.log {
     daily
     missingok
     rotate 30
     compress
     delaycompress
     notifempty
     postrotate
       systemctl reload rlm-server
     endscript
   }
   ```

### Performance Tuning

1. **System Limits:**
   ```bash
   # /etc/security/limits.conf
   rlm soft nofile 65536
   rlm hard nofile 65536
   rlm soft nproc 32768
   rlm hard nproc 32768
   ```

2. **Kernel Parameters:**
   ```bash
   # /etc/sysctl.conf
   net.core.somaxconn = 65535
   net.ipv4.tcp_tw_reuse = 1
   net.ipv4.tcp_fin_timeout = 30
   vm.swappiness = 10
   ```

3. **Application Tuning:**
   ```yaml
   server:
     worker_threads: 8
     max_concurrent_requests: 1000
     keepalive_timeout: 60
     request_buffer_size: 8192
   ```

## Conclusion

This deployment guide provides comprehensive instructions for deploying the RLM server in various environments, from local development to production cloud deployments. Follow the security best practices and monitoring guidelines to ensure a reliable and secure deployment.

For additional support, consult the project documentation, check the troubleshooting section, or refer to the project's GitHub repository for updates and community support.