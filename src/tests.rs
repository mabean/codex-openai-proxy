use std::fs;

use tempfile::NamedTempFile;

use crate::{AnthropicMessage, AnthropicMessagesRequest, ProxyServer};

#[tokio::test]
async fn parses_legacy_auth_file() {
    let file = NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        r#"{
            "OPENAI_API_KEY": null,
            "tokens": {
                "access_token": "legacy-token",
                "account_id": "acc_123"
            }
        }"#,
    )
    .unwrap();

    let proxy = ProxyServer::new(file.path().to_str().unwrap(), "https://example.test")
        .await
        .unwrap();
    assert_eq!(proxy.auth_data.access_token.as_deref(), Some("legacy-token"));
    assert_eq!(proxy.auth_data.account_id.as_deref(), Some("acc_123"));
}

#[tokio::test]
async fn parses_openclaw_auth_profiles_file() {
    let file = NamedTempFile::new().unwrap();
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjX29wZW5jbGF3In19.signature";
    fs::write(
        file.path(),
        format!(
            r#"{{
                "profiles": {{
                    "profile-1": {{
                        "type": "oauth",
                        "access": "{}"
                    }}
                }},
                "lastGood": {{
                    "openai-codex": "profile-1"
                }}
            }}"#,
            jwt
        ),
    )
    .unwrap();

    let proxy = ProxyServer::new(file.path().to_str().unwrap(), "https://example.test")
        .await
        .unwrap();
    assert_eq!(proxy.auth_data.access_token.as_deref(), Some(jwt));
    assert_eq!(proxy.auth_data.account_id.as_deref(), Some("acc_openclaw"));
}

#[tokio::test]
async fn extracts_account_id_from_openclaw_style_jwt() {
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjX2p3dCJ9fQ.signature";
    let account_id = crate::extract_account_id_from_jwt(jwt);
    assert_eq!(account_id.as_deref(), Some("acc_jwt"));
}

#[tokio::test]
async fn rejects_file_without_usable_credentials() {
    let file = NamedTempFile::new().unwrap();
    fs::write(file.path(), r#"{"tokens": {}}"#).unwrap();

    let result = ProxyServer::new(file.path().to_str().unwrap(), "https://example.test").await;
    assert!(result.is_err());
    let err = result.err().unwrap();
    let msg = err.to_string();
    assert!(msg.contains("parse") || msg.contains("supported auth file format") || msg.contains("usable"));
}

#[test]
fn converts_anthropic_messages_to_internal_chat_shape() {
    let proxy = ProxyServer {
        client: reqwest::Client::new(),
        auth_data: crate::AuthData {
            api_key: None,
            access_token: None,
            account_id: None,
        },
        upstream_base_url: "https://example.test".to_string(),
    };

    let req = AnthropicMessagesRequest {
        model: "claude-test".to_string(),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: serde_json::json!("hello"),
        }],
        system: Some(serde_json::json!("be concise")),
        max_tokens: Some(128),
        stream: Some(false),
    };

    let converted = proxy.convert_anthropic_to_chat(req);
    assert_eq!(converted.model, "claude-test");
    assert_eq!(converted.messages.len(), 2);
    assert_eq!(converted.messages[0].role, "system");
    assert_eq!(converted.messages[1].role, "user");
}

#[test]
fn converts_anthropic_messages_without_system_prompt() {
    let proxy = ProxyServer {
        client: reqwest::Client::new(),
        auth_data: crate::AuthData {
            api_key: None,
            access_token: None,
            account_id: None,
        },
        upstream_base_url: "https://example.test".to_string(),
    };

    let req = AnthropicMessagesRequest {
        model: "claude-test".to_string(),
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!("hello"),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: serde_json::json!("hi"),
            },
        ],
        system: None,
        max_tokens: Some(64),
        stream: Some(false),
    };

    let converted = proxy.convert_anthropic_to_chat(req);
    assert_eq!(converted.messages.len(), 2);
    assert_eq!(converted.messages[0].role, "user");
    assert_eq!(converted.messages[1].role, "assistant");
}
