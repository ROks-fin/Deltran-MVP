# DelTran Rail MVP - Makefile
.PHONY: help up down build logs seed test load chaos demo clean status restart

# Default target
help: ## Show this help message
	@echo "DelTran Rail MVP - Available Commands:"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# Infrastructure commands
up: ## Start all services with Docker Compose (3 ledger instances)
	@echo "🚀 Starting DelTran Rail MVP..."
	docker compose up -d --scale ledger=3
	@echo "✅ Services started successfully"
	@echo "   Gateway:    http://localhost:8000"
	@echo "   Grafana:    http://localhost:3000 (admin/admin)"
	@echo "   Jaeger:     http://localhost:16686"
	@echo "   Prometheus: http://localhost:9090"

down: ## Stop all services
	@echo "🛑 Stopping DelTran Rail MVP..."
	docker compose down -v
	@echo "✅ Services stopped"

reup: down up ## Restart all services (down + up)

build: ## Build all Docker images
	@echo "🔨 Building Docker images..."
	docker compose build
	@echo "✅ Build complete"

logs: ## Show logs from all services
	docker compose logs -f --tail=200

ps: ## Show status of all services
	@echo "📊 Service Status:"
	@docker compose ps

restart: ## Restart all services
	@echo "🔄 Restarting services..."
	docker compose restart
	@echo "✅ Services restarted"

status: ## Show status of all services
	@echo "📊 Service Status:"
	@docker compose ps
	@echo ""
	@echo "🏥 Health Checks:"
	@curl -s http://localhost:8000/health | jq -r '.status // "❌ Gateway not responding"' || echo "❌ Gateway not responding"
	@curl -s http://localhost:3000/api/health | jq -r '.database // "❌ Grafana not responding"' || echo "❌ Grafana not responding"

# Database commands
db-shell: ## Connect to PostgreSQL shell
	docker-compose exec postgres psql -U deltran -d deltran

migrate: ## Run database migrations
	@echo "🗄️  Running database migrations..."
	docker-compose exec postgres psql -U deltran -d deltran -f /docker-entrypoint-initdb.d/001_initial_schema.sql
	@echo "✅ Migrations complete"

db-migrate: migrate ## Alias for migrate

db-reset: ## Reset database (WARNING: destroys all data)
	@echo "⚠️  WARNING: This will destroy all data!"
	@read -p "Type 'yes' to continue: " confirm && [ "$$confirm" = "yes" ] || exit 1
	docker-compose down -v
	docker volume prune -f
	docker-compose up -d postgres redis nats
	@sleep 10
	$(MAKE) db-migrate
	@echo "✅ Database reset complete"

# Data seeding
seed: ## Generate and load seed data
	@echo "🌱 Seeding database with test data..."
	@if [ ! -f seed/seed_data.py ]; then \
		echo "❌ Seed script not found. Creating basic seed data..."; \
		python -c "print('Creating seed data...')"; \
	else \
		python seed/seed_data.py; \
	fi
	@echo "✅ Seed data loaded"

seed-banks: ## Seed bank participant data
	@echo "🏦 Seeding bank participants..."
	python seed/banks.py

seed-payments: ## Generate 1M micropayment test data
	@echo "💰 Generating 1M micropayment transactions..."
	python seed/payments.py --count 1000000

seed-scenarios: ## Load AED↔INR test scenarios
	@echo "🌍 Loading AED↔INR test scenarios..."
	python seed/scenarios.py

# Testing
test: ## Run all tests
	@echo "🧪 Running test suite..."
	pytest tests/ -v --tb=short
	@echo "✅ Tests complete"

test-unit: ## Run unit tests only
	@echo "🧪 Running unit tests..."
	pytest tests/unit/ -v

test-integration: ## Run integration tests
	@echo "🧪 Running integration tests..."
	pytest tests/integration/ -v

test-contracts: ## Run API contract tests
	@echo "🧪 Running contract tests..."
	pytest tests/contracts/ -v

test-coverage: ## Run tests with coverage report
	@echo "🧪 Running tests with coverage..."
	coverage run -m pytest tests/
	coverage report
	coverage html
	@echo "📊 Coverage report: htmlcov/index.html"

# Load testing
load: ## Run k6 load tests (500 TPS for 30-60 min)
	@echo "⚡ Starting load test (500 TPS)..."
	@if command -v k6 >/dev/null 2>&1; then \
		k6 run load/scenarios/payment_flow.js; \
	else \
		docker run --rm -i --network host grafana/k6:latest run - < load/scenarios/payment_flow.js; \
	fi
	@echo "✅ Load test complete"

load-stress: ## Run stress test with higher load
	@echo "⚡ Starting stress test (1000 TPS)..."
	K6_TPS=1000 K6_DURATION=10m make load

load-endurance: ## Run endurance test (500 TPS for 60 min)
	@echo "⚡ Starting endurance test (500 TPS for 60 min)..."
	K6_TPS=500 K6_DURATION=60m make load

# Chaos testing
chaos: ## Run chaos engineering tests
	@echo "🌪️  Starting chaos tests..."
	@if command -v toxiproxy-cli >/dev/null 2>&1; then \
		bash chaos/run_chaos.sh; \
	else \
		docker-compose exec toxiproxy /go/bin/toxiproxy-cli create postgres -l 0.0.0.0:5433 -u postgres:5432; \
		docker-compose exec toxiproxy /go/bin/toxiproxy-cli toxic add postgres -t latency -a latency=1000; \
		sleep 30; \
		docker-compose exec toxiproxy /go/bin/toxiproxy-cli toxic remove postgres latency; \
		docker-compose exec toxiproxy /go/bin/toxiproxy-cli delete postgres; \
	fi
	@echo "✅ Chaos tests complete"

chaos-network: ## Simulate network delays
	@echo "🌪️  Simulating network delays..."
	python chaos/network_chaos.py

chaos-node: ## Simulate node failures
	@echo "🌪️  Simulating node failures..."
	python chaos/node_chaos.py

chaos-consumer: ## Simulate consumer lag
	@echo "🌪️  Simulating consumer lag..."
	python chaos/consumer_chaos.py

# Demo and validation
demo: ## Run complete demo with acceptance checks
	@echo "🎬 Starting DelTran Rail MVP Demo..."
	@echo ""
	@echo "Step 1: Starting services..."
	$(MAKE) up
	@sleep 20
	@echo ""
	@echo "Step 2: Seeding data..."
	$(MAKE) seed
	@sleep 10
	@echo ""
	@echo "Step 3: Running acceptance tests..."
	$(MAKE) test-contracts
	@echo ""
	@echo "Step 4: Validating APIs..."
	@bash scripts/validate_apis.sh
	@echo ""
	@echo "Step 5: Running mini load test..."
	K6_TPS=50 K6_DURATION=2m make load
	@echo ""
	@echo "Step 6: Running chaos test..."
	$(MAKE) chaos-network
	@echo ""
	@echo "🎉 Demo complete! Check dashboards:"
	@echo "   Gateway:    http://localhost:8000"
	@echo "   Grafana:    http://localhost:3000"
	@echo "   Jaeger:     http://localhost:16686"

acceptance: ## Run acceptance checks
	@echo "✅ Running acceptance checks..."
	@bash scripts/acceptance_checks.sh

# Development
dev-up: ## Start services for development (with file watching)
	@echo "👨‍💻 Starting development environment..."
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

dev-logs: ## Show development logs
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml logs -f

lint: ## Run code linting
	@echo "🔍 Running linters..."
	flake8 --config=.flake8 .
	black --check .
	isort --check-only .

format: ## Format code
	@echo "✨ Formatting code..."
	black .
	isort .

type-check: ## Run type checking
	@echo "🔍 Running type checks..."
	mypy --config-file=mypy.ini .

pre-commit: ## Run pre-commit checks
	$(MAKE) lint
	$(MAKE) type-check
	$(MAKE) test-unit

# Monitoring
metrics: ## Show key metrics
	@echo "📊 Key Metrics:"
	@curl -s http://localhost:8000/metrics | grep -E "(http_requests_total|payments_total|settlement_batch_size)" | head -20

monitor: ## Start monitoring dashboard
	@echo "📈 Opening monitoring dashboard..."
	@open http://localhost:3000 || xdg-open http://localhost:3000 || echo "Open http://localhost:3000 manually"

alerts: ## Check for any alerts
	@echo "🚨 Checking alerts..."
	@curl -s http://localhost:9090/api/v1/alerts | jq -r '.data.alerts[] | select(.state=="firing") | .labels.alertname' || echo "No active alerts"

# Cleanup
clean: ## Clean up Docker resources
	@echo "🧹 Cleaning up..."
	docker-compose down -v --remove-orphans
	docker system prune -f
	docker volume prune -f
	@echo "✅ Cleanup complete"

clean-all: ## Clean up everything including images
	@echo "🧹 Deep cleaning..."
	docker-compose down -v --remove-orphans --rmi all
	docker system prune -a -f
	docker volume prune -f
	@echo "✅ Deep cleanup complete"

# Backup and restore
backup: ## Backup database
	@echo "💾 Creating database backup..."
	docker-compose exec postgres pg_dump -U deltran -d deltran > backup_$(shell date +%Y%m%d_%H%M%S).sql
	@echo "✅ Backup complete"

restore: ## Restore database from backup (specify BACKUP_FILE=filename.sql)
	@echo "📥 Restoring database..."
	@if [ -z "$(BACKUP_FILE)" ]; then echo "❌ Please specify BACKUP_FILE=filename.sql"; exit 1; fi
	docker-compose exec -T postgres psql -U deltran -d deltran < $(BACKUP_FILE)
	@echo "✅ Restore complete"

# Documentation
docs: ## Generate API documentation
	@echo "📚 Generating API documentation..."
	@curl -s http://localhost:8000/openapi.json > docs/openapi.json
	@echo "✅ API docs saved to docs/openapi.json"

docs-serve: ## Serve documentation locally
	@echo "📚 Serving documentation..."
	@open http://localhost:8000/docs || xdg-open http://localhost:8000/docs || echo "Open http://localhost:8000/docs manually"

# Quick shortcuts for common tasks
quick-test: up seed test-contracts ## Quick test: up + seed + contracts
quick-demo: up seed acceptance ## Quick demo: up + seed + acceptance
quick-clean: down clean ## Quick clean: down + clean

# Environment info
env-info: ## Show environment information
	@echo "🔍 Environment Information:"
	@echo "Docker version: $$(docker --version)"
	@echo "Docker Compose version: $$(docker-compose --version)"
	@echo "Python version: $$(python --version 2>&1)"
	@echo "Available services:"
	@docker-compose config --services
	@echo ""
	@echo "Service URLs:"
	@echo "  Gateway API:     http://localhost:8000"
	@echo "  Gateway Docs:    http://localhost:8000/docs"
	@echo "  Grafana:         http://localhost:3000"
	@echo "  Prometheus:      http://localhost:9090"
	@echo "  Jaeger:          http://localhost:16686"

# Performance profiling
profile: ## Run performance profiling
	@echo "🔬 Running performance profiling..."
	python -m cProfile -o profile_results.prof seed/payments.py --count 10000
	@echo "📊 Profile results: profile_results.prof"