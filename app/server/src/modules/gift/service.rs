//! Gift 服务层
//!
//! - `GiftServicePort` — 抽象接口，供 HTTP handler 注入
//! - `GiftService`     — 真实实现：查询 `GiftRepoPort` + in-memory 缓存（TTL 60s）
//! - `FakeGiftService` — 内存测试替身，供 `AppState::for_test()` 注入，返回预置 8 款礼物
//!
//! ## 缓存策略
//! 以 lang 为 key，在进程内存中缓存礼物列表。
//! 首次请求（miss）查询 DB 并缓存，TTL 60s 后自动过期。
//! 后台（T-10014）CRUD 写入后调用 `invalidate_all()` 清除所有语言缓存。

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::Mutex;

use crate::common::error::AppError;

use super::{
    dto::{GiftItem, GiftListData},
    repo::GiftRepoPort,
};

// ─── GiftServicePort trait ────────────────────────────────────────────────────

/// Gift 服务抽象接口，供 HTTP handler 注入
#[async_trait]
pub trait GiftServicePort: Send + Sync {
    /// 查询上架礼物列表，根据 `lang` 选择 `name_en`（en）或 `name_ar`（其他/默认）
    async fn list_active(&self, lang: &str) -> Result<GiftListData, AppError>;
}

// ─── 缓存内部结构 ─────────────────────────────────────────────────────────────

struct CacheEntry {
    data: GiftListData,
    expires_at: Instant,
}

// ─── GiftService ──────────────────────────────────────────────────────────────

/// Gift 服务实现：透过 `GiftRepoPort` 查询 DB，并在进程内存中缓存结果（TTL 60s）
pub struct GiftService {
    repo: Arc<dyn GiftRepoPort>,
    cache: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl GiftService {
    /// 使用指定 repo 创建 GiftService，TTL 默认 60s
    pub fn new(repo: Arc<dyn GiftRepoPort>) -> Self {
        Self {
            repo,
            cache: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(60),
        }
    }

    /// 使用指定 repo 和自定义 TTL 创建 GiftService（供测试使用）
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_with_ttl(repo: Arc<dyn GiftRepoPort>, ttl: Duration) -> Self {
        Self {
            repo,
            cache: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// 使用 PgPool 创建 GiftService（供 main.rs 调用）
    pub fn new_with_pool(pool: sqlx::PgPool) -> Self {
        use super::repo::PgGiftRepo;
        Self::new(Arc::new(PgGiftRepo::new(pool)))
    }

    /// 清除所有语言的缓存（T-10014 Admin CRUD 后调用）
    pub async fn invalidate_all(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    /// 将 GiftModel 列表转换为 GiftItem 列表（按 lang 选择名称）
    fn build_items(
        gifts: &[voice_room_shared::models::gift::GiftModel],
        lang: &str,
    ) -> Vec<GiftItem> {
        gifts
            .iter()
            .map(|g| GiftItem {
                id: g.id,
                code: g.code.clone(),
                name: if lang == "en" {
                    g.name_en.clone()
                } else {
                    g.name_ar.clone()
                },
                icon_url: g.icon_url.clone(),
                price: g.price,
                tier: g.tier,
                effect_level: g.effect_level,
                animation_url: g.animation_url.clone(),
                sort_order: g.sort_order,
            })
            .collect()
    }
}

#[async_trait]
impl GiftServicePort for GiftService {
    async fn list_active(&self, lang: &str) -> Result<GiftListData, AppError> {
        // 1. 尝试缓存命中
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.get(lang) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.data.clone());
                }
            }
        } // 释放锁

        // 2. 缓存 miss — 查询 DB
        let gifts = self.repo.list_active().await?;

        // 3. 按 lang 构建 item 列表
        let items = Self::build_items(&gifts, lang);
        let version = Utc::now().timestamp().to_string();
        let data = GiftListData { items, version };

        // 4. 写入缓存
        {
            let mut cache = self.cache.lock().await;
            cache.insert(
                lang.to_string(),
                CacheEntry {
                    data: data.clone(),
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }

        Ok(data)
    }
}

// ─── FakeGiftService（仅测试/test-utils）─────────────────────────────────────

/// FakeGiftService の内部礼物データ構造（型複雑度を下げる）
#[cfg(any(test, feature = "test-utils"))]
struct FakeGiftData {
    code: &'static str,
    name_en: &'static str,
    name_ar: &'static str,
    icon_url: &'static str,
    price: i64,
    tier: i16,
    effect_level: i16,
    sort_order: i32,
}

/// 内存测试替身，返回预置的 8 款 MVP 礼物数据。
/// 供 `AppState::for_test()` 注入，无需 DB 或缓存。
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeGiftService;

#[cfg(any(test, feature = "test-utils"))]
impl Default for FakeGiftService {
    fn default() -> Self {
        Self
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeGiftService {
    const PRESET_GIFTS: &'static [FakeGiftData] = &[
        FakeGiftData {
            code: "rose_01",
            name_en: "Rose",
            name_ar: "وردة",
            icon_url: "/assets/gifts/rose.png",
            price: 1,
            tier: 1,
            effect_level: 1,
            sort_order: 10,
        },
        FakeGiftData {
            code: "coffee_01",
            name_en: "Arabic Coffee",
            name_ar: "قهوة عربية",
            icon_url: "/assets/gifts/coffee.png",
            price: 10,
            tier: 2,
            effect_level: 2,
            sort_order: 20,
        },
        FakeGiftData {
            code: "kaaba_01",
            name_en: "Kaaba Candle",
            name_ar: "شمعة الكعبة",
            icon_url: "/assets/gifts/kaaba.png",
            price: 10,
            tier: 2,
            effect_level: 2,
            sort_order: 21,
        },
        FakeGiftData {
            code: "camel_01",
            name_en: "Desert Camel",
            name_ar: "جمل",
            icon_url: "/assets/gifts/camel.png",
            price: 66,
            tier: 3,
            effect_level: 3,
            sort_order: 30,
        },
        FakeGiftData {
            code: "falcon_01",
            name_en: "Golden Falcon",
            name_ar: "صقر ذهبي",
            icon_url: "/assets/gifts/falcon.png",
            price: 88,
            tier: 3,
            effect_level: 3,
            sort_order: 31,
        },
        FakeGiftData {
            code: "moon_786",
            name_en: "Bismillah Moon",
            name_ar: "هلال بسم الله",
            icon_url: "/assets/gifts/moon786.png",
            price: 786,
            tier: 4,
            effect_level: 4,
            sort_order: 40,
        },
        FakeGiftData {
            code: "castle_01",
            name_en: "Royal Castle",
            name_ar: "قصر ملكي",
            icon_url: "/assets/gifts/castle.png",
            price: 520,
            tier: 4,
            effect_level: 4,
            sort_order: 41,
        },
        FakeGiftData {
            code: "diamond_ring",
            name_en: "Diamond Ring",
            name_ar: "خاتم الماس",
            icon_url: "/assets/gifts/diamond.png",
            price: 1314,
            tier: 5,
            effect_level: 5,
            sort_order: 50,
        },
    ];
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl GiftServicePort for FakeGiftService {
    async fn list_active(&self, lang: &str) -> Result<GiftListData, AppError> {
        use uuid::Uuid;
        let items = Self::PRESET_GIFTS
            .iter()
            .map(|g| GiftItem {
                id: Uuid::new_v4(),
                code: g.code.to_string(),
                name: if lang == "en" {
                    g.name_en.to_string()
                } else {
                    g.name_ar.to_string()
                },
                icon_url: g.icon_url.to_string(),
                price: g.price,
                tier: g.tier,
                effect_level: g.effect_level,
                animation_url: None,
                sort_order: g.sort_order,
            })
            .collect();

        let version = Utc::now().timestamp().to_string();
        Ok(GiftListData { items, version })
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gift::repo::FakeGiftRepo;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_gift_model(
        code: &str,
        name_en: &str,
        name_ar: &str,
        tier: i16,
        sort_order: i32,
    ) -> voice_room_shared::models::gift::GiftModel {
        voice_room_shared::models::gift::GiftModel {
            id: Uuid::new_v4(),
            code: code.to_string(),
            name_en: name_en.to_string(),
            name_ar: name_ar.to_string(),
            icon_url: "/test.png".to_string(),
            price: 1,
            tier,
            effect_level: 1,
            animation_url: None,
            sort_order,
            is_active: true,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // GS01: FakeGiftService 默认返回 8 款礼物
    #[tokio::test]
    async fn gs01_fake_service_returns_8_gifts() {
        let svc = FakeGiftService::default();
        let data = svc.list_active("ar").await.unwrap();
        assert_eq!(
            data.items.len(),
            8,
            "GS01: FakeGiftService should return 8 gifts"
        );
    }

    // GS02: FakeGiftService lang=ar 返回阿拉伯语名称
    #[tokio::test]
    async fn gs02_fake_service_ar_returns_arabic_names() {
        let svc = FakeGiftService::default();
        let data = svc.list_active("ar").await.unwrap();
        let rose = data.items.iter().find(|i| i.code == "rose_01").unwrap();
        assert_eq!(rose.name, "وردة", "GS02: ar lang should return Arabic name");
    }

    // GS03: FakeGiftService lang=en 返回英文名称
    #[tokio::test]
    async fn gs03_fake_service_en_returns_english_names() {
        let svc = FakeGiftService::default();
        let data = svc.list_active("en").await.unwrap();
        let rose = data.items.iter().find(|i| i.code == "rose_01").unwrap();
        assert_eq!(
            rose.name, "Rose",
            "GS03: en lang should return English name"
        );
    }

    // GS04: FakeGiftService 返回的 version 是有效的时间戳字符串
    #[tokio::test]
    async fn gs04_fake_service_version_is_timestamp_string() {
        let svc = FakeGiftService::default();
        let data = svc.list_active("ar").await.unwrap();
        let version_ts: i64 = data
            .version
            .parse()
            .expect("GS04: version should be parseable as i64 timestamp");
        assert!(version_ts > 0, "GS04: version timestamp should be positive");
    }

    // GS05: FakeGiftService 礼物按 tier 排序（tier 1 在前）
    #[tokio::test]
    async fn gs05_fake_service_sorted_by_tier() {
        let svc = FakeGiftService::default();
        let data = svc.list_active("ar").await.unwrap();
        // rose_01 tier=1 应在第一位
        assert_eq!(
            data.items[0].code, "rose_01",
            "GS05: rose_01 (tier=1) should be first"
        );
        // diamond_ring tier=5 应在最后
        assert_eq!(
            data.items.last().unwrap().code,
            "diamond_ring",
            "GS05: diamond_ring (tier=5) should be last"
        );
    }

    // GS06: GiftService 第一次查询 repo，第二次命中缓存
    #[tokio::test]
    async fn gs06_gift_service_caches_on_second_call() {
        let gifts = vec![make_gift_model("rose_01", "Rose", "وردة", 1, 10)];
        let repo = Arc::new(FakeGiftRepo::new(gifts));
        let service = GiftService::new(repo.clone());

        let _ = service.list_active("ar").await.unwrap();
        assert_eq!(repo.call_count(), 1, "GS06: first call should query repo");

        let _ = service.list_active("ar").await.unwrap();
        assert_eq!(repo.call_count(), 1, "GS06: second call should use cache");
    }

    // GS07: GiftService 缓存过期后重新查询 repo
    #[tokio::test]
    async fn gs07_gift_service_cache_expires() {
        let gifts = vec![make_gift_model("rose_01", "Rose", "وردة", 1, 10)];
        let repo = Arc::new(FakeGiftRepo::new(gifts));
        // 设置极短 TTL（1ms）
        let service = GiftService::new_with_ttl(repo.clone(), Duration::from_millis(1));

        let _ = service.list_active("ar").await.unwrap();
        assert_eq!(repo.call_count(), 1, "GS07: first call queries repo");

        // 等待缓存过期
        tokio::time::sleep(Duration::from_millis(10)).await;

        let _ = service.list_active("ar").await.unwrap();
        assert_eq!(
            repo.call_count(),
            2,
            "GS07: after expiry, should query repo again"
        );
    }

    // GS08: GiftService lang=en 返回英文名称
    #[tokio::test]
    async fn gs08_gift_service_en_returns_english() {
        let gifts = vec![make_gift_model("rose_01", "Rose", "وردة", 1, 10)];
        let repo = Arc::new(FakeGiftRepo::new(gifts));
        let service = GiftService::new(repo);

        let data = service.list_active("en").await.unwrap();
        assert_eq!(
            data.items[0].name, "Rose",
            "GS08: en should return English name"
        );
    }

    // GS09: GiftService lang=ar 返回阿拉伯语名称
    #[tokio::test]
    async fn gs09_gift_service_ar_returns_arabic() {
        let gifts = vec![make_gift_model("rose_01", "Rose", "وردة", 1, 10)];
        let repo = Arc::new(FakeGiftRepo::new(gifts));
        let service = GiftService::new(repo);

        let data = service.list_active("ar").await.unwrap();
        assert_eq!(
            data.items[0].name, "وردة",
            "GS09: ar should return Arabic name"
        );
    }

    // GS10: GiftService invalidate_all 清除缓存后再次查询 repo
    #[tokio::test]
    async fn gs10_invalidate_all_clears_cache() {
        let gifts = vec![make_gift_model("rose_01", "Rose", "وردة", 1, 10)];
        let repo = Arc::new(FakeGiftRepo::new(gifts));
        let service = GiftService::new(repo.clone());

        // 首次查询并缓存
        let _ = service.list_active("ar").await.unwrap();
        assert_eq!(repo.call_count(), 1);

        // 清除缓存
        service.invalidate_all().await;

        // 再次查询 — 应重新查询 repo
        let _ = service.list_active("ar").await.unwrap();
        assert_eq!(
            repo.call_count(),
            2,
            "GS10: after invalidation, should query repo again"
        );
    }

    // GS11: FakeGiftService 满足 Arc<dyn GiftServicePort> 约束
    #[test]
    fn gs11_fake_service_is_gift_service_port() {
        let _: Arc<dyn GiftServicePort> = Arc::new(FakeGiftService::default());
    }

    // GS12: GiftService 满足 Arc<dyn GiftServicePort> 约束
    #[test]
    fn gs12_gift_service_is_gift_service_port() {
        let repo: Arc<dyn GiftRepoPort> = Arc::new(FakeGiftRepo::new(vec![]));
        let _: Arc<dyn GiftServicePort> = Arc::new(GiftService::new(repo));
    }
}
