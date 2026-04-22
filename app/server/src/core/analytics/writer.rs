//! EventWriter — 统一事件写入服务（T-00022）
//!
//! ## 功能
//! - 批量写入 events 分区表（最多 100 条/次，超出加入 rejected_indices）
//! - properties 超 8KB 截断为 `{"_truncated": true}` 并 log warn
//! - JWT user_id 存在时覆盖请求体 user_id（不一致时 log warn）
//! - device_id 为空时返回 `AppError::ParameterMissing`
//!
//! ## 用法
//! ```rust,ignore
//! let writer = EventWriter::new(pool.clone());
//! let result = writer.persist(batch, jwt_user_id).await?;
//! println!("received={}, rejected={:?}", result.received, result.rejected_indices);
//! ```

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use crate::common::error::AppError;

/// 最大批量大小（超过此数量的事件将加入 rejected_indices）
pub const MAX_BATCH_SIZE: usize = 100;

/// properties JSON 序列化后最大字节数（8KB）
pub const MAX_PROPERTIES_SIZE: usize = 8 * 1024;

// ─── 输入数据结构 ──────────────────────────────────────────────────────────────

/// 来自 HTTP / WS 通道的统一事件输入结构
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventInput {
    /// 事件名称（必填）
    pub event_name: String,
    /// 设备 ID（必填，不可为空字符串）
    #[serde(default)]
    pub device_id: String,
    /// 用户 ID（可选，未登录时为 null）
    pub user_id: Option<Uuid>,
    /// 会话 ID
    pub session_id: Option<String>,
    /// 客户端时间戳（毫秒 epoch）
    pub client_ts: Option<i64>,
    /// 事件属性（JSON，超 8KB 将被截断）
    #[serde(default = "default_properties")]
    pub properties: serde_json::Value,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    pub locale: Option<String>,
    pub network_type: Option<String>,
}

fn default_properties() -> serde_json::Value {
    serde_json::json!({})
}

impl EventInput {
    /// 将毫秒时间戳转换为 DateTime<Utc>
    pub fn client_ts_utc(&self) -> Option<chrono::DateTime<Utc>> {
        self.client_ts.and_then(|ts_ms| {
            let secs = ts_ms / 1000;
            let nanos = ((ts_ms % 1000) * 1_000_000) as u32;
            chrono::DateTime::from_timestamp(secs, nanos)
        })
    }
}

// ─── 输出数据结构 ──────────────────────────────────────────────────────────────

/// `EventWriter::persist` 返回值
#[derive(Debug, Clone)]
pub struct PersistResult {
    /// 成功写入的事件数量
    pub received: usize,
    /// 被拒绝的事件索引（0-based，超出 100 条限制或其他校验失败）
    pub rejected_indices: Vec<usize>,
}

// ─── EventWriterPort trait ─────────────────────────────────────────────────────

/// 事件写入服务抽象（支持真实 DB 实现和测试 Fake）
#[async_trait]
pub trait EventWriterPort: Send + Sync {
    /// 批量写入事件
    ///
    /// # 参数
    /// - `batch`: 待写入的事件列表
    /// - `jwt_user_id`: JWT 解析出的 user_id（存在时覆盖请求体 user_id）
    ///
    /// # 错误
    /// - `AppError::ParameterMissing` — 任意事件的 device_id 为空
    /// - `AppError::DatabaseError` — 数据库写入失败
    async fn persist(
        &self,
        batch: Vec<EventInput>,
        jwt_user_id: Option<Uuid>,
    ) -> Result<PersistResult, AppError>;
}

// ─── 纯函数辅助（供单元测试直接调用）──────────────────────────────────────────

/// 若 properties JSON 序列化后超过 MAX_PROPERTIES_SIZE (8KB)，截断为 `{"_truncated": true}`
///
/// 返回 `(处理后的 properties, 是否被截断)`
pub fn truncate_properties(props: serde_json::Value) -> (serde_json::Value, bool) {
    let serialized = serde_json::to_string(&props).unwrap_or_default();
    if serialized.len() > MAX_PROPERTIES_SIZE {
        (serde_json::json!({"_truncated": true}), true)
    } else {
        (props, false)
    }
}

/// 解析最终 user_id：JWT 存在时覆盖请求体值（不一致时 log warn）
///
/// 返回最终使用的 user_id
pub fn resolve_user_id(
    request_user_id: Option<Uuid>,
    jwt_user_id: Option<Uuid>,
) -> Option<Uuid> {
    match (request_user_id, jwt_user_id) {
        (Some(req_uid), Some(jwt_uid)) => {
            if req_uid != jwt_uid {
                tracing::warn!(
                    jwt_user_id = %jwt_uid,
                    request_user_id = %req_uid,
                    "user_id mismatch: JWT overrides request body user_id"
                );
            }
            Some(jwt_uid)
        }
        (None, Some(jwt_uid)) => Some(jwt_uid),
        (Some(req_uid), None) => Some(req_uid),
        (None, None) => None,
    }
}

// ─── EventWriter（真实 DB 实现）────────────────────────────────────────────────

/// 真实 PostgreSQL 实现的事件写入服务
pub struct EventWriter {
    pool: PgPool,
}

impl EventWriter {
    /// 创建 EventWriter
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventWriterPort for EventWriter {
    async fn persist(
        &self,
        batch: Vec<EventInput>,
        jwt_user_id: Option<Uuid>,
    ) -> Result<PersistResult, AppError> {
        let mut rejected_indices = Vec::new();
        let mut valid_events: Vec<EventInput> = Vec::new();

        for (i, mut event) in batch.into_iter().enumerate() {
            // 超出批量限制
            if i >= MAX_BATCH_SIZE {
                rejected_indices.push(i);
                continue;
            }

            // 校验 device_id 必填
            if event.device_id.trim().is_empty() {
                return Err(AppError::ParameterMissing("device_id".to_string()));
            }

            // 应用 JWT user_id 覆盖
            event.user_id = resolve_user_id(event.user_id, jwt_user_id);

            // 截断超大 properties
            let (props, was_truncated) = truncate_properties(event.properties);
            if was_truncated {
                tracing::warn!(
                    event_name = %event.event_name,
                    device_id = %event.device_id,
                    "event properties exceed 8KB limit, truncating to {{_truncated: true}}"
                );
            }
            event.properties = props;

            valid_events.push(event);
        }

        let received = valid_events.len();

        if !valid_events.is_empty() {
            let now = Utc::now();

            let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
                "INSERT INTO events \
                 (user_id, device_id, event_name, properties, session_id, \
                  client_ts, server_ts, app_version, os_version, locale, network_type) ",
            );

            qb.push_values(valid_events.iter(), |mut b, e| {
                b.push_bind(e.user_id)
                    .push_bind(&e.device_id)
                    .push_bind(&e.event_name)
                    .push_bind(&e.properties)
                    .push_bind(&e.session_id)
                    .push_bind(e.client_ts_utc())
                    .push_bind(now)
                    .push_bind(&e.app_version)
                    .push_bind(&e.os_version)
                    .push_bind(&e.locale)
                    .push_bind(&e.network_type);
            });

            qb.build().execute(&self.pool).await?;
        }

        Ok(PersistResult {
            received,
            rejected_indices,
        })
    }
}

// ─── FakeEventWriter（测试替身）────────────────────────────────────────────────

/// 内存测试替身，供 AppState::for_test() 注入
#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakeEventWriter {
    /// 存储写入的 (event, jwt_user_id) 对，供测试断言
    pub stored: std::sync::Mutex<Vec<(EventInput, Option<Uuid>)>>,
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl EventWriterPort for FakeEventWriter {
    async fn persist(
        &self,
        batch: Vec<EventInput>,
        jwt_user_id: Option<Uuid>,
    ) -> Result<PersistResult, AppError> {
        let mut rejected_indices = Vec::new();
        let mut valid_events = Vec::new();

        for (i, mut event) in batch.into_iter().enumerate() {
            if i >= MAX_BATCH_SIZE {
                rejected_indices.push(i);
                continue;
            }

            // 同样校验 device_id
            if event.device_id.trim().is_empty() {
                return Err(AppError::ParameterMissing("device_id".to_string()));
            }

            // 应用 JWT 覆盖
            event.user_id = resolve_user_id(event.user_id, jwt_user_id);

            // 截断处理
            let (props, _) = truncate_properties(event.properties);
            event.properties = props;

            valid_events.push(event);
        }

        let received = valid_events.len();
        let mut stored = self.stored.lock().unwrap();
        for e in valid_events {
            stored.push((e, jwt_user_id));
        }

        Ok(PersistResult {
            received,
            rejected_indices,
        })
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // W-01: truncate_properties — 小于 8KB 不截断
    #[test]
    fn w01_small_properties_not_truncated() {
        let props = serde_json::json!({"key": "value", "num": 42});
        let (result, was_truncated) = truncate_properties(props.clone());
        assert!(!was_truncated);
        assert_eq!(result, props);
    }

    // W-02: truncate_properties — 超过 8KB 截断
    #[test]
    fn w02_large_properties_truncated() {
        let large = "x".repeat(9000);
        let props = serde_json::json!({"data": large});
        let (result, was_truncated) = truncate_properties(props);
        assert!(was_truncated);
        assert_eq!(result, serde_json::json!({"_truncated": true}));
    }

    // W-03: truncate_properties — 恰好 8192 字节不截断（边界值）
    #[test]
    fn w03_exactly_8kb_boundary() {
        // {"data":"..."} 的序列化长度 = 11 + content_len
        // ("{"  = 1, "\"data\":" = 8, "\"" = 1, content, "\"}" = 2) = 12 + content_len
        // 实际: '{"data":"' = 9 chars, then content, then '"}' = 2 chars → 11 + content_len
        // 为了得到精确 8192 字节: content_len = 8192 - 11 = 8181
        let content = "x".repeat(8181);
        let props = serde_json::json!({"data": content});
        let serialized = serde_json::to_string(&props).unwrap();
        assert_eq!(serialized.len(), 8192, "test setup: should be exactly 8192 bytes");
        let (_, was_truncated) = truncate_properties(props);
        assert!(!was_truncated, "exactly 8KB should NOT be truncated");
    }

    // W-04: truncate_properties — 8193 字节被截断（边界值 +1）
    #[test]
    fn w04_one_byte_over_8kb_truncated() {
        let content = "x".repeat(8182);
        let props = serde_json::json!({"data": content});
        let serialized = serde_json::to_string(&props).unwrap();
        assert_eq!(serialized.len(), 8193, "test setup: should be 8193 bytes");
        let (_, was_truncated) = truncate_properties(props);
        assert!(was_truncated, "8193 bytes should be truncated");
    }

    // W-05: resolve_user_id — JWT 覆盖请求体
    #[test]
    fn w05_resolve_user_id_jwt_overrides() {
        let jwt = Uuid::new_v4();
        let req = Uuid::new_v4();
        let result = resolve_user_id(Some(req), Some(jwt));
        assert_eq!(result, Some(jwt));
    }

    // W-06: resolve_user_id — 无 JWT 使用请求体
    #[test]
    fn w06_resolve_user_id_no_jwt_uses_request() {
        let req = Uuid::new_v4();
        let result = resolve_user_id(Some(req), None);
        assert_eq!(result, Some(req));
    }

    // W-07: resolve_user_id — 两者都为 None
    #[test]
    fn w07_resolve_user_id_both_none() {
        let result = resolve_user_id(None, None);
        assert_eq!(result, None);
    }

    // W-08: resolve_user_id — 无请求 user_id，有 JWT
    #[test]
    fn w08_resolve_user_id_only_jwt() {
        let jwt = Uuid::new_v4();
        let result = resolve_user_id(None, Some(jwt));
        assert_eq!(result, Some(jwt));
    }

    // W-09: MAX_BATCH_SIZE 常量为 100
    #[test]
    fn w09_max_batch_size_is_100() {
        assert_eq!(MAX_BATCH_SIZE, 100);
    }

    // W-10: MAX_PROPERTIES_SIZE 常量为 8192
    #[test]
    fn w10_max_properties_size_is_8192() {
        assert_eq!(MAX_PROPERTIES_SIZE, 8 * 1024);
    }

    // W-11: EventInput::client_ts_utc — 毫秒转换正确
    #[test]
    fn w11_client_ts_utc_conversion() {
        let event = EventInput {
            event_name: "test".to_string(),
            device_id: "device-001".to_string(),
            user_id: None,
            session_id: None,
            client_ts: Some(1720000000000), // 2024-07-03
            properties: serde_json::json!({}),
            app_version: None,
            os_version: None,
            locale: None,
            network_type: None,
        };
        let ts = event.client_ts_utc();
        assert!(ts.is_some(), "should parse valid timestamp");
        assert_eq!(ts.unwrap().timestamp(), 1720000000);
    }

    // W-12: EventInput::client_ts_utc — None 输入返回 None
    #[test]
    fn w12_client_ts_utc_none() {
        let event = EventInput {
            event_name: "test".to_string(),
            device_id: "d".to_string(),
            user_id: None,
            session_id: None,
            client_ts: None,
            properties: serde_json::json!({}),
            app_version: None,
            os_version: None,
            locale: None,
            network_type: None,
        };
        assert!(event.client_ts_utc().is_none());
    }

    // W-13: FakeEventWriter — 测试替身正常写入并可查询
    #[cfg(feature = "test-utils")]
    #[tokio::test]
    async fn w13_fake_event_writer_stores_events() {
        let fake = FakeEventWriter::default();
        let batch = vec![EventInput {
            event_name: "test".to_string(),
            device_id: "device-001".to_string(),
            user_id: None,
            session_id: None,
            client_ts: None,
            properties: serde_json::json!({}),
            app_version: None,
            os_version: None,
            locale: None,
            network_type: None,
        }];
        let result = fake.persist(batch, None).await.unwrap();
        assert_eq!(result.received, 1);
        assert!(result.rejected_indices.is_empty());
        let stored = fake.stored.lock().unwrap();
        assert_eq!(stored.len(), 1);
    }
}
