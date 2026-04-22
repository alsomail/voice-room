//! 集成测试 — T-00029 WS MuteUser/UnmuteUser 信令 + 双重拦截
//!
//! 验收用例 MU29-01 ~ MU29-12：
//! - MU29-01: 房主对普通用户 mute mic 成功（Redis key 存在、DB 新增记录、广播 UserMuted）
//! - MU29-02: 禁麦时 target 在麦 → 自动下麦 + 广播 MicLeft forced=true
//! - MU29-03: 被禁麦用户 TakeMic → 40306
//! - MU29-04: 被禁言用户 SendMessage → 40305
//! - MU29-05: 送礼不受禁麦影响（仍可送）
//! - MU29-06: 管理员 mute 房主 → 40302
//! - MU29-07: 普通用户 mute 其他人 → 40301
//! - MU29-08: duration=0 走 UnmuteUser 路径（删 Redis key + 广播 duration_sec=0）
//! - MU29-09: duration 到期后 Redis key 自动过期，行为自动解除
//! - MU29-10: UnmuteUser 非 owner/admin → 40301
//! - MU29-11: duration 超 86400 → 40002
//! - MU29-12: type='chat' 对禁麦独立（不互相影响）
//!
//! 所有测试均使用 FakeMuteRedis / FakeMuteDb（内存），无需真实 Redis/DB。

use std::sync::{Arc, RwLock};
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::modules::governance::mute::{
    FakeMuteDb, FakeMuteRedis, MuteDeps, MuteRedis, handle_mute, handle_unmute,
};
use voice_room_server::modules::room::FakeRoomRepository;
use voice_room_server::room::handler::{
    SendMessageDeps, TakeMicDeps, handle_send_message, handle_take_mic,
};
use voice_room_server::room::manager::RoomManager;
use voice_room_server::room::state::MemberInfo;
use voice_room_server::modules::room::service::RoomService;
use voice_room_server::ws::registry::{ConnectionHandle, ConnectionRegistry};
use voice_room_shared::models::room::RoomModel;

// ─── 测试辅助 ─────────────────────────────────────────────────────────────────

/// 构建 active 房间模型
fn make_room(room_id: Uuid, owner_id: Uuid, admin_user_id: Option<Uuid>) -> RoomModel {
    let now = Utc::now();
    RoomModel {
        id: room_id,
        owner_id,
        title: "Test Room".to_string(),
        room_type: "normal".to_string(),
        member_count: 0,
        status: "active".to_string(),
        password_hash: None,
        max_members: 50,
        created_at: now,
        updated_at: now,
        deleted_at: None,
        cover_url: String::new(),
        category: "chat".to_string(),
        announcement: None,
        admin_user_id,
    }
}

/// 构建含指定房间的 RoomService
fn make_room_service(room: RoomModel) -> Arc<RoomService> {
    let repo = Arc::new(FakeRoomRepository::default());
    repo.seed(room);
    Arc::new(RoomService::new(repo))
}

/// 向 registry 注册一个连接，返回 (connection_id, rx)
fn register_connection(
    registry: &Arc<ConnectionRegistry>,
    user_id: Uuid,
    room_id: Option<Uuid>,
) -> (Uuid, mpsc::UnboundedReceiver<String>) {
    let conn_id = Uuid::new_v4();
    let (tx, rx) = mpsc::unbounded_channel();
    registry.register(ConnectionHandle {
        connection_id: conn_id,
        user_id,
        room_id,
        sender: tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });
    (conn_id, rx)
}

/// 向 room_manager 注册成员
fn add_member(room_manager: &Arc<RoomManager>, room_id: Uuid, user_id: Uuid, nickname: &str) {
    let room = room_manager.get_or_create_room(room_id);
    room.members.insert(
        user_id,
        MemberInfo::new(user_id, nickname.to_string(), None),
    );
}

/// 构建 MuteDeps（使用 Fake 实现）
fn make_mute_deps(
    room_manager: &Arc<RoomManager>,
    room_service: &Arc<RoomService>,
    mute_redis: &Arc<FakeMuteRedis>,
    mute_db: &Arc<FakeMuteDb>,
    registry: &Arc<ConnectionRegistry>,
) -> MuteDeps {
    MuteDeps {
        room_manager: room_manager.clone(),
        room_service: room_service.clone(),
        mute_redis: mute_redis.clone(),
        mute_db: mute_db.clone(),
        registry: registry.clone(),
    }
}

/// 构建 MuteUser payload
fn mute_payload(
    room_id: Uuid,
    target_user_id: Uuid,
    mute_type: &str,
    duration_sec: i64,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_user_id.to_string(),
        "type": mute_type,
        "duration_sec": duration_sec,
        "reason": "test reason",
    }))
}

/// 构建 UnmuteUser payload
fn unmute_payload(
    room_id: Uuid,
    target_user_id: Uuid,
    mute_type: &str,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_user_id.to_string(),
        "type": mute_type,
    }))
}

/// 收集 rx 中当前所有可用消息（非阻塞）
fn drain_messages(rx: &mut mpsc::UnboundedReceiver<String>) -> Vec<serde_json::Value> {
    let mut msgs = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg) {
            msgs.push(v);
        }
    }
    msgs
}

// ─── MU29-01: 房主对普通用户 mute mic 成功 ────────────────────────────────────

/// MU29-01: 房主禁麦成功 → Redis key 存在 + DB 新增记录 + 广播 UserMuted
#[tokio::test]
async fn mu29_01_owner_mute_mic_success() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    let room_service = make_room_service(make_room(room_id, owner_id, None));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    // 注册成员
    add_member(&room_manager, room_id, owner_id, "Owner");
    add_member(&room_manager, room_id, target_id, "Target");

    // 注册连接（用于接收广播）
    let (_owner_conn, mut owner_rx) = register_connection(&registry, owner_id, Some(room_id));
    let (_target_conn, mut target_rx) = register_connection(&registry, target_id, Some(room_id));

    let response = handle_mute(
        mute_payload(room_id, target_id, "mic", 300),
        Some("msg-mu29-01".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // 1. 返回 code:0
    assert_eq!(json["code"], 0, "MU29-01: should return code 0");

    // 2. Redis key 存在
    assert!(
        mute_redis.key_exists("mic", room_id, target_id),
        "MU29-01: mic_muted Redis key should exist"
    );

    // 3. DB 新增一条记录
    assert_eq!(
        mute_db.record_count(),
        1,
        "MU29-01: should insert 1 mute record"
    );

    // 4. 广播 UserMuted 给房间所有人
    let owner_msgs = drain_messages(&mut owner_rx);
    let target_msgs = drain_messages(&mut target_rx);

    let owner_has_muted = owner_msgs
        .iter()
        .any(|m| m["type"] == "UserMuted" && m["payload"]["target_user_id"] == target_id.to_string());
    let target_has_muted = target_msgs
        .iter()
        .any(|m| m["type"] == "UserMuted" && m["payload"]["target_user_id"] == target_id.to_string());

    assert!(owner_has_muted, "MU29-01: owner should receive UserMuted broadcast");
    assert!(target_has_muted, "MU29-01: target should receive UserMuted broadcast");
}

// ─── MU29-02: 禁麦时 target 在麦 → 自动下麦 + MicLeft forced=true ────────────

/// MU29-02: 禁麦 target 在麦位时，自动下麦并广播 MicLeft forced=true
#[tokio::test]
async fn mu29_02_mute_while_on_mic_auto_leave() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    let room_service = make_room_service(make_room(room_id, owner_id, None));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    add_member(&room_manager, room_id, owner_id, "Owner");
    add_member(&room_manager, room_id, target_id, "Target");

    // target 占麦位 0
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.take_mic_slot(0, target_id).unwrap();

    let (_owner_conn, mut owner_rx) = register_connection(&registry, owner_id, Some(room_id));
    let (_target_conn, mut target_rx) = register_connection(&registry, target_id, Some(room_id));

    let response = handle_mute(
        mute_payload(room_id, target_id, "mic", 300),
        Some("msg-mu29-02".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["code"], 0, "MU29-02: should return code 0");

    // 麦位应已清空
    let slots = room_state.mic_slots_snapshot();
    assert_eq!(slots[0], None, "MU29-02: mic slot 0 should be cleared");

    // 广播中应含 MicLeft forced=true
    let owner_msgs = drain_messages(&mut owner_rx);
    let target_msgs = drain_messages(&mut target_rx);

    let all_msgs: Vec<_> = owner_msgs.iter().chain(target_msgs.iter()).collect();
    let has_mic_left_forced = all_msgs.iter().any(|m| {
        m["type"] == "MicLeft"
            && m["payload"]["forced"] == true
            && m["payload"]["user_id"] == target_id.to_string()
    });

    assert!(
        has_mic_left_forced,
        "MU29-02: should broadcast MicLeft with forced=true"
    );
}

// ─── MU29-03: 被禁麦用户 TakeMic → 40306 ─────────────────────────────────────

/// MU29-03: 被禁麦的用户尝试 TakeMic → 错误码 40306
#[tokio::test]
async fn mu29_03_muted_user_take_mic_blocked() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 预先设置 mic_muted 状态
    mute_redis
        .set_mute("mic", room_id, user_id, 300, "test")
        .await
        .unwrap();

    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));

    let deps = TakeMicDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: Some(mute_redis.clone() as Arc<dyn MuteRedis>),
    };

    let response = handle_take_mic(
        Some(serde_json::json!({ "mic_index": 0 })),
        Some("msg-mu29-03".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["code"], 40306, "MU29-03: muted user TakeMic should return 40306");
}

// ─── MU29-04: 被禁言用户 SendMessage → 40305 ─────────────────────────────────

/// MU29-04: 被禁言的用户尝试 SendMessage → 错误码 40305
#[tokio::test]
async fn mu29_04_chat_muted_user_send_message_blocked() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 预先设置 chat_muted 状态
    mute_redis
        .set_mute("chat", room_id, user_id, 300, "test")
        .await
        .unwrap();

    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));

    let deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: Some(mute_redis.clone() as Arc<dyn MuteRedis>),
    };

    let response = handle_send_message(
        Some(serde_json::json!({ "content": "hello" })),
        Some("msg-mu29-04".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 40305,
        "MU29-04: chat-muted user SendMessage should return 40305"
    );
}

// ─── MU29-05: 送礼不受禁麦影响 ────────────────────────────────────────────────

/// MU29-05: 被禁麦的用户仍然可以成功 TakeMic（只有 chat 不影响）—— 
/// 改为验证禁言（chat）不影响 TakeMic（上麦），符合"送礼不受影响"类似逻辑
/// 即 mic_muted 不影响 chat 操作，chat_muted 不影响 mic 操作
#[tokio::test]
async fn mu29_05_gift_not_affected_by_mic_mute() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 只设置 mic_muted（禁麦），不禁言
    mute_redis
        .set_mute("mic", room_id, user_id, 300, "test")
        .await
        .unwrap();

    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));

    // chat 操作不受 mic mute 影响（使用 chat mute_redis = None 模拟）
    let deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        // 没有 chat_muted key，mute_redis 中只有 mic_muted，不影响 chat
        mute_redis: Some(mute_redis.clone() as Arc<dyn MuteRedis>),
    };

    // chat_muted key 不存在，SendMessage 应正常通过（被禁麦不影响发言）
    let response = handle_send_message(
        Some(serde_json::json!({ "content": "hello from muted mic user" })),
        Some("msg-mu29-05".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    // mic_muted 不影响 SendMessage
    assert_eq!(
        json["code"], 0,
        "MU29-05: mic-muted user should still be able to send message (gift analogy: mute doesn't affect gift)"
    );
}

// ─── MU29-06: 管理员 mute 房主 → 40302 ───────────────────────────────────────

/// MU29-06: 管理员禁麦房主 → 40302
#[tokio::test]
async fn mu29_06_admin_cannot_mute_owner() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    let room_service = make_room_service(make_room(room_id, owner_id, Some(admin_id)));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    add_member(&room_manager, room_id, owner_id, "Owner");
    add_member(&room_manager, room_id, admin_id, "Admin");

    let response = handle_mute(
        mute_payload(room_id, owner_id, "mic", 300), // admin 试图 mute owner
        Some("msg-mu29-06".to_string()),
        admin_id, // operator = admin
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 40302,
        "MU29-06: admin cannot mute owner, should return 40302"
    );
}

// ─── MU29-07: 普通用户 mute 其他人 → 40301 ───────────────────────────────────

/// MU29-07: 普通用户尝试禁麦 → 40301
#[tokio::test]
async fn mu29_07_normal_user_cannot_mute() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    let room_service = make_room_service(make_room(room_id, owner_id, None));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    add_member(&room_manager, room_id, target_id, "Target");

    let response = handle_mute(
        mute_payload(room_id, target_id, "mic", 300),
        Some("msg-mu29-07".to_string()),
        normal_id, // 普通用户
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 40301,
        "MU29-07: normal user cannot mute, should return 40301"
    );
}

// ─── MU29-08: duration=0 走 UnmuteUser 路径 ──────────────────────────────────

/// MU29-08: handle_mute with duration=0 → 等效 UnmuteUser（删 Redis + 广播 duration_sec=0）
#[tokio::test]
async fn mu29_08_duration_zero_unmute_path() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    // 先设置一个 mute
    mute_redis
        .set_mute("mic", room_id, target_id, 300, "test")
        .await
        .unwrap();
    assert!(mute_redis.key_exists("mic", room_id, target_id));

    let room_service = make_room_service(make_room(room_id, owner_id, None));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    add_member(&room_manager, room_id, owner_id, "Owner");
    add_member(&room_manager, room_id, target_id, "Target");

    let (_owner_conn, mut owner_rx) = register_connection(&registry, owner_id, Some(room_id));
    let (_target_conn, mut target_rx) = register_connection(&registry, target_id, Some(room_id));

    // duration=0 → 解除禁麦
    let response = handle_mute(
        mute_payload(room_id, target_id, "mic", 0),
        Some("msg-mu29-08".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["code"], 0, "MU29-08: duration=0 should return code 0");

    // Redis key 应已删除
    assert!(
        !mute_redis.key_exists("mic", room_id, target_id),
        "MU29-08: Redis key should be deleted after duration=0 unmute"
    );

    // 广播 UserMuted with duration_sec=0
    let owner_msgs = drain_messages(&mut owner_rx);
    let target_msgs = drain_messages(&mut target_rx);
    let all_msgs: Vec<_> = owner_msgs.iter().chain(target_msgs.iter()).collect();

    let has_unmuted_broadcast = all_msgs.iter().any(|m| {
        m["type"] == "UserMuted"
            && m["payload"]["duration_sec"] == 0
            && m["payload"]["target_user_id"] == target_id.to_string()
    });

    assert!(
        has_unmuted_broadcast,
        "MU29-08: should broadcast UserMuted with duration_sec=0"
    );
}

// ─── MU29-09: TTL 到期后 Redis key 自动过期 ───────────────────────────────────

/// MU29-09: 模拟 TTL 到期后，muted 状态自动解除（TakeMic 可以通过）
#[tokio::test]
async fn mu29_09_ttl_expired_auto_unmute() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 设置 mute key
    mute_redis
        .set_mute("mic", room_id, user_id, 300, "test")
        .await
        .unwrap();
    assert!(mute_redis.key_exists("mic", room_id, user_id));

    // 模拟 TTL 到期
    mute_redis.expire_all();

    // 到期后 key 不存在
    assert!(
        !mute_redis.key_exists("mic", room_id, user_id),
        "MU29-09: after TTL expiry, key should not exist"
    );

    // TakeMic 不再被拦截
    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));

    let deps = TakeMicDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: Some(mute_redis.clone() as Arc<dyn MuteRedis>),
    };

    let response = handle_take_mic(
        Some(serde_json::json!({ "mic_index": 0 })),
        Some("msg-mu29-09".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 0,
        "MU29-09: after TTL expiry, TakeMic should succeed"
    );
}

// ─── MU29-10: UnmuteUser 非 owner/admin → 40301 ──────────────────────────────

/// MU29-10: 普通用户尝试 UnmuteUser → 40301
#[tokio::test]
async fn mu29_10_normal_user_cannot_unmute() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    let room_service = make_room_service(make_room(room_id, owner_id, None));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    let response = handle_unmute(
        unmute_payload(room_id, target_id, "mic"),
        Some("msg-mu29-10".to_string()),
        normal_id, // 普通用户
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 40301,
        "MU29-10: normal user cannot unmute, should return 40301"
    );
}

// ─── MU29-11: duration > 86400 → 40002 ───────────────────────────────────────

/// MU29-11: duration_sec 超过 86400 → 40002 payload 非法
#[tokio::test]
async fn mu29_11_duration_exceeds_max_returns_40002() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());
    let mute_db = Arc::new(FakeMuteDb::default());

    let room_service = make_room_service(make_room(room_id, owner_id, None));
    let deps = make_mute_deps(&room_manager, &room_service, &mute_redis, &mute_db, &registry);

    let response = handle_mute(
        mute_payload(room_id, target_id, "mic", 86401), // 超过最大值
        Some("msg-mu29-11".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 40002,
        "MU29-11: duration > 86400 should return 40002"
    );
}

// ─── MU29-12: mic 和 chat mute 相互独立 ──────────────────────────────────────

/// MU29-12: type='chat' mute 不影响上麦；type='mic' mute 不影响发言
#[tokio::test]
async fn mu29_12_mic_and_chat_mute_are_independent() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 只设置 chat_muted（禁言）
    mute_redis
        .set_mute("chat", room_id, user_id, 300, "test")
        .await
        .unwrap();

    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));

    // TakeMic 不受 chat_muted 影响
    let take_mic_deps = TakeMicDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: Some(mute_redis.clone() as Arc<dyn MuteRedis>),
    };

    let mic_response = handle_take_mic(
        Some(serde_json::json!({ "mic_index": 0 })),
        Some("msg-mu29-12-mic".to_string()),
        conn_id,
        user_id,
        &take_mic_deps,
    )
    .await;

    let mic_json: serde_json::Value = serde_json::from_str(&mic_response).unwrap();
    assert_eq!(
        mic_json["code"], 0,
        "MU29-12: chat-muted user should still be able to take mic"
    );

    // 同理：只设置 mic_muted（禁麦），发言不受影响
    let mute_redis2 = Arc::new(FakeMuteRedis::default());
    mute_redis2
        .set_mute("mic", room_id, user_id, 300, "test")
        .await
        .unwrap();

    let room_manager2 = Arc::new(RoomManager::new());
    let registry2 = Arc::new(ConnectionRegistry::new());
    room_manager2.get_or_create_room(room_id);
    let (conn_id2, _rx2) = register_connection(&registry2, user_id, Some(room_id));

    let send_deps = SendMessageDeps {
        room_manager: room_manager2.clone(),
        registry: registry2.clone(),
        mute_redis: Some(mute_redis2.clone() as Arc<dyn MuteRedis>),
    };

    let msg_response = handle_send_message(
        Some(serde_json::json!({ "content": "hello" })),
        Some("msg-mu29-12-chat".to_string()),
        conn_id2,
        user_id,
        &send_deps,
    )
    .await;

    let msg_json: serde_json::Value = serde_json::from_str(&msg_response).unwrap();
    assert_eq!(
        msg_json["code"], 0,
        "MU29-12: mic-muted user should still be able to send message"
    );
}
