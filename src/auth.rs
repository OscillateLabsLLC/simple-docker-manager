use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct Session {
    #[allow(dead_code)]
    pub user_id: String,
    pub username: String,
    #[allow(dead_code)]
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    config: Arc<Config>,
}

impl SessionStore {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn create_session(&self, username: &str) -> String {
        let session_id = Uuid::new_v4().to_string();
        let session = Session {
            user_id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        tracing::info!("Created session for user: {}", username);
        session_id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let mut sessions = self.sessions.write().await;

        // Check if session exists and is not expired
        let should_remove = if let Some(session) = sessions.get(session_id) {
            let session_duration = SystemTime::now()
                .duration_since(session.last_accessed)
                .unwrap_or(Duration::ZERO);

            session_duration.as_secs() > self.config.session_timeout_seconds
        } else {
            return None;
        };

        if should_remove {
            if let Some(session) = sessions.remove(session_id) {
                tracing::info!("Removed expired session for user: {}", session.username);
            }
            return None;
        }

        // Update last accessed time and return session
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_accessed = SystemTime::now();
            Some(session.clone())
        } else {
            None
        }
    }

    pub async fn remove_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            tracing::info!("Removed session for user: {}", session.username);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let now = SystemTime::now();
        let timeout = Duration::from_secs(self.config.session_timeout_seconds);

        sessions.retain(|_, session| {
            let age = now
                .duration_since(session.last_accessed)
                .unwrap_or(Duration::ZERO);
            age < timeout
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub async fn auth_middleware(
    State(session_store): State<Arc<SessionStore>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Skip auth for health endpoints, static assets, and login/logout
    let path = request.uri().path();
    if path.starts_with("/health")
        || path.starts_with("/ready")
        || path.starts_with("/static/")
        || path == "/login"
        || path == "/logout"
    {
        return next.run(request).await;
    }

    // Skip auth if authentication is disabled
    if !session_store.config.auth_enabled {
        return next.run(request).await;
    }

    // Check for session cookie
    if let Some(cookie_header) = request.headers().get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(session_id) = extract_session_id(cookie_str) {
                if let Some(session) = session_store.get_session(&session_id).await {
                    // Add session info to request extensions
                    request.extensions_mut().insert(session);
                    return next.run(request).await;
                }
            }
        }
    }

    // No valid session - handle differently for API vs web requests
    if path.starts_with("/api/") {
        // For API endpoints, return 401 Unauthorized instead of redirecting
        use axum::http::StatusCode;
        (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    } else {
        // For web pages, redirect to login
        Redirect::to("/login").into_response()
    }
}

#[allow(dead_code)]
pub async fn login_handler(State(session_store): State<Arc<SessionStore>>) -> impl IntoResponse {
    // If auth is disabled, redirect to main page
    if !session_store.config.auth_enabled {
        return Redirect::to("/").into_response();
    }

    let template = include_str!("../templates/login.html");
    let html = template.replace("{{ERROR_MESSAGE}}", "");
    Html(html).into_response()
}

#[allow(dead_code)]
pub async fn login_post_handler(
    State(session_store): State<Arc<SessionStore>>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    // If auth is disabled, redirect to main page
    if !session_store.config.auth_enabled {
        return Redirect::to("/").into_response();
    }

    // Verify credentials
    if form.username == session_store.config.auth_username {
        match session_store.config.verify_password(&form.password) {
            Ok(true) => {
                // Create session
                let session_id = session_store.create_session(&form.username).await;

                // Set session cookie and redirect
                let cookie = format!(
                    "session_id={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
                    session_id, session_store.config.session_timeout_seconds
                );

                let mut response = Redirect::to("/").into_response();
                response
                    .headers_mut()
                    .insert("Set-Cookie", HeaderValue::from_str(&cookie).unwrap());
                response
            }
            _ => {
                tracing::warn!("Failed login attempt for user: {}", form.username);
                let template = include_str!("../templates/login.html");
                let error_html = r#"<div class="error-message">Invalid username or password</div>"#;
                let html = template.replace("{{ERROR_MESSAGE}}", error_html);
                Html(html).into_response()
            }
        }
    } else {
        tracing::warn!("Failed login attempt for unknown user: {}", form.username);
        let template = include_str!("../templates/login.html");
        let error_html = r#"<div class="error-message">Invalid username or password</div>"#;
        let html = template.replace("{{ERROR_MESSAGE}}", error_html);
        Html(html).into_response()
    }
}

#[allow(dead_code)]
pub async fn logout_handler(
    State(session_store): State<Arc<SessionStore>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Extract session ID from cookie and remove session
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(session_id) = extract_session_id(cookie_str) {
                session_store.remove_session(&session_id).await;
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
