use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

use super::dto::ListLogsQuery;

/// GET /api/v1/admin/logs
///
/// 查询审计日志接口，需要 LogRead 权限（super_admin / operator 可访问）。
pub async fn list_logs_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<ListLogsQuery>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::LogRead) {
        return err_response(e, rc.request_id());
    }

    match state.audit_service.list_logs(query).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// IP 地址提取辅助函数。
///
/// 优先读取 `X-Forwarded-For` 的第一个地址，备选 `X-Real-IP`。
pub fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            if let Some(first) = s.split(',').next() {
                let trimmed = first.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(s) = xri.to_str() {
            let trimmed = s.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

// ─── 单元测试（extract_ip）──────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// IP 从 X-Forwarded-For 取第一段
    #[test]
    fn extract_ip_from_x_forwarded_for_takes_first_segment() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "1.2.3.4, 5.6.7.8".parse().unwrap(),
        );
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("1.2.3.4".to_string()), "应取 XFF 的第一个 IP");
    }

    /// IP 从 X-Real-IP 备选读取
    #[test]
    fn extract_ip_from_x_real_ip_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.10.11.12".parse().unwrap());
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("9.10.11.12".to_string()), "备选 X-Real-IP 应被读取");
    }

    /// 无 IP 头时返回 None
    #[test]
    fn extract_ip_returns_none_when_no_headers() {
        let headers = HeaderMap::new();
        let ip = extract_ip(&headers);
        assert!(ip.is_none(), "无 IP 头时应返回 None");
    }

    /// X-Forwarded-For 单个 IP（无逗号）正确读取
    #[test]
    fn extract_ip_single_ip_in_xff() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.100".parse().unwrap());
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("192.168.1.100".to_string()));
    }
}
