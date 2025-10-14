# DelTran Kubernetes Deployment

Infrastructure configuration for DelTran on AWS EKS.

## Architecture

- **AWS EKS**: Kubernetes 1.28+
- **Multi-AZ**: 2 availability zones (us-east-1a, us-east-1b)
- **PostgreSQL**: RDS Aurora Multi-AZ (external)
- **Redis**: ElastiCache or in-cluster deployment
- **NATS JetStream**: 3-node cluster
- **Observability**: Prometheus + Grafana + Jaeger

## Directory Structure

```
k8s/
├── base/                    # Base manifests
│   ├── namespace.yaml
│   ├── gateway-deployment.yaml
│   ├── nats-statefulset.yaml
│   ├── redis-deployment.yaml
│   ├── postgres-rds.yaml   # External RDS
│   ├── ingress.yaml        # AWS ALB
│   ├── monitoring.yaml     # Prometheus/Grafana/Jaeger
│   └── hpa.yaml           # Autoscaling
└── overlays/
    ├── dev/
    ├── staging/
    └── production/         # Production overrides
```

## Prerequisites

1. **AWS CLI** configured with appropriate credentials
2. **kubectl** v1.28+
3. **kustomize** v5.0+
4. **eksctl** for cluster creation
5. **helm** for Prometheus/Grafana operators

## Setup

### 1. Create EKS Cluster

```bash
eksctl create cluster \
  --name deltran-prod \
  --region us-east-1 \
  --zones us-east-1a,us-east-1b \
  --nodegroup-name deltran-nodes \
  --node-type m5.2xlarge \
  --nodes 6 \
  --nodes-min 3 \
  --nodes-max 20 \
  --managed \
  --with-oidc
```

### 2. Install AWS Load Balancer Controller

```bash
helm repo add eks https://aws.github.io/eks-charts
helm install aws-load-balancer-controller eks/aws-load-balancer-controller \
  -n kube-system \
  --set clusterName=deltran-prod \
  --set serviceAccount.create=false \
  --set serviceAccount.name=aws-load-balancer-controller
```

### 3. Install Prometheus Operator

```bash
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install kube-prometheus prometheus-community/kube-prometheus-stack \
  -n monitoring --create-namespace
```

### 4. Create RDS Database

```bash
aws rds create-db-cluster \
  --db-cluster-identifier deltran-prod \
  --engine aurora-postgresql \
  --engine-version 15.4 \
  --master-username deltran \
  --master-user-password <STRONG_PASSWORD> \
  --db-subnet-group-name deltran-db-subnet \
  --vpc-security-group-ids sg-xxxxx \
  --backup-retention-period 7 \
  --preferred-backup-window "03:00-04:00" \
  --preferred-maintenance-window "mon:04:00-mon:05:00" \
  --storage-encrypted \
  --enable-iam-database-authentication \
  --multi-az
```

### 5. Deploy Application

```bash
# Production
kubectl apply -k overlays/production

# Verify deployment
kubectl get pods -n deltran
kubectl get svc -n deltran
kubectl get ingress -n deltran
```

## Configuration

### Secrets

Create secrets before deployment:

```bash
kubectl create secret generic deltran-secrets \
  --from-literal=database-url="postgresql://user:pass@host:5432/db" \
  --from-literal=redis-password="..." \
  --from-literal=nats-password="..." \
  --from-literal=jwt-secret="..." \
  -n deltran
```

Or use AWS Secrets Manager with External Secrets Operator:

```bash
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets external-secrets/external-secrets -n external-secrets-system --create-namespace
```

## Scaling

### Manual Scaling

```bash
kubectl scale deployment gateway --replicas=10 -n deltran
```

### Autoscaling (HPA)

Automatically scales based on:
- CPU: 70% threshold
- Memory: 80% threshold
- Custom: 500 RPS per pod

```bash
kubectl get hpa -n deltran
```

## Monitoring

### Access Grafana

```bash
kubectl port-forward svc/kube-prometheus-grafana 3000:80 -n monitoring
```

Default credentials: `admin / prom-operator`

### Access Jaeger UI

```bash
kubectl port-forward svc/jaeger-query 16686:16686 -n deltran
```

### View Logs

```bash
# Gateway logs
kubectl logs -f deployment/gateway -n deltran

# NATS logs
kubectl logs -f statefulset/nats -n deltran

# All logs
kubectl logs -f -l app=gateway -n deltran --all-containers=true
```

## Performance Targets

- **Throughput**: 5000+ TPS
- **Latency**: p95 < 500ms, p99 < 1s
- **Availability**: 99.9% (Multi-AZ)
- **Error Rate**: < 1%

## Database Configuration

### RDS Multi-AZ Setup

- **Instance Class**: db.r6g.2xlarge (2 instances)
- **Storage**: 500GB GP3
- **IOPS**: 12000
- **Backup**: 7 days retention
- **Encryption**: At-rest + in-transit

### Connection Pooling

Applications use PgBouncer/connection pooling:
- Max connections: 100 per pod
- Idle timeout: 10 minutes
- Connection timeout: 30 seconds

## Disaster Recovery

### Backup Strategy

1. **RDS Automated Backups**: Daily snapshots, 7-day retention
2. **NATS JetStream**: Replicated across 3 nodes
3. **Redis**: No persistence (cache only)

### Multi-Region DR

For DR setup:
1. Enable RDS cross-region replication
2. Deploy secondary cluster in `us-west-2`
3. Configure Route53 health checks and failover

## Security

### Network Policies

All pods have network policies restricting ingress/egress.

### Pod Security

- Non-root containers
- Read-only root filesystem where possible
- No privileged escalation
- Security contexts enforced

### TLS/SSL

- ALB terminates SSL (ACM certificate)
- Internal traffic: mTLS via service mesh (optional)

## Troubleshooting

### Pod not starting

```bash
kubectl describe pod <pod-name> -n deltran
kubectl logs <pod-name> -n deltran --previous
```

### Database connectivity

```bash
kubectl run -it --rm debug --image=postgres:15 --restart=Never -- \
  psql postgresql://user:pass@host:5432/db
```

### NATS connection issues

```bash
kubectl exec -it nats-0 -n deltran -- nats account info
```

## Cost Optimization

- Use Spot instances for non-critical workloads
- Enable Cluster Autoscaler
- Right-size RDS instances based on metrics
- Use S3 lifecycle policies for backups

## Support

For issues, contact DevOps team or check internal documentation.
