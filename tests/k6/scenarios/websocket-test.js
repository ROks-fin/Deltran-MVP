// WebSocket Test for Notification Engine
// Tests WebSocket connections and real-time notifications

import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Counter, Trend } from 'k6/metrics';
import { SERVICES } from '../config/services.js';

const wsConnections = new Counter('ws_connections');
const wsMessagesReceived = new Counter('ws_messages_received');
const wsMessageLatency = new Trend('ws_message_latency');

export const options = {
    stages: [
        { duration: '30s', target: 20 },  // Ramp up to 20 connections
        { duration: '1m', target: 20 },   // Stay at 20 connections
        { duration: '30s', target: 0 },   // Ramp down
    ],
    thresholds: {
        'ws_connections': ['count>0'],
        'ws_messages_received': ['count>0'],
        'ws_message_latency': ['p(95)<500'],
    },
};

export default function() {
    const url = `ws://localhost:8089/ws`;
    const params = {
        tags: { name: 'NotificationWebSocket' },
        headers: {
            'Authorization': 'Bearer test-jwt-token',
        }
    };

    const res = ws.connect(url, params, function(socket) {
        socket.on('open', () => {
            console.log(`✅ WebSocket connected (VU ${__VU})`);
            wsConnections.add(1);

            // Subscribe to channels
            socket.send(JSON.stringify({
                type: 'subscribe',
                channels: ['transactions', 'settlements', 'notifications'],
                user_id: `user-${__VU}`,
            }));

            // Send periodic heartbeat
            socket.setInterval(() => {
                socket.send(JSON.stringify({
                    type: 'ping',
                    timestamp: Date.now(),
                }));
            }, 10000);
        });

        socket.on('message', (data) => {
            const receiveTime = Date.now();
            wsMessagesReceived.add(1);

            try {
                const msg = JSON.parse(data);

                // Check message structure
                const isValid = check(msg, {
                    'message has type': (m) => m.type !== undefined,
                    'message has data or timestamp': (m) => m.data !== undefined || m.timestamp !== undefined,
                });

                // Calculate latency for pong messages
                if (msg.type === 'pong' && msg.timestamp) {
                    const latency = receiveTime - msg.timestamp;
                    wsMessageLatency.add(latency);
                }

                // Log transaction notifications
                if (msg.type === 'transaction_update') {
                    console.log(`📨 Transaction update: ${msg.data.transaction_id} - ${msg.data.status}`);
                }

                // Log settlement notifications
                if (msg.type === 'settlement_complete') {
                    console.log(`💰 Settlement complete: ${msg.data.settlement_id}`);
                }
            } catch (e) {
                console.log(`❌ Failed to parse message: ${e}`);
            }
        });

        socket.on('error', (e) => {
            if (e.error() != "websocket: close sent") {
                console.log(`❌ WebSocket error: ${e.error()}`);
            }
        });

        socket.on('close', () => {
            console.log(`🔌 WebSocket closed (VU ${__VU})`);
        });

        // Keep connection alive for the duration
        socket.setTimeout(() => {
            console.log(`⏰ Closing WebSocket after timeout (VU ${__VU})`);
            socket.close();
        }, 60000); // 60 seconds
    });

    check(res, {
        'WebSocket handshake successful': (r) => r && r.status === 101
    });

    sleep(1);
}

export function handleSummary(data) {
    console.log('📊 WebSocket Test Summary:');
    console.log(`   Total Connections: ${data.metrics.ws_connections.values.count}`);
    console.log(`   Messages Received: ${data.metrics.ws_messages_received.values.count}`);
    console.log(`   Avg Message Latency: ${data.metrics.ws_message_latency.values.avg.toFixed(2)}ms`);
    console.log(`   P95 Message Latency: ${data.metrics.ws_message_latency.values['p(95)'].toFixed(2)}ms`);

    return {
        'stdout': JSON.stringify(data, null, 2),
        '../results/websocket.json': JSON.stringify(data),
    };
}
