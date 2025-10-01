# DelTran Protobuf Makefile

.PHONY: help generate generate-go generate-rust validate clean lint

# Colors for output
GREEN  := \033[0;32m
YELLOW := \033[0;33m
RED    := \033[0;31m
NC     := \033[0m # No Color

help: ## Show this help message
	@echo "DelTran Protobuf Code Generation"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# Directories
PROTO_DIR := .
GO_OUT_DIR := ../pb
RUST_OUT_DIR := ../ledger-core/src/pb

# Proto files
PROTO_FILES := $(wildcard *.proto)

# Check prerequisites
check-prereqs: ## Check if required tools are installed
	@echo "$(YELLOW)Checking prerequisites...$(NC)"
	@which protoc > /dev/null || (echo "$(RED)protoc not found. Install: brew install protobuf$(NC)" && exit 1)
	@which protoc-gen-go > /dev/null || (echo "$(RED)protoc-gen-go not found. Run: go install google.golang.org/protobuf/cmd/protoc-gen-go@latest$(NC)" && exit 1)
	@which protoc-gen-go-grpc > /dev/null || (echo "$(RED)protoc-gen-go-grpc not found. Run: go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest$(NC)" && exit 1)
	@echo "$(GREEN)✓ All prerequisites installed$(NC)"

# Generate all
generate: check-prereqs generate-go generate-rust ## Generate code for all languages
	@echo "$(GREEN)✓ Code generation complete$(NC)"

# Generate Go code
generate-go: check-prereqs ## Generate Go code from protobuf
	@echo "$(YELLOW)Generating Go code...$(NC)"
	@mkdir -p $(GO_OUT_DIR)

	@# Generate payment.proto
	@protoc \
		--go_out=$(GO_OUT_DIR) \
		--go_opt=paths=source_relative \
		--go-grpc_out=$(GO_OUT_DIR) \
		--go-grpc_opt=paths=source_relative \
		-I $(PROTO_DIR) \
		payment.proto

	@# Generate ledger.proto
	@protoc \
		--go_out=$(GO_OUT_DIR) \
		--go_opt=paths=source_relative \
		--go-grpc_out=$(GO_OUT_DIR) \
		--go-grpc_opt=paths=source_relative \
		-I $(PROTO_DIR) \
		ledger.proto

	@# Generate settlement.proto
	@protoc \
		--go_out=$(GO_OUT_DIR) \
		--go_opt=paths=source_relative \
		--go-grpc_out=$(GO_OUT_DIR) \
		--go-grpc_opt=paths=source_relative \
		-I $(PROTO_DIR) \
		settlement.proto

	@# Generate risk.proto
	@protoc \
		--go_out=$(GO_OUT_DIR) \
		--go_opt=paths=source_relative \
		--go-grpc_out=$(GO_OUT_DIR) \
		--go-grpc_opt=paths=source_relative \
		-I $(PROTO_DIR) \
		risk.proto

	@# Generate compliance.proto
	@protoc \
		--go_out=$(GO_OUT_DIR) \
		--go_opt=paths=source_relative \
		--go-grpc_out=$(GO_OUT_DIR) \
		--go-grpc_opt=paths=source_relative \
		-I $(PROTO_DIR) \
		compliance.proto

	@echo "$(GREEN)✓ Go code generated in $(GO_OUT_DIR)$(NC)"

# Generate Rust code
generate-rust: ## Generate Rust code from protobuf
	@echo "$(YELLOW)Generating Rust code...$(NC)"
	@echo "$(YELLOW)Note: Rust code generation is handled by build.rs in ledger-core$(NC)"
	@echo "$(YELLOW)Run 'cargo build' in ledger-core directory$(NC)"
	@echo "$(GREEN)✓ Rust generation configured$(NC)"

# Validate schemas
validate: check-prereqs ## Validate protobuf schemas
	@echo "$(YELLOW)Validating schemas...$(NC)"
	@for proto in $(PROTO_FILES); do \
		echo "Validating $$proto..."; \
		protoc --descriptor_set_out=/dev/null -I $(PROTO_DIR) $$proto || exit 1; \
	done
	@echo "$(GREEN)✓ All schemas valid$(NC)"

# Lint schemas (requires buf)
lint: ## Lint protobuf schemas with buf
	@if which buf > /dev/null; then \
		echo "$(YELLOW)Linting with buf...$(NC)"; \
		buf lint; \
		echo "$(GREEN)✓ Linting complete$(NC)"; \
	else \
		echo "$(YELLOW)buf not installed, skipping lint$(NC)"; \
		echo "Install buf: brew install bufbuild/buf/buf"; \
	fi

# Check for breaking changes
breaking: ## Check for breaking changes (requires buf)
	@if which buf > /dev/null; then \
		echo "$(YELLOW)Checking for breaking changes...$(NC)"; \
		buf breaking --against '.git#branch=main' || echo "$(YELLOW)Warning: Breaking changes detected$(NC)"; \
	else \
		echo "$(YELLOW)buf not installed, skipping breaking check$(NC)"; \
	fi

# Clean generated files
clean: ## Clean generated code
	@echo "$(YELLOW)Cleaning generated files...$(NC)"
	@rm -rf $(GO_OUT_DIR)/*
	@rm -rf $(RUST_OUT_DIR)/*
	@echo "$(GREEN)✓ Cleaned$(NC)"

# Generate documentation
docs: ## Generate HTML documentation
	@echo "$(YELLOW)Generating documentation...$(NC)"
	@mkdir -p docs
	@protoc --doc_out=html,index.html:./docs -I $(PROTO_DIR) $(PROTO_FILES)
	@echo "$(GREEN)✓ Documentation generated in ./docs/index.html$(NC)"
	@echo "$(YELLOW)View: open docs/index.html$(NC)"

# Watch for changes (requires entr)
watch: ## Watch for changes and regenerate
	@if which entr > /dev/null; then \
		echo "$(YELLOW)Watching for changes...$(NC)"; \
		ls *.proto | entr make generate; \
	else \
		echo "$(RED)entr not installed$(NC)"; \
		echo "Install: brew install entr"; \
	fi

# Format protos (requires clang-format)
format: ## Format protobuf files
	@if which clang-format > /dev/null; then \
		echo "$(YELLOW)Formatting protos...$(NC)"; \
		clang-format -i $(PROTO_FILES); \
		echo "$(GREEN)✓ Formatted$(NC)"; \
	else \
		echo "$(YELLOW)clang-format not installed, skipping format$(NC)"; \
	fi

# Install tools
install-tools: ## Install required tools
	@echo "$(YELLOW)Installing tools...$(NC)"
	@echo "Installing Go plugins..."
	@go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
	@go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
	@echo "$(GREEN)✓ Go plugins installed$(NC)"
	@echo ""
	@echo "$(YELLOW)Optional tools:$(NC)"
	@echo "  buf:          brew install bufbuild/buf/buf"
	@echo "  entr:         brew install entr"
	@echo "  clang-format: brew install clang-format"

# CI target
ci: validate lint generate ## Run CI checks
	@echo "$(GREEN)✓ All CI checks passed$(NC)"