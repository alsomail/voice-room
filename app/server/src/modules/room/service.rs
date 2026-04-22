use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;

use super::{
    dto::{
        CreateRoomRequest, CreateRoomResponse, NewRoom, OwnerInfo, PatchRoomRequest,
        PatchRoomResponse, RoomDetailResponse, RoomFieldsUpdate, RoomListItem, RoomListQuery,
        RoomListResponse,
    },
    repository::RoomRepository,
    validator,
};

/// 测试环境使用低成本 bcrypt（避免每个测试等待 300ms）
#[cfg(not(test))]
const BCRYPT_COST: u32 = bcrypt::DEFAULT_COST;
#[cfg(test)]
const BCRYPT_COST: u32 = 4;

/// 合法 room_type 值（与 DB CHECK 约束保持一致）
const VALID_ROOM_TYPES: &[&str] = &["normal", "password", "paid"];

pub struct RoomService {
    room_repo: Arc<dyn RoomRepository>,
}

impl RoomService {
    pub fn new(room_repo: Arc<dyn RoomRepository>) -> Self {
        Self { room_repo }
    }

    /// T-00007 / T-00025: 创建房间
    ///
    /// 验证规则：
    /// - title：1–30 Unicode 字符（chars().count()）
    /// - room_type：必须是 normal / password / paid
    /// - password：room_type=password 时必须提供且为 6 位数字
    /// - cover_url：若提供，必须匹配白名单前缀（T-00025）
    /// - category：若提供，必须是 6 类枚举之一（T-00025）
    /// - announcement：若提供，≤200 Unicode 字符（T-00025）
    /// - 同一 owner 同时只能有 1 个 active 房间
    pub async fn create_room(
        &self,
        owner_id: Uuid,
        req: CreateRoomRequest,
    ) -> Result<CreateRoomResponse, AppError> {
        // ── 1. 验证 title ────────────────────────────────────────────────────
        let title_len = req.title.chars().count();
        if title_len == 0 {
            return Err(AppError::ValidationError(
                "title must not be empty".to_string(),
            ));
        }
        if title_len > 30 {
            return Err(AppError::ValidationError(format!(
                "title must be at most 30 characters, got {title_len}"
            )));
        }

        // ── 2. 验证 room_type ────────────────────────────────────────────────
        if !VALID_ROOM_TYPES.contains(&req.room_type.as_str()) {
            return Err(AppError::ValidationError(format!(
                "room_type must be one of {:?}, got {:?}",
                VALID_ROOM_TYPES, req.room_type
            )));
        }

        // ── 3. 验证 cover_url（T-00025）──────────────────────────────────────
        if let Some(ref url) = req.cover_url {
            validator::validate_cover_url(url)?;
        }

        // ── 4. 验证 category（T-00025）──────────────────────────────────────
        if let Some(ref cat) = req.category {
            validator::validate_category(cat)?;
        }

        // ── 5. 验证 announcement（T-00025）──────────────────────────────────
        if let Some(ref ann) = req.announcement {
            validator::validate_announcement(ann)?;
        }

        // ── 6. 验证 password（password 类型时必须提供且为 6 位数字）──────────
        // M-01: 非密码房间（normal/paid）即使请求携带 password 字段，也必须存 NULL，
        // 防止数据污染和后续进入逻辑误判。
        if req.room_type == "password" {
            let pwd = req.password.as_deref().ok_or_else(|| {
                AppError::ValidationError(
                    "password is required for password rooms".to_string(),
                )
            })?;
            validator::validate_password(pwd)?;
        }

        // ── 7. 检查同 owner 是否已有 active 房间 ───────────────────────────
        if self
            .room_repo
            .find_active_by_owner(owner_id)
            .await?
            .is_some()
        {
            return Err(AppError::ActiveRoomExists);
        }

        // ── 8. bcrypt 密码（仅 password 类型，spawn_blocking 避免阻塞运行时）────
        let password_hash = if req.room_type == "password" {
            let pwd = req.password.clone().unwrap(); // 步骤6已确保不为 None
            let hash = tokio::task::spawn_blocking(move || bcrypt::hash(&pwd, BCRYPT_COST))
                .await
                .map_err(|e| AppError::Internal(format!("spawn_blocking error: {e}")))?
                .map_err(|e| AppError::Internal(format!("bcrypt error: {e}")))?;
            Some(hash)
        } else {
            None // normal / paid 房间，忽略 password 字段
        };

        // ── 9. 创建房间（含 T-00025 新字段）──────────────────────────────────
        let new_room = NewRoom {
            owner_id,
            title: req.title,
            room_type: req.room_type,
            password_hash,
            cover_url: req.cover_url.unwrap_or_default(),
            category: req.category.unwrap_or_else(|| "chat".to_string()),
            announcement: req.announcement,
        };

        let room = self.room_repo.create(new_room).await?;

        Ok(CreateRoomResponse {
            room_id: room.id.to_string(),
            title: room.title,
            room_type: room.room_type,
            created_at: room.created_at.to_rfc3339(),
        })
    }

    /// T-00008: 房间列表
    ///
    /// 验证规则：
    /// - page 默认 1，最小 1
    /// - size 默认 20，范围 1–100
    ///
    /// 返回：total / page / size / items（按 member_count DESC, created_at DESC）
    pub async fn list_rooms(&self, query: RoomListQuery) -> Result<RoomListResponse, AppError> {        // ── 1. 默认值 ────────────────────────────────────────────────────────
        let page = query.page.unwrap_or(1);
        let size = query.size.unwrap_or(20);

        // ── 2. 参数校验 ──────────────────────────────────────────────────────
        if page < 1 {
            return Err(AppError::ValidationError(
                "page must be >= 1".to_string(),
            ));
        }
        if size < 1 {
            return Err(AppError::ValidationError(
                "size must be >= 1".to_string(),
            ));
        }
        if size > 100 {
            return Err(AppError::ValidationError(format!(
                "size must be <= 100, got {size}"
            )));
        }

        // ── 3. 查询 ──────────────────────────────────────────────────────────
        let total = self.room_repo.count_active_rooms().await?;
        let rows = self.room_repo.find_active_rooms(page, size).await?;

        let items = rows
            .into_iter()
            .map(|r| RoomListItem {
                room_id: r.room_id.to_string(),
                title: r.title,
                room_type: r.room_type,
                member_count: r.member_count,
                max_members: r.max_members,
                owner_id: r.owner_id.to_string(),
                owner_nickname: r.owner_nickname,
                owner_avatar: r.owner_avatar,
                created_at: r.created_at.to_rfc3339(),
            })
            .collect();

        Ok(RoomListResponse {
            total,
            page,
            size,
            items,
        })
    }

    /// T-00009: 房间详情
    ///
    /// - room_id 在 active 房间中存在：返回完整 RoomDetailResponse（含 owner、mic_slots）
    /// - 不存在 / closed / soft-deleted：返回 NotFound
    pub async fn get_room_detail(&self, room_id: uuid::Uuid) -> Result<RoomDetailResponse, AppError> {
        let row = self
            .room_repo
            .find_room_by_id(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("room {room_id}")))?;

        Ok(RoomDetailResponse {
            room_id: row.room_id.to_string(),
            title: row.title,
            room_type: row.room_type,
            member_count: row.member_count,
            max_members: row.max_members,
            owner: OwnerInfo {
                user_id: row.owner_user_id.to_string(),
                nickname: row.owner_nickname,
                avatar: row.owner_avatar,
            },
            mic_slots: vec![], // MVP 固定空数组
            created_at: row.created_at.to_rfc3339(),
        })
    }

    /// T-00012: 获取活跃房间详情（用于 JoinRoom 信令校验）
    ///
    /// 返回 `Option<RoomDetailRow>`：
    /// - `Some`：房间存在且 status = active
    /// - `None`：房间不存在、已关闭或已软删除
    pub async fn get_active_room_detail(
        &self,
        room_id: uuid::Uuid,
    ) -> Result<Option<crate::modules::room::repository::RoomDetailRow>, AppError> {
        self.room_repo.find_room_by_id(room_id).await
    }

    /// T-00026: 获取活跃房间的完整 Model（含 password_hash）
    ///
    /// 用于密码房校验场景，返回 None 表示房间不存在或已关闭/软删除。
    pub async fn get_active_room_model(
        &self,
        room_id: Uuid,
    ) -> Result<Option<voice_room_shared::models::room::RoomModel>, AppError> {
        match self.room_repo.find_room_any_status(room_id).await? {
            Some(room) if room.status == "active" && room.deleted_at.is_none() => Ok(Some(room)),
            _ => Ok(None),
        }
    }

    /// T-00010: 关闭房间
    ///
    /// 验证规则：
    /// 1. find_room_any_status → None 则 NotFound（软删除或根本不存在）
    /// 2. owner_id != current_user_id → Forbidden
    /// 3. status == "closed" → RoomAlreadyClosed
    /// 4. 执行 set_room_closed
    pub async fn close_room(
        &self,
        room_id: Uuid,
        current_user_id: Uuid,
    ) -> Result<(), AppError> {
        // ── 1. 查询房间（不过滤 status，只排除软删除）──────────────────────────
        let room = self
            .room_repo
            .find_room_any_status(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("room {room_id}")))?;

        // ── 2. 校验 owner ──────────────────────────────────────────────────────
        if room.owner_id != current_user_id {
            return Err(AppError::Forbidden(
                "only the owner can close this room".to_string(),
            ));
        }

        // ── 3. 校验状态 ────────────────────────────────────────────────────────
        if room.status == "closed" {
            return Err(AppError::RoomAlreadyClosed);
        }

        // ── 4. 执行关闭 ────────────────────────────────────────────────────────
        self.room_repo.set_room_closed(room_id).await?;

        Ok(())
    }

    /// T-00025: PATCH 房间信息（仅房主，房间 active 时可用）
    ///
    /// 验证规则：
    /// 1. 至少提供一个字段（title / announcement / category），否则 40003
    /// 2. title 若提供：1–30 Unicode 字符
    /// 3. category 若提供：6 类枚举之一
    /// 4. announcement 若提供：≤200 字符（空串 = 清空）
    /// 5. 房间不存在（含软删除）→ 404
    /// 6. current_user_id != owner_id → 403
    /// 7. status == "closed" → 409
    /// 8. 更新字段，返回 PatchRoomResponse（广播由 controller 发起）
    pub async fn patch_room(
        &self,
        room_id: Uuid,
        current_user_id: Uuid,
        req: PatchRoomRequest,
    ) -> Result<PatchRoomResponse, AppError> {
        // ── 1. 至少一个字段 ──────────────────────────────────────────────────
        if req.title.is_none() && req.announcement.is_none() && req.category.is_none() {
            return Err(AppError::ValidationError(
                "at least one field (title, announcement, category) must be provided".to_string(),
            ));
        }

        // ── 2. 校验 title ────────────────────────────────────────────────────
        if let Some(ref title) = req.title {
            let len = title.chars().count();
            if len == 0 {
                return Err(AppError::ValidationError(
                    "title must not be empty".to_string(),
                ));
            }
            if len > 30 {
                return Err(AppError::ValidationError(format!(
                    "title must be at most 30 characters, got {len}"
                )));
            }
        }

        // ── 3. 校验 category ─────────────────────────────────────────────────
        if let Some(ref cat) = req.category {
            validator::validate_category(cat)?;
        }

        // ── 4. 校验 announcement ─────────────────────────────────────────────
        if let Some(ref ann) = req.announcement {
            validator::validate_announcement(ann)?;
        }

        // ── 5. 查询房间（不过滤 status，只排除软删除）──────────────────────
        let room = self
            .room_repo
            .find_room_any_status(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("room {room_id}")))?;

        // ── 6. 校验 owner ────────────────────────────────────────────────────
        if room.owner_id != current_user_id {
            return Err(AppError::Forbidden(
                "only the owner can modify this room".to_string(),
            ));
        }

        // ── 7. 校验状态 ──────────────────────────────────────────────────────
        if room.status == "closed" {
            return Err(AppError::RoomAlreadyClosed);
        }

        // ── 8. 执行 partial update ───────────────────────────────────────────
        let updated = self
            .room_repo
            .update_room_fields(room_id, RoomFieldsUpdate {
                title: req.title,
                announcement: req.announcement,
                category: req.category,
            })
            .await?;

        Ok(PatchRoomResponse {
            room_id: updated.id.to_string(),
            title: updated.title,
            announcement: updated.announcement,
            category: updated.category,
            cover_url: updated.cover_url,
            has_password: updated.password_hash.is_some(),
        })
    }
}

// ─── 单元测试（T-00007 验收用例）─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::room::repository::FakeRoomRepository;
    use chrono::{Duration, Utc};
    use std::sync::Arc;
    use uuid::Uuid;
    use voice_room_shared::models::room::RoomModel;

    // ── 测试辅助 ─────────────────────────────────────────────────────────────

    fn make_service() -> (RoomService, Arc<FakeRoomRepository>) {
        let repo = Arc::new(FakeRoomRepository::default());
        let svc = RoomService::new(repo.clone());
        (svc, repo)
    }

    fn normal_req(title: &str) -> CreateRoomRequest {
        CreateRoomRequest {
            title: title.to_string(),
            room_type: "normal".to_string(),
            password: None,
            cover_url: None,
            category: None,
            announcement: None,
        }
    }

    fn seed_active_room(repo: &FakeRoomRepository, owner_id: Uuid) {
        let now = Utc::now();
        repo.seed(RoomModel {
            id: Uuid::new_v4(),
            owner_id,
            title: "Existing Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
    }

    // ── C-01: 正向创建成功 ────────────────────────────────────────────────────

    /// C-01: 正常创建 normal 类型房间，返回 room_id / title / room_type / created_at
    #[tokio::test]
    async fn c01_create_normal_room_succeeds() {
        let (svc, _) = make_service();
        let owner_id = Uuid::new_v4();
        let resp = svc
            .create_room(owner_id, normal_req("My Room"))
            .await
            .unwrap();

        assert!(!resp.room_id.is_empty(), "room_id should not be empty");
        assert_eq!(resp.title, "My Room");
        assert_eq!(resp.room_type, "normal");
        assert!(!resp.created_at.is_empty(), "created_at should not be empty");
        // created_at 应是合法的 RFC3339 时间戳
        assert!(
            resp.created_at.contains('T'),
            "created_at should be RFC3339 format"
        );
    }

    // ── C-04: 空 title ────────────────────────────────────────────────────────

    /// C-04: title 为空字符串 → ValidationError
    #[tokio::test]
    async fn c04_empty_title_returns_validation_error() {
        let (svc, _) = make_service();
        let err = svc
            .create_room(Uuid::new_v4(), normal_req(""))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "expected ValidationError, got: {err:?}"
        );
    }

    // ── C-05: 超过 30 字符 title（Unicode）────────────────────────────────────

    /// C-05: title 超过 30 个 Unicode 字符 → ValidationError（chars().count() 测试）
    #[tokio::test]
    async fn c05_title_over_30_unicode_chars_returns_validation_error() {
        let (svc, _) = make_service();
        // 31 个中文字符（每个占 3 字节，但 chars().count() == 31）
        let long_title: String = "房".repeat(31);
        assert_eq!(long_title.chars().count(), 31, "should be 31 chars");

        let err = svc
            .create_room(Uuid::new_v4(), normal_req(&long_title))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "expected ValidationError for 31-char title, got: {err:?}"
        );
    }

    // ── C-06: 恰好 30 字符通过 ───────────────────────────────────────────────

    /// C-06: title 恰好 30 个 Unicode 字符 → 创建成功
    #[tokio::test]
    async fn c06_title_exactly_30_unicode_chars_passes() {
        let (svc, _) = make_service();
        let title_30: String = "音".repeat(30);
        assert_eq!(title_30.chars().count(), 30, "should be exactly 30 chars");

        let resp = svc
            .create_room(Uuid::new_v4(), normal_req(&title_30))
            .await
            .unwrap();
        assert_eq!(resp.title, title_30);
    }

    // ── C-07: 已有 active 房间 → ActiveRoomExists ─────────────────────────────

    /// C-07: owner 已有 active 房间时返回 ActiveRoomExists (HTTP 409)
    #[tokio::test]
    async fn c07_active_room_exists_returns_conflict() {
        let (svc, repo) = make_service();
        let owner_id = Uuid::new_v4();
        seed_active_room(&repo, owner_id);

        let err = svc
            .create_room(owner_id, normal_req("Second Room"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ActiveRoomExists),
            "expected ActiveRoomExists, got: {err:?}"
        );
    }

    // ── C-08: room_type=password 未提供 password ──────────────────────────────

    /// C-08: room_type=password 但没有 password 字段 → ValidationError
    #[tokio::test]
    async fn c08_password_room_without_password_returns_validation_error() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "Secret Room".to_string(),
            room_type: "password".to_string(),
            password: None,
            cover_url: None,
            category: None,
            announcement: None,
        };

        let err = svc
            .create_room(Uuid::new_v4(), req)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "expected ValidationError, got: {err:?}"
        );
    }

    // ── C-09: 密码房 bcrypt hash 正确 ────────────────────────────────────────

    /// C-09: room_type=password，验证 bcrypt hash 可被 verify（防明文存储）
    #[tokio::test]
    async fn c09_password_room_stores_bcrypt_hash() {
        let (svc, repo) = make_service();
        let owner_id = Uuid::new_v4();
        let req = CreateRoomRequest {
            title: "Locked Room".to_string(),
            room_type: "password".to_string(),
            password: Some("123456".to_string()),
            cover_url: None,
            category: None,
            announcement: None,
        };

        svc.create_room(owner_id, req).await.unwrap();

        // 从 repo 取出房间，验证 password_hash
        let room = repo
            .find_active_by_owner(owner_id)
            .await
            .unwrap()
            .expect("room should exist");
        let hash = room.password_hash.expect("password_hash should be Some");

        // bcrypt::verify 应该返回 true
        let valid =
            bcrypt::verify("123456", &hash).expect("bcrypt verify should not fail");
        assert!(valid, "bcrypt hash should verify against original password");

        // 确保明文没有被存储
        assert_ne!(hash, "123456", "must not store plain text");
    }

    // ── C-10: 非法 room_type ─────────────────────────────────────────────────

    /// C-10: room_type 不在枚举中 → ValidationError
    #[tokio::test]
    async fn c10_invalid_room_type_returns_validation_error() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "Test Room".to_string(),
            room_type: "vip_only".to_string(),
            password: None,
            cover_url: None,
            category: None,
            announcement: None,
        };

        let err = svc
            .create_room(Uuid::new_v4(), req)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "expected ValidationError for invalid room_type, got: {err:?}"
        );
    }

    // ── 边界：title 只有 1 个字符 ─────────────────────────────────────────────

    /// 边界: title 只有 1 个字符应该通过
    #[tokio::test]
    async fn title_single_char_passes() {
        let (svc, _) = make_service();
        let resp = svc
            .create_room(Uuid::new_v4(), normal_req("A"))
            .await
            .unwrap();
        assert_eq!(resp.title, "A");
    }

    // ── 边界：paid 类型不需要 password ───────────────────────────────────────

    /// paid 类型无 password 字段应该成功
    #[tokio::test]
    async fn paid_room_without_password_succeeds() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "Paid Room".to_string(),
            room_type: "paid".to_string(),
            password: None,
            cover_url: None,
            category: None,
            announcement: None,
        };
        let resp = svc.create_room(Uuid::new_v4(), req).await.unwrap();
        assert_eq!(resp.room_type, "paid");
    }

    // ── 边界：不同 owner 可以各自创建 active 房间 ─────────────────────────────

    /// 两个不同 owner 可以各自持有 active 房间（不互相阻塞）
    #[tokio::test]
    async fn different_owners_can_each_have_active_room() {
        let (svc, _) = make_service();
        let owner_a = Uuid::new_v4();
        let owner_b = Uuid::new_v4();

        svc.create_room(owner_a, normal_req("Room A")).await.unwrap();
        svc.create_room(owner_b, normal_req("Room B")).await.unwrap();
        // 两个都应成功
    }

    // ── C-04 变体：仅空白字符的 title 长度不为零，应通过（DB 会接受空格）────────
    // 注意：service 只检查 chars().count()；1 个空格算 1 个字符。
    #[tokio::test]
    async fn title_single_space_is_one_char_passes() {
        let (svc, _) = make_service();
        // " " 是 1 个字符，应通过 service 验证
        let resp = svc
            .create_room(Uuid::new_v4(), normal_req(" "))
            .await
            .unwrap();
        assert_eq!(resp.title, " ");
    }

    // ── M-01: 非密码房间携带 password 字段时，password_hash 必须为 None ──────

    /// M-01: normal 房间即使请求携带了 password 字段，password_hash 也必须存 None
    #[tokio::test]
    async fn create_normal_room_with_password_field_ignored() {
        let (svc, repo) = make_service();
        let owner_id = Uuid::new_v4();
        let req = CreateRoomRequest {
            title: "Normal Room".to_string(),
            room_type: "normal".to_string(),
            password: Some("should_be_ignored".to_string()), // 客户端误传
            cover_url: None,
            category: None,
            announcement: None,
        };

        svc.create_room(owner_id, req).await.unwrap();

        let room = repo
            .find_active_by_owner(owner_id)
            .await
            .unwrap()
            .expect("room should exist after creation");

        assert!(
            room.password_hash.is_none(),
            "normal room must not store password_hash, got: {:?}",
            room.password_hash
        );
    }

    /// M-01 补充: paid 房间即使请求携带了 password 字段，password_hash 也必须为 None
    #[tokio::test]
    async fn create_paid_room_with_password_field_ignored() {
        let (svc, repo) = make_service();
        let owner_id = Uuid::new_v4();
        let req = CreateRoomRequest {
            title: "Paid Room".to_string(),
            room_type: "paid".to_string(),
            password: Some("should_be_ignored".to_string()), // 客户端误传
            cover_url: None,
            category: None,
            announcement: None,
        };

        svc.create_room(owner_id, req).await.unwrap();

        let room = repo
            .find_active_by_owner(owner_id)
            .await
            .unwrap()
            .expect("room should exist after creation");

        assert!(
            room.password_hash.is_none(),
            "paid room must not store password_hash, got: {:?}",
            room.password_hash
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00008: list_rooms 单元测试（L-01 ~ L-13）
    // ═══════════════════════════════════════════════════════════════════════

    /// 构造 (service, fake_repo) 对
    fn make_list_service() -> (RoomService, Arc<FakeRoomRepository>) {
        let repo = Arc::new(FakeRoomRepository::default());
        let svc = RoomService::new(repo.clone());
        (svc, repo)
    }

    /// 快速构造一个活跃房间并植入 repo
    fn seed_room_with_owner(
        repo: &FakeRoomRepository,
        owner_id: Uuid,
        title: &str,
        member_count: i32,
        created_at_offset_secs: i64,
    ) -> Uuid {
        let now = Utc::now() + Duration::seconds(created_at_offset_secs);
        let room_id = Uuid::new_v4();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: title.to_string(),
            room_type: "normal".to_string(),
            member_count,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        room_id
    }

    // ── L-01: 3 个活跃房间按 member_count DESC 返回 ───────────────────────

    /// L-01: 3 个活跃房间 → 按 member_count DESC 返回
    #[tokio::test]
    async fn l01_three_active_rooms_sorted_by_member_count_desc() {
        let (svc, repo) = make_list_service();
        let o1 = Uuid::new_v4();
        let o2 = Uuid::new_v4();
        let o3 = Uuid::new_v4();

        repo.seed_user(o1, "Alice".into(), None);
        repo.seed_user(o2, "Bob".into(), None);
        repo.seed_user(o3, "Carol".into(), None);

        seed_room_with_owner(&repo, o1, "Room A", 5, 0);
        seed_room_with_owner(&repo, o2, "Room B", 20, 1);
        seed_room_with_owner(&repo, o3, "Room C", 10, 2);

        let resp = svc
            .list_rooms(RoomListQuery { page: None, size: None })
            .await
            .unwrap();

        assert_eq!(resp.items.len(), 3);
        assert_eq!(resp.items[0].member_count, 20, "first should be most popular");
        assert_eq!(resp.items[1].member_count, 10);
        assert_eq!(resp.items[2].member_count, 5);
    }

    // ── L-02: 分页（total=25，page=2，size=10）────────────────────────────

    /// L-02: 25 个房间，page=2, size=10 → total=25, 第 2 页 10 条
    #[tokio::test]
    async fn l02_pagination_page2_size10() {
        let (svc, repo) = make_list_service();
        for i in 0..25_i32 {
            let owner = Uuid::new_v4();
            repo.seed_user(owner, format!("User{i}"), None);
            repo.seed(RoomModel {
                id: Uuid::new_v4(),
                owner_id: owner,
                title: format!("Room {i}"),
                room_type: "normal".to_string(),
                member_count: i,
                status: "active".to_string(),
                password_hash: None,
                max_members: 50,
                created_at: Utc::now() + Duration::seconds(i as i64),
                updated_at: Utc::now(),
                deleted_at: None,
                cover_url: String::new(),
                category: "chat".to_string(),
                announcement: None,
                admin_user_id: None,
            });
        }

        let resp = svc
            .list_rooms(RoomListQuery { page: Some(2), size: Some(10) })
            .await
            .unwrap();

        assert_eq!(resp.total, 25, "total should be 25");
        assert_eq!(resp.page, 2);
        assert_eq!(resp.size, 10);
        assert_eq!(resp.items.len(), 10, "page 2 should have 10 items");
    }

    // ── L-03: closed 房间不在列表中 ──────────────────────────────────────

    /// L-03: status='closed' 的房间不出现在列表
    #[tokio::test]
    async fn l03_closed_rooms_excluded() {
        let (svc, repo) = make_list_service();
        let owner = Uuid::new_v4();
        repo.seed_user(owner, "Owner".into(), None);

        // 一个活跃房间
        seed_room_with_owner(&repo, owner, "Active Room", 5, 0);

        // 一个关闭的房间
        let closed_owner = Uuid::new_v4();
        repo.seed_user(closed_owner, "ClosedOwner".into(), None);
        repo.seed(RoomModel {
            id: Uuid::new_v4(),
            owner_id: closed_owner,
            title: "Closed Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 100,
            status: "closed".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let resp = svc
            .list_rooms(RoomListQuery { page: None, size: None })
            .await
            .unwrap();

        assert_eq!(resp.total, 1, "closed rooms must not be counted");
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].title, "Active Room");
    }

    // ── L-04: soft-deleted 房间不在列表中 ────────────────────────────────

    /// L-04: deleted_at IS NOT NULL 的房间不出现在列表
    #[tokio::test]
    async fn l04_soft_deleted_rooms_excluded() {
        let (svc, repo) = make_list_service();
        let owner = Uuid::new_v4();
        repo.seed_user(owner, "Owner".into(), None);
        seed_room_with_owner(&repo, owner, "Active Room", 3, 0);

        let del_owner = Uuid::new_v4();
        repo.seed_user(del_owner, "DeletedOwner".into(), None);
        repo.seed(RoomModel {
            id: Uuid::new_v4(),
            owner_id: del_owner,
            title: "Deleted Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 50,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: Some(Utc::now()),
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let resp = svc
            .list_rooms(RoomListQuery { page: None, size: None })
            .await
            .unwrap();

        assert_eq!(resp.total, 1, "soft-deleted rooms must not be counted");
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].title, "Active Room");
    }

    // ── L-05: 无参数使用默认值（page=1, size=20）─────────────────────────

    /// L-05: None 参数使用默认值 page=1, size=20
    #[tokio::test]
    async fn l05_defaults_page1_size20() {
        let (svc, _) = make_list_service();
        let resp = svc
            .list_rooms(RoomListQuery { page: None, size: None })
            .await
            .unwrap();

        assert_eq!(resp.page, 1);
        assert_eq!(resp.size, 20);
    }

    // ── L-06: 超出页数返回空 items，total 正确 ────────────────────────────

    /// L-06: 仅 2 个房间，请求第 5 页 → items 为空，total=2
    #[tokio::test]
    async fn l06_page_beyond_total_returns_empty_items() {
        let (svc, repo) = make_list_service();
        for i in 0..2 {
            let o = Uuid::new_v4();
            repo.seed_user(o, format!("User{i}"), None);
            seed_room_with_owner(&repo, o, &format!("Room {i}"), i, i as i64);
        }

        let resp = svc
            .list_rooms(RoomListQuery { page: Some(5), size: Some(10) })
            .await
            .unwrap();

        assert_eq!(resp.total, 2, "total must reflect full count");
        assert_eq!(resp.items.len(), 0, "page beyond range must return empty items");
    }

    // ── L-07: size=101 返回 ValidationError ──────────────────────────────

    /// L-07: size=101 → ValidationError（超过上界 100）
    #[tokio::test]
    async fn l07_size_101_returns_validation_error() {
        let (svc, _) = make_list_service();
        let err = svc
            .list_rooms(RoomListQuery { page: None, size: Some(101) })
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "size=101 should be ValidationError, got: {err:?}"
        );
    }

    // ── L-08: size=0 返回 ValidationError ────────────────────────────────

    /// L-08: size=0 → ValidationError（低于下界 1）
    #[tokio::test]
    async fn l08_size_0_returns_validation_error() {
        let (svc, _) = make_list_service();
        let err = svc
            .list_rooms(RoomListQuery { page: None, size: Some(0) })
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "size=0 should be ValidationError, got: {err:?}"
        );
    }

    // ── L-09: page=0 返回 ValidationError ────────────────────────────────

    /// L-09: page=0 → ValidationError
    #[tokio::test]
    async fn l09_page_0_returns_validation_error() {
        let (svc, _) = make_list_service();
        let err = svc
            .list_rooms(RoomListQuery { page: Some(0), size: None })
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "page=0 should be ValidationError, got: {err:?}"
        );
    }

    // ── L-10: owner_nickname 和 owner_avatar 正确 ─────────────────────────

    /// L-10: owner_nickname / owner_avatar 从 seed_user 正确填充
    #[tokio::test]
    async fn l10_owner_info_correctly_populated() {
        let (svc, repo) = make_list_service();
        let owner = Uuid::new_v4();
        repo.seed_user(
            owner,
            "VoiceKing".into(),
            Some("https://cdn.example.com/avatar.jpg".into()),
        );
        seed_room_with_owner(&repo, owner, "King's Room", 99, 0);

        let resp = svc
            .list_rooms(RoomListQuery { page: None, size: None })
            .await
            .unwrap();

        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].owner_nickname, "VoiceKing");
        assert_eq!(
            resp.items[0].owner_avatar.as_deref(),
            Some("https://cdn.example.com/avatar.jpg")
        );
    }

    // ── L-11: size=100（边界最大值）正常 ─────────────────────────────────

    /// L-11: size=100 是合法的最大值，不应报错
    #[tokio::test]
    async fn l11_size_100_boundary_max_ok() {
        let (svc, _) = make_list_service();
        let resp = svc
            .list_rooms(RoomListQuery { page: Some(1), size: Some(100) })
            .await
            .unwrap();
        assert_eq!(resp.size, 100);
    }

    // ── L-12: size=1（边界最小值）正常 ───────────────────────────────────

    /// L-12: size=1 是合法的最小值，不应报错
    #[tokio::test]
    async fn l12_size_1_boundary_min_ok() {
        let (svc, _) = make_list_service();
        let resp = svc
            .list_rooms(RoomListQuery { page: Some(1), size: Some(1) })
            .await
            .unwrap();
        assert_eq!(resp.size, 1);
    }

    // ── L-13: member_count 相同时按 created_at DESC 排序 ─────────────────

    /// L-13: member_count 相同时按 created_at DESC 排序（最新创建的在前）
    #[tokio::test]
    async fn l13_same_member_count_sorted_by_created_at_desc() {
        let (svc, repo) = make_list_service();
        let o1 = Uuid::new_v4();
        let o2 = Uuid::new_v4();
        let o3 = Uuid::new_v4();

        repo.seed_user(o1, "Oldest".into(), None);
        repo.seed_user(o2, "Middle".into(), None);
        repo.seed_user(o3, "Newest".into(), None);

        // 全部 member_count=10，created_at 递增（offset 越大越新）
        seed_room_with_owner(&repo, o1, "Old Room", 10, -200);   // oldest
        seed_room_with_owner(&repo, o2, "Mid Room", 10, -100);   // middle
        seed_room_with_owner(&repo, o3, "New Room", 10, 0);      // newest

        let resp = svc
            .list_rooms(RoomListQuery { page: None, size: None })
            .await
            .unwrap();

        assert_eq!(resp.items.len(), 3);
        // 最新创建的应该排在最前面
        assert_eq!(resp.items[0].title, "New Room", "newest first when member_count tied");
        assert_eq!(resp.items[1].title, "Mid Room");
        assert_eq!(resp.items[2].title, "Old Room");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00009: get_room_detail 单元测试（D-01 ~ D-07）
    // ═══════════════════════════════════════════════════════════════════════

    fn make_detail_service() -> (RoomService, Arc<FakeRoomRepository>) {
        let repo = Arc::new(FakeRoomRepository::default());
        let svc = RoomService::new(repo.clone());
        (svc, repo)
    }

    /// 测试辅助：向 Fake 仓库注入一个可自定义 status / deleted_at 的房间
    fn seed_room_for_detail(
        repo: &FakeRoomRepository,
        room_id: Uuid,
        owner_id: Uuid,
        title: &str,
        status: &str,
        deleted_at: Option<chrono::DateTime<Utc>>,
    ) {
        let now = Utc::now();
        repo.seed(voice_room_shared::models::room::RoomModel {
            id: room_id,
            owner_id,
            title: title.to_string(),
            room_type: "normal".to_string(),
            member_count: 5,
            status: status.to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
    }

    // ── D-01: 合法 active 房间，返回完整 RoomDetailResponse ─────────────

    /// D-01: active 房间存在 → 返回正确的 room_id / title / member_count / owner 等字段
    #[tokio::test]
    async fn d01_active_room_returns_full_detail() {
        let (svc, repo) = make_detail_service();
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        repo.seed_user(
            owner_id,
            "Alice".to_string(),
            Some("https://example.com/avatar.jpg".to_string()),
        );
        seed_room_for_detail(&repo, room_id, owner_id, "Test Room", "active", None);

        let resp = svc.get_room_detail(room_id).await.unwrap();

        assert_eq!(resp.room_id, room_id.to_string());
        assert_eq!(resp.title, "Test Room");
        assert_eq!(resp.room_type, "normal");
        assert_eq!(resp.member_count, 5);
        assert_eq!(resp.max_members, 50);
        assert_eq!(resp.owner.user_id, owner_id.to_string());
        assert_eq!(resp.owner.nickname, "Alice");
        assert_eq!(
            resp.owner.avatar,
            Some("https://example.com/avatar.jpg".to_string())
        );
    }

    // ── D-02: 不存在的 room_id 返回 NotFound ─────────────────────────────

    /// D-02: room_id 完全不存在 → AppError::NotFound
    #[tokio::test]
    async fn d02_nonexistent_room_returns_not_found() {
        let (svc, _) = make_detail_service();
        let random_id = Uuid::new_v4();
        let err = svc.get_room_detail(random_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "expected NotFound, got: {err:?}"
        );
    }

    // ── D-03: status='closed' 房间返回 NotFound ──────────────────────────

    /// D-03: status='closed' 时不当作活跃房间 → AppError::NotFound
    #[tokio::test]
    async fn d03_closed_room_returns_not_found() {
        let (svc, repo) = make_detail_service();
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Bob".to_string(), None);
        seed_room_for_detail(&repo, room_id, owner_id, "Closed Room", "closed", None);

        let err = svc.get_room_detail(room_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "expected NotFound for closed room, got: {err:?}"
        );
    }

    // ── D-04: soft-deleted 房间返回 NotFound ─────────────────────────────

    /// D-04: deleted_at IS NOT NULL 的房间 → AppError::NotFound
    #[tokio::test]
    async fn d04_soft_deleted_room_returns_not_found() {
        let (svc, repo) = make_detail_service();
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Charlie".to_string(), None);
        seed_room_for_detail(
            &repo,
            room_id,
            owner_id,
            "Deleted Room",
            "active",
            Some(Utc::now()),
        );

        let err = svc.get_room_detail(room_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "expected NotFound for soft-deleted room, got: {err:?}"
        );
    }

    // ── D-05: mic_slots 为空数组 ──────────────────────────────────────────

    /// D-05: MVP 阶段 mic_slots 固定为空数组
    #[tokio::test]
    async fn d05_mic_slots_is_empty_array() {
        let (svc, repo) = make_detail_service();
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Dave".to_string(), None);
        seed_room_for_detail(&repo, room_id, owner_id, "Test Room", "active", None);

        let resp = svc.get_room_detail(room_id).await.unwrap();
        assert!(
            resp.mic_slots.is_empty(),
            "mic_slots should be empty array in MVP"
        );
    }

    // ── D-06: owner_avatar 为 None 时正确 ───────────────────────────────

    /// D-06: 用户没有 avatar 时 owner.avatar 应为 None（不崩溃）
    #[tokio::test]
    async fn d06_owner_avatar_none_handled_correctly() {
        let (svc, repo) = make_detail_service();
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Eve".to_string(), None); // no avatar
        seed_room_for_detail(&repo, room_id, owner_id, "Test Room", "active", None);

        let resp = svc.get_room_detail(room_id).await.unwrap();
        assert!(
            resp.owner.avatar.is_none(),
            "owner.avatar should be None when user has no avatar"
        );
    }

    // ── D-07: created_at 为合法 RFC3339 格式 ────────────────────────────

    /// D-07: created_at 字段必须是合法的 RFC3339 时间戳格式
    #[tokio::test]
    async fn d07_created_at_is_valid_rfc3339() {
        let (svc, repo) = make_detail_service();
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Frank".to_string(), None);
        seed_room_for_detail(&repo, room_id, owner_id, "Test Room", "active", None);

        let resp = svc.get_room_detail(room_id).await.unwrap();

        // RFC3339 必须包含 'T' 分隔符
        assert!(
            resp.created_at.contains('T'),
            "created_at should contain 'T' separator: {}",
            resp.created_at
        );
        // 必须能被解析为 DateTime
        let parsed = chrono::DateTime::parse_from_rfc3339(&resp.created_at);
        assert!(
            parsed.is_ok(),
            "created_at should be valid RFC3339, got: {}",
            resp.created_at
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00010: close_room 单元测试（U-C-01 ~ U-C-07）
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试辅助：构建 close_room 专用的 (service, repo) 对
    fn make_close_service() -> (RoomService, Arc<FakeRoomRepository>) {
        let repo = Arc::new(FakeRoomRepository::default());
        let svc = RoomService::new(repo.clone());
        (svc, repo)
    }

    /// 测试辅助：向 repo 注入一个可自定义 status / deleted_at 的房间，返回 room_id
    fn seed_room_close(
        repo: &FakeRoomRepository,
        owner_id: Uuid,
        status: &str,
        deleted_at: Option<chrono::DateTime<Utc>>,
    ) -> Uuid {
        let now = Utc::now();
        let room_id = Uuid::new_v4();
        repo.seed(voice_room_shared::models::room::RoomModel {
            id: room_id,
            owner_id,
            title: "Test Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: status.to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        room_id
    }

    // ── U-C-01: 合法房主关闭 active 房间 → Ok(()) ──────────────────────────

    /// U-C-01: 合法房主关闭 active 房间 → Ok(())
    #[tokio::test]
    async fn uc01_owner_closes_active_room_ok() {
        let (svc, repo) = make_close_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_room_close(&repo, owner_id, "active", None);

        let result = svc.close_room(room_id, owner_id).await;
        assert!(result.is_ok(), "owner should be able to close active room, got: {result:?}");
    }

    // ── U-C-02: 不存在的 room_id → NotFound ─────────────────────────────────

    /// U-C-02: room_id 在 repo 中完全不存在 → AppError::NotFound
    #[tokio::test]
    async fn uc02_nonexistent_room_returns_not_found() {
        let (svc, _) = make_close_service();
        let nonexistent_id = Uuid::new_v4();
        let err = svc.close_room(nonexistent_id, Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "expected NotFound for nonexistent room_id, got: {err:?}"
        );
    }

    // ── U-C-03: 非房主 → Forbidden ───────────────────────────────────────────

    /// U-C-03: current_user_id != owner_id → AppError::Forbidden
    #[tokio::test]
    async fn uc03_non_owner_returns_forbidden() {
        let (svc, repo) = make_close_service();
        let owner_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let room_id = seed_room_close(&repo, owner_id, "active", None);

        let err = svc.close_room(room_id, other_user).await.unwrap_err();
        assert!(
            matches!(err, AppError::Forbidden(_)),
            "expected Forbidden for non-owner, got: {err:?}"
        );
    }

    // ── U-C-04: 已 closed 房间 → RoomAlreadyClosed ──────────────────────────

    /// U-C-04: 房间 status 已经是 'closed' → AppError::RoomAlreadyClosed
    #[tokio::test]
    async fn uc04_already_closed_room_returns_room_already_closed() {
        let (svc, repo) = make_close_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_room_close(&repo, owner_id, "closed", None);

        let err = svc.close_room(room_id, owner_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::RoomAlreadyClosed),
            "expected RoomAlreadyClosed for already-closed room, got: {err:?}"
        );
    }

    // ── U-C-05: 软删除房间 → NotFound ────────────────────────────────────────

    /// U-C-05: deleted_at IS NOT NULL（软删除）→ AppError::NotFound
    #[tokio::test]
    async fn uc05_soft_deleted_room_returns_not_found() {
        let (svc, repo) = make_close_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_room_close(&repo, owner_id, "active", Some(Utc::now()));

        let err = svc.close_room(room_id, owner_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "expected NotFound for soft-deleted room, got: {err:?}"
        );
    }

    // ── U-C-07: 关闭后 find_room_by_id 返回 None ────────────────────────────

    /// U-C-07: close_room 成功后，find_room_by_id（只查 active）应返回 None
    #[tokio::test]
    async fn uc07_after_close_find_room_by_id_returns_none() {
        let (svc, repo) = make_close_service();
        let owner_id = Uuid::new_v4();
        repo.seed_user(owner_id, "Owner".to_string(), None);
        let room_id = seed_room_close(&repo, owner_id, "active", None);

        // 关闭前可以找到
        let before = repo.find_room_by_id(room_id).await.unwrap();
        assert!(before.is_some(), "room should exist as active before close");

        // 执行关闭
        svc.close_room(room_id, owner_id).await.unwrap();

        // 关闭后 find_room_by_id 应返回 None（因为 status='closed'，不是 active）
        let after = repo.find_room_by_id(room_id).await.unwrap();
        assert!(
            after.is_none(),
            "find_room_by_id should return None after room is closed"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00025: 创建房间 API 升级单元测试（CR25-01 ~ CR25-07）
    // ═══════════════════════════════════════════════════════════════════════

    /// CR25-01: 成功创建带所有新字段的房间 → 201
    #[tokio::test]
    async fn cr25_01_create_room_with_all_fields_succeeds() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "中东夜话".to_string(),
            room_type: "normal".to_string(),
            password: None,
            cover_url: Some("/assets/covers/desert.png".to_string()),
            category: Some("chat".to_string()),
            announcement: Some("欢迎来到中东夜话~".to_string()),
        };
        let resp = svc.create_room(Uuid::new_v4(), req).await.unwrap();
        assert!(!resp.room_id.is_empty(), "CR25-01: room_id should not be empty");
        assert_eq!(resp.title, "中东夜话");
    }

    /// CR25-02: password="12345"（5 位）→ 40003
    #[tokio::test]
    async fn cr25_02_invalid_password_length_returns_validation_error() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "密码房".to_string(),
            room_type: "password".to_string(),
            password: Some("12345".to_string()), // 5 位
            cover_url: None,
            category: None,
            announcement: None,
        };
        let err = svc.create_room(Uuid::new_v4(), req).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "CR25-02: 5-digit password should be ValidationError, got: {err:?}"
        );
    }

    /// CR25-03: category="unknown" → 40003
    #[tokio::test]
    async fn cr25_03_invalid_category_returns_validation_error() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "分类房".to_string(),
            room_type: "normal".to_string(),
            password: None,
            cover_url: None,
            category: Some("unknown".to_string()),
            announcement: None,
        };
        let err = svc.create_room(Uuid::new_v4(), req).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "CR25-03: invalid category should be ValidationError, got: {err:?}"
        );
    }

    /// CR25-04: announcement 超 200 字符 → 40003
    #[tokio::test]
    async fn cr25_04_announcement_over_200_chars_returns_validation_error() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "公告房".to_string(),
            room_type: "normal".to_string(),
            password: None,
            cover_url: None,
            category: None,
            announcement: Some("音".repeat(201)),
        };
        let err = svc.create_room(Uuid::new_v4(), req).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "CR25-04: 201-char announcement should be ValidationError, got: {err:?}"
        );
    }

    /// CR25-05: cover_url 非白名单 → 40003
    #[tokio::test]
    async fn cr25_05_invalid_cover_url_returns_validation_error() {
        let (svc, _) = make_service();
        let req = CreateRoomRequest {
            title: "封面房".to_string(),
            room_type: "normal".to_string(),
            password: None,
            cover_url: Some("https://evil.com/hack.png".to_string()),
            category: None,
            announcement: None,
        };
        let err = svc.create_room(Uuid::new_v4(), req).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "CR25-05: non-whitelist cover_url should be ValidationError, got: {err:?}"
        );
    }

    /// CR25-06: room_type=normal 时带 password 被忽略（password_hash NULL）
    #[tokio::test]
    async fn cr25_06_normal_room_password_ignored() {
        let (svc, repo) = make_service();
        let owner_id = Uuid::new_v4();
        let req = CreateRoomRequest {
            title: "普通房".to_string(),
            room_type: "normal".to_string(),
            password: Some("123456".to_string()), // 应被忽略
            cover_url: None,
            category: None,
            announcement: None,
        };
        svc.create_room(owner_id, req).await.unwrap();
        let room = repo
            .find_active_by_owner(owner_id)
            .await
            .unwrap()
            .expect("room should exist");
        assert!(
            room.password_hash.is_none(),
            "CR25-06: normal room should NOT store password_hash"
        );
    }

    /// CR25-07: 用户已有活跃房间 → 409
    #[tokio::test]
    async fn cr25_07_active_room_exists_returns_conflict() {
        let (svc, repo) = make_service();
        let owner_id = Uuid::new_v4();
        seed_active_room(&repo, owner_id);
        let req = CreateRoomRequest {
            title: "Second Room".to_string(),
            room_type: "normal".to_string(),
            password: None,
            cover_url: None,
            category: None,
            announcement: None,
        };
        let err = svc.create_room(owner_id, req).await.unwrap_err();
        assert!(
            matches!(err, AppError::ActiveRoomExists),
            "CR25-07: should be ActiveRoomExists, got: {err:?}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00025: PATCH /api/v1/rooms/:id 单元测试（PR25-08 ~ PR25-12）
    // ═══════════════════════════════════════════════════════════════════════

    fn make_patch_service() -> (RoomService, Arc<FakeRoomRepository>) {
        let repo = Arc::new(FakeRoomRepository::default());
        let svc = RoomService::new(repo.clone());
        (svc, repo)
    }

    /// 辅助：植入 active 房间，返回 room_id
    fn seed_active_room_for_patch(repo: &FakeRoomRepository, owner_id: Uuid) -> Uuid {
        let now = Utc::now();
        let room_id = Uuid::new_v4();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Original Title".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        room_id
    }

    /// 辅助：植入 closed 房间，返回 room_id
    fn seed_closed_room_for_patch(repo: &FakeRoomRepository, owner_id: Uuid) -> Uuid {
        let now = Utc::now();
        let room_id = Uuid::new_v4();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Closed Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "closed".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        room_id
    }

    /// PR25-08: 房主 PATCH 成功 → title/category/announcement 被更新
    #[tokio::test]
    async fn pr25_08_owner_patch_succeeds() {
        let (svc, repo) = make_patch_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_active_room_for_patch(&repo, owner_id);

        let req = PatchRoomRequest {
            title: Some("新标题".to_string()),
            announcement: Some("欢迎来到新房间".to_string()),
            category: Some("music".to_string()),
        };

        let resp = svc.patch_room(room_id, owner_id, req).await.unwrap();
        assert_eq!(resp.room_id, room_id.to_string(), "PR25-08: room_id must match");
        assert_eq!(resp.title, "新标题", "PR25-08: title should be updated");
        assert_eq!(resp.category, "music", "PR25-08: category should be updated");
        assert_eq!(
            resp.announcement.as_deref(),
            Some("欢迎来到新房间"),
            "PR25-08: announcement should be updated"
        );
    }

    /// PR25-09: 非房主 PATCH → 403
    #[tokio::test]
    async fn pr25_09_non_owner_patch_returns_forbidden() {
        let (svc, repo) = make_patch_service();
        let owner_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let room_id = seed_active_room_for_patch(&repo, owner_id);

        let req = PatchRoomRequest {
            title: Some("非法改名".to_string()),
            announcement: None,
            category: None,
        };

        let err = svc
            .patch_room(room_id, other_user, req)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Forbidden(_)),
            "PR25-09: non-owner should get Forbidden, got: {err:?}"
        );
    }

    /// PR25-10: 房间 closed PATCH → 409
    #[tokio::test]
    async fn pr25_10_closed_room_patch_returns_conflict() {
        let (svc, repo) = make_patch_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_closed_room_for_patch(&repo, owner_id);

        let req = PatchRoomRequest {
            title: Some("试图改名".to_string()),
            announcement: None,
            category: None,
        };

        let err = svc.patch_room(room_id, owner_id, req).await.unwrap_err();
        assert!(
            matches!(err, AppError::RoomAlreadyClosed),
            "PR25-10: closed room PATCH should return RoomAlreadyClosed, got: {err:?}"
        );
    }

    /// PR25-11: PATCH 空 body → 40003
    #[tokio::test]
    async fn pr25_11_empty_patch_body_returns_validation_error() {
        let (svc, repo) = make_patch_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_active_room_for_patch(&repo, owner_id);

        let req = PatchRoomRequest {
            title: None,
            announcement: None,
            category: None,
        };

        let err = svc.patch_room(room_id, owner_id, req).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "PR25-11: empty PATCH body should be ValidationError, got: {err:?}"
        );
    }

    /// PR25-12: announcement 空串清空 + has_password 布尔正确
    #[tokio::test]
    async fn pr25_12_clear_announcement_and_has_password_correct() {
        let (svc, repo) = make_patch_service();
        let owner_id = Uuid::new_v4();
        let room_id = seed_active_room_for_patch(&repo, owner_id);

        let req = PatchRoomRequest {
            title: None,
            announcement: Some("".to_string()), // 空串 = 清空
            category: Some("chat".to_string()),
        };

        let resp = svc.patch_room(room_id, owner_id, req).await.unwrap();
        assert!(
            resp.announcement.is_none(),
            "PR25-12: empty string announcement should clear it (None)"
        );
        assert_eq!(
            resp.has_password,
            false,
            "PR25-12: normal room should have has_password=false"
        );
    }
}
