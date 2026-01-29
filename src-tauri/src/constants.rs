use std::sync::LazyLock;

/// URL to fetch the latest Antigravity version
const VERSION_URL: &str = "https://antigravity-auto-updater-974169037036.us-central1.run.app";

/// Hardcoded fallback version if all else fails
const FALLBACK_VERSION: &str = "1.15.8";

/// Parse version from response text
/// Expected format: "Auto updater is running. Stable Version: 1.15.8-5724687216017408"
fn parse_version(text: &str) -> Option<String> {
    // Try to extract version after "Version: " or "version: "
    let text_lower = text.to_lowercase();
    if let Some(idx) = text_lower.find("version:") {
        let after_version = &text[idx + 8..]; // Skip "version:"
        let trimmed = after_version.trim();
        // Take the first word/token
        let version_part = trimmed.split_whitespace().next()?;
        // Extract just the semver part (before any hyphen suffix)
        let clean_version = version_part.split('-').next().unwrap_or(version_part);
        if !clean_version.is_empty() && clean_version.contains('.') {
            return Some(clean_version.to_string());
        }
    }

    // Fallback: try to find anything that looks like a version (X.Y.Z pattern)
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if clean.contains('.') && clean.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            let version_part = clean.split('-').next().unwrap_or(clean);
            return Some(version_part.to_string());
        }
    }

    None
}

/// Fetch version from remote endpoint, with multiple fallbacks
fn fetch_remote_version() -> String {
    // Use blocking client for one-time initialization
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(client) => {
            match client.get(VERSION_URL).send() {
                Ok(response) => {
                    if let Ok(text) = response.text() {
                        if let Some(version) = parse_version(&text) {
                            return version;
                        }
                    }
                }
                Err(_) => {}
            }
        }
        Err(_) => {}
    }

    // Fallback 1: Use compile-time version from Cargo.toml if it looks like a valid version
    let cargo_version = env!("CARGO_PKG_VERSION");
    if !cargo_version.is_empty() && cargo_version.contains('.') {
        return cargo_version.to_string();
    }

    // Fallback 2: Hardcoded version as last resort
    FALLBACK_VERSION.to_string()
}

/// Shared User-Agent string for all upstream API requests.
/// Format: antigravity/{version} {os}/{arch}
/// Version priority: remote endpoint > Cargo.toml > hardcoded fallback
/// OS and architecture are detected at runtime.
pub static USER_AGENT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "antigravity/{} {}/{}",
        fetch_remote_version(),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
});
