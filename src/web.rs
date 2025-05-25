use axum::{
    extract::{Path, State, Form},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router, Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use html_escape;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::docker;
use crate::models::{ContainerSummary, LocalImageSummary};

#[derive(Deserialize)]
pub struct StartImageParams {
    image_name: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
    docker_available: bool,
    timestamp: String,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    metrics_interval_seconds: u64,
    metrics_history_limit: usize,
}

struct AppState {
    config: Config,
}

fn get_status_class(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        s if s.contains("running") || s.contains("up") => "status-running",
        s if s.contains("exited") || s.contains("stopped") || s.contains("created") => "status-exited",
        _ => "status-other",
    }
}

fn generate_running_container_rows(containers: &[ContainerSummary]) -> String {
    if containers.is_empty() {
        return r#"<tr><td colspan="4"><div class="empty-state">No running containers found</div></td></tr>"#.to_string();
    }

    let mut rows_html = String::new();
    for container in containers {
        let status_class = get_status_class(&container.status);
        let actions = format!(r#"
            <div class="actions">
                <form action="/stop/{}" method="post">
                    <button class="btn btn-stop" type="submit">🛑 Stop</button>
                </form>
                <form action="/restart/{}" method="post">
                    <button class="btn btn-restart" type="submit">🔄 Restart</button>
                </form>
            </div>
        "#, container.id, container.id);

        rows_html.push_str(&format!(r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td><span class="{}">{}</span></td>
                <td>
                    {}
                </td>
            </tr>
        "#, container.name, container.image, status_class, container.status, actions));
    }
    rows_html
}

fn generate_image_rows(images: &[LocalImageSummary]) -> String {
    if images.is_empty() {
        return r#"<tr><td colspan="2"><div class="empty-state">No downloaded images found</div></td></tr>"#.to_string();
    }

    let mut rows_html = String::new();
    for image in images {
        let display_tag = image.repo_tags.get(0).map_or("N/A", |s| s.as_str());
        let actions = format!(r#"
            <div class="actions">
                <form action="/start-image" method="post">
                    <input type="hidden" name="image_name" value="{}">
                    <button class="btn btn-start" type="submit">🚀 Start New Container</button>
                </form>
            </div>
        "#, html_escape::encode_text(display_tag));

        rows_html.push_str(&format!(r#"
            <tr>
                <td>{}</td>
                <td>
                    {}
                </td>
            </tr>
        "#, html_escape::encode_text(display_tag), actions));
    }
    rows_html
}

async fn index_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let docker_socket = state.config.docker_socket.as_deref();
    let running_containers_result = crate::docker::list_running_containers_with_config(docker_socket).await;
    let downloaded_images_result = crate::docker::list_downloaded_images_with_config(docker_socket).await;

    // Load the template
    let template = include_str!("../templates/management.html");

    // Generate running containers rows
    let running_containers_rows = match running_containers_result {
        Ok(containers) => generate_running_container_rows(&containers),
        Err(e) => format!(r#"<tr><td colspan="4"><div class="error-message">Error listing running containers: {}</div></td></tr>"#, e),
    };

    // Generate image rows
    let image_rows = match downloaded_images_result {
        Ok(images) => generate_image_rows(&images),
        Err(e) => format!(r#"<tr><td colspan="2"><div class="error-message">Error listing images: {}</div></td></tr>"#, e),
    };

    // Replace placeholders in template
    let html_output = template
        .replace("{{RUNNING_CONTAINERS_ROWS}}", &running_containers_rows)
        .replace("{{IMAGE_ROWS}}", &image_rows);

    Html(html_output)
}

async fn start_image_handler(State(_state): State<Arc<AppState>>, Form(params): Form<StartImageParams>) -> impl IntoResponse {
    match docker::create_and_start_container_from_image(&params.image_name).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => {
            tracing::error!("Failed to start container from image {}: {}", params.image_name, e);
            Html(format!("Error starting container from image {}: {}. <a href=\"/\">Go back</a>", html_escape::encode_text(&params.image_name), e)).into_response()
        }
    }
}

async fn start_container_handler(Path(container_id): Path<String>) -> impl IntoResponse {
    match docker::start_container(&container_id).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => Html(format!("Error starting container {}: {}", container_id, e)).into_response(),
    }
}

async fn stop_container_handler(Path(container_id): Path<String>) -> impl IntoResponse {
    match docker::stop_container(&container_id).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => Html(format!("Error stopping container {}: {}", container_id, e)).into_response(),
    }
}

async fn restart_container_handler(Path(container_id): Path<String>) -> impl IntoResponse {
    match docker::restart_container(&container_id).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => Html(format!("Error restarting container {}: {}", container_id, e)).into_response(),
    }
}

async fn metrics_json_handler() -> impl IntoResponse {
    match docker::get_all_metrics().await {
        Ok(metrics) => Json(metrics).into_response(),
        Err(e) => {
            tracing::error!("Failed to get metrics: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Error getting metrics: {}", e)).into_response()
        }
    }
}

async fn metrics_dashboard_handler() -> impl IntoResponse {
    Html(include_str!("../templates/dashboard.html"))
}

async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let docker_socket = state.config.docker_socket.as_deref();
    let docker_available = match crate::docker::list_running_containers_with_config(docker_socket).await {
        Ok(_) => true,
        Err(_) => false,
    };

    let health = HealthResponse {
        status: if docker_available { "healthy".to_string() } else { "unhealthy".to_string() },
        version: env!("CARGO_PKG_VERSION").to_string(),
        docker_available,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let status_code = if docker_available {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(health))
}

async fn readiness_handler() -> impl IntoResponse {
    // Simple readiness check - just verify we can respond
    Json(serde_json::json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn config_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(ConfigResponse {
        metrics_interval_seconds: state.config.metrics_interval_seconds,
        metrics_history_limit: state.config.metrics_history_limit,
    })
}

pub fn app_router(config: &Config) -> Router {
    let state = Arc::new(AppState {
        config: config.clone(),
    });
    Router::new()
        .route("/", get(index_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(readiness_handler))
        .route("/api/config", get(config_handler))
        .route("/start-image", post(start_image_handler))
        .route("/start/:id", post(start_container_handler))
        .route("/stop/:id", post(stop_container_handler))
        .route("/restart/:id", post(restart_container_handler))
        .route("/metrics", get(metrics_dashboard_handler))
        .route("/api/metrics", get(metrics_json_handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
} 