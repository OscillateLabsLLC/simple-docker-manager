use serde::{Deserialize, Serialize};

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