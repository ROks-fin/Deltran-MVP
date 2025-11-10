// K6 Configuration for DelTran MVP Services
// Port mapping for all 11 services

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
            metrics: '/metrics',
            tokens: '/api/v1/tokens',
            mint: '/api/v1/tokens/mint',
            burn: '/api/v1/tokens/burn',
            transfer: '/api/v1/tokens/transfer',
            balance: (bankId) => `/api/v1/tokens/balance/${bankId}`,
        }
    },
    obligationEngine: {
        url: 'http://localhost:8082',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            obligations: '/obligations',
            create: '/obligations/create',
            netting: '/obligations/netting',
        }
    },
    liquidityRouter: {
        url: 'http://localhost:8083',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            routes: '/routes',
            optimize: '/routes/optimize',
        }
    },
    riskEngine: {
        url: 'http://localhost:8084',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            assess: '/risk/assess',
            limits: '/risk/limits',
        }
    },
    clearingEngine: {
        url: 'http://localhost:8085',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            windows: '/api/v1/clearing/windows',
            currentWindow: '/api/v1/clearing/windows/current',
            clearingMetrics: '/api/v1/clearing/metrics',
        }
    },
    complianceEngine: {
        url: 'http://localhost:8086',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            check: '/compliance/check',
            rules: '/compliance/rules',
        }
    },
    settlementEngine: {
        url: 'http://localhost:8087',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            settlements: '/settlements',
            status: (id) => `/settlements/${id}/status`,
        }
    },
    reportingEngine: {
        url: 'http://localhost:8088',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            reports: '/reports',
            generate: '/reports/generate',
        }
    },
    notificationEngine: {
        http: 'http://localhost:8089',
        ws: 'ws://localhost:8090',
        endpoints: {
            health: '/health',
            metrics: '/metrics',
            notifications: '/notifications',
            send: '/notifications/send',
        }
    },
};

// Common headers
export const HEADERS = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
};

// Test data generators
export function generateRandomTransaction() {
    const senderBanks = ['ICICI', 'HDFC', 'AXIS', 'SBI'];
    const receiverBanks = ['ENBD', 'ADCB', 'DIB', 'NBAD'];
    const amounts = [1000, 5000, 10000, 50000, 100000, 500000];

    return {
        sender_bank: senderBanks[Math.floor(Math.random() * senderBanks.length)],
        receiver_bank: receiverBanks[Math.floor(Math.random() * receiverBanks.length)],
        amount: amounts[Math.floor(Math.random() * amounts.length)],
        from_currency: 'INR',
        to_currency: 'AED',
        sender_account: `ACC${Math.floor(Math.random() * 10000)}`,
        receiver_account: `ACC${Math.floor(Math.random() * 10000)}`,
        timestamp: new Date().toISOString(),
    };
}

export function generateRandomTokenRequest() {
    return {
        bank_id: `bank-${Math.floor(Math.random() * 100)}`,
        currency: Math.random() > 0.5 ? 'INR' : 'AED',
        amount: Math.floor(Math.random() * 1000000) + 1000,
    };
}
