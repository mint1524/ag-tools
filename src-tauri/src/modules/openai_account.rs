//! ChatGPT account lifecycle — fork addition.
//!
//! Bridges [`crate::modules::openai_oauth`] and the shared account store: turning tokens
//! into pooled accounts, keeping access tokens fresh, and translating the Codex
//! rate-limit response headers into the same `QuotaData` shape the UI already renders for
//! Google accounts.

use crate::models::{
    Account, AccountProvider, ModelQuota, OpenAiAccountInfo, QuotaData, TokenData,
};
use crate::modules::openai_oauth::{self, OpenAiTokens, RefreshFailure};

/// Build the `TokenData` for a ChatGPT account.
///
/// `oauth_client_key` stays `None` on purpose: it selects a *Google* OAuth client in
/// [`crate::modules::oauth`], and an OpenAI account must never be routed there.
fn token_data_from(tokens: &OpenAiTokens, email: Option<String>) -> TokenData {
    let mut token = TokenData::new(
        tokens.access_token.clone(),
        tokens.refresh_token.clone(),
        tokens.expires_in,
        email,
        None,
        None,
        false,
        if tokens.id_token.is_empty() {
            None
        } else {
            Some(tokens.id_token.clone())
        },
    );
    // TokenData::new recomputes expiry from `expires_in`; prefer the exact value we
    // derived from the token response / JWT.
    token.expiry_timestamp = tokens.expiry_timestamp;
    token
}

fn info_from_tokens(tokens: &OpenAiTokens) -> Result<(String, OpenAiAccountInfo), String> {
    let claims = if tokens.id_token.is_empty() {
        openai_oauth::IdTokenInfo::default()
    } else {
        openai_oauth::parse_id_token(&tokens.id_token)?
    };

    // Email is the pool's identity key. ChatGPT tokens normally carry one; fall back to
    // the account id so a workspace token without an email claim is still usable.
    let email = claims
        .email
        .clone()
        .or_else(|| {
            claims
                .chatgpt_account_id
                .as_ref()
                .map(|id| format!("chatgpt-{}", id))
        })
        .ok_or_else(|| {
            "ChatGPT token carries neither an email nor an account id claim".to_string()
        })?;

    Ok((
        email,
        OpenAiAccountInfo {
            chatgpt_account_id: claims.chatgpt_account_id,
            chatgpt_user_id: claims.chatgpt_user_id,
            plan_type: claims.plan_type,
            is_fedramp: claims.is_fedramp,
            last_refresh: Some(chrono::Utc::now().timestamp()),
        },
    ))
}

/// Create or update a pooled ChatGPT account from a fresh set of tokens.
pub fn save_account_from_tokens(tokens: &OpenAiTokens) -> Result<Account, String> {
    let (email, info) = info_from_tokens(tokens)?;

    // Display name: plan makes the pool readable at a glance ("ChatGPT Pro").
    let name = match info.plan_type.as_deref() {
        Some(plan) if !plan.is_empty() => Some(format!("ChatGPT {}", pretty_plan(plan))),
        _ => Some("ChatGPT".to_string()),
    };

    let token = token_data_from(tokens, Some(email.clone()));

    let account = crate::modules::account::upsert_provider_account(
        AccountProvider::Openai,
        &email,
        name,
        |account| {
            account.token = token;
            account.openai = Some(info);
        },
    )?;

    crate::modules::logger::log_info(&format!(
        "[OpenAI] Saved ChatGPT account {} (plan: {}, account_id: {})",
        account.email,
        account
            .openai
            .as_ref()
            .and_then(|i| i.plan_type.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        account.chatgpt_account_id().unwrap_or("none")
    ));

    Ok(account)
}

fn pretty_plan(plan: &str) -> String {
    match plan.to_ascii_lowercase().as_str() {
        "plus" => "Plus".to_string(),
        "pro" => "Pro".to_string(),
        "free" => "Free".to_string(),
        "business" => "Business".to_string(),
        "enterprise" => "Enterprise".to_string(),
        "edu" => "Edu".to_string(),
        other => other.to_string(),
    }
}

/// Shape of the Codex CLI credential file (`~/.codex/auth.json`).
#[derive(serde::Deserialize)]
struct CodexAuthFile {
    #[serde(default)]
    tokens: Option<CodexAuthTokens>,
}

#[derive(serde::Deserialize)]
struct CodexAuthTokens {
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
}

/// Import an account from a pasted Codex `auth.json`.
///
/// On a headless server this is the fastest way to add an account that is already logged
/// in somewhere else, and it needs no network round trip.
pub fn import_from_codex_auth_json(raw: &str) -> Result<Account, String> {
    let parsed: CodexAuthFile = serde_json::from_str(raw.trim())
        .map_err(|e| format!("auth.json is not valid JSON: {}", e))?;

    let tokens = parsed
        .tokens
        .ok_or_else(|| "auth.json has no \"tokens\" object".to_string())?;

    let access_token = tokens
        .access_token
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| "auth.json has no access_token".to_string())?;
    let refresh_token = tokens
        .refresh_token
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| {
            "auth.json has no refresh_token — the account could not be kept alive".to_string()
        })?;
    let id_token = tokens.id_token.unwrap_or_default();

    let now = chrono::Utc::now().timestamp();
    let expiry_timestamp = openai_oauth::jwt_expiry(&access_token).unwrap_or(now);

    save_account_from_tokens(&OpenAiTokens {
        id_token,
        access_token,
        refresh_token,
        expiry_timestamp,
        expires_in: (expiry_timestamp - now).max(0),
    })
}

/// Persist refreshed tokens for an existing account.
fn persist_refreshed(account: &Account, tokens: &OpenAiTokens) -> Result<Account, String> {
    let (_, mut info) = info_from_tokens(tokens).unwrap_or_else(|_| {
        (
            account.email.clone(),
            account.openai.clone().unwrap_or_default(),
        )
    });

    // A refresh response may omit the id_token; keep previously known facts in that case.
    if let Some(previous) = account.openai.as_ref() {
        if info.chatgpt_account_id.is_none() {
            info.chatgpt_account_id = previous.chatgpt_account_id.clone();
        }
        if info.chatgpt_user_id.is_none() {
            info.chatgpt_user_id = previous.chatgpt_user_id.clone();
        }
        if info.plan_type.is_none() {
            info.plan_type = previous.plan_type.clone();
        }
    }
    info.last_refresh = Some(chrono::Utc::now().timestamp());

    let mut token = token_data_from(tokens, Some(account.email.clone()));
    if token.id_token.is_none() {
        token.id_token = account.token.id_token.clone();
    }

    crate::modules::account::upsert_provider_account(
        AccountProvider::Openai,
        &account.email,
        None,
        |target| {
            target.token = token;
            target.openai = Some(info);
        },
    )
}

/// Whether this account's access token is (nearly) expired.
pub fn needs_refresh(account: &Account) -> bool {
    let now = chrono::Utc::now().timestamp();
    account.token.expiry_timestamp <= now + openai_oauth::TOKEN_REFRESH_SKEW_SECONDS
}

/// Return a usable access token for a ChatGPT account, refreshing it when needed.
///
/// A permanently dead refresh token disables the account (same contract the Google path
/// uses for `invalid_grant`) so the pool stops handing it out.
pub async fn ensure_fresh_access_token(account: &Account) -> Result<String, String> {
    if !account.provider.is_openai() {
        return Err(format!(
            "Account {} is not a ChatGPT account",
            account.email
        ));
    }

    if !needs_refresh(account) {
        return Ok(account.token.access_token.clone());
    }

    if account.token.refresh_token.trim().is_empty() {
        return Err(format!(
            "ChatGPT account {} has no refresh token; log in again",
            account.email
        ));
    }

    crate::modules::logger::log_info(&format!(
        "[OpenAI] Refreshing access token for {}",
        account.email
    ));

    match openai_oauth::refresh_tokens(&account.token.refresh_token).await {
        Ok(tokens) => {
            let updated = persist_refreshed(account, &tokens)?;
            Ok(updated.token.access_token)
        }
        Err(failure) => {
            let message = failure.to_string();
            if failure.is_permanent() {
                crate::modules::logger::log_error(&format!(
                    "[OpenAI] Refresh token for {} is dead, disabling account: {}",
                    account.email, message
                ));
                if let Err(e) = crate::modules::account::mark_account_forbidden(
                    &account.id,
                    &format!("ChatGPT refresh token rejected: {}", message),
                ) {
                    crate::modules::logger::log_error(&format!(
                        "[OpenAI] Failed to disable account {}: {}",
                        account.email, e
                    ));
                }
            }
            Err(message)
        }
    }
}

/// Refresh every ChatGPT account whose token is close to expiry.
/// Returns (refreshed, failed).
pub async fn refresh_all_openai_tokens() -> (usize, usize) {
    let accounts = match crate::modules::account::list_accounts_by_provider(AccountProvider::Openai)
    {
        Ok(accounts) => accounts,
        Err(e) => {
            crate::modules::logger::log_error(&format!(
                "[OpenAI] Failed to list ChatGPT accounts: {}",
                e
            ));
            return (0, 0);
        }
    };

    let mut refreshed = 0usize;
    let mut failed = 0usize;

    for account in accounts {
        if account.disabled || !needs_refresh(&account) {
            continue;
        }
        match ensure_fresh_access_token(&account).await {
            Ok(_) => refreshed += 1,
            Err(_) => failed += 1,
        }
    }

    (refreshed, failed)
}

// ============================================================================
// Rate limits / quota
// ============================================================================

/// One rate-limit window as reported by the Codex backend.
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimitWindow {
    /// Percentage of the window already consumed (0-100).
    pub used_percent: f64,
    /// Window length in minutes (e.g. 300 for the 5h window, 10080 for weekly).
    pub window_minutes: Option<i64>,
    /// Seconds until the window resets.
    pub resets_in_seconds: Option<i64>,
}

impl RateLimitWindow {
    fn remaining_percent(&self) -> i32 {
        (100.0 - self.used_percent).clamp(0.0, 100.0).round() as i32
    }

    /// RFC3339 reset timestamp, matching the Google quota format used by the UI.
    fn reset_time(&self) -> String {
        match self.resets_in_seconds {
            Some(seconds) if seconds > 0 => {
                let when = chrono::Utc::now() + chrono::Duration::seconds(seconds);
                when.to_rfc3339()
            }
            _ => String::new(),
        }
    }

    fn label(&self) -> String {
        match self.window_minutes {
            Some(minutes) if minutes >= 1440 => format!("{}d window", (minutes / 1440).max(1)),
            Some(minutes) if minutes >= 60 => format!("{}h window", minutes / 60),
            Some(minutes) => format!("{}m window", minutes),
            None => "window".to_string(),
        }
    }
}

/// Snapshot of both rate-limit windows the Codex backend reports.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RateLimitSnapshot {
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
}

impl RateLimitSnapshot {
    pub fn is_empty(&self) -> bool {
        self.primary.is_none() && self.secondary.is_none()
    }
}

/// Parse the `x-codex-*` rate-limit headers.
///
/// Takes a lookup closure instead of a `HeaderMap` so it works with both the axum and the
/// upstream HTTP client header types without coupling to either crate version.
pub fn parse_rate_limit_headers<F>(get: F) -> RateLimitSnapshot
where
    F: Fn(&str) -> Option<String>,
{
    let window = |prefix: &str| -> Option<RateLimitWindow> {
        let used_percent = get(&format!("x-codex-{}-used-percent", prefix))?
            .trim()
            .parse::<f64>()
            .ok()?;
        let window_minutes = get(&format!("x-codex-{}-window-minutes", prefix))
            .and_then(|v| v.trim().parse::<i64>().ok());
        let resets_in_seconds =
            get(&format!("x-codex-{}-reset-at", prefix)).and_then(|v| parse_reset_at(v.trim()));
        Some(RateLimitWindow {
            used_percent,
            window_minutes,
            resets_in_seconds,
        })
    };

    RateLimitSnapshot {
        primary: window("primary"),
        secondary: window("secondary"),
    }
}

/// `*-reset-at` is either "seconds from now" or an absolute unix timestamp; normalize to
/// seconds from now.
fn parse_reset_at(raw: &str) -> Option<i64> {
    let value = raw.parse::<i64>().ok()?;
    if value <= 0 {
        return None;
    }
    let now = chrono::Utc::now().timestamp();
    // Anything past ~2001 in unix-seconds terms is an absolute timestamp; a rate-limit
    // window is never a billion seconds long.
    if value > 1_000_000_000 {
        Some((value - now).max(0))
    } else {
        Some(value)
    }
}

/// Store a rate-limit snapshot as the account's quota so the existing pool UI shows it.
pub fn apply_rate_limit_snapshot(
    account_id: &str,
    snapshot: &RateLimitSnapshot,
    plan_type: Option<&str>,
) -> Result<(), String> {
    if snapshot.is_empty() {
        return Ok(());
    }

    let mut quota = QuotaData::new();
    quota.subscription_tier = plan_type.map(|p| pretty_plan(p).to_uppercase());

    if let Some(primary) = &snapshot.primary {
        quota.add_model(ModelQuota {
            name: format!("codex-primary ({})", primary.label()),
            percentage: primary.remaining_percent(),
            reset_time: primary.reset_time(),
            display_name: Some(format!("Codex primary · {}", primary.label())),
            supports_images: None,
            supports_thinking: None,
            thinking_budget: None,
            recommended: None,
            max_tokens: None,
            max_output_tokens: None,
            supported_mime_types: None,
        });
    }

    if let Some(secondary) = &snapshot.secondary {
        quota.add_model(ModelQuota {
            name: format!("codex-secondary ({})", secondary.label()),
            percentage: secondary.remaining_percent(),
            reset_time: secondary.reset_time(),
            display_name: Some(format!("Codex secondary · {}", secondary.label())),
            supports_images: None,
            supports_thinking: None,
            thinking_budget: None,
            recommended: None,
            max_tokens: None,
            max_output_tokens: None,
            supported_mime_types: None,
        });
    }

    crate::modules::account::update_account_quota(account_id, quota)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_parses_rate_limit_headers_into_windows() {
        let headers = vec![
            ("x-codex-primary-used-percent", "42.5"),
            ("x-codex-primary-window-minutes", "300"),
            ("x-codex-primary-reset-at", "600"),
            ("x-codex-secondary-used-percent", "10"),
            ("x-codex-secondary-window-minutes", "10080"),
        ];
        let snapshot = parse_rate_limit_headers(|name| {
            headers
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        });

        let primary = snapshot.primary.expect("primary window");
        assert_eq!(primary.used_percent, 42.5);
        assert_eq!(primary.remaining_percent(), 58);
        assert_eq!(primary.label(), "5h window");
        assert_eq!(primary.resets_in_seconds, Some(600));

        let secondary = snapshot.secondary.expect("secondary window");
        assert_eq!(secondary.label(), "7d window");
        assert_eq!(secondary.remaining_percent(), 90);
        assert_eq!(secondary.resets_in_seconds, None);
    }

    #[test]
    fn openai_rate_limit_snapshot_is_empty_without_headers() {
        let snapshot = parse_rate_limit_headers(|_| None);
        assert!(snapshot.is_empty());
    }

    #[test]
    fn openai_reset_at_accepts_absolute_timestamps() {
        let future = chrono::Utc::now().timestamp() + 120;
        let seconds = parse_reset_at(&future.to_string()).expect("absolute reset");
        assert!((115..=125).contains(&seconds), "got {}", seconds);
        assert_eq!(parse_reset_at("90"), Some(90));
        assert_eq!(parse_reset_at("0"), None);
        assert_eq!(parse_reset_at("nonsense"), None);
    }

    #[test]
    fn openai_import_rejects_auth_json_without_refresh_token() {
        let err = import_from_codex_auth_json("{\"tokens\":{\"access_token\":\"a\"}}")
            .expect_err("must fail");
        assert!(err.contains("refresh_token"), "unexpected error: {}", err);

        let err = import_from_codex_auth_json("not json").expect_err("must fail");
        assert!(err.contains("valid JSON"), "unexpected error: {}", err);
    }
}
