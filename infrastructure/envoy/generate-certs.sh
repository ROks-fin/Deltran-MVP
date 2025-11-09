#!/bin/bash
# Generate self-signed certificates for Envoy mTLS (MVP only)
# For production, use proper CA-signed certificates

set -e

CERTS_DIR="./certs"
DAYS_VALID=365

echo "Generating TLS certificates for DelTran Envoy..."

# Create certs directory
mkdir -p "$CERTS_DIR"

# Generate CA private key
echo "1. Generating CA private key..."
openssl genrsa -out "$CERTS_DIR/ca-key.pem" 4096

# Generate CA certificate
echo "2. Generating CA certificate..."
openssl req -new -x509 -days $DAYS_VALID -key "$CERTS_DIR/ca-key.pem" \
    -out "$CERTS_DIR/ca-cert.pem" \
    -subj "/C=AE/ST=Dubai/L=Dubai/O=DelTran/OU=IT/CN=DelTran Root CA"

# Generate server private key
echo "3. Generating server private key..."
openssl genrsa -out "$CERTS_DIR/server-key.pem" 4096

# Generate server CSR
echo "4. Generating server certificate signing request..."
openssl req -new -key "$CERTS_DIR/server-key.pem" \
    -out "$CERTS_DIR/server.csr" \
    -subj "/C=AE/ST=Dubai/L=Dubai/O=DelTran/OU=IT/CN=localhost"

# Generate server certificate signed by CA
echo "5. Generating server certificate..."
openssl x509 -req -days $DAYS_VALID \
    -in "$CERTS_DIR/server.csr" \
    -CA "$CERTS_DIR/ca-cert.pem" \
    -CAkey "$CERTS_DIR/ca-key.pem" \
    -CAcreateserial \
    -out "$CERTS_DIR/server-cert.pem" \
    -extfile <(printf "subjectAltName=DNS:localhost,DNS:envoy,DNS:deltran-envoy,IP:127.0.0.1")

# Generate client private key (for mTLS testing)
echo "6. Generating client private key..."
openssl genrsa -out "$CERTS_DIR/client-key.pem" 4096

# Generate client CSR
echo "7. Generating client certificate signing request..."
openssl req -new -key "$CERTS_DIR/client-key.pem" \
    -out "$CERTS_DIR/client.csr" \
    -subj "/C=AE/ST=Dubai/L=Dubai/O=DelTran/OU=IT/CN=DelTran Client"

# Generate client certificate signed by CA
echo "8. Generating client certificate..."
openssl x509 -req -days $DAYS_VALID \
    -in "$CERTS_DIR/client.csr" \
    -CA "$CERTS_DIR/ca-cert.pem" \
    -CAkey "$CERTS_DIR/ca-key.pem" \
    -CAcreateserial \
    -out "$CERTS_DIR/client-cert.pem"

# Clean up CSR files
rm "$CERTS_DIR/server.csr" "$CERTS_DIR/client.csr"

# Set appropriate permissions
chmod 600 "$CERTS_DIR"/*.pem
chmod 644 "$CERTS_DIR/ca-cert.pem" "$CERTS_DIR/server-cert.pem" "$CERTS_DIR/client-cert.pem"

echo "
✅ Certificate generation complete!

Files created in $CERTS_DIR/:
  - ca-cert.pem       (CA certificate)
  - ca-key.pem        (CA private key)
  - server-cert.pem   (Server certificate)
  - server-key.pem    (Server private key)
  - client-cert.pem   (Client certificate for mTLS)
  - client-key.pem    (Client private key)

Test mTLS connection:
  curl --cacert $CERTS_DIR/ca-cert.pem \\
       --cert $CERTS_DIR/client-cert.pem \\
       --key $CERTS_DIR/client-key.pem \\
       https://localhost:8443/health

⚠️  WARNING: These are self-signed certificates for MVP/development only!
    For production, use certificates from a trusted CA.
"
