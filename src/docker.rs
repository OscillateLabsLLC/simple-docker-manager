use bollard::container::{
    Config,
    CreateContainerOptions,
    ListContainersOptions,
    StartContainerOptions, 
    StopContainerOptions, 
    RestartContainerOptions,
    StatsOptions
};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use std::default::Default;
use chrono::Utc;
use futures_util::stream::StreamExt;
use super::models::{ContainerSummary, LocalImageSummary, ContainerMetrics, SystemMetrics, MetricsResponse};

/// Get a Docker client with optional custom socket configuration
fn get_docker_client(socket_path: Option<&str>) -> Result<Docker, bollard::errors::Error> {
    match socket_path {
        Some(path) => Docker::connect_with_socket(path, 120, bollard::API_DEFAULT_VERSION),
        None => Docker::connect_with_local_defaults(),
    }
}

fn is_image_id(image_name: &str) -> bool {
    // Simple check: a common image ID is a 64-character hex string, or prefixed with sha256:
    (image_name.len() == 64 && image_name.chars().all(|c| c.is_ascii_hexdigit())) || 
    image_name.starts_with("sha256:") || 
    image_name.contains("<none>")
}

// Returns only RUNNING containers
pub async fn list_running_containers() -> Result<Vec<ContainerSummary>, bollard::errors::Error> {
    list_running_containers_with_config(None).await
}

pub async fn list_running_containers_with_config(socket_path: Option<&str>) -> Result<Vec<ContainerSummary>, bollard::errors::Error> {
    let docker = get_docker_client(socket_path)?;
    let options = Some(ListContainersOptions::<String> {
        all: false, // Only running
        filters: std::collections::HashMap::from([("status".to_string(), vec!["running".to_string()])]),
        ..
        Default::default()
    });

    let containers = docker.list_containers(options).await?;

    Ok(containers.into_iter()
        .filter_map(|c| {
            let image = c.image.clone().unwrap_or_default();
            if is_image_id(&image) {
                None
            } else {
                Some(ContainerSummary {
                    id: c.id.unwrap_or_default(),
                    name: c.names.unwrap_or_default().get(0).unwrap_or(&"".to_string()).trim_start_matches('/').to_string(),
                    image: c.image.unwrap_or_default(),
                    status: c.state.unwrap_or_default(),
                })
            }
        })
        .collect())
}

pub async fn list_downloaded_images() -> Result<Vec<LocalImageSummary>, bollard::errors::Error> {
    list_downloaded_images_with_config(None).await
}

pub async fn list_downloaded_images_with_config(socket_path: Option<&str>) -> Result<Vec<LocalImageSummary>, bollard::errors::Error> {
    let docker = get_docker_client(socket_path)?;
    let options = Some(ListImagesOptions::<String> {
        all: false, // Set to true if you want intermediate layers too
        digests: false,
        filters: std::collections::HashMap::new(), // Add filters if needed, e.g. dangling=false
    });

    let images = docker.list_images(options).await?;

    Ok(images.into_iter()
        .filter_map(|img| {
            // We only want images with actual tags, not <none>:<none>
            if img.repo_tags.iter().any(|tag| !tag.contains("<none>")) {
                Some(LocalImageSummary {
                    id: img.id,
                    repo_tags: img.repo_tags,
                })
            } else {
                None
            }
        })
        .collect())
}

pub async fn create_and_start_container_from_image(image_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    
    // Generate a simple name for the new container, e.g., "my-image-timestamp"
    // You might want a more robust naming strategy in a real application
    let container_name = format!("{}-{}", 
        image_name.split(':').next().unwrap_or("container").replace("/", "-"), 
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() % 10000
    );

    let options = Some(CreateContainerOptions{
            name: container_name,
            platform: None, // Some("linux/amd64") or Some("linux/arm64") for example
    });

    let config = Config {
        image: Some(image_name),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false), // Set to true if you need an interactive TTY
        open_stdin: Some(false),
        ..
        Default::default()
    };

    let response = docker.create_container(options, config).await?;
    // Bollard's create_container returns a CreateContainerResponse which has an 'id' field.
    // We should use this ID to start the container for robustness.
    docker.start_container(&response.id, None::<StartContainerOptions<String>>).await
}

pub async fn start_container(container_id_or_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    docker.start_container(container_id_or_name, None::<StartContainerOptions<String>>).await
}

pub async fn stop_container(container_id_or_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    docker.stop_container(container_id_or_name, None::<StopContainerOptions>).await
}

pub async fn restart_container(container_id_or_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    docker.restart_container(container_id_or_name, None::<RestartContainerOptions>).await
}

pub async fn get_container_metrics(container_id: &str) -> Result<Option<ContainerMetrics>, bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    
    // Get container info for name
    let container_info = docker.inspect_container(container_id, None).await?;
    let container_name = container_info.name
        .unwrap_or_default()
        .trim_start_matches('/')
        .to_string();

    let stats_options = Some(StatsOptions {
        stream: false,
        one_shot: true,
    });

    let mut stats_stream = docker.stats(container_id, stats_options);
    
    if let Some(stats_result) = stats_stream.next().await {
        let stats = stats_result?;
        
        // Calculate CPU usage percentage
        let cpu_usage_percent = {
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage - stats.precpu_stats.cpu_usage.total_usage;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) - stats.precpu_stats.system_cpu_usage.unwrap_or(0);
            let number_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
            
            if system_delta > 0 {
                (cpu_delta as f64 / system_delta as f64) * number_cpus * 100.0
            } else {
                0.0
            }
        };

        // Memory stats
        let (memory_usage_mb, memory_limit_mb, memory_usage_percent) = {
            let usage = stats.memory_stats.usage.unwrap_or(0) as f64 / 1024.0 / 1024.0; // Convert to MB
            let limit = stats.memory_stats.limit.unwrap_or(0) as f64 / 1024.0 / 1024.0; // Convert to MB
            let usage_percent = if limit > 0.0 { (usage / limit) * 100.0 } else { 0.0 };
            (usage, limit, usage_percent)
        };

        // Network stats
        let (network_rx_bytes, network_tx_bytes) = if let Some(networks) = &stats.networks {
            let (mut rx_total, mut tx_total) = (0u64, 0u64);
            for (_interface, network_stats) in networks {
                rx_total += network_stats.rx_bytes;
                tx_total += network_stats.tx_bytes;
            }
            (rx_total, tx_total)
        } else {
            (0, 0)
        };

        // Block I/O stats
        let (block_read_bytes, block_write_bytes) = {
            let read_bytes = stats.blkio_stats.io_service_bytes_recursive
                .as_ref()
                .and_then(|ios| ios.iter().find(|io| io.op == "read"))
                .map(|io| io.value)
                .unwrap_or(0);
            
            let write_bytes = stats.blkio_stats.io_service_bytes_recursive
                .as_ref()
                .and_then(|ios| ios.iter().find(|io| io.op == "write"))
                .map(|io| io.value)
                .unwrap_or(0);
            
            (read_bytes, write_bytes)
        };

        let pids = stats.pids_stats.current.unwrap_or(0);

        Ok(Some(ContainerMetrics {
            container_id: container_id.to_string(),
            container_name,
            timestamp: Utc::now(),
            cpu_usage_percent,
            memory_usage_mb,
            memory_limit_mb,
            memory_usage_percent,
            network_rx_bytes,
            network_tx_bytes,
            block_read_bytes,
            block_write_bytes,
            pids,
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_system_metrics() -> Result<SystemMetrics, bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    
    // Get version info
    let version_info = docker.version().await?;
    let docker_version = version_info.version.unwrap_or_else(|| "Unknown".to_string());
    
    // Get container counts
    let all_containers = docker.list_containers(Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    })).await?;
    
    let running_containers = docker.list_containers(Some(ListContainersOptions::<String> {
        all: false,
        filters: std::collections::HashMap::from([("status".to_string(), vec!["running".to_string()])]),
        ..Default::default()
    })).await?;
    
    // Get image count
    let images = docker.list_images(Some(ListImagesOptions::<String> {
        all: false,
        ..Default::default()
    })).await?;
    
    Ok(SystemMetrics {
        timestamp: Utc::now(),
        total_containers: all_containers.len() as u32,
        running_containers: running_containers.len() as u32,
        total_images: images.len() as u32,
        docker_version,
    })
}

pub async fn get_all_metrics() -> Result<MetricsResponse, bollard::errors::Error> {
    let system_metrics = get_system_metrics().await?;
    let running_containers = list_running_containers().await?;
    
    let mut container_metrics = Vec::new();
    for container in running_containers {
        if let Ok(Some(metrics)) = get_container_metrics(&container.id).await {
            container_metrics.push(metrics);
        }
    }
    
    Ok(MetricsResponse {
        system: system_metrics,
        containers: container_metrics,
    })
} 