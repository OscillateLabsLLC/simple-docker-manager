use bollard::container::{
    Config,
    CreateContainerOptions,
    ListContainersOptions,
    StartContainerOptions, 
    StopContainerOptions, 
    RestartContainerOptions,
    StatsOptions,
    LogsOptions
};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use std::default::Default;
use chrono::Utc;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use super::models::{
    ContainerSummary, 
    LocalImageSummary, 
    ContainerMetrics, 
    SystemMetrics, 
    MetricsResponse, 
    PortMapping,
    CreateContainerRequest,
    ImageInfo,
    ContainerPortMapping,
    EnvironmentVariable
};

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
    let mut detailed_containers = Vec::new();

    for container in containers {
        let image = container.image.clone().unwrap_or_default();
        if is_image_id(&image) {
            continue;
        }

        let container_id = container.id.clone().unwrap_or_default();
        let container_name = container.names
            .unwrap_or_default()
            .get(0)
            .unwrap_or(&"".to_string())
            .trim_start_matches('/')
            .to_string();

        // Get detailed information for each container
        match docker.inspect_container(&container_id, None).await {
            Ok(inspect_result) => {
                // Extract ports
                let mut ports = Vec::new();
                if let Some(network_settings) = &inspect_result.network_settings {
                    if let Some(port_map) = &network_settings.ports {
                        for (port_key, port_bindings) in port_map {
                            if let Some(bindings) = port_bindings {
                                let (container_port, protocol) = if let Some(slash_pos) = port_key.find('/') {
                                    let port_str = &port_key[..slash_pos];
                                    let protocol_str = &port_key[slash_pos + 1..];
                                    (port_str.parse::<u16>().unwrap_or(0), protocol_str.to_string())
                                } else {
                                    (port_key.parse::<u16>().unwrap_or(0), "tcp".to_string())
                                };

                                for binding in bindings {
                                    let host_port = binding.host_port.as_ref()
                                        .and_then(|p| p.parse::<u16>().ok());
                                    
                                    ports.push(PortMapping {
                                        container_port,
                                        host_port,
                                        protocol: protocol.clone(),
                                    });
                                }
                            }
                        }
                    }
                }

                // Sort ports for consistent display order
                // Sort by container port first, then by protocol
                ports.sort_by(|a, b| {
                    a.container_port.cmp(&b.container_port)
                        .then_with(|| a.protocol.cmp(&b.protocol))
                });

                // Extract environment variables
                let mut environment = if let Some(config) = &inspect_result.config {
                    config.env.clone().unwrap_or_default()
                } else {
                    Vec::new()
                };

                // Sort environment variables for consistent display order
                environment.sort();

                detailed_containers.push(ContainerSummary {
                    id: container_id,
                    name: container_name,
                    image,
                    status: container.state.unwrap_or_default(),
                    ports,
                    environment,
                });
            }
            Err(_) => {
                // Fallback to basic info if inspection fails
                detailed_containers.push(ContainerSummary {
                    id: container_id,
                    name: container_name,
                    image,
                    status: container.state.unwrap_or_default(),
                    ports: Vec::new(),
                    environment: Vec::new(),
                });
            }
        }
    }

    Ok(detailed_containers)
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
    let docker = get_docker_client(None)?;
    
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

/// Enhanced container creation with environment variables, port mappings, and restart policies
pub async fn create_and_start_container_enhanced(request: CreateContainerRequest) -> Result<String, bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    
    // Generate container name if not provided
    let container_name = request.container_name.unwrap_or_else(|| {
        format!("{}-{}", 
            request.image_name.split(':').next().unwrap_or("container").replace("/", "-"), 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() % 10000
        )
    });

    let options = Some(CreateContainerOptions {
        name: container_name.clone(),
        platform: None,
    });

    // Convert environment variables to Docker format
    let env_vars: Vec<String> = request.environment_variables
        .iter()
        .map(|env| format!("{}={}", env.key, env.value))
        .collect();

    // Convert port mappings to Docker format
    let mut exposed_ports = HashMap::new();
    let mut port_bindings = HashMap::new();

    for port_mapping in &request.port_mappings {
        // Skip empty port mappings (container port of 0)
        if port_mapping.container_port == 0 {
            continue;
        }
        
        let port_key = format!("{}/{}", port_mapping.container_port, port_mapping.protocol);
        
        // Expose the port
        exposed_ports.insert(port_key.clone(), HashMap::new());
        
        // Determine host port: use specified host port, or default to same as container port
        let host_port = port_mapping.host_port.unwrap_or(port_mapping.container_port);
        
        port_bindings.insert(
            port_key,
            Some(vec![bollard::models::PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(host_port.to_string()),
            }])
        );
    }

    // Convert restart policy using the correct enum
    let restart_policy = request.restart_policy.as_ref().map(|policy| {
        use bollard::models::RestartPolicyNameEnum;
        let policy_enum = match policy.as_str() {
            "no" => RestartPolicyNameEnum::NO,
            "always" => RestartPolicyNameEnum::ALWAYS,
            "unless-stopped" => RestartPolicyNameEnum::UNLESS_STOPPED,
            "on-failure" => RestartPolicyNameEnum::ON_FAILURE,
            _ => RestartPolicyNameEnum::NO, // Default fallback
        };
        
        bollard::models::RestartPolicy {
            name: Some(policy_enum),
            maximum_retry_count: if policy == "on-failure" { Some(3) } else { None },
        }
    });

    let host_config = if !port_bindings.is_empty() || restart_policy.is_some() {
        Some(bollard::models::HostConfig {
            port_bindings: if port_bindings.is_empty() { None } else { Some(port_bindings) },
            restart_policy,
            ..Default::default()
        })
    } else {
        None
    };

    let config = Config {
        image: Some(request.image_name.clone()),
        env: if env_vars.is_empty() { None } else { Some(env_vars) },
        exposed_ports: if exposed_ports.is_empty() { None } else { Some(exposed_ports) },
        host_config,
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        open_stdin: Some(false),
        ..Default::default()
    };

    let response = docker.create_container(options, config).await?;
    docker.start_container(&response.id, None::<StartContainerOptions<String>>).await?;
    
    Ok(response.id)
}

pub async fn start_container(container_id_or_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    docker.start_container(container_id_or_name, None::<StartContainerOptions<String>>).await
}

pub async fn stop_container(container_id_or_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    docker.stop_container(container_id_or_name, None::<StopContainerOptions>).await
}

pub async fn restart_container(container_id_or_name: &str) -> Result<(), bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    docker.restart_container(container_id_or_name, None::<RestartContainerOptions>).await
}

pub async fn get_container_metrics(container_id: &str) -> Result<Option<ContainerMetrics>, bollard::errors::Error> {
    get_container_metrics_with_config(container_id, None).await
}

pub async fn get_container_metrics_with_config(container_id: &str, socket_path: Option<&str>) -> Result<Option<ContainerMetrics>, bollard::errors::Error> {
    let docker = get_docker_client(socket_path)?;
    
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
    get_system_metrics_with_config(None).await
}

pub async fn get_system_metrics_with_config(socket_path: Option<&str>) -> Result<SystemMetrics, bollard::errors::Error> {
    let docker = get_docker_client(socket_path)?;
    
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
    get_all_metrics_with_config(None).await
}

pub async fn get_all_metrics_with_config(socket_path: Option<&str>) -> Result<MetricsResponse, bollard::errors::Error> {
    let system_metrics = get_system_metrics_with_config(socket_path).await?;
    let running_containers = list_running_containers_with_config(socket_path).await?;
    
    let mut container_metrics = Vec::new();
    for container in running_containers {
        if let Ok(Some(metrics)) = get_container_metrics_with_config(&container.id, socket_path).await {
            container_metrics.push(metrics);
        }
    }
    
    Ok(MetricsResponse {
        system: system_metrics,
        containers: container_metrics,
    })
}

/// Get logs for a specific container
pub async fn get_container_logs(container_id: &str, tail: Option<&str>, follow: bool) -> Result<impl futures_util::Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>>, bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    
    let logs_options = Some(LogsOptions::<String> {
        follow,
        stdout: true,
        stderr: true,
        since: 0,
        until: 0,
        timestamps: true,
        tail: tail.unwrap_or("1000").to_string(),
    });

    Ok(docker.logs(container_id, logs_options))
}

/// Get recent logs for a specific container as a vector of strings
pub async fn get_container_logs_recent(container_id: &str, tail: Option<&str>) -> Result<Vec<String>, bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    
    let logs_options = Some(LogsOptions::<String> {
        follow: false,
        stdout: true,
        stderr: true,
        since: 0,
        until: 0,
        timestamps: true,
        tail: tail.unwrap_or("1000").to_string(),
    });

    let mut logs_stream = docker.logs(container_id, logs_options);
    let mut log_lines = Vec::new();
    
    while let Some(log_result) = logs_stream.next().await {
        match log_result {
            Ok(log_output) => {
                let log_text = String::from_utf8_lossy(&log_output.into_bytes()).trim().to_string();
                if !log_text.is_empty() {
                    log_lines.push(log_text);
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(log_lines)
}

/// Get detailed information about a Docker image including exposed ports and environment variables
pub async fn get_image_info(image_name: &str) -> Result<ImageInfo, bollard::errors::Error> {
    let docker = get_docker_client(None)?;
    
    // Inspect the image
    let image_inspect = docker.inspect_image(image_name).await?;
    
    // Extract exposed ports
    let mut exposed_ports = Vec::new();
    if let Some(config) = &image_inspect.config {
        if let Some(exposed_ports_map) = &config.exposed_ports {
            for port_key in exposed_ports_map.keys() {
                if let Some(slash_pos) = port_key.find('/') {
                    let port_str = &port_key[..slash_pos];
                    let protocol_str = &port_key[slash_pos + 1..];
                    if let Ok(port_num) = port_str.parse::<u16>() {
                        exposed_ports.push(ContainerPortMapping {
                            container_port: port_num,
                            host_port: None, // Image doesn't specify host port
                            protocol: protocol_str.to_string(),
                        });
                    }
                }
            }
        }
    }
    
    // Sort ports for consistent display
    exposed_ports.sort_by_key(|p| p.container_port);
    
    // Extract environment variables
    let mut environment_variables = Vec::new();
    if let Some(config) = &image_inspect.config {
        if let Some(env_vars) = &config.env {
            for env_var in env_vars {
                if let Some(eq_pos) = env_var.find('=') {
                    let (key, value) = env_var.split_at(eq_pos);
                    let value = &value[1..]; // Skip the '=' character
                    environment_variables.push(EnvironmentVariable {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
            }
        }
    }
    
    // Sort environment variables for consistent display
    environment_variables.sort_by(|a, b| a.key.cmp(&b.key));
    
    Ok(ImageInfo {
        id: image_inspect.id.unwrap_or_default(),
        repo_tags: image_inspect.repo_tags.unwrap_or_default(),
        exposed_ports,
        environment_variables,
    })
} 