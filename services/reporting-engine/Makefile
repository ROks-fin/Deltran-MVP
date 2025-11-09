.PHONY: build test run clean docker-build docker-run

BINARY_NAME=reporting-engine
VERSION?=1.0.0
BUILD_DIR=build

build:
	@echo "Building $(BINARY_NAME)..."
	@go build -o $(BUILD_DIR)/$(BINARY_NAME) cmd/server/main.go

test:
	@echo "Running tests..."
	@go test -v -cover ./...

test-coverage:
	@echo "Running tests with coverage..."
	@go test -v -coverprofile=coverage.out ./...
	@go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report generated: coverage.html"

run:
	@echo "Running $(BINARY_NAME)..."
	@go run cmd/server/main.go

clean:
	@echo "Cleaning..."
	@rm -rf $(BUILD_DIR)
	@rm -f coverage.out coverage.html

docker-build:
	@echo "Building Docker image..."
	@docker build -t deltran/reporting-engine:$(VERSION) .

docker-run:
	@echo "Running Docker container..."
	@docker run -p 8087:8087 -p 9097:9097 \
		-e DB_PASSWORD=deltran \
		-e S3_ENDPOINT=http://localhost:9000 \
		-e AWS_ACCESS_KEY_ID=minioadmin \
		-e AWS_SECRET_ACCESS_KEY=minioadmin \
		deltran/reporting-engine:$(VERSION)

deps:
	@echo "Downloading dependencies..."
	@go mod download
	@go mod tidy

lint:
	@echo "Running linter..."
	@golangci-lint run

benchmark:
	@echo "Running benchmarks..."
	@go test -bench=. -benchmem ./...

performance-test:
	@echo "Running performance test for 1M rows..."
	@go test -v -run=TestCSVGenerator_Performance ./internal/generators/

help:
	@echo "Available targets:"
	@echo "  build           - Build the binary"
	@echo "  test            - Run tests"
	@echo "  test-coverage   - Run tests with coverage report"
	@echo "  run             - Run the service locally"
	@echo "  clean           - Clean build artifacts"
	@echo "  docker-build    - Build Docker image"
	@echo "  docker-run      - Run Docker container"
	@echo "  deps            - Download dependencies"
	@echo "  lint            - Run linter"
	@echo "  benchmark       - Run benchmarks"
	@echo "  performance-test - Run 1M row performance test"
