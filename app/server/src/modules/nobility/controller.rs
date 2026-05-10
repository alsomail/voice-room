//! 贵族模块 HTTP handler（T-00066 / T-00067）
//!
//! - `list_tiers_handler`  — GET /api/v1/nobles/tiers（无需鉴权）
//! - `get_me_handler`      — GET /api/v1/nobles/me（JWT 鉴权）
//! - `purchase_handler`    — POST /api/v1/nobles/purchase（JWT 鉴权）
//! - `set_auto_renew_handler` — PATCH /api/v1/nobles/me/auto_renew（JWT 鉴权）

use axum::{
    extract::{Json, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension,
};
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext,
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

use super::dto::{AutoRenewRequest, AutoRenewResponse, PurchaseRequest};

/// 解析 Accept-Language header → 语言代码（"ar-SA" → "ar"，默认 "en"）
fn parse_lang(headers: &HeaderMap) -> String {
    headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // 取第一个语言标记
            let first = s.split(',').next().unwrap_or("en").trim();
            let lang_part = first.split('-').next().unwrap_or("en");
            Some(lang_part.to_string())
        })
        .unwrap_or_else(|| "en".to_string())
}

/// 向用户所有在线 WS 连接单播 `BalanceUpdated` 信令（T-00067）
///
/// 在贵族购买/续费/升级成功后，通知余额变化。
fn send_balance_updated(
    registry: &crate::ws::ConnectionRegistry,
    user_id: Uuid,
    reason: &str,
    balance_after: i64,
    delta: i64,
) {
    let connections = registry.get_by_user_id(user_id);
    for (_, sender) in connections {
        let msg = serde_json::json!({
            "type": "BalanceUpdated",
            "msg_id": Uuid::new_v4().to_string(),
            "payload": {
                "diamond_balance": balance_after,
                "delta": delta,
                "reason": reason,
                "ref_id": serde_json::Value::Null,
            },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        if let Ok(s) = serde_json::to_string(&msg) {
            let _ = sender.send(s);
        }
    }
}

/// 向用户所有在线 WS 连接单播 `NobleChanged` 信令（T-00067 §10.4.1）
///
/// 同时向用户当前所在房间广播 `NobleChanged`（若用户在房间内）。
fn send_noble_changed(
    registry: &crate::ws::ConnectionRegistry,
    room_manager: &crate::room::RoomManager,
    user_id: Uuid,
    from_tier: Option<&str>,
    to_tier: &str,
    expire_at_ms: i64,
    operation: &str,
) {
    let payload = serde_json::json!({
        "user_id": user_id.to_string(),
        "from_tier": from_tier,
        "to_tier": to_tier,
        "expire_at": expire_at_ms,
        "operation": operation,
    });

    // 单播给购买用户（所有在线连接）
    let connections = registry.get_by_user_id(user_id);
    for (_, sender) in &connections {
        let msg = serde_json::json!({
            "type": "NobleChanged",
            "msg_id": Uuid::new_v4().to_string(),
            "payload": payload,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        if let Ok(s) = serde_json::to_string(&msg) {
            let _ = sender.send(s);
        }
    }

    // 若用户当前在房间，则广播给房间内所有连接
    // 查找用户的 room_id（从第一条连接的 room_id 取）
    let room_id_opt = registry.connections.iter()
        .find(|entry| entry.user_id == user_id && entry.room_id.is_some())
        .and_then(|entry| entry.room_id);

    if let Some(room_id) = room_id_opt {
        let envelope = serde_json::json!({
            "type": "NobleChanged",
            "payload": payload,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        if let Some(rs) = room_manager.get_room(room_id) {
            crate::ws::broadcaster::broadcast_to_room(registry, &rs, envelope);
        }
    }
}

// ─── GET /api/v1/nobles/tiers ─────────────────────────────────────────────────

/// GET /api/v1/nobles/tiers — 无需鉴权；返回所有上架 tier（本地化）
pub async fn list_tiers_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
) -> axum::response::Response {
    let lang = parse_lang(&headers);
    match state.nobility_service.list_tiers(&lang).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── GET /api/v1/nobles/me ────────────────────────────────────────────────────

/// GET /api/v1/nobles/me — JWT 鉴权；返回当前用户贵族状态
pub async fn get_me_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    headers: HeaderMap,
) -> axum::response::Response {
    let lang = parse_lang(&headers);
    match state.nobility_service.get_my_noble(ctx.user_id, &lang).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── POST /api/v1/nobles/purchase ─────────────────────────────────────────────

/// POST /api/v1/nobles/purchase — JWT 鉴权；钻石购买/续费/升级
///
/// T-00067: 购买成功后发送 WS 信令：
///   1. `BalanceUpdated`（reason = noble_purchase / noble_renew / noble_upgrade_proration）
///   2. `NobleChanged`（单播给购买用户 + 若在房间则房间广播）
pub async fn purchase_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<PurchaseRequest>,
) -> axum::response::Response {
    let user_id = ctx.user_id;
    match state.nobility_service.purchase(user_id, req).await {
        Ok(resp) => {
            // T-00067: 购买成功后推送 WS 信令
            let reason = match resp.operation.as_str() {
                "renew"   => "noble_renew",
                "upgrade" => "noble_upgrade_proration",
                _         => "noble_purchase",
            };

            // 1. BalanceUpdated（余额变化通知）
            send_balance_updated(
                &state.ws_registry,
                user_id,
                reason,
                resp.balance_after,
                -resp.diamonds_charged,
            );

            // 2. NobleChanged（贵族状态变化通知）
            let to_tier = resp.user_noble.tier_id.as_deref().unwrap_or("");
            let expire_at_ms = resp.user_noble.expire_at
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            send_noble_changed(
                &state.ws_registry,
                &state.room_manager,
                user_id,
                None, // from_tier: 暂用 None（FakeService 不追踪旧 tier）
                to_tier,
                expire_at_ms,
                &resp.operation,
            );

            Json(ApiResponse::ok(resp, rc.request_id())).into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── PATCH /api/v1/nobles/me/auto_renew ──────────────────────────────────────

/// PATCH /api/v1/nobles/me/auto_renew — JWT 鉴权；切换自动续费
pub async fn set_auto_renew_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<AutoRenewRequest>,
) -> axum::response::Response {
    match state
        .nobility_service
        .set_auto_renew(ctx.user_id, req.enabled)
        .await
    {
        Ok(enabled) => Json(ApiResponse::ok(
            AutoRenewResponse { auto_renew: enabled },
            rc.request_id(),
        ))
        .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::bootstrap::{build_app, AppState};

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // NC01: GET /nobles/tiers 无需鉴权 → 200
    #[tokio::test]
    async fn nc01_list_tiers_no_auth_returns_200() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/tiers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["code"], 0);
        assert!(body["data"]["tiers"].is_array());
    }

    // NC02: GET /nobles/tiers 返回 6 个 tier
    #[tokio::test]
    async fn nc02_list_tiers_returns_six_tiers() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/tiers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = body_json(response).await;
        let tiers = body["data"]["tiers"].as_array().unwrap();
        assert_eq!(tiers.len(), 6);
    }

    // NC03: GET /nobles/me 无 token → 401
    #[tokio::test]
    async fn nc03_get_me_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // NC04: GET /nobles/me 有效 token → 200 + tier_id=null（FakeNobilityService）
    #[tokio::test]
    async fn nc04_get_me_valid_token_returns_null_tier() {
        use std::time::{SystemTime, UNIX_EPOCH};
        use voice_room_shared::jwt::token::{encode_token, AppClaims};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = AppClaims {
            sub: uuid::Uuid::new_v4().to_string(),
            iss: "voiceroom".to_string(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode_token(&claims, b"test-secret").unwrap();

        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/me")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["code"], 0);
        assert!(body["data"]["tier_id"].is_null());
    }

    // NC05: Accept-Language: ar-SA → tiers 里有阿拉伯文 name（FakeNobilityService 暂时返回英文）
    // 真实实现需要从 DB 读取 name_ar，Fake 返回英文，此测试验证 200 成功即可
    #[tokio::test]
    async fn nc05_accept_language_ar_returns_200() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/tiers")
                    .header("Accept-Language", "ar-SA")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // NC06: POST /nobles/purchase 无 token → 401
    #[tokio::test]
    async fn nc06_purchase_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/nobles/purchase")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"tier_id":"duke","msg_id":"abc"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // NC07: parse_lang 解析正确
    #[test]
    fn nc07_parse_lang_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("accept-language", "ar-SA,ar;q=0.9".parse().unwrap());
        assert_eq!(parse_lang(&headers), "ar");

        let mut headers2 = HeaderMap::new();
        headers2.insert("accept-language", "en-US".parse().unwrap());
        assert_eq!(parse_lang(&headers2), "en");

        // 无 header → 默认 en
        let headers3 = HeaderMap::new();
        assert_eq!(parse_lang(&headers3), "en");
    }

    // NC08: send_balance_updated — 注册连接后能收到 BalanceUpdated 信令
    #[tokio::test]
    async fn nc08_send_balance_updated_delivers_ws_signal() {
        use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};
        use std::sync::{Arc, RwLock};
        use std::time::Instant;
        use tokio::sync::mpsc;

        let registry = Arc::new(ConnectionRegistry::new());
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let user_id = uuid::Uuid::new_v4();
        let conn_id = uuid::Uuid::new_v4();

        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        // Act: send BalanceUpdated signal
        send_balance_updated(&registry, user_id, "noble_purchase", 700000, -300000);

        // Assert: WS connection received the signal
        let msg = rx.try_recv().expect("should have received BalanceUpdated");
        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "BalanceUpdated");
        assert_eq!(json["payload"]["reason"], "noble_purchase");
        assert_eq!(json["payload"]["diamond_balance"], 700000);
        assert_eq!(json["payload"]["delta"], -300000);
    }

    // NC09: send_noble_changed — 注册连接后能收到 NobleChanged 信令
    #[tokio::test]
    async fn nc09_send_noble_changed_delivers_ws_signal() {
        use crate::room::RoomManager;
        use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};
        use std::sync::{Arc, RwLock};
        use std::time::Instant;
        use tokio::sync::mpsc;

        let registry = Arc::new(ConnectionRegistry::new());
        let room_manager = Arc::new(RoomManager::new());
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let user_id = uuid::Uuid::new_v4();
        let conn_id = uuid::Uuid::new_v4();

        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        // Act: send NobleChanged signal
        send_noble_changed(&registry, &room_manager, user_id, None, "duke", 9999999, "purchase");

        // Assert: WS connection received the signal
        let msg = rx.try_recv().expect("should have received NobleChanged");
        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "NobleChanged");
        assert_eq!(json["payload"]["to_tier"], "duke");
        assert_eq!(json["payload"]["operation"], "purchase");
        assert!(json["payload"]["from_tier"].is_null());
    }

    // NC10: send_noble_changed — 用户在房间时，房间内其他连接也收到 NobleChanged
    #[tokio::test]
    async fn nc10_noble_changed_broadcasts_to_room() {
        use crate::room::RoomManager;
        use crate::room::state::MemberInfo;
        use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};
        use std::sync::{Arc, RwLock};
        use std::time::Instant;
        use tokio::sync::mpsc;

        let registry = Arc::new(ConnectionRegistry::new());
        let room_manager = Arc::new(RoomManager::new());
        let room_id = uuid::Uuid::new_v4();

        // Buyer user
        let buyer_id = uuid::Uuid::new_v4();
        let (buyer_tx, mut buyer_rx) = mpsc::unbounded_channel::<String>();
        let buyer_conn = uuid::Uuid::new_v4();
        registry.register(ConnectionHandle {
            connection_id: buyer_conn,
            user_id: buyer_id,
            room_id: Some(room_id),
            sender: buyer_tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        // Other room member
        let other_id = uuid::Uuid::new_v4();
        let (other_tx, mut other_rx) = mpsc::unbounded_channel::<String>();
        let other_conn = uuid::Uuid::new_v4();
        registry.register(ConnectionHandle {
            connection_id: other_conn,
            user_id: other_id,
            room_id: Some(room_id),
            sender: other_tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        // Setup room state
        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(buyer_id, MemberInfo::new(buyer_id, "Buyer".into(), None));
        room_state.members.insert(other_id, MemberInfo::new(other_id, "Other".into(), None));

        // Act
        send_noble_changed(&registry, &room_manager, buyer_id, None, "king", 9999999, "purchase");

        // Assert: buyer receives NobleChanged (unicast, may appear twice: unicast + broadcast)
        // Drain buyer's channel
        let mut buyer_msgs = vec![];
        while let Ok(m) = buyer_rx.try_recv() {
            buyer_msgs.push(m);
        }
        assert!(!buyer_msgs.is_empty(), "buyer should receive NobleChanged");

        // Other member also receives NobleChanged (room broadcast)
        let other_msg = other_rx.try_recv().expect("other member should receive NobleChanged");
        let json: serde_json::Value = serde_json::from_str(&other_msg).unwrap();
        assert_eq!(json["type"], "NobleChanged");
        assert_eq!(json["payload"]["to_tier"], "king");
    }
}
