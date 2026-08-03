//! Account provider (fork addition).
//!
//! Upstream models an account as implicitly Google (Antigravity). This fork adds
//! OpenAI (ChatGPT subscription accounts, the same credentials Codex CLI uses) as a
//! second provider that lives in the very same account pool.
//!
//! Backwards compatibility is a hard requirement: existing `accounts.json` /
//! `accounts/<id>.json` files carry no `provider` field, and a user may roll the
//! container image back to an upstream build at any time. Therefore `provider`
//! deserializes to `Google` when missing and serializes as a plain lowercase string.

use serde::{Deserialize, Serialize};

/// Which upstream a pooled account authenticates against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountProvider {
    /// Google / Antigravity OAuth account (upstream default).
    #[default]
    Google,
    /// OpenAI ChatGPT account (Codex OAuth: subscription-backed, not an API key).
    #[serde(alias = "chatgpt")]
    Openai,
}

impl AccountProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountProvider::Google => "google",
            AccountProvider::Openai => "openai",
        }
    }

    /// Human-facing label used in logs and UI badges.
    pub fn label(&self) -> &'static str {
        match self {
            AccountProvider::Google => "Google",
            AccountProvider::Openai => "ChatGPT",
        }
    }

    pub fn is_openai(&self) -> bool {
        matches!(self, AccountProvider::Openai)
    }

    pub fn is_google(&self) -> bool {
        matches!(self, AccountProvider::Google)
    }

    /// Parse a provider from user input / query strings. Unknown values fall back to
    /// `None` so callers can decide between "all providers" and an error.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "google" | "antigravity" | "gemini" => Some(AccountProvider::Google),
            "openai" | "chatgpt" | "codex" => Some(AccountProvider::Openai),
            _ => None,
        }
    }
}

impl std::fmt::Display for AccountProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// OpenAI-specific account facts, parsed out of the `id_token` JWT at login and on
/// every refresh. Kept in its own struct so the Google account shape is untouched.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAiAccountInfo {
    /// `chatgpt_account_id` claim — sent as the `ChatGPT-Account-ID` request header.
    /// Without it the Codex backend rejects or misroutes requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_id: Option<String>,
    /// `chatgpt_user_id` claim (stable user identifier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatgpt_user_id: Option<String>,
    /// Subscription plan: free / plus / pro / business / enterprise / edu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    /// Workspace requires the FedRAMP edge (`X-OpenAI-Fedramp: true`).
    #[serde(default)]
    pub is_fedramp: bool,
    /// Unix timestamp of the last successful refresh, for diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_provider_defaults_to_google_when_missing() {
        #[derive(Deserialize)]
        struct Holder {
            #[serde(default)]
            provider: AccountProvider,
        }

        let holder: Holder = serde_json::from_str("{}").unwrap();
        assert_eq!(holder.provider, AccountProvider::Google);
    }

    #[test]
    fn openai_provider_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&AccountProvider::Openai).unwrap(),
            "\"openai\""
        );
        assert_eq!(
            serde_json::to_string(&AccountProvider::Google).unwrap(),
            "\"google\""
        );
    }

    #[test]
    fn openai_provider_accepts_chatgpt_alias() {
        let parsed: AccountProvider = serde_json::from_str("\"chatgpt\"").unwrap();
        assert_eq!(parsed, AccountProvider::Openai);
        assert_eq!(
            AccountProvider::parse("Codex"),
            Some(AccountProvider::Openai)
        );
        assert_eq!(
            AccountProvider::parse("antigravity"),
            Some(AccountProvider::Google)
        );
        assert_eq!(AccountProvider::parse("nope"), None);
    }
}
