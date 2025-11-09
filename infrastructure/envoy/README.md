# Envoy Proxy Configuration

## Overview
Envoy serves as the edge proxy for DelTran, providing mTLS termination, rate limiting, circuit breaking, and intelligent routing to backend services.

## Features

### Security
- **mTLS Termination**: Mutual TLS for secure client authentication
- **TLS 1.3**: Modern encryption standards
- **Certificate Management**: Automated certificate rotation
- **RBAC**: Role-based access control at the edge

### Traffic Management
- **Rate Limiting**: Per-client and per-endpoint limits
- **Circuit Breakers**: Automatic failure detection and recovery
- **Retry Logic**: Intelligent retry policies for transient failures
- **Timeout Management**: Request and response timeouts
- **Load Balancing**: Round-robin distribution across backend instances

### Observability
- **Access Logs**: Detailed request/response logging
- **Metrics**: Prometheus-compatible metrics export
- **Tracing**: Distributed tracing support (ready for Jaeger/Zipkin)
- **Health Checks**: Automated upstream health monitoring

## Architecture

```
External Clients
        |
        v
  Envoy Proxy (Edge)
   :8080 (HTTP)
   :8443 (HTTPS)
   :9901 (Admin)
        |
        +-- Gateway Service (:8080)
        |
        +-- Notification Engine (:8089) - WebSocket
        |
        +-- Prometheus (:9090) - Metrics
```

## Configuration

### Listeners

#### HTTPS Listener (:8443)
- **Purpose**: Secure external API access
- **Features**: mTLS, rate limiting, circuit breakers
- **Routes**:
  - `/api/v1/*` → Gateway (1000 req/min per client)
  - `/health` → Gateway health check
  - `/ws` → Notification Engine (WebSocket)
  - `/admin` → Admin panel (100 req/min)

#### HTTP Listener (:8080)
- **Purpose**: Internal/development access (MVP)
- **Routes**: Direct routing to gateway
- **Production**: Should redirect to HTTPS

### Clusters

#### Gateway Cluster
- **Load Balancing**: Round-robin
- **Health Check**: HTTP on `/health` every 10s
- **Circuit Breaker**:
  - Max connections: 1000
  - Max pending requests: 1000
  - Max retries: 3
  - Outlier detection: 5 consecutive 5xx errors
- **Timeouts**: 30s request, 5s connect

#### Notification Cluster
- **Purpose**: WebSocket connections
- **Special**: No timeout for persistent connections
- **Health Check**: HTTP on `/health` every 10s

### Rate Limiting

#### Global Rate Limits
- **Default**: 10,000 requests per minute
- **Enforcement**: 100% (all requests checked)
- **Response Headers**: `x-rate-limit-status`

#### Per-Route Limits
- **API Routes**: 1000 req/min per client
- **Admin Routes**: 100 req/min per client

#### Configuration
Rate limits are enforced using token bucket algorithm:
```yaml
token_bucket:
  max_tokens: 1000        # Burst capacity
  tokens_per_fill: 1000   # Refill amount
  fill_interval: 60s      # Refill interval
```

### Circuit Breakers

#### Configuration
```yaml
Priority: DEFAULT
- Max connections: 1000
- Max pending requests: 1000
- Max requests: 1000
- Max retries: 3

Priority: HIGH
- Max connections: 2000
- Max pending requests: 2000
- Max requests: 2000
- Max retries: 5
```

#### Outlier Detection
- **Consecutive 5xx**: 5 errors trigger ejection
- **Ejection Time**: 30 seconds base
- **Max Ejection**: 50% of hosts
- **Success Rate**: < 50% triggers ejection

### Retry Policies

Default retry policy for API routes:
```yaml
retry_on: "5xx,reset,connect-failure,refused-stream"
num_retries: 3
per_try_timeout: 10s
retry_host_predicate: previous_hosts  # Don't retry on same host
```

## Setup Instructions

### 1. Generate TLS Certificates

For MVP (self-signed):
```bash
cd infrastructure/envoy
./generate-certs.sh
```

For Production (Let's Encrypt/Commercial CA):
```bash
# Use cert-manager or ACME client
# Place certificates in:
# - /etc/envoy/certs/server-cert.pem
# - /etc/envoy/certs/server-key.pem
# - /etc/envoy/certs/ca-cert.pem
```

### 2. Start Envoy

Using Docker Compose:
```bash
docker-compose up -d envoy
```

Standalone:
```bash
docker run -d \
  --name deltran-envoy \
  -p 8080:8080 \
  -p 8443:8443 \
  -p 9901:9901 \
  -v $(pwd)/envoy.yaml:/etc/envoy/envoy.yaml \
  -v $(pwd)/certs:/etc/envoy/certs \
  envoyproxy/envoy:v1.28-latest
```

### 3. Verify Configuration

```bash
# Check admin interface
curl http://localhost:9901/

# View configuration dump
curl http://localhost:9901/config_dump

# Check cluster health
curl http://localhost:9901/clusters

# View stats
curl http://localhost:9901/stats
```

### 4. Test Routing

```bash
# Test HTTP routing
curl http://localhost:8080/health

# Test HTTPS routing (with certificates)
curl --cacert certs/ca-cert.pem \
     --cert certs/client-cert.pem \
     --key certs/client-key.pem \
     https://localhost:8443/api/v1/health

# Test rate limiting
for i in {1..1100}; do
  curl http://localhost:8080/api/v1/test
done
# Should see 429 after 1000 requests
```

## Monitoring

### Admin Interface
- **URL**: http://localhost:9901
- **Endpoints**:
  - `/`: Admin home
  - `/stats`: Real-time statistics
  - `/clusters`: Cluster status
  - `/config_dump`: Full configuration
  - `/runtime`: Runtime configuration
  - `/listeners`: Listener details

### Key Metrics

#### Request Metrics
```
envoy_http_downstream_rq_total
envoy_http_downstream_rq_xx (2xx, 3xx, 4xx, 5xx)
envoy_http_downstream_rq_time
envoy_http_downstream_cx_total
```

#### Cluster Metrics
```
envoy_cluster_upstream_cx_total
envoy_cluster_upstream_rq_total
envoy_cluster_upstream_rq_time
envoy_cluster_health_check_success
envoy_cluster_health_check_failure
```

#### Rate Limit Metrics
```
envoy_http_local_rate_limit_enabled
envoy_http_local_rate_limit_enforced
envoy_http_local_rate_limit_rate_limited
```

#### Circuit Breaker Metrics
```
envoy_cluster_circuit_breakers_default_cx_open
envoy_cluster_circuit_breakers_default_rq_pending_open
envoy_cluster_outlier_detection_ejections_active
```

### Prometheus Integration

Envoy exports metrics in Prometheus format:
```bash
curl http://localhost:9901/stats/prometheus
```

Add to Prometheus scrape config:
```yaml
scrape_configs:
  - job_name: 'envoy'
    static_configs:
      - targets: ['envoy:9901']
```

### Grafana Dashboards

Use official Envoy dashboards:
- **Envoy Global**: Dashboard ID 11022
- **Envoy Clusters**: Dashboard ID 11021

## Security Hardening

### Production Checklist

- [ ] Replace self-signed certificates with CA-signed certificates
- [ ] Enable mTLS for client authentication
- [ ] Restrict admin interface to internal network only
- [ ] Configure RBAC policies for fine-grained access control
- [ ] Enable request ID tracking for audit trails
- [ ] Set up WAF (Web Application Firewall) rules
- [ ] Configure DDoS protection
- [ ] Enable HTTP/2 and TLS 1.3 only
- [ ] Set up certificate rotation automation
- [ ] Configure external authorization service

### Client Certificate Authentication

For mTLS, clients must present valid certificates:
```yaml
validation_context:
  trusted_ca:
    filename: /etc/envoy/certs/ca-cert.pem
  verify_certificate_hash:
  - "sha256_hash_of_client_cert"
```

### RBAC Example

Restrict admin endpoints to specific IPs:
```yaml
envoy.filters.http.rbac:
  rules:
    action: ALLOW
    policies:
      "admin-access":
        permissions:
        - header:
            name: ":path"
            prefix_match: "/admin"
        principals:
        - remote_ip:
            address_prefix: "10.0.0.0"
            prefix_len: 8
```

## Troubleshooting

### High Latency
1. Check upstream health: `curl localhost:9901/clusters`
2. Review outlier detection: Look for ejected hosts
3. Check circuit breaker status
4. Review retry policies (may be amplifying issues)

### Rate Limit Issues
1. Check current limits: `curl localhost:9901/config_dump | grep token_bucket`
2. View rate limit metrics: `curl localhost:9901/stats | grep rate_limit`
3. Adjust token bucket settings if needed

### Circuit Breaker Triggered
1. Check cluster health: `curl localhost:9901/clusters | grep gateway_cluster`
2. Review upstream errors: Look for 5xx responses
3. Check connection limits
4. Verify upstream service is healthy

### Connection Issues
1. Verify listener configuration: `curl localhost:9901/listeners`
2. Check certificate validity
3. Review TLS configuration
4. Ensure upstream services are reachable

### Configuration Errors
```bash
# Validate configuration
envoy --mode validate -c envoy.yaml

# View error logs
docker logs deltran-envoy

# Check admin interface for errors
curl localhost:9901/config_dump?include_eds
```

## Performance Tuning

### For High Throughput
```yaml
# Increase connection limits
circuit_breakers:
  thresholds:
  - priority: DEFAULT
    max_connections: 10000
    max_pending_requests: 10000
    max_requests: 10000

# Increase buffer sizes
per_connection_buffer_limit_bytes: 1048576  # 1MB

# Enable HTTP/2
typed_extension_protocol_options:
  explicit_http_config:
    http2_protocol_options:
      max_concurrent_streams: 1000
```

### For Low Latency
```yaml
# Reduce timeouts
connect_timeout: 1s

# Disable retries for non-idempotent requests
retry_policy:
  num_retries: 0

# Use shortest timeout possible
timeout: 5s
```

## Upgrading

### Rolling Update Process
1. Deploy new Envoy version alongside current
2. Update load balancer to route 10% traffic to new version
3. Monitor metrics for errors
4. Gradually increase traffic to new version
5. Deprecate old version once stable

### Configuration Hot Reload
Envoy supports hot reload without dropping connections:
```bash
# Send SIGHUP to reload config
docker exec deltran-envoy kill -HUP 1

# Or use admin API
curl -X POST http://localhost:9901/quitquitquit
```

## References

- [Envoy Documentation](https://www.envoyproxy.io/docs)
- [Circuit Breaker Guide](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/circuit_breaking)
- [Rate Limiting Guide](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/local_rate_limit_filter)
- [Security Best Practices](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/security/ssl)
