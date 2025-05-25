use axum::{
    extract::{Path, State, Form, Query, WebSocketUpgrade, ws::{WebSocket, Message}},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router, Json,
    http::{StatusCode, HeaderMap, HeaderValue},
    middleware,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use html_escape;
use tower_http::services::ServeDir;
use futures_util::stream::StreamExt;
use urlencoding;

use crate::config::Config;
use crate::docker;
use crate::models::{ContainerSummary, LocalImageSummary, CreateContainerRequest, EnvironmentVariable, ContainerPortMapping};
use crate::auth::{SessionStore, LoginForm};

#[derive(Deserialize)]
pub struct StartImageParams {
    image_name: String,
}

#[derive(Deserialize)]
pub struct EnhancedStartImageParams {
    image_name: String,
    container_name: Option<String>,
    environment_variables: Option<String>, // JSON string of environment variables
    port_mappings: Option<String>, // JSON string of port mappings
    restart_policy: Option<String>,
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

#[derive(Deserialize)]
pub struct LogQuery {
    tail: Option<String>,
}

struct AppState {
    config: Config,
    session_store: Arc<SessionStore>,
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
        return r#"<tr><td colspan="5"><div class="empty-state">No running containers found</div></td></tr>"#.to_string();
    }

    let mut rows_html = String::new();
    for container in containers {
        let status_class = get_status_class(&container.status);
        
        // Format ports
        let ports_display = if container.ports.is_empty() {
            "<span class='no-ports'>No exposed ports</span>".to_string()
        } else {
            container.ports.iter()
                .map(|port| {
                    if let Some(host_port) = port.host_port {
                        format!("<span class='port-mapping'>{}:{}->{}/{}</span>", 
                               "0.0.0.0", host_port, port.container_port, port.protocol)
                    } else {
                        format!("<span class='port-internal'>{}/{}</span>", 
                               port.container_port, port.protocol)
                    }
                })
                .collect::<Vec<_>>()
                .join("<br>")
        };

        // Format environment variables for details view
        let env_vars_display = if container.environment.is_empty() {
            "<div class='env-empty'>No environment variables</div>".to_string()
        } else {
            container.environment.iter()
                .map(|env| {
                    if let Some(eq_pos) = env.find('=') {
                        let (key, value) = env.split_at(eq_pos);
                        let value = &value[1..]; // Skip the '=' character
                        format!("<div class='env-var'><span class='env-key'>{}</span>=<span class='env-value'>{}</span></div>", 
                               html_escape::encode_text(key), 
                               html_escape::encode_text(value))
                    } else {
                        format!("<div class='env-var'><span class='env-key'>{}</span></div>", 
                               html_escape::encode_text(env))
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        };

        let actions = format!(r#"
            <div class="actions">
                <button class="btn btn-details" onclick="toggleDetails('{}')">
                    <span id="toggle-{}">▶</span> Details
                </button>
                <a href="/logs/{}" class="btn btn-logs">📜 Logs</a>
                <form action="/stop/{}" method="post">
                    <button class="btn btn-stop" type="submit">🛑 Stop</button>
                </form>
                <form action="/restart/{}" method="post">
                    <button class="btn btn-restart" type="submit">🔄 Restart</button>
                </form>
            </div>
        "#, container.id, container.id, container.id, container.id, container.id);

        // Main container row
        rows_html.push_str(&format!(r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td><span class="{}">{}</span></td>
                <td>{}</td>
                <td>
                    {}
                </td>
            </tr>
        "#, container.name, container.image, status_class, container.status, ports_display, actions));

        // Details row (initially hidden)
        rows_html.push_str(&format!(r#"
            <tr id="details-{}" style="display: none;" class="details-row">
                <td colspan="5">
                    <div class="container-details">
                        <div class="details-section">
                            <h4>🔧 Environment Variables</h4>
                            <div class="env-vars">
                                {}
                            </div>
                        </div>
                        <div class="details-section">
                            <h4>📋 Container Information</h4>
                            <div class="container-info">
                                <div class="info-item">
                                    <span class="info-label">Container ID:</span>
                                    <span class="info-value">{}</span>
                                </div>
                                <div class="info-item">
                                    <span class="info-label">Full Image:</span>
                                    <span class="info-value">{}</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </td>
            </tr>
        "#, container.id, env_vars_display, container.id, container.image));
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
                <form action="/start-image" method="post" style="display: inline;">
                    <input type="hidden" name="image_name" value="{}">
                    <button class="btn btn-start" type="submit">🚀 Quick Start</button>
                </form>
                <button class="btn btn-configure" onclick="showAdvancedForm('{}')">⚙️ Configure & Start</button>
            </div>
        "#, html_escape::encode_text(display_tag), html_escape::encode_text(display_tag));

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

    // Generate logout button if auth is enabled
    let logout_button = if state.config.auth_enabled {
        r#"<form action="/logout" method="post" style="display: inline;">
            <button type="submit" class="btn btn-logout" style="background: #e74c3c; color: white; padding: 0.5rem 1rem; border: none; border-radius: 5px; cursor: pointer;">🚪 Logout</button>
        </form>"#
    } else {
        ""
    };

    // Replace placeholders in template
    let html_output = template
        .replace("{{RUNNING_CONTAINERS_ROWS}}", &running_containers_rows)
        .replace("{{IMAGE_ROWS}}", &image_rows)
        .replace("{{AUTH_LOGOUT_BUTTON}}", logout_button);

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

async fn metrics_dashboard_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let template = include_str!("../templates/dashboard.html");
    
    // Generate logout button if auth is enabled
    let logout_button = if state.config.auth_enabled {
        r#"<form action="/logout" method="post" style="display: inline;">
            <button type="submit" class="btn btn-logout" style="background: #e74c3c; color: white; padding: 0.5rem 1rem; border: none; border-radius: 5px; cursor: pointer;">🚪 Logout</button>
        </form>"#
    } else {
        ""
    };

    let html_output = template.replace("{{AUTH_LOGOUT_BUTTON}}", logout_button);
    Html(html_output)
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

async fn logs_handler(Path(container_id): Path<String>, Query(params): Query<LogQuery>) -> impl IntoResponse {
    let tail = params.tail.as_deref();
    
    // Get container info first
    let container_info = match crate::docker::list_running_containers().await {
        Ok(containers) => containers.into_iter().find(|c| c.id == container_id || c.name == container_id),
        Err(_) => None,
    };

    let container_name = container_info.map(|c| c.name).unwrap_or_else(|| container_id.clone());

    // Get recent logs
    let logs_result = crate::docker::get_container_logs_recent(&container_id, tail).await;
    let logs_content = match logs_result {
        Ok(logs) => logs.join("\n"),
        Err(e) => format!("Error fetching logs: {}", e),
    };

    // Load the logs template
    let template = include_str!("../templates/logs.html");
    
    let html_output = template
        .replace("{{CONTAINER_ID}}", &html_escape::encode_text(&container_id))
        .replace("{{CONTAINER_NAME}}", &html_escape::encode_text(&container_name))
        .replace("{{LOGS_CONTENT}}", &html_escape::encode_text(&logs_content))
        .replace("{{TAIL_VALUE}}", tail.unwrap_or("1000"));

    Html(html_output)
}

async fn logs_ws_handler(
    Path(container_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| logs_websocket(socket, container_id))
}

async fn logs_websocket(mut socket: WebSocket, container_id: String) {
    // Get the logs stream
    let logs_stream = match crate::docker::get_container_logs(&container_id, Some("100"), true).await {
        Ok(stream) => stream,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("Error: {}", e))).await;
            let _ = socket.close().await;
            return;
        }
    };

    let mut logs_stream = std::pin::pin!(logs_stream);
    
    // Send initial message
    let _ = socket.send(Message::Text("Connected to logs stream...".to_string())).await;

    // Stream logs to websocket
    while let Some(log_result) = logs_stream.next().await {
        match log_result {
            Ok(log_output) => {
                let log_text = String::from_utf8_lossy(&log_output.into_bytes()).trim().to_string();
                if !log_text.is_empty() {
                    if socket.send(Message::Text(log_text)).await.is_err() {
                        break;
                    }
                }
            }
            Err(e) => {
                let _ = socket.send(Message::Text(format!("Error: {}", e))).await;
                break;
            }
        }
    }
    
    let _ = socket.close().await;
}

async fn login_handler_wrapper(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // If auth is disabled, redirect to main page
    if !state.config.auth_enabled {
        return Redirect::to("/").into_response();
    }

    Html(crate::auth::LOGIN_TEMPLATE.replace("{{ERROR}}", "")).into_response()
}

#[axum::debug_handler]
async fn login_post_handler_wrapper(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    // If auth is disabled, redirect to main page
    if !state.config.auth_enabled {
        return Redirect::to("/").into_response();
    }

    // Verify credentials
    if form.username == state.config.auth_username {
        match state.config.verify_password(&form.password) {
            Ok(true) => {
                // Create session
                let session_id = state.session_store.create_session(&form.username).await;
                
                // Set session cookie and redirect
                let cookie = format!("session_id={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}", 
                    session_id, state.config.session_timeout_seconds);
                
                let mut response = Redirect::to("/").into_response();
                response.headers_mut().insert(
                    "Set-Cookie",
                    HeaderValue::from_str(&cookie).unwrap(),
                );
                response
            }
            _ => {
                tracing::warn!("Failed login attempt for user: {}", form.username);
                Html(crate::auth::LOGIN_TEMPLATE.replace("{{ERROR}}", "<div class=\"error-message\">❌ Invalid username or password</div>")).into_response()
            }
        }
    } else {
        tracing::warn!("Failed login attempt for unknown user: {}", form.username);
        Html(crate::auth::LOGIN_TEMPLATE.replace("{{ERROR}}", "<div class=\"error-message\">❌ Invalid username or password</div>")).into_response()
    }
}

async fn logout_handler_wrapper(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Extract session ID from cookie and remove session
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(session_id) = extract_session_id(cookie_str) {
                state.session_store.remove_session(&session_id).await;
            }
        }
    }

    // Clear cookie and redirect to login
    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        "Set-Cookie",
        HeaderValue::from_str("session_id=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0").unwrap(),
    );
    response
}

fn extract_session_id(cookie_str: &str) -> Option<String> {
    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("session_id=") {
            return Some(value.to_string());
        }
    }
    None
}

async fn start_image_enhanced_handler(State(_state): State<Arc<AppState>>, Form(params): Form<EnhancedStartImageParams>) -> impl IntoResponse {
    // Parse environment variables from JSON string
    let environment_variables = if let Some(env_str) = &params.environment_variables {
        if env_str.trim().is_empty() {
            Vec::new()
        } else {
            match serde_json::from_str::<Vec<EnvironmentVariable>>(env_str) {
                Ok(vars) => vars,
                Err(e) => {
                    tracing::error!("Failed to parse environment variables: {}", e);
                    return Html(format!("Error parsing environment variables: {}. <a href=\"/\">Go back</a>", e)).into_response();
                }
            }
        }
    } else {
        Vec::new()
    };

    // Parse port mappings from JSON string
    let port_mappings = if let Some(ports_str) = &params.port_mappings {
        if ports_str.trim().is_empty() {
            Vec::new()
        } else {
            match serde_json::from_str::<Vec<ContainerPortMapping>>(ports_str) {
                Ok(ports) => ports,
                Err(e) => {
                    tracing::error!("Failed to parse port mappings: {}", e);
                    return Html(format!("Error parsing port mappings: {}. <a href=\"/\">Go back</a>", e)).into_response();
                }
            }
        }
    } else {
        Vec::new()
    };

    let request = CreateContainerRequest {
        image_name: params.image_name.clone(),
        container_name: params.container_name.filter(|s| !s.trim().is_empty()),
        environment_variables,
        port_mappings,
        restart_policy: params.restart_policy.filter(|s| !s.trim().is_empty()),
    };

    match docker::create_and_start_container_enhanced(request).await {
        Ok(container_id) => {
            tracing::info!("Successfully created and started container {} from image {}", container_id, params.image_name);
            Redirect::to("/").into_response()
        },
        Err(e) => {
            tracing::error!("Failed to start container from image {}: {}", params.image_name, e);
            Html(format!("Error starting container from image {}: {}. <a href=\"/\">Go back</a>", html_escape::encode_text(&params.image_name), e)).into_response()
        }
    }
}

async fn image_info_handler(Path(image_name): Path<String>) -> impl IntoResponse {
    // URL decode the image name (in case it contains special characters like :)
    let decoded_image_name = urlencoding::decode(&image_name)
        .map_err(|e| format!("Invalid image name encoding: {}", e))
        .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&image_name));
    
    match docker::get_image_info(&decoded_image_name).await {
        Ok(image_info) => Json(image_info).into_response(),
        Err(e) => {
            tracing::error!("Failed to get image info for {}: {}", decoded_image_name, e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Error getting image info: {}", e)).into_response()
        }
    }
}

pub fn app_router(config: &Config) -> Router {
    let state = Arc::new(AppState {
        config: config.clone(),
        session_store: Arc::new(SessionStore::new(Arc::new(config.clone()))),
    });
    
    Router::new()
        .route("/", get(index_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(readiness_handler))
        .route("/api/config", get(config_handler))
        .route("/api/image/:image_name", get(image_info_handler))
        .route("/start-image", post(start_image_handler))
        .route("/start/:id", post(start_container_handler))
        .route("/stop/:id", post(stop_container_handler))
        .route("/restart/:id", post(restart_container_handler))
        .route("/metrics", get(metrics_dashboard_handler))
        .route("/api/metrics", get(metrics_json_handler))
        .route("/logs/:id", get(logs_handler))
        .route("/logs/:id/ws", get(logs_ws_handler))
        .route("/login", get(login_handler_wrapper))
        .route("/login", post(login_post_handler_wrapper))
        .route("/logout", post(logout_handler_wrapper))
        .route("/start-image-enhanced", post(start_image_enhanced_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn_with_state(
            state.session_store.clone(),
            crate::auth::auth_middleware,
        ))
        .with_state(state)
}