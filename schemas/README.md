# DelTran Protocol Schemas

Protobuf definitions for DelTran Settlement Rail.

## Overview

These schemas define the **protocol layer** for DelTran, providing:
- Type-safe communication between Rust and Go services
- Formal API contracts
- Backward-compatible versioning
- Multi-language code generation

## Schemas

| Schema | Description | Primary Consumer |
|--------|-------------|------------------|
| `payment.proto` | Payment lifecycle and events | Gateway, Ledger |
| `ledger.proto` | Append-only event log operations | Rust Ledger Core |
| `settlement.proto` | Netting and settlement cycles | Settlement Engine |
| `risk.proto` | Risk assessment and FX exposure | Risk Engine |
| `compliance.proto` | Sanctions, AML, Travel Rule | Compliance Service |

## Code Generation

### Prerequisites

```bash
# Install protoc compiler
brew install protobuf  # macOS
# or
apt-get install protobuf-compiler  # Linux

# Install language-specific plugins
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest

cargo install protobuf-codegen
```

### Generate Code

```bash
# Generate all (Go + Rust)
make generate

# Generate Go only
make generate-go

# Generate Rust only
make generate-rust
```

### Generated Files

```
Generated code structure:

Go:
pb/
├── payment/v1/
│   ├── payment.pb.go
│   └── payment_grpc.pb.go
├── ledger/v1/
│   ├── ledger.pb.go
│   └── ledger_grpc.pb.go
├── settlement/v1/
├── risk/v1/
└── compliance/v1/

Rust:
ledger-core/src/pb/
├── payment.rs
├── ledger.rs
├── settlement.rs
├── risk.rs
└── compliance.rs
```

## Usage Examples

### Go (Gateway Service)

```go
package main

import (
    "context"
    pb "github.com/deltran/pb/payment/v1"
)

func initiatePayment(client pb.PaymentServiceClient) error {
    req := &pb.PaymentInitiateRequest{
        Amount:          "1000.00",
        Currency:        "USD",
        DebtorAccount:   "US1234567890",
        CreditorAccount: "AE9876543210",
        IdempotencyKey:  generateKey(),
    }

    resp, err := client.InitiatePayment(context.Background(), req)
    if err != nil {
        return err
    }

    log.Printf("Payment initiated: %s", resp.PaymentId)
    return nil
}
```

### Rust (Ledger Core)

```rust
use crate::pb::payment::{Payment, PaymentStatus};
use crate::pb::ledger::{AppendEventRequest, AppendEventResponse};

pub async fn append_event(
    request: AppendEventRequest,
) -> Result<AppendEventResponse, Error> {
    let event = LedgerEvent {
        event_id: Uuid::new_v7(),
        payment_id: Uuid::parse_str(&request.payment_id)?,
        event_type: request.event_type.into(),
        amount: Decimal::from_str(&request.amount)?,
        // ...
    };

    ledger.append(event).await?;

    Ok(AppendEventResponse {
        event_id: event.event_id.to_string(),
        payment_id: event.payment_id.to_string(),
        timestamp_nanos: event.timestamp_nanos,
        signature: event.signature.to_bytes().to_vec(),
        block_id: None,
    })
}
```

## Versioning Strategy

We use **semantic versioning** with package namespaces:
- `deltran.payment.v1` - Initial stable version
- `deltran.payment.v2` - Breaking changes (future)

### Backward Compatibility Rules

1. **Safe changes** (no version bump):
   - Add optional fields
   - Add new enum values (with UNSPECIFIED=0)
   - Add new RPC methods
   - Add new messages

2. **Breaking changes** (requires v2):
   - Remove fields
   - Change field types
   - Remove enum values
   - Change RPC signatures

## Field Conventions

### Decimal Values

Use **string** for monetary amounts (not double):

```protobuf
message Payment {
  string amount = 1;  // ✅ "1000.50" (exact)
  // double amount = 1;  ❌ floating-point (lossy)
}
```

### Timestamps

Use `google.protobuf.Timestamp` for wall-clock times:

```protobuf
import "google/protobuf/timestamp.proto";

message Payment {
  google.protobuf.Timestamp created_at = 1;
}
```

Use `int64` for nanoseconds-since-epoch for high-precision ordering:

```protobuf
message PaymentEvent {
  int64 timestamp_nanos = 1;  // For deterministic ordering
}
```

### Identifiers

Use **string** for UUIDs (not bytes):

```protobuf
message Payment {
  string payment_id = 1;  // ✅ "550e8400-e29b-41d4-a716-446655440000"
  // bytes payment_id = 1;   ❌ binary (harder to debug)
}
```

### Enums

Always start with `_UNSPECIFIED = 0`:

```protobuf
enum PaymentStatus {
  PAYMENT_STATUS_UNSPECIFIED = 0;  // Required for proto3
  PAYMENT_STATUS_INITIATED = 1;
  PAYMENT_STATUS_COMPLETED = 2;
}
```

## Testing

### Contract Tests

Ensure generated code matches expectations:

```bash
# Go contract tests
cd ../gateway
go test ./internal/contracts/...

# Rust contract tests
cd ../ledger-core
cargo test --test contract_tests
```

### Schema Validation

```bash
# Validate all schemas
make validate

# Check for breaking changes
buf breaking --against '.git#branch=main'
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Protobuf CI

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: bufbuild/buf-setup-action@v1

      - name: Validate schemas
        run: buf lint

      - name: Check breaking changes
        run: buf breaking --against 'https://github.com/deltran/deltran.git#branch=main'

      - name: Generate code
        run: make generate

      - name: Run contract tests
        run: |
          cd gateway && go test ./...
          cd ../ledger-core && cargo test
```

## Documentation

### Generate HTML docs

```bash
protoc --doc_out=html,index.html:./docs *.proto
```

### Serve docs locally

```bash
cd docs
python -m http.server 8080
# Open http://localhost:8080
```

## Troubleshooting

### Common Issues

**Issue:** `protoc: command not found`

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt-get install -y protobuf-compiler

# Verify
protoc --version
```

**Issue:** `protoc-gen-go: program not found`

```bash
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest

# Ensure $GOPATH/bin is in PATH
export PATH=$PATH:$(go env GOPATH)/bin
```

**Issue:** `prost-build` errors in Rust

```bash
# Update dependencies
cd ledger-core
cargo update
cargo build
```

## Contributing

### Adding New Messages

1. Create/update `.proto` file
2. Run `buf lint` to validate
3. Run `make generate`
4. Add tests for new types
5. Update this README

### Breaking Changes

Breaking changes require:
1. Create new version (e.g., `v2`)
2. Update import paths
3. Maintain `v1` for 6 months minimum
4. Provide migration guide

## References

- [Protocol Buffers Language Guide](https://protobuf.dev/programming-guides/proto3/)
- [gRPC Go Quick Start](https://grpc.io/docs/languages/go/quickstart/)
- [Prost (Rust)](https://docs.rs/prost/latest/prost/)
- [Buf](https://buf.build/docs/) - Schema management

## License

Proprietary - DelTran Settlement Rail