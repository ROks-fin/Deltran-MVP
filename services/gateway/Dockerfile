# syntax=docker/dockerfile:1
FROM golang:1.24-alpine AS builder

ENV GOTOOLCHAIN=auto

RUN apk add --no-cache git ca-certificates tzdata

WORKDIR /build

COPY go.mod go.sum ./
RUN go mod download && go mod verify

COPY . .

RUN go mod tidy

RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
    go build -ldflags="-s -w" -a -installsuffix cgo \
    -o /app/service ./cmd/main.go

FROM alpine:3.21 AS runtime

RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

WORKDIR /app

COPY --from=builder --chown=appuser:appuser /app/service /app/service

USER appuser

EXPOSE 8080

ENTRYPOINT ["/app/service"]
