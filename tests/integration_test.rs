// Integration tests for Simple Docker Manager
// These tests verify end-to-end functionality

#[cfg(test)]
mod integration_tests {
    use simple_docker_manager::*;

    #[test]
    fn test_config_integration() {
        // Test that config can be created with defaults
        let config = config::Config::default();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert!(config.auth_enabled);

        // Test bind address generation
        let bind_addr = config.bind_address();
        assert_eq!(bind_addr, "0.0.0.0:3000");
    }

    #[test]
    fn test_models_roundtrip() {
        use models::*;

        // Test that models can be serialized and deserialized
        let port = PortMapping {
            container_port: 8080,
            host_port: Some(80),
            protocol: "tcp".to_string(),
        };

        let json = serde_json::to_string(&port).expect("Should serialize");
        let deserialized: PortMapping = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.container_port, port.container_port);
        assert_eq!(deserialized.host_port, port.host_port);
        assert_eq!(deserialized.protocol, port.protocol);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        use std::sync::Arc;
        use auth::SessionStore;

        let config = Arc::new(config::Config {
            auth_enabled: true,
            auth_username: "admin".to_string(),
            session_timeout_seconds: 3600,
            ..Default::default()
        });

        let store = SessionStore::new(config);

        // Create session
        let session_id = store.create_session("admin").await;
        assert!(!session_id.is_empty());

        // Retrieve session
        let session = store.get_session(&session_id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().username, "admin");

        // Remove session
        let removed = store.remove_session(&session_id).await;
        assert!(removed);

        // Verify session is gone
        let session = store.get_session(&session_id).await;
        assert!(session.is_none());
    }
}
