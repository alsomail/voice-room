//! EventModel — events 表的 Rust 数据模型（T-00022）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// events 表的完整记录结构（供 SELECT 查询映射使用）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventModel {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub device_id: String,
    pub event_name: String,
    pub properties: serde_json::Value,
    pub session_id: Option<String>,
    pub client_ts: Option<DateTime<Utc>>,
    pub server_ts: DateTime<Utc>,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    pub locale: Option<String>,
    pub network_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_model_fields_accessible() {
        // 确保结构体字段类型正确
        let _model = EventModel {
            id: Uuid::new_v4(),
            user_id: None,
            device_id: "device-001".to_string(),
            event_name: "gift_send".to_string(),
            properties: serde_json::json!({"key": "value"}),
            session_id: Some("sess-001".to_string()),
            client_ts: None,
            server_ts: Utc::now(),
            app_version: Some("1.0.0".to_string()),
            os_version: Some("Android 14".to_string()),
            locale: Some("ar-SA".to_string()),
            network_type: Some("wifi".to_string()),
        };
    }
}
