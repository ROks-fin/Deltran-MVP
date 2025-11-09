FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git make ca-certificates tzdata

WORKDIR /app

# Copy go mod files
COPY go.mod go.sum ./
RUN go mod download

# Copy source code
COPY . .

# Build binary
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o reporting-engine cmd/server/main.go

# Final stage
FROM alpine:latest

RUN apk --no-cache add ca-certificates tzdata

WORKDIR /root/

# Copy binary from builder
COPY --from=builder /app/reporting-engine .
COPY --from=builder /app/config.yaml .

# Create temp directory for reports
RUN mkdir -p /tmp/reports

EXPOSE 8087 9097

CMD ["./reporting-engine"]
