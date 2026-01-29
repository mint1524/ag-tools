use std::sync::LazyLock;

/// URL to fetch the latest Antigravity version
const VERSION_URL: &str = "https://antigravity-auto-updater-974169037036.us-central1.run.app";

/// Hardcoded fallback version if all else fails
const FALLBACK_VERSION: &str = "1.15.8";

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
                        let version = text.trim();
                        // Extract just the version part (e.g., "1.15.8" from "1.15.8-5724687216017408")
                        let clean_version = version.split('-').next().unwrap_or(version);
                        if !clean_version.is_empty() {
                            return clean_version.to_string();
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
