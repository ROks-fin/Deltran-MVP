#!/bin/bash
# Generate self-signed certificates for Envoy mTLS
# For MVP/Development use only - use proper CA-signed certs in production

set -e

CERT_DIR="../envoy/certs"
DAYS_VALID=365

echo "=== Generating Self-Signed Certificates for Envoy mTLS ==="
echo "Certificate directory: $CERT_DIR"
echo ""

# Create certs directory if it doesn't exist
mkdir -p "$CERT_DIR"
cd "$CERT_DIR"

# Generate CA private key
echo "1. Generating CA private key..."
openssl genrsa -out ca-key.pem 4096

# Generate CA certificate
echo "2. Generating CA certificate..."
openssl req -new -x509 -days $DAYS_VALID -key ca-key.pem -sha256 -out ca-cert.pem \
    -subj "/C=US/ST=Delaware/L=Wilmington/O=DelTran/OU=Infrastructure/CN=DelTran Root CA"

# Generate server private key
echo "3. Generating server private key..."
openssl genrsa -out server-key.pem 4096

# Generate server CSR
echo "4. Generating server certificate signing request..."
openssl req -subj "/CN=deltran-envoy" -sha256 -new -key server-key.pem -out server.csr

# Create server certificate extensions
echo "5. Creating server certificate extensions..."
cat > server-extfile.cnf <<EOF
subjectAltName = DNS:deltran-envoy,DNS:localhost,IP:127.0.0.1,DNS:gateway
extendedKeyUsage = serverAuth
EOF

# Sign server certificate
echo "6. Signing server certificate with CA..."
openssl x509 -req -days $DAYS_VALID -sha256 -in server.csr -CA ca-cert.pem \
    -CAkey ca-key.pem -CAcreateserial -out server-cert.pem -extfile server-extfile.cnf

# Generate client private key (for mTLS)
echo "7. Generating client private key..."
openssl genrsa -out client-key.pem 4096

# Generate client CSR
echo "8. Generating client certificate signing request..."
openssl req -subj "/CN=deltran-client" -new -key client-key.pem -out client.csr

# Create client certificate extensions
echo "9. Creating client certificate extensions..."
cat > client-extfile.cnf <<EOF
extendedKeyUsage = clientAuth
EOF

# Sign client certificate
echo "10. Signing client certificate with CA..."
openssl x509 -req -days $DAYS_VALID -sha256 -in client.csr -CA ca-cert.pem \
    -CAkey ca-key.pem -CAcreateserial -out client-cert.pem -extfile client-extfile.cnf

# Set proper permissions
echo "11. Setting file permissions..."
chmod 0444 ca-cert.pem server-cert.pem client-cert.pem
chmod 0400 ca-key.pem server-key.pem client-key.pem

# Clean up temporary files
echo "12. Cleaning up temporary files..."
rm -f server.csr client.csr server-extfile.cnf client-extfile.cnf ca-cert.srl

echo ""
echo "=== Certificate Generation Complete ==="
echo ""
echo "Generated files:"
echo "  CA Certificate:     ca-cert.pem"
echo "  CA Private Key:     ca-key.pem (keep secure!)"
echo "  Server Certificate: server-cert.pem"
echo "  Server Private Key: server-key.pem (keep secure!)"
echo "  Client Certificate: client-cert.pem"
echo "  Client Private Key: client-key.pem (keep secure!)"
echo ""
echo "To verify certificates:"
echo "  openssl x509 -in server-cert.pem -text -noout"
echo "  openssl verify -CAfile ca-cert.pem server-cert.pem"
echo ""
echo "WARNING: These are self-signed certificates for development only!"
echo "Use proper CA-signed certificates in production."
