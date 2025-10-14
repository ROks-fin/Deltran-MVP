# DelTran B2B Integration Guide

## Overview

DelTran is a B2B payment rail for banks, EMIs, fintech providers, and large marketplaces. This guide covers API integration, authentication, webhooks, and best practices.

---

## Table of Contents

1. [Business Model](#business-model)
2. [Getting Started](#getting-started)
3. [Authentication](#authentication)
4. [API Reference](#api-reference)
5. [Webhooks](#webhooks)
6. [Settlement Windows](#settlement-windows)
7. [Compliance & Risk](#compliance--risk)
8. [Testing](#testing)
9. [Production Checklist](#production-checklist)
10. [Support](#support)

---

## Business Model

**Target Customers:**
- Banks
- EMI/Fintech providers
- Large marketplaces (corporate clients via partners)

**Primary Use Case:**
- Cross-border and domestic interbank payments
- Multilateral netting for liquidity optimization
- Compliance-checked, auditable transactions

**Supported Assets:**
- Fiat currencies: USD, EUR, GBP, NIS, PKR, AED, INR + local
- Tokenized 1:1 after funds received in our accounts

---

## Getting Started

### 1. Onboarding

Contact our partnership team to:
1. Complete KYB (Know Your Business)
2. Sign service agreement
3. Provide bank details and BIC/SWIFT codes
4. Set up sandbox credentials

### 2. Sandbox Access

```
Base URL: https://sandbox-api.deltran.io
Web Dashboard: https://sandbox-web.deltran.io
```

Credentials provided via secure channel.

### 3. SDK & Libraries

```bash
# JavaScript/TypeScript
npm install @deltran/sdk

# Python
pip install deltran-sdk

# Go
go get github.com/deltran/go-sdk
```

---

## Authentication

### API Keys

Generate API keys in the dashboard:

```http
POST /api/auth/keys
Authorization: Bearer <session_token>
```

Response:
```json
{
  "api_key": "dk_live_xxxxxxxx",
  "secret_key": "sk_live_xxxxxxxx"
}
```

### Request Signing

All requests must be signed using HMAC-SHA256:

```typescript
import crypto from 'crypto';

const timestamp = Date.now();
const payload = JSON.stringify(body);
const message = `${timestamp}.${payload}`;
const signature = crypto
  .createHmac('sha256', secretKey)
  .update(message)
  .digest('hex');

fetch('https://api.deltran.io/api/payments', {
  method: 'POST',
  headers: {
    'X-DelTran-Key': apiKey,
    'X-DelTran-Timestamp': timestamp.toString(),
    'X-DelTran-Signature': signature,
    'Content-Type': 'application/json',
  },
  body: payload,
});
```

---

## API Reference

### Create Payment

```http
POST /api/payments
```

**Request:**
```json
{
  "sender": {
    "name": "Acme Corp",
    "account_id": "ACT-12345",
    "bank_bic": "CHASUS33",
    "country": "US"
  },
  "recipient": {
    "name": "Global Services Ltd",
    "account_id": "ACT-67890",
    "bank_bic": "DEUTDEFF",
    "country": "DE"
  },
  "amount": "10000.00",
  "currency": "USD",
  "reference": "Invoice-2024-001",
  "idempotency_key": "uuid-v4-here"
}
```

**Response:**
```json
{
  "payment_id": "pay_xxxxxxxx",
  "status": "pending",
  "created_at": "2025-10-09T12:00:00Z",
  "settlement_window": "2025-10-09T18:00:00Z",
  "compliance_status": "clear",
  "risk_score": 15
}
```

### Get Payment Status

```http
GET /api/payments/{payment_id}
```

**Response:**
```json
{
  "payment_id": "pay_xxxxxxxx",
  "status": "settled",
  "settlement_batch_id": "batch_yyyyyyy",
  "net_amount": "9950.00",
  "fee": "50.00",
  "settled_at": "2025-10-09T18:30:00Z"
}
```

### List Payments

```http
GET /api/payments?limit=100&offset=0&status=settled
```

### Cancel Payment

```http
POST /api/payments/{payment_id}/cancel
```

Only allowed before settlement window closes.

### Get Settlement Batch

```http
GET /api/settlements/{batch_id}
```

**Response:**
```json
{
  "batch_id": "batch_yyyyyyy",
  "window_start": "2025-10-09T12:00:00Z",
  "window_end": "2025-10-09T18:00:00Z",
  "currency": "USD",
  "payment_count": 1243,
  "gross_amount": "45678900.00",
  "net_amount": "12345600.00",
  "netting_efficiency": 0.73,
  "status": "completed",
  "iso20022_files": [
    "DELTRAN-20251009-180000-001.xml"
  ]
}
```

---

## Webhooks

Subscribe to events via dashboard or API:

```http
POST /api/webhooks
{
  "url": "https://your-server.com/webhooks/deltran",
  "events": ["payment.created", "payment.settled", "settlement.completed"]
}
```

### Event Types

- `payment.created` - Payment submitted
- `payment.compliance_checked` - Compliance screening completed
- `payment.risk_assessed` - Risk assessment completed
- `payment.settled` - Payment included in settlement batch
- `payment.failed` - Payment failed
- `settlement.window_opened` - New settlement window opened
- `settlement.window_closed` - Settlement window closed
- `settlement.completed` - Settlement batch finalized

### Webhook Payload

```json
{
  "event": "payment.settled",
  "timestamp": "2025-10-09T18:30:00Z",
  "data": {
    "payment_id": "pay_xxxxxxxx",
    "batch_id": "batch_yyyyyyy",
    "net_amount": "9950.00",
    "status": "settled"
  }
}
```

### Webhook Verification

Verify webhook signatures:

```typescript
const signature = req.headers['x-deltran-signature'];
const payload = JSON.stringify(req.body);
const expectedSignature = crypto
  .createHmac('sha256', webhookSecret)
  .update(payload)
  .digest('hex');

if (signature !== expectedSignature) {
  throw new Error('Invalid webhook signature');
}
```

---

## Settlement Windows

### Default Schedule (Production)

4 windows per day (UTC):
- 00:00 - 06:00
- 06:00 - 12:00
- 12:00 - 18:00
- 18:00 - 00:00

### Pilot Schedule

2 windows per day:
- 06:00 - 18:00
- 18:00 - 06:00

### Timeline

```
12:00 - Payment submission opens
17:45 - Cutoff for window (grace period)
18:00 - Window closes, netting begins
18:15 - ISO 20022 files generated
18:30 - Settlement completed
```

### Ad-Hoc Settlement

For urgent payments, contact ops team to trigger manual settlement.

---

## Compliance & Risk

### Sanctions Screening

All payments screened against:
- OFAC (US)
- EU Sanctions List
- UN Sanctions List
- UK HMT Sanctions

**Blocked payments** trigger immediate notification.

### Velocity Controls

Per account/24h:
- Max transactions: 10
- Max amount: $2,000,000

### Corridor Limits

Per payment:
- Soft limit: $250,000 (warning)
- Hard limit: $1,000,000 (blocked)

Custom limits available for high-volume partners.

### Risk Scoring

Each payment receives risk score (0-100):
- 0-49: Low risk (auto-approve)
- 50-74: Medium risk (flagged)
- 75-100: High risk (manual review)

### FX Risk

Our ML-based FX predictor monitors currency exposure. You'll receive warnings for high-volatility corridors.

---

## Testing

### Sandbox Environment

Use test BIC codes:
- `TESTUS33` - Test US bank
- `TESTDE33` - Test DE bank
- `TESTGB33` - Test UK bank

### Test Scenarios

#### Successful Payment
```json
{
  "sender": {"bank_bic": "TESTUS33"},
  "recipient": {"bank_bic": "TESTDE33"},
  "amount": "1000.00"
}
```

#### Sanctions Block
```json
{
  "sender": {"name": "Sanctioned Entity"},
  "amount": "1000.00"
}
```

#### Velocity Limit
Submit 11 payments within 1 minute to trigger limit.

#### Corridor Limit
```json
{
  "amount": "1500000.00"
}
```

### Mock Webhook Events

Trigger test webhooks from dashboard.

---

## Production Checklist

- [ ] Complete KYB/compliance verification
- [ ] Production API keys generated
- [ ] Webhook endpoints configured and tested
- [ ] Error handling implemented
- [ ] Idempotency keys used for all requests
- [ ] Monitoring/alerting set up
- [ ] IP whitelist configured (if applicable)
- [ ] Disaster recovery plan documented
- [ ] SLA reviewed and accepted
- [ ] Support contact information saved

---

## Performance & SLO

**Service Level Objectives:**
- **Throughput**: 5000+ TPS (pilot: 500-1000 TPS)
- **Latency**: p95 < 500ms, p99 < 1s
- **Availability**: 99.9% (Multi-AZ)
- **Error Rate**: < 1%

**Rate Limits:**
- 1000 requests/minute per API key (adjustable)

---

## Error Handling

### Error Codes

| Code | Description |
|------|-------------|
| `payment_invalid` | Invalid payment parameters |
| `compliance_blocked` | Sanctions match detected |
| `risk_limit_exceeded` | Risk threshold exceeded |
| `velocity_limit_exceeded` | Too many transactions |
| `corridor_limit_exceeded` | Amount exceeds corridor limit |
| `insufficient_balance` | Not enough funds |
| `window_closed` | Settlement window closed |
| `duplicate_payment` | Duplicate idempotency key |

### Retry Logic

Implement exponential backoff:
```typescript
async function retryRequest(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      if (!isRetryable(error)) throw error;
      await sleep(Math.pow(2, i) * 1000);
    }
  }
}
```

---

## Multi-Currency Support

**Supported Currencies:**
- USD, EUR, GBP (major)
- NIS, PKR, AED, INR (regional)
- Additional on request

**FX Conversion:**
- Provided via third-party providers
- Competitive rates
- Transparent fee structure

---

## ISO 20022 Integration

For banks requiring ISO 20022 files:

### Download Files
```http
GET /api/settlements/{batch_id}/iso20022
```

### File Format

Files follow `pacs.008.001.08` standard:
- FIToFICustomerCreditTransfer
- Valid XML with BIC, amounts, references

### SWIFT Integration

ISO 20022 files compatible with SWIFT FileAct or manual upload.

---

## Monitoring & Observability

### Health Check
```http
GET /health
```

### Metrics Endpoint
```http
GET /metrics
```

Prometheus-compatible metrics available.

---

## Support

### Technical Support
- Email: support@deltran.io
- Slack: #deltran-api-support (partners only)
- Phone: +1-XXX-XXX-XXXX (critical issues)

### Documentation
- API Reference: https://docs.deltran.io/api
- SDKs: https://github.com/deltran
- Status Page: https://status.deltran.io

### SLA
- Response time: < 4 hours (business hours)
- Resolution time: < 24 hours (P1), < 72 hours (P2)

---

## Appendix

### BIC Code Requirements

All participating banks must have valid BIC/SWIFT codes (8 or 11 characters).

### Idempotency

Always use unique `idempotency_key` (UUID v4) for payment creation to prevent duplicates.

### Data Retention

Payment data retained for 7 years per regulatory requirements.

### GDPR Compliance

We are GDPR-compliant. Contact DPO for data requests: privacy@deltran.io

---

**Version:** 1.0
**Last Updated:** 2025-10-09
