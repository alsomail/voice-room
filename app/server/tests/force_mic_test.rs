//! 集成测试 — T-00030 WS ForceTakeMic / ForceLeaveMic 信令
//!
//! 验收用例：
//! - FM30-07: 房主 ForceTakeMic 空闲麦位 → 广播 MicTaken forced_by
//! - FM30-08: 麦位被占 → 40907
//! - FM30-09: target 被禁麦 → 40306
//! - FM30-10: ForceLeaveMic 麦上用户 → 广播 MicLeft forced_by
//! - FM30-11: ForceLeaveMic 非麦上用户 → 40404
//! - FM30-12: 管理员 ForceLeaveMic 房主 → 40302
//! - FM30-13: 普通用户 ForceTakeMic/ForceLeaveMic → 40301

use std::sync::{Arc, RwLock};
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::modules::governance::force_mic::{
    handle_force_leave_mic, handle_force_take_mic, ForceLeaveMicDeps, ForceTakeMicDeps,
};
use voice_room_server::modules::governance::mute::{FakeMuteRedis, MuteRedis};
use voice_room_server::modules::room::service::RoomService;
use voice_room_server::modules::room::FakeRoomRepository;
use voice_room_server::room::manager::RoomManager;
use voice_room_server::room::state::MemberInfo;
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

/// 构建 ForceTakeMicDeps
fn make_force_take_deps(
    room_manager: &Arc<RoomManager>,
    room_service: &Arc<RoomService>,
    mute_redis: Arc<dyn MuteRedis>,
    registry: &Arc<ConnectionRegistry>,
) -> ForceTakeMicDeps {
    ForceTakeMicDeps {
        room_manager: room_manager.clone(),
        room_service: room_service.clone(),
        mute_redis,
        registry: registry.clone(),
    }
}

/// 构建 ForceLeaveMicDeps
fn make_force_leave_deps(
    room_manager: &Arc<RoomManager>,
    room_service: &Arc<RoomService>,
    registry: &Arc<ConnectionRegistry>,
) -> ForceLeaveMicDeps {
    ForceLeaveMicDeps {
        room_manager: room_manager.clone(),
        room_service: room_service.clone(),
        registry: registry.clone(),
    }
}

/// 构建 ForceTakeMic payload
fn take_mic_payload(
    room_id: Uuid,
    target_user_id: Uuid,
    slot_index: u64,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_user_id.to_string(),
        "slot_index": slot_index,
    }))
}

/// 构建 ForceLeaveMic payload
fn leave_mic_payload(room_id: Uuid, target_user_id: Uuid) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_user_id.to_string(),
    }))
}

// ─── 测试用例 ─────────────────────────────────────────────────────────────────

/// FM30-07: 房主 ForceTakeMic 空闲麦位 → 广播 MicTaken { forced_by }
#[tokio::test]
async fn fm30_07_owner_force_take_mic_success() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 创建房间内存状态 + 添加成员
    room_manager.get_or_create_room(room_id);
    add_member(&room_manager, room_id, owner_id, "owner");
    add_member(&room_manager, room_id, target_id, "target");

    // 注册连接
    let (_, mut rx_owner) = register_connection(&registry, owner_id, Some(room_id));

    let deps = make_force_take_deps(&room_manager, &room_service, mute_redis, &registry);

    let resp = handle_force_take_mic(
        take_mic_payload(room_id, target_id, 2),
        Some("msg-7".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["type"], "ForceTakeMicResult",
        "FM30-07: type should be ForceTakeMicResult"
    );
    assert_eq!(v["code"], 0, "FM30-07: should succeed");
    assert_eq!(
        v["payload"]["mic_index"], 2,
        "FM30-07: mic_index should be 2"
    );

    // 验证广播 MicTaken 含 forced_by
    let broadcast = rx_owner
        .try_recv()
        .expect("FM30-07: should receive MicTaken broadcast");
    let bv: serde_json::Value = serde_json::from_str(&broadcast).unwrap();
    assert_eq!(
        bv["type"], "MicTaken",
        "FM30-07: broadcast type should be MicTaken"
    );
    assert_eq!(
        bv["payload"]["forced_by"],
        owner_id.to_string(),
        "FM30-07: forced_by should be operator (owner)"
    );
    assert_eq!(
        bv["payload"]["user_id"],
        target_id.to_string(),
        "FM30-07: user_id should be target"
    );
    assert_eq!(
        bv["payload"]["mic_index"], 2,
        "FM30-07: mic_index should be 2"
    );

    // 验证内存状态更新
    let room_state = room_manager.get_room(room_id).unwrap();
    let slots = room_state.mic_slots_snapshot();
    assert_eq!(
        slots[2],
        Some(target_id),
        "FM30-07: slot 2 should be occupied by target"
    );
}

/// FM30-08: 麦位被占 → 40907
#[tokio::test]
async fn fm30_08_occupied_slot_returns_40907() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let occupier_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 创建房间 + slot 1 已被 occupier 占用
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.take_mic_slot(1, occupier_id).unwrap();

    let deps = make_force_take_deps(&room_manager, &room_service, mute_redis, &registry);

    let resp = handle_force_take_mic(
        take_mic_payload(room_id, target_id, 1),
        Some("msg-8".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["code"], 40907,
        "FM30-08: occupied slot should return 40907"
    );
}

/// FM30-09: target 被禁麦 → 40306
#[tokio::test]
async fn fm30_09_muted_target_returns_40306() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // 设置禁麦（3600 秒）
    mute_redis
        .set_mute("mic", room_id, target_id, 3600, "test mute")
        .await
        .unwrap();

    room_manager.get_or_create_room(room_id);

    let deps = make_force_take_deps(&room_manager, &room_service, mute_redis, &registry);

    let resp = handle_force_take_mic(
        take_mic_payload(room_id, target_id, 3),
        Some("msg-9".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["code"], 40306,
        "FM30-09: muted target should return 40306"
    );
}

/// FM30-10: ForceLeaveMic 麦上用户 → 广播 MicLeft { forced_by }
#[tokio::test]
async fn fm30_10_force_leave_mic_success() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    // 创建房间 + target 占 slot 0
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.take_mic_slot(0, target_id).unwrap();
    add_member(&room_manager, room_id, target_id, "target");

    // 注册连接
    let (_, mut rx_owner) = register_connection(&registry, owner_id, Some(room_id));
    let (_, mut rx_target) = register_connection(&registry, target_id, Some(room_id));

    let deps = make_force_leave_deps(&room_manager, &room_service, &registry);

    let resp = handle_force_leave_mic(
        leave_mic_payload(room_id, target_id),
        Some("msg-10".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["type"], "ForceLeaveMicResult",
        "FM30-10: type should be ForceLeaveMicResult"
    );
    assert_eq!(v["code"], 0, "FM30-10: should succeed");

    // 验证麦位已释放
    let room_state = room_manager.get_room(room_id).unwrap();
    let slots = room_state.mic_slots_snapshot();
    assert_eq!(slots[0], None, "FM30-10: slot 0 should be freed");

    // 验证广播 MicLeft 含 forced_by
    let check_broadcast = |name: &str, rx: &mut mpsc::UnboundedReceiver<String>| {
        let msg = rx
            .try_recv()
            .unwrap_or_else(|_| panic!("FM30-10: {name} should receive MicLeft broadcast"));
        let bv: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            bv["type"], "MicLeft",
            "FM30-10: {name} should receive MicLeft"
        );
        assert_eq!(
            bv["payload"]["forced"], true,
            "FM30-10: {name} forced should be true"
        );
        assert_eq!(
            bv["payload"]["forced_by"],
            owner_id.to_string(),
            "FM30-10: {name} forced_by should be owner"
        );
        assert_eq!(
            bv["payload"]["mic_index"], 0,
            "FM30-10: {name} mic_index should be 0"
        );
    };

    check_broadcast("owner", &mut rx_owner);
    check_broadcast("target", &mut rx_target);
}

/// FM30-11: ForceLeaveMic 非麦上用户 → 40404 MIC_NOT_FOUND
#[tokio::test]
async fn fm30_11_force_leave_not_on_mic_returns_40404() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    // 创建房间但 target 不在麦上
    room_manager.get_or_create_room(room_id);

    let deps = make_force_leave_deps(&room_manager, &room_service, &registry);

    let resp = handle_force_leave_mic(
        leave_mic_payload(room_id, target_id),
        Some("msg-11".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["code"], 40404,
        "FM30-11: target not on mic should return 40404"
    );
}

/// FM30-12: 管理员 ForceLeaveMic 房主 → 40302
#[tokio::test]
async fn fm30_12_admin_cannot_force_leave_owner() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();

    // 房间有管理员 admin_id
    let room = make_room(room_id, owner_id, Some(admin_id));
    let room_service = make_room_service(room);
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    // owner 在麦上
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.take_mic_slot(0, owner_id).unwrap();

    let deps = make_force_leave_deps(&room_manager, &room_service, &registry);

    // 管理员尝试把房主抱下麦
    let resp = handle_force_leave_mic(
        leave_mic_payload(room_id, owner_id),
        Some("msg-12".to_string()),
        admin_id, // operator = admin
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["code"], 40302,
        "FM30-12: admin cannot force owner off mic, should return 40302"
    );

    // 验证房主还在麦上
    let room_state = room_manager.get_room(room_id).unwrap();
    let slots = room_state.mic_slots_snapshot();
    assert_eq!(
        slots[0],
        Some(owner_id),
        "FM30-12: owner should still be on mic"
    );
}

/// FM30-13: 普通用户 ForceTakeMic/ForceLeaveMic → 40301
#[tokio::test]
async fn fm30_13_regular_user_force_mic_returns_40301() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let regular_user_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service_take = make_room_service(room.clone());
    let room_service_leave = make_room_service(room);

    let room_manager_take = Arc::new(RoomManager::new());
    room_manager_take.get_or_create_room(room_id);

    let room_manager_leave = Arc::new(RoomManager::new());
    let room_state = room_manager_leave.get_or_create_room(room_id);
    room_state.take_mic_slot(0, target_id).unwrap();

    let registry = Arc::new(ConnectionRegistry::new());
    let mute_redis = Arc::new(FakeMuteRedis::default());

    // ForceTakeMic by regular user → 40301
    let take_deps = make_force_take_deps(
        &room_manager_take,
        &room_service_take,
        mute_redis,
        &registry,
    );
    let resp_take = handle_force_take_mic(
        take_mic_payload(room_id, target_id, 3),
        Some("msg-13a".to_string()),
        regular_user_id, // operator = regular user
        &take_deps,
    )
    .await;

    let vt: serde_json::Value = serde_json::from_str(&resp_take).unwrap();
    assert_eq!(
        vt["code"], 40301,
        "FM30-13: regular user ForceTakeMic should return 40301"
    );

    // ForceLeaveMic by regular user → 40301
    let leave_deps = make_force_leave_deps(&room_manager_leave, &room_service_leave, &registry);
    let resp_leave = handle_force_leave_mic(
        leave_mic_payload(room_id, target_id),
        Some("msg-13b".to_string()),
        regular_user_id, // operator = regular user
        &leave_deps,
    )
    .await;

    let vl: serde_json::Value = serde_json::from_str(&resp_leave).unwrap();
    assert_eq!(
        vl["code"], 40301,
        "FM30-13: regular user ForceLeaveMic should return 40301"
    );
}
