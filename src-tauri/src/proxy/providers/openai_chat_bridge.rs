//! chat/completions ⇄ Responses bridge for the ChatGPT upstream — fork addition.
//!
//! The Codex backend only speaks the (streaming) Responses API, while most clients speak
//! `/v1/chat/completions`. This module converts in both directions with pure functions so
//! the mapping is unit-testable without a network:
//!
//! - [`chat_to_responses`] rewrites a chat request as a Responses request;
//! - [`ChatStreamTranslator`] turns Responses SSE events into `chat.completion.chunk`s;
//! - [`responses_to_chat_completion`] folds a finished Responses object into a
//!   `chat.completion` for non-streaming callers.

use serde_json::{json, Map, Value};

/// Convert a chat/completions request body into a Responses request body.
///
/// System/developer messages become `instructions`; everything else becomes `input`
/// items. Unsupported sampling knobs are dropped rather than passed through, because the
/// Codex endpoint rejects unknown fields.
pub fn chat_to_responses(chat: &Value, model: &str) -> Value {
    let mut instructions: Vec<String> = Vec::new();
    let mut input: Vec<Value> = Vec::new();

    if let Some(messages) = chat.get("messages").and_then(|v| v.as_array()) {
        for message in messages {
            let role = message
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user");
            match role {
                "system" | "developer" => {
                    if let Some(text) = message_text(message) {
                        instructions.push(text);
                    }
                }
                "tool" => {
                    // Chat sends tool results as their own message; Responses wants a
                    // function_call_output item tied to the call id.
                    let call_id = message
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": call_id,
                        "output": message_text(message).unwrap_or_default(),
                    }));
                }
                "assistant" => {
                    if let Some(text) = message_text(message) {
                        if !text.is_empty() {
                            input.push(json!({
                                "type": "message",
                                "role": "assistant",
                                "content": [{ "type": "output_text", "text": text }],
                            }));
                        }
                    }
                    if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
                        for call in tool_calls {
                            let function = call.get("function");
                            input.push(json!({
                                "type": "function_call",
                                "call_id": call.get("id").and_then(|v| v.as_str()).unwrap_or_default(),
                                "name": function
                                    .and_then(|f| f.get("name"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default(),
                                "arguments": function
                                    .and_then(|f| f.get("arguments"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("{}"),
                            }));
                        }
                    }
                }
                _ => {
                    // user (and anything unknown) -> a user message with typed content
                    let content = user_content_items(message);
                    if !content.is_empty() {
                        input.push(json!({
                            "type": "message",
                            "role": "user",
                            "content": content,
                        }));
                    }
                }
            }
        }
    }

    let mut body = Map::new();
    body.insert("model".to_string(), json!(model));
    body.insert("input".to_string(), json!(input));

    if !instructions.is_empty() {
        body.insert("instructions".to_string(), json!(instructions.join("\n\n")));
    }

    if let Some(tools) = chat.get("tools").and_then(|v| v.as_array()) {
        let converted: Vec<Value> = tools.iter().filter_map(convert_tool).collect();
        if !converted.is_empty() {
            body.insert("tools".to_string(), json!(converted));
        }
    }

    if let Some(tool_choice) = chat.get("tool_choice") {
        body.insert("tool_choice".to_string(), convert_tool_choice(tool_choice));
    }

    // Token cap: chat has two spellings, Responses has one.
    if let Some(max_tokens) = chat
        .get("max_completion_tokens")
        .or_else(|| chat.get("max_tokens"))
        .and_then(|v| v.as_u64())
    {
        body.insert("max_output_tokens".to_string(), json!(max_tokens));
    }

    for key in ["temperature", "top_p", "parallel_tool_calls"] {
        if let Some(value) = chat.get(key) {
            body.insert(key.to_string(), value.clone());
        }
    }

    if let Some(effort) = chat.get("reasoning_effort").and_then(|v| v.as_str()) {
        body.insert("reasoning".to_string(), json!({ "effort": effort }));
    }

    Value::Object(body)
}

/// Flatten the text of a chat message, whether it is a string or a content array.
fn message_text(message: &Value) -> Option<String> {
    match message.get("content") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Array(parts)) => {
            let joined: Vec<String> = parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            Some(joined.join(""))
        }
        Some(Value::Null) | None => None,
        Some(other) => Some(other.to_string()),
    }
}

/// Build Responses `content` items for a user message, keeping images intact.
fn user_content_items(message: &Value) -> Vec<Value> {
    match message.get("content") {
        Some(Value::String(text)) => {
            if text.is_empty() {
                vec![]
            } else {
                vec![json!({ "type": "input_text", "text": text })]
            }
        }
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| {
                let kind = part.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                match kind {
                    "text" | "input_text" => part
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(|text| json!({ "type": "input_text", "text": text })),
                    "image_url" | "input_image" => {
                        // chat: {image_url: {url}}; responses: {image_url: "<url>"}
                        let url = part
                            .get("image_url")
                            .and_then(|v| {
                                v.get("url").and_then(|u| u.as_str()).or_else(|| v.as_str())
                            })
                            .map(|s| s.to_string());
                        url.map(|url| json!({ "type": "input_image", "image_url": url }))
                    }
                    _ => None,
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// chat tool -> responses tool (the function fields move up one level).
fn convert_tool(tool: &Value) -> Option<Value> {
    let function = tool.get("function")?;
    let name = function.get("name").and_then(|v| v.as_str())?;
    let mut converted = Map::new();
    converted.insert("type".to_string(), json!("function"));
    converted.insert("name".to_string(), json!(name));
    if let Some(description) = function.get("description") {
        converted.insert("description".to_string(), description.clone());
    }
    converted.insert(
        "parameters".to_string(),
        function
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
    );
    Some(Value::Object(converted))
}

fn convert_tool_choice(choice: &Value) -> Value {
    match choice {
        Value::String(_) => choice.clone(),
        Value::Object(map) => match map.get("function").and_then(|f| f.get("name")) {
            Some(name) => json!({ "type": "function", "name": name }),
            None => choice.clone(),
        },
        _ => json!("auto"),
    }
}

// ============================================================================
// Responses -> chat.completion
// ============================================================================

fn usage_to_chat(usage: Option<&Value>) -> Value {
    let get = |key: &str| -> u64 {
        usage
            .and_then(|u| u.get(key))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    };
    let input = get("input_tokens");
    let output = get("output_tokens");
    let cached = usage
        .and_then(|u| u.get("input_tokens_details"))
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    json!({
        "prompt_tokens": input,
        "completion_tokens": output,
        "total_tokens": input + output,
        "prompt_tokens_details": { "cached_tokens": cached },
    })
}

/// Collect assistant text and tool calls out of a finished Responses object.
fn collect_output(response: &Value) -> (String, Vec<Value>) {
    let mut text = String::new();
    let mut tool_calls: Vec<Value> = Vec::new();

    if let Some(items) = response.get("output").and_then(|v| v.as_array()) {
        for item in items {
            match item.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                "message" => {
                    if let Some(parts) = item.get("content").and_then(|v| v.as_array()) {
                        for part in parts {
                            if let Some(chunk) = part.get("text").and_then(|v| v.as_str()) {
                                text.push_str(chunk);
                            }
                        }
                    }
                }
                "function_call" => {
                    tool_calls.push(json!({
                        "index": tool_calls.len(),
                        "id": item.get("call_id").and_then(|v| v.as_str()).unwrap_or_default(),
                        "type": "function",
                        "function": {
                            "name": item.get("name").and_then(|v| v.as_str()).unwrap_or_default(),
                            "arguments": item.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}"),
                        }
                    }));
                }
                _ => {}
            }
        }
    }

    (text, tool_calls)
}

/// Fold a finished Responses object into a `chat.completion`.
pub fn responses_to_chat_completion(response: &Value, model: &str) -> Value {
    let (text, tool_calls) = collect_output(response);

    let finish_reason = if !tool_calls.is_empty() {
        "tool_calls"
    } else if response
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s == "incomplete")
        .unwrap_or(false)
    {
        "length"
    } else {
        "stop"
    };

    let mut message = Map::new();
    message.insert("role".to_string(), json!("assistant"));
    message.insert("content".to_string(), json!(text));
    if !tool_calls.is_empty() {
        message.insert("tool_calls".to_string(), json!(tool_calls));
    }

    json!({
        "id": response.get("id").and_then(|v| v.as_str()).unwrap_or("chatcmpl-openai"),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": response.get("model").and_then(|v| v.as_str()).unwrap_or(model),
        "choices": [{
            "index": 0,
            "message": Value::Object(message),
            "finish_reason": finish_reason,
        }],
        "usage": usage_to_chat(response.get("usage")),
    })
}

// ============================================================================
// Streaming translation
// ============================================================================

/// Turns Responses SSE into chat.completion.chunk SSE.
///
/// Feed raw upstream bytes with [`push_chunk`](Self::push_chunk); it returns the SSE text
/// to forward to the client (possibly empty). Line buffering is internal, so events split
/// across network chunks are handled.
pub struct ChatStreamTranslator {
    buffer: String,
    id: String,
    model: String,
    /// Maps upstream output item index -> chat tool_call index.
    tool_call_slots: Vec<usize>,
    role_sent: bool,
    finished: bool,
}

impl ChatStreamTranslator {
    pub fn new(model: &str) -> Self {
        Self {
            buffer: String::new(),
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
            model: model.to_string(),
            tool_call_slots: Vec::new(),
            role_sent: false,
            finished: false,
        }
    }

    fn chunk(&self, delta: Value, finish_reason: Option<&str>, usage: Option<Value>) -> String {
        let mut payload = json!({
            "id": self.id,
            "object": "chat.completion.chunk",
            "created": chrono::Utc::now().timestamp(),
            "model": self.model,
            "choices": [{
                "index": 0,
                "delta": delta,
                "finish_reason": finish_reason,
            }],
        });
        if let Some(usage) = usage {
            payload["usage"] = usage;
        }
        format!("data: {}\n\n", payload)
    }

    pub fn push_chunk(&mut self, raw: &str) -> String {
        self.buffer.push_str(raw);
        let mut out = String::new();

        while let Some(newline) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=newline).collect();
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            out.push_str(&self.translate_line(&line));
        }

        out
    }

    /// Flush any trailing partial line (upstream may end without a newline).
    pub fn finish(&mut self) -> String {
        let tail = std::mem::take(&mut self.buffer);
        let mut out = String::new();
        if !tail.trim().is_empty() {
            out.push_str(&self.translate_line(tail.trim_end()));
        }
        if !self.finished {
            // Upstream died mid-stream: close the chat stream cleanly anyway.
            out.push_str(&self.chunk(json!({}), Some("stop"), None));
            out.push_str("data: [DONE]\n\n");
            self.finished = true;
        }
        out
    }

    fn translate_line(&mut self, line: &str) -> String {
        let payload = match line.strip_prefix("data:") {
            Some(rest) => rest.trim(),
            None => return String::new(),
        };
        if payload.is_empty() || payload == "[DONE]" {
            return String::new();
        }
        let Ok(event) = serde_json::from_str::<Value>(payload) else {
            return String::new();
        };

        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let mut out = String::new();

        // The first chat chunk must announce the assistant role.
        let mut ensure_role = |out: &mut String, this: &mut Self| {
            if !this.role_sent {
                this.role_sent = true;
                out.push_str(&this.chunk(json!({ "role": "assistant" }), None, None));
            }
        };

        match event_type {
            "response.output_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(|v| v.as_str()) {
                    ensure_role(&mut out, self);
                    out.push_str(&self.chunk(json!({ "content": delta }), None, None));
                }
            }
            // Reasoning summaries are surfaced as reasoning_content, the convention
            // clients like Cherry Studio / OpenWebUI already understand.
            "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(|v| v.as_str()) {
                    ensure_role(&mut out, self);
                    out.push_str(&self.chunk(json!({ "reasoning_content": delta }), None, None));
                }
            }
            "response.output_item.added" => {
                let item = event.get("item");
                let is_function = item
                    .and_then(|i| i.get("type"))
                    .and_then(|v| v.as_str())
                    .map(|t| t == "function_call")
                    .unwrap_or(false);
                if is_function {
                    let output_index = event
                        .get("output_index")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    let slot = self.slot_for(output_index);
                    ensure_role(&mut out, self);
                    out.push_str(&self.chunk(
                        json!({
                            "tool_calls": [{
                                "index": slot,
                                "id": item.and_then(|i| i.get("call_id")).and_then(|v| v.as_str()).unwrap_or_default(),
                                "type": "function",
                                "function": {
                                    "name": item.and_then(|i| i.get("name")).and_then(|v| v.as_str()).unwrap_or_default(),
                                    "arguments": "",
                                }
                            }]
                        }),
                        None,
                        None,
                    ));
                }
            }
            "response.function_call_arguments.delta" => {
                if let Some(delta) = event.get("delta").and_then(|v| v.as_str()) {
                    let output_index = event
                        .get("output_index")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    let slot = self.slot_for(output_index);
                    ensure_role(&mut out, self);
                    out.push_str(&self.chunk(
                        json!({
                            "tool_calls": [{
                                "index": slot,
                                "function": { "arguments": delta }
                            }]
                        }),
                        None,
                        None,
                    ));
                }
            }
            "response.completed" | "response.incomplete" => {
                let response = event.get("response");
                let finish_reason = if !self.tool_call_slots.is_empty() {
                    "tool_calls"
                } else if event_type == "response.incomplete" {
                    "length"
                } else {
                    "stop"
                };
                ensure_role(&mut out, self);
                out.push_str(
                    &self.chunk(
                        json!({}),
                        Some(finish_reason),
                        response
                            .and_then(|r| r.get("usage"))
                            .map(|u| usage_to_chat(Some(u))),
                    ),
                );
                out.push_str("data: [DONE]\n\n");
                self.finished = true;
            }
            "response.failed" | "error" => {
                let message = event
                    .get("response")
                    .and_then(|r| r.get("error"))
                    .and_then(|e| e.get("message"))
                    .or_else(|| event.get("error").and_then(|e| e.get("message")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("ChatGPT upstream error");
                out.push_str(&format!(
                    "data: {}\n\n",
                    json!({ "error": { "message": message, "type": "upstream_error" } })
                ));
                out.push_str("data: [DONE]\n\n");
                self.finished = true;
            }
            _ => {}
        }

        out
    }

    /// Stable chat tool_call index for an upstream output index.
    fn slot_for(&mut self, output_index: usize) -> usize {
        if let Some(position) = self
            .tool_call_slots
            .iter()
            .position(|known| *known == output_index)
        {
            return position;
        }
        self.tool_call_slots.push(output_index);
        self.tool_call_slots.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_chat_request_becomes_responses_request() {
        let chat = json!({
            "model": "gpt-5.1",
            "messages": [
                { "role": "system", "content": "be brief" },
                { "role": "user", "content": "hi" },
                { "role": "assistant", "content": "hello" },
                { "role": "user", "content": [
                    { "type": "text", "text": "look" },
                    { "type": "image_url", "image_url": { "url": "data:image/png;base64,AAA" } }
                ]}
            ],
            "max_tokens": 256,
            "temperature": 0.2,
            "reasoning_effort": "high"
        });

        let converted = chat_to_responses(&chat, "gpt-5.1-codex");

        assert_eq!(converted["model"], json!("gpt-5.1-codex"));
        assert_eq!(converted["instructions"], json!("be brief"));
        assert_eq!(converted["max_output_tokens"], json!(256));
        assert_eq!(converted["temperature"], json!(0.2));
        assert_eq!(converted["reasoning"], json!({ "effort": "high" }));

        let input = converted["input"].as_array().unwrap();
        assert_eq!(
            input.len(),
            3,
            "system message must not become an input item"
        );
        assert_eq!(input[0]["role"], json!("user"));
        assert_eq!(input[0]["content"][0]["type"], json!("input_text"));
        assert_eq!(input[1]["role"], json!("assistant"));
        assert_eq!(input[1]["content"][0]["type"], json!("output_text"));
        assert_eq!(input[2]["content"][1]["type"], json!("input_image"));
        assert_eq!(
            input[2]["content"][1]["image_url"],
            json!("data:image/png;base64,AAA")
        );
    }

    #[test]
    fn openai_chat_tools_and_results_are_converted() {
        let chat = json!({
            "messages": [
                { "role": "user", "content": "weather?" },
                { "role": "assistant", "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": { "name": "get_weather", "arguments": "{\"city\":\"Berlin\"}" }
                }]},
                { "role": "tool", "tool_call_id": "call_1", "content": "sunny" }
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "weather by city",
                    "parameters": { "type": "object", "properties": { "city": { "type": "string" } } }
                }
            }],
            "tool_choice": { "type": "function", "function": { "name": "get_weather" } }
        });

        let converted = chat_to_responses(&chat, "gpt-5.1-codex");
        let input = converted["input"].as_array().unwrap();

        assert_eq!(input[1]["type"], json!("function_call"));
        assert_eq!(input[1]["call_id"], json!("call_1"));
        assert_eq!(input[2]["type"], json!("function_call_output"));
        assert_eq!(input[2]["output"], json!("sunny"));

        // Tool definitions are flattened, not nested under "function".
        assert_eq!(converted["tools"][0]["name"], json!("get_weather"));
        assert!(converted["tools"][0].get("function").is_none());
        assert_eq!(
            converted["tool_choice"],
            json!({ "type": "function", "name": "get_weather" })
        );
    }

    #[test]
    fn openai_finished_response_folds_into_chat_completion() {
        let response = json!({
            "id": "resp_1",
            "model": "gpt-5.1-codex",
            "status": "completed",
            "output": [
                { "type": "message", "content": [{ "type": "output_text", "text": "done" }] },
                { "type": "function_call", "call_id": "call_9", "name": "run", "arguments": "{}" }
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 4,
                "input_tokens_details": { "cached_tokens": 8 }
            }
        });

        let chat = responses_to_chat_completion(&response, "fallback");
        assert_eq!(chat["object"], json!("chat.completion"));
        assert_eq!(chat["model"], json!("gpt-5.1-codex"));
        assert_eq!(chat["choices"][0]["message"]["content"], json!("done"));
        assert_eq!(chat["choices"][0]["finish_reason"], json!("tool_calls"));
        assert_eq!(
            chat["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            json!("run")
        );
        assert_eq!(chat["usage"]["prompt_tokens"], json!(10));
        assert_eq!(chat["usage"]["total_tokens"], json!(14));
        assert_eq!(
            chat["usage"]["prompt_tokens_details"]["cached_tokens"],
            json!(8)
        );
    }

    #[test]
    fn openai_incomplete_response_reports_length_finish_reason() {
        let response = json!({ "status": "incomplete", "output": [] });
        let chat = responses_to_chat_completion(&response, "m");
        assert_eq!(chat["choices"][0]["finish_reason"], json!("length"));
    }

    #[test]
    fn openai_stream_translator_emits_role_text_and_done() {
        let mut translator = ChatStreamTranslator::new("gpt-5.1-codex");

        let mut out = translator
            .push_chunk("data: {\"type\":\"response.output_text.delta\",\"delta\":\"He\"}\n\n");
        out.push_str(
            &translator.push_chunk(
                "data: {\"type\":\"response.output_text.delta\",\"delta\":\"llo\"}\n\n",
            ),
        );
        out.push_str(&translator.push_chunk(
            "data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":3,\"output_tokens\":2}}}\n\n",
        ));

        assert!(out.contains("\"role\":\"assistant\""));
        assert_eq!(out.matches("chat.completion.chunk").count(), 4);
        assert!(out.contains("\"content\":\"He\""));
        assert!(out.contains("\"content\":\"llo\""));
        assert!(out.contains("\"finish_reason\":\"stop\""));
        assert!(out.contains("\"prompt_tokens\":3"));
        assert!(out.trim_end().ends_with("data: [DONE]"));
    }

    #[test]
    fn openai_stream_translator_handles_split_lines_and_tool_calls() {
        let mut translator = ChatStreamTranslator::new("m");

        // A single event split across two network chunks.
        let event = "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"call_id\":\"c1\",\"name\":\"run\"}}\n\n";
        let (first, second) = event.split_at(event.len() / 2);
        let mut out = translator.push_chunk(first);
        assert!(!out.contains("tool_calls"), "must wait for the full line");
        out.push_str(&translator.push_chunk(second));
        out.push_str(&translator.push_chunk(
            "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"delta\":\"{\\\"a\\\":1}\"}\n\n",
        ));
        out.push_str(
            &translator.push_chunk("data: {\"type\":\"response.completed\",\"response\":{}}\n\n"),
        );

        assert!(out.contains("\"id\":\"c1\""));
        assert!(out.contains("\"name\":\"run\""));
        assert!(out.contains("\"arguments\":\"{\\\"a\\\":1}\""));
        assert!(out.contains("\"finish_reason\":\"tool_calls\""));
    }

    #[test]
    fn openai_stream_translator_closes_stream_when_upstream_dies() {
        let mut translator = ChatStreamTranslator::new("m");
        let _ = translator
            .push_chunk("data: {\"type\":\"response.output_text.delta\",\"delta\":\"x\"}\n\n");
        let tail = translator.finish();
        assert!(tail.contains("\"finish_reason\":\"stop\""));
        assert!(tail.contains("[DONE]"));
        // A second finish must not emit another terminator.
        assert!(translator.finish().is_empty());
    }

    #[test]
    fn openai_stream_translator_forwards_upstream_errors() {
        let mut translator = ChatStreamTranslator::new("m");
        let out = translator.push_chunk(
            "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"nope\"}}}\n\n",
        );
        assert!(out.contains("\"type\":\"upstream_error\""));
        assert!(out.contains("nope"));
        assert!(out.contains("[DONE]"));
    }
}
