use std::sync::Arc;

use axum::{
    extract::{Path, State},
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
    dto::{AdjustBalanceRequest, AdjustBalanceResponse},
    service::WalletService,
};

/// POST /api/v1/admin/users/:id/wallet/adjust
///
/// 需要权限：`WalletAdjust`（super_admin / operator / finance）
pub async fn adjust_balance_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<AdjustBalanceRequest>,
) -> Response {
    // 1. RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::WalletAdjust) {
        return err_response(e, req_ctx.request_id());
    }

    // 2. 参数校验
    if let Err(e) = validate_request(&req) {
        return err_response(e, req_ctx.request_id());
    }

    // 3. 调用 WalletService
    let svc: &Arc<WalletService> = &state.wallet_service;
    match svc
        .adjust_balance(ctx.admin_id, user_id, req.amount, &req.reason)
        .await
    {
        Ok((new_balance, delta)) => ApiResponse::ok(
            AdjustBalanceResponse {
                user_id: user_id.to_string(),
                new_balance,
                delta,
            },
            req_ctx.request_id(),
        )
        .into_response(),
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

/// 参数校验：amount、reason 合法性检查
fn validate_request(
    req: &AdjustBalanceRequest,
) -> Result<(), crate::common::error::AppError> {
    use crate::common::error::AppError;

    // amount 不能为 0
    if req.amount == 0 {
        return Err(AppError::ValidationError(
            "amount 不能为 0".to_string(),
        ));
    }

    // amount 绝对值不能超过 10,000,000
    if req.amount.abs() > 10_000_000 {
        return Err(AppError::ValidationError(
            "amount 绝对值不能超过 10,000,000".to_string(),
        ));
    }

    // reason 必填且长度 2~200
    let reason_len = req.reason.chars().count();
    if reason_len < 2 {
        return Err(AppError::ValidationError(
            "reason 长度不能少于 2 个字符".to_string(),
        ));
    }
    if reason_len > 200 {
        return Err(AppError::ValidationError(
            "reason 长度不能超过 200 个字符".to_string(),
        ));
    }

    Ok(())
}

// ─── Handler 单元测试 ──────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::error::AppError;

    fn make_req(amount: i64, reason: &str) -> AdjustBalanceRequest {
        AdjustBalanceRequest {
            amount,
            reason: reason.to_string(),
        }
    }

    // ── VR-01: 正常请求 → Ok ─────────────────────────────────────────────────
    #[test]
    fn vr01_valid_request_passes() {
        assert!(validate_request(&make_req(1000, "运营补偿")).is_ok());
        assert!(validate_request(&make_req(-500, "扣款")).is_ok());
        assert!(validate_request(&make_req(10_000_000, "最大值")).is_ok());
        assert!(validate_request(&make_req(-10_000_000, "最小值")).is_ok());
    }

    // ── VR-02: amount=0 → ValidationError ───────────────────────────────────
    #[test]
    fn vr02_amount_zero_returns_validation_error() {
        let err = validate_request(&make_req(0, "有原因")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    // ── VR-03: amount 超限 → ValidationError ─────────────────────────────────
    #[test]
    fn vr03_amount_exceeds_limit_returns_validation_error() {
        let err = validate_request(&make_req(20_000_000, "超限")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
        let err2 = validate_request(&make_req(-20_000_000, "超限")).unwrap_err();
        assert!(matches!(err2, AppError::ValidationError(_)));
    }

    // ── VR-04: reason 为空 → ValidationError ─────────────────────────────────
    #[test]
    fn vr04_empty_reason_returns_validation_error() {
        let err = validate_request(&make_req(100, "")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    // ── VR-05: reason 单字符 → ValidationError（< 2 字符）─────────────────────
    #[test]
    fn vr05_single_char_reason_returns_validation_error() {
        let err = validate_request(&make_req(100, "a")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    // ── VR-06: reason 恰好 2 字符 → Ok ──────────────────────────────────────
    #[test]
    fn vr06_two_char_reason_passes() {
        assert!(validate_request(&make_req(100, "ab")).is_ok());
    }

    // ── VR-07: reason 超过 200 字符 → ValidationError ────────────────────────
    #[test]
    fn vr07_reason_over_200_chars_returns_validation_error() {
        let long_reason = "a".repeat(201);
        let err = validate_request(&make_req(100, &long_reason)).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }
}
