//! MembersService — 观众席列表 API 业务逻辑（T-00027）
//!
//! 从 `RoomManager` 读取内存成员快照，批量查询用户信息，
//! 计算角色、排序、分页后返回响应。
//!
//! T-00070 §1: 集成贵族隐身过滤——若提供 `MembersNobilityPort`，
//! 则在分页前对每个成员的可见性进行过滤。

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

/// 贵族隐身过滤端口（T-00070 §1）
///
/// 用于在 list_members 中对每个成员应用隐身过滤。
/// 生产实现：查询 user_nobles 表获取特权 JSONB；
/// 测试实现：内存 HashMap。
#[async_trait]
pub trait MembersNobilityPort: Send + Sync {
    /// 判断指定成员对指定观看者是否可见（已封装隐身逻辑）
    ///
    /// - `member_id`: 要检查的成员
    /// - `viewer_is_admin_or_owner`: 观看者是否为房主/管理员
    /// - `viewer_is_on_mic`: 观看者是否在麦上
    async fn is_member_visible(
        &self,
        member_id: Uuid,
        viewer_is_admin_or_owner: bool,
        viewer_is_on_mic: bool,
    ) -> bool;
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
    /// 贵族隐身过滤端口（T-00070 §1）；None = 不过滤
    nobility_port: Option<Arc<dyn MembersNobilityPort>>,
}

impl MembersService {
    /// 构造服务实例（无隐身过滤，保持向后兼容）。
    pub fn new(
        room_manager: Arc<RoomManager>,
        user_repo: Arc<dyn MembersUserRepo>,
        room_repo: Arc<dyn MembersRoomRepo>,
    ) -> Self {
        Self {
            room_manager,
            user_repo,
            room_repo,
            nobility_port: None,
        }
    }

    /// 构造服务实例（含贵族隐身过滤，T-00070 §1）。
    pub fn new_with_nobility(
        room_manager: Arc<RoomManager>,
        user_repo: Arc<dyn MembersUserRepo>,
        room_repo: Arc<dyn MembersRoomRepo>,
        nobility_port: Arc<dyn MembersNobilityPort>,
    ) -> Self {
        Self {
            room_manager,
            user_repo,
            room_repo,
            nobility_port: Some(nobility_port),
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

        // ── 4.5 T-00070 §1: 贵族隐身过滤 ────────────────────────────────────
        // 仅在提供 nobility_port 时生效
        if let Some(ref port) = self.nobility_port {
            let viewer_role = compute_role(caller_user_id, &owner_info);
            let viewer_is_admin_or_owner =
                viewer_role == "admin" || viewer_role == "owner";
            let viewer_is_on_mic = {
                let mic_snap = state.mic_slots_snapshot();
                mic_snap.contains(&Some(caller_user_id))
            };

            let mut visible_snapshots = Vec::with_capacity(snapshots.len());
            for snap in snapshots {
                // 调用者自己始终可见
                if snap.user_id == caller_user_id {
                    visible_snapshots.push(snap);
                    continue;
                }
                if port
                    .is_member_visible(snap.user_id, viewer_is_admin_or_owner, viewer_is_on_mic)
                    .await
                {
                    visible_snapshots.push(snap);
                }
            }
            snapshots = visible_snapshots;
        }

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

// ─── 单元测试（T-00070 §1: 隐身过滤）─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;

    use crate::room::manager::RoomManager;
    use crate::room::state::MemberInfo;

    // ── Fake 实现 ──────────────────────────────────────────────────────────────

    struct FakeUserRepo {
        users: HashMap<Uuid, UserInfo>,
    }

    impl FakeUserRepo {
        fn new(users: Vec<UserInfo>) -> Self {
            Self {
                users: users.into_iter().map(|u| (u.id, u)).collect(),
            }
        }
    }

    #[async_trait]
    impl MembersUserRepo for FakeUserRepo {
        async fn find_users_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserInfo>, AppError> {
            Ok(ids
                .iter()
                .filter_map(|id| self.users.get(id).cloned())
                .collect())
        }
    }

    struct FakeRoomRepo {
        owners: HashMap<Uuid, RoomOwnerInfo>,
    }

    impl FakeRoomRepo {
        fn new_with_owner(room_id: Uuid, owner_id: Uuid) -> Self {
            let mut owners = HashMap::new();
            owners.insert(room_id, RoomOwnerInfo { owner_id, admin_user_id: None });
            Self { owners }
        }
    }

    #[async_trait]
    impl MembersRoomRepo for FakeRoomRepo {
        async fn find_room_owner(&self, room_id: Uuid) -> Result<Option<RoomOwnerInfo>, AppError> {
            Ok(self.owners.get(&room_id).cloned())
        }
    }

    /// 隐身过滤：指定隐身用户集合（invisible_ids 中的成员对非管理员不可见）
    struct FakeNobilityPort {
        invisible_ids: Mutex<HashSet<Uuid>>,
    }

    impl FakeNobilityPort {
        fn new(invisible_ids: Vec<Uuid>) -> Self {
            Self {
                invisible_ids: Mutex::new(invisible_ids.into_iter().collect()),
            }
        }
    }

    #[async_trait]
    impl MembersNobilityPort for FakeNobilityPort {
        async fn is_member_visible(
            &self,
            member_id: Uuid,
            viewer_is_admin_or_owner: bool,
            _viewer_is_on_mic: bool,
        ) -> bool {
            let invisible = self.invisible_ids.lock().unwrap();
            if invisible.contains(&member_id) {
                // 隐身用户只对管理员可见
                viewer_is_admin_or_owner
            } else {
                true
            }
        }
    }

    // ── 辅助：创建服务 ──────────────────────────────────────────────────────────

    fn make_service(
        room_manager: Arc<RoomManager>,
        room_id: Uuid,
        owner_id: Uuid,
        members: Vec<(Uuid, &str)>,
        nobility_port: Option<Arc<dyn MembersNobilityPort>>,
    ) -> MembersService {
        let user_repo = Arc::new(FakeUserRepo::new(
            members
                .iter()
                .map(|(id, nick)| UserInfo {
                    id: *id,
                    nickname: nick.to_string(),
                    avatar: None,
                })
                .collect(),
        ));
        let room_repo = Arc::new(FakeRoomRepo::new_with_owner(room_id, owner_id));

        // 将成员添加到房间状态
        let room_state = room_manager.get_or_create_room(room_id);
        for (id, nick) in &members {
            room_state.members.insert(*id, MemberInfo::new(*id, nick.to_string(), None));
        }

        match nobility_port {
            Some(port) => MembersService::new_with_nobility(room_manager, user_repo, room_repo, port),
            None => MembersService::new(room_manager, user_repo, room_repo),
        }
    }

    // MS70-01: 无 nobility_port 时，所有成员可见（默认行为不变）
    #[tokio::test]
    async fn ms70_01_without_nobility_port_all_members_visible() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let manager = Arc::new(RoomManager::new());

        let svc = make_service(
            manager,
            room_id,
            owner_id,
            vec![(owner_id, "Owner"), (member_a, "A"), (member_b, "B")],
            None,
        );

        let result = svc.list_members(room_id, owner_id, 1, 20).await.unwrap();
        assert_eq!(result.total, 3, "Without noble port: all 3 members visible");
    }

    // MS70-02: 隐身成员对普通观众不可见
    #[tokio::test]
    async fn ms70_02_invisible_member_hidden_from_regular_viewer() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let invisible_member = Uuid::new_v4();
        let regular_member = Uuid::new_v4();
        let manager = Arc::new(RoomManager::new());

        let port = Arc::new(FakeNobilityPort::new(vec![invisible_member]));
        let svc = make_service(
            manager,
            room_id,
            owner_id,
            vec![
                (owner_id, "Owner"),
                (invisible_member, "Invisible"),
                (regular_member, "Regular"),
            ],
            Some(port),
        );

        let result = svc.list_members(room_id, regular_member, 1, 20).await.unwrap();
        let ids: Vec<Uuid> = result.items.iter().map(|i| i.user_id).collect();
        assert!(
            !ids.contains(&invisible_member),
            "MS70-02: invisible member should not be visible to regular viewer"
        );
        assert!(
            ids.contains(&regular_member),
            "MS70-02: caller (regular) should be visible to themselves"
        );
    }

    // MS70-03: 隐身成员对管理员可见
    #[tokio::test]
    async fn ms70_03_invisible_member_visible_to_admin() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let invisible_member = Uuid::new_v4();
        let manager = Arc::new(RoomManager::new());

        let port = Arc::new(FakeNobilityPort::new(vec![invisible_member]));
        let svc = make_service(
            manager,
            room_id,
            owner_id,
            vec![(owner_id, "Owner"), (invisible_member, "Invisible")],
            Some(port),
        );

        // owner 调用（admin/owner 角色）
        let result = svc.list_members(room_id, owner_id, 1, 20).await.unwrap();
        let ids: Vec<Uuid> = result.items.iter().map(|i| i.user_id).collect();
        assert!(
            ids.contains(&invisible_member),
            "MS70-03: invisible member should be visible to owner"
        );
    }

    // MS70-04: 隐身用户自己始终在自己的成员列表中可见
    #[tokio::test]
    async fn ms70_04_invisible_member_visible_to_themselves() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let invisible_id = Uuid::new_v4();
        let manager = Arc::new(RoomManager::new());

        let port = Arc::new(FakeNobilityPort::new(vec![invisible_id]));
        let svc = make_service(
            manager,
            room_id,
            owner_id,
            vec![(owner_id, "Owner"), (invisible_id, "Invisible")],
            Some(port),
        );

        // 调用者就是隐身用户自己
        let result = svc.list_members(room_id, invisible_id, 1, 20).await.unwrap();
        let ids: Vec<Uuid> = result.items.iter().map(|i| i.user_id).collect();
        assert!(
            ids.contains(&invisible_id),
            "MS70-04: invisible member should see themselves"
        );
    }

    // MS70-05: total 反映过滤后的数量
    #[tokio::test]
    async fn ms70_05_total_reflects_filtered_count() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let invisible1 = Uuid::new_v4();
        let invisible2 = Uuid::new_v4();
        let regular = Uuid::new_v4();
        let manager = Arc::new(RoomManager::new());

        let port = Arc::new(FakeNobilityPort::new(vec![invisible1, invisible2]));
        let svc = make_service(
            manager,
            room_id,
            owner_id,
            vec![
                (owner_id, "Owner"),
                (invisible1, "Invis1"),
                (invisible2, "Invis2"),
                (regular, "Regular"),
            ],
            Some(port),
        );

        // regular 调用 → 2 visible (owner + regular)
        let result = svc.list_members(room_id, regular, 1, 20).await.unwrap();
        assert_eq!(result.total, 2, "MS70-05: total should be 2 (owner + regular) after filtering invisible members");
    }
}
