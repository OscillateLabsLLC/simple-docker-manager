# Contributing to Simple Docker Manager

Thanks for your interest in contributing! This document provides guidelines for contributing to Simple Docker Manager.

## Development Setup

### Prerequisites

- Rust (latest stable version)
- Docker installed and running
- Access to Docker daemon (usually requires being in the `docker` group on Linux)
- [just](https://github.com/casey/just) command runner (recommended)
- Git

### Getting Started

1. Fork and clone the repository:

   ```bash
   git clone https://github.com/OscillateLabsLLC/simple-docker-manager
   cd simple-docker-manager
   ```

2. Install [just](https://github.com/casey/just) (task runner):

   ```bash
   # macOS
   brew install just
   # Linux
   cargo install just
   # Windows
   cargo install just
   ```

3. Copy the environment configuration:

   ```bash
   cp env.example .env
   ```

4. Build the project:

   ```bash
   cargo build
   ```

5. Run the development server:

   ```bash
   cargo run
   ```

   The application will be available at `http://localhost:3000`.

## Project Structure

```text
simple-docker-manager/
├── src/
│   ├── main.rs           # Application entry point with 12-Factor setup
│   ├── config.rs         # Environment-based configuration
│   ├── web.rs            # Web routes and handlers
│   ├── docker.rs         # Docker API integration
│   └── models.rs         # Data structures
├── templates/            # HTML templates
├── static/              # CSS, JavaScript, and static assets
├── Dockerfile           # Multi-stage Docker build
├── docker-compose.yml   # Compose configuration
├── docker-build.sh      # Build script with options
├── justfile             # Task runner commands
└── README.md           # Project documentation
```

## Making Changes

### Code Style

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` to catch common mistakes
- Use meaningful variable names
- Add comments for complex logic
- Document public APIs with doc comments

### Security

This project provides web-based control over Docker containers. Security is paramount:

- **Never** disable authentication in production
- **Always** validate user input
- **Review** Docker socket access patterns
- Run security checks before committing: `just security-all` or `just security-quick`
- See [SECURITY.md](SECURITY.md) for detailed security guidelines

### Testing

- Add tests for new functionality
- Ensure all tests pass: `cargo test`
- Run security checks: `just security-quick`
- Test coverage is tracked but no hard minimums — focus on testing critical paths

#### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Run tests with coverage (requires cargo-llvm-cov)
cargo llvm-cov --html
```

#### Test Organization

- **Unit tests**: In the same file as the code being tested, in a `#[cfg(test)]` module
- **Integration tests**: In the `tests/` directory for end-to-end scenarios
- **Mock external dependencies**: Docker API calls should be mockable for testing

### Commits

We use [Conventional Commits](https://www.conventionalcommits.org/) for automatic changelog generation:

```text
feat: add container log export functionality
fix: resolve authentication timeout issue
docs: update deployment instructions
test: add tests for metrics collection
chore: update dependencies
```

**Common prefixes:**
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `test:` - Test additions or modifications
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks
- `ci:` - CI/CD changes
- `build:` - Build system changes

### Pull Requests

1. Create a feature branch: `git checkout -b feat/my-feature`
2. Make your changes
3. Add tests
4. Run `cargo fmt` and `cargo clippy`
5. Run `cargo test`
6. Run `just security-quick` (or `just security-all` if you have all tools installed)
7. Commit with conventional commit messages
8. Push to your fork
9. Open a pull request

**PR Guidelines:**
- Keep PRs focused on a single concern
- Include tests for new functionality
- Update documentation as needed
- Ensure CI checks pass
- Respond to review feedback promptly

## Areas for Contribution

### High Priority

- [ ] Increase test coverage for core modules
- [ ] Add integration tests for web endpoints
- [ ] Improve error handling and user feedback
- [ ] Add container exec functionality
- [ ] Performance optimization for metrics collection

### Medium Priority

- [ ] Add support for Docker Compose projects
- [ ] Implement container search/filtering
- [ ] Add support for Docker networks and volumes
- [ ] Improve mobile responsiveness
- [ ] Add dark mode toggle

### Documentation

- [ ] Video tutorials for deployment
- [ ] More usage examples
- [ ] Troubleshooting guide expansion
- [ ] API documentation

## Running Security Checks

We provide a `justfile` for running security checks locally:

```bash
# Run all security checks (matches CI)
just

# Quick security check (Rust only)
just security-quick

# Individual checks
just rust-security      # Cargo audit + deny
just container-security # Docker security tests
just secret-scan       # GitLeaks (if installed)
just policy-check      # Security policy validation

# Install required security tools
just install-tools

# Show available commands
just --list
```

## Development Workflow

### Local Development

```bash
# Run with debug logging
SDM_LOG_LEVEL=debug cargo run

# Run with custom configuration
SDM_PORT=8080 SDM_METRICS_INTERVAL_SECONDS=10 cargo run

# Watch for changes and rebuild (requires cargo-watch)
cargo watch -x run
```

### Building for Release

```bash
# Build optimized binary
cargo build --release

# The binary will be in target/release/simple-docker-manager
./target/release/simple-docker-manager
```

### Docker Development

```bash
# Build Docker image
./docker-build.sh

# Build and run
./docker-build.sh --run

# Build with custom tag
./docker-build.sh --tag v1.0.0

# See all options
./docker-build.sh --help
```

## Architecture Decisions

### 12-Factor App Principles

Configuration is exclusively via environment variables to support containerized deployments and cloud-native patterns.

### Minimal Dependencies

We prioritize battle-tested, minimal dependencies to reduce attack surface and maintenance burden.

### Security-First Design

- Non-root container execution
- Read-only Docker socket mounting
- Scratch-based container images
- Argon2 password hashing
- Mandatory authentication by default

### Separation of Concerns

- HTML templates separate from Rust code
- Shared CSS for consistent design
- Clear module boundaries (config, web, docker, models)

## Questions?

- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- For security issues, see [SECURITY.md](SECURITY.md)

## Code of Conduct

Be respectful and constructive. We're building a tool that manages critical infrastructure — professionalism and clear communication are essential.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
