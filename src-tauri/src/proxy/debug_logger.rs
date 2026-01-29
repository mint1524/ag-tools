use serde_json::Value;
use tokio::fs;
use std::path::PathBuf;
use futures::StreamExt;

use crate::proxy::config::DebugLoggingConfig;

fn build_filename(prefix: &str, trace_id: Option<&str>) -> String {
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f");
    let tid = trace_id.unwrap_or("unknown");
    format!("{}_{}_{}.json", ts, tid, prefix)
}

fn resolve_output_dir(cfg: &DebugLoggingConfig) -> Option<PathBuf> {
    if let Some(dir) = cfg.output_dir.as_ref() {
        return Some(PathBuf::from(dir));
    }
    if let Ok(data_dir) = crate::modules::account::get_data_dir() {
        return Some(data_dir.join("debug_logs"));
    }
    None
}

pub async fn write_debug_payload(
    cfg: &DebugLoggingConfig,
    trace_id: Option<&str>,
    prefix: &str,
    payload: &Value,
) {
    if !cfg.enabled {
        return;
    }

    let output_dir = match resolve_output_dir(cfg) {
        Some(dir) => dir,
        None => {
            tracing::warn!("[Debug-Log] Enabled but output_dir is not available.");
            return;
        }
    };

    if let Err(e) = fs::create_dir_all(&output_dir).await {
        tracing::warn!("[Debug-Log] Failed to create output dir: {}", e);
        return;
    }

    let filename = build_filename(prefix, trace_id);
    let path = output_dir.join(filename);

    match serde_json::to_vec_pretty(payload) {
        Ok(bytes) => {
            if let Err(e) = fs::write(&path, bytes).await {
                tracing::warn!("[Debug-Log] Failed to write file: {}", e);
            }
        }
        Err(e) => {
            tracing::warn!("[Debug-Log] Failed to serialize payload: {}", e);
        }
    }
}

pub fn is_enabled(cfg: &DebugLoggingConfig) -> bool {
    cfg.enabled
}

pub fn wrap_reqwest_stream_with_debug(
    stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    cfg: DebugLoggingConfig,
    trace_id: String,
    prefix: &'static str,
    meta: Value,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>> {
    if !is_enabled(&cfg) {
        return stream;
    }

    let wrapped = async_stream::stream! {
        let mut collected: Vec<u8> = Vec::new();
        let mut inner = stream;
        while let Some(item) = inner.next().await {
            if let Ok(bytes) = &item {
                collected.extend_from_slice(bytes);
            }
            yield item;
        }

        let response_text = String::from_utf8_lossy(&collected).to_string();
        let payload = serde_json::json!({
            "kind": "upstream_response",
            "trace_id": trace_id,
            "meta": meta,
            "response_text": response_text,
        });

        write_debug_payload(&cfg, Some(&payload["trace_id"].as_str().unwrap_or("unknown")), prefix, &payload).await;
    };

    Box::pin(wrapped)
}
