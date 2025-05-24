use axum::{
    extract::{Path, Query, State, Form},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use html_escape;

use crate::docker;
use crate::models::{ContainerSummary, LocalImageSummary};

#[derive(Deserialize)]
pub struct StartImageParams {
    image_name: String,
}

struct AppState {}

fn get_status_class(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        s if s.contains("running") || s.contains("up") => "status-running",
        s if s.contains("exited") || s.contains("stopped") || s.contains("created") => "status-exited",
        _ => "status-other",
    }
}

fn generate_running_container_rows(containers: &[ContainerSummary]) -> String {
    let mut rows_html = String::new();
    for container in containers {
        let status_class = get_status_class(&container.status);
        let actions = format!("
            <form action=\"/stop/{}\" method=\"post\"><button class=\"stop-button\" type=\"submit\">Stop</button></form>
            <form action=\"/restart/{}\" method=\"post\"><button class=\"restart-button\" type=\"submit\">Restart</button></form>
        ", container.id, container.id);

        rows_html.push_str(&format!("
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td><span class=\"{}\">{}</span></td>
                <td class=\"actions\">
                    {}
                </td>
            </tr>
        ", container.name, container.image, status_class, container.status, actions));
    }
    rows_html
}

fn generate_image_rows(images: &[LocalImageSummary]) -> String {
    let mut rows_html = String::new();
    for image in images {
        let display_tag = image.repo_tags.get(0).map_or("N/A", |s| s.as_str());
        let actions = format!("
            <form action=\"/start-image\" method=\"post\">
                <input type=\"hidden\" name=\"image_name\" value=\"{}\">
                <button class=\"start-button\" type=\"submit\">Start New Container</button>
            </form>
        ", html_escape::encode_text(display_tag));

        rows_html.push_str(&format!("
            <tr>
                <td>{}</td>
                <td class=\"actions\">
                    {}
                </td>
            </tr>
        ", html_escape::encode_text(display_tag), actions));
    }
    rows_html
}

async fn index_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let running_containers_result = docker::list_running_containers().await;
    let downloaded_images_result = docker::list_downloaded_images().await;

    let mut html_output = String::from("
        <!DOCTYPE html>
        <html>
        <head>
            <title>Docker Manager</title>
            <style>
                body { font-family: sans-serif; margin: 20px; background-color: #f4f4f4; color: #333; }
                h1, h2 { color: #005A9C; }
                table { width: 100%; border-collapse: collapse; margin-top: 20px; margin-bottom: 30px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
                th, td { padding: 10px; border: 1px solid #ddd; text-align: left; }
                th { background-color: #007ACC; color: white; }
                tr:nth-child(even) { background-color: #eef; }
                tr:hover { background-color: #ddd; }
                .actions form { display: inline-block; margin-right: 5px; }
                .actions button { 
                    padding: 8px 12px; 
                    border: none; 
                    border-radius: 4px; 
                    cursor: pointer; 
                    font-weight: bold;
                    transition: background-color 0.3s ease;
                }
                .start-button { background-color: #28a745; color: white; }
                .start-button:hover { background-color: #218838; }
                .stop-button { background-color: #dc3545; color: white; }
                .stop-button:hover { background-color: #c82333; }
                .restart-button { background-color: #ffc107; color: #333; }
                .restart-button:hover { background-color: #e0a800; }
                .status-running { color: green; font-weight: bold; }
                .status-exited { color: red; font-weight: bold; }
                .status-other { color: orange; font-weight: bold; }
                .error-message { color: red; font-style: italic; margin-top: 10px; }
            </style>
        </head>
        <body>
            <h1>Docker Container Management</h1>
    ");

    // Running Containers Section
    html_output.push_str("
        <h2>Running Containers</h2>
        <table>
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Image</th>
                    <th>Status</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
    ");
    match running_containers_result {
        Ok(containers) => {
            if containers.is_empty() {
                html_output.push_str("<tr><td colspan=\"4\">No running containers found.</td></tr>");
            } else {
                html_output.push_str(&generate_running_container_rows(&containers));
            }
        }
        Err(e) => html_output.push_str(&format!("<tr><td colspan=\"4\" class=\"error-message\">Error listing running containers: {}</td></tr>", e)),
    }
    html_output.push_str("
            </tbody>
        </table>
    ");

    // Available Images Section
    html_output.push_str("
        <h2>Available Images (to start new containers)</h2>
        <table>
            <thead>
                <tr>
                    <th>Image Tag</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
    ");
    match downloaded_images_result {
        Ok(images) => {
            if images.is_empty() {
                html_output.push_str("<tr><td colspan=\"2\">No downloaded images found.</td></tr>");
            } else {
                html_output.push_str(&generate_image_rows(&images));
            }
        }
        Err(e) => html_output.push_str(&format!("<tr><td colspan=\"2\" class=\"error-message\">Error listing images: {}</td></tr>", e)),
    }
    html_output.push_str("
            </tbody>
        </table>
        </body>
        </html>
    ");
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

pub fn app_router() -> Router {
    let state = Arc::new(AppState {});
    Router::new()
        .route("/", get(index_handler))
        .route("/start-image", post(start_image_handler))
        .route("/start/:id", post(start_container_handler))
        .route("/stop/:id", post(stop_container_handler))
        .route("/restart/:id", post(restart_container_handler))
        .with_state(state)
} 