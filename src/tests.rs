use std::fs;

use tempfile::NamedTempFile;

use crate::{
    extract_response_content, flatten_message_content, AnthropicMessage, AnthropicMessagesRequest,
    ChatCompletionsRequest, ChatMessage, ProxyError, ProxyServer,
};

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
    assert_eq!(
        proxy.auth_data.access_token.as_deref(),
        Some("legacy-token")
    );
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
async fn rejects_file_without_usable_credentials() {
    let file = NamedTempFile::new().unwrap();
    fs::write(file.path(), r#"{"tokens": {}}"#).unwrap();

    let result = ProxyServer::new(file.path().to_str().unwrap(), "https://example.test").await;
    assert!(result.is_err());
    let err = result.err().unwrap();
    let msg = err.to_string();
    assert!(
        msg.contains("parse")
            || msg.contains("supported auth file format")
            || msg.contains("usable")
    );
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

    let converted = proxy.convert_anthropic_to_chat(req).unwrap();
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

    let converted = proxy.convert_anthropic_to_chat(req).unwrap();
    assert_eq!(converted.messages.len(), 2);
    assert_eq!(converted.messages[0].role, "user");
    assert_eq!(converted.messages[1].role, "assistant");
}

#[test]
fn anthropic_requires_max_tokens() {
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
        system: None,
        max_tokens: None,
        stream: Some(false),
    };

    let err = proxy.convert_anthropic_to_chat(req).unwrap_err();
    match err {
        ProxyError::Validation { field, .. } => {
            assert_eq!(field.as_deref(), Some("max_tokens"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn anthropic_rejects_invalid_role() {
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
            role: "system".to_string(),
            content: serde_json::json!("hello"),
        }],
        system: None,
        max_tokens: Some(16),
        stream: Some(false),
    };

    let err = proxy.convert_anthropic_to_chat(req).unwrap_err();
    match err {
        ProxyError::Validation { field, .. } => {
            assert_eq!(field.as_deref(), Some("messages[0].role"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn chat_request_requires_messages() {
    let proxy = ProxyServer {
        client: reqwest::Client::new(),
        auth_data: crate::AuthData {
            api_key: None,
            access_token: None,
            account_id: None,
        },
        upstream_base_url: "https://example.test".to_string(),
    };

    let req = ChatCompletionsRequest {
        model: "gpt-test".to_string(),
        messages: vec![],
        stream: Some(false),
        tools: None,
    };

    let err = proxy.convert_chat_to_responses(req).unwrap_err();
    match err {
        ProxyError::Validation { field, .. } => {
            assert_eq!(field.as_deref(), Some("messages"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn flatten_message_content_supports_text_blocks() {
    let content = serde_json::json!([
        {"type": "text", "text": "hello"},
        {"type": "text", "text": "world"}
    ]);
    let flat = flatten_message_content(&content).unwrap();
    assert_eq!(flat, "hello world");
}

#[test]
fn flatten_message_content_rejects_empty_arrays() {
    let content = serde_json::json!([]);
    let err = flatten_message_content(&content).unwrap_err();
    assert!(err.contains("must contain at least one text item"));
}

#[test]
fn extract_response_content_prefers_deltas() {
    let sse = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hel\"}\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"lo\"}\n",
        "data: [DONE]\n"
    );

    let content = extract_response_content(sse).unwrap();
    assert_eq!(content, "Hello");
}

#[test]
fn extract_response_content_falls_back_to_final_item() {
    let sse = concat!(
        "data: {\"type\":\"response.output_item.done\",\"item\":{\"content\":[{\"text\":\"Fallback\"}]}}\n",
        "data: [DONE]\n"
    );

    let content = extract_response_content(sse).unwrap();
    assert_eq!(content, "Fallback");
}

#[test]
fn extract_response_content_errors_when_empty() {
    let sse = "data: [DONE]\n";
    let err = extract_response_content(sse).unwrap_err();
    match err {
        ProxyError::UpstreamProtocol { message } => {
            assert!(message.contains("no parsable response content"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn convert_chat_to_responses_supports_array_message_content() {
    let proxy = ProxyServer {
        client: reqwest::Client::new(),
        auth_data: crate::AuthData {
            api_key: None,
            access_token: None,
            account_id: None,
        },
        upstream_base_url: "https://example.test".to_string(),
    };

    let req = ChatCompletionsRequest {
        model: "gpt-test".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: serde_json::json!([
                {"type": "text", "text": "first"},
                {"type": "text", "text": "second"}
            ]),
        }],
        stream: Some(false),
        tools: None,
    };

    let converted = proxy.convert_chat_to_responses(req).unwrap();
    assert_eq!(converted.model, "gpt-test");
    assert_eq!(converted.input.len(), 1);
}
