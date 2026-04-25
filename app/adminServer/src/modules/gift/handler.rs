use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

use super::{
    dto::{
        CreateGiftRequest, GiftResponse, ListGiftsQuery, ListGiftsResponse, UpdateGiftRequest,
        UploadGiftFileResponse,
    },
    service::{validate_file_upload, GiftService},
};

// ─── GET /api/v1/admin/gifts ──────────────────────────────────────────────────

/// 礼物列表查询（含未上架选项）。
///
/// 权限：`GiftWrite`（super_admin + operator）
pub async fn list_gifts_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Query(query): Query<ListGiftsQuery>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::GiftWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let include_inactive = query.include_inactive.unwrap_or(false);
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);

    let svc: &Arc<GiftService> = &state.gift_service;
    match svc.list_gifts(include_inactive, page, size).await {
        Ok((total, gifts)) => {
            let items: Vec<GiftResponse> = gifts.into_iter().map(GiftResponse::from).collect();
            ApiResponse::ok(
                ListGiftsResponse {
                    total,
                    page,
                    size,
                    items,
                },
                req_ctx.request_id(),
            )
            .into_response()
        }
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── POST /api/v1/admin/gifts ─────────────────────────────────────────────────

/// 新增礼物。
///
/// 权限：`GiftWrite`（super_admin + operator）
pub async fn create_gift_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Json(req): Json<CreateGiftRequest>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::GiftWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let svc: &Arc<GiftService> = &state.gift_service;
    match svc.create_gift(req).await {
        Ok(gift) => {
            // 写审计日志（fire-and-forget）
            state
                .audit_logger
                .log_action(
                    ctx.admin_id,
                    "gift_create",
                    Some("gift"),
                    Some(gift.id),
                    None,
                    Some(serde_json::json!({ "code": gift.code })),
                )
                .await;

            // HTTP 201 Created
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "code": 0,
                    "message": "ok",
                    "data": GiftResponse::from(gift),
                    "request_id": req_ctx.request_id(),
                })),
            )
                .into_response()
        }
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── PUT /api/v1/admin/gifts/:id ─────────────────────────────────────────────

/// 更新礼物（含 is_active 上/下架切换）。
///
/// 权限：`GiftWrite`（super_admin + operator）
pub async fn update_gift_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateGiftRequest>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::GiftWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let svc: &Arc<GiftService> = &state.gift_service;
    // MEDIUM-2: 在 move req 进 service 之前，先序列化变更字段用于 audit detail
    let detail_changes = serde_json::to_value(&req).unwrap_or_default();
    match svc.update_gift(id, req).await {
        Ok(gift) => {
            // 写审计日志：记录变更内容（before id + after changes）
            state
                .audit_logger
                .log_action(
                    ctx.admin_id,
                    "gift_update",
                    Some("gift"),
                    Some(id),
                    None,
                    Some(serde_json::json!({
                        "id": id.to_string(),
                        "changes": detail_changes,
                    })),
                )
                .await;

            ApiResponse::ok(GiftResponse::from(gift), req_ctx.request_id()).into_response()
        }
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── DELETE /api/v1/admin/gifts/:id ──────────────────────────────────────────

/// 软删除礼物（is_deleted=true）。
///
/// 权限：`GiftDelete`（仅 super_admin）
pub async fn delete_gift_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::GiftDelete) {
        return err_response(e, req_ctx.request_id());
    }

    let svc: &Arc<GiftService> = &state.gift_service;
    match svc.delete_gift(id).await {
        Ok(gift) => {
            // MEDIUM-2: 写审计日志，记录被删除礼物的关键字段（code、name_en）便于审计溯源
            state
                .audit_logger
                .log_action(
                    ctx.admin_id,
                    "gift_delete",
                    Some("gift"),
                    Some(id),
                    None,
                    Some(serde_json::json!({
                        "id": id.to_string(),
                        "code": gift.code,
                        "name_en": gift.name_en,
                        "is_active": gift.is_active,
                    })),
                )
                .await;

            ApiResponse::ok(serde_json::json!(null), req_ctx.request_id()).into_response()
        }
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── POST /api/v1/admin/gifts/upload ─────────────────────────────────────────

/// 上传礼物图片或 Lottie 动效文件。
///
/// 权限：`GiftWrite`（super_admin + operator）
///
/// multipart/form-data 字段：
/// - `file`: 文件内容，MIME 必须为白名单之一
/// - `kind`: `icon` | `animation`
///
/// 文件大小限制：图片 ≤ 1MB，Lottie JSON ≤ 2MB
pub async fn upload_gift_file_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::GiftWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let mut file_content_type: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut kind: Option<String> = None;

    // 解析 multipart 字段
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                return err_response(
                    crate::common::error::AppError::ValidationError(format!(
                        "multipart 解析失败: {e}"
                    )),
                    req_ctx.request_id(),
                );
            }
        };

        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_content_type = field.content_type().map(|ct| ct.to_string());
                file_name = field.file_name().map(|n| n.to_string());
                match field.bytes().await {
                    Ok(bytes) => file_data = Some(bytes.to_vec()),
                    Err(e) => {
                        return err_response(
                            crate::common::error::AppError::ValidationError(format!(
                                "读取文件失败: {e}"
                            )),
                            req_ctx.request_id(),
                        );
                    }
                }
            }
            "kind" => {
                kind = field.text().await.ok();
            }
            _ => {} // 忽略未知字段
        }
    }

    // 校验必要字段
    let content_type = match file_content_type {
        Some(ct) if !ct.is_empty() => ct,
        _ => {
            return err_response(
                crate::common::error::AppError::ValidationError(
                    "缺少文件或 Content-Type".to_string(),
                ),
                req_ctx.request_id(),
            );
        }
    };
    let data = match file_data {
        Some(d) => d,
        None => {
            return err_response(
                crate::common::error::AppError::ValidationError("缺少 file 字段".to_string()),
                req_ctx.request_id(),
            );
        }
    };

    // 校验 MIME 类型和文件大小（GC07, GC08）
    if let Err(e) = validate_file_upload(&content_type, data.len()) {
        return err_response(e, req_ctx.request_id());
    }

    // 确定文件扩展名
    let ext = match content_type.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "application/json" => "json",
        _ => "bin",
    };

    // 生成存储路径（HIGH-1: upload_dir 取自 gift_service，保持职责内聚）
    let today = chrono::Utc::now().format("%Y-%m-%d");
    let file_id = Uuid::new_v4();
    let sub_dir = format!("{}/{}", state.gift_service.upload_dir, today);
    let file_path = format!("{sub_dir}/{file_id}.{ext}");
    let url = format!("/uploads/gifts/{today}/{file_id}.{ext}");

    // 创建目录并写入文件（HIGH-1: 使用 tokio 异步 I/O，避免阻塞 Tokio 运行时）
    if let Err(e) = tokio::fs::create_dir_all(&sub_dir).await {
        tracing::error!(error = %e, path = %sub_dir, "创建上传目录失败");
        return err_response(
            crate::common::error::AppError::Internal(format!("创建目录失败: {e}")),
            req_ctx.request_id(),
        );
    }
    if let Err(e) = tokio::fs::write(&file_path, &data).await {
        tracing::error!(error = %e, path = %file_path, "写入文件失败");
        return err_response(
            crate::common::error::AppError::Internal(format!("写入文件失败: {e}")),
            req_ctx.request_id(),
        );
    }

    // 写审计日志
    state
        .audit_logger
        .log_action(
            ctx.admin_id,
            "gift_upload",
            None,
            None,
            None,
            Some(serde_json::json!({
                "url": url,
                "kind": kind.unwrap_or_default(),
                "file_name": file_name.unwrap_or_default(),
                "content_type": content_type,
                "size_bytes": data.len(),
            })),
        )
        .await;

    ApiResponse::ok(UploadGiftFileResponse { url }, req_ctx.request_id()).into_response()
}

// ─── 单元测试（Handler 层校验逻辑）───────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gift::service::{validate_create_request, validate_file_upload};

    fn valid_req(code: &str) -> CreateGiftRequest {
        CreateGiftRequest {
            code: code.to_string(),
            name_en: "Test".to_string(),
            name_ar: "تست".to_string(),
            icon_url: "/uploads/gifts/test.png".to_string(),
            price: 10,
            tier: 2,
            effect_level: None,
            animation_url: None,
            sort_order: None,
            is_active: None,
        }
    }

    /// HV-01: 有效请求通过验证
    #[test]
    fn hv01_valid_create_request_passes() {
        assert!(validate_create_request(&valid_req("rose_01")).is_ok());
    }

    /// HV-02: price=0 → ValidationError（GC09）
    #[test]
    fn hv02_price_zero_fails() {
        let mut req = valid_req("t1");
        req.price = 0;
        assert!(validate_create_request(&req).is_err());
    }

    /// HV-03: price=-1 → ValidationError
    #[test]
    fn hv03_negative_price_fails() {
        let mut req = valid_req("t2");
        req.price = -1;
        assert!(validate_create_request(&req).is_err());
    }

    /// HV-04: tier=6 → ValidationError（GC10）
    #[test]
    fn hv04_tier_out_of_range_fails() {
        let mut req = valid_req("t3");
        req.tier = 6;
        assert!(validate_create_request(&req).is_err());
    }

    /// HV-05: tier=0 → ValidationError
    #[test]
    fn hv05_tier_zero_fails() {
        let mut req = valid_req("t4");
        req.tier = 0;
        assert!(validate_create_request(&req).is_err());
    }

    /// HV-06: 非白名单 MIME → ValidationError（GC07）
    #[test]
    fn hv06_non_whitelist_mime_fails() {
        assert!(validate_file_upload("image/gif", 100).is_err());
        assert!(validate_file_upload("video/mp4", 100).is_err());
        assert!(validate_file_upload("text/plain", 100).is_err());
    }

    /// HV-07: 图片文件 >1MB → ValidationError（GC08）
    #[test]
    fn hv07_image_over_1mb_fails() {
        let over_1mb = 1024 * 1024 + 1;
        assert!(validate_file_upload("image/png", over_1mb).is_err());
        assert!(validate_file_upload("image/jpeg", over_1mb).is_err());
        assert!(validate_file_upload("image/webp", over_1mb).is_err());
    }

    /// HV-08: 图片 1MB（恰好）→ 通过
    #[test]
    fn hv08_image_exactly_1mb_passes() {
        assert!(validate_file_upload("image/png", 1024 * 1024).is_ok());
    }

    /// HV-09: Lottie >2MB → ValidationError
    #[test]
    fn hv09_lottie_over_2mb_fails() {
        let over_2mb = 2 * 1024 * 1024 + 1;
        assert!(validate_file_upload("application/json", over_2mb).is_err());
    }

    /// HV-10: 全部白名单 MIME 通过
    #[test]
    fn hv10_all_allowed_mimes_pass_small_size() {
        for mime in &["image/png", "image/jpeg", "image/webp", "application/json"] {
            assert!(
                validate_file_upload(mime, 1024).is_ok(),
                "MIME {mime} 应通过"
            );
        }
    }

    /// HV-11: icon_url 白名单校验（Review MEDIUM-1）
    /// 外部 URL 和非白名单路径应被 validate_create_request 拒绝
    #[test]
    fn hv11_icon_url_whitelist_validation() {
        // 外部 URL → 应失败
        let mut req = valid_req("r1");
        req.icon_url = "https://evil.com/hack.png".to_string();
        assert!(
            validate_create_request(&req).is_err(),
            "HV-11: 外部 URL 应被拒绝"
        );

        // 非白名单本地路径 → 应失败
        req.icon_url = "/api/images/test.png".to_string();
        assert!(
            validate_create_request(&req).is_err(),
            "HV-11: 非白名单路径应被拒绝"
        );

        // 白名单路径 → 应通过
        req.icon_url = "/uploads/gifts/test.png".to_string();
        assert!(
            validate_create_request(&req).is_ok(),
            "HV-11: 白名单路径应通过"
        );
    }
}
