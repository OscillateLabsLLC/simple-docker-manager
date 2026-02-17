use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: Vec<PortMapping>,
    pub environment: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalImageSummary {
    pub id: String,
    pub repo_tags: Vec<String>, // e.g., ["ubuntu:latest", "ubuntu:22.04"]
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageInfo {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub exposed_ports: Vec<ContainerPortMapping>,
    pub environment_variables: Vec<EnvironmentVariable>,
}

// New structures for enhanced container creation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerPortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String, // "tcp" or "udp"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateContainerRequest {
    pub image_name: String,
    pub container_name: Option<String>,
    pub environment_variables: Vec<EnvironmentVariable>,
    pub port_mappings: Vec<ContainerPortMapping>,
    pub restart_policy: Option<String>, // "no", "always", "unless-stopped", "on-failure"
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_port_mapping_serialization() {
        let port = PortMapping {
            container_port: 8080,
            host_port: Some(80),
            protocol: "tcp".to_string(),
        };

        let json = serde_json::to_string(&port).expect("Should serialize");
        assert!(json.contains("8080"));
        assert!(json.contains("80"));
        assert!(json.contains("tcp"));

        let deserialized: PortMapping =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.container_port, 8080);
        assert_eq!(deserialized.host_port, Some(80));
        assert_eq!(deserialized.protocol, "tcp");
    }

    #[test]
    fn test_container_summary_serialization() {
        let summary = ContainerSummary {
            id: "abc123".to_string(),
            name: "test-container".to_string(),
            image: "nginx:latest".to_string(),
            status: "running".to_string(),
            ports: vec![PortMapping {
                container_port: 80,
                host_port: Some(8080),
                protocol: "tcp".to_string(),
            }],
            environment: vec!["ENV=production".to_string()],
        };

        let json = serde_json::to_string(&summary).expect("Should serialize");
        let deserialized: ContainerSummary =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.id, "abc123");
        assert_eq!(deserialized.name, "test-container");
        assert_eq!(deserialized.image, "nginx:latest");
        assert_eq!(deserialized.status, "running");
        assert_eq!(deserialized.ports.len(), 1);
        assert_eq!(deserialized.environment.len(), 1);
    }

    #[test]
    fn test_environment_variable() {
        let env_var = EnvironmentVariable {
            key: "DATABASE_URL".to_string(),
            value: "postgres://localhost/db".to_string(),
        };

        let json = serde_json::to_string(&env_var).expect("Should serialize");
        let deserialized: EnvironmentVariable =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.key, "DATABASE_URL");
        assert_eq!(deserialized.value, "postgres://localhost/db");
    }

    #[test]
    fn test_create_container_request() {
        let request = CreateContainerRequest {
            image_name: "nginx:alpine".to_string(),
            container_name: Some("my-nginx".to_string()),
            environment_variables: vec![EnvironmentVariable {
                key: "NGINX_PORT".to_string(),
                value: "80".to_string(),
            }],
            port_mappings: vec![ContainerPortMapping {
                container_port: 80,
                host_port: Some(8080),
                protocol: "tcp".to_string(),
            }],
            restart_policy: Some("unless-stopped".to_string()),
        };

        let json = serde_json::to_string(&request).expect("Should serialize");
        let deserialized: CreateContainerRequest =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.image_name, "nginx:alpine");
        assert_eq!(deserialized.container_name, Some("my-nginx".to_string()));
        assert_eq!(deserialized.environment_variables.len(), 1);
        assert_eq!(deserialized.port_mappings.len(), 1);
        assert_eq!(
            deserialized.restart_policy,
            Some("unless-stopped".to_string())
        );
    }

    #[test]
    fn test_container_metrics() {
        let metrics = ContainerMetrics {
            container_id: "container123".to_string(),
            container_name: "test-app".to_string(),
            timestamp: Utc::now(),
            cpu_usage_percent: 25.5,
            memory_usage_mb: 512.0,
            memory_limit_mb: 1024.0,
            memory_usage_percent: 50.0,
            network_rx_bytes: 1024000,
            network_tx_bytes: 512000,
            block_read_bytes: 204800,
            block_write_bytes: 102400,
            pids: 15,
        };

        let json = serde_json::to_string(&metrics).expect("Should serialize");
        let deserialized: ContainerMetrics =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.container_id, "container123");
        assert_eq!(deserialized.container_name, "test-app");
        assert_eq!(deserialized.cpu_usage_percent, 25.5);
        assert_eq!(deserialized.memory_usage_mb, 512.0);
        assert_eq!(deserialized.memory_limit_mb, 1024.0);
        assert_eq!(deserialized.network_rx_bytes, 1024000);
        assert_eq!(deserialized.pids, 15);
    }

    #[test]
    fn test_system_metrics() {
        let system = SystemMetrics {
            timestamp: Utc::now(),
            total_containers: 10,
            running_containers: 7,
            total_images: 15,
            docker_version: "24.0.0".to_string(),
        };

        let json = serde_json::to_string(&system).expect("Should serialize");
        let deserialized: SystemMetrics =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.total_containers, 10);
        assert_eq!(deserialized.running_containers, 7);
        assert_eq!(deserialized.total_images, 15);
        assert_eq!(deserialized.docker_version, "24.0.0");
    }

    #[test]
    fn test_metrics_response() {
        let response = MetricsResponse {
            system: SystemMetrics {
                timestamp: Utc::now(),
                total_containers: 5,
                running_containers: 3,
                total_images: 10,
                docker_version: "24.0.0".to_string(),
            },
            containers: vec![ContainerMetrics {
                container_id: "test123".to_string(),
                container_name: "app".to_string(),
                timestamp: Utc::now(),
                cpu_usage_percent: 10.0,
                memory_usage_mb: 256.0,
                memory_limit_mb: 512.0,
                memory_usage_percent: 50.0,
                network_rx_bytes: 1000,
                network_tx_bytes: 500,
                block_read_bytes: 2000,
                block_write_bytes: 1000,
                pids: 5,
            }],
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        let deserialized: MetricsResponse =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.system.total_containers, 5);
        assert_eq!(deserialized.containers.len(), 1);
        assert_eq!(deserialized.containers[0].container_name, "app");
    }

    #[test]
    fn test_local_image_summary() {
        let image = LocalImageSummary {
            id: "sha256:abc123".to_string(),
            repo_tags: vec!["ubuntu:latest".to_string(), "ubuntu:22.04".to_string()],
        };

        let json = serde_json::to_string(&image).expect("Should serialize");
        let deserialized: LocalImageSummary =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.id, "sha256:abc123");
        assert_eq!(deserialized.repo_tags.len(), 2);
        assert!(deserialized.repo_tags.contains(&"ubuntu:latest".to_string()));
    }
}
