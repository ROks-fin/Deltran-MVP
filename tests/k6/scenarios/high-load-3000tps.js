// High Load Test - 3000 TPS across all DelTran services
// Tests system capacity and performance under heavy load

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { SERVICES, HEADERS, generateRandomTransaction } from '../config/services.js';

// Custom metrics
const requestRate = new Rate('request_success_rate');
const transactionDuration = new Trend('transaction_duration_ms');
const totalRequests = new Counter('total_requests');
const serviceErrors = new Counter('service_errors');

// Load test configuration for 3000 TPS
export const options = {
    stages: [
        { duration: '30s', target: 100 },   // Ramp up to 100 VUs
        { duration: '30s', target: 300 },   // Ramp up to 300 VUs
        { duration: '1m', target: 500 },    // Ramp up to 500 VUs (targeting ~3000 TPS)
        { duration: '2m', target: 500 },    // Sustain 500 VUs for 2 minutes
        { duration: '30s', target: 0 },     // Ramp down
    ],
    thresholds: {
        'http_req_duration': ['p(95)<2000', 'p(99)<5000'], // 95% under 2s, 99% under 5s
        'http_req_failed': ['rate<0.05'],                   // Less than 5% failures
        'request_success_rate': ['rate>0.95'],              // More than 95% success
    },
};

export default function () {
    const startTime = Date.now();

    // Distribute load across all services
    group('Health Checks (10% of traffic)', () => {
        if (Math.random() < 0.1) {
            const services = [
                SERVICES.gateway,
                SERVICES.tokenEngine,
                SERVICES.obligationEngine,
                SERVICES.liquidityRouter,
                SERVICES.riskEngine,
                SERVICES.clearingEngine,
                SERVICES.complianceEngine,
                SERVICES.settlementEngine,
                SERVICES.reportingEngine,
                SERVICES.notificationEngine,
            ];

            const service = services[Math.floor(Math.random() * services.length)];
            const res = http.get(`${service.url}/health`);
            const success = check(res, {
                'Health check successful': (r) => r.status === 200,
            });
            requestRate.add(success);
            totalRequests.add(1);
            if (!success) serviceErrors.add(1);
        }
    });

    group('Token Operations (30% of traffic)', () => {
        if (Math.random() < 0.3) {
            const operations = ['mint', 'transfer', 'burn'];
            const operation = operations[Math.floor(Math.random() * operations.length)];

            const payload = {
                bank_id: `bank-${Math.floor(Math.random() * 100)}`,
                currency: Math.random() > 0.5 ? 'INR' : 'AED',
                amount: Math.floor(Math.random() * 1000000) + 1000,
            };

            if (operation === 'transfer') {
                payload.to_bank_id = `bank-${Math.floor(Math.random() * 100)}`;
            }

            const res = http.post(
                `${SERVICES.tokenEngine.url}/api/v1/tokens/${operation}`,
                JSON.stringify(payload),
                { headers: HEADERS }
            );

            const success = check(res, {
                [`Token ${operation} successful`]: (r) => r.status === 200,
            });
            requestRate.add(success);
            totalRequests.add(1);
            if (!success) serviceErrors.add(1);
        }
    });

    group('Obligation Operations (25% of traffic)', () => {
        if (Math.random() < 0.25) {
            const payload = {
                from_bank_id: `bank-${Math.floor(Math.random() * 50)}`,
                to_bank_id: `bank-${Math.floor(Math.random() * 50) + 50}`,
                currency: Math.random() > 0.5 ? 'INR' : 'AED',
                amount: Math.floor(Math.random() * 500000) + 5000,
            };

            const res = http.post(
                `${SERVICES.obligationEngine.url}/api/v1/obligations/create`,
                JSON.stringify(payload),
                { headers: HEADERS }
            );

            const success = check(res, {
                'Obligation created': (r) => r.status === 200,
            });
            requestRate.add(success);
            totalRequests.add(1);
            if (!success) serviceErrors.add(1);
        }
    });

    group('Liquidity Predictions (15% of traffic)', () => {
        if (Math.random() < 0.15) {
            const payload = {
                corridor: `${Math.random() > 0.5 ? 'INR' : 'AED'}-${Math.random() > 0.5 ? 'USD' : 'EUR'}`,
                amount: Math.floor(Math.random() * 1000000) + 10000,
            };

            const res = http.post(
                `${SERVICES.liquidityRouter.url}/api/v1/liquidity/predict`,
                JSON.stringify(payload),
                { headers: HEADERS }
            );

            const success = check(res, {
                'Liquidity prediction received': (r) => r.status === 200,
            });
            requestRate.add(success);
            totalRequests.add(1);
            if (!success) serviceErrors.add(1);
        }
    });

    group('Clearing Window Queries (10% of traffic)', () => {
        if (Math.random() < 0.1) {
            const res = http.get(`${SERVICES.clearingEngine.url}/api/v1/clearing/windows/current`);

            const success = check(res, {
                'Current window retrieved': (r) => r.status === 200,
            });
            requestRate.add(success);
            totalRequests.add(1);
            if (!success) serviceErrors.add(1);
        }
    });

    group('Metrics Endpoints (10% of traffic)', () => {
        if (Math.random() < 0.1) {
            const services = [
                SERVICES.tokenEngine,
                SERVICES.obligationEngine,
                SERVICES.clearingEngine,
            ];

            const service = services[Math.floor(Math.random() * services.length)];
            const res = http.get(`${service.url}/metrics`);

            const success = check(res, {
                'Metrics retrieved': (r) => r.status === 200 && r.body.includes('# HELP'),
            });
            requestRate.add(success);
            totalRequests.add(1);
            if (!success) serviceErrors.add(1);
        }
    });

    // Record transaction duration
    const duration = Date.now() - startTime;
    transactionDuration.add(duration);

    // Small sleep to control rate (adjust based on VU count)
    sleep(Math.random() * 0.1);
}

export function handleSummary(data) {
    const requestsPerSecond = data.metrics.http_reqs.values.rate;
    const successRate = data.metrics.request_success_rate.values.rate * 100;
    const avgDuration = data.metrics.transaction_duration_ms.values.avg;
    const p95Duration = data.metrics.http_req_duration.values['p(95)'];
    const p99Duration = data.metrics.http_req_duration.values['p(99)'];
    const totalReqs = data.metrics.total_requests.values.count;
    const errors = data.metrics.service_errors.values.count || 0;

    console.log('\n📊 High Load Test Results (3000 TPS Target):');
    console.log('═══════════════════════════════════════════════════');
    console.log(`   Total Requests: ${totalReqs}`);
    console.log(`   Requests/sec: ${requestsPerSecond.toFixed(2)} TPS`);
    console.log(`   Success Rate: ${successRate.toFixed(2)}%`);
    console.log(`   Failed Requests: ${errors}`);
    console.log(`   Avg Duration: ${avgDuration.toFixed(2)}ms`);
    console.log(`   P95 Latency: ${p95Duration.toFixed(2)}ms`);
    console.log(`   P99 Latency: ${p99Duration.toFixed(2)}ms`);
    console.log('═══════════════════════════════════════════════════\n');

    return {
        'stdout': JSON.stringify(data, null, 2),
        '../results/high-load-3000tps.json': JSON.stringify(data),
    };
}
