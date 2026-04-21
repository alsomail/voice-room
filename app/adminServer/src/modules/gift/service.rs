use std::sync::Arc;

use uuid::Uuid;
use voice_room_shared::models::gift::GiftModel;

use crate::common::error::AppError;
use crate::modules::event::publisher::{AdminEvent, EventPublisher};

use super::dto::{CreateGiftRequest, UpdateGiftRequest};
use super::repo::{CreateGiftData, GiftRepository, UpdateGiftData};

// ─── GiftService ──────────────────────────────────────────────────────────────

/// 礼物管理业务层。
pub struct GiftService {
    repo: Arc<dyn GiftRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    /// 文件上传目录（用于 upload 端点存储图片/Lottie）
    pub upload_dir: String,
}

impl GiftService {
    pub fn new(
        repo: Arc<dyn GiftRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        upload_dir: String,
    ) -> Self {
        Self {
            repo,
            event_publisher,
            upload_dir,
        }
    }

    /// 列出礼物。
    ///
    /// - `include_inactive=false`（默认）：只返回 `is_active=true AND is_deleted=false`
    /// - `include_inactive=true`：返回所有非软删礼物（含未上架）
    pub async fn list_gifts(
        &self,
        include_inactive: bool,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<GiftModel>), AppError> {
        self.repo
            .list(include_inactive, page, size)
            .await
            .map_err(AppError::from)
    }

    /// 创建礼物。
    ///
    /// # 错误
    /// - `ValidationError` — 参数非法（price/tier/name 等校验失败）
    /// - `DuplicateCode` — code 已存在
    pub async fn create_gift(&self, req: CreateGiftRequest) -> Result<GiftModel, AppError> {
        validate_create_request(&req)?;

        // 检查 code 唯一性
        if self.repo.find_by_code(&req.code).await?.is_some() {
            return Err(AppError::DuplicateCode(req.code));
        }

        let data = CreateGiftData {
            code: req.code,
            name_en: req.name_en,
            name_ar: req.name_ar,
            icon_url: req.icon_url,
            price: req.price,
            tier: req.tier,
            effect_level: req.effect_level.unwrap_or(1),
            animation_url: req.animation_url,
            sort_order: req.sort_order.unwrap_or(0),
            is_active: req.is_active.unwrap_or(true),
        };

        let gift = self.repo.create(data).await?;
        self.publish_cache_invalidate().await;
        Ok(gift)
    }

    /// 更新礼物（含 is_active 切换）。
    ///
    /// # 错误
    /// - `NotFound` — 礼物不存在或已软删
    /// - `ValidationError` — 参数非法
    pub async fn update_gift(
        &self,
        id: Uuid,
        req: UpdateGiftRequest,
    ) -> Result<GiftModel, AppError> {
        validate_update_request(&req)?;

        // 确认礼物存在（未软删）
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("gift {id}")))?;

        let data = UpdateGiftData {
            name_en: req.name_en,
            name_ar: req.name_ar,
            icon_url: req.icon_url,
            price: req.price,
            tier: req.tier,
            effect_level: req.effect_level,
            animation_url: req.animation_url,
            sort_order: req.sort_order,
            is_active: req.is_active,
        };

        let gift = self.repo.update(id, data).await?;
        self.publish_cache_invalidate().await;
        Ok(gift)
    }

    /// 软删除礼物（is_deleted=true）。
    ///
    /// # 错误
    /// - `NotFound` — 礼物不存在或已软删
    pub async fn delete_gift(&self, id: Uuid) -> Result<(), AppError> {
        let found = self.repo.soft_delete(id).await?;
        if !found {
            return Err(AppError::NotFound(format!("gift {id}")));
        }
        self.publish_cache_invalidate().await;
        Ok(())
    }

    /// 通知 App Server 清除礼物缓存（fire-and-forget）。
    async fn publish_cache_invalidate(&self) {
        let event = AdminEvent {
            r#type: "gift_cache_invalidate".to_string(),
            payload: serde_json::json!({}),
            admin_id: "system".to_string(),
            ts: chrono::Utc::now().timestamp(),
        };
        if let Err(e) = self
            .event_publisher
            .publish("admin:events", event)
            .await
        {
            tracing::warn!(error = %e, "gift service: failed to publish cache invalidate event");
        }
    }
}

// ─── 参数校验辅助函数 ─────────────────────────────────────────────────────────

/// 校验创建礼物请求参数（满足 GC09/GC10 等验收用例）。
pub fn validate_create_request(req: &CreateGiftRequest) -> Result<(), AppError> {
    // code: 1-32 字符，只允许英文字母、数字、下划线
    let code_len = req.code.len();
    if code_len == 0 || code_len > 32 {
        return Err(AppError::ValidationError(
            "code 长度必须在 1~32 之间".to_string(),
        ));
    }
    if !req.code.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::ValidationError(
            "code 只能包含英文字母、数字和下划线".to_string(),
        ));
    }

    // price >= 1 (GC09)
    if req.price < 1 {
        return Err(AppError::ValidationError(
            "price 必须 >= 1".to_string(),
        ));
    }

    // tier ∈ [1,5] (GC10)
    if !(1..=5).contains(&req.tier) {
        return Err(AppError::ValidationError(
            "tier 必须在 1~5 之间".to_string(),
        ));
    }

    // effect_level ∈ [1,5]（若传入）
    if let Some(el) = req.effect_level {
        if !(1..=5).contains(&el) {
            return Err(AppError::ValidationError(
                "effect_level 必须在 1~5 之间".to_string(),
            ));
        }
    }

    // name_en: 1-64 字符
    let name_en_len = req.name_en.chars().count();
    if name_en_len == 0 || name_en_len > 64 {
        return Err(AppError::ValidationError(
            "name_en 长度必须在 1~64 之间".to_string(),
        ));
    }

    // name_ar: 1-64 字符
    let name_ar_len = req.name_ar.chars().count();
    if name_ar_len == 0 || name_ar_len > 64 {
        return Err(AppError::ValidationError(
            "name_ar 长度必须在 1~64 之间".to_string(),
        ));
    }

    Ok(())
}

/// 校验更新礼物请求参数。
pub fn validate_update_request(req: &UpdateGiftRequest) -> Result<(), AppError> {
    if let Some(price) = req.price {
        if price < 1 {
            return Err(AppError::ValidationError("price 必须 >= 1".to_string()));
        }
    }
    if let Some(tier) = req.tier {
        if !(1..=5).contains(&tier) {
            return Err(AppError::ValidationError(
                "tier 必须在 1~5 之间".to_string(),
            ));
        }
    }
    if let Some(el) = req.effect_level {
        if !(1..=5).contains(&el) {
            return Err(AppError::ValidationError(
                "effect_level 必须在 1~5 之间".to_string(),
            ));
        }
    }
    if let Some(ref name_en) = req.name_en {
        let len = name_en.chars().count();
        if len == 0 || len > 64 {
            return Err(AppError::ValidationError(
                "name_en 长度必须在 1~64 之间".to_string(),
            ));
        }
    }
    if let Some(ref name_ar) = req.name_ar {
        let len = name_ar.chars().count();
        if len == 0 || len > 64 {
            return Err(AppError::ValidationError(
                "name_ar 长度必须在 1~64 之间".to_string(),
            ));
        }
    }
    Ok(())
}

/// 校验文件上传参数（MIME 白名单 + 大小限制）。
///
/// 白名单：`image/png`, `image/jpeg`, `image/webp`, `application/json`（Lottie）
/// 大小限制：图片 ≤ 1MB，Lottie ≤ 2MB
pub fn validate_file_upload(content_type: &str, data_len: usize) -> Result<(), AppError> {
    const ALLOWED: &[&str] = &[
        "image/png",
        "image/jpeg",
        "image/webp",
        "application/json",
    ];
    if !ALLOWED.contains(&content_type) {
        return Err(AppError::ValidationError(format!(
            "不支持的文件类型: {content_type}，白名单: image/png, image/jpeg, image/webp, application/json"
        )));
    }
    let max_size: usize = if content_type == "application/json" {
        2 * 1024 * 1024 // 2MB
    } else {
        1024 * 1024 // 1MB
    };
    if data_len > max_size {
        return Err(AppError::ValidationError(format!(
            "文件大小 {data_len} 字节超过限制 {max_size} 字节"
        )));
    }
    Ok(())
}

// ─── 单元测试（Service 层）────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::event::publisher::NoopEventPublisher;
    use crate::modules::gift::repo::FakeGiftRepository;
    use chrono::Utc;
    use uuid::Uuid;
    use voice_room_shared::models::gift::GiftModel;

    fn make_service(
        fake_repo: Arc<FakeGiftRepository>,
    ) -> GiftService {
        GiftService::new(
            fake_repo,
            Arc::new(NoopEventPublisher::default()),
            "/tmp".to_string(),
        )
    }

    fn make_gift_model(code: &str, is_active: bool, is_deleted: bool) -> GiftModel {
        GiftModel {
            id: Uuid::new_v4(),
            code: code.to_string(),
            name_en: "Gift".to_string(),
            name_ar: "هدية".to_string(),
            icon_url: "/uploads/gifts/gift.png".to_string(),
            price: 10,
            tier: 1,
            effect_level: 1,
            animation_url: None,
            sort_order: 10,
            is_active,
            is_deleted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn valid_create_req(code: &str) -> CreateGiftRequest {
        CreateGiftRequest {
            code: code.to_string(),
            name_en: "Rose".to_string(),
            name_ar: "وردة".to_string(),
            icon_url: "/uploads/gifts/rose.png".to_string(),
            price: 1,
            tier: 1,
            effect_level: None,
            animation_url: None,
            sort_order: None,
            is_active: Some(true),
        }
    }

    // ── GS-01: 创建成功，缓存失效事件发布 ────────────────────────────────────
    #[tokio::test]
    async fn gs01_create_gift_success_publishes_cache_invalidate() {
        let repo = Arc::new(FakeGiftRepository::default());
        let publisher = Arc::new(NoopEventPublisher::default());
        let svc = GiftService::new(repo.clone(), publisher.clone(), "/tmp".to_string());

        let gift = svc.create_gift(valid_create_req("rose_01")).await.unwrap();

        assert_eq!(gift.code, "rose_01");
        assert!(gift.is_active);
        assert!(!gift.is_deleted);

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "GS-01: 应发布 1 次缓存失效事件");
        assert_eq!(calls[0].1.r#type, "gift_cache_invalidate");
    }

    // ── GS-02: duplicate code → DuplicateCode 错误 ───────────────────────────
    #[tokio::test]
    async fn gs02_create_duplicate_code_returns_error() {
        let repo = Arc::new(FakeGiftRepository::default());
        let svc = make_service(repo.clone());

        svc.create_gift(valid_create_req("rose_01")).await.unwrap();
        let err = svc.create_gift(valid_create_req("rose_01")).await.unwrap_err();
        assert!(matches!(err, AppError::DuplicateCode(_)), "GS-02: 重复 code 应返回 DuplicateCode");
    }

    // ── GS-03: price=0 → ValidationError ────────────────────────────────────
    #[tokio::test]
    async fn gs03_price_zero_returns_validation_error() {
        let repo = Arc::new(FakeGiftRepository::default());
        let svc = make_service(repo);
        let mut req = valid_create_req("test_01");
        req.price = 0;
        let err = svc.create_gift(req).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "GS-03: price=0 应返回 ValidationError");
    }

    // ── GS-04: tier=6 → ValidationError ─────────────────────────────────────
    #[tokio::test]
    async fn gs04_tier_out_of_range_returns_validation_error() {
        let repo = Arc::new(FakeGiftRepository::default());
        let svc = make_service(repo);
        let mut req = valid_create_req("test_02");
        req.tier = 6;
        let err = svc.create_gift(req).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "GS-04: tier=6 应返回 ValidationError");
    }

    // ── GS-05: 更新 is_active=false 切换下架 ─────────────────────────────────
    #[tokio::test]
    async fn gs05_update_is_active_switches_state() {
        let repo = Arc::new(FakeGiftRepository::default());
        let svc = make_service(repo.clone());

        let gift = svc.create_gift(valid_create_req("rose_04")).await.unwrap();
        let updated = svc
            .update_gift(
                gift.id,
                UpdateGiftRequest {
                    is_active: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(!updated.is_active, "GS-05: is_active 应被切换为 false");
    }

    // ── GS-06: 删除不存在的礼物 → NotFound ──────────────────────────────────
    #[tokio::test]
    async fn gs06_delete_nonexistent_returns_not_found() {
        let repo = Arc::new(FakeGiftRepository::default());
        let svc = make_service(repo);
        let err = svc.delete_gift(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "GS-06: 删除不存在礼物应返回 NotFound");
    }

    // ── GS-07: 软删后再次删除 → NotFound ─────────────────────────────────────
    #[tokio::test]
    async fn gs07_double_delete_returns_not_found() {
        let repo = Arc::new(FakeGiftRepository::default());
        let svc = make_service(repo.clone());
        let gift = svc.create_gift(valid_create_req("delete_me")).await.unwrap();

        svc.delete_gift(gift.id).await.unwrap();
        let err = svc.delete_gift(gift.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "GS-07: 重复软删应返回 NotFound");
    }

    // ── GS-08: list 默认不含 inactive ────────────────────────────────────────
    #[tokio::test]
    async fn gs08_list_default_excludes_inactive() {
        let repo = Arc::new(FakeGiftRepository::default());
        repo.seed(make_gift_model("active_01", true, false));
        repo.seed(make_gift_model("inactive_01", false, false));
        repo.seed(make_gift_model("deleted_01", true, true));

        let svc = make_service(repo);
        let (total, items) = svc.list_gifts(false, 1, 20).await.unwrap();
        assert_eq!(total, 1, "GS-08: 默认只返回 active");
        assert_eq!(items[0].code, "active_01");
    }

    // ── GS-09: 文件白名单校验 ─────────────────────────────────────────────────
    #[test]
    fn gs09_validate_file_upload_allowed_types() {
        assert!(validate_file_upload("image/png", 100).is_ok());
        assert!(validate_file_upload("image/jpeg", 100).is_ok());
        assert!(validate_file_upload("image/webp", 100).is_ok());
        assert!(validate_file_upload("application/json", 100).is_ok());
    }

    // ── GS-10: 非白名单 MIME → ValidationError（GC07）────────────────────────
    #[test]
    fn gs10_validate_file_upload_non_whitelist_returns_error() {
        let err = validate_file_upload("image/gif", 100).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "GS-10: gif 应返回 ValidationError");
        let err2 = validate_file_upload("application/pdf", 100).unwrap_err();
        assert!(matches!(err2, AppError::ValidationError(_)));
    }

    // ── GS-11: 文件大小超限 → ValidationError（GC08）────────────────────────
    #[test]
    fn gs11_validate_file_upload_size_limit() {
        // 图片 >1MB
        let err = validate_file_upload("image/png", 1024 * 1024 + 1).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "GS-11: 图片超限应返回 ValidationError");

        // Lottie ≤2MB 允许
        assert!(validate_file_upload("application/json", 2 * 1024 * 1024).is_ok());
        // Lottie >2MB 拒绝
        let err2 = validate_file_upload("application/json", 2 * 1024 * 1024 + 1).unwrap_err();
        assert!(matches!(err2, AppError::ValidationError(_)));
    }

    // ── GS-12: validate_create_request 边界验证 ──────────────────────────────
    #[test]
    fn gs12_validate_create_request_boundaries() {
        // 空 code
        let mut req = valid_create_req("");
        assert!(validate_create_request(&req).is_err(), "GS-12: 空 code 应失败");

        // code 含特殊字符
        req.code = "rose-01".to_string();
        assert!(validate_create_request(&req).is_err(), "GS-12: code 含连字符应失败");

        // 合法 code
        req.code = "rose_01".to_string();
        assert!(validate_create_request(&req).is_ok(), "GS-12: 合法 code 应通过");

        // 超长 code (33字符)
        req.code = "a".repeat(33);
        assert!(validate_create_request(&req).is_err(), "GS-12: 超长 code 应失败");
    }
}
