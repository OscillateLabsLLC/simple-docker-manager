let charts = {};
let metricsHistory = [];
let configLoaded = false;
let config = {
  metrics_interval_seconds: 5,
  metrics_history_limit: 20,
  max_chart_containers: 5, // Default value, will be overridden by backend config
};

async function fetchConfig() {
  try {
    const response = await fetch("/api/config", {
      credentials: "same-origin",
    });
    if (!response.ok) throw new Error("Failed to fetch config");
    const config = await response.json();

    // Store config globally
    window.dashboardConfig = config;
    configLoaded = true;
  } catch (error) {
    console.error("Error fetching config:", error);
    configLoaded = true; // Use defaults if config fails to load
  }
}

async function fetchMetrics() {
  try {
    const response = await fetch("/api/metrics", {
      credentials: "same-origin",
    });
    if (!response.ok) throw new Error("Failed to fetch metrics");
    return await response.json();
  } catch (error) {
    console.error("Error fetching metrics:", error);
    document.getElementById("error").style.display = "block";
    document.getElementById("error").textContent =
      "Failed to load metrics: " + error.message;
    return null;
  }
}

function formatBytes(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

function updateSystemStats(systemMetrics) {
  const statsContainer = document.getElementById("systemStats");
  statsContainer.innerHTML = `
        <div class="stat-item">
            <span class="stat-value">${systemMetrics.running_containers}</span>
            <div class="stat-label">Running Containers</div>
        </div>
        <div class="stat-item">
            <span class="stat-value">${systemMetrics.total_containers}</span>
            <div class="stat-label">Total Containers</div>
        </div>
        <div class="stat-item">
            <span class="stat-value">${systemMetrics.total_images}</span>
            <div class="stat-label">Images</div>
        </div>
        <div class="stat-item">
            <span class="stat-value">${systemMetrics.docker_version}</span>
            <div class="stat-label">Docker Version</div>
        </div>
    `;
}

function updateContainerList(containers) {
  const containerList = document.getElementById("containerList");

  if (containers.length === 0) {
    containerList.innerHTML =
      '<div class="empty-state">No running containers with metrics available</div>';
    return;
  }

  let html = "";
  containers.forEach((container) => {
    html += `
      <div class="container-item">
        <div>
          <div class="container-name">${container.container_name}</div>
          <div class="container-metrics">
            <span class="metric-badge cpu-badge">CPU: ${container.cpu_usage_percent.toFixed(
              1
            )}%</span>
            <span class="metric-badge memory-badge">Memory: ${container.memory_usage_mb.toFixed(
              0
            )} MB</span>
            <span class="metric-badge network-badge">Network: ↓${formatBytes(
              container.network_rx_bytes
            )} ↑${formatBytes(container.network_tx_bytes)}</span>
          </div>
        </div>
        <div style="font-size: 0.9rem; color: #666;">
          PIDs: ${container.pids} | 
          Disk: R:${formatBytes(container.block_read_bytes)} W:${formatBytes(
      container.block_write_bytes
    )}
        </div>
      </div>
    `;
  });

  containerList.innerHTML = html;
}

function initializeCharts() {
  const commonOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: "top",
      },
    },
    scales: {
      y: {
        beginAtZero: true,
      },
    },
  };

  // CPU Chart
  charts.cpu = new Chart(document.getElementById("cpuChart"), {
    type: "line",
    data: {
      labels: [],
      datasets: [],
    },
    options: {
      ...commonOptions,
      scales: {
        y: {
          beginAtZero: true,
          max: 100,
          ticks: {
            callback: function (value) {
              return value + "%";
            },
          },
        },
      },
    },
  });

  // Memory Chart
  charts.memory = new Chart(document.getElementById("memoryChart"), {
    type: "line",
    data: {
      labels: [],
      datasets: [],
    },
    options: {
      ...commonOptions,
      scales: {
        y: {
          beginAtZero: true,
          ticks: {
            callback: function (value) {
              return value + " MB";
            },
          },
        },
      },
    },
  });

  // Network Chart
  charts.network = new Chart(document.getElementById("networkChart"), {
    type: "line",
    data: {
      labels: [],
      datasets: [],
    },
    options: {
      ...commonOptions,
      scales: {
        y: {
          beginAtZero: true,
          ticks: {
            callback: function (value) {
              return formatBytes(value);
            },
          },
        },
      },
    },
  });

  // Disk Chart
  charts.disk = new Chart(document.getElementById("diskChart"), {
    type: "line",
    data: {
      labels: [],
      datasets: [],
    },
    options: {
      ...commonOptions,
      scales: {
        y: {
          beginAtZero: true,
          ticks: {
            callback: function (value) {
              return formatBytes(value);
            },
          },
        },
      },
    },
  });
}

function updateCharts(containers) {
  // Don't update charts until configuration is loaded
  if (!configLoaded) {
    console.log("Skipping chart update - configuration not yet loaded");
    return;
  }

  const now = new Date().toLocaleTimeString();

  // Use configurable history limit
  const historyLimit = window.dashboardConfig?.metrics_history_limit || 20;
  if (metricsHistory.length >= historyLimit) {
    metricsHistory.shift();
  }
  metricsHistory.push({ time: now, containers });

  const labels = metricsHistory.map((m) => m.time);

  // Limit containers shown in charts to top N by CPU usage to prevent chart overload
  const maxChartContainers = window.dashboardConfig?.max_chart_containers || 5;
  console.log(
    `Total containers: ${containers.length}, Max chart containers: ${maxChartContainers}`
  );

  const topContainers = containers
    .sort((a, b) => b.cpu_usage_percent - a.cpu_usage_percent)
    .slice(0, maxChartContainers);

  console.log(
    `Showing ${topContainers.length} containers in charts:`,
    topContainers.map((c) => c.container_name)
  );

  // Update CPU chart
  const cpuDatasets = topContainers.map((container, index) => ({
    label: container.container_name,
    data: metricsHistory.map((m) => {
      const c = m.containers.find(
        (cc) => cc.container_id === container.container_id
      );
      return c ? c.cpu_usage_percent : 0;
    }),
    borderColor: `hsl(${(index * 137.5) % 360}, 70%, 50%)`,
    backgroundColor: `hsla(${(index * 137.5) % 360}, 70%, 50%, 0.1)`,
    tension: 0.4,
  }));

  charts.cpu.data.labels = labels;
  charts.cpu.data.datasets = cpuDatasets;
  charts.cpu.update("none");

  // Update Memory chart
  const memoryDatasets = topContainers.map((container, index) => ({
    label: container.container_name,
    data: metricsHistory.map((m) => {
      const c = m.containers.find(
        (cc) => cc.container_id === container.container_id
      );
      return c ? c.memory_usage_mb : 0;
    }),
    borderColor: `hsl(${(index * 137.5 + 60) % 360}, 70%, 50%)`,
    backgroundColor: `hsla(${(index * 137.5 + 60) % 360}, 70%, 50%, 0.1)`,
    tension: 0.4,
  }));

  charts.memory.data.labels = labels;
  charts.memory.data.datasets = memoryDatasets;
  charts.memory.update("none");

  // Update Network chart - show RX and TX separately (limit to top containers)
  const networkDatasets = [];
  topContainers.forEach((container, index) => {
    networkDatasets.push({
      label: `${container.container_name} RX`,
      data: metricsHistory.map((m) => {
        const c = m.containers.find(
          (cc) => cc.container_id === container.container_id
        );
        return c ? c.network_rx_bytes : 0;
      }),
      borderColor: `hsl(${(index * 137.5 + 120) % 360}, 70%, 50%)`,
      backgroundColor: `hsla(${(index * 137.5 + 120) % 360}, 70%, 50%, 0.1)`,
      tension: 0.4,
    });
    networkDatasets.push({
      label: `${container.container_name} TX`,
      data: metricsHistory.map((m) => {
        const c = m.containers.find(
          (cc) => cc.container_id === container.container_id
        );
        return c ? c.network_tx_bytes : 0;
      }),
      borderColor: `hsl(${(index * 137.5 + 180) % 360}, 70%, 50%)`,
      backgroundColor: `hsla(${(index * 137.5 + 180) % 360}, 70%, 50%, 0.1)`,
      tension: 0.4,
      borderDash: [5, 5],
    });
  });

  charts.network.data.labels = labels;
  charts.network.data.datasets = networkDatasets;
  charts.network.update("none");

  // Update Disk chart (limit to top containers)
  const diskDatasets = [];
  topContainers.forEach((container, index) => {
    diskDatasets.push({
      label: `${container.container_name} Read`,
      data: metricsHistory.map((m) => {
        const c = m.containers.find(
          (cc) => cc.container_id === container.container_id
        );
        return c ? c.block_read_bytes : 0;
      }),
      borderColor: `hsl(${(index * 137.5 + 240) % 360}, 70%, 50%)`,
      backgroundColor: `hsla(${(index * 137.5 + 240) % 360}, 70%, 50%, 0.1)`,
      tension: 0.4,
    });
    diskDatasets.push({
      label: `${container.container_name} Write`,
      data: metricsHistory.map((m) => {
        const c = m.containers.find(
          (cc) => cc.container_id === container.container_id
        );
        return c ? c.block_write_bytes : 0;
      }),
      borderColor: `hsl(${(index * 137.5 + 300) % 360}, 70%, 50%)`,
      backgroundColor: `hsla(${(index * 137.5 + 300) % 360}, 70%, 50%, 0.1)`,
      tension: 0.4,
      borderDash: [5, 5],
    });
  });

  charts.disk.data.labels = labels;
  charts.disk.data.datasets = diskDatasets;
  charts.disk.update("none");

  // Add a note if we're showing limited containers
  if (containers.length > maxChartContainers) {
    updateChartNote(containers.length, maxChartContainers);
  } else {
    hideChartNote();
  }
}

function updateChartNote(totalContainers, shownContainers) {
  let noteElement = document.getElementById("chartNote");
  if (!noteElement) {
    noteElement = document.createElement("div");
    noteElement.id = "chartNote";
    noteElement.style.cssText = `
      background: rgba(255, 193, 7, 0.1);
      border: 1px solid rgba(255, 193, 7, 0.3);
      border-radius: 8px;
      padding: 12px;
      margin: 16px 0;
      color: #856404;
      font-size: 14px;
      text-align: center;
    `;
    const dashboard = document.getElementById("dashboard");
    const firstChart = dashboard.querySelector(".chart-container");
    if (firstChart) {
      dashboard.insertBefore(noteElement, firstChart);
    }
  }
  noteElement.innerHTML = `
    <strong>📊 Chart Display Note:</strong> Showing top ${shownContainers} containers by CPU usage in charts (${totalContainers} total containers running).
    <br><small>This helps maintain chart readability and performance.</small>
  `;
  noteElement.style.display = "block";
}

function hideChartNote() {
  const noteElement = document.getElementById("chartNote");
  if (noteElement) {
    noteElement.style.display = "none";
  }
}

async function updateDashboard() {
  const metrics = await fetchMetrics();
  if (!metrics) return;

  document.getElementById("loading").style.display = "none";
  document.getElementById("dashboard").style.display = "block";

  updateSystemStats(metrics.system);
  updateContainerList(metrics.containers);
  updateCharts(metrics.containers);
}

// Initialize the dashboard
document.addEventListener("DOMContentLoaded", async () => {
  // Load configuration first and wait for it
  await fetchConfig();

  // Log the loaded configuration for debugging
  console.log("Dashboard initialized with config:", window.dashboardConfig);

  initializeCharts();
  await updateDashboard();

  // Use configurable update interval (convert seconds to milliseconds)
  const intervalMs =
    (window.dashboardConfig?.metrics_interval_seconds || 5) * 1000;
  console.log(
    `Setting update interval to ${
      window.dashboardConfig?.metrics_interval_seconds || 5
    } seconds`
  );
  setInterval(updateDashboard, intervalMs);
});