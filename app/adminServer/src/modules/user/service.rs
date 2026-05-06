use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::event::publisher::{AdminEvent, BanUserPayload, EventPublisher, UnbanUserPayload};

use super::{
    dto::{
        AdminBanUserRequest, AdminBanUserResponse, AdminUserDetailResponse, AdminUserFilter,
        AdminUserItem, AdminUserListQuery, AdminUserListResponse,
    },
    repository::AdminUserRepository,
};

// ─── AdminUserService ─────────────────────────────────────────────────────────

/// 用户列表业务逻辑层。
///
/// 职责：
/// 1. 参数处理（page/size 默认值与 clamp）
/// 2. status 字符串 → is_banned 布尔映射
/// 3. 构建过滤器传递给仓库
/// 4. 组装分页响应
pub struct AdminUserService {
    user_repo: Arc<dyn AdminUserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl AdminUserService {
    pub fn new(
        user_repo: Arc<dyn AdminUserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            user_repo,
            event_publisher,
        }
    }

    /// 查询用户列表，返回分页结果。
    ///
    /// # 默认值与 clamp 规则
    /// - page: 默认 1，最小 1（< 1 自动 clamp 为 1）
    /// - size: 默认 20，最大 100（> 100 自动 clamp 为 100）
    ///
    /// # status 映射
    /// - `"normal"` → `is_banned = Some(false)`
    /// - `"banned"` → `is_banned = Some(true)`
    /// - `None` / 其他 → `is_banned = None`（全部）
    pub async fn list_users(
        &self,
        query: AdminUserListQuery,
    ) -> Result<AdminUserListResponse, AppError> {
        // ── 参数 clamp（不返回错误，与 room 模块不同）──────────────────────────
        let page = query.page.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);

        // ── status 字符串 → is_banned 布尔映射 ────────────────────────────────
        let is_banned = match query.status.as_deref() {
            Some("normal") => Some(false),
            Some("banned") => Some(true),
            _ => None,
        };

        // ── 构建过滤器 ─────────────────────────────────────────────────────────
        let filter = AdminUserFilter {
            phone: query.phone,
            user_id: query.user_id,
            nickname: query.nickname,
            is_banned,
        };

        // ── 查询仓库 ───────────────────────────────────────────────────────────
        let total = self.user_repo.count_users(&filter).await?;
        let offset = ((page - 1) as i64) * (size as i64);
        let rows = self
            .user_repo
            .find_users(&filter, offset, size as i64)
            .await?;

        Ok(AdminUserListResponse {
            total,
            page,
            size,
            items: rows.into_iter().map(AdminUserItem::from).collect(),
        })
    }

    /// 查询单个用户详情。
    ///
    /// # 错误
    /// - `AppError::UserNotFound` → HTTP 404：用户不存在或已软删除
    /// - `AppError::DatabaseError` → HTTP 500：数据库内部错误
    pub async fn get_user_detail(
        &self,
        user_id: Uuid,
    ) -> Result<AdminUserDetailResponse, AppError> {
        match self.user_repo.find_user_by_id(user_id).await? {
            Some(row) => Ok(AdminUserDetailResponse::from(row)),
            None => Err(AppError::UserNotFound("用户不存在".to_string())),
        }
    }

    /// 封禁或解封用户。
    ///
    /// # 参数
    /// - `user_id`：目标用户 ID
    /// - `req`：包含 action("ban"|"unban") 的请求体
    ///
    /// # 错误
    /// - `AppError::ValidationError` → HTTP 400：action 非法
    /// - `AppError::UserNotFound` → HTTP 404：用户不存在或已软删除
    /// - `AppError::UserAlreadyBanned` → HTTP 409：用户已封禁，重复 ban
    /// - `AppError::UserAlreadyNormal` → HTTP 409：用户已正常，重复 unban
    /// - `AppError::DatabaseError` → HTTP 500：数据库内部错误
    pub async fn ban_user(
        &self,
        operator_id: Uuid,
        user_id: Uuid,
        req: AdminBanUserRequest,
    ) -> Result<AdminBanUserResponse, AppError> {
        // 校验 action
        if req.action != "ban" && req.action != "unban" {
            return Err(AppError::ValidationError(format!(
                "invalid action: '{}', must be 'ban' or 'unban'",
                req.action
            )));
        }

        // 查询用户是否存在
        let user = match self.user_repo.find_user_by_id(user_id).await? {
            Some(u) => u,
            None => return Err(AppError::UserNotFound("用户不存在".to_string())),
        };

        let is_banned = req.action == "ban";

        // 幂等校验
        if is_banned && user.is_banned {
            return Err(AppError::UserAlreadyBanned);
        }
        if !is_banned && !user.is_banned {
            return Err(AppError::UserAlreadyNormal);
        }

        // P0-2: ban 校验 — 拒绝错位/缺失字段，避免静默写入 duration_hours=0
        if is_banned {
            let ban_type = req.ban_type.as_deref().unwrap_or("permanent");
            match ban_type {
                "permanent" => {
                    if req.duration_hours.is_some() {
                        return Err(AppError::ValidationError(
                            "duration_hours must be omitted when ban_type=permanent".to_string(),
                        ));
                    }
                }
                "temporary" => match req.duration_hours {
                    None => {
                        return Err(AppError::ValidationError(
                            "duration_hours is required when ban_type=temporary".to_string(),
                        ));
                    }
                    Some(0) => {
                        return Err(AppError::ValidationError(
                            "duration_hours must be > 0 for temporary ban".to_string(),
                        ));
                    }
                    Some(_) => {}
                },
                other => {
                    return Err(AppError::ValidationError(format!(
                        "invalid ban_type: '{}', must be 'permanent' or 'temporary'",
                        other
                    )));
                }
            }
        }

        // 更新封禁状态
        self.user_repo.update_ban_status(user_id, is_banned).await?;

        // P1-7: 移除业务层 tracing 伪审计；审计统一由 controller 调用 audit_logger.log_action

        // 发布管理事件（fire-and-forget，失败不影响主业务）
        // T-00105: 使用 strict AdminEvent enum（来自 voice_room_shared），消除字符串拼写错误风险
        let event = if is_banned {
            AdminEvent::BanUser {
                payload: BanUserPayload { user_id },
                admin_id: operator_id,
                ts: chrono::Utc::now().timestamp_millis(),
            }
        } else {
            AdminEvent::UnbanUser {
                payload: UnbanUserPayload { user_id },
                admin_id: operator_id,
                ts: chrono::Utc::now().timestamp_millis(),
            }
        };
        if let Err(e) = self.event_publisher.publish("admin:events", event).await {
            // fire-and-forget：发布失败不影响主业务，仅记录警告
            tracing::warn!(error = %e, "failed to publish admin event");
        }

        Ok(AdminBanUserResponse {
            id: user_id.to_string(),
            status: if is_banned {
                "banned".to_string()
            } else {
                "normal".to_string()
            },
        })
    }
}

// ─── Service 单元测试（TDD T-10007 验收用例 S-01~S-06）──────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::user::repository::FakeAdminUserRepository;
    use chrono::Utc;
    use uuid::Uuid;

    use super::super::dto::AdminUserListRow;

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    fn make_row(phone: &str, nickname: &str, is_banned: bool) -> AdminUserListRow {
        AdminUserListRow {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: None,
            coin_balance: 500,
            vip_level: 1,
            is_banned,
            created_at: Utc::now(),
        }
    }

    fn make_service() -> (AdminUserService, Arc<FakeAdminUserRepository>) {
        let repo = Arc::new(FakeAdminUserRepository::default());
        let publisher = Arc::new(crate::modules::event::publisher::NoopEventPublisher::default());
        let svc = AdminUserService::new(
            repo.clone() as Arc<dyn AdminUserRepository>,
            publisher as Arc<dyn crate::modules::event::publisher::EventPublisher>,
        );
        (svc, repo)
    }

    fn query(
        page: Option<u32>,
        size: Option<u32>,
        status: Option<&str>,
        phone: Option<&str>,
        nickname: Option<&str>,
    ) -> AdminUserListQuery {
        AdminUserListQuery {
            page,
            size,
            status: status.map(str::to_string),
            phone: phone.map(str::to_string),
            nickname: nickname.map(str::to_string),
            user_id: None,
        }
    }

    // ── S-01: 无过滤条件，返回 AdminUserListResponse 结构正确 ────────────────
    #[tokio::test]
    async fn s01_no_filter_returns_correct_response_structure() {
        let (svc, repo) = make_service();
        repo.seed(make_row("13800000001", "Alice", false));
        repo.seed(make_row("13800000002", "Bob", false));

        let result = svc
            .list_users(query(None, None, None, None, None))
            .await
            .unwrap();

        assert_eq!(result.total, 2, "S-01: total 应为 2");
        assert_eq!(result.page, 1, "S-01: page 默认为 1");
        assert_eq!(result.size, 20, "S-01: size 默认为 20");
        assert_eq!(result.items.len(), 2, "S-01: items 应有 2 条");
    }

    // ── S-02: status="normal" → filter.is_banned = Some(false) ──────────────
    #[tokio::test]
    async fn s02_status_normal_maps_to_is_banned_false() {
        let (svc, repo) = make_service();
        repo.seed(make_row("111", "Normal User", false));
        repo.seed(make_row("222", "Banned User", true));

        let result = svc
            .list_users(query(None, None, Some("normal"), None, None))
            .await
            .unwrap();

        assert_eq!(result.total, 1, "S-02: status=normal 应只返回 1 个正常用户");
        assert_eq!(
            result.items[0].status, "normal",
            "S-02: 返回的用户 status 应为 normal"
        );
    }

    // ── S-03: status="banned" → filter.is_banned = Some(true) ───────────────
    #[tokio::test]
    async fn s03_status_banned_maps_to_is_banned_true() {
        let (svc, repo) = make_service();
        repo.seed(make_row("111", "Normal User", false));
        repo.seed(make_row("222", "Banned User 1", true));
        repo.seed(make_row("333", "Banned User 2", true));

        let result = svc
            .list_users(query(None, None, Some("banned"), None, None))
            .await
            .unwrap();

        assert_eq!(result.total, 2, "S-03: status=banned 应返回 2 个封禁用户");
        for item in &result.items {
            assert_eq!(item.status, "banned", "S-03: 每个 item.status 应为 banned");
        }
    }

    // ── S-04: 空结果时 total=0, items=[] ─────────────────────────────────────
    #[tokio::test]
    async fn s04_empty_result_total_zero_items_empty() {
        let (svc, _) = make_service();

        let result = svc
            .list_users(query(None, None, None, None, None))
            .await
            .unwrap();

        assert_eq!(result.total, 0, "S-04: 空仓库 total 应为 0");
        assert!(result.items.is_empty(), "S-04: 空仓库 items 应为空");
    }

    // ── S-05: size > 100 时 clamp 为 100 ────────────────────────────────────
    #[tokio::test]
    async fn s05_size_over_100_clamped_to_100() {
        let (svc, _) = make_service();

        let result = svc
            .list_users(query(None, Some(999), None, None, None))
            .await
            .unwrap();

        assert_eq!(result.size, 100, "S-05: size > 100 应 clamp 为 100");
    }

    // ── S-06: AdminUserItem.status 字段映射正确 ──────────────────────────────
    #[tokio::test]
    async fn s06_user_item_status_field_maps_is_banned_correctly() {
        let (svc, repo) = make_service();
        repo.seed(make_row("111", "Normal", false));
        repo.seed(make_row("222", "Banned", true));

        let result = svc
            .list_users(query(None, None, None, None, None))
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2, "S-06: 应有 2 条 items");
        // 验证 is_banned → status 映射
        let normal_item = result.items.iter().find(|i| i.phone == "111").unwrap();
        let banned_item = result.items.iter().find(|i| i.phone == "222").unwrap();
        assert_eq!(
            normal_item.status, "normal",
            "S-06: is_banned=false 应映射为 normal"
        );
        assert_eq!(
            banned_item.status, "banned",
            "S-06: is_banned=true 应映射为 banned"
        );
    }

    // ── 额外：page clamp（page=0 → 1）────────────────────────────────────────
    #[tokio::test]
    async fn s_page_zero_clamped_to_1() {
        let (svc, _) = make_service();
        let result = svc
            .list_users(query(Some(0), None, None, None, None))
            .await
            .unwrap();
        assert_eq!(result.page, 1, "page=0 应 clamp 为 1");
    }

    // ── 额外：pagination 分页结果正确 ─────────────────────────────────────────
    #[tokio::test]
    async fn s_pagination_returns_correct_slice() {
        let (svc, repo) = make_service();
        for i in 0..7 {
            repo.seed(make_row(
                &format!("138000000{:02}", i),
                &format!("User{:02}", i),
                false,
            ));
        }

        let result = svc
            .list_users(query(Some(2), Some(3), None, None, None))
            .await
            .unwrap();

        assert_eq!(result.total, 7, "分页: total 应为 7");
        assert_eq!(result.page, 2, "分页: page 应为 2");
        assert_eq!(result.size, 3, "分页: size 应为 3");
        assert_eq!(result.items.len(), 3, "分页: 第 2 页应有 3 条");
    }

    // ── 额外：user_id 精确过滤通过 service 层 ────────────────────────────────
    #[tokio::test]
    async fn s_user_id_filter_via_service() {
        let (svc, repo) = make_service();
        let target_id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id: target_id,
            phone: "99999999999".to_string(),
            nickname: "Target".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        });
        repo.seed(make_row("111", "Other", false));

        let result = svc
            .list_users(AdminUserListQuery {
                user_id: Some(target_id),
                phone: None,
                nickname: None,
                status: None,
                page: None,
                size: None,
            })
            .await
            .unwrap();

        assert_eq!(result.total, 1, "user_id 过滤应只返回 1 条");
        assert_eq!(result.items[0].id, target_id.to_string());
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10008 Service 测试 SD-01~04
    // ════════════════════════════════════════════════════════════════════════

    // ── SD-01: 正常情况：用户存在 → 返回 AdminUserDetailResponse，字段映射正确 ──
    #[tokio::test]
    async fn sd01_existing_user_returns_detail_response_with_correct_fields() {
        let (svc, repo) = make_service();
        let id = Uuid::new_v4();
        let row = AdminUserListRow {
            id,
            phone: "+8613800138001".to_string(),
            nickname: "TestUser".to_string(),
            avatar: Some("https://cdn.example.com/avatar.jpg".to_string()),
            coin_balance: 1000,
            vip_level: 2,
            is_banned: false,
            created_at: Utc::now(),
        };
        repo.seed(row);

        let result = svc.get_user_detail(id).await.unwrap();

        assert_eq!(result.id, id.to_string(), "SD-01: id 应映射为 String");
        assert_eq!(result.phone, "+8613800138001", "SD-01: phone 字段应一致");
        assert_eq!(result.nickname, "TestUser", "SD-01: nickname 字段应一致");
        assert_eq!(
            result.avatar_url,
            Some("https://cdn.example.com/avatar.jpg".to_string()),
            "SD-01: avatar → avatar_url 映射应正确"
        );
        assert_eq!(result.coin_balance, 1000, "SD-01: coin_balance 应正确");
        assert_eq!(result.vip_level, 2, "SD-01: vip_level 应正确");
        assert_eq!(
            result.status, "normal",
            "SD-01: is_banned=false → status='normal'"
        );
        assert!(
            result.recharge_records.is_empty(),
            "SD-01: recharge_records MVP 应为空数组"
        );
        assert!(
            result.consume_records.is_empty(),
            "SD-01: consume_records MVP 应为空数组"
        );
        assert!(result.devices.is_empty(), "SD-01: devices MVP 应为空数组");
    }

    // ── SD-02: 用户不存在 → get_user_detail 返回 AppError::UserNotFound ────────
    #[tokio::test]
    async fn sd02_nonexistent_user_returns_user_not_found_error() {
        let (svc, _) = make_service();
        let nonexistent_id = Uuid::new_v4();

        let err = svc.get_user_detail(nonexistent_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::UserNotFound(_)),
            "SD-02: 用户不存在应返回 AppError::UserNotFound，实际: {err:?}"
        );
    }

    // ── SD-03: 用户 is_banned=true → status="banned" 映射正确 ────────────────
    #[tokio::test]
    async fn sd03_banned_user_status_maps_to_banned_string() {
        let (svc, repo) = make_service();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800138003".to_string(),
            nickname: "BannedUser".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: true, // 封禁状态
            created_at: Utc::now(),
        });

        let result = svc.get_user_detail(id).await.unwrap();
        assert_eq!(
            result.status, "banned",
            "SD-03: is_banned=true → status 应为 'banned'"
        );
    }

    // ── SD-04: DB 错误 → get_user_detail 透传 AppError::DatabaseError (HTTP 500) ──
    #[tokio::test]
    async fn sd04_db_error_propagates_as_database_error() {
        let repo = Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default());
        repo.inject_find_by_id_error(); // 注入 DB 错误

        let svc = super::AdminUserService::new(
            repo as Arc<dyn crate::modules::user::repository::AdminUserRepository>,
            Arc::new(crate::modules::event::publisher::NoopEventPublisher::default()),
        );

        let err = svc.get_user_detail(Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::DatabaseError(_)),
            "SD-04: DB 错误应透传为 AppError::DatabaseError (HTTP 500)，实际: {err:?}"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10009 Service 测试 SB-01~05
    // ════════════════════════════════════════════════════════════════════════

    use super::super::dto::AdminBanUserRequest;

    fn ban_req(action: &str) -> AdminBanUserRequest {
        AdminBanUserRequest {
            action: action.to_string(),
            ban_type: None,
            duration_hours: None,
            reason: None,
        }
    }

    // ── SB-01: 正常封禁：用户存在且未封禁 → Ok(response)，status="banned" ─────
    #[tokio::test]
    async fn sb01_ban_normal_user_returns_banned_status() {
        let (svc, repo) = make_service();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800000001".to_string(),
            nickname: "SB01User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        });

        let result = svc
            .ban_user(Uuid::new_v4(), id, ban_req("ban"))
            .await
            .unwrap();
        assert_eq!(
            result.id,
            id.to_string(),
            "SB-01: 返回的 id 应与用户 id 一致"
        );
        assert_eq!(result.status, "banned", "SB-01: 封禁后 status 应为 banned");

        // 验证 update_ban_status 已被调用（通过 find_user_by_id 查验状态）
        let found = repo.find_user_by_id(id).await.unwrap().unwrap();
        assert!(found.is_banned, "SB-01: 封禁后 is_banned 应为 true");
    }

    // ── SB-02: 正常解封：用户已封禁 → Ok(response)，status="normal" ─────────────
    #[tokio::test]
    async fn sb02_unban_banned_user_returns_normal_status() {
        let (svc, repo) = make_service();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800000002".to_string(),
            nickname: "SB02User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: true, // 已封禁
            created_at: Utc::now(),
        });

        let result = svc
            .ban_user(Uuid::new_v4(), id, ban_req("unban"))
            .await
            .unwrap();
        assert_eq!(
            result.id,
            id.to_string(),
            "SB-02: 返回的 id 应与用户 id 一致"
        );
        assert_eq!(result.status, "normal", "SB-02: 解封后 status 应为 normal");
    }

    // ── SB-03: 用户不存在 → AppError::UserNotFound (HTTP 404) ──────────────────
    #[tokio::test]
    async fn sb03_nonexistent_user_returns_user_not_found() {
        let (svc, _) = make_service();
        let nonexistent_id = Uuid::new_v4();

        let err = svc
            .ban_user(Uuid::new_v4(), nonexistent_id, ban_req("ban"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::UserNotFound(_)),
            "SB-03: 用户不存在应返回 UserNotFound，实际: {err:?}"
        );
    }

    // ── SB-04: 用户已封禁（is_banned=true）再次 ban → AppError::UserAlreadyBanned ──
    #[tokio::test]
    async fn sb04_ban_already_banned_user_returns_already_banned() {
        let (svc, repo) = make_service();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800000004".to_string(),
            nickname: "SB04User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: true, // 已封禁
            created_at: Utc::now(),
        });

        let err = svc
            .ban_user(Uuid::new_v4(), id, ban_req("ban"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::UserAlreadyBanned),
            "SB-04: 重复 ban 应返回 UserAlreadyBanned (409)，实际: {err:?}"
        );
    }

    // ── SB-05: 用户已正常（is_banned=false）再次 unban → AppError::UserAlreadyNormal ──
    #[tokio::test]
    async fn sb05_unban_normal_user_returns_already_normal() {
        let (svc, repo) = make_service();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800000005".to_string(),
            nickname: "SB05User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false, // 正常状态
            created_at: Utc::now(),
        });

        let err = svc
            .ban_user(Uuid::new_v4(), id, ban_req("unban"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::UserAlreadyNormal),
            "SB-05: 重复 unban 应返回 UserAlreadyNormal (409)，实际: {err:?}"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10011 Service 测试 SB-06~08（事件发布）
    // ════════════════════════════════════════════════════════════════════════

    use crate::modules::event::publisher::{
        ErrorEventPublisher, EventPublisher, NoopEventPublisher,
    };

    fn make_service_with_publisher() -> (
        AdminUserService,
        Arc<FakeAdminUserRepository>,
        Arc<NoopEventPublisher>,
    ) {
        let repo = Arc::new(FakeAdminUserRepository::default());
        let publisher = Arc::new(NoopEventPublisher::default());
        let svc = AdminUserService::new(
            repo.clone() as Arc<dyn AdminUserRepository>,
            publisher.clone() as Arc<dyn EventPublisher>,
        );
        (svc, repo, publisher)
    }

    // ── SB-06: ban 操作后 NoopEventPublisher 收到 1 次 "admin:events" 调用，event.type="ban_user" ──
    #[tokio::test]
    async fn sb06_ban_publishes_ban_user_event_to_admin_events() {
        let (svc, repo, publisher) = make_service_with_publisher();
        let operator_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id: user_id,
            phone: "+8613800000006".to_string(),
            nickname: "SB06User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        });

        let result = svc.ban_user(operator_id, user_id, ban_req("ban")).await;
        assert!(result.is_ok(), "SB-06: ban 操作应返回 Ok");

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "SB-06: 应发布恰好 1 次事件");
        assert_eq!(
            calls[0].0, "admin:events",
            "SB-06: channel 应为 admin:events"
        );
        // T-00105: 使用枚举变体匹配，不再依赖字符串类型字段
        match &calls[0].1 {
            crate::modules::event::publisher::AdminEvent::BanUser { payload, admin_id, .. } => {
                assert_eq!(payload.user_id, user_id, "SB-06: payload.user_id 应与 user_id 一致");
                assert_eq!(*admin_id, operator_id, "SB-06: admin_id 应为 operator_id");
            }
            other => panic!("SB-06: expected BanUser variant, got {:?}", other),
        }
    }

    // ── SB-07: unban 操作后 event.type="unban_user" ────────────────────────────
    #[tokio::test]
    async fn sb07_unban_publishes_unban_user_event() {
        let (svc, repo, publisher) = make_service_with_publisher();
        let operator_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id: user_id,
            phone: "+8613800000007".to_string(),
            nickname: "SB07User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: true, // 已封禁，可以解封
            created_at: Utc::now(),
        });

        let result = svc.ban_user(operator_id, user_id, ban_req("unban")).await;
        assert!(result.is_ok(), "SB-07: unban 操作应返回 Ok");

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "SB-07: 应发布恰好 1 次事件");
        // T-00105: 使用枚举变体匹配，不再依赖字符串类型字段
        assert!(
            matches!(&calls[0].1, crate::modules::event::publisher::AdminEvent::UnbanUser { .. }),
            "SB-07: event 应为 UnbanUser 变体"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // P0-2 验证测试 SB-09 ~ SB-12：ban_type / duration_hours 校验
    // ════════════════════════════════════════════════════════════════════════

    fn ban_req_full(
        action: &str,
        ban_type: Option<&str>,
        duration_hours: Option<u32>,
    ) -> AdminBanUserRequest {
        AdminBanUserRequest {
            action: action.to_string(),
            ban_type: ban_type.map(|s| s.to_string()),
            duration_hours,
            reason: Some("test".to_string()),
        }
    }

    fn seed_normal_user(repo: &Arc<FakeAdminUserRepository>) -> Uuid {
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613899999999".to_string(),
            nickname: "ValUser".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        });
        id
    }

    // SB-09: ban_type=temporary 但 duration_hours 缺失 → ValidationError
    #[tokio::test]
    async fn sb09_temporary_ban_without_duration_returns_validation_error() {
        let (svc, repo) = make_service();
        let id = seed_normal_user(&repo);
        let err = svc
            .ban_user(Uuid::new_v4(), id, ban_req_full("ban", Some("temporary"), None))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "SB-09: 临时封禁缺失 duration_hours 应返回 ValidationError，实际: {err:?}"
        );
    }

    // SB-10: ban_type=temporary + duration_hours=0 → ValidationError
    #[tokio::test]
    async fn sb10_temporary_ban_zero_duration_returns_validation_error() {
        let (svc, repo) = make_service();
        let id = seed_normal_user(&repo);
        let err = svc
            .ban_user(
                Uuid::new_v4(),
                id,
                ban_req_full("ban", Some("temporary"), Some(0)),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "SB-10: duration_hours=0 应返回 ValidationError"
        );
    }

    // SB-11: ban_type=permanent + duration_hours=Some → ValidationError
    #[tokio::test]
    async fn sb11_permanent_ban_with_duration_returns_validation_error() {
        let (svc, repo) = make_service();
        let id = seed_normal_user(&repo);
        let err = svc
            .ban_user(
                Uuid::new_v4(),
                id,
                ban_req_full("ban", Some("permanent"), Some(24)),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "SB-11: 永久封禁不应携带 duration_hours"
        );
    }

    // SB-12: 未知 ban_type → ValidationError（无静默 default 行为）
    #[tokio::test]
    async fn sb12_unknown_ban_type_returns_validation_error() {
        let (svc, repo) = make_service();
        let id = seed_normal_user(&repo);
        let err = svc
            .ban_user(
                Uuid::new_v4(),
                id,
                ban_req_full("ban", Some("forever"), None),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "SB-12: 非法 ban_type 应返回 ValidationError"
        );
    }

    // SB-13: ban_type=temporary + duration_hours>0 → 成功封禁
    #[tokio::test]
    async fn sb13_valid_temporary_ban_succeeds() {
        let (svc, repo) = make_service();
        let id = seed_normal_user(&repo);
        let res = svc
            .ban_user(
                Uuid::new_v4(),
                id,
                ban_req_full("ban", Some("temporary"), Some(24)),
            )
            .await
            .unwrap();
        assert_eq!(res.status, "banned");
    }

    // ── SB-14 (P3-18): ban / unban 事件 payload 字段对称 ────────────────────────
    #[tokio::test]
    async fn sb14_ban_and_unban_event_payloads_have_symmetric_keys() {
        let (svc, repo, publisher) = make_service_with_publisher();
        let operator_id = Uuid::new_v4();

        // ban 一个新用户
        let ban_user_id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id: ban_user_id,
            phone: "+8613811111111".to_string(),
            nickname: "BanU".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        });
        svc.ban_user(
            operator_id,
            ban_user_id,
            ban_req_full("ban", Some("temporary"), Some(24)),
        )
        .await
        .expect("ban should succeed");

        // unban 一个已封禁用户
        let unban_user_id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id: unban_user_id,
            phone: "+8613822222222".to_string(),
            nickname: "UnbanU".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: true,
            created_at: Utc::now(),
        });
        svc.ban_user(operator_id, unban_user_id, ban_req("unban"))
            .await
            .expect("unban should succeed");

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 2, "应有 ban + unban 两次发布");

        // T-00105: 严格类型匹配，schema `additionalProperties: false` 要求 payload 只有 user_id
        match &calls[0].1 {
            crate::modules::event::publisher::AdminEvent::BanUser { payload, .. } => {
                assert_eq!(
                    payload.user_id, ban_user_id,
                    "SB-14: ban 事件 payload.user_id 应与 ban_user_id 一致"
                );
            }
            other => panic!("SB-14: expected BanUser, got {:?}", other),
        }
        match &calls[1].1 {
            crate::modules::event::publisher::AdminEvent::UnbanUser { payload, admin_id, .. } => {
                assert_eq!(
                    payload.user_id, unban_user_id,
                    "SB-14: unban 事件 payload.user_id 应与 unban_user_id 一致"
                );
                assert_eq!(*admin_id, operator_id, "SB-14: admin_id 应为 operator_id");
            }
            other => panic!("SB-14: expected UnbanUser, got {:?}", other),
        }
    }
}
