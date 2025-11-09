// k6 load test for DelTran MVP
// Run: k6 run load_test.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const transactionDuration = new Trend('transaction_duration');

// Test configuration
export const options = {
  stages: [
    { duration: '1m', target: 50 },   // Ramp up to 50 VUs
    { duration: '3m', target: 100 },  // Stay at 100 VUs (100+ TPS target)
    { duration: '1m', target: 200 },  // Spike to 200 VUs
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500'],     // 95% of requests under 500ms
    'http_req_failed': ['rate<0.1'],        // Error rate under 10%
    'errors': ['rate<0.1'],
  },
};

// Test data generator
function generateTransfer() {
  const timestamp = Date.now();
  return {
    sender_bank: 'BANK001',
    receiver_bank: `BANK${String(Math.floor(Math.random() * 10)).padStart(3, '0')}`,
    amount: Math.floor(Math.random() * 10000) + 100,
    from_currency: 'USD',
    to_currency: 'USD',
    reference: `LOAD-TEST-${timestamp}-${__VU}-${__ITER}`,
    idempotency_key: `load-${timestamp}-${__VU}-${__ITER}`,
  };
}

export default function () {
  const transfer = generateTransfer();

  const payload = JSON.stringify(transfer);
  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
    timeout: '30s',
  };

  const startTime = Date.now();

  // Submit transfer
  const res = http.post('http://localhost:8080/api/v1/transfer', payload, params);

  const duration = Date.now() - startTime;
  transactionDuration.add(duration);

  // Validate response
  const success = check(res, {
    'status is 200 or 201': (r) => r.status === 200 || r.status === 201,
    'has transaction_id': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.transaction_id !== undefined;
      } catch (e) {
        return false;
      }
    },
    'response time < 1s': (r) => r.timings.duration < 1000,
  });

  if (!success) {
    errorRate.add(1);
  } else {
    errorRate.add(0);
  }

  // Random think time
  sleep(Math.random() * 2);
}

// Summary handler
export function handleSummary(data) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const reportPath = `./tests/reports/load_test_${timestamp}.json`;

  return {
    [reportPath]: JSON.stringify(data, null, 2),
    stdout: textSummary(data, { indent: ' ', enableColors: true }),
  };
}

function textSummary(data, options) {
  const indent = options.indent || '';
  const enableColors = options.enableColors || false;

  let summary = `
${indent}Load Test Summary
${indent}================

${indent}Checks:
${indent}  ✓ ${data.metrics.checks.values.passes} passed
${indent}  ✗ ${data.metrics.checks.values.fails} failed

${indent}HTTP Requests:
${indent}  Total: ${data.metrics.http_reqs.values.count}
${indent}  Rate: ${data.metrics.http_reqs.values.rate.toFixed(2)} req/s
${indent}  Failed: ${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%

${indent}Response Times:
${indent}  Min: ${data.metrics.http_req_duration.values.min.toFixed(2)}ms
${indent}  Avg: ${data.metrics.http_req_duration.values.avg.toFixed(2)}ms
${indent}  Med: ${data.metrics.http_req_duration.values.med.toFixed(2)}ms
${indent}  P95: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms
${indent}  Max: ${data.metrics.http_req_duration.values.max.toFixed(2)}ms

${indent}Virtual Users:
${indent}  Min: ${data.metrics.vus.values.min}
${indent}  Max: ${data.metrics.vus.values.max}
`;

  return summary;
}
