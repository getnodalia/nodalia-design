//! The two request and reply shapes: Anthropic's Messages API and Chat Completions.
//!
//! Pure functions from the conversation to JSON and back, so every case is tested without a
//! network.

use serde_json::{json, Map, Value};

use crate::message::{Message, Part, Role, ToolCall, ToolSpec};
use crate::provider::Protocol;

/// How a model's turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// It finished what it had to say.
    End,
    /// It wants tools run first.
    Tools,
    /// It ran out of room mid-answer.
    Length,
    /// The provider declined to continue.
    Refused,
}

/// One model turn.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub text: String,
    pub calls: Vec<ToolCall>,
    pub stop: Stop,
}

/// Longest answer asked of Anthropic, whose API requires a limit.
const MAX_TOKENS: u32 = 8192;

/// The JSON body for one request.
pub fn request(
    protocol: Protocol,
    model: &str,
    system: &str,
    messages: &[Message],
    tools: &[ToolSpec],
) -> Value {
    match protocol {
        Protocol::Anthropic => anthropic_request(model, system, messages, tools),
        Protocol::OpenAi => openai_request(model, system, messages, tools),
    }
}

/// What a reply body says.
pub fn parse_reply(protocol: Protocol, body: &Value) -> Result<Reply, String> {
    match protocol {
        Protocol::Anthropic => anthropic_reply(body),
        Protocol::OpenAi => openai_reply(body),
    }
}

fn object_or_empty(v: &Value) -> Value {
    if v.is_object() {
        v.clone()
    } else {
        json!({})
    }
}

fn anthropic_request(model: &str, system: &str, messages: &[Message], tools: &[ToolSpec]) -> Value {
    let msgs: Vec<Value> = messages
        .iter()
        .filter_map(|m| {
            let content: Vec<Value> = m
                .parts
                .iter()
                .map(|p| match p {
                    Part::Text(t) | Part::Context(t) => json!({"type": "text", "text": t}),
                    Part::ToolUse(c) => json!({
                        "type": "tool_use",
                        "id": c.id,
                        "name": c.name,
                        "input": object_or_empty(&c.input),
                    }),
                    Part::ToolResult(r) => json!({
                        "type": "tool_result",
                        "tool_use_id": r.call_id,
                        "content": r.content,
                        "is_error": r.is_error,
                    }),
                })
                .collect();
            (!content.is_empty()).then(|| {
                json!({
                    "role": match m.role { Role::User => "user", Role::Assistant => "assistant" },
                    "content": content,
                })
            })
        })
        .collect();
    let mut body = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        // Marking the system prompt caches it and the tools before it between requests.
        "system": [{"type": "text", "text": system, "cache_control": {"type": "ephemeral"}}],
        "messages": msgs,
    });
    if let (Some(o), false) = (body.as_object_mut(), tools.is_empty()) {
        o.insert(
            "tools".into(),
            Value::Array(
                tools
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "input_schema": t.schema,
                        })
                    })
                    .collect(),
            ),
        );
    }
    body
}

fn anthropic_reply(body: &Value) -> Result<Reply, String> {
    if body.get("type").and_then(Value::as_str) == Some("error") {
        return Err(error_text(None, body));
    }
    let blocks = body
        .get("content")
        .and_then(Value::as_array)
        .ok_or("The provider's answer had no content.")?;
    let mut text = String::new();
    let mut calls = Vec::new();
    for b in blocks {
        match b.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(t) = b.get("text").and_then(Value::as_str) {
                    text.push_str(t);
                }
            }
            Some("tool_use") => calls.push(ToolCall {
                id: b
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                name: b
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                input: object_or_empty(b.get("input").unwrap_or(&Value::Null)),
            }),
            _ => {}
        }
    }
    let stop = match body.get("stop_reason").and_then(Value::as_str) {
        Some("tool_use") => Stop::Tools,
        Some("max_tokens") => Stop::Length,
        Some("refusal") => Stop::Refused,
        _ if !calls.is_empty() => Stop::Tools,
        _ => Stop::End,
    };
    Ok(Reply { text, calls, stop })
}

fn openai_request(model: &str, system: &str, messages: &[Message], tools: &[ToolSpec]) -> Value {
    let mut msgs = vec![json!({"role": "system", "content": system})];
    for m in messages {
        match m.role {
            Role::User => {
                let mut text = String::new();
                for p in &m.parts {
                    match p {
                        Part::Context(t) | Part::Text(t) => {
                            if !text.is_empty() {
                                text.push_str("\n\n");
                            }
                            text.push_str(t);
                        }
                        Part::ToolResult(r) => msgs.push(json!({
                            "role": "tool",
                            "tool_call_id": r.call_id,
                            "content": r.content,
                        })),
                        Part::ToolUse(_) => {}
                    }
                }
                if !text.is_empty() {
                    msgs.push(json!({"role": "user", "content": text}));
                }
            }
            Role::Assistant => {
                let text: String = m
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let calls: Vec<Value> = m
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::ToolUse(c) => Some(json!({
                            "id": c.id,
                            "type": "function",
                            "function": {"name": c.name, "arguments": c.input.to_string()},
                        })),
                        _ => None,
                    })
                    .collect();
                let mut msg = Map::new();
                msg.insert("role".into(), json!("assistant"));
                msg.insert(
                    "content".into(),
                    if text.is_empty() {
                        Value::Null
                    } else {
                        json!(text)
                    },
                );
                if !calls.is_empty() {
                    msg.insert("tool_calls".into(), Value::Array(calls));
                }
                msgs.push(Value::Object(msg));
            }
        }
    }
    let mut body = json!({"model": model, "messages": msgs});
    if let (Some(o), false) = (body.as_object_mut(), tools.is_empty()) {
        o.insert(
            "tools".into(),
            Value::Array(
                tools
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.schema,
                            },
                        })
                    })
                    .collect(),
            ),
        );
    }
    body
}

fn openai_reply(body: &Value) -> Result<Reply, String> {
    if body.get("error").is_some() && body.get("choices").is_none() {
        return Err(error_text(None, body));
    }
    let choice = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .ok_or("The provider's answer had no choices.")?;
    let message = choice
        .get("message")
        .ok_or("The provider's answer had no message.")?;
    let text = match message.get("content") {
        Some(Value::String(s)) => s.clone(),
        // Some servers answer with typed parts.
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    };
    let mut calls = Vec::new();
    if let Some(list) = message.get("tool_calls").and_then(Value::as_array) {
        for (i, c) in list.iter().enumerate() {
            let function = c.get("function").unwrap_or(&Value::Null);
            let name = function
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let input = match function.get("arguments") {
                Some(Value::String(raw)) if raw.trim().is_empty() => json!({}),
                Some(Value::String(raw)) => serde_json::from_str::<Value>(raw)
                    .ok()
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({"_invalid_arguments": raw})),
                Some(v) if v.is_object() => v.clone(),
                _ => json!({}),
            };
            // Some servers leave the id empty.
            let id = c
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("call_{i}"), str::to_string);
            calls.push(ToolCall { id, name, input });
        }
    }
    let stop = match choice.get("finish_reason").and_then(Value::as_str) {
        Some("tool_calls") | Some("function_call") => Stop::Tools,
        Some("length") => Stop::Length,
        Some("content_filter") => Stop::Refused,
        _ if !calls.is_empty() => Stop::Tools,
        _ => Stop::End,
    };
    Ok(Reply { text, calls, stop })
}

/// The message in an error body, or in a failed status's body, cut short. The API key is never
/// part of what is returned.
pub fn error_text(status: Option<u16>, body: &Value) -> String {
    let from_body = body
        .pointer("/error/message")
        .or_else(|| body.get("message"))
        .or_else(|| body.get("error").filter(|e| e.is_string()))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let text = from_body.map_or_else(
        || {
            status.map_or_else(
                || "The provider returned an error.".to_string(),
                |s| format!("The provider answered with status {s}."),
            )
        },
        str::to_string,
    );
    truncate(&text, 400)
}

pub(crate) fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{cut}…")
}

/// The chat models in a `/models` answer, sorted, without the embedding, speech and image
/// models the same list carries.
pub fn parse_models(body: &Value) -> Vec<String> {
    let list = body
        .get("data")
        .or_else(|| body.get("models"))
        .and_then(Value::as_array);
    let mut ids: Vec<String> = list
        .into_iter()
        .flatten()
        .filter_map(|m| {
            m.get("id")
                .or_else(|| m.get("name"))
                .and_then(Value::as_str)
        })
        // Gemini lists `models/gemini-…`, and wants the bare id back.
        .map(|id| id.strip_prefix("models/").unwrap_or(id).to_string())
        .filter(|id| {
            let l = id.to_ascii_lowercase();
            ![
                "embed",
                "whisper",
                "tts",
                "dall-e",
                "moderation",
                "image",
                "audio",
                "transcribe",
                "realtime",
                "rerank",
            ]
            .iter()
            .any(|bad| l.contains(bad))
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ToolResult;

    fn tool() -> ToolSpec {
        ToolSpec {
            name: "read_range".into(),
            description: "Read cells".into(),
            schema: json!({"type": "object", "properties": {"range": {"type": "string"}}}),
            read_only: true,
        }
    }

    fn conversation() -> Vec<Message> {
        vec![
            Message::user("Sum column B", Some("state".into())),
            Message::assistant(
                "Let me look.".into(),
                vec![ToolCall {
                    id: "t1".into(),
                    name: "read_range".into(),
                    input: json!({"range": "B1:B3"}),
                }],
            ),
            Message::tool_results(vec![ToolResult {
                call_id: "t1".into(),
                content: "[1,2,3]".into(),
                is_error: false,
            }]),
        ]
    }

    #[test]
    fn anthropic_request_carries_tools_system_and_blocks() {
        let b = request(Protocol::Anthropic, "m", "sys", &conversation(), &[tool()]);
        assert_eq!(b["model"], "m");
        assert_eq!(b["system"][0]["text"], "sys");
        assert_eq!(b["tools"][0]["input_schema"]["type"], "object");
        let msgs = b["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0]["content"][0]["text"], "state");
        assert_eq!(msgs[1]["content"][1]["type"], "tool_use");
        assert_eq!(msgs[2]["role"], "user");
        assert_eq!(msgs[2]["content"][0]["type"], "tool_result");
        assert_eq!(msgs[2]["content"][0]["tool_use_id"], "t1");
    }

    #[test]
    fn openai_request_sends_results_as_tool_messages_and_arguments_as_text() {
        let b = request(Protocol::OpenAi, "m", "sys", &conversation(), &[tool()]);
        let msgs = b["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "state\n\nSum column B");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(
            msgs[2]["tool_calls"][0]["function"]["arguments"],
            "{\"range\":\"B1:B3\"}"
        );
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["tool_call_id"], "t1");
        assert_eq!(b["tools"][0]["function"]["name"], "read_range");
    }

    #[test]
    fn no_tools_means_no_tools_field() {
        for p in [Protocol::Anthropic, Protocol::OpenAi] {
            let b = request(p, "m", "s", &conversation(), &[]);
            assert!(b.get("tools").is_none());
        }
    }

    #[test]
    fn anthropic_reply_reads_text_and_calls() {
        let body = json!({
            "stop_reason": "tool_use",
            "content": [
                {"type": "text", "text": "Reading."},
                {"type": "tool_use", "id": "t9", "name": "read_range", "input": {"range": "A1"}},
            ],
        });
        let r = parse_reply(Protocol::Anthropic, &body).unwrap();
        assert_eq!(r.text, "Reading.");
        assert_eq!(r.stop, Stop::Tools);
        assert_eq!(r.calls[0].input["range"], "A1");
    }

    #[test]
    fn openai_reply_reads_calls_with_text_arguments() {
        let body = json!({"choices": [{
            "finish_reason": "tool_calls",
            "message": {"content": null, "tool_calls": [
                {"id": "", "type": "function", "function": {"name": "f", "arguments": "{\"a\":1}"}},
                {"id": "x", "type": "function", "function": {"name": "g", "arguments": "not json"}},
                {"id": "y", "type": "function", "function": {"name": "h", "arguments": ""}},
            ]},
        }]});
        let r = parse_reply(Protocol::OpenAi, &body).unwrap();
        assert_eq!(r.stop, Stop::Tools);
        assert_eq!(r.calls[0].id, "call_0", "an empty id is replaced");
        assert_eq!(r.calls[0].input["a"], 1);
        assert_eq!(r.calls[1].input["_invalid_arguments"], "not json");
        assert_eq!(r.calls[2].input, json!({}));
    }

    #[test]
    fn replies_with_errors_say_what_the_provider_said() {
        let anthropic = json!({"type": "error", "error": {"type": "x", "message": "bad key"}});
        assert_eq!(
            parse_reply(Protocol::Anthropic, &anthropic).unwrap_err(),
            "bad key"
        );
        let openai = json!({"error": {"message": "quota"}});
        assert_eq!(parse_reply(Protocol::OpenAi, &openai).unwrap_err(), "quota");
        assert!(parse_reply(Protocol::OpenAi, &json!({})).is_err());
        assert!(parse_reply(Protocol::Anthropic, &json!({})).is_err());
    }

    #[test]
    fn error_text_falls_back_to_the_status_and_is_bounded() {
        assert_eq!(
            error_text(Some(503), &json!({})),
            "The provider answered with status 503."
        );
        let long = "x".repeat(1000);
        let t = error_text(Some(400), &json!({"error": {"message": long}}));
        assert!(t.chars().count() <= 401);
    }

    #[test]
    fn model_lists_drop_non_chat_models_and_gemini_prefixes() {
        let body = json!({"data": [
            {"id": "models/gemini-x"}, {"id": "text-embedding-3"}, {"id": "gpt-x"},
            {"id": "whisper-1"}, {"id": "gpt-x"},
        ]});
        assert_eq!(parse_models(&body), vec!["gemini-x", "gpt-x"]);
        assert!(parse_models(&json!({})).is_empty());
    }

    #[test]
    fn a_length_stop_is_reported() {
        let body = json!({"stop_reason": "max_tokens", "content": [{"type": "text", "text": "a"}]});
        assert_eq!(
            parse_reply(Protocol::Anthropic, &body).unwrap().stop,
            Stop::Length
        );
    }
}
