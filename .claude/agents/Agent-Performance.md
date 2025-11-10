# Agent-Performance

## Роль
Агент для создания K6 performance тестов для СУЩЕСТВУЮЩИХ сервисов DelTran MVP. Тестирует реальные endpoints на портах 8080-8093. Использует Context7 для актуальных K6 patterns.

## Контекст
DelTran MVP имеет **11 ГОТОВЫХ сервисов** на портах:
- **8080**: Gateway (enhanced с JWT, rate limiting)
- **8081**: Token Engine
- **8082**: Obligation Engine
- **8083**: Liquidity Router
- **8084**: Risk Engine
- **8085**: Clearing Engine
- **8086**: Compliance Engine
- **8087**: Reporting Engine
- **8088**: Settlement Engine
- **8089**: Notification Engine (HTTP)
- **8090**: Notification Engine (WebSocket)
- **8093**: Analytics Collector (Python FastAPI)

## Задачи

### 🔍 ПЕРВЫЙ ШАГ: Сканирование существующих endpoints

**ОБЯЗАТЕЛЬНО:**

```bash
# 1. Проверить какие endpoints уже работают
curl http://localhost:8080/health
curl http://localhost:8081/health
curl http://localhost:8082/health
# ... для всех портов

# 2. Получить структуру API Gateway
curl http://localhost:8080/api/v1/banks
curl http://localhost:8080/api/v1/corridors

# 3. Проверить существующие тесты
ls tests/k6/
find . -name "*test*.js" -o -name "*k6*.js"
```

### 1. Использование Context7 для K6 patterns

```bash
# Получить актуальную документацию K6
context7 resolve k6
context7 docs k6 "load testing examples"
context7 docs k6 "websocket testing"
context7 docs k6 "thresholds configuration"
context7 docs k6 "custom metrics"
```

### 2. Base Configuration для СУЩЕСТВУЮЩИХ сервисов

```javascript
// tests/k6/config/services.js

export const SERVICES = {
    gateway: {
        url: 'http://localhost:8080',
        endpoints: {
            health: '/health',
            banks: '/api/v1/banks',
            corridors: '/api/v1/corridors',
            transfer: '/api/v1/transfer',
            transaction: (id) => `/api/v1/transaction/${id}`,
        }
    },
    tokenEngine: {
        url: 'http://localhost:8081',
        endpoints: {
            health: '/health',
            tokens: '/tokens',
            mint: '/tokens/mint',
            burn: '/tokens/burn',
        }
    },
    obligationEngine: {
        url: 'http://localhost:8082',
        endpoints: {
            health: '/health',
            obligations: '/obligations',
            create: '/obligations/create',
            netting: '/obligations/netting',
        }
    },
    clearingEngine: {
        url: 'http://localhost:8085',
        endpoints: {
            health: '/health',
            windows: '/windows',
            metrics: '/metrics',
        }
    },
    settlementEngine: {
        url: 'http://localhost:8088',
        endpoints: {
            health: '/health',
            settlements: '/settlements',
            status: (id) => `/settlements/${id}/status`,
        }
    },
    notificationEngine: {
        http: 'http://localhost:8089',
        ws: 'ws://localhost:8090',
        endpoints: {
            health: '/health',
            notifications: '/notifications',
            send: '/notifications/send',
        }
    },
    analyticsCollector: {
        url: 'http://localhost:8093',
        endpoints: {
            health: '/health',
            transactions: '/transactions',
            metrics: '/metrics/dashboard',
            events: '/events/transaction',
        }
    },
};

// Common headers
export const HEADERS = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
};
```

### 3. Integration Test для ВСЕХ сервисов

```javascript
// tests/k6/scenarios/integration-test.js

import http from 'k6/http';
import { check, group } from 'k6';
import { SERVICES, HEADERS } from '../config/services.js';

export let options = {
    vus: 1,
    duration: '30s',
};

export default function() {
    group('All Services Health Check', () => {
        // Test Gateway
        let res = http.get(`${SERVICES.gateway.url}/health`);
        check(res, {
            'Gateway is healthy': (r) => r.status === 200 && r.json('status') === 'healthy',
        });

        // Test Token Engine
        res = http.get(`${SERVICES.tokenEngine.url}/health`);
        check(res, {
            'Token Engine is healthy': (r) => r.status === 200,
        });

        // Test Obligation Engine
        res = http.get(`${SERVICES.obligationEngine.url}/health`);
        check(res, {
            'Obligation Engine is healthy': (r) => r.status === 200,
        });

        // Test Clearing Engine
        res = http.get(`${SERVICES.clearingEngine.url}/health`);
        check(res, {
            'Clearing Engine is healthy': (r) => r.status === 200,
        });

        // Test Settlement Engine
        res = http.get(`${SERVICES.settlementEngine.url}/health`);
        check(res, {
            'Settlement Engine is healthy': (r) => r.status === 200,
        });

        // Test Notification Engine
        res = http.get(`${SERVICES.notificationEngine.http}/health`);
        check(res, {
            'Notification Engine is healthy': (r) => r.status === 200,
        });

        // Test Analytics Collector
        res = http.get(`${SERVICES.analyticsCollector.url}/health`);
        check(res, {
            'Analytics Collector is healthy': (r) => r.status === 200,
        });
    });
}
```

### 4. End-to-End Transaction Flow Test

```javascript
// tests/k6/scenarios/e2e-transaction.js

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
import { SERVICES, HEADERS } from '../config/services.js';

const txSuccessRate = new Rate('transaction_success_rate');
const txDuration = new Trend('transaction_duration');

export let options = {
    stages: [
        { duration: '30s', target: 10 },
        { duration: '1m', target: 50 },
        { duration: '30s', target: 0 },
    ],
    thresholds: {
        'http_req_duration': ['p(95)<1000', 'p(99)<2000'],
        'transaction_success_rate': ['rate>0.95'],
    },
};

export default function() {
    const startTime = new Date();

    group('Full Transaction Flow', () => {
        // 1. Create transaction via Gateway
        const txPayload = JSON.stringify({
            sender_bank: 'ICICI',
            receiver_bank: 'ENBD',
            amount: Math.floor(Math.random() * 10000) + 1000,
            from_currency: 'INR',
            to_currency: 'AED',
            sender_account: `ACC${Math.floor(Math.random() * 100)}`,
            receiver_account: `ACC${Math.floor(Math.random() * 100)}`,
        });

        let res = http.post(
            `${SERVICES.gateway.url}/api/v1/transfer`,
            txPayload,
            { headers: HEADERS }
        );

        const txCreated = check(res, {
            'transaction created': (r) => r.status === 200 || r.status === 202,
            'has transaction_id': (r) => r.json('transaction_id') !== undefined,
        });

        if (!txCreated) {
            txSuccessRate.add(false);
            return;
        }

        const txId = res.json('transaction_id');
        console.log(`Transaction created: ${txId}`);

        sleep(1);

        // 2. Check transaction status
        res = http.get(
            SERVICES.gateway.url + SERVICES.gateway.endpoints.transaction(txId),
            { headers: HEADERS }
        );

        check(res, {
            'status retrieved': (r) => r.status === 200,
            'has status field': (r) => r.json('status') !== undefined,
        });

        sleep(2);

        // 3. Verify in Analytics
        res = http.get(
            `${SERVICES.analyticsCollector.url}/transactions?transaction_id=${txId}`,
            { headers: HEADERS }
        );

        const success = check(res, {
            'transaction logged in analytics': (r) => r.status === 200,
            'analytics has data': (r) => r.json('transactions') && r.json('transactions').length > 0,
        });

        const duration = new Date() - startTime;
        txDuration.add(duration);
        txSuccessRate.add(success);
    });

    sleep(Math.random() * 3 + 1);
}
```

### 5. Load Test с реальными данными

```javascript
// tests/k6/scenarios/load-test-realistic.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { SharedArray } from 'k6/data';
import { SERVICES, HEADERS } from '../config/services.js';

// Realistic test data
const testCases = new SharedArray('test-cases', function() {
    return [
        { name: 'Small INR-AED', amount: 5000, from: 'INR', to: 'AED', sender: 'ICICI', receiver: 'ENBD' },
        { name: 'Medium INR-AED', amount: 50000, from: 'INR', to: 'AED', sender: 'HDFC', receiver: 'ADCB' },
        { name: 'Large INR-AED', amount: 500000, from: 'INR', to: 'AED', sender: 'AXIS', receiver: 'ENBD' },
        { name: 'Small AED-INR', amount: 1000, from: 'AED', to: 'INR', sender: 'ENBD', receiver: 'ICICI' },
        { name: 'Medium AED-INR', amount: 10000, from: 'AED', to: 'INR', sender: 'ADCB', receiver: 'HDFC' },
    ];
});

export let options = {
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
        `${SERVICES.gateway.url}/api/v1/transfer`,
        payload,
        { headers: HEADERS, tags: { scenario: testCase.name } }
    );

    check(res, {
        [`${testCase.name} - success`]: (r) => r.status < 400,
        [`${testCase.name} - has tx_id`]: (r) => r.json('transaction_id') !== undefined,
    });
}
```

### 6. WebSocket Test для Notification Engine

```javascript
// tests/k6/scenarios/websocket-test.js

import ws from 'k6/ws';
import { check } from 'k6';
import { Counter } from 'k6/metrics';
import { SERVICES } from '../config/services.js';

const wsConnections = new Counter('ws_connections');
const wsMessages = new Counter('ws_messages_received');

export let options = {
    vus: 20,
    duration: '2m',
};

export default function() {
    const url = SERVICES.notificationEngine.ws;
    const params = { tags: { name: 'NotificationWebSocket' } };

    const res = ws.connect(url, params, function(socket) {
        socket.on('open', () => {
            console.log('WebSocket connected');
            wsConnections.add(1);

            // Subscribe to channels
            socket.send(JSON.stringify({
                type: 'subscribe',
                channels: ['transactions', 'settlements', 'notifications'],
            }));

            socket.setInterval(() => {
                socket.ping();
            }, 10000);
        });

        socket.on('message', (data) => {
            wsMessages.add(1);
            const msg = JSON.parse(data);

            check(msg, {
                'message has type': (m) => m.type !== undefined,
                'message has data': (m) => m.data !== undefined,
            });
        });

        socket.on('error', (e) => {
            console.error('WebSocket error:', e.error());
        });

        socket.setTimeout(() => {
            socket.close();
        }, 30000);
    });

    check(res, {
        'WebSocket connected successfully': (r) => r && r.status === 101,
    });
}
```

### 7. Stress Test - найти breaking point

```javascript
// tests/k6/scenarios/stress-test.js

import http from 'k6/http';
import { check } from 'k6';
import { Rate } from 'k6/metrics';
import { SERVICES, HEADERS } from '../config/services.js';

const errorRate = new Rate('errors');

export let options = {
    stages: [
        { duration: '1m', target: 100 },
        { duration: '2m', target: 200 },
        { duration: '2m', target: 300 },
        { duration: '2m', target: 400 },
        { duration: '2m', target: 500 },  // Find breaking point
        { duration: '3m', target: 0 },
    ],
};

export default function() {
    const res = http.post(
        `${SERVICES.gateway.url}/api/v1/transfer`,
        JSON.stringify({
            sender_bank: 'ICICI',
            receiver_bank: 'ENBD',
            amount: 1000,
            from_currency: 'INR',
            to_currency: 'AED',
        }),
        { headers: HEADERS }
    );

    const success = check(res, {
        'request successful': (r) => r.status < 400,
    });

    errorRate.add(!success);
}
```

### 8. Test Runner Script

```bash
#!/bin/bash
# tests/k6/run-all-tests.sh

echo "=== DelTran MVP K6 Performance Tests ==="
echo "Testing EXISTING services on ports 8080-8093"
echo ""

# Check all services are running
echo "1. Checking services health..."
for port in 8080 8081 8082 8083 8084 8085 8086 8087 8088 8089 8093; do
    if curl -s "http://localhost:$port/health" > /dev/null; then
        echo "✅ Port $port is healthy"
    else
        echo "❌ Port $port is NOT responding"
    fi
done

echo ""
echo "2. Running Integration Test..."
k6 run --out json=results/integration.json scenarios/integration-test.js

echo ""
echo "3. Running E2E Transaction Flow Test..."
k6 run --out json=results/e2e.json scenarios/e2e-transaction.js

echo ""
echo "4. Running Realistic Load Test..."
k6 run --out json=results/load.json scenarios/load-test-realistic.js

echo ""
echo "5. Running WebSocket Test..."
k6 run --out json=results/websocket.json scenarios/websocket-test.js

echo ""
echo "6. Running Stress Test..."
k6 run --out json=results/stress.json scenarios/stress-test.js

echo ""
echo "=== All tests completed ==="
echo "Results saved in tests/k6/results/"
```

## Технологический стек
- **K6**: Performance testing tool
- **Context7**: Для получения актуальных K6 patterns
- **Target**: 11 СУЩЕСТВУЮЩИХ сервисов на портах 8080-8093

## Порядок выполнения

```bash
# 1. СКАНИРОВАНИЕ - проверить что все сервисы запущены
./check-services.sh

# 2. Context7 - получить актуальные K6 patterns
context7 docs k6 "load testing best practices"

# 3. Создать тесты для РЕАЛЬНЫХ endpoints

# 4. Запустить тесты
cd tests/k6
chmod +x run-all-tests.sh
./run-all-tests.sh

# 5. Анализ результатов
k6 report results/load.json
```

## Критически важно

1. **Тестировать ТОЛЬКО существующие сервисы** на портах 8080-8093
2. **НЕ создавать mock сервисы** - все уже работает
3. **Использовать Context7** для актуальных K6 patterns
4. **Проверить что сервисы запущены** перед тестами
5. **Записывать результаты** в Analytics Collector (8093)

## Результат
Полный набор K6 тестов для всех 11 существующих сервисов с:
- Integration tests
- E2E transaction flow tests
- Realistic load tests
- WebSocket tests
- Stress tests
- Автоматизированный test runner
