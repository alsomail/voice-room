use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 送礼记录模型
///
/// 对应数据库表 `gift_records`，由迁移 `006_create_gift_records.sql` 创建。
/// - `(sender_id, msg_id)` UNIQUE 约束实现幂等
/// - `count` 范围 1-9999
/// - `total_price` = gift.price * count
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GiftRecordModel {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub receiver_id: Uuid,
    pub room_id: Uuid,
    pub gift_id: Uuid,
    pub count: i32,
    pub total_price: i64,
    pub msg_id: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // GR01: GiftRecordModel 所有字段可构造
    #[test]
    fn gr01_gift_record_model_constructible() {
        let model = GiftRecordModel {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            receiver_id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            gift_id: Uuid::new_v4(),
            count: 1,
            total_price: 520,
            msg_id: "test-msg-id".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(model.count, 1);
        assert_eq!(model.total_price, 520);
        assert_eq!(model.msg_id, "test-msg-id");
    }

    // GR02: GiftRecordModel 实现 Clone
    #[test]
    fn gr02_gift_record_model_clone() {
        let original = GiftRecordModel {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            receiver_id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            gift_id: Uuid::new_v4(),
            count: 2,
            total_price: 1040,
            msg_id: "clone-test".to_string(),
            created_at: Utc::now(),
        };
        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.total_price, original.total_price);
    }

    // GR03: GiftRecordModel 实现 Debug
    #[test]
    fn gr03_gift_record_model_debug() {
        let model = GiftRecordModel {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            receiver_id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            gift_id: Uuid::new_v4(),
            count: 1,
            total_price: 1,
            msg_id: "debug-test".to_string(),
            created_at: Utc::now(),
        };
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("GiftRecordModel"));
    }

    // GR04: GiftRecordModel 支持 JSON 序列化
    #[test]
    fn gr04_gift_record_model_serializes_to_json() {
        let model = GiftRecordModel {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            receiver_id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            gift_id: Uuid::new_v4(),
            count: 3,
            total_price: 999,
            msg_id: "json-test".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&model).expect("GR04: should serialize to JSON");
        assert!(json.contains("total_price"));
        assert!(json.contains("999"));
    }

    // GR05: msg_id 支持 Unicode 特殊字符
    #[test]
    fn gr05_msg_id_supports_unicode() {
        let model = GiftRecordModel {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            receiver_id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            gift_id: Uuid::new_v4(),
            count: 1,
            total_price: 1,
            msg_id: "测试-消息-🎁".to_string(),
            created_at: Utc::now(),
        };
        assert!(model.msg_id.contains('🎁'));
    }
}
