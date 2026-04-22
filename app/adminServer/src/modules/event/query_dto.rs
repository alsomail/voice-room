use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 请求 DTO ─────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/users/:id/events 查询参数。
#[derive(Debug, Deserialize)]
pub struct EventQueryParams {
    /// 事件名多值过滤（逗号分隔，例：`gift_send,room_join`）
    pub event_name: Option<String>,
    /// 起始时间 ISO8601（默认 24h 前）
    pub from: Option<String>,
    /// 结束时间 ISO8601（默认当前时间）
    pub to: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

// ─── 仓库层内部过滤器 ─────────────────────────────────────────────────────────

/// 传给 EventQueryRepository 的已解析过滤条件。
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// None = 不按名称过滤；Some([]) = 全部过滤（cs 把请求的 admin_* 都移除后为空）
    pub event_names: Option<Vec<String>>,
    /// true = 排除所有 `admin_` 前缀事件（cs / operator 角色）
    pub exclude_admin_prefix: bool,
}

// ─── 仓库行 ──────────────────────────────────────────────────────────────────

/// events 表查询单行结果（由 repository 返回，供 service 层消费）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventRow {
    pub id: Uuid,
    pub event_name: String,
    pub server_ts: DateTime<Utc>,
    pub client_ts: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub device_id: String,
    pub properties: serde_json::Value,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    pub locale: Option<String>,
    pub network_type: Option<String>,
}

// ─── 响应 DTO ─────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/users/:id/events 响应中每条 event 的 JSON 结构。
#[derive(Debug, Clone, Serialize)]
pub struct EventItem {
    pub id: String,
    pub event_name: String,
    pub server_ts: String,            // RFC 3339
    pub client_ts: Option<String>,    // RFC 3339
    pub session_id: Option<String>,
    pub device_id: String,
    pub properties: serde_json::Value,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    pub locale: Option<String>,
    pub network_type: Option<String>,
}

impl From<EventRow> for EventItem {
    fn from(row: EventRow) -> Self {
        Self {
            id: row.id.to_string(),
            event_name: row.event_name,
            server_ts: row.server_ts.to_rfc3339(),
            client_ts: row.client_ts.map(|t| t.to_rfc3339()),
            session_id: row.session_id,
            device_id: row.device_id,
            properties: row.properties,
            app_version: row.app_version,
            os_version: row.os_version,
            locale: row.locale,
            network_type: row.network_type,
        }
    }
}

/// GET /api/v1/admin/users/:id/events 成功响应的 data 部分。
#[derive(Debug, Serialize)]
pub struct EventQueryResponse {
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub items: Vec<EventItem>,
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// EventItem::from(EventRow) 所有字段都正确映射
    #[test]
    fn event_item_from_row_maps_all_fields() {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let row = EventRow {
            id,
            event_name: "gift_send_success".to_string(),
            server_ts: now,
            client_ts: Some(now),
            session_id: Some("sess-abc".to_string()),
            device_id: "dev-001".to_string(),
            properties: serde_json::json!({"amount": 100}),
            app_version: Some("1.2.0".to_string()),
            os_version: Some("Android 14".to_string()),
            locale: Some("ar-SA".to_string()),
            network_type: Some("wifi".to_string()),
        };

        let item = EventItem::from(row.clone());
        assert_eq!(item.id, id.to_string());
        assert_eq!(item.event_name, "gift_send_success");
        assert!(item.client_ts.is_some());
        assert_eq!(item.device_id, "dev-001");
        assert_eq!(item.locale.as_deref(), Some("ar-SA"));
    }

    /// EventItem::from — client_ts 为 None 时映射为 None
    #[test]
    fn event_item_from_row_client_ts_none() {
        let row = EventRow {
            id: Uuid::new_v4(),
            event_name: "room_join".to_string(),
            server_ts: Utc::now(),
            client_ts: None,
            session_id: None,
            device_id: "dev-002".to_string(),
            properties: serde_json::json!({}),
            app_version: None,
            os_version: None,
            locale: None,
            network_type: None,
        };
        let item = EventItem::from(row);
        assert!(item.client_ts.is_none());
    }
}
