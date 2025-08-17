# CrewChief Web UI Makefile
# Provides convenient shortcuts for Docker operations

.PHONY: help dev prod build clean logs shell test db-migrate db-seed

# Colors
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m # No Color

# Default target
help: ## Show this help message
	@echo "$(BLUE)CrewChief Web UI Docker Commands$(NC)"
	@echo "=================================="
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make $(GREEN)<target>$(NC)\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(BLUE)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

dev: ## Start development environment
	@echo "$(GREEN)🚀 Starting development environment...$(NC)"
	./scripts/docker-run.sh up

dev-tools: ## Start development environment with tools (pgAdmin, Redis Commander)
	@echo "$(GREEN)🚀 Starting development environment with tools...$(NC)"
	./scripts/docker-run.sh up --profile dev-tools

dev-bg: ## Start development environment in background
	@echo "$(GREEN)🚀 Starting development environment in background...$(NC)"
	./scripts/docker-run.sh up -d

dev-build: ## Start development environment with build
	@echo "$(GREEN)🚀 Starting development environment with build...$(NC)"
	./scripts/docker-run.sh up --build

##@ Production

prod: ## Start production environment
	@echo "$(GREEN)🚀 Starting production environment...$(NC)"
	./scripts/docker-run.sh up -e production

prod-bg: ## Start production environment in background
	@echo "$(GREEN)🚀 Starting production environment in background...$(NC)"
	./scripts/docker-run.sh up -e production -d

prod-build: ## Build and start production environment
	@echo "$(GREEN)🚀 Building and starting production environment...$(NC)"
	./scripts/docker-run.sh up -e production --build

##@ Building

build: ## Build production Docker image
	@echo "$(GREEN)🔨 Building production image...$(NC)"
	./scripts/docker-build.sh

build-dev: ## Build development Docker image
	@echo "$(GREEN)🔨 Building development image...$(NC)"
	./scripts/docker-build.sh -t development

build-no-cache: ## Build production image without cache
	@echo "$(GREEN)🔨 Building production image without cache...$(NC)"
	./scripts/docker-build.sh --no-cache

build-push: ## Build and push production image
	@echo "$(GREEN)🔨 Building and pushing production image...$(NC)"
	./scripts/docker-build.sh --push --tag-latest

##@ Management

stop: ## Stop all services
	@echo "$(YELLOW)🛑 Stopping services...$(NC)"
	./scripts/docker-run.sh down

restart: ## Restart all services
	@echo "$(YELLOW)🔄 Restarting services...$(NC)"
	./scripts/docker-run.sh restart

clean: ## Clean up Docker resources
	@echo "$(YELLOW)🧹 Cleaning up Docker resources...$(NC)"
	./scripts/docker-run.sh clean

status: ## Show service status
	@echo "$(BLUE)📊 Service status:$(NC)"
	./scripts/docker-run.sh ps

##@ Logs and Debugging

logs: ## Show all service logs
	@echo "$(BLUE)📋 Showing all logs...$(NC)"
	./scripts/docker-run.sh logs

logs-web: ## Show web-ui service logs
	@echo "$(BLUE)📋 Showing web-ui logs...$(NC)"
	./scripts/docker-run.sh logs web-ui

logs-db: ## Show database logs
	@echo "$(BLUE)📋 Showing database logs...$(NC)"
	./scripts/docker-run.sh logs postgres

logs-redis: ## Show Redis logs
	@echo "$(BLUE)📋 Showing Redis logs...$(NC)"
	./scripts/docker-run.sh logs redis

logs-follow: ## Follow all service logs
	@echo "$(BLUE)📋 Following all logs...$(NC)"
	./scripts/docker-run.sh logs -f

shell: ## Open shell in web-ui container
	@echo "$(BLUE)🐚 Opening shell in web-ui container...$(NC)"
	./scripts/docker-run.sh shell

shell-db: ## Open shell in database container
	@echo "$(BLUE)🐚 Opening shell in database container...$(NC)"
	./scripts/docker-run.sh shell postgres psql

##@ Testing and Development

test: ## Run tests in container
	@echo "$(GREEN)🧪 Running tests...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm test

test-watch: ## Run tests in watch mode
	@echo "$(GREEN)🧪 Running tests in watch mode...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm test:watch

lint: ## Run linting
	@echo "$(GREEN)🔍 Running linting...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm lint

format: ## Format code
	@echo "$(GREEN)💅 Formatting code...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm format

##@ Database Operations

db-migrate: ## Run database migrations
	@echo "$(GREEN)📊 Running database migrations...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm db:migrate

db-seed: ## Seed database with sample data
	@echo "$(GREEN)🌱 Seeding database...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm db:seed

db-reset: ## Reset database (migrate:reset)
	@echo "$(YELLOW)⚠️  Resetting database...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm db:migrate:reset

db-status: ## Check migration status
	@echo "$(BLUE)📊 Checking migration status...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm db:migrate:status

db-health: ## Check database health
	@echo "$(BLUE)❤️  Checking database health...$(NC)"
	./scripts/docker-run.sh exec web-ui pnpm db:health

db-backup: ## Create database backup
	@echo "$(GREEN)💾 Creating database backup...$(NC)"
	@mkdir -p backups
	docker compose exec postgres pg_dump -U postgres crewchief > backups/backup-$(shell date +%Y%m%d-%H%M%S).sql
	@echo "$(GREEN)✅ Backup created in backups/ directory$(NC)"

##@ Quick Commands

up: dev ## Alias for dev (start development environment)

down: stop ## Alias for stop

ps: status ## Alias for status

exec: ## Execute custom command in web-ui container (usage: make exec CMD="command")
	@if [ -z "$(CMD)" ]; then echo "$(YELLOW)Usage: make exec CMD=\"your command\"$(NC)"; exit 1; fi
	./scripts/docker-run.sh exec web-ui $(CMD)

##@ Health Checks

health: ## Check all service health
	@echo "$(BLUE)❤️  Checking service health...$(NC)"
	@echo "Web UI:"
	@curl -s http://localhost:3456/api/health | grep -q "ok" && echo "$(GREEN)✅ Web UI healthy$(NC)" || echo "$(YELLOW)⚠️  Web UI not responding$(NC)"
	@echo "Database:"
	@./scripts/docker-run.sh exec postgres pg_isready -U postgres -d crewchief >/dev/null 2>&1 && echo "$(GREEN)✅ Database healthy$(NC)" || echo "$(YELLOW)⚠️  Database not responding$(NC)"
	@echo "Redis:"
	@./scripts/docker-run.sh exec redis redis-cli ping >/dev/null 2>&1 && echo "$(GREEN)✅ Redis healthy$(NC)" || echo "$(YELLOW)⚠️  Redis not responding$(NC)"

##@ Environment Setup

env: ## Copy .env.example to .env if it doesn't exist
	@if [ ! -f .env ]; then \
		echo "$(GREEN)📝 Creating .env from .env.example...$(NC)"; \
		cp .env.example .env; \
		echo "$(YELLOW)⚠️  Please edit .env with your configuration$(NC)"; \
	else \
		echo "$(BLUE)ℹ️  .env file already exists$(NC)"; \
	fi

setup: env ## Initial setup (create .env and start development)
	@echo "$(GREEN)🎯 Running initial setup...$(NC)"
	@$(MAKE) dev-build

##@ Security

scan: ## Run security scan on images
	@echo "$(BLUE)🔍 Running security scan...$(NC)"
	@if command -v docker-scout >/dev/null 2>&1; then \
		docker scout quickview crewchief/web-ui:latest || echo "$(YELLOW)⚠️  Security scan failed or image not found$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  docker-scout not available$(NC)"; \
	fi

##@ Information

info: ## Show environment information
	@echo "$(BLUE)CrewChief Web UI Environment Information$(NC)"
	@echo "========================================"
	@echo "Docker version: $$(docker --version)"
	@echo "Docker Compose version: $$(docker compose version)"
	@echo "Project directory: $$(pwd)"
	@echo "Environment file: $$(if [ -f .env ]; then echo '✅ .env exists'; else echo '❌ .env missing'; fi)"
	@echo ""
	@echo "$(BLUE)Service URLs:$(NC)"
	@echo "Web UI: http://localhost:3456"
	@echo "Frontend Dev: http://localhost:3000"
	@echo "pgAdmin: http://localhost:8080 (dev-tools profile)"
	@echo "Redis Commander: http://localhost:8081 (dev-tools profile)"