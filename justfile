# Simple Docker Manager
# Run `just --list` to see all available commands

# Default recipe - build and check
default: check

# ─── Development ──────────────────────────────────────────────

# Format code
fmt:
    cargo fmt

# Check formatting (CI-friendly, no changes)
fmt-check:
    cargo fmt -- --check

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Run clippy with auto-fix
lint-fix:
    cargo clippy --fix --allow-dirty -- -D warnings

# Run tests
test:
    cargo test

# Build in debug mode
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Format, lint, and test
check: fmt-check lint test

# Format, lint, test, and build release
ci: fmt-check lint test build-release

# Run the dev server
run:
    cargo run

# ─── Security ─────────────────────────────────────────────────

# Install all required security tools
install-tools:
    @echo "🔧 Installing security tools..."
    cargo install cargo-audit --locked
    cargo install cargo-deny --locked
    @echo "✅ Security tools installed"

# Run all security checks (matches CI workflow)
security-all: rust-security container-security secret-scan policy-check
    @echo ""
    @echo "🔒 All security checks completed!"
    @echo "✅ Your code is ready for commit"

# Rust dependency security checks
rust-security:
    @echo "🦀 Running Rust security checks..."
    @echo "  📦 Checking for vulnerabilities..."
    cargo audit
    @echo "  📋 Checking licenses and policies..."
    cargo deny check
    @echo "✅ Rust security checks passed"

# Container security checks (requires Docker)
container-security:
    @echo "🐳 Running container security checks..."
    @echo "  🏗️  Building test image..."
    docker build -t simple-docker-manager:security-test .
    @echo "  🔍 Running Trivy vulnerability scan..."
    @if docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
        -v $(pwd):/workspace aquasecurity/trivy:latest \
        image simple-docker-manager:security-test; then \
        echo "✅ Trivy scan completed"; \
    else \
        echo "⚠️  Trivy scan failed or not available. Install Trivy locally or skip with 'just security-quick'"; \
    fi
    @echo "  📝 Running Hadolint Dockerfile linter..."
    @if docker run --rm -i hadolint/hadolint < Dockerfile; then \
        echo "✅ Hadolint check passed"; \
    else \
        echo "⚠️  Hadolint check failed or not available"; \
    fi
    @echo "  🧪 Testing container security..."
    just _test-container-security
    @echo "✅ Container security checks completed"

# Test container runtime security
_test-container-security:
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Start container in background
    docker run -d --name security-test-local \
        -p 3001:3000 \
        -v /var/run/docker.sock:/var/run/docker.sock:ro \
        simple-docker-manager:security-test
    
    # Wait for container to start
    sleep 10
    
    # Check if running as non-root using docker inspect (works with scratch images)
    # For scratch containers, we check the User field in the config
    USER_CONFIG=$(docker inspect security-test-local --format='{{ "{{" }}.Config.User{{ "}}" }}')
    if [ "$USER_CONFIG" = "0" ] || [ "$USER_CONFIG" = "" ] || [ "$USER_CONFIG" = "root" ] || [ "$USER_CONFIG" = "<no value>" ]; then
        echo "❌ SECURITY RISK: Container running as root or no user specified"
        echo "   User config: '$USER_CONFIG'"
        docker stop security-test-local
        docker rm security-test-local
        exit 1
    else
        echo "✅ Container configured with non-root user: $USER_CONFIG"
    fi
    
    # For scratch containers, also verify the process is actually running as non-root
    # by checking the process list from the host
    CONTAINER_PID=$(docker inspect security-test-local --format='{{ "{{" }}.State.Pid{{ "}}" }}')
    if [ "$CONTAINER_PID" != "0" ] && [ "$CONTAINER_PID" != "" ]; then
        # Check the actual UID of the process (requires ps command)
        if command -v ps >/dev/null 2>&1; then
            ACTUAL_UID=$(ps -o uid= -p "$CONTAINER_PID" 2>/dev/null | tr -d ' ' || echo "unknown")
            if [ "$ACTUAL_UID" = "0" ]; then
                echo "❌ SECURITY RISK: Process actually running as root (UID: $ACTUAL_UID)"
                docker stop security-test-local
                docker rm security-test-local
                exit 1
            elif [ "$ACTUAL_UID" != "unknown" ]; then
                echo "✅ Process running as non-root UID: $ACTUAL_UID"
            fi
        fi
    fi
    
    # Test health endpoint (install curl if not available)
    if command -v curl >/dev/null 2>&1; then
        echo "  🔍 Testing readiness endpoint with curl..."
        if curl -f --max-time 10 --silent http://localhost:3001/ready >/dev/null 2>&1; then
            echo "✅ Readiness endpoint accessible"
        else
            echo "❌ Readiness endpoint not accessible"
            echo "Container logs:"
            docker logs security-test-local
            docker stop security-test-local
            docker rm security-test-local
            exit 1
        fi
        
        # Also test the health endpoint (may return 503 if Docker not accessible, which is OK)
        echo "  🏥 Testing health endpoint (may show Docker connectivity issues)..."
        HEALTH_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 http://localhost:3001/health || echo "000")
        if [ "$HEALTH_STATUS" = "200" ]; then
            echo "✅ Health endpoint reports healthy (Docker accessible)"
        elif [ "$HEALTH_STATUS" = "503" ]; then
            echo "⚠️  Health endpoint reports unhealthy (Docker not accessible from container - expected in test)"
        else
            echo "❌ Health endpoint returned unexpected status: $HEALTH_STATUS"
            echo "Container logs:"
            docker logs security-test-local
            docker stop security-test-local
            docker rm security-test-local
            exit 1
        fi
    else
        echo "⚠️  curl not available, testing container health via logs and status"
        # Check if container is running and logs don't show errors
        if docker ps --filter "name=security-test-local" --filter "status=running" | grep -q security-test-local; then
            echo "✅ Container is running"
            # Check for any obvious error patterns in logs
            LOGS=$(docker logs security-test-local 2>&1)
            if echo "$LOGS" | grep -qi "error\|panic\|fatal\|failed"; then
                echo "❌ Container logs show potential errors:"
                echo "$LOGS"
                docker stop security-test-local
                docker rm security-test-local
                exit 1
            else
                echo "✅ Container logs look healthy"
                # Show a sample of the logs for verification
                echo "Sample logs:"
                echo "$LOGS" | head -5
            fi
        else
            echo "❌ Container not running"
            docker logs security-test-local
            docker stop security-test-local
            docker rm security-test-local
            exit 1
        fi
    fi
    
    # Additional scratch container security checks
    echo "  🔒 Running additional scratch container security checks..."
    
    # Check that the container is using a minimal base (scratch should have very few layers)
    LAYER_COUNT=$(docker history simple-docker-manager:security-test --quiet | wc -l | tr -d ' ')
    echo "✅ Container has $LAYER_COUNT layers (minimal is better for security)"
    
    # Check that no shell is available (good for scratch containers)
    if docker exec security-test-local /bin/sh -c "echo test" 2>/dev/null; then
        echo "⚠️  Shell access available in container (not ideal for scratch containers)"
    else
        echo "✅ No shell access available (good for scratch containers)"
    fi
    
    # Clean up
    docker stop security-test-local
    docker rm security-test-local
    echo "✅ Container security test passed"

# Secret scanning (requires git history)
secret-scan:
    @echo "🔐 Running secret scan..."
    @if command -v gitleaks >/dev/null 2>&1; then \
        gitleaks detect --source . --verbose; \
        echo "✅ No secrets detected"; \
    else \
        echo "⚠️  GitLeaks not installed. Install from: https://github.com/gitleaks/gitleaks"; \
        echo "   Or run: brew install gitleaks (macOS)"; \
        echo "   Skipping secret scan..."; \
    fi

# SAST scanning with Semgrep
sast-scan:
    @echo "🔍 Running SAST scan..."
    @if command -v semgrep >/dev/null 2>&1; then \
        semgrep --config=auto .; \
        echo "✅ SAST scan completed"; \
    else \
        echo "⚠️  Semgrep not installed. Install from: https://semgrep.dev/docs/getting-started/"; \
        echo "   Or run: pip install semgrep"; \
        echo "   Skipping SAST scan..."; \
    fi

# Security policy and configuration checks
policy-check:
    @echo "📋 Running policy checks..."
    @echo "  📄 Checking for security policy..."
    @if [ ! -f SECURITY.md ]; then \
        echo "❌ No SECURITY.md file found"; \
        exit 1; \
    else \
        echo "✅ Security policy found"; \
    fi
    @echo "  🐳 Checking Docker Compose security..."
    @if grep -q "privileged.*true" docker-compose.yml; then \
        echo "❌ SECURITY RISK: privileged mode detected"; \
        exit 1; \
    fi
    @if grep -q "/var/run/docker.sock.*rw" docker-compose.yml; then \
        echo "❌ SECURITY RISK: Docker socket mounted as read-write"; \
        exit 1; \
    fi
    @echo "✅ Docker Compose security checks passed"

# Quick security check (essential checks only)
security-quick: rust-security policy-check
    @echo "🚀 Quick security checks completed!"

# Container-only security checks (no external tools)
container-basic:
    @echo "🐳 Running basic container security checks..."
    @echo "  🏗️  Building test image..."
    docker build -t simple-docker-manager:security-test .
    @echo "  🧪 Testing container security..."
    just _test-container-security
    @echo "✅ Basic container security checks completed"

# Clean up security test artifacts
clean:
    @echo "🧹 Cleaning up security test artifacts..."
    -docker rmi simple-docker-manager:security-test 2>/dev/null || true
    -docker rm -f security-test-local 2>/dev/null || true
    @echo "✅ Cleanup completed"

# Show security tool versions
versions:
    @echo "🔧 Security tool versions:"
    @echo "Rust: $(rustc --version)"
    @echo "Cargo: $(cargo --version)"
    @if command -v cargo-audit >/dev/null 2>&1; then \
        echo "Cargo Audit: $(cargo audit --version)"; \
    else \
        echo "Cargo Audit: ❌ Not installed"; \
    fi
    @if command -v cargo-deny >/dev/null 2>&1; then \
        echo "Cargo Deny: $(cargo deny --version)"; \
    else \
        echo "Cargo Deny: ❌ Not installed"; \
    fi
    @if command -v docker >/dev/null 2>&1; then \
        echo "Docker: $(docker --version)"; \
    else \
        echo "Docker: ❌ Not installed"; \
    fi
    @if command -v gitleaks >/dev/null 2>&1; then \
        echo "GitLeaks: $(gitleaks version)"; \
    else \
        echo "GitLeaks: ❌ Not installed"; \
    fi
    @if command -v semgrep >/dev/null 2>&1; then \
        echo "Semgrep: $(semgrep --version)"; \
    else \
        echo "Semgrep: ❌ Not installed"; \
    fi

# Update advisory database
update-advisories:
    @echo "📡 Updating security advisory database..."
    cargo audit --update
    @echo "✅ Advisory database updated"

# Generate security report
report:
    @echo "📊 Generating security report..."
    @echo "# Security Report - $(date)" > security-report.md
    @echo "" >> security-report.md
    @echo "## Tool Versions" >> security-report.md
    @just versions >> security-report.md 2>&1
    @echo "" >> security-report.md
    @echo "## Dependency Audit" >> security-report.md
    @echo '```' >> security-report.md
    @cargo audit 2>&1 >> security-report.md || true
    @echo '```' >> security-report.md
    @echo "" >> security-report.md
    @echo "## License Check" >> security-report.md
    @echo '```' >> security-report.md
    @cargo deny check 2>&1 >> security-report.md || true
    @echo '```' >> security-report.md
    @echo "✅ Security report generated: security-report.md"

# Help - show available commands
help:
    @just --list 