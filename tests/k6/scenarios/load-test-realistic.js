// Realistic Load Test with Various Transaction Scenarios
// Tests Gateway under realistic load patterns

import http from 'k6/http';
import { check, sleep } from 'k6';
import { SharedArray } from 'k6/data';
import { SERVICES, HEADERS } from '../config/services.js';

// Realistic test scenarios
const testCases = new SharedArray('test-cases', function() {
    return [
        { name: 'Small INR-AED', amount: 5000, from: 'INR', to: 'AED', sender: 'ICICI', receiver: 'ENBD' },
        { name: 'Medium INR-AED', amount: 50000, from: 'INR', to: 'AED', sender: 'HDFC', receiver: 'ADCB' },
        { name: 'Large INR-AED', amount: 500000, from: 'INR', to: 'AED', sender: 'AXIS', receiver: 'ENBD' },
        { name: 'Small AED-INR', amount: 1000, from: 'AED', to: 'INR', sender: 'ENBD', receiver: 'ICICI' },
        { name: 'Medium AED-INR', amount: 10000, from: 'AED', to: 'INR', sender: 'ADCB', receiver: 'HDFC' },
        { name: 'Large AED-INR', amount: 100000, from: 'AED', to: 'INR', sender: 'DIB', receiver: 'SBI' },
        { name: 'XL Transaction', amount: 1000000, from: 'INR', to: 'AED', sender: 'SBI', receiver: 'NBAD' },
    ];
});

export const options = {
    scenarios: {
        constant_load: {
            executor: 'constant-arrival-rate',
            rate: 100, // 100 transactions per second
            timeUnit: '1s',
            duration: '5m',
            preAllocatedVUs: 50,
            maxVUs: 200,
        },
    },
    thresholds: {
        'http_req_duration': ['p(95)<500', 'p(99)<1000'],
        'http_req_failed': ['rate<0.05'],
    },
};

export default function() {
    const testCase = testCases[__VU % testCases.length];

    const payload = JSON.stringify({
        sender_bank: testCase.sender,
        receiver_bank: testCase.receiver,
        amount: testCase.amount,
        from_currency: testCase.from,
        to_currency: testCase.to,
        sender_account: `ACC${Math.floor(Math.random() * 1000)}`,
        receiver_account: `ACC${Math.floor(Math.random() * 1000)}`,
        test_run_id: `LOAD-${Date.now()}`,
        test_scenario: testCase.name,
    });

    const res = http.post(
        `${SERVICES.gateway.url}${SERVICES.gateway.endpoints.transfer}`,
        payload,
        {
            headers: HEADERS,
            tags: { scenario: testCase.name }
        }
    );

    check(res, {
        [`${testCase.name} - success`]: (r) => r.status < 400,
        [`${testCase.name} - has tx_id`]: (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.transaction_id !== undefined;
            } catch(e) {
                return false;
            }
        },
    });
}

export function handleSummary(data) {
    console.log('📊 Load Test Summary:');
    console.log(`   Total Requests: ${data.metrics.http_reqs.values.count}`);
    console.log(`   RPS: ${data.metrics.http_reqs.values.rate.toFixed(2)}`);
    console.log(`   Failed Requests: ${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%`);
    console.log(`   P95 Latency: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms`);
    console.log(`   P99 Latency: ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms`);

    return {
        'stdout': JSON.stringify(data, null, 2),
        '../results/load.json': JSON.stringify(data),
    };
}
