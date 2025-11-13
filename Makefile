.PHONY: help docker-build docker-run docker-push docker-clean test lint fmt fmt-check format check pre-commit

# Default target
.DEFAULT_GOAL := help

# Variables
DOCKER_IMAGE := locai-server
DOCKER_TAG := latest
# Extract version from Cargo.toml workspace package section
VERSION := $(shell grep '^version = ' Cargo.toml | head -n 1 | sed 's/version = "\(.*\)"/\1/')
REGISTRY := # Set to your registry, e.g., ghcr.io/username

# Help target
help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Version: $(VERSION)'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# Docker targets
docker-build: ## Build server Docker image
	@echo "Building Docker image..."
	DOCKER_BUILDKIT=1 docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .
	@echo "Tagging with version $(VERSION)..."
	docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) $(DOCKER_IMAGE):$(VERSION)

docker-build-cli: ## Build CLI Docker image
	@echo "Building CLI Docker image..."
	DOCKER_BUILDKIT=1 docker build -f Dockerfile.cli -t locai-cli:$(DOCKER_TAG) .
	@echo "Tagging with version $(VERSION)..."
	docker tag locai-cli:$(DOCKER_TAG) locai-cli:$(VERSION)

docker-build-all: docker-build docker-build-cli ## Build both server and CLI images

docker-run: ## Run Docker container locally
	@echo "Starting Locai server..."
	docker run -d \
		--name locai-server \
		-p 3000:3000 \
		-v locai-data:/data \
		-e RUST_LOG=info,locai=debug \
		$(DOCKER_IMAGE):$(DOCKER_TAG)
	@echo "Server running at http://localhost:3000"
	@echo "API docs at http://localhost:3000/docs"

docker-stop: ## Stop running container
	@echo "Stopping Locai server..."
	docker stop locai-server || true
	docker rm locai-server || true

docker-logs: ## View container logs
	docker logs -f locai-server

docker-shell: ## Open shell in running container
	docker exec -it locai-server bash

docker-push: ## Push image to registry
ifndef REGISTRY
	$(error REGISTRY is not set. Use: make docker-push REGISTRY=ghcr.io/username)
endif
	@echo "Tagging image for registry..."
	docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) $(REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) $(REGISTRY)/$(DOCKER_IMAGE):$(VERSION)
	@echo "Pushing to $(REGISTRY)..."
	docker push $(REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	docker push $(REGISTRY)/$(DOCKER_IMAGE):$(VERSION)

docker-clean: ## Clean up Docker images and containers
	@echo "Cleaning up Docker resources..."
	docker stop locai-server 2>/dev/null || true
	docker rm locai-server 2>/dev/null || true
	docker rmi $(DOCKER_IMAGE):$(DOCKER_TAG) 2>/dev/null || true
	docker rmi $(DOCKER_IMAGE):$(VERSION) 2>/dev/null || true
	docker rmi locai-cli:$(DOCKER_TAG) 2>/dev/null || true
	docker rmi locai-cli:$(VERSION) 2>/dev/null || true
	@echo "Cleanup complete"

# Docker Compose targets
compose-up: ## Start services with docker-compose
	docker-compose up -d

compose-down: ## Stop services with docker-compose
	docker-compose down

compose-logs: ## View docker-compose logs
	docker-compose logs -f

compose-restart: ## Restart services
	docker-compose restart

compose-rebuild: ## Rebuild and restart services
	docker-compose up -d --build

# CLI-specific compose targets
cli-run: ## Run CLI command (usage: make cli-run CMD="memory list")
	docker-compose -f docker-compose.cli.yml run --rm locai-cli $(CMD)

cli-shell: ## Open shell with CLI available
	docker-compose -f docker-compose.cli.yml run --rm locai-cli bash

cli-up: ## Start server and CLI setup
	docker-compose -f docker-compose.cli.yml up -d locai-server

# Development targets
dev: ## Run development server locally (no Docker)
	cargo run --bin locai-server

test: ## Run tests
	cargo test --workspace

test-integration: ## Run integration tests
	cargo test --package locai-server --test integration_tests

test-docker: ## Test Docker build locally
	@echo "Testing Docker build..."
	DOCKER_BUILDKIT=1 docker build -t $(DOCKER_IMAGE):test .
	@echo "Testing container startup..."
	docker run --rm --name locai-test -p 3001:3000 -e RUST_LOG=debug $(DOCKER_IMAGE):test &
	@sleep 5
	@echo "Testing health endpoint..."
	curl -f http://localhost:3001/api/health || (echo "Health check failed" && exit 1)
	@echo "Stopping test container..."
	docker stop locai-test
	@echo "Docker test passed!"

lint: ## Run linter
	cargo clippy --all-features --workspace -- -D warnings

fmt: ## Format all code with rustfmt
	cargo fmt --all

format: fmt ## Alias for fmt (format all code)

fmt-check: ## Check code formatting without modifying files
	cargo fmt --all -- --check

# Build targets
build: ## Build release binary
	cargo build --release --bin locai-server

build-debug: ## Build debug binary
	cargo build --bin locai-server

clean: ## Clean build artifacts
	cargo clean

# Multi-architecture build
docker-buildx: ## Build multi-architecture image (requires buildx)
	@echo "Creating builder..."
	docker buildx create --use --name locai-builder || true
	@echo "Building for multiple architectures..."
	docker buildx build \
		--platform linux/amd64,linux/arm64 \
		-t $(DOCKER_IMAGE):$(DOCKER_TAG) \
		--push \
		.

# Utility targets
check: ## Run all checks (fmt-check, lint, test)
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test

pre-commit: fmt lint fmt-check ## Format code and run checks (useful before committing)
	@echo "Pre-commit checks complete!"

size: ## Show Docker image size
	@docker images $(DOCKER_IMAGE) --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"

inspect: ## Inspect Docker image layers
	docker history $(DOCKER_IMAGE):$(DOCKER_TAG)

# CI/CD helpers
ci-build: ## Build for CI/CD
	DOCKER_BUILDKIT=1 docker build \
		--cache-from $(DOCKER_IMAGE):$(DOCKER_TAG) \
		-t $(DOCKER_IMAGE):$(DOCKER_TAG) \
		.

ci-test: ## Run tests in CI/CD
	cargo test --workspace --release

# Documentation
docs: ## Generate and open documentation
	cargo doc --workspace --no-deps --open

docs-check: ## Check documentation
	cargo doc --workspace --no-deps

