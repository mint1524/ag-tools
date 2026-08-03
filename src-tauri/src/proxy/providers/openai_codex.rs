//! ChatGPT (Codex) upstream provider — fork addition.
//!
//! Serves requests from the pool of `provider: openai` accounts against
//! `https://chatgpt.com/backend-api/codex/responses`, the endpoint a ChatGPT
//! subscription actually authorizes (no platform API key involved).
//!
//! Responsibilities kept here so upstream files stay nearly untouched:
//! - account selection (round-robin over healthy accounts, reusing the shared
//!   rate-limit tracker so cooldowns are visible in the existing UI),
//! - access-token freshness and failover on 401/429,
//! - request-body normalization for the Codex endpoint (it only speaks streaming
//!   Responses API),
//! - capturing usage and the `x-codex-*` rate-limit headers into the existing
//!   stats/quota storage.

use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures::StreamExt;
use serde_json::{json, Value};

use crate::models::{Account, AccountProvider};
use crate::modules::openai_account;
use crate::modules::openai_oauth;
use crate::proxy::providers::openai_chat_bridge::{
    chat_to_responses, responses_to_chat_completion, ChatStreamTranslator,
};
use crate::proxy::server::AppState;

/// Which protocol the *client* speaks. The upstream is always streaming Responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientWire {
    /// Client speaks the Responses API — relay verbatim.
    Responses,
    /// Client speaks chat/completions — translate both ways.
    Chat,
}

/// Models the Codex backend is known to serve. Used by `Auto` dispatch to decide
/// whether an unprefixed model belongs to ChatGPT rather than Antigravity.
const KNOWN_OPENAI_MODEL_PREFIXES: &[&str] = &[
    "gpt-5", "gpt-4", "gpt-6", "o1", "o3", "o4", "codex", "chatgpt",
];

/// Explicit routing prefixes a client can use regardless of dispatch mode.
const ROUTING_PREFIXES: &[&str] = &["openai:", "chatgpt:", "codex:"];

/// Strip a routing prefix, returning the bare model id when one was present.
pub fn strip_routing_prefix(model: &str) -> Option<String> {
    let lower = model.to_ascii_lowercase();
    ROUTING_PREFIXES.iter().find_map(|prefix| {
        lower
            .strip_prefix(prefix)
            .map(|rest| model[model.len() - rest.len()..].to_string())
    })
}

/// Does this model look like an OpenAI model?
pub fn looks_like_openai_model(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    KNOWN_OPENAI_MODEL_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// Decide whether a request should be served by a ChatGPT account.
///
/// Pure function of config + model so it can be unit-tested and reasoned about; the
/// caller additionally checks that a usable account exists.
pub fn should_route_to_openai(config: &crate::proxy::OpenAiConfig, model: Option<&str>) -> bool {
    use crate::proxy::OpenAiDispatchMode as Mode;

    if !config.enabled {
        return false;
    }

    match config.dispatch_mode {
        Mode::Off => false,
        Mode::Exclusive => true,
        Mode::Prefix => model
            .map(|m| strip_routing_prefix(m).is_some())
            .unwrap_or(false),
        Mode::Auto => model
            .map(|m| strip_routing_prefix(m).is_some() || looks_like_openai_model(m))
            .unwrap_or(false),
    }
}

/// Resolve the upstream model id for a request.
pub fn resolve_model(config: &crate::proxy::OpenAiConfig, requested: Option<&str>) -> String {
    let requested = match requested {
        Some(model) if !model.trim().is_empty() => model.trim(),
        _ => return config.default_model.clone(),
    };

    let bare = strip_routing_prefix(requested).unwrap_or_else(|| requested.to_string());

    if let Some(mapped) = config.model_mapping.get(&bare) {
        return mapped.clone();
    }
    if let Some(mapped) = config.model_mapping.get(requested) {
        return mapped.clone();
    }

    if bare.is_empty() {
        config.default_model.clone()
    } else {
        bare
    }
}

/// Normalize a Responses API body for the Codex backend.
///
/// The endpoint only accepts streaming requests and rejects server-side storage, so
/// those two fields are forced. Encrypted reasoning must be requested explicitly or
/// multi-turn tool calls lose their reasoning context.
pub fn normalize_request_body(body: &mut Value, model: &str) {
    if !body.is_object() {
        *body = json!({});
    }
    let obj = body.as_object_mut().expect("object");

    obj.insert("model".to_string(), Value::String(model.to_string()));
    obj.insert("stream".to_string(), Value::Bool(true));
    obj.insert("store".to_string(), Value::Bool(false));

    // `previous_response_id` only works together with server-side storage.
    obj.remove("previous_response_id");

    let include_has_reasoning = obj
        .get("include")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .any(|i| i.as_str() == Some("reasoning.encrypted_content"))
        })
        .unwrap_or(false);

    if !include_has_reasoning {
        let mut include = obj
            .get("include")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        include.push(Value::String("reasoning.encrypted_content".to_string()));
        obj.insert("include".to_string(), Value::Array(include));
    }
}

/// Usage numbers extracted from the terminal `response.completed` SSE event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageTotals {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: u32,
}

/// Pull usage out of a Responses API `usage` object.
pub fn parse_usage(usage: &Value) -> UsageTotals {
    let as_u32 = |value: Option<&Value>| -> u32 {
        value
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(u32::MAX as u64) as u32
    };

    UsageTotals {
        input_tokens: as_u32(usage.get("input_tokens")),
        output_tokens: as_u32(usage.get("output_tokens")),
        cached_tokens: as_u32(
            usage
                .get("input_tokens_details")
                .and_then(|d| d.get("cached_tokens")),
        ),
    }
}

/// Incremental SSE scanner: feeds it raw chunks, keeps the final response object and
/// usage totals when the stream completes.
#[derive(Default)]
pub struct SseInspector {
    buffer: String,
    pub usage: Option<UsageTotals>,
    pub final_response: Option<Value>,
}

impl SseInspector {
    pub fn push_chunk(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);

        // Process complete lines only; a `data:` payload may be split across chunks.
        while let Some(newline) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=newline).collect();
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            self.inspect_line(&line);
        }
    }

    fn inspect_line(&mut self, line: &str) {
        let payload = match line.strip_prefix("data:") {
            Some(rest) => rest.trim(),
            None => return,
        };
        if payload.is_empty() || payload == "[DONE]" {
            return;
        }
        let Ok(event) = serde_json::from_str::<Value>(payload) else {
            return;
        };

        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if event_type != "response.completed" && event_type != "response.incomplete" {
            return;
        }

        if let Some(response) = event.get("response") {
            if let Some(usage) = response.get("usage") {
                self.usage = Some(parse_usage(usage));
            }
            self.final_response = Some(response.clone());
        }
    }
}

// ============================================================================
// Account selection
// ============================================================================

/// A ChatGPT account ready to serve a request.
pub struct PreparedAccount {
    pub account: Account,
    pub access_token: String,
}

/// List pool candidates, least-recently-used first, skipping unusable ones.
fn candidate_accounts(state: &AppState, model: &str, exclude: &[String]) -> Vec<Account> {
    let mut accounts = crate::modules::account::list_accounts_by_provider(AccountProvider::Openai)
        .unwrap_or_default();

    accounts.retain(|account| {
        if account.disabled || account.proxy_disabled {
            return false;
        }
        if exclude.contains(&account.id) {
            return false;
        }
        if account.token.refresh_token.trim().is_empty() {
            return false;
        }
        // Reuse the shared tracker so ChatGPT cooldowns show up in the same UI as
        // Antigravity ones. NB: the tracker-only variant — `is_rate_limited_sync` takes an
        // async lock via `blocking_read` and would panic here.
        if state
            .token_manager
            .is_openai_rate_limited(&account.id, Some(model))
        {
            return false;
        }
        true
    });

    accounts.sort_by_key(|account| account.last_used);
    accounts
}

/// Pick the next ChatGPT account and make sure its access token is fresh.
pub async fn prepare_account(
    state: &AppState,
    model: &str,
    exclude: &[String],
) -> Result<PreparedAccount, String> {
    let candidates = candidate_accounts(state, model, exclude);

    if candidates.is_empty() {
        return Err(
            "No usable ChatGPT account in the pool (all disabled, rate-limited or missing a refresh token)"
                .to_string(),
        );
    }

    let mut last_error = String::new();
    for account in candidates {
        match openai_account::ensure_fresh_access_token(&account).await {
            Ok(access_token) => {
                // Re-read: the refresh may have rotated tokens and bumped account info.
                let account = crate::modules::account::load_account(&account.id).unwrap_or(account);
                return Ok(PreparedAccount {
                    account,
                    access_token,
                });
            }
            Err(e) => {
                crate::modules::logger::log_warn(&format!(
                    "[OpenAI] Skipping account {}: {}",
                    account.email, e
                ));
                last_error = e;
            }
        }
    }

    Err(if last_error.is_empty() {
        "No usable ChatGPT account in the pool".to_string()
    } else {
        format!("All ChatGPT accounts failed: {}", last_error)
    })
}

// ============================================================================
// Forwarding
// ============================================================================

fn upstream_url(config: &crate::proxy::OpenAiConfig, path: &str) -> String {
    let base = config.base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{}/{}", base, path)
}

fn session_id_from(incoming: &HeaderMap) -> String {
    for name in ["session_id", "session-id", "x-session-id"] {
        if let Some(value) = incoming.get(name).and_then(|v| v.to_str().ok()) {
            if !value.trim().is_empty() {
                return value.to_string();
            }
        }
    }
    uuid::Uuid::new_v4().to_string()
}

/// One upstream attempt against a single account.
struct AttemptOutcome {
    status: StatusCode,
    response: Response,
    /// Set when the attempt should be retried on another account.
    retryable: bool,
}

#[allow(clippy::too_many_arguments)]
async fn attempt_once(
    state: &AppState,
    config: &crate::proxy::OpenAiConfig,
    prepared: &PreparedAccount,
    path: &str,
    incoming_headers: &HeaderMap,
    body_bytes: Vec<u8>,
    model: &str,
    client_wants_stream: bool,
    wire: ClientWire,
) -> AttemptOutcome {
    let url = upstream_url(config, path);
    // Emulated TLS client: chatgpt.com sits behind Cloudflare and a plain fingerprint
    // gets challenged.
    let client = crate::utils::http::get_long_client();

    let mut request = client
        .post(&url)
        .header(header::CONTENT_TYPE.as_str(), "application/json")
        .header(header::ACCEPT.as_str(), "text/event-stream")
        .header(
            header::AUTHORIZATION.as_str(),
            format!("Bearer {}", prepared.access_token),
        )
        .header("originator", openai_oauth::originator())
        .header("session-id", session_id_from(incoming_headers));

    // The real Codex CLI sends no `OpenAI-Beta` on the SSE responses path (that header is
    // websocket-only upstream), so we send none either. A User-Agent is only set when the
    // operator provides one — inventing a client version would be worse than staying with
    // the HTTP client default.
    if let Ok(user_agent) = std::env::var("ABV_OPENAI_USER_AGENT") {
        if !user_agent.trim().is_empty() {
            request = request.header(header::USER_AGENT.as_str(), user_agent);
        }
    }

    if let Some(account_id) = prepared.account.chatgpt_account_id() {
        request = request.header("ChatGPT-Account-ID", account_id);
    }
    if prepared
        .account
        .openai
        .as_ref()
        .map(|info| info.is_fedramp)
        .unwrap_or(false)
    {
        request = request.header("X-OpenAI-Fedramp", "true");
    }

    let response = match request.body(body_bytes).send().await {
        Ok(response) => response,
        Err(e) => {
            let message = format!("ChatGPT upstream request failed: {}", e);
            crate::modules::logger::log_error(&format!("[OpenAI] {}", message));
            return AttemptOutcome {
                status: StatusCode::BAD_GATEWAY,
                response: (StatusCode::BAD_GATEWAY, message).into_response(),
                retryable: true,
            };
        }
    };

    let status =
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    // Rate-limit headers arrive on every response, including errors.
    let snapshot = openai_account::parse_rate_limit_headers(|name| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string())
    });
    if !snapshot.is_empty() {
        let plan = prepared
            .account
            .openai
            .as_ref()
            .and_then(|info| info.plan_type.clone());
        if let Err(e) = openai_account::apply_rate_limit_snapshot(
            &prepared.account.id,
            &snapshot,
            plan.as_deref(),
        ) {
            crate::modules::logger::log_warn(&format!(
                "[OpenAI] Failed to store rate limits for {}: {}",
                prepared.account.email, e
            ));
        }
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let retryable = matches!(
            status.as_u16(),
            429 | 500 | 502 | 503 | 504 // transient or quota-exhausted
        ) || status.as_u16() == 401;

        crate::modules::logger::log_warn(&format!(
            "[OpenAI] Upstream {} for {} ({}): {}",
            status,
            prepared.account.email,
            model,
            body.chars().take(300).collect::<String>()
        ));

        if status.as_u16() == 429 {
            // Park the account for the window the upstream reported, else a default.
            let cooldown = snapshot
                .primary
                .as_ref()
                .and_then(|w| w.resets_in_seconds)
                .unwrap_or(300)
                .clamp(30, 6 * 3600) as u64;
            state.token_manager.mark_openai_rate_limited(
                &prepared.account.id,
                Some(model.to_string()),
                cooldown,
            );
        }

        if status.as_u16() == 401 {
            state.token_manager.record_failure(&prepared.account.id);
        }

        return AttemptOutcome {
            status,
            response: (status, body).into_response(),
            retryable,
        };
    }

    state.token_manager.record_success(&prepared.account.id);
    state
        .token_manager
        .mark_account_success(&prepared.account.id);

    let account_email = prepared.account.email.clone();
    let account_id = prepared.account.id.clone();
    let model_for_stats = model.to_string();

    if client_wants_stream {
        // Relay the SSE stream while sniffing the terminal event for usage. For chat
        // clients the events are rewritten into chat.completion.chunk on the way out.
        let mut inspector = SseInspector::default();
        let mut translator = match wire {
            ClientWire::Chat => Some(ChatStreamTranslator::new(model)),
            ClientWire::Responses => None,
        };
        // async_stream (rather than `.map`) so the chat stream can still be terminated
        // after the upstream ends — a stream that stops without `response.completed`
        // would otherwise leave a chat client waiting for `[DONE]` forever.
        let stream = async_stream::stream! {
            let mut upstream = response.bytes_stream();

            while let Some(chunk) = upstream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        inspector.push_chunk(&text);
                        if let Some(usage) = inspector.usage.take() {
                            record_usage(&account_email, &model_for_stats, usage);
                        }
                        match translator.as_mut() {
                            Some(translator) => {
                                let translated = translator.push_chunk(&text);
                                if !translated.is_empty() {
                                    yield Ok::<Bytes, std::io::Error>(Bytes::from(translated));
                                }
                            }
                            None => yield Ok::<Bytes, std::io::Error>(bytes),
                        }
                    }
                    Err(e) => {
                        match translator.as_mut() {
                            // Close the chat stream properly instead of leaking a raw
                            // error frame a chat client cannot parse.
                            Some(translator) => {
                                let tail = translator.finish();
                                if !tail.is_empty() {
                                    yield Ok::<Bytes, std::io::Error>(Bytes::from(tail));
                                }
                            }
                            None => {
                                yield Ok::<Bytes, std::io::Error>(Bytes::from(format!(
                                    "data: {{\"type\":\"error\",\"error\":{{\"message\":\"ChatGPT stream error: {}\"}}}}\n\n",
                                    e
                                )));
                            }
                        }
                        break;
                    }
                }
            }

            // Upstream finished. For chat clients, make sure the stream is terminated.
            if let Some(translator) = translator.as_mut() {
                let tail = translator.finish();
                if !tail.is_empty() {
                    yield Ok::<Bytes, std::io::Error>(Bytes::from(tail));
                }
            }
        };

        let built = Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from_stream(stream));

        return match built {
            Ok(response) => AttemptOutcome {
                status,
                response,
                retryable: false,
            },
            Err(_) => AttemptOutcome {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                response: (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to build ChatGPT stream response",
                )
                    .into_response(),
                retryable: false,
            },
        };
    }

    // Non-streaming client: the endpoint still streams, so aggregate here and hand back
    // the final `response` object.
    let text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            return AttemptOutcome {
                status: StatusCode::BAD_GATEWAY,
                response: (
                    StatusCode::BAD_GATEWAY,
                    format!("ChatGPT upstream stream failed: {}", e),
                )
                    .into_response(),
                retryable: true,
            };
        }
    };

    let mut inspector = SseInspector::default();
    inspector.push_chunk(&text);
    // Flush a trailing line without a newline.
    inspector.push_chunk("\n");

    if let Some(usage) = inspector.usage {
        record_usage(&account_email, &model_for_stats, usage);
    }
    let _ = account_id;

    match inspector.final_response {
        Some(final_response) => {
            let payload = match wire {
                ClientWire::Chat => responses_to_chat_completion(&final_response, model),
                ClientWire::Responses => final_response,
            };
            AttemptOutcome {
                status,
                response: (status, axum::Json(payload)).into_response(),
                retryable: false,
            }
        }
        None => AttemptOutcome {
            status: StatusCode::BAD_GATEWAY,
            response: (
                StatusCode::BAD_GATEWAY,
                "ChatGPT upstream ended without a completed response".to_string(),
            )
                .into_response(),
            retryable: true,
        },
    }
}

fn record_usage(email: &str, model: &str, usage: UsageTotals) {
    if usage.input_tokens == 0 && usage.output_tokens == 0 {
        return;
    }
    if let Err(e) = crate::modules::token_stats::record_usage_with_provider(
        email,
        model,
        usage.input_tokens,
        usage.output_tokens,
        usage.cached_tokens,
        crate::models::AccountProvider::Openai,
    ) {
        crate::modules::logger::log_warn(&format!("[OpenAI] Failed to record usage: {}", e));
    }
}

/// Forward a Responses API request to the Codex backend, failing over across accounts.
pub async fn forward_responses(
    state: &AppState,
    path: &str,
    incoming_headers: &HeaderMap,
    body: Value,
) -> Response {
    let client_wants_stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    forward(
        state,
        path,
        incoming_headers,
        body,
        ClientWire::Responses,
        client_wants_stream,
    )
    .await
}

/// Forward a chat/completions request: converted to Responses on the way in and back to
/// chat.completion(.chunk) on the way out.
pub async fn forward_chat_completions(
    state: &AppState,
    incoming_headers: &HeaderMap,
    chat_body: Value,
) -> Response {
    let config = state.openai.read().await.clone();
    let requested_model = chat_body.get("model").and_then(|v| v.as_str());
    let model = resolve_model(&config, requested_model);
    let client_wants_stream = chat_body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let responses_body = chat_to_responses(&chat_body, &model);

    forward(
        state,
        "responses",
        incoming_headers,
        responses_body,
        ClientWire::Chat,
        client_wants_stream,
    )
    .await
}

async fn forward(
    state: &AppState,
    path: &str,
    incoming_headers: &HeaderMap,
    mut body: Value,
    wire: ClientWire,
    client_wants_stream: bool,
) -> Response {
    let config = state.openai.read().await.clone();

    if !config.enabled {
        return (
            StatusCode::BAD_REQUEST,
            "ChatGPT provider is disabled".to_string(),
        )
            .into_response();
    }

    let requested_model = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model = resolve_model(&config, requested_model.as_deref());

    normalize_request_body(&mut body, &model);
    let body_bytes = match serde_json::to_vec(&body) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to serialize request for ChatGPT: {}", e),
            )
                .into_response()
        }
    };

    let max_attempts = config.max_failover_accounts.max(1);
    let mut tried: Vec<String> = Vec::new();
    let mut last_response: Option<Response> = None;

    for attempt in 0..max_attempts {
        let prepared = match prepare_account(state, &model, &tried).await {
            Ok(prepared) => prepared,
            Err(e) => {
                return last_response
                    .unwrap_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, e).into_response())
            }
        };

        tried.push(prepared.account.id.clone());

        let outcome = attempt_once(
            state,
            &config,
            &prepared,
            path,
            incoming_headers,
            body_bytes.clone(),
            &model,
            client_wants_stream,
            wire,
        )
        .await;

        if !outcome.retryable {
            return outcome.response;
        }

        crate::modules::logger::log_warn(&format!(
            "[OpenAI] Attempt {}/{} on {} returned {}, failing over",
            attempt + 1,
            max_attempts,
            prepared.account.email,
            outcome.status
        ));
        last_response = Some(outcome.response);
    }

    last_response.unwrap_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "All ChatGPT accounts failed".to_string(),
        )
            .into_response()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::{OpenAiConfig, OpenAiDispatchMode};

    fn config(mode: OpenAiDispatchMode, enabled: bool) -> OpenAiConfig {
        OpenAiConfig {
            enabled,
            dispatch_mode: mode,
            ..Default::default()
        }
    }

    #[test]
    fn openai_routing_respects_dispatch_mode() {
        let off = config(OpenAiDispatchMode::Off, true);
        assert!(!should_route_to_openai(&off, Some("openai:gpt-5.1")));

        let disabled = config(OpenAiDispatchMode::Auto, false);
        assert!(!should_route_to_openai(&disabled, Some("gpt-5.1")));

        let prefix = config(OpenAiDispatchMode::Prefix, true);
        assert!(should_route_to_openai(&prefix, Some("codex:gpt-5.1")));
        assert!(!should_route_to_openai(&prefix, Some("gpt-5.1")));

        let auto = config(OpenAiDispatchMode::Auto, true);
        assert!(should_route_to_openai(&auto, Some("gpt-5.1-codex")));
        assert!(should_route_to_openai(&auto, Some("openai:whatever")));
        assert!(!should_route_to_openai(&auto, Some("gemini-3-pro")));
        assert!(!should_route_to_openai(&auto, Some("claude-sonnet-4")));

        let exclusive = config(OpenAiDispatchMode::Exclusive, true);
        assert!(should_route_to_openai(&exclusive, Some("gemini-3-pro")));
        assert!(should_route_to_openai(&exclusive, None));
    }

    #[test]
    fn openai_strips_routing_prefixes_preserving_case() {
        assert_eq!(
            strip_routing_prefix("openai:GPT-5.1-Codex").as_deref(),
            Some("GPT-5.1-Codex")
        );
        assert_eq!(
            strip_routing_prefix("Codex:gpt-5").as_deref(),
            Some("gpt-5")
        );
        assert_eq!(strip_routing_prefix("gpt-5"), None);
    }

    #[test]
    fn openai_resolves_model_through_mapping_and_default() {
        let mut cfg = config(OpenAiDispatchMode::Auto, true);
        cfg.model_mapping
            .insert("gpt-5.1".to_string(), "gpt-5.1-codex".to_string());

        assert_eq!(resolve_model(&cfg, Some("openai:gpt-5.1")), "gpt-5.1-codex");
        assert_eq!(resolve_model(&cfg, Some("gpt-5.1")), "gpt-5.1-codex");
        assert_eq!(resolve_model(&cfg, Some("gpt-6-mini")), "gpt-6-mini");
        assert_eq!(resolve_model(&cfg, None), cfg.default_model);
        assert_eq!(resolve_model(&cfg, Some("   ")), cfg.default_model);
    }

    #[test]
    fn openai_normalizes_body_for_codex_endpoint() {
        let mut body = json!({
            "model": "whatever",
            "stream": false,
            "store": true,
            "previous_response_id": "resp_1",
            "input": []
        });
        normalize_request_body(&mut body, "gpt-5.1-codex");

        assert_eq!(body["model"], json!("gpt-5.1-codex"));
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["store"], json!(false));
        assert!(body.get("previous_response_id").is_none());
        assert_eq!(
            body["include"],
            json!(["reasoning.encrypted_content"]),
            "encrypted reasoning must be requested"
        );

        // Idempotent: a second pass must not duplicate the include entry.
        normalize_request_body(&mut body, "gpt-5.1-codex");
        assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
    }

    #[test]
    fn openai_normalizes_non_object_body() {
        let mut body = json!("nonsense");
        normalize_request_body(&mut body, "gpt-5.1-codex");
        assert_eq!(body["model"], json!("gpt-5.1-codex"));
        assert_eq!(body["stream"], json!(true));
    }

    #[test]
    fn openai_sse_inspector_extracts_usage_across_chunk_boundaries() {
        let event = json!({
            "type": "response.completed",
            "response": {
                "id": "resp_123",
                "usage": {
                    "input_tokens": 120,
                    "output_tokens": 45,
                    "input_tokens_details": { "cached_tokens": 100 }
                }
            }
        });
        let payload = format!("data: {}\n\n", serde_json::to_string(&event).unwrap());
        let (first, second) = payload.split_at(payload.len() / 2);

        let mut inspector = SseInspector::default();
        inspector.push_chunk(first);
        assert!(inspector.usage.is_none(), "must wait for the full line");
        inspector.push_chunk(second);

        let usage = inspector.usage.expect("usage after full event");
        assert_eq!(usage.input_tokens, 120);
        assert_eq!(usage.output_tokens, 45);
        assert_eq!(usage.cached_tokens, 100);
        assert_eq!(
            inspector
                .final_response
                .and_then(|r| r["id"].as_str().map(str::to_string)),
            Some("resp_123".to_string())
        );
    }

    #[test]
    fn openai_sse_inspector_ignores_noise() {
        let mut inspector = SseInspector::default();
        inspector.push_chunk("event: response.output_text.delta\n");
        inspector.push_chunk("data: [DONE]\n");
        inspector.push_chunk("data: not-json\n");
        inspector.push_chunk("data: {\"type\":\"response.output_text.delta\"}\n");
        assert!(inspector.usage.is_none());
        assert!(inspector.final_response.is_none());
    }

    #[test]
    fn openai_parses_usage_defaults_to_zero() {
        let usage = parse_usage(&json!({}));
        assert_eq!(usage, UsageTotals::default());
    }
}
