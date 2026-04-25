use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
    modules::audit::controller::extract_ip,
};

use super::{
    repo::{KickLogItem, MuteLogItem},
    service::GovernanceQueryParams,
};

// ─── HTTP Query 参数（Deserialize from query string）────────────────────────

/// GET query params（kicks 和 mutes 共用）
#[derive(Debug, Deserialize, Default)]
pub struct GovernanceHttpQuery {
    pub room_id: Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub operator_user_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    /// 仅 mutes 有效
    #[serde(rename = "type")]
    pub mute_type: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl From<GovernanceHttpQuery> for GovernanceQueryParams {
    fn from(q: GovernanceHttpQuery) -> Self {
        Self {
            room_id: q.room_id,
            target_user_id: q.target_user_id,
            operator_user_id: q.operator_user_id,
            from: q.from,
            to: q.to,
            mute_type: q.mute_type,
            page: q.page,
            limit: q.limit,
        }
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/governance/kicks
///
/// 查询踢人审计日志。
/// 权限：super_admin / operator / cs 可查；finance → 403。
pub async fn list_kicks_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Query(query): Query<GovernanceHttpQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::GovernanceRead) {
        return err_response(e, rc.request_id());
    }

    let ip = extract_ip(&headers);
    let params: GovernanceQueryParams = query.into();

    match state
        .governance_service
        .query_kicks(params, ctx.admin_id, ip)
        .await
    {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/admin/governance/mutes
///
/// 查询禁言审计日志。
/// 权限：super_admin / operator / cs 可查；finance → 403。
pub async fn list_mutes_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Query(query): Query<GovernanceHttpQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::GovernanceRead) {
        return err_response(e, rc.request_id());
    }

    let ip = extract_ip(&headers);
    let params: GovernanceQueryParams = query.into();

    match state
        .governance_service
        .query_mutes(params, ctx.admin_id, ip)
        .await
    {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── T-10016 #5 / T-20014 #4: CSV 导出（R1 P1-6）─────────────────────────────

/// CSV 导出最大行数硬上限（单次请求）。
/// 上限 5000 条与 limit clamp 保持自洽（不暴露未限制的全表导出）。
const CSV_EXPORT_MAX_ROWS: i64 = 5000;

/// `type` 查询参数：`kick` | `mute` | `mic` | `chat` | None（默认全部）
fn parse_export_type(t: Option<&str>) -> Result<ExportType, String> {
    match t.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => Ok(ExportType::All),
        Some("kick") => Ok(ExportType::Kick),
        Some("mute") => Ok(ExportType::Mute(None)),
        Some("mic") => Ok(ExportType::Mute(Some("mic".into()))),
        Some("chat") => Ok(ExportType::Mute(Some("chat".into()))),
        Some(other) => Err(format!(
            "invalid type '{other}': must be 'kick' | 'mute' | 'mic' | 'chat'"
        )),
    }
}

#[derive(Debug, Clone)]
enum ExportType {
    All,
    Kick,
    /// inner Option<String>：mute_type 子过滤（mic / chat / None=全部）
    Mute(Option<String>),
}

/// 把客户端可能注入的非法字符（CR / LF / 双引号）从 filename 中剥离。
/// 返回安全可用于 `Content-Disposition: filename="..."` 的字符串。
fn safe_filename_segment(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\r' | '\n' | '"' | '\\' | '/' | '\0'))
        .collect()
}

/// 将单元格内容按 RFC 4180 转义：含逗号 / 引号 / 换行 → 双引号包裹 + 内部引号双写。
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

/// 把 KickLogItem 序列化为一行 CSV（按导出列顺序）
fn kick_row(k: &KickLogItem) -> String {
    [
        csv_escape(&k.created_at.to_rfc3339()),
        csv_escape("kick"),
        csv_escape(""),
        csv_escape(&k.room_id.to_string()),
        csv_escape(&k.room_title),
        csv_escape(&k.target_user_id.to_string()),
        csv_escape(&k.target_nickname),
        csv_escape(&k.operator_user_id.to_string()),
        csv_escape(&k.operator_nickname),
        csv_escape(""),
        csv_escape(k.reason.as_deref().unwrap_or("")),
    ]
    .join(",")
}

/// 把 MuteLogItem 序列化为一行 CSV
fn mute_row(m: &MuteLogItem) -> String {
    let dur = m
        .duration_sec
        .map(|d| d.to_string())
        .unwrap_or_default();
    [
        csv_escape(&m.created_at.to_rfc3339()),
        csv_escape("mute"),
        csv_escape(&m.mute_type),
        csv_escape(&m.room_id.to_string()),
        csv_escape(&m.room_title),
        csv_escape(&m.target_user_id.to_string()),
        csv_escape(&m.target_nickname),
        csv_escape(&m.operator_user_id.to_string()),
        csv_escape(&m.operator_nickname),
        csv_escape(&dur),
        csv_escape(m.reason.as_deref().unwrap_or("")),
    ]
    .join(",")
}

const CSV_HEADER: &str =
    "created_at,action,mute_type,room_id,room_title,target_user_id,target_nickname,operator_user_id,operator_nickname,duration_sec,reason";

/// GET /api/v1/admin/governance/logs.csv
///
/// 导出治理日志（kick + mute 合并）为 CSV 流。
///
/// 行为约定（R1 P1-6 / T-10016 #5 / T-20014 #4）：
/// - body 首字节写入 `\xEF\xBB\xBF` UTF-8 BOM，便于 Excel 直接识别中文
/// - `Content-Type: text/csv; charset=utf-8`
/// - `Content-Disposition: attachment; filename="governance-logs-YYYYMMDD.csv"`（filename 经
///   [`safe_filename_segment`] 转义，剥离 CR/LF/双引号）
/// - 单次最多导出 [`CSV_EXPORT_MAX_ROWS`] 行（对 kick 与 mute 各自限流）
/// - 通过 `audit_logger` 记录操作类型 `governance.logs.export`
/// - 权限：`GovernanceRead`（与 `/kicks` / `/mutes` 一致）
pub async fn export_logs_csv_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Query(query): Query<GovernanceHttpQuery>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::GovernanceRead) {
        return err_response(e, rc.request_id());
    }

    let ip = extract_ip(&headers);

    // 解析 type （独立于 mutes 接口的 mute_type 字段，二者表达不同维度）
    let export_type = match parse_export_type(query.mute_type.as_deref()) {
        Ok(t) => t,
        Err(msg) => {
            return err_response(
                crate::common::error::AppError::ValidationError(msg),
                rc.request_id(),
            );
        }
    };

    // 走 service 复用 from/to/window 90 天校验、mute_type 白名单等
    let mut params: GovernanceQueryParams = query.into();
    // CSV 不分页；强制 page=1 + limit=MAX
    params.page = Some(1);
    params.limit = Some(CSV_EXPORT_MAX_ROWS);
    // type 维度由 export_type 接管，避免 service 层把 type=kick 当作 mute_type 校验失败
    params.mute_type = match &export_type {
        ExportType::Mute(Some(sub)) => Some(sub.clone()),
        _ => None,
    };

    // 拉取数据（kick / mute 两侧根据 export_type 决定是否调用）
    let kicks_result = match export_type {
        ExportType::All | ExportType::Kick => {
            match state
                .governance_service
                .query_kicks(params.clone(), ctx.admin_id, ip.clone())
                .await
            {
                Ok(resp) => Some(resp),
                Err(e) => return err_response(e, rc.request_id()),
            }
        }
        _ => None,
    };

    let mutes_result = match export_type {
        ExportType::All | ExportType::Mute(_) => {
            match state
                .governance_service
                .query_mutes(params.clone(), ctx.admin_id, ip.clone())
                .await
            {
                Ok(resp) => Some(resp),
                Err(e) => return err_response(e, rc.request_id()),
            }
        }
        _ => None,
    };

    // ── 构造 CSV body ────────────────────────────────────────────────────────
    let mut body: Vec<u8> = Vec::with_capacity(8192);
    // T-10016 #5 / T-20014 #4: UTF-8 BOM 必须为前 3 字节
    body.extend_from_slice(b"\xEF\xBB\xBF");
    body.extend_from_slice(CSV_HEADER.as_bytes());
    body.push(b'\n');

    if let Some(k) = &kicks_result {
        for item in &k.items {
            body.extend_from_slice(kick_row(item).as_bytes());
            body.push(b'\n');
        }
    }
    if let Some(m) = &mutes_result {
        for item in &m.items {
            body.extend_from_slice(mute_row(item).as_bytes());
            body.push(b'\n');
        }
    }

    // 审计：governance.logs.export
    state
        .governance_service
        .audit_export(ctx.admin_id, ip, &params)
        .await;

    let date = Utc::now().format("%Y%m%d").to_string();
    let filename = safe_filename_segment(&format!("governance-logs-{date}.csv"));

    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
                .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
        )
        .body(axum::body::Body::from(body))
        .unwrap();
    resp.headers_mut().insert(
        "X-Request-ID",
        HeaderValue::from_str(rc.request_id())
            .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );
    resp
}

// ─── 单元/集成测试（G16-06 handler 层 + R1 P1-6 CSV 导出）──────────────────
#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use uuid::Uuid;
    use voice_room_shared::jwt::token::{encode_token, AdminClaims};

    use crate::bootstrap::{build_app, AppState};

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn make_jwt(role: &str) -> String {
        let claims = AdminClaims {
            sub: Uuid::new_v4().to_string(),
            role: role.to_string(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        encode_token(&claims, "test-secret".as_bytes()).unwrap()
    }

    /// G16-06-handler: finance 角色访问 kicks → 403
    #[tokio::test]
    async fn h_g16_06_finance_kicks_forbidden() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("finance");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/kicks")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// G16-06-handler: finance 角色访问 mutes → 403
    #[tokio::test]
    async fn h_g16_06_finance_mutes_forbidden() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("finance");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/mutes")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// super_admin 访问 kicks → 200
    #[tokio::test]
    async fn h_super_admin_kicks_ok() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/kicks")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// cs 角色访问 mutes → 200
    #[tokio::test]
    async fn h_cs_mutes_ok() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("cs");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/mutes")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// 无 JWT → 401
    #[tokio::test]
    async fn h_no_auth_returns_401() {
        let state = AppState::for_test();
        let app = build_app(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/kicks")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ─── R1 P1-6: CSV 导出 ───────────────────────────────────────────────────

    /// CSV-01: super_admin 导出 logs.csv → 200 + UTF-8 BOM 前 3 字节
    #[tokio::test]
    async fn csv01_export_logs_returns_utf8_bom_first_3_bytes() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/logs.csv")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let ct = resp
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.starts_with("text/csv"),
            "CSV-01: Content-Type must be text/csv, got {ct}"
        );

        let cd = resp
            .headers()
            .get("Content-Disposition")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        assert!(
            cd.starts_with("attachment; filename=\"governance-logs-")
                && cd.ends_with(".csv\""),
            "CSV-01: filename pattern mismatch: {cd}"
        );

        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            body_bytes.len() >= 3,
            "CSV-01: body should at least contain BOM"
        );
        assert_eq!(
            &body_bytes[..3],
            &[0xEFu8, 0xBB, 0xBF],
            "CSV-01: first 3 bytes must be UTF-8 BOM EF BB BF"
        );

        // 接下来必须紧跟 CSV 表头
        let after_bom = std::str::from_utf8(&body_bytes[3..]).unwrap();
        assert!(
            after_bom.starts_with(
                "created_at,action,mute_type,room_id,room_title,target_user_id"
            ),
            "CSV-01: header line missing or malformed: {after_bom}"
        );
    }

    /// CSV-02: finance 角色导出 logs.csv → 403
    #[tokio::test]
    async fn csv02_finance_export_forbidden() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("finance");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/logs.csv")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// CSV-03: type=invalid → 400 ValidationError
    #[tokio::test]
    async fn csv03_invalid_type_returns_400() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/logs.csv?type=foo")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// CSV-04: type=mic → 200，仍然写入 BOM 与表头（mutes 子集）
    #[tokio::test]
    async fn csv04_mic_filter_export_ok() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/logs.csv?type=mic")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body_bytes[..3], &[0xEFu8, 0xBB, 0xBF]);
    }

    /// CSV-05: 无 JWT → 401
    #[tokio::test]
    async fn csv05_no_auth_returns_401() {
        let state = AppState::for_test();
        let app = build_app(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/logs.csv")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
