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
- **Clean Architecture**: HTML templates separated from Rust code

## 🛠️ Technology Stack

- **Backend**: Rust with Axum web framework
- **Docker Integration**: Bollard (Docker API client)
- **Frontend**: Vanilla JavaScript with Chart.js
- **Styling**: Modern CSS with gradients and animations
- **File Serving**: Static assets served efficiently

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

2. **Build the project**

   ```bash
   cargo build --release
   ```

3. **Run the service**

   ```bash
   cargo run
   ```

4. **Open your browser**
   Navigate to `http://localhost:3000`

## 📁 Project Structure

```
simple-docker-manager/
├── src/
│   ├── main.rs           # Application entry point
│   ├── web.rs            # Web routes and handlers
│   ├── docker.rs         # Docker API integration
│   └── models.rs         # Data structures
├── templates/
│   └── dashboard.html    # Metrics dashboard template
├── static/
│   └── dashboard.js      # Frontend JavaScript
├── Cargo.toml           # Rust dependencies
└── README.md           # This file
```

## 🌐 API Endpoints

### Web Interface

- `GET /` - Main container management interface
- `GET /metrics` - Real-time metrics dashboard

### Container Management

- `POST /start-image` - Start a new container from an image
- `POST /start/:id` - Start a stopped container
- `POST /stop/:id` - Stop a running container
- `POST /restart/:id` - Restart a container

### API Endpoints

- `GET /api/metrics` - JSON metrics data for all containers

### Static Assets

- `/static/*` - CSS, JavaScript, and other static files

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

- **Live Updates**: Charts update every 5 seconds
- **History Tracking**: Maintains 20 data points for trend visualization
- **Multiple Metrics**: Separate charts for CPU, memory, network, and disk
- **Color Coding**: Unique colors per container for easy identification

### Clean Architecture

- **Separation of Concerns**: HTML templates separate from Rust code
- **Static Asset Serving**: Efficient file serving for CSS/JS
- **Type Safety**: Strong typing with Rust's type system
- **Error Handling**: Graceful error handling and user feedback

## 🔧 Configuration

### Changing the Port

Modify the port in `src/main.rs`:

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:8080") // Change from 3000
```

### Docker Connection

The service connects to Docker using the default Docker daemon socket. For custom configurations, modify the Docker connection in `src/docker.rs`.

## 🐛 Troubleshooting

### Port Already in Use

If you get "Address already in use" error:

```bash
# Find and kill the process using port 3000
lsof -ti:3000 | xargs kill -9
```

### Docker Permission Issues

On Linux, add your user to the docker group:

```bash
sudo usermod -aG docker $USER
# Then log out and back in
```

### No Containers Showing

- Ensure Docker is running: `docker ps`
- Check Docker daemon accessibility
- Verify container status: some containers might be stopped

## 🚀 Future Enhancements

- **Container Logs**: View real-time container logs
- **Image Management**: Pull, build, and manage Docker images
- **Container Shell**: Execute commands in running containers
- **Alerts**: Set up alerts for resource thresholds
- **Historical Data**: Store metrics in a database for long-term analysis
- **Multi-host Support**: Manage containers across multiple Docker hosts

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📄 License

This project is open source and available under the [MIT License](LICENSE).

## 🙏 Acknowledgments

- [Bollard](https://github.com/fussybeaver/bollard) - Excellent Docker API client for Rust
- [Axum](https://github.com/tokio-rs/axum) - Modern, ergonomic web framework
- [Chart.js](https://www.chartjs.org/) - Beautiful charts made simple
- [Docker](https://www.docker.com/) - Container platform that makes this all possible

---

**Built with ❤️ and Rust** 🦀
