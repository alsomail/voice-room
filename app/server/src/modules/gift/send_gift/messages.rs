//! SendGift WS / 广播消息 envelope 构造器
//!
//! 拆分自原 `send_gift.rs`（缺陷 #5）。所有 JSON 构造集中在此，
//! 便于 schema 演进与字段单测。

use uuid::Uuid;

/// 构建 GiftReceived 广播消息 JSON（含 sender / receiver / gift 全部展示字段）。
///
// PROTO-BINDING: doc/protocol/schemas/ws/GiftReceived.schema.json (S→Room broadcast)
#[allow(clippy::too_many_arguments)]
pub fn build_gift_received_msg(
    gift_record_id: Uuid,
    sender_id: Uuid,
    sender_nickname: &str,
    sender_avatar: Option<&str>,
    receiver_id: Uuid,
    receiver_nickname: &str,
    receiver_avatar: Option<&str>,
    gift_id: Uuid,
    gift_code: &str,
    gift_name: &str,
    gift_icon_url: &str,
    gift_animation_url: Option<&str>,
    gift_effect_level: i16,
    count: i32,
    total_price: i64,
) -> serde_json::Value {
    serde_json::json!({
        "type": "GiftReceived",
        "payload": {
            "gift_record_id": gift_record_id.to_string(),
            "sender": {
                "user_id": sender_id.to_string(),
                "nickname": sender_nickname,
                "avatar": sender_avatar,
            },
            "receiver": {
                "user_id": receiver_id.to_string(),
                "nickname": receiver_nickname,
                "avatar": receiver_avatar,
            },
            "gift": {
                "id": gift_id.to_string(),
                "code": gift_code,
                "name": gift_name,
                "icon_url": gift_icon_url,
                "animation_url": gift_animation_url,
                "effect_level": gift_effect_level,
            },
            "count": count,
            "total_price": total_price,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    })
}

/// 构建 SendGiftResult 成功响应 JSON
///
// PROTO-BINDING: doc/protocol/schemas/ws/SendGiftResult.schema.json (S→C result)
pub fn build_send_gift_result_response(
    msg_id: Option<String>,
    gift_record_id: Uuid,
    total_price: i64,
) -> String {
    let resp = serde_json::json!({
        "type": "SendGiftResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": {
            "gift_record_id": gift_record_id.to_string(),
            "total_price": total_price,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

/// 构建 SendGiftResult 错误响应 JSON
pub fn send_gift_error_response(msg_id: Option<String>, code: i64, message: &str) -> String {
    let resp = serde_json::json!({
        "type": "SendGiftResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // SGM-01: build_gift_received_msg 包含 TDS 规定的全部字段
    #[test]
    fn sgm01_build_gift_received_msg_contains_required_fields() {
        let json = build_gift_received_msg(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Alice",
            Some("https://cdn.example.com/alice.png"),
            Uuid::new_v4(),
            "Bob",
            None,
            Uuid::new_v4(),
            "castle_01",
            "قصر",
            "https://cdn.example.com/castle.png",
            Some("https://cdn.example.com/castle.mp4"),
            4i16,
            2,
            1040,
        );
        assert_eq!(json["type"], "GiftReceived");
        assert_eq!(json["payload"]["count"], 2);
        assert_eq!(json["payload"]["total_price"], 1040);
        assert!(json.get("msg_id").is_none());
        assert_eq!(json["payload"]["sender"]["nickname"], "Alice");
        assert_eq!(
            json["payload"]["sender"]["avatar"],
            "https://cdn.example.com/alice.png"
        );
        assert_eq!(json["payload"]["receiver"]["nickname"], "Bob");
        assert!(json["payload"]["receiver"]["avatar"].is_null());
        assert_eq!(json["payload"]["gift"]["code"], "castle_01");
        assert_eq!(json["payload"]["gift"]["name"], "قصر");
        assert_eq!(
            json["payload"]["gift"]["icon_url"],
            "https://cdn.example.com/castle.png"
        );
        assert_eq!(
            json["payload"]["gift"]["animation_url"],
            "https://cdn.example.com/castle.mp4"
        );
        assert_eq!(json["payload"]["gift"]["effect_level"], 4);
    }

    // SGM-02: 成功响应 JSON 含 code=0 与 gift_record_id
    #[test]
    fn sgm02_success_response_shape() {
        let r = build_send_gift_result_response(
            Some("m-1".to_string()),
            Uuid::new_v4(),
            123,
        );
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["type"], "SendGiftResult");
        assert_eq!(v["code"], 0);
        assert_eq!(v["payload"]["total_price"], 123);
        assert!(v["payload"]["gift_record_id"].is_string());
    }

    // SGM-03: 错误响应 JSON 含 code 与 message
    #[test]
    fn sgm03_error_response_shape() {
        let r = send_gift_error_response(Some("m-2".to_string()), 40290, "insufficient balance");
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["code"], 40290);
        assert_eq!(v["message"], "insufficient balance");
    }
}
