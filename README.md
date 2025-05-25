# 🐳 Simple Docker Manager

A beautiful, lightweight Docker container management service built with Rust, featuring real-time metrics visualization and an intuitive web interface.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)
![JavaScript](https://img.shields.io/badge/javascript-%23323330.svg?style=for-the-badge&logo=javascript&logoColor=%23F7DF1E)

## ✨ Features

### 🚀 Container Management

- **View Running Containers**: See all your running containers at a glance
- **Start/Stop/Restart**: Easy one-click container controls
- **Launch New Containers**: Start new containers from available images
- **Real-time Status**: Live updates of container states

### 📊 Advanced Metrics Dashboard

- **Real-time Monitoring**: Live container resource usage tracking
- **Beautiful Charts**: Interactive charts powered by Chart.js
- **System Overview**: Docker version, container counts, and image statistics
- **Resource Tracking**:
  - CPU usage percentage per container
  - Memory usage and limits
  - Network I/O (RX/TX bytes)
  - Disk I/O (read/write bytes)
  - Process counts (PIDs)

### 🎨 Modern UI

- **Responsive Design**: Works on desktop and mobile
- **Glass Morphism**: Beautiful gradient backgrounds with frosted glass effects
- **Smooth Animations**: Hover effects and transitions
- **Clean Architecture**: HTML templates separated from Rust code with shared CSS

### 🏭 Production Ready

- **12-Factor App**: Environment-based configuration
- **Graceful Shutdown**: Proper signal handling for containers
- **Health Checks**: Built-in health and readiness endpoints
- **Structured Logging**: Configurable log levels and output
- **Zero Downtime**: Hot configuration reloads via environment

## 🛠️ Technology Stack

- **Backend**: Rust with Axum web framework
- **Docker Integration**: Bollard (Docker API client)
- **Frontend**: Vanilla JavaScript with Chart.js
- **Styling**: Modern CSS with gradients and animations
- **File Serving**: Static assets served efficiently
- **Configuration**: Environment variables with .env support

## 🚀 Quick Start

### Prerequisites

- Rust (latest stable version)
- Docker installed and running
- Access to Docker daemon (usually requires being in the `docker` group on Linux)

### Installation

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd simple-docker-manager
   ```

2. **Configure the application (optional)**

   ```bash
   # Copy the example configuration
   cp env.example .env

   # Edit configuration as needed
   vim .env
   ```

3. **Build the project**

   ```bash
   cargo build --release
   ```

4. **Run the service**

   ```bash
   # With default configuration
   cargo run

   # Or with custom environment
   SDM_PORT=8080 SDM_LOG_LEVEL=debug cargo run
   ```

5. **Open your browser**
   Navigate to `http://localhost:3000` (or your configured port)

## 📁 Project Structure

```
simple-docker-manager/
├── src/
│   ├── main.rs           # Application entry point with 12-Factor setup
│   ├── config.rs         # Environment-based configuration
│   ├── web.rs            # Web routes and handlers
│   ├── docker.rs         # Docker API integration
│   └── models.rs         # Data structures
├── templates/
│   ├── dashboard.html    # Metrics dashboard template
│   └── management.html   # Container management template
├── static/
│   ├── styles.css        # Shared CSS styles
│   └── dashboard.js      # Frontend JavaScript
├── env.example          # Configuration template
├── Cargo.toml           # Rust dependencies
└── README.md           # This file
```

## 🌐 API Endpoints

### Web Interface

- `GET /` - Main container management interface
- `GET /metrics` - Real-time metrics dashboard

### Health & Monitoring

- `GET /health` - Health check endpoint (returns 200/503 with Docker status)
- `GET /ready` - Readiness probe endpoint (always returns 200 when server is up)

### Container Management

- `POST /start-image` - Start a new container from an image
- `POST /start/:id` - Start a stopped container
- `POST /stop/:id` - Stop a running container
- `POST /restart/:id` - Restart a container

### API Endpoints

- `GET /api/metrics` - JSON metrics data for all containers

### Static Assets

- `/static/*` - CSS, JavaScript, and other static files

## ⚙️ Configuration

The application follows the [12-Factor App](https://12factor.net/) methodology and is configured entirely through environment variables.

### Environment Variables

All configuration is done via environment variables prefixed with `SDM_`:

| Variable                       | Default       | Description                                           |
| ------------------------------ | ------------- | ----------------------------------------------------- |
| `SDM_HOST`                     | `0.0.0.0`     | Server bind address                                   |
| `SDM_PORT`                     | `3000`        | Server port                                           |
| `SDM_LOG_LEVEL`                | `info`        | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `SDM_DOCKER_SOCKET`            | auto-detected | Docker socket path                                    |
| `SDM_METRICS_INTERVAL_SECONDS` | `5`           | Metrics update interval                               |
| `SDM_METRICS_HISTORY_LIMIT`    | `20`          | Max metrics history points                            |
| `SDM_SHUTDOWN_TIMEOUT_SECONDS` | `30`          | Graceful shutdown timeout                             |

### Configuration Methods

#### 1. Environment Variables (Recommended for Production)

```bash
export SDM_PORT=8080
export SDM_LOG_LEVEL=warn
cargo run --release
```

#### 2. .env File (Recommended for Development)

```bash
# Copy the example
cp env.example .env

# Edit your configuration
echo "SDM_PORT=8080" >> .env
echo "SDM_LOG_LEVEL=debug" >> .env

# Run (automatically loads .env)
cargo run
```

#### 3. Runtime Override

```bash
# Override specific values at runtime
SDM_PORT=9000 SDM_LOG_LEVEL=trace cargo run
```

### Container Deployment

For Docker deployment, pass environment variables:

```bash
docker run -e SDM_PORT=8080 -e SDM_LOG_LEVEL=warn your-image
```

For Kubernetes, use ConfigMaps and Secrets:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: docker-manager-config
data:
  SDM_LOG_LEVEL: "info"
  SDM_METRICS_INTERVAL_SECONDS: "10"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: docker-manager
spec:
  template:
    spec:
      containers:
        - name: docker-manager
          envFrom:
            - configMapRef:
                name: docker-manager-config
          ports:
            - containerPort: 3000
          livenessProbe:
            httpGet:
              path: /health
              port: 3000
          readinessProbe:
            httpGet:
              path: /ready
              port: 3000
```

## 📊 Metrics Data Structure

The `/api/metrics` endpoint returns comprehensive container metrics:

```json
{
  "system": {
    "timestamp": "2024-01-01T12:00:00Z",
    "total_containers": 5,
    "running_containers": 3,
    "total_images": 10,
    "docker_version": "24.0.0"
  },
  "containers": [
    {
      "container_id": "abc123...",
      "container_name": "my-app",
      "timestamp": "2024-01-01T12:00:00Z",
      "cpu_usage_percent": 15.5,
      "memory_usage_mb": 256.0,
      "memory_limit_mb": 512.0,
      "memory_usage_percent": 50.0,
      "network_rx_bytes": 1024000,
      "network_tx_bytes": 512000,
      "block_read_bytes": 204800,
      "block_write_bytes": 102400,
      "pids": 25
    }
  ]
}
```

## 🎯 Key Features Explained

### Real-time Metrics Collection

The service continuously polls Docker for container statistics, calculating:

- **CPU Usage**: Percentage based on system CPU time
- **Memory Usage**: Current usage vs. container limits
- **Network Traffic**: Bytes received and transmitted
- **Disk I/O**: Read and write operations
- **Process Count**: Number of running processes

### Responsive Charts

- **Live Updates**: Charts update every 5 seconds (configurable)
- **History Tracking**: Maintains configurable data points for trend visualization
- **Multiple Metrics**: Separate charts for CPU, memory, network, and disk
- **Color Coding**: Unique colors per container for easy identification

### Clean Architecture

- **Separation of Concerns**: HTML templates separate from Rust code
- **Shared Styling**: Single CSS file for consistent design across views
- **Static Asset Serving**: Efficient file serving for CSS/JS
- **Type Safety**: Strong typing with Rust's type system
- **Error Handling**: Graceful error handling and user feedback

## 🔧 Operations

### Health Monitoring

The application provides standard health check endpoints:

- **Health Check** (`/health`): Returns 200 if Docker is accessible, 503 otherwise
- **Readiness Check** (`/ready`): Returns 200 if the server can handle requests

### Graceful Shutdown

The application handles shutdown signals gracefully:

- **SIGTERM**: Kubernetes/container termination
- **SIGINT**: Ctrl+C for development
- **Configurable timeout**: Prevents hanging shutdowns

### Logging

Structured logging with configurable levels:

```bash
# Development
SDM_LOG_LEVEL=debug cargo run

# Production
SDM_LOG_LEVEL=warn cargo run

# Environment-based (respects RUST_LOG if SDM_LOG_LEVEL not set)
RUST_LOG=simple_docker_manager=debug cargo run
```

## 🐛 Troubleshooting

### Common Issues

#### Application Won't Start

1. **Port in use**: Change the port via environment variable

   ```bash
   SDM_PORT=8080 cargo run
   ```

2. **Permission denied**: Ensure Docker access

   ```bash
   # Add user to docker group (Linux)
   sudo usermod -aG docker $USER
   # Log out and back in
   ```

3. **Docker not accessible**: Check Docker daemon
   ```bash
   docker ps  # Should work without sudo
   ```

#### Configuration Issues

1. **Check current configuration**: The app logs its configuration on startup
2. **Validate environment variables**: Ensure proper naming (`SDM_` prefix)
3. **Check .env file**: Ensure it's in the working directory

#### Performance Issues

1. **Adjust metrics interval**:

   ```bash
   SDM_METRICS_INTERVAL_SECONDS=10 cargo run
   ```

2. **Reduce history retention**:
   ```bash
   SDM_METRICS_HISTORY_LIMIT=10 cargo run
   ```

#### Container Access Issues

- **Docker socket**: The app auto-detects Docker socket location
- **Custom socket**: Set `SDM_DOCKER_SOCKET=/path/to/docker.sock`
- **Remote Docker**: Currently not supported (local socket only)

### Getting Help

1. **Enable debug logging**: `SDM_LOG_LEVEL=debug`
2. **Check health endpoint**: `curl http://localhost:3000/health`
3. **Verify Docker access**: `docker ps` should work for the same user

## 🚀 Future Enhancements

- **Container Logs**: View real-time container logs
- **Image Management**: Pull, build, and manage Docker images
- **Container Shell**: Execute commands in running containers
- **Alerts**: Set up alerts for resource thresholds
- **Historical Data**: Store metrics in a database for long-term analysis
- **Multi-host Support**: Manage containers across multiple Docker hosts
- **RBAC**: Role-based access control
- **API Authentication**: Secure API endpoints

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📦 Releases

This project uses automated releases powered by [release-please](https://github.com/googleapis/release-please).

### How Releases Work

1. **Conventional Commits**: Use conventional commit messages for automatic changelog generation:

   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `refactor:` for code refactoring
   - `perf:` for performance improvements
   - `test:` for test changes
   - `chore:` for maintenance tasks

2. **Automated Versioning**: When commits are pushed to `main`, release-please automatically:

   - Analyzes commit messages
   - Updates the version in `Cargo.toml`
   - Generates a changelog
   - Creates a release PR

3. **Binary Publishing**: When a release PR is merged:
   - Cross-platform binaries are built automatically
   - Binaries are attached to the GitHub release
   - SHA256 checksums are generated for verification

### Available Binaries

Pre-built binaries are available for each release:

- **Linux x86_64**: `simple-docker-manager-linux-amd64.tar.gz`
- **macOS Intel**: `simple-docker-manager-macos-amd64.tar.gz`
- **macOS Apple Silicon**: `simple-docker-manager-macos-arm64.tar.gz`

### Installation from Release

```bash
# Download and extract (replace with latest version)
curl -L https://github.com/YOUR_USERNAME/simple-docker-manager/releases/download/v0.1.0/simple-docker-manager-linux-amd64.tar.gz | tar -xz

# Make executable and move to PATH
chmod +x simple-docker-manager
sudo mv simple-docker-manager /usr/local/bin/

# Run
simple-docker-manager
```

## 📄 License

This project is open source and available under the [MIT License](LICENSE).

## 🙏 Acknowledgments

- [Bollard](https://github.com/fussybeaver/bollard) - Excellent Docker API client for Rust
- [Axum](https://github.com/tokio-rs/axum) - Modern, ergonomic web framework
- [Chart.js](https://www.chartjs.org/) - Beautiful charts made simple
- [Docker](https://www.docker.com/) - Container platform that makes this all possible
- [12-Factor App](https://12factor.net/) - Methodology for building modern applications

---

**Built with ❤️ and Rust** 🦀
