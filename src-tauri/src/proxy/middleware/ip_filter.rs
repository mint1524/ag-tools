use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    http::StatusCode,
    body::Body,
};
use crate::proxy::server::AppState;
use crate::modules::security_db;

/// IP 黑白名单过滤中间件
pub async fn ip_filter_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // 提取客户端 IP
    let client_ip = extract_client_ip(&request);
    
    if let Some(ip) = &client_ip {
        // 读取安全配置
        let security_config = state.security.read().await;
        
        // 1. 检查白名单 (如果启用白名单模式,只允许白名单 IP)
        if security_config.security_monitor.whitelist.enabled {
            match security_db::is_ip_in_whitelist(ip) {
                Ok(true) => {
                    // 在白名单中,直接放行
                    tracing::debug!("[IP Filter] IP {} is in whitelist, allowing", ip);
                    return next.run(request).await;
                }
                Ok(false) => {
                    // 不在白名单中,且启用了白名单模式,拒绝访问
                    tracing::warn!("[IP Filter] IP {} not in whitelist, blocking", ip);
                    return create_blocked_response(
                        ip,
                        "IP not in whitelist",
                        &security_config.security_monitor.blacklist.block_message,
                    );
                }
                Err(e) => {
                    tracing::error!("[IP Filter] Failed to check whitelist: {}", e);
                }
            }
        } else {
            // 白名单优先模式: 如果在白名单中,跳过黑名单检查
            if security_config.security_monitor.whitelist.whitelist_priority {
                match security_db::is_ip_in_whitelist(ip) {
                    Ok(true) => {
                        tracing::debug!("[IP Filter] IP {} is in whitelist (priority mode), skipping blacklist check", ip);
                        return next.run(request).await;
                    }
                    Ok(false) => {
                        // 继续检查黑名单
                    }
                    Err(e) => {
                        tracing::error!("[IP Filter] Failed to check whitelist: {}", e);
                    }
                }
            }
        }

        // 2. 检查黑名单
        if security_config.security_monitor.blacklist.enabled {
            match security_db::is_ip_in_blacklist(ip) {
                Ok(true) => {
                    tracing::warn!("[IP Filter] IP {} is in blacklist, blocking", ip);
                    
                    // 记录被封禁的访问日志
                    let log = security_db::IpAccessLog {
                        id: uuid::Uuid::new_v4().to_string(),
                        client_ip: ip.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                        method: Some(request.method().to_string()),
                        path: Some(request.uri().to_string()),
                        user_agent: request
                            .headers()
                            .get("user-agent")
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.to_string()),
                        status: Some(403),
                        duration: Some(0),
                        api_key_hash: None,
                        blocked: true,
                        block_reason: Some("IP in blacklist".to_string()),
                    };
                    
                    tokio::spawn(async move {
                        if let Err(e) = security_db::save_ip_access_log(&log) {
                            tracing::error!("[IP Filter] Failed to save blocked access log: {}", e);
                        }
                    });
                    
                    let block_message = security_config.security_monitor.blacklist.block_message.clone();
                    return create_blocked_response(
                        ip,
                        "IP in blacklist",
                        &block_message,
                    );
                }
                Ok(false) => {
                    // 不在黑名单中,放行
                    tracing::debug!("[IP Filter] IP {} not in blacklist, allowing", ip);
                }
                Err(e) => {
                    tracing::error!("[IP Filter] Failed to check blacklist: {}", e);
                }
            }
        }
    } else {
        tracing::warn!("[IP Filter] Unable to extract client IP from request");
    }

    // 放行请求
    next.run(request).await
}

/// 从请求中提取客户端 IP
fn extract_client_ip(request: &Request) -> Option<String> {
    // 优先从 X-Forwarded-For 提取 (取第一个 IP)
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            // 备选从 X-Real-IP 提取
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// 创建被封禁的响应
fn create_blocked_response(ip: &str, reason: &str, custom_message: &str) -> Response {
    let message = if custom_message.is_empty() {
        format!("Access denied for IP: {}", ip)
    } else {
        custom_message.to_string()
    };
    
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": "ip_blocked",
            "code": "ip_blocked",
            "ip": ip,
            "reason": reason,
        }
    });
    
    (
        StatusCode::FORBIDDEN,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&body).unwrap_or_else(|_| message),
    )
        .into_response()
}
