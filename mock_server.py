#!/usr/bin/env python3
"""
Mock DelTran MVP Server for K6 Testing
Provides basic health and transaction endpoints for all 11 services
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import time
import random
import uuid
from datetime import datetime

class MockDelTranHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        # Health check endpoint
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {
                'status': 'healthy',
                'service': 'deltran-mock',
                'timestamp': datetime.utcnow().isoformat(),
                'version': '1.0.0'
            }
            self.wfile.write(json.dumps(response).encode())
            return

        # Metrics endpoint
        elif self.path == '/metrics':
            self.send_response(200)
            self.send_header('Content-type', 'text/plain; version=0.0.4')
            self.end_headers()
            metrics = """# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/health",status="200"} 42
http_requests_total{method="POST",path="/api/v1/transfer",status="200"} 123

# HELP http_request_duration_seconds HTTP request duration in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="POST",path="/api/v1/transfer",le="0.005"} 50
http_request_duration_seconds_bucket{method="POST",path="/api/v1/transfer",le="0.01"} 100
http_request_duration_seconds_bucket{method="POST",path="/api/v1/transfer",le="0.05"} 120
http_request_duration_seconds_bucket{method="POST",path="/api/v1/transfer",le="+Inf"} 123
"""
            self.wfile.write(metrics.encode())
            return

        # Transaction status endpoint
        elif self.path.startswith('/api/v1/transactions/'):
            tx_id = self.path.split('/')[-1]
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {
                'transaction_id': tx_id,
                'status': random.choice(['pending', 'processing', 'completed']),
                'timestamp': datetime.utcnow().isoformat()
            }
            self.wfile.write(json.dumps(response).encode())
            return

        # Not found
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        # Transfer endpoint
        if self.path == '/api/v1/transfer':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)

            try:
                data = json.loads(post_data.decode())
            except:
                data = {}

            # Simulate processing time
            time.sleep(random.uniform(0.01, 0.1))

            # Generate transaction ID
            tx_id = f"TX-{uuid.uuid4().hex[:12].upper()}"

            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()

            response = {
                'transaction_id': tx_id,
                'status': 'pending',
                'sender_bank': data.get('sender_bank', 'UNKNOWN'),
                'receiver_bank': data.get('receiver_bank', 'UNKNOWN'),
                'amount': data.get('amount', 0),
                'from_currency': data.get('from_currency', 'INR'),
                'to_currency': data.get('to_currency', 'AED'),
                'timestamp': datetime.utcnow().isoformat(),
                'estimated_completion': '2-3 minutes'
            }

            self.wfile.write(json.dumps(response).encode())
            return

        # Token mint endpoint
        elif self.path == '/tokens/mint':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)

            token_id = f"TK-{uuid.uuid4().hex[:12].upper()}"

            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()

            response = {
                'token_id': token_id,
                'status': 'minted',
                'timestamp': datetime.utcnow().isoformat()
            }

            self.wfile.write(json.dumps(response).encode())
            return

        # Not found
        self.send_response(404)
        self.end_headers()

    def log_message(self, format, *args):
        # Suppress default logging
        pass

def run_mock_server(port):
    server_address = ('', port)
    httpd = HTTPServer(server_address, MockDelTranHandler)
    print(f'✅ Mock DelTran server running on port {port}')
    print(f'   Health: http://localhost:{port}/health')
    print(f'   Metrics: http://localhost:{port}/metrics')
    print(f'   Transfer: POST http://localhost:{port}/api/v1/transfer')
    httpd.serve_forever()

if __name__ == '__main__':
    import sys
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    try:
        run_mock_server(port)
    except KeyboardInterrupt:
        print(f'\n⏹️  Mock server on port {port} stopped')
