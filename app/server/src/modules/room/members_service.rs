//! MembersService — 观众席列表 API 业务逻辑（T-00027）
//!
//! 从 `RoomManager` 读取内存成员快照，批量查询用户信息，
//! 计算角色、排序、分页后返回响应。

use std::cmp::Ordering;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::error::AppError;
use crate::room::manager::RoomManager;

// ─── 端口（可 Fake 的依赖抽象） ───────────────────────────────────────────────

/// 用户基础信息（昵称 + 头像），供列表展示用。
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: Uuid,
    pub nickname: String,
    pub avatar: Option<String>,
}

/// 房间归属信息（owner + admin），用于计算角色。
#[derive(Debug, Clone)]
pub struct RoomOwnerInfo {
    pub owner_id: Uuid,
    pub admin_user_id: Option<Uuid>,
}

/// 批量查询用户信息的端口。
///
/// 生产实现：通过 AuthService/UserRepository 查 DB；
/// 测试实现：内存 HashMap。
#[async_trait]
pub trait MembersUserRepo: Send + Sync {
    async fn find_users_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserInfo>, AppError>;
}

/// 查询房间归属信息的端口。
///
/// 生产实现：通过 RoomService 查 DB；
/// 测试实现：内存 HashMap。
#[async_trait]
pub trait MembersRoomRepo: Send + Sync {
    async fn find_room_owner(&self, room_id: Uuid) -> Result<Option<RoomOwnerInfo>, AppError>;
}

// ─── 响应 DTO ─────────────────────────────────────────────────────────────────

/// 单个成员条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberItem {
    /// 用户 UUID（字符串）
    pub user_id: Uuid,
    /// 昵称
    pub nickname: String,
    /// 头像 URL（可空）
    pub avatar: Option<String>,
    /// 角色：`owner` | `admin` | `member`
    pub role: String,
    /// 麦位索引（None 表示观众席）
    pub mic_slot: Option<usize>,
    /// RFC3339 进房时间
    pub joined_at: String,
    /// 是否被禁麦
    pub muted_mic: bool,
    /// 是否被禁言
    pub muted_chat: bool,
}

/// 分页列表响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembersListResponse {
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub items: Vec<MemberItem>,
}

// ─── MembersService ───────────────────────────────────────────────────────────

/// 观众席列表业务服务。
pub struct MembersService {
    room_manager: Arc<RoomManager>,
    user_repo: Arc<dyn MembersUserRepo>,
    room_repo: Arc<dyn MembersRoomRepo>,
}

impl MembersService {
    /// 构造服务实例。
    pub fn new(
        room_manager: Arc<RoomManager>,
        user_repo: Arc<dyn MembersUserRepo>,
        room_repo: Arc<dyn MembersRoomRepo>,
    ) -> Self {
        Self {
            room_manager,
            user_repo,
            room_repo,
        }
    }

    /// 获取指定房间的成员分页列表。
    ///
    /// # 参数
    /// - `room_id`：目标房间
    /// - `caller_user_id`：调用者 user_id（需已在房间内，否则返回 403）
    /// - `page`：页码（≥1，否则返回 ValidationError 40003）
    /// - `limit`：每页数量（1–100，超界自动 clamp）
    ///
    /// # 排序规则
    /// 1. 麦上用户（`mic_slot != None`）置顶，按 `slot ASC`
    /// 2. 观众按 `joined_at DESC`（最新进房者在前）
    pub async fn list_members(
        &self,
        room_id: Uuid,
        caller_user_id: Uuid,
        page: u32,
        limit: u32,
    ) -> Result<MembersListResponse, AppError> {
        // ── 1. 参数校验 ──────────────────────────────────────────────────────
        if page == 0 {
            return Err(AppError::ValidationError("page must be >= 1".to_string()));
        }
        let limit = limit.clamp(1, 100);

        // ── 2. 查询房间归属信息（owner_id / admin_user_id）───────────────────
        let owner_info = self
            .room_repo
            .find_room_owner(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("room {room_id}")))?;

        // ── 3. 权限校验：调用者必须是房间成员 ───────────────────────────────
        let state = self
            .room_manager
            .get_room(room_id)
            .ok_or_else(|| AppError::Forbidden("not a room member".to_string()))?;

        if !state.members.contains_key(&caller_user_id) {
            return Err(AppError::Forbidden("not a room member".to_string()));
        }

        // ── 4. 获取全量成员快照（内存 O(n)）────────────────────────────────
        let mut snapshots = self.room_manager.list_members(room_id).unwrap_or_default();

        let total = snapshots.len();

        // ── 5. 排序：麦上用户 slot ASC → 观众 joined_at DESC ────────────────
        snapshots.sort_by(|a, b| match (a.mic_slot, b.mic_slot) {
            (Some(sa), Some(sb)) => sa.cmp(&sb),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.joined_at.cmp(&a.joined_at),
        });

        // ── 6. 分页切片 ──────────────────────────────────────────────────────
        let offset = ((page - 1) as usize) * (limit as usize);
        let page_snapshots: Vec<_> = snapshots.iter().skip(offset).take(limit as usize).collect();

        // ── 7. 批量查询用户信息 ──────────────────────────────────────────────
        let user_ids: Vec<Uuid> = page_snapshots.iter().map(|s| s.user_id).collect();
        let users = self.user_repo.find_users_by_ids(&user_ids).await?;
        let user_map: std::collections::HashMap<Uuid, UserInfo> =
            users.into_iter().map(|u| (u.id, u)).collect();

        // ── 8. 构建响应条目 ──────────────────────────────────────────────────
        let items: Vec<MemberItem> = page_snapshots
            .iter()
            .map(|s| {
                let user = user_map.get(&s.user_id);
                let role = compute_role(s.user_id, &owner_info);
                MemberItem {
                    user_id: s.user_id,
                    nickname: user.map(|u| u.nickname.clone()).unwrap_or_default(),
                    avatar: user.and_then(|u| u.avatar.clone()),
                    role,
                    mic_slot: s.mic_slot,
                    joined_at: s.joined_at.to_rfc3339(),
                    muted_mic: s.muted_mic,
                    muted_chat: s.muted_chat,
                }
            })
            .collect();

        Ok(MembersListResponse {
            total,
            page,
            limit,
            items,
        })
    }
}

// ─── 内部辅助 ─────────────────────────────────────────────────────────────────

/// 根据 user_id 与 owner_info 计算角色字符串。
fn compute_role(user_id: Uuid, owner_info: &RoomOwnerInfo) -> String {
    if user_id == owner_info.owner_id {
        "owner".to_string()
    } else if owner_info.admin_user_id == Some(user_id) {
        "admin".to_string()
    } else {
        "member".to_string()
    }
}
