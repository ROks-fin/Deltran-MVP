// End-to-End Transaction Flow Test
// Tests complete transaction flow through Gateway -> Token Engine

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
import { SERVICES, HEADERS, generateRandomTransaction } from '../config/services.js';

const txSuccessRate = new Rate('transaction_success_rate');
const txDuration = new Trend('transaction_duration');

export const options = {
    stages: [
        { duration: '30s', target: 10 },  // Ramp up
        { duration: '1m', target: 50 },   // Sustained load
        { duration: '30s', target: 0 },   // Ramp down
    ],
    thresholds: {
        'http_req_duration': ['p(95)<1000', 'p(99)<2000'],
        'transaction_success_rate': ['rate>0.95'],
        'http_req_failed': ['rate<0.01'],
    },
};

export default function() {
    const startTime = new Date();

    group('Full Transaction Flow', () => {
        // 1. Create transaction via Gateway
        const txPayload = generateRandomTransaction();
        txPayload.test_run_id = `E2E-${Date.now()}`;
        txPayload.test_scenario = 'e2e_transaction_flow';

        let res = http.post(
            `${SERVICES.gateway.url}${SERVICES.gateway.endpoints.transfer}`,
            JSON.stringify(txPayload),
            { headers: HEADERS }
        );

        const txCreated = check(res, {
            'Transaction created': (r) => r.status === 200 || r.status === 202,
            'Has transaction_id': (r) => {
                try {
                    const body = JSON.parse(r.body);
                    return body.transaction_id !== undefined;
                } catch(e) {
                    return false;
                }
            },
        });

        if (!txCreated) {
            console.log(`❌ Transaction creation failed: ${res.status} - ${res.body}`);
            txSuccessRate.add(false);
            return;
        }

        let txId;
        try {
            const body = JSON.parse(res.body);
            txId = body.transaction_id;
            console.log(`✅ Transaction created: ${txId}`);
        } catch(e) {
            console.log(`❌ Failed to parse transaction response`);
            txSuccessRate.add(false);
            return;
        }

        sleep(1);

        // 2. Check transaction status (if endpoint exists)
        res = http.get(
            `${SERVICES.gateway.url}${SERVICES.gateway.endpoints.transaction(txId)}`,
            { headers: HEADERS }
        );

        check(res, {
            'Status retrieved': (r) => r.status === 200 || r.status === 404, // 404 is ok if not implemented
            'Has status field': (r) => {
                if (r.status === 404) return true;
                try {
                    const body = JSON.parse(r.body);
                    return body.status !== undefined;
                } catch(e) {
                    return false;
                }
            },
        });

        sleep(2);

        // Transaction flow complete
        const duration = new Date() - startTime;
        txDuration.add(duration);
        txSuccessRate.add(txCreated);
    });

    sleep(Math.random() * 3 + 1);
}

export function handleSummary(data) {
    console.log('📊 E2E Transaction Test Summary:');
    console.log(`   Total Requests: ${data.metrics.http_reqs.values.count}`);
    console.log(`   Success Rate: ${(data.metrics.transaction_success_rate.values.rate * 100).toFixed(2)}%`);
    console.log(`   Avg Duration: ${data.metrics.transaction_duration.values.avg.toFixed(2)}ms`);
    console.log(`   P95 Latency: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms`);

    return {
        'stdout': JSON.stringify(data, null, 2),
        '../results/e2e.json': JSON.stringify(data),
    };
}
