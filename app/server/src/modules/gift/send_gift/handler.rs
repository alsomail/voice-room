//! WS SendGift 信令处理器
//!
//! 拆分自原 `send_gift.rs`（缺陷 #5）。负责：解析 WS payload、从 registry
//! 查询发送者所在房间、调用 [`SendGiftServicePort::send`]、把业务错误映射为
//! TDS 文档定义的错误码。

use std::sync::Arc;

use uuid::Uuid;

use crate::ws::registry::ConnectionRegistry;

use super::messages::{build_send_gift_result_response, send_gift_error_response};
use super::types::{SendGiftError, SendGiftPayload, SendGiftServicePort};

/// SendGift 信令所需依赖
pub struct SendGiftDeps {
    pub send_gift_service: Arc<dyn SendGiftServicePort>,
    pub registry: Arc<ConnectionRegistry>,
}

/// 处理 WS SendGift 信令，返回 JSON 字符串响应（给发送者）
pub async fn handle_send_gift(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &SendGiftDeps,
) -> String {
    let payload_val = match &payload {
        Some(p) => p,
        None => return send_gift_error_response(msg_id, 40002, "missing payload"),
    };

    let gift_id = match payload_val
        .get("gift_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return send_gift_error_response(msg_id, 40002, "invalid or missing gift_id"),
    };

    let receiver_id = match payload_val
        .get("receiver_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return send_gift_error_response(msg_id, 40002, "invalid or missing receiver_id"),
    };

    let count = match payload_val
        .get("count")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
    {
        Some(c) => c,
        None => return send_gift_error_response(msg_id, 40002, "invalid or missing count"),
    };

    let room_id = match deps.registry.get_room_id(connection_id) {
        Some(id) => id,
        None => return send_gift_error_response(msg_id, 40400, "sender not in any room"),
    };

    let payload_struct = SendGiftPayload {
        gift_id,
        receiver_id,
        count,
        msg_id: msg_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
    };

    match deps
        .send_gift_service
        .send(user_id, room_id, payload_struct)
        .await
    {
        Ok(result) => {
            build_send_gift_result_response(msg_id, result.gift_record_id, result.total_price)
        }
        Err(SendGiftError::InvalidCount) => {
            send_gift_error_response(msg_id, 40001, "invalid count: must be 1-9999")
        }
        Err(SendGiftError::SenderNotInRoom) => {
            send_gift_error_response(msg_id, 40400, "sender not in room")
        }
        Err(SendGiftError::GiftUnavailable) => {
            send_gift_error_response(msg_id, 40402, "gift not found or inactive")
        }
        Err(SendGiftError::ReceiverUnavailable) => {
            send_gift_error_response(msg_id, 40403, "receiver not on mic")
        }
        Err(SendGiftError::InsufficientBalance) => {
            send_gift_error_response(msg_id, 40290, "insufficient balance")
        }
        Err(SendGiftError::Internal(e)) => {
            tracing::error!("SendGift internal error: {}", e);
            send_gift_error_response(msg_id, 50000, "internal error")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::registry::ConnectionHandle;
    use std::sync::RwLock;
    use std::time::Instant;
    use tokio::sync::mpsc;

    use super::super::fake::FakeSendGiftService;

    fn make_registry_with_connection(
        user_id: Uuid,
        room_id: Uuid,
    ) -> (Arc<ConnectionRegistry>, Uuid, mpsc::UnboundedReceiver<String>) {
        let registry = Arc::new(ConnectionRegistry::new());
        let conn_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: Some(room_id),
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        (registry, conn_id, rx)
    }

    // SGH-01: 缺失 payload → code 40002
    #[tokio::test]
    async fn sgh01_missing_payload_returns_40002() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let (registry, conn_id, _rx) = make_registry_with_connection(user_id, room_id);
        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };
        let response =
            handle_send_gift(None, Some("msg-1".to_string()), conn_id, user_id, &deps).await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40002);
    }

    // SGH-02: 缺失 gift_id → code 40002
    #[tokio::test]
    async fn sgh02_missing_gift_id_returns_40002() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let (registry, conn_id, _rx) = make_registry_with_connection(user_id, room_id);
        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };
        let payload = serde_json::json!({
            "receiver_id": Uuid::new_v4().to_string(),
            "count": 1
        });
        let response = handle_send_gift(
            Some(payload),
            Some("msg-2".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40002);
    }

    // SGH-03: 发送者不在任何房间 → code 40400
    #[tokio::test]
    async fn sgh03_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let registry = Arc::new(ConnectionRegistry::new());
        let conn_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };
        let payload = serde_json::json!({
            "gift_id": Uuid::new_v4().to_string(),
            "receiver_id": Uuid::new_v4().to_string(),
            "count": 1
        });
        let response = handle_send_gift(
            Some(payload),
            Some("msg-3".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40400);
    }

    // SGH-04: 成功 → code 0，含 gift_record_id
    #[tokio::test]
    async fn sgh04_success_returns_code_0() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let (registry, conn_id, _rx) = make_registry_with_connection(user_id, room_id);
        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };
        let payload = serde_json::json!({
            "gift_id": Uuid::new_v4().to_string(),
            "receiver_id": Uuid::new_v4().to_string(),
            "count": 1
        });
        let response = handle_send_gift(
            Some(payload),
            Some("msg-4".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0);
        assert!(json["payload"]["gift_record_id"].is_string());
    }

    // SGH-05: SendGiftError Debug 实现
    #[test]
    fn sgh05_send_gift_error_debug() {
        let _ = format!("{:?}", SendGiftError::InvalidCount);
        let _ = format!("{:?}", SendGiftError::InsufficientBalance);
        let _ = format!("{:?}", SendGiftError::ReceiverUnavailable);
    }
}
