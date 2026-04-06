use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::ProxyError;

#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalStreamEvent {
    MessageStart,
    TextDelta {
        text: String,
    },
    TextDone {
        full_text: String,
    },
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    Completed,
}

pub fn parse_codex_sse_to_events(
    response_text: &str,
) -> Result<Vec<CanonicalStreamEvent>, ProxyError> {
    let mut events = Vec::new();
    let mut saw_message_start = false;
    let mut accumulated_text = String::new();

    for line in response_text.lines() {
        let Some(json_data) = line.strip_prefix("data: ") else {
            continue;
        };

        if json_data == "[DONE]" {
            break;
        }

        let Ok(event) = serde_json::from_str::<Value>(json_data) else {
            continue;
        };

        let Some(event_type) = event.get("type").and_then(Value::as_str) else {
            continue;
        };

        match event_type {
            "response.output_item.added" => {
                let is_message = event
                    .get("item")
                    .and_then(|item| item.get("type"))
                    .and_then(Value::as_str)
                    == Some("message");
                if is_message && !saw_message_start {
                    saw_message_start = true;
                    events.push(CanonicalStreamEvent::MessageStart);
                }
            }
            "response.output_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    accumulated_text.push_str(delta);
                    events.push(CanonicalStreamEvent::TextDelta {
                        text: delta.to_string(),
                    });
                }
            }
            "response.output_text.done" => {
                if let Some(text) = event.get("text").and_then(Value::as_str) {
                    if accumulated_text.is_empty() {
                        accumulated_text.push_str(text);
                    }
                    events.push(CanonicalStreamEvent::TextDone {
                        full_text: text.to_string(),
                    });
                }
            }
            "response.completed" => {
                if let Some(usage) = event.get("response").and_then(|r| r.get("usage")) {
                    let input_tokens = usage
                        .get("input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    events.push(CanonicalStreamEvent::Usage {
                        input_tokens,
                        output_tokens,
                    });
                }
                events.push(CanonicalStreamEvent::Completed);
            }
            "response.failed" => {
                let message = event
                    .get("response")
                    .and_then(|r| r.get("error"))
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Codex response failed")
                    .to_string();
                return Err(ProxyError::UpstreamProtocol { message });
            }
            "error" => {
                let message = event
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Codex upstream error")
                    .to_string();
                return Err(ProxyError::UpstreamProtocol { message });
            }
            _ => {}
        }
    }

    if events.is_empty() {
        return Err(ProxyError::UpstreamProtocol {
            message: "upstream returned success but no parsable stream events".to_string(),
        });
    }

    Ok(events)
}

#[derive(Serialize)]
struct OpenAiChunkChoiceDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Serialize)]
struct OpenAiChunkChoice {
    index: u32,
    delta: OpenAiChunkChoiceDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
}

#[derive(Serialize)]
struct OpenAiChunk {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<OpenAiChunkChoice>,
}

pub fn render_openai_sse(events: &[CanonicalStreamEvent], model: &str) -> String {
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = chrono::Utc::now().timestamp();
    let mut out = String::new();

    for event in events {
        match event {
            CanonicalStreamEvent::MessageStart => {
                let chunk = OpenAiChunk {
                    id: id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created,
                    model: model.to_string(),
                    choices: vec![OpenAiChunkChoice {
                        index: 0,
                        delta: OpenAiChunkChoiceDelta {
                            role: Some("assistant".to_string()),
                            content: None,
                        },
                        finish_reason: None,
                    }],
                };
                out.push_str("data: ");
                out.push_str(&serde_json::to_string(&chunk).unwrap());
                out.push_str("\n\n");
            }
            CanonicalStreamEvent::TextDelta { text } => {
                let chunk = OpenAiChunk {
                    id: id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created,
                    model: model.to_string(),
                    choices: vec![OpenAiChunkChoice {
                        index: 0,
                        delta: OpenAiChunkChoiceDelta {
                            role: None,
                            content: Some(text.clone()),
                        },
                        finish_reason: None,
                    }],
                };
                out.push_str("data: ");
                out.push_str(&serde_json::to_string(&chunk).unwrap());
                out.push_str("\n\n");
            }
            CanonicalStreamEvent::Completed => {
                let chunk = OpenAiChunk {
                    id: id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created,
                    model: model.to_string(),
                    choices: vec![OpenAiChunkChoice {
                        index: 0,
                        delta: OpenAiChunkChoiceDelta {
                            role: None,
                            content: None,
                        },
                        finish_reason: Some("stop".to_string()),
                    }],
                };
                out.push_str("data: ");
                out.push_str(&serde_json::to_string(&chunk).unwrap());
                out.push_str("\n\n");
                out.push_str("data: [DONE]\n\n");
            }
            _ => {}
        }
    }

    out
}

pub fn render_anthropic_sse(events: &[CanonicalStreamEvent], model: &str) -> String {
    let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
    let mut out = String::new();

    out.push_str("event: message_start\n");
    out.push_str("data: ");
    out.push_str(&json_string(&serde_json::json!({
        "type": "message_start",
        "message": {
            "id": message_id,
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": model,
            "stop_reason": null,
            "stop_sequence": null,
            "usage": { "input_tokens": 0, "output_tokens": 0 }
        }
    })));
    out.push_str("\n\n");

    out.push_str("event: content_block_start\n");
    out.push_str("data: ");
    out.push_str(&json_string(&serde_json::json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": {
            "type": "text",
            "text": ""
        }
    })));
    out.push_str("\n\n");

    for event in events {
        match event {
            CanonicalStreamEvent::TextDelta { text } => {
                out.push_str("event: content_block_delta\n");
                out.push_str("data: ");
                out.push_str(&json_string(&serde_json::json!({
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {
                        "type": "text_delta",
                        "text": text
                    }
                })));
                out.push_str("\n\n");
            }
            CanonicalStreamEvent::Completed => {
                out.push_str("event: content_block_stop\n");
                out.push_str("data: {\"type\":\"content_block_stop\",\"index\":0}\n\n");
                out.push_str("event: message_delta\n");
                out.push_str("data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":0}}\n\n");
                out.push_str("event: message_stop\n");
                out.push_str("data: {\"type\":\"message_stop\"}\n\n");
            }
            _ => {}
        }
    }

    out
}

fn json_string(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap()
}
