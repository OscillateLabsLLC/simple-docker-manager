use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc, time::{Duration, SystemTime}};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub username: String,
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

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let now = SystemTime::now();
        let timeout = Duration::from_secs(self.config.session_timeout_seconds);
        
        sessions.retain(|_, session| {
            let age = now.duration_since(session.last_accessed).unwrap_or(Duration::ZERO);
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

pub async fn login_handler(State(session_store): State<Arc<SessionStore>>) -> impl IntoResponse {
    // If auth is disabled, redirect to main page
    if !session_store.config.auth_enabled {
        return Redirect::to("/").into_response();
    }

    Html(LOGIN_TEMPLATE).into_response()
}

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
                let cookie = format!("session_id={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}", 
                    session_id, session_store.config.session_timeout_seconds);
                
                let mut response = Redirect::to("/").into_response();
                response.headers_mut().insert(
                    "Set-Cookie",
                    HeaderValue::from_str(&cookie).unwrap(),
                );
                response
            }
            _ => {
                tracing::warn!("Failed login attempt for user: {}", form.username);
                Html(LOGIN_TEMPLATE.replace("{{ERROR}}", "Invalid username or password")).into_response()
            }
        }
    } else {
        tracing::warn!("Failed login attempt for unknown user: {}", form.username);
        Html(LOGIN_TEMPLATE.replace("{{ERROR}}", "Invalid username or password")).into_response()
    }
}

pub async fn logout_handler(State(session_store): State<Arc<SessionStore>>, headers: HeaderMap) -> impl IntoResponse {
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

pub const LOGIN_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🐳 Simple Docker Manager - Login</title>
    <link rel="stylesheet" href="/static/styles.css">
    <style>
        .login-container {
            max-width: 400px;
            margin: 100px auto;
            padding: 2rem;
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            border: 1px solid rgba(255, 255, 255, 0.2);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
        }
        
        .login-form {
            display: flex;
            flex-direction: column;
            gap: 1rem;
        }
        
        .form-group {
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }
        
        .form-group label {
            font-weight: 500;
            color: var(--text-light);
        }
        
        .form-group input {
            padding: 0.75rem;
            border: 1px solid rgba(255, 255, 255, 0.3);
            border-radius: 8px;
            background: rgba(255, 255, 255, 0.1);
            color: var(--text-light);
            font-size: 1rem;
        }
        
        .form-group input:focus {
            outline: none;
            border-color: var(--accent-blue);
            box-shadow: 0 0 0 2px rgba(74, 144, 226, 0.2);
        }
        
        .login-btn {
            padding: 0.75rem;
            background: var(--accent-blue);
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 1rem;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.3s ease;
        }
        
        .login-btn:hover {
            background: var(--accent-blue-dark);
            transform: translateY(-2px);
        }
        
        .error-message {
            color: #ff6b6b;
            font-size: 0.9rem;
            text-align: center;
            margin-top: 1rem;
            padding: 0.75rem;
            background: rgba(255, 107, 107, 0.1);
            border: 1px solid rgba(255, 107, 107, 0.3);
            border-radius: 8px;
        }
        
        .app-title {
            text-align: center;
            margin-bottom: 2rem;
            color: var(--text-light);
        }
        
        .security-note {
            font-size: 0.85rem;
            color: var(--text-muted);
            text-align: center;
            margin-top: 1rem;
            padding: 1rem;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 8px;
        }
    </style>
</head>
<body>
    <div class="login-container">
        <h1 class="app-title">🐳 Simple Docker Manager</h1>
        <form class="login-form" method="post" action="/login">
            <div class="form-group">
                <label for="username">Username</label>
                <input type="text" id="username" name="username" required>
            </div>
            <div class="form-group">
                <label for="password">Password</label>
                <input type="password" id="password" name="password" required>
            </div>
            <button type="submit" class="login-btn">🔐 Login</button>
        </form>
        {{ERROR}}
        <div class="security-note">
            🔒 This application manages Docker containers with privileged access. 
            Please ensure you're using a secure password.
        </div>
    </div>
</body>
</html>
"#; 