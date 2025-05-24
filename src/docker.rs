use bollard::container::{
    Config,
    CreateContainerOptions,
    ListContainersOptions,
    StartContainerOptions, 
    StopContainerOptions, 
    RestartContainerOptions
};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use std::default::Default;
use super::models::{ContainerSummary, LocalImageSummary};

fn is_image_id(image_name: &str) -> bool {
    // Simple check: a common image ID is a 64-character hex string, or prefixed with sha256:
    (image_name.len() == 64 && image_name.chars().all(|c| c.is_ascii_hexdigit())) || 
    image_name.starts_with("sha256:") || 
    image_name.contains("<none>")
}

// Returns only RUNNING containers
pub async fn list_running_containers() -> Result<Vec<ContainerSummary>, bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
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
    let docker = Docker::connect_with_local_defaults()?;
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