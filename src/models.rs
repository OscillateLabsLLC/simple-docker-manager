use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalImageSummary {
    pub id: String,
    pub repo_tags: Vec<String>, // e.g., ["ubuntu:latest", "ubuntu:22.04"]
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerMetrics {
    pub container_id: String,
    pub container_name: String,
    pub timestamp: DateTime<Utc>,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub memory_usage_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
    pub pids: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub total_containers: u32,
    pub running_containers: u32,
    pub total_images: u32,
    pub docker_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetricsResponse {
    pub system: SystemMetrics,
    pub containers: Vec<ContainerMetrics>,
} 