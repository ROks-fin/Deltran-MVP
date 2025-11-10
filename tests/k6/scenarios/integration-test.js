// Integration Test for All DelTran MVP Services
// Tests health endpoints and basic connectivity

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate } from 'k6/metrics';
import { SERVICES, HEADERS } from '../config/services.js';

const healthCheckRate = new Rate('health_check_success_rate');

export const options = {
    vus: 1,
    duration: '30s',
    thresholds: {
        'health_check_success_rate': ['rate>0.95'],
        'http_req_duration': ['p(95)<1000'],
        'http_req_failed': ['rate<0.05'],
    },
};

export default function() {
    group('Gateway Health Check', () => {
        const res = http.get(`${SERVICES.gateway.url}${SERVICES.gateway.endpoints.health}`);
        const success = check(res, {
            'Gateway is healthy': (r) => r.status === 200,
            'Gateway has correct status': (r) => {
                try {
                    const body = JSON.parse(r.body);
                    return body.status === 'healthy' || body.status === 'ok';
                } catch(e) {
                    return false;
                }
            },
        });
        healthCheckRate.add(success);
    });

    group('Token Engine Health Check', () => {
        const res = http.get(`${SERVICES.tokenEngine.url}${SERVICES.tokenEngine.endpoints.health}`);
        const success = check(res, {
            'Token Engine is healthy': (r) => r.status === 200,
            'Token Engine service name correct': (r) => {
                try {
                    const body = JSON.parse(r.body);
                    return body.service === 'token-engine';
                } catch(e) {
                    return false;
                }
            },
        });
        healthCheckRate.add(success);
    });

    group('Obligation Engine Health Check', () => {
        const res = http.get(`${SERVICES.obligationEngine.url}${SERVICES.obligationEngine.endpoints.health}`);
        const success = check(res, {
            'Obligation Engine is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Liquidity Router Health Check', () => {
        const res = http.get(`${SERVICES.liquidityRouter.url}${SERVICES.liquidityRouter.endpoints.health}`);
        const success = check(res, {
            'Liquidity Router is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Risk Engine Health Check', () => {
        const res = http.get(`${SERVICES.riskEngine.url}${SERVICES.riskEngine.endpoints.health}`);
        const success = check(res, {
            'Risk Engine is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Clearing Engine Health Check', () => {
        const res = http.get(`${SERVICES.clearingEngine.url}${SERVICES.clearingEngine.endpoints.health}`);
        const success = check(res, {
            'Clearing Engine is healthy': (r) => r.status === 200,
            'Clearing Engine has service name': (r) => {
                try {
                    const body = JSON.parse(r.body);
                    return body.service === 'clearing-engine';
                } catch(e) {
                    return false;
                }
            },
        });
        healthCheckRate.add(success);
    });

    group('Compliance Engine Health Check', () => {
        const res = http.get(`${SERVICES.complianceEngine.url}${SERVICES.complianceEngine.endpoints.health}`);
        const success = check(res, {
            'Compliance Engine is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Reporting Engine Health Check', () => {
        const res = http.get(`${SERVICES.reportingEngine.url}${SERVICES.reportingEngine.endpoints.health}`);
        const success = check(res, {
            'Reporting Engine is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Settlement Engine Health Check', () => {
        const res = http.get(`${SERVICES.settlementEngine.url}${SERVICES.settlementEngine.endpoints.health}`);
        const success = check(res, {
            'Settlement Engine is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Notification Engine Health Check', () => {
        const res = http.get(`${SERVICES.notificationEngine.http}${SERVICES.notificationEngine.endpoints.health}`);
        const success = check(res, {
            'Notification Engine is healthy': (r) => r.status === 200,
        });
        healthCheckRate.add(success);
    });

    group('Prometheus Metrics Endpoints', () => {
        // Test metrics endpoints for Rust services
        const metricsServices = [
            { name: 'Token Engine', url: SERVICES.tokenEngine.url },
            { name: 'Obligation Engine', url: SERVICES.obligationEngine.url },
            { name: 'Clearing Engine', url: SERVICES.clearingEngine.url },
        ];

        metricsServices.forEach(service => {
            const res = http.get(`${service.url}/metrics`);
            check(res, {
                [`${service.name} metrics available`]: (r) => r.status === 200,
                [`${service.name} metrics in Prometheus format`]: (r) =>
                    r.body.includes('# HELP') && r.body.includes('# TYPE'),
            });
        });
    });

    sleep(1);
}

export function handleSummary(data) {
    return {
        'stdout': JSON.stringify(data, null, 2),
        '../results/integration.json': JSON.stringify(data),
    };
}
