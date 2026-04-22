//! members_handler — GET /api/v1/rooms/:id/members（T-00027）
//!
//! 返回带角色 + 麦位信息的房间成员分页列表。
//! 需 JWT 鉴权；仅房间内连接中的用户可调（非成员 → 403）。

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext,
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
    modules::room::members_service::{
        MembersRoomRepo, MembersService, MembersUserRepo, RoomOwnerInfo, UserInfo,
    },
};

// ─── 查询参数 ─────────────────────────────────────────────────────────────────

/// GET 参数：?page=1&limit=20
#[derive(Debug, Deserialize)]
pub struct MembersQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}
fn default_limit() -> u32 {
    20
}

// ─── 生产适配器 ───────────────────────────────────────────────────────────────

/// 将 `AuthService` 适配为 `MembersUserRepo`。
///
/// 通过 `auth_service.get_user_by_id()` 逐一查询（MVP，100 人房间 1 次/人，
/// 可接受；后续可优化为批量 SQL）。
struct AuthServiceUserAdapter(Arc<crate::modules::auth::service::AuthService>);

#[async_trait]
impl MembersUserRepo for AuthServiceUserAdapter {
    async fn find_users_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserInfo>, AppError> {
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(user) = self.0.get_user_by_id(*id).await? {
                result.push(UserInfo {
                    id: user.id,
                    nickname: user.nickname,
                    avatar: user.avatar,
                });
            }
        }
        Ok(result)
    }
}

/// 将 `RoomService` 适配为 `MembersRoomRepo`。
struct RoomServiceRoomAdapter(Arc<crate::modules::room::RoomService>);

#[async_trait]
impl MembersRoomRepo for RoomServiceRoomAdapter {
    async fn find_room_owner(&self, room_id: Uuid) -> Result<Option<RoomOwnerInfo>, AppError> {
        let model = self.0.get_active_room_model(room_id).await?;
        Ok(model.map(|r| RoomOwnerInfo {
            owner_id: r.owner_id,
            admin_user_id: r.admin_user_id,
        }))
    }
}

// ─── Handler ─────────────────────────────────────────────────────────────────

/// GET /api/v1/rooms/:id/members（需要 JWT 鉴权）
///
/// 成功：HTTP 200 + MembersListResponse
/// 失败：
/// - 400/40003 page=0
/// - 401 未登录
/// - 403/40301 非房间成员
/// - 404/40400 房间不存在
pub async fn list_members_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Path(id): Path<String>,
    Query(query): Query<MembersQuery>,
) -> axum::response::Response {
    // ── 1. 解析 room_id ──────────────────────────────────────────────────────
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid room id format: {id:?}")),
                rc.request_id(),
            );
        }
    };

    // ── 2. 构建 service（inline adapter） ───────────────────────────────────
    let svc = MembersService::new(
        state.room_manager.clone(),
        Arc::new(AuthServiceUserAdapter(state.auth_service.clone())),
        Arc::new(RoomServiceRoomAdapter(state.room_service.clone())),
    );

    // ── 3. 调用 service ──────────────────────────────────────────────────────
    match svc
        .list_members(room_id, ctx.user_id, query.page, query.limit)
        .await
    {
        Ok(resp) => (
            axum::http::StatusCode::OK,
            Json(ApiResponse::ok(resp, rc.request_id())),
        )
            .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}
