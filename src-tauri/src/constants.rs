use std::sync::LazyLock;

/// URL to fetch the latest Antigravity version
const VERSION_URL: &str = "https://antigravity-auto-updater-974169037036.us-central1.run.app";

/// Fetch version from remote endpoint, fallback to Cargo.toml version on failure
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
    // Fallback to compile-time version from Cargo.toml
    env!("CARGO_PKG_VERSION").to_string()
}

/// Shared User-Agent string for all upstream API requests.
/// Format: antigravity/{version} {os}/{arch}
/// Version is fetched from remote endpoint, with Cargo.toml as fallback.
/// OS and architecture are detected at runtime.
pub static USER_AGENT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "antigravity/{} {}/{}",
        fetch_remote_version(),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
});
