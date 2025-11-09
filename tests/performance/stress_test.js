// k6 stress test for DelTran MVP - 500 TPS target
// Run: k6 run stress_test.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Counter } from 'k6/metrics';

const errorRate = new Rate('errors');
const successCounter = new Counter('successful_transactions');

export const options = {
  stages: [
    { duration: '30s', target: 100 },  // Warm up
    { duration: '1m', target: 500 },   // Stress to 500 VUs (500+ TPS)
    { duration: '30s', target: 0 },    // Cool down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<2000'],  // More lenient for stress test
    'errors': ['rate<0.2'],                // Allow 20% errors under stress
  },
};

function generateTransfer() {
  const timestamp = Date.now();
  return {
    sender_bank: 'BANK001',
    receiver_bank: `BANK${String(Math.floor(Math.random() * 20)).padStart(3, '0')}`,
    amount: Math.floor(Math.random() * 5000) + 50,
    from_currency: 'USD',
    to_currency: 'USD',
    reference: `STRESS-TEST-${timestamp}-${__VU}`,
    idempotency_key: `stress-${timestamp}-${__VU}-${__ITER}`,
  };
}

export default function () {
  const transfer = generateTransfer();
  const payload = JSON.stringify(transfer);

  const params = {
    headers: { 'Content-Type': 'application/json' },
    timeout: '10s',
  };

  const res = http.post('http://localhost:8080/api/v1/transfer', payload, params);

  const success = check(res, {
    'status is 2xx or 429': (r) => r.status >= 200 && r.status < 300 || r.status === 429,
  });

  if (success && res.status < 300) {
    successCounter.add(1);
    errorRate.add(0);
  } else {
    errorRate.add(1);
  }

  sleep(0.1); // Minimal sleep for maximum throughput
}
