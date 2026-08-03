//! ChatGPT (OpenAI) OAuth — fork addition.
//!
//! Authenticates *subscription* ChatGPT accounts, i.e. the exact credentials the Codex
//! CLI obtains, not platform API keys. Two flows are implemented:
//!
//! 1. **Device code** (`request_device_code` + `wait_for_device_code`) — the primary flow
//!    for this deployment: the server never needs a browser or a reachable redirect URI,
//!    the user just opens a URL and types a short code.
//! 2. **PKCE authorization code** (`build_authorize_url` + `exchange_code`) — mirrors the
//!    existing Google flow, including the "paste the callback URL back" escape hatch used
//!    when the app runs in Docker and `http://localhost:1455/...` cannot be served.
//!
//! Endpoints and parameters follow the openai/codex sources (`codex-rs/login`): the auth
//! surface is not publicly documented, so everything that could drift is overridable via
//! environment variables and kept in this single module.

use base64::Engine;
use serde::{Deserialize, Serialize};

/// Default issuer. Override with `ABV_OPENAI_ISSUER`.
const DEFAULT_ISSUER: &str = "https://auth.openai.com";
/// Public Codex CLI client id. Override with `ABV_OPENAI_CLIENT_ID`.
const DEFAULT_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
/// The redirect URI registered for the Codex CLI client; the port is fixed upstream.
const DEFAULT_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPE: &str = "openid profile email offline_access";
/// Refresh this long before the access token actually expires.
pub const TOKEN_REFRESH_SKEW_SECONDS: i64 = 300;
/// Fallback lifetime when the token endpoint omits `expires_in` and the JWT has no `exp`.
const FALLBACK_EXPIRES_IN: i64 = 3600;

pub fn issuer() -> String {
    std::env::var("ABV_OPENAI_ISSUER")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_ISSUER.to_string())
}

pub fn client_id() -> String {
    std::env::var("ABV_OPENAI_CLIENT_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string())
}

pub fn default_redirect_uri() -> String {
    std::env::var("ABV_OPENAI_REDIRECT_URI")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_string())
}

fn token_url() -> String {
    format!("{}/oauth/token", issuer())
}

fn authorize_url() -> String {
    format!("{}/oauth/authorize", issuer())
}

/// Device-auth endpoints live under `/api/accounts`, the verification page under `/codex/device`.
fn device_api_base() -> String {
    format!("{}/api/accounts", issuer())
}

fn auth_http_client() -> rquest::Client {
    // Pure-native client (no TLS emulation): auth.openai.com is a plain API surface and
    // the emulated fingerprint buys nothing here.
    crate::utils::http::get_long_standard_client()
}

// ============================================================================
// PKCE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkceCodes {
    pub code_verifier: String,
    pub code_challenge: String,
}

fn random_url_safe(bytes: usize) -> String {
    use rand::RngCore;
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

impl PkceCodes {
    pub fn generate() -> Self {
        use sha2::{Digest, Sha256};
        let code_verifier = random_url_safe(64);
        let digest = Sha256::digest(code_verifier.as_bytes());
        let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        Self {
            code_verifier,
            code_challenge,
        }
    }
}

/// Random `state` value for CSRF protection.
pub fn generate_state() -> String {
    random_url_safe(32)
}

/// Build the authorization URL for the PKCE flow.
pub fn build_authorize_url(pkce: &PkceCodes, state: &str, redirect_uri: &str) -> String {
    let params = [
        ("response_type", "code"),
        ("client_id", &client_id()),
        ("redirect_uri", redirect_uri),
        ("scope", SCOPE),
        ("code_challenge", pkce.code_challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("state", state),
        ("originator", originator().as_str()),
    ];

    match url::Url::parse_with_params(&authorize_url(), params) {
        Ok(url) => url.to_string(),
        // parse_with_params only fails on an invalid base, which would mean a broken
        // ABV_OPENAI_ISSUER; surface it as an unusable URL rather than panicking.
        Err(e) => {
            crate::modules::logger::log_error(&format!(
                "[OpenAI OAuth] Failed to build authorize URL: {}",
                e
            ));
            String::new()
        }
    }
}

/// `originator` header/param value identifying the client to OpenAI.
pub fn originator() -> String {
    std::env::var("ABV_OPENAI_ORIGINATOR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "codex_cli_rs".to_string())
}

/// Extract the `code` (and optional `state`) from either a bare code or a full
/// callback URL pasted by the user.
pub fn extract_code_from_input(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if let Ok(parsed) = url::Url::parse(trimmed) {
            let mut code = None;
            let mut state = None;
            for (k, v) in parsed.query_pairs() {
                match k.as_ref() {
                    "code" => code = Some(v.to_string()),
                    "state" => state = Some(v.to_string()),
                    _ => {}
                }
            }
            if let Some(code) = code {
                return (code, state);
            }
        }
    }
    (trimmed.to_string(), None)
}

// ============================================================================
// Tokens
// ============================================================================

#[derive(Debug, Clone)]
pub struct OpenAiTokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    /// Absolute expiry (unix seconds) derived from `expires_in` or the JWT `exp` claim.
    pub expiry_timestamp: i64,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct TokenEndpointResponse {
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Facts parsed out of the `id_token` JWT.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdTokenInfo {
    pub email: Option<String>,
    pub chatgpt_account_id: Option<String>,
    pub chatgpt_user_id: Option<String>,
    pub plan_type: Option<String>,
    pub is_fedramp: bool,
}

#[derive(Debug, Deserialize)]
struct IdClaims {
    #[serde(default)]
    email: Option<String>,
    #[serde(rename = "https://api.openai.com/profile", default)]
    profile: Option<ProfileClaims>,
    #[serde(rename = "https://api.openai.com/auth", default)]
    auth: Option<AuthClaims>,
}

#[derive(Debug, Deserialize)]
struct ProfileClaims {
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthClaims {
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
    #[serde(default)]
    chatgpt_user_id: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    chatgpt_account_is_fedramp: bool,
}

#[derive(Debug, Deserialize)]
struct StandardClaims {
    #[serde(default)]
    exp: Option<i64>,
}

/// Decode a JWT payload without signature verification.
///
/// These tokens are handed to us over TLS by the issuer and are only read for
/// routing metadata (account id, plan, expiry) — we never make a trust decision
/// based on them, so verification would add a JWKS dependency for no benefit.
fn decode_jwt_payload<T: serde::de::DeserializeOwned>(jwt: &str) -> Result<T, String> {
    let mut parts = jwt.split('.');
    let payload = match (parts.next(), parts.next(), parts.next()) {
        (Some(h), Some(p), Some(s)) if !h.is_empty() && !p.is_empty() && !s.is_empty() => p,
        _ => return Err("invalid JWT format".to_string()),
    };

    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| format!("invalid JWT payload encoding: {}", e))?;

    serde_json::from_slice(&bytes).map_err(|e| format!("invalid JWT payload JSON: {}", e))
}

pub fn parse_id_token(jwt: &str) -> Result<IdTokenInfo, String> {
    let claims: IdClaims = decode_jwt_payload(jwt)?;
    let email = claims
        .email
        .or_else(|| claims.profile.and_then(|p| p.email));

    Ok(match claims.auth {
        Some(auth) => IdTokenInfo {
            email,
            chatgpt_account_id: auth.chatgpt_account_id,
            chatgpt_user_id: auth.chatgpt_user_id.or(auth.user_id),
            plan_type: auth.chatgpt_plan_type,
            is_fedramp: auth.chatgpt_account_is_fedramp,
        },
        None => IdTokenInfo {
            email,
            ..Default::default()
        },
    })
}

/// Expiry (unix seconds) from a JWT `exp` claim, if present.
pub fn jwt_expiry(jwt: &str) -> Option<i64> {
    decode_jwt_payload::<StandardClaims>(jwt).ok().and_then(|c| c.exp)
}

fn build_tokens(
    resp: TokenEndpointResponse,
    fallback_refresh_token: Option<&str>,
) -> Result<OpenAiTokens, String> {
    let access_token = resp
        .access_token
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| "token endpoint returned no access_token".to_string())?;

    // The refresh token rotates on every refresh; when the response omits it, the
    // previous one stays valid.
    let refresh_token = resp
        .refresh_token
        .filter(|t| !t.trim().is_empty())
        .or_else(|| fallback_refresh_token.map(|t| t.to_string()))
        .ok_or_else(|| "token endpoint returned no refresh_token".to_string())?;

    let id_token = resp.id_token.unwrap_or_default();

    let now = chrono::Utc::now().timestamp();
    let (expiry_timestamp, expires_in) = match resp.expires_in {
        Some(expires_in) if expires_in > 0 => (now + expires_in, expires_in),
        _ => match jwt_expiry(&access_token) {
            Some(exp) if exp > now => (exp, exp - now),
            _ => (now + FALLBACK_EXPIRES_IN, FALLBACK_EXPIRES_IN),
        },
    };

    Ok(OpenAiTokens {
        id_token,
        access_token,
        refresh_token,
        expiry_timestamp,
        expires_in,
    })
}

/// Exchange an authorization code (PKCE or device-auth issued) for tokens.
pub async fn exchange_code(
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<OpenAiTokens, String> {
    let client = auth_http_client();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", &client_id()),
        ("code_verifier", code_verifier),
    ];

    let response = client
        .post(token_url())
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("OpenAI token exchange request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!(
            "OpenAI token exchange failed ({}): {}",
            status,
            summarize_error_body(&body)
        ));
    }

    let parsed: TokenEndpointResponse = serde_json::from_str(&body)
        .map_err(|e| format!("OpenAI token response parsing failed: {}", e))?;

    crate::modules::logger::log_info("[OpenAI OAuth] Token exchange succeeded");
    build_tokens(parsed, None)
}

/// Classification of a refresh failure, so callers know whether to retry or to
/// disable the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshFailure {
    /// Network/5xx — worth retrying later, the account stays enabled.
    Transient(String),
    /// Refresh token revoked/expired/reused — the account needs a fresh login.
    Permanent(String),
}

impl std::fmt::Display for RefreshFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshFailure::Transient(m) => write!(f, "{}", m),
            RefreshFailure::Permanent(m) => write!(f, "{}", m),
        }
    }
}

impl RefreshFailure {
    pub fn is_permanent(&self) -> bool {
        matches!(self, RefreshFailure::Permanent(_))
    }
}

/// Refresh an access token. The endpoint takes JSON (not form-encoded) and rotates
/// the refresh token, so callers must persist whatever comes back.
pub async fn refresh_tokens(refresh_token: &str) -> Result<OpenAiTokens, RefreshFailure> {
    let client = auth_http_client();
    let payload = serde_json::json!({
        "client_id": client_id(),
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
    });

    let response = client
        .post(token_url())
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            RefreshFailure::Transient(format!("OpenAI token refresh request failed: {}", e))
        })?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        let detail = summarize_error_body(&body);
        let message = format!("OpenAI token refresh failed ({}): {}", status, detail);
        return Err(if status.as_u16() == 400 || status.as_u16() == 401 {
            RefreshFailure::Permanent(message)
        } else {
            RefreshFailure::Transient(message)
        });
    }

    let parsed: TokenEndpointResponse = serde_json::from_str(&body).map_err(|e| {
        RefreshFailure::Transient(format!("OpenAI refresh response parsing failed: {}", e))
    })?;

    build_tokens(parsed, Some(refresh_token)).map_err(RefreshFailure::Transient)
}

fn summarize_error_body(body: &str) -> String {
    // Prefer the structured error fields when present, else a truncated body.
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        let code = value
            .get("error")
            .and_then(|v| v.as_str())
            .or_else(|| value.get("error").and_then(|e| e.get("code")?.as_str()));
        let message = value
            .get("error_description")
            .and_then(|v| v.as_str())
            .or_else(|| value.get("error").and_then(|e| e.get("message")?.as_str()))
            .or_else(|| value.get("detail").and_then(|v| v.as_str()));
        match (code, message) {
            (Some(c), Some(m)) => return format!("{}: {}", c, m),
            (Some(c), None) => return c.to_string(),
            (None, Some(m)) => return m.to_string(),
            _ => {}
        }
    }
    body.chars().take(300).collect()
}

// ============================================================================
// Device code flow
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCode {
    /// Page the user opens to enter the code.
    pub verification_url: String,
    /// Short code the user types.
    pub user_code: String,
    /// Opaque handle used when polling.
    pub device_auth_id: String,
    /// Server-suggested polling interval in seconds.
    pub interval: u64,
    /// Unix timestamp after which polling should stop.
    pub expires_at: i64,
}

#[derive(Deserialize)]
struct UserCodeResponse {
    device_auth_id: String,
    #[serde(alias = "user_code", alias = "usercode")]
    user_code: String,
    #[serde(default, deserialize_with = "deserialize_lenient_u64")]
    interval: u64,
}

/// The interval arrives as a JSON string in practice, but tolerate a number too.
fn deserialize_lenient_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s.trim().parse::<u64>().unwrap_or(5),
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(5),
        _ => 5,
    })
}

#[derive(Deserialize)]
struct DeviceTokenResponse {
    authorization_code: String,
    #[allow(dead_code)]
    code_challenge: String,
    code_verifier: String,
}

/// Device-auth lifetime: the user code is valid for 15 minutes upstream.
const DEVICE_CODE_TTL_SECONDS: i64 = 15 * 60;

/// Step 1 — ask OpenAI for a user code.
pub async fn request_device_code() -> Result<DeviceCode, String> {
    let client = auth_http_client();
    let url = format!("{}/deviceauth/usercode", device_api_base());

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "client_id": client_id() }))
        .send()
        .await
        .map_err(|e| format!("OpenAI device code request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        if status.as_u16() == 404 {
            return Err(
                "OpenAI device code login is not available (404). Use the PKCE link flow instead."
                    .to_string(),
            );
        }
        return Err(format!(
            "OpenAI device code request failed ({}): {}",
            status,
            summarize_error_body(&body)
        ));
    }

    let parsed: UserCodeResponse = serde_json::from_str(&body)
        .map_err(|e| format!("OpenAI device code response parsing failed: {}", e))?;

    Ok(DeviceCode {
        verification_url: format!("{}/codex/device", issuer()),
        user_code: parsed.user_code,
        device_auth_id: parsed.device_auth_id,
        interval: parsed.interval.clamp(1, 30),
        expires_at: chrono::Utc::now().timestamp() + DEVICE_CODE_TTL_SECONDS,
    })
}

/// Outcome of a single poll, so HTTP callers can poll without holding a connection open.
#[derive(Debug, Clone)]
pub enum DevicePollResult {
    /// The user has not approved yet.
    Pending,
    /// Approved: tokens obtained.
    Complete(OpenAiTokens),
}

/// Step 2 — poll once. `Pending` means "call again after `interval` seconds".
pub async fn poll_device_code_once(device: &DeviceCode) -> Result<DevicePollResult, String> {
    if chrono::Utc::now().timestamp() > device.expires_at {
        return Err("OpenAI device code expired, start the login again".to_string());
    }

    let client = auth_http_client();
    let url = format!("{}/deviceauth/token", device_api_base());

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "device_auth_id": device.device_auth_id,
            "user_code": device.user_code,
        }))
        .send()
        .await
        .map_err(|e| format!("OpenAI device code poll failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    // Upstream signals "not approved yet" with 403/404.
    if status.as_u16() == 403 || status.as_u16() == 404 {
        return Ok(DevicePollResult::Pending);
    }

    if !status.is_success() {
        return Err(format!(
            "OpenAI device code poll failed ({}): {}",
            status,
            summarize_error_body(&body)
        ));
    }

    let parsed: DeviceTokenResponse = serde_json::from_str(&body)
        .map_err(|e| format!("OpenAI device token response parsing failed: {}", e))?;

    // Device auth hands us both the code and the verifier it was minted with.
    let redirect_uri = format!("{}/deviceauth/callback", issuer());
    let tokens = exchange_code(
        &parsed.authorization_code,
        &redirect_uri,
        &parsed.code_verifier,
    )
    .await?;

    Ok(DevicePollResult::Complete(tokens))
}

/// Poll until approved or the code expires. Used by the blocking "login" call.
pub async fn wait_for_device_code(device: &DeviceCode) -> Result<OpenAiTokens, String> {
    loop {
        match poll_device_code_once(device).await? {
            DevicePollResult::Complete(tokens) => return Ok(tokens),
            DevicePollResult::Pending => {
                if chrono::Utc::now().timestamp() > device.expires_at {
                    return Err("OpenAI device code expired, start the login again".to_string());
                }
                tokio::time::sleep(std::time::Duration::from_secs(device.interval)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt_with_payload(payload: serde_json::Value) -> String {
        let encode = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        format!(
            "{}.{}.{}",
            encode(b"{\"alg\":\"none\"}"),
            encode(serde_json::to_string(&payload).unwrap().as_bytes()),
            encode(b"sig")
        )
    }

    #[test]
    fn openai_pkce_challenge_is_derived_from_verifier() {
        let pkce = PkceCodes::generate();
        assert!(!pkce.code_verifier.is_empty());
        assert!(!pkce.code_challenge.is_empty());
        assert_ne!(pkce.code_verifier, pkce.code_challenge);
        // S256 of a 64-byte verifier is 32 bytes -> 43 base64url chars.
        assert_eq!(pkce.code_challenge.len(), 43);
        // Deterministic for a fixed verifier.
        let second = PkceCodes::generate();
        assert_ne!(pkce.code_verifier, second.code_verifier);
    }

    #[test]
    fn openai_authorize_url_carries_pkce_and_state() {
        let pkce = PkceCodes::generate();
        let url = build_authorize_url(&pkce, "state-123", DEFAULT_REDIRECT_URI);
        assert!(url.starts_with("https://auth.openai.com/oauth/authorize?"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("code_challenge={}", pkce.code_challenge)));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"));
        assert!(url.contains("id_token_add_organizations=true"));
    }

    #[test]
    fn openai_parses_account_id_and_plan_from_id_token() {
        let jwt = jwt_with_payload(serde_json::json!({
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acct-42",
                "chatgpt_user_id": "user-7",
                "chatgpt_plan_type": "pro",
                "chatgpt_account_is_fedramp": false
            }
        }));

        let info = parse_id_token(&jwt).unwrap();
        assert_eq!(info.email.as_deref(), Some("user@example.com"));
        assert_eq!(info.chatgpt_account_id.as_deref(), Some("acct-42"));
        assert_eq!(info.chatgpt_user_id.as_deref(), Some("user-7"));
        assert_eq!(info.plan_type.as_deref(), Some("pro"));
        assert!(!info.is_fedramp);
    }

    #[test]
    fn openai_falls_back_to_profile_email_claim() {
        let jwt = jwt_with_payload(serde_json::json!({
            "https://api.openai.com/profile": { "email": "profile@example.com" }
        }));
        let info = parse_id_token(&jwt).unwrap();
        assert_eq!(info.email.as_deref(), Some("profile@example.com"));
        assert_eq!(info.chatgpt_account_id, None);
    }

    #[test]
    fn openai_rejects_malformed_id_token() {
        assert!(parse_id_token("not-a-jwt").is_err());
        assert!(parse_id_token("a.b").is_err());
    }

    #[test]
    fn openai_expiry_prefers_expires_in_then_jwt_exp() {
        let now = chrono::Utc::now().timestamp();
        let jwt = jwt_with_payload(serde_json::json!({ "exp": now + 7200 }));

        let with_expires_in = build_tokens(
            TokenEndpointResponse {
                id_token: None,
                access_token: Some(jwt.clone()),
                refresh_token: Some("r".to_string()),
                expires_in: Some(60),
            },
            None,
        )
        .unwrap();
        assert_eq!(with_expires_in.expires_in, 60);

        let from_jwt = build_tokens(
            TokenEndpointResponse {
                id_token: None,
                access_token: Some(jwt),
                refresh_token: Some("r".to_string()),
                expires_in: None,
            },
            None,
        )
        .unwrap();
        assert!(from_jwt.expires_in > 7000 && from_jwt.expires_in <= 7200);
    }

    #[test]
    fn openai_refresh_keeps_previous_refresh_token_when_omitted() {
        let tokens = build_tokens(
            TokenEndpointResponse {
                id_token: None,
                access_token: Some("access".to_string()),
                refresh_token: None,
                expires_in: Some(120),
            },
            Some("old-refresh"),
        )
        .unwrap();
        assert_eq!(tokens.refresh_token, "old-refresh");
    }

    #[test]
    fn openai_extracts_code_from_pasted_callback_url() {
        let (code, state) = extract_code_from_input(
            "  http://localhost:1455/auth/callback?code=abc123&state=xyz  ",
        );
        assert_eq!(code, "abc123");
        assert_eq!(state.as_deref(), Some("xyz"));

        let (bare, no_state) = extract_code_from_input(" plaincode ");
        assert_eq!(bare, "plaincode");
        assert_eq!(no_state, None);
    }

    #[test]
    fn openai_summarizes_structured_error_bodies() {
        let summary = summarize_error_body(
            "{\"error\":\"invalid_grant\",\"error_description\":\"token expired\"}",
        );
        assert_eq!(summary, "invalid_grant: token expired");
    }
}
