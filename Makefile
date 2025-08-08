# Distributed WAL Makefile

.PHONY: help build test clean docker-build docker-push helm-install helm-uninstall run-local

# Default target
help:
	@echo "Available targets:"
	@echo "  build         - Build the project"
	@echo "  test          - Run all tests"
	@echo "  clean         - Clean build artifacts"
	@echo "  docker-build  - Build Docker image"
	@echo "  docker-push   - Push Docker image"
	@echo "  helm-install  - Install Helm chart"
	@echo "  helm-uninstall- Uninstall Helm chart"
	@echo "  run-local     - Run local cluster"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test
	cargo test --test wal_test
	cargo test --test distributed_test

# Clean build artifacts
clean:
	cargo clean
	rm -rf /tmp/walrus*

# Build Docker image
docker-build:
	docker build -t walrus:latest .

# Push Docker image (update registry as needed)
docker-push:
	docker tag walrus:latest your-registry/walrus:latest
	docker push your-registry/walrus:latest

# Install Helm chart
helm-install:
	helm install walrus ./helm/walrus \
		--set image.repository=your-registry/walrus \
		--set image.tag=latest

# Uninstall Helm chart
helm-uninstall:
	helm uninstall walrus

# Run local cluster (requires 3 terminals)
run-local:
	@echo "Starting local cluster..."
	@echo "Terminal 1: ./target/release/walrus --node-id node-1 --bind-addr 127.0.0.1:8080"
	@echo "Terminal 2: ./target/release/walrus --node-id node-2 --bind-addr 127.0.0.1:8081"
	@echo "Terminal 3: ./target/release/walrus --node-id node-3 --bind-addr 127.0.0.1:8082"

# Development helpers
dev-build:
	cargo build

dev-run:
	cargo run -- --node-id dev-node --bind-addr 127.0.0.1:8080

# Kubernetes helpers
k8s-deploy:
	kubectl apply -f k8s/

k8s-delete:
	kubectl delete -f k8s/

# Monitoring
logs:
	kubectl logs -f deployment/walrus

status:
	kubectl get pods -l app=walrus
	kubectl get services -l app=walrus

# Performance testing
bench:
	cargo bench

# Documentation
docs:
	cargo doc --open

# Format code
fmt:
	cargo fmt

# Lint code
lint:
	cargo clippy

# Security audit
audit:
	cargo audit


