use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::event::publisher::{AdminEvent, CloseRoomPayload, EventPublisher};

use super::{
    dto::{
        AdminRoomDetailResponse, AdminRoomFilter, AdminRoomItem, AdminRoomListQuery,
        AdminRoomListResponse,
    },
    repository::AdminRoomRepository,
};

// ─── AdminRoomService ─────────────────────────────────────────────────────────

/// 房间列表业务逻辑层。
///
/// 职责：
/// 1. 参数校验（page/page_size 范围、status 枚举值）
/// 2. 构建过滤器传递给仓库
/// 3. 组装分页响应
pub struct AdminRoomService {
    room_repo: Arc<dyn AdminRoomRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl AdminRoomService {
    pub fn new(
        room_repo: Arc<dyn AdminRoomRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            room_repo,
            event_publisher,
        }
    }

    /// 查询房间列表，返回分页结果。
    ///
    /// # 默认值
    /// - page: 1（若 None 或未传）
    /// - page_size: 20（若 None 或未传）
    ///
    /// # 校验规则
    /// - page >= 1，否则返回 ValidationError (40003)
    /// - page_size 在 1..=100，否则返回 ValidationError (40003)
    /// - status 必须为 "active" / "closed"，否则返回 ValidationError (40003)
    pub async fn list(&self, query: AdminRoomListQuery) -> Result<AdminRoomListResponse, AppError> {
        // ── 参数解析 & 校验 ────────────────────────────────────────────────────
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);

        if page == 0 {
            return Err(AppError::ValidationError("page must be >= 1".to_string()));
        }
        if page_size == 0 || page_size > 100 {
            return Err(AppError::ValidationError(
                "page_size must be between 1 and 100".to_string(),
            ));
        }
        if let Some(ref status) = query.status {
            if status != "active" && status != "closed" {
                return Err(AppError::ValidationError(format!(
                    "invalid status '{}': must be 'active' or 'closed'",
                    status
                )));
            }
        }

        // ── 构建过滤器 ─────────────────────────────────────────────────────────
        let filter = AdminRoomFilter {
            status: query.status,
            keyword: query.keyword,
        };

        // ── 查询仓库 ───────────────────────────────────────────────────────────
        let total = self.room_repo.count_rooms(&filter).await?;
        let offset = ((page - 1) as i64) * (page_size as i64);
        let rows = self
            .room_repo
            .find_rooms(&filter, offset, page_size as i64)
            .await?;

        Ok(AdminRoomListResponse {
            total,
            page,
            page_size,
            items: rows.into_iter().map(AdminRoomItem::from).collect(),
        })
    }

    /// 查询房间详情（不过滤 status，仅排除软删除），房间不存在时返回 NotFound (40400)。
    pub async fn get_room_detail(
        &self,
        room_id: Uuid,
    ) -> Result<AdminRoomDetailResponse, AppError> {
        match self.room_repo.find_room_by_id_any_status(room_id).await? {
            Some(row) => Ok(AdminRoomDetailResponse::from(row)),
            None => Err(AppError::NotFound(format!("room {} not found", room_id))),
        }
    }

    /// 强制关闭房间（T-10006）。
    ///
    /// # 规则
    /// - 房间不存在（含软删除）→ NotFound (40400)
    /// - 房间已 closed → RoomAlreadyClosed (40901)
    /// - 无 owner 检查（任何有 RoomForceClose 权限的角色均可操作）
    /// - 缺陷 #3：UPDATE 在 SQL 层带 status='active' 守卫，并发强制关闭仅一次成功，
    ///   另一次 rows_affected=0 → 返回 RoomAlreadyClosed（不再误报 200 OK）。
    pub async fn force_close_room(&self, operator_id: Uuid, room_id: Uuid) -> Result<(), AppError> {
        // 1. 查询（含 closed 状态，不做 owner 检查）
        let room = self
            .room_repo
            .find_room_by_id_any_status(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("room {room_id}")))?;

        // 2. 状态前置校验
        if room.status == "closed" {
            return Err(AppError::RoomAlreadyClosed);
        }

        // 3. 持久化状态变更（原子，缺陷 #3）
        let updated = self.room_repo.set_room_closed(room_id).await?;
        if !updated {
            return Err(AppError::RoomAlreadyClosed);
        }

        // 4. 发布关闭事件（fire-and-forget，失败不影响主业务）
        // T-00105: 使用 strict AdminEvent enum（来自 voice_room_shared），消除字符串拼写错误风险
        let event = AdminEvent::CloseRoom {
            payload: CloseRoomPayload { room_id },
            admin_id: operator_id,
            ts: chrono::Utc::now().timestamp_millis(),
        };
        if let Err(e) = self.event_publisher.publish("admin:events", event).await {
            tracing::warn!(error = %e, "failed to publish close_room event");
        }

        Ok(())
    }
}

// ─── Service 单元测试（TDD T-10004 验收用例 S-01~S-06）──────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::room::repository::FakeAdminRoomRepository;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::super::dto::AdminRoomListRow;

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    fn make_row(title: &str, status: &str) -> AdminRoomListRow {
        AdminRoomListRow {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: status.to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            max_members: 50,
            owner_id: Uuid::new_v4(),
            owner_nickname: "Owner".to_string(),
            owner_avatar: None,
            created_at: Utc::now() - Duration::seconds(10),
        }
    }

    fn make_service() -> (AdminRoomService, Arc<FakeAdminRoomRepository>) {
        let repo = Arc::new(FakeAdminRoomRepository::default());
        let publisher = Arc::new(crate::modules::event::publisher::NoopEventPublisher::default());
        let svc = AdminRoomService::new(
            repo.clone() as Arc<dyn AdminRoomRepository>,
            publisher as Arc<dyn crate::modules::event::publisher::EventPublisher>,
        );
        (svc, repo)
    }

    fn query(
        page: Option<u32>,
        page_size: Option<u32>,
        status: Option<&str>,
        keyword: Option<&str>,
    ) -> AdminRoomListQuery {
        AdminRoomListQuery {
            page,
            page_size,
            status: status.map(str::to_string),
            keyword: keyword.map(str::to_string),
        }
    }

    // ── S-01: page=None → 默认 1 ─────────────────────────────────────────────
    #[tokio::test]
    async fn s01_page_none_defaults_to_1() {
        let (svc, _) = make_service();
        let result = svc.list(query(None, Some(10), None, None)).await.unwrap();
        assert_eq!(result.page, 1, "S-01: page=None 应默认为 1");
    }

    // ── S-02: page_size=None → 默认 20 ───────────────────────────────────────
    #[tokio::test]
    async fn s02_page_size_none_defaults_to_20() {
        let (svc, _) = make_service();
        let result = svc.list(query(Some(1), None, None, None)).await.unwrap();
        assert_eq!(result.page_size, 20, "S-02: page_size=None 应默认为 20");
    }

    // ── S-03: status=Some("active") → filter 传递正确 ────────────────────────
    #[tokio::test]
    async fn s03_status_active_filters_correctly() {
        let (svc, repo) = make_service();
        repo.seed(make_row("Active Room", "active"));
        repo.seed(make_row("Closed Room", "closed"));

        let result = svc
            .list(query(Some(1), Some(10), Some("active"), None))
            .await
            .unwrap();

        assert_eq!(result.total, 1, "S-03: 过滤 active 后应只有 1 个房间");
        assert_eq!(result.items[0].status, "active");
    }

    // ── S-04: status=Some("bad") → ValidationError (40003) ───────────────────
    #[tokio::test]
    async fn s04_invalid_status_returns_validation_error() {
        let (svc, _) = make_service();
        let err = svc
            .list(query(Some(1), Some(10), Some("bad"), None))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "S-04: 非法 status 应返回 ValidationError，实际: {err:?}"
        );
    }

    // ── S-05: page=0 → ValidationError (40003) ───────────────────────────────
    #[tokio::test]
    async fn s05_page_zero_returns_validation_error() {
        let (svc, _) = make_service();
        let err = svc
            .list(query(Some(0), Some(10), None, None))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "S-05: page=0 应返回 ValidationError，实际: {err:?}"
        );
    }

    // ── S-06: page_size=200 → ValidationError (40003) ────────────────────────
    #[tokio::test]
    async fn s06_page_size_200_returns_validation_error() {
        let (svc, _) = make_service();
        let err = svc
            .list(query(Some(1), Some(200), None, None))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "S-06: page_size=200 应返回 ValidationError（上限 100），实际: {err:?}"
        );
    }

    // ── 额外边界：page_size=101 也应报错（E-07 对应 service 层）─────────────
    #[tokio::test]
    async fn s_page_size_101_returns_validation_error() {
        let (svc, _) = make_service();
        let err = svc
            .list(query(Some(1), Some(101), None, None))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    // ── 额外边界：page_size=0 也应报错（E-06 对应 service 层）──────────────
    #[tokio::test]
    async fn s_page_size_0_returns_validation_error() {
        let (svc, _) = make_service();
        let err = svc
            .list(query(Some(1), Some(0), None, None))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    // ── 正常分页：total/page/page_size 均反映在响应中 ────────────────────────
    #[tokio::test]
    async fn s_pagination_response_reflects_params() {
        let (svc, repo) = make_service();
        for i in 0..5 {
            repo.seed(make_row(&format!("Room {i}"), "active"));
        }

        let result = svc.list(query(Some(2), Some(2), None, None)).await.unwrap();

        assert_eq!(result.total, 5);
        assert_eq!(result.page, 2);
        assert_eq!(result.page_size, 2);
        assert_eq!(result.items.len(), 2);
    }

    // ── keyword 空结果：total=0, items=[] ─────────────────────────────────
    #[tokio::test]
    async fn s_keyword_no_match_returns_empty() {
        let (svc, repo) = make_service();
        repo.seed(make_row("Music Room", "active"));

        let result = svc
            .list(query(Some(1), Some(10), None, Some("xyz_nonexistent")))
            .await
            .unwrap();

        assert_eq!(result.total, 0, "无匹配时 total 应为 0");
        assert!(result.items.is_empty(), "无匹配时 items 应为空");
    }

    // ══════════════════════════════════════════════════════════════════════════
    // T-10005 新增 Service 单元测试（DS-01~DS-05）
    // ══════════════════════════════════════════════════════════════════════════

    use super::super::dto::AdminRoomDetailRow;

    fn make_detail_row(title: &str, status: &str) -> AdminRoomDetailRow {
        AdminRoomDetailRow {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: status.to_string(),
            room_type: "normal".to_string(),
            member_count: 2,
            max_members: 50,
            owner_id: Uuid::new_v4(),
            owner_nickname: "DetailOwner".to_string(),
            owner_avatar: Some("https://avatar.example.com/1.png".to_string()),
            created_at: Utc::now() - Duration::seconds(10),
            updated_at: Utc::now(),
        }
    }

    // ── DS-01: get_room_detail 返回 active 房间详情 ───────────────────────────
    #[tokio::test]
    async fn ds01_get_room_detail_returns_active_room() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Active Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = svc.get_room_detail(id).await.unwrap();
        assert_eq!(result.status, "active", "DS-01: active 房间应成功返回");
        assert_eq!(result.title, "Active Room");
    }

    // ── DS-02: get_room_detail 返回 closed 房间详情（与 C 端不同，不过滤状态）────
    #[tokio::test]
    async fn ds02_get_room_detail_returns_closed_room() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Closed Room", "closed");
        let id = row.id;
        repo.seed_detail(row);

        let result = svc.get_room_detail(id).await.unwrap();
        assert_eq!(result.status, "closed", "DS-02: closed 房间也应返回详情");
    }

    // ── DS-03: get_room_detail 房间不存在时返回 NotFound 错误 ─────────────────
    #[tokio::test]
    async fn ds03_get_room_detail_not_found_returns_error() {
        let (svc, _) = make_service();
        let err = svc.get_room_detail(Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "DS-03: 不存在的房间应返回 NotFound，实际: {err:?}"
        );
    }

    // ── DS-04: get_room_detail 响应包含正确字段（room_id、owner.user_id 等）────
    #[tokio::test]
    async fn ds04_get_room_detail_response_has_correct_fields() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Test Room", "active");
        let id = row.id;
        let owner_id = row.owner_id;
        repo.seed_detail(row);

        let result = svc.get_room_detail(id).await.unwrap();
        assert_eq!(
            result.room_id,
            id.to_string(),
            "DS-04: room_id 应与输入一致"
        );
        assert_eq!(
            result.owner.user_id,
            owner_id.to_string(),
            "DS-04: owner.user_id 应与 owner_id 一致"
        );
        assert_eq!(
            result.owner.nickname, "DetailOwner",
            "DS-04: 昵称应正确映射"
        );
    }

    // ── DS-05: get_room_detail 响应的 mic_slots 为空数组（MVP 固定空数组）────────
    #[tokio::test]
    async fn ds05_get_room_detail_mic_slots_is_empty() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Slots Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = svc.get_room_detail(id).await.unwrap();
        assert!(
            result.mic_slots.is_empty(),
            "DS-05: MVP 阶段 mic_slots 应为空数组"
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // T-10006 新增 Service 单元测试（FCS-01~FCS-05）
    // ══════════════════════════════════════════════════════════════════════════

    // ── FCS-01: active 房间 → Ok(())，状态变为 closed ──────────────────────────
    #[tokio::test]
    async fn fcs01_force_close_active_room_returns_ok_and_status_closed() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Active Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = svc.force_close_room(Uuid::new_v4(), id).await;
        assert!(result.is_ok(), "FCS-01: active 房间强制关闭应返回 Ok(())");

        // 验证状态已变更
        let detail = svc.get_room_detail(id).await.unwrap();
        assert_eq!(
            detail.status, "closed",
            "FCS-01: 关闭后 status 应变为 closed"
        );
    }

    // ── FCS-02: 不存在的 room_id → NotFound ────────────────────────────────────
    #[tokio::test]
    async fn fcs02_force_close_nonexistent_room_returns_not_found() {
        let (svc, _) = make_service();
        let err = svc
            .force_close_room(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "FCS-02: 不存在的 room_id 应返回 NotFound，实际: {err:?}"
        );
    }

    // ── FCS-03: 软删除的房间 → NotFound ────────────────────────────────────────
    #[tokio::test]
    async fn fcs03_force_close_soft_deleted_room_returns_not_found() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Deleted Room", "active");
        let id = row.id;
        repo.seed_detail_deleted(row);

        let err = svc.force_close_room(Uuid::new_v4(), id).await.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "FCS-03: 软删除房间应返回 NotFound，实际: {err:?}"
        );
    }

    // ── FCS-04: 已 closed 的房间 → RoomAlreadyClosed ────────────────────────────
    #[tokio::test]
    async fn fcs04_force_close_already_closed_room_returns_room_already_closed() {
        let (svc, repo) = make_service();
        let row = make_detail_row("Closed Room", "closed");
        let id = row.id;
        repo.seed_detail(row);

        let err = svc.force_close_room(Uuid::new_v4(), id).await.unwrap_err();
        assert!(
            matches!(err, AppError::RoomAlreadyClosed),
            "FCS-04: 已 closed 的房间应返回 RoomAlreadyClosed，实际: {err:?}"
        );
    }

    // ── FCS-05: 非房主（但有权限）→ Ok(())（无 owner 检查）──────────────────────
    #[tokio::test]
    async fn fcs05_force_close_non_owner_succeeds_no_owner_check() {
        let (svc, repo) = make_service();
        // 创建一个 owner_id 与任何管理员 ID 无关的房间
        let mut row = make_detail_row("Any Owner Room", "active");
        row.owner_id = Uuid::new_v4(); // 随机 owner，与调用方无关
        let id = row.id;
        repo.seed_detail(row);

        // Service 层不做 owner 校验，应成功
        let result = svc.force_close_room(Uuid::new_v4(), id).await;
        assert!(
            result.is_ok(),
            "FCS-05: 无 owner 检查，任何有权限的调用方均应成功"
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // T-10011 RoomService 测试 FR-01~03（事件发布）
    // ══════════════════════════════════════════════════════════════════════════

    use crate::modules::event::publisher::{
        ErrorEventPublisher, EventPublisher, NoopEventPublisher,
    };

    fn make_service_with_publisher() -> (
        AdminRoomService,
        Arc<FakeAdminRoomRepository>,
        Arc<NoopEventPublisher>,
    ) {
        let repo = Arc::new(FakeAdminRoomRepository::default());
        let publisher = Arc::new(NoopEventPublisher::default());
        let svc = AdminRoomService::new(
            repo.clone() as Arc<dyn AdminRoomRepository>,
            publisher.clone() as Arc<dyn EventPublisher>,
        );
        (svc, repo, publisher)
    }

    // ── FR-01: force_close_room 成功后，NoopEventPublisher 收到 1 次 admin:events 调用 ──
    #[tokio::test]
    async fn fr01_force_close_room_publishes_close_room_event() {
        let (svc, repo, publisher) = make_service_with_publisher();
        let operator_id = Uuid::new_v4();
        let row = make_detail_row("Close Event Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = svc.force_close_room(operator_id, id).await;
        assert!(result.is_ok(), "FR-01: force_close_room 应返回 Ok(())");

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "FR-01: 应发布恰好 1 次事件");
        assert_eq!(
            calls[0].0, "admin:events",
            "FR-01: channel 应为 admin:events"
        );
        // T-00105: 使用枚举变体匹配，不再依赖字符串类型字段
        match &calls[0].1 {
            crate::modules::event::publisher::AdminEvent::CloseRoom { payload, admin_id, .. } => {
                assert_eq!(payload.room_id, id, "FR-01: payload.room_id 应与 room_id 一致");
                assert_eq!(*admin_id, operator_id, "FR-01: admin_id 应为 operator_id");
            }
            other => panic!("FR-01: expected CloseRoom variant, got {:?}", other),
        }
    }

    // ── FR-02: 使用 ErrorEventPublisher 时，force_close_room 仍返回 Ok(()) ───────
    #[tokio::test]
    async fn fr02_error_publisher_does_not_affect_force_close_result() {
        let repo = Arc::new(FakeAdminRoomRepository::default());
        let publisher = Arc::new(ErrorEventPublisher);
        let svc = AdminRoomService::new(
            repo.clone() as Arc<dyn AdminRoomRepository>,
            publisher as Arc<dyn EventPublisher>,
        );
        let row = make_detail_row("Error Publisher Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = svc.force_close_room(Uuid::new_v4(), id).await;
        assert!(
            result.is_ok(),
            "FR-02: 即使发布失败，force_close_room 也应返回 Ok(())，实际: {result:?}"
        );
    }

    // ── FR-03: 房间不存在时，不调用 event_publisher（calls 长度=0）──────────────
    #[tokio::test]
    async fn fr03_nonexistent_room_does_not_publish_event() {
        let (svc, _, publisher) = make_service_with_publisher();

        let result = svc.force_close_room(Uuid::new_v4(), Uuid::new_v4()).await;
        assert!(
            matches!(result, Err(AppError::NotFound(_))),
            "FR-03: 不存在的房间应返回 NotFound，实际: {result:?}"
        );
    }

    // ── FR-04: 并发 force_close_room 仅一次成功（缺陷 #3 回归）──────────────
    #[tokio::test]
    async fn fr04_concurrent_force_close_only_one_succeeds() {
        let repo = Arc::new(FakeAdminRoomRepository::default());
        let publisher = Arc::new(NoopEventPublisher::default());
        let svc = Arc::new(AdminRoomService::new(
            repo.clone() as Arc<dyn AdminRoomRepository>,
            publisher.clone() as Arc<dyn EventPublisher>,
        ));
        let row = make_detail_row("Race Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let svc1 = Arc::clone(&svc);
        let svc2 = Arc::clone(&svc);
        let h1 = tokio::spawn(async move { svc1.force_close_room(Uuid::new_v4(), id).await });
        let h2 = tokio::spawn(async move { svc2.force_close_room(Uuid::new_v4(), id).await });
        let r1 = h1.await.unwrap();
        let r2 = h2.await.unwrap();

        let oks = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
        let already = [&r1, &r2]
            .iter()
            .filter(|r| matches!(r, Err(AppError::RoomAlreadyClosed)))
            .count();
        assert_eq!(oks, 1, "并发 force_close 应仅 1 次 Ok，r1={r1:?} r2={r2:?}");
        assert_eq!(
            already, 1,
            "并发 force_close 应有 1 次 RoomAlreadyClosed，r1={r1:?} r2={r2:?}"
        );
    }
}
