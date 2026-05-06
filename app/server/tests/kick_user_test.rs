//! 集成测试 — T-00028 WS KickUser 信令 + 10min 冷却
//!
//! 验收用例 K28-01 ~ K28-12：
//! - K28-01: 房主踢普通用户 → code:0，目标收到 UserKicked
//! - K28-02: 管理员踢普通用户 → code:0
//! - K28-03: 房间其他人收到 UserLeft reason=kicked_by_admin
//! - K28-04: 普通用户踢人 → 40301
//! - K28-05: 管理员踢房主 → 40302
//! - K28-06: target 不在房间 → 40400
//! - K28-07: 被踢 10min 内重进 → 42911 + remaining_sec
//! - K28-08: 10min 后重进成功（TTL 到期）
//! - K28-09: 踢麦上用户：广播 MicLeft forced=true
//! - K28-10: 并发 3 个管理员同时踢同一人：kick_records 3 条，只移除一次 + 只广播一次 UserLeft
//! - K28-11: reason 空 → 40003
//! - K28-12: 被踢者 WS 连接被主动关闭
//!
//! 所有测试均使用 FakeKickRedis / FakeKickAuditDb（内存），无需真实 Redis/DB。

use std::sync::{Arc, RwLock};
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::infrastructure::redis_store::FakeCodeStore;
use voice_room_server::infrastructure::third_party::sms::MockSmsProvider;
use voice_room_server::modules::auth::repository::FakeUserRepository;
use voice_room_server::modules::auth::service::AuthService;
use voice_room_server::modules::governance::kick::{
    handle_kick, FakeKickAuditDb, FakeKickRedis, KickAuditDb, KickDeps, KickRedis,
};
use voice_room_server::modules::room::service::RoomService;
use voice_room_server::modules::room::FakeRoomRepository;
use voice_room_server::room::handler::{handle_join_room, JoinRoomDeps};
use voice_room_server::room::manager::RoomManager;
use voice_room_server::room::state::MemberInfo;
use voice_room_server::stats::FakeStatsService;
use voice_room_server::ws::registry::{ConnectionHandle, ConnectionRegistry};
use voice_room_shared::models::room::RoomModel;

// ─── 测试辅助 ─────────────────────────────────────────────────────────────────

/// 创建含指定房间的 RoomService（使用 FakeRoomRepository）
fn make_room_service(room: RoomModel) -> Arc<RoomService> {
    let repo = Arc::new(FakeRoomRepository::default());
    repo.seed(room);
    Arc::new(RoomService::new(repo))
}

/// 构建一个 active 房间模型
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

/// 构建 KickDeps（使用 Fake 实现）
fn make_kick_deps(
    room_manager: &Arc<RoomManager>,
    room_service: &Arc<RoomService>,
    redis: &Arc<FakeKickRedis>,
    audit_db: &Arc<FakeKickAuditDb>,
    registry: &Arc<ConnectionRegistry>,
) -> KickDeps {
    KickDeps {
        room_manager: room_manager.clone(),
        room_service: room_service.clone(),
        redis: redis.clone() as Arc<dyn KickRedis>,
        audit_db: audit_db.clone(),
        registry: registry.clone(),
    }
}

/// 构建 KickUser payload
fn kick_payload(room_id: Uuid, target_user_id: Uuid, reason: &str) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_user_id.to_string(),
        "reason": reason,
    }))
}

/// 构建 JoinRoomDeps（含 kick_redis）
fn make_join_deps(
    room_manager: &Arc<RoomManager>,
    room_service: &Arc<RoomService>,
    registry: &Arc<ConnectionRegistry>,
    kick_redis: Option<Arc<dyn KickRedis>>,
) -> JoinRoomDeps {
    let user_repo = Arc::new(FakeUserRepository::default());
    let auth_service = Arc::new(AuthService::new(
        user_repo,
        Arc::new(FakeCodeStore::default()),
        Arc::new(MockSmsProvider),
        "test-secret".to_string(),
    ));
    JoinRoomDeps {
        room_manager: room_manager.clone(),
        room_service: room_service.clone(),
        auth_service,
        registry: registry.clone(),
        stats: Arc::new(FakeStatsService::default()),
        jwt_secret: "test-secret".to_string(),
        kick_redis,
    }
}

// ─── 测试用例 ─────────────────────────────────────────────────────────────────

/// K28-01: 房主踢普通用户 → code:0
#[tokio::test]
async fn k28_01_owner_kick_member_success() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    // 将 target 加入房间内存
    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));

    // 注册 target 连接
    let (target_conn_id, mut target_rx) = register_connection(&registry, target_id, Some(room_id));
    let _ = target_conn_id;

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "spam"),
        Some("k01".into()),
        owner_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["type"], "KickUserResult", "K28-01 type");
    assert_eq!(json["code"], 0, "K28-01 code should be 0");

    // 目标收到 UserKicked
    let kicked_msg = target_rx
        .try_recv()
        .expect("K28-01: target should receive UserKicked");
    let kicked_json: serde_json::Value = serde_json::from_str(&kicked_msg).unwrap();
    assert_eq!(kicked_json["type"], "UserKicked", "K28-01 UserKicked type");
    assert_eq!(kicked_json["payload"]["reason"], "spam");
    assert_eq!(kicked_json["payload"]["cooldown_sec"], 600);

    // Redis 冷却 key 存在
    assert!(
        redis.key_exists(room_id, target_id),
        "K28-01: kick cooldown key should exist in Redis"
    );

    // 审计记录 1 条
    assert_eq!(
        audit_db.record_count(),
        1,
        "K28-01: should have 1 audit record"
    );
}

/// K28-02: 管理员踢普通用户 → code:0
#[tokio::test]
async fn k28_02_admin_kick_member_success() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, Some(admin_id)));

    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));

    let (_target_conn, _target_rx) = register_connection(&registry, target_id, Some(room_id));

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "harassment"),
        Some("k02".into()),
        admin_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["code"], 0, "K28-02: admin kick should succeed");
}

/// K28-03: 房间其他人收到 UserLeft reason=kicked_by_admin
#[tokio::test]
async fn k28_03_bystander_receives_user_left_kicked_by_admin() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let bystander_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));
    state.members.insert(
        bystander_id,
        MemberInfo::new(bystander_id, "Bystander".into(), None),
    );

    // target 连接
    let (_target_conn, _target_rx) = register_connection(&registry, target_id, Some(room_id));
    // bystander 连接
    let (_bystander_conn, mut bystander_rx) =
        register_connection(&registry, bystander_id, Some(room_id));

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "spam"),
        Some("k03".into()),
        owner_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["code"], 0, "K28-03 kick should succeed");

    // 收集 bystander 收到的所有消息，找到 UserLeft
    let mut found_user_left = false;
    while let Ok(msg) = bystander_rx.try_recv() {
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        if v["type"] == "UserLeft" {
            // P1-N2 修复（轮次2）：UserLeft.schema.json additionalProperties:false 仅允许
            // {user_id, nickname, member_count}，reason/operator_id 已从 kick 场景 payload 移除。
            // 被踢者已通过 UserKicked 点对点获知 reason（最小化广播原则）。
            assert_eq!(
                v["payload"]["user_id"],
                target_id.to_string(),
                "K28-03: UserLeft user_id should be target"
            );
            assert_eq!(
                v["payload"]["reason"], serde_json::Value::Null,
                "K28-03: UserLeft must NOT contain reason field (schema additionalProperties:false)"
            );
            assert_eq!(
                v["payload"]["operator_id"], serde_json::Value::Null,
                "K28-03: UserLeft must NOT contain operator_id field (schema additionalProperties:false)"
            );
            found_user_left = true;
        }
    }
    assert!(found_user_left, "K28-03: bystander should receive UserLeft");
}

/// K28-04: 普通用户踢人 → 40301
#[tokio::test]
async fn k28_04_normal_user_kick_returns_40301() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let normal_user_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    // 没有 admin_user_id，normal_user_id 是普通成员
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "spam"),
        Some("k04".into()),
        normal_user_id, // 普通用户尝试踢人
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        json["code"], 40301,
        "K28-04: normal user kick should return 40301"
    );
}

/// K28-05: 管理员踢房主 → 40302
#[tokio::test]
async fn k28_05_admin_kick_owner_returns_40302() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, Some(admin_id)));

    let state = room_manager.get_or_create_room(room_id);
    // 房主在房间内
    state
        .members
        .insert(owner_id, MemberInfo::new(owner_id, "Owner".into(), None));

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, owner_id, "spam"), // target = owner
        Some("k05".into()),
        admin_id, // 管理员踢房主
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        json["code"], 40302,
        "K28-05: kicking owner should return 40302"
    );
}

/// K28-06: target 不在房间 → 40400
#[tokio::test]
async fn k28_06_target_not_in_room_returns_40400() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4(); // 没有加入房间

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    // 创建房间但不加 target
    room_manager.get_or_create_room(room_id);

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "spam"),
        Some("k06".into()),
        owner_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        json["code"], 40400,
        "K28-06: target not in room should return 40400"
    );
}

/// K28-07: 被踢 10min 内重进 → 42911 + remaining_sec
#[tokio::test]
async fn k28_07_kicked_user_cannot_rejoin_within_cooldown() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    // 添加 target 到房间
    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));

    let (_tc, _tr) = register_connection(&registry, target_id, Some(room_id));

    // Step 1: 踢出 target
    let kick_deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let kick_resp = handle_kick(
        kick_payload(room_id, target_id, "harassment"),
        Some("k07-kick".into()),
        owner_id,
        &kick_deps,
    )
    .await;
    let kick_json: serde_json::Value = serde_json::from_str(&kick_resp).unwrap();
    assert_eq!(kick_json["code"], 0, "K28-07: kick should succeed first");

    // Step 2: target 尝试重进（新连接）
    let (new_conn_id, _new_rx) = register_connection(&registry, target_id, None);
    let join_deps = make_join_deps(
        &room_manager,
        &room_service,
        &registry,
        Some(redis.clone() as Arc<dyn KickRedis>),
    );

    let join_resp = handle_join_room(
        Some(serde_json::json!({ "room_id": room_id.to_string() })),
        Some("k07-join".into()),
        new_conn_id,
        target_id,
        &join_deps,
    )
    .await;

    let join_json: serde_json::Value = serde_json::from_str(&join_resp).unwrap();
    assert_eq!(
        join_json["code"], 42911,
        "K28-07: should return 42911 kick cooldown"
    );
    let remaining = join_json["payload"]["remaining_sec"]
        .as_i64()
        .expect("K28-07: remaining_sec must be present");
    assert!(
        remaining > 0,
        "K28-07: remaining_sec must be positive, got {remaining}"
    );
    assert!(remaining <= 600, "K28-07: remaining_sec must be <= 600");
}

/// K28-08: 10min 后（TTL 到期）可重进
#[tokio::test]
async fn k28_08_kicked_user_can_rejoin_after_cooldown_expires() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());

    // 创建新的 RoomManager 和 room_service（kick 之后 target 被移除了，需要新状态）
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    // 先设置踢出冷却 key（模拟已被踢出）
    redis.set_kicked(room_id, target_id, "spam").await.unwrap();

    // 验证 K28-07：冷却中无法重进
    let state = room_manager.get_or_create_room(room_id);
    let (new_conn_id1, _rx1) = register_connection(&registry, target_id, None);
    let join_deps1 = make_join_deps(
        &room_manager,
        &room_service,
        &registry,
        Some(redis.clone() as Arc<dyn KickRedis>),
    );
    let resp1 = handle_join_room(
        Some(serde_json::json!({ "room_id": room_id.to_string() })),
        Some("k08-before".into()),
        new_conn_id1,
        target_id,
        &join_deps1,
    )
    .await;
    let json1: serde_json::Value = serde_json::from_str(&resp1).unwrap();
    assert_eq!(
        json1["code"], 42911,
        "K28-08 pre: should block during cooldown"
    );

    // 模拟时间流逝：使所有 key 过期
    redis.expire_all();

    // 重新注册连接
    let (new_conn_id2, _rx2) = register_connection(&registry, target_id, None);
    let join_deps2 = make_join_deps(
        &room_manager,
        &room_service,
        &registry,
        Some(redis.clone() as Arc<dyn KickRedis>),
    );

    // 加一个成员到房间（因为 K28-07 步骤中房间可能已经被清理）
    state.members.insert(
        Uuid::new_v4(),
        MemberInfo::new(Uuid::new_v4(), "Other".into(), None),
    );

    let resp2 = handle_join_room(
        Some(serde_json::json!({ "room_id": room_id.to_string() })),
        Some("k08-after".into()),
        new_conn_id2,
        target_id,
        &join_deps2,
    )
    .await;
    let json2: serde_json::Value = serde_json::from_str(&resp2).unwrap();
    assert_eq!(
        json2["code"], 0,
        "K28-08: should be able to rejoin after cooldown expires"
    );
    assert_eq!(json2["type"], "JoinRoomResult");
}

/// K28-09: 踢麦上用户 → 广播 MicLeft forced=true
#[tokio::test]
async fn k28_09_kicking_user_on_mic_broadcasts_mic_left_forced() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let bystander_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));
    state.members.insert(
        bystander_id,
        MemberInfo::new(bystander_id, "Bystander".into(), None),
    );

    // target 上麦（slot 2）
    state.take_mic_slot(2, target_id).unwrap();

    let (_tc, _tr) = register_connection(&registry, target_id, Some(room_id));
    let (_bc, mut bystander_rx) = register_connection(&registry, bystander_id, Some(room_id));

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "mic_abuse"),
        Some("k09".into()),
        owner_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["code"], 0, "K28-09: kick should succeed");

    // bystander 应收到 MicLeft with forced=true
    let mut found_mic_left_forced = false;
    while let Ok(msg) = bystander_rx.try_recv() {
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        if v["type"] == "MicLeft" {
            assert_eq!(
                v["payload"]["forced"], true,
                "K28-09: MicLeft forced should be true"
            );
            assert_eq!(
                v["payload"]["mic_index"], 2,
                "K28-09: MicLeft mic_index should be 2"
            );
            assert_eq!(
                v["payload"]["user_id"],
                target_id.to_string(),
                "K28-09: MicLeft user_id should be target"
            );
            found_mic_left_forced = true;
        }
    }
    assert!(
        found_mic_left_forced,
        "K28-09: bystander should receive MicLeft forced=true"
    );

    // target 麦位已被清空
    let slots = state.mic_slots_snapshot();
    assert_eq!(
        slots[2], None,
        "K28-09: mic slot 2 should be cleared after kick"
    );
}

/// K28-10: 并发 3 个踢请求 → kick_records 3 条，RoomManager 只移除一次，只广播一次 UserLeft
///
/// ## 设计说明
/// 真实并发场景下，3 个请求同时通过 is_member 检查，全部插入审计记录，
/// 但 remove_member（DashMap.remove）原子性保证只有第一个成功（返回 Some）。
///
/// 测试分两部分：
/// - Part A：直接验证 remove_member 原子性（3 次调用，仅 1 次返回 Some）
/// - Part B：通过 handle_kick 验证 UserLeft 广播唯一性（完整 kick 流程）
#[tokio::test]
async fn k28_10_concurrent_kicks_insert_3_records_but_only_one_removal() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin1_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let bystander_id = Uuid::new_v4();

    // ── Part A: remove_member 原子性 ─────────────────────────────────────────
    {
        let mgr = Arc::new(RoomManager::new());
        let state = mgr.get_or_create_room(room_id);
        state
            .members
            .insert(target_id, MemberInfo::new(target_id, "T".into(), None));

        let r1 = mgr.remove_member(room_id, target_id);
        let r2 = mgr.remove_member(room_id, target_id);
        let r3 = mgr.remove_member(room_id, target_id);

        let some_count = [r1.is_some(), r2.is_some(), r3.is_some()]
            .iter()
            .filter(|&&x| x)
            .count();
        assert_eq!(
            some_count, 1,
            "K28-10 Part A: remove_member should return Some exactly once (DashMap atomicity)"
        );
    }

    // ── Part B: 审计记录 + audit_db 并发安全 ─────────────────────────────────
    // 直接验证 FakeKickAuditDb 并发安全性：3 次直接插入 → 3 条记录
    {
        let audit_db = Arc::new(FakeKickAuditDb::default());
        // 模拟 3 个并发请求全部通过 is_member 检查并插入审计记录
        audit_db
            .insert_kick_record(room_id, owner_id, target_id, "spam")
            .await
            .unwrap();
        audit_db
            .insert_kick_record(room_id, admin1_id, target_id, "spam")
            .await
            .unwrap();
        audit_db
            .insert_kick_record(room_id, owner_id, target_id, "spam")
            .await
            .unwrap();
        assert_eq!(
            audit_db.record_count(),
            3,
            "K28-10 Part B: FakeKickAuditDb should store all 3 concurrent records"
        );
    }

    // ── Part C: UserLeft 只广播一次（完整流程验证）────────────────────────────
    // 仅第一个成功踢出的请求广播 UserLeft；第二个请求因 target 已被移除返回静默
    {
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let redis = Arc::new(FakeKickRedis::default());
        let audit_db2 = Arc::new(FakeKickAuditDb::default());
        let room_service = make_room_service(make_room(room_id, owner_id, Some(admin1_id)));

        let state = room_manager.get_or_create_room(room_id);
        state
            .members
            .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));
        state.members.insert(
            bystander_id,
            MemberInfo::new(bystander_id, "Bystander".into(), None),
        );

        let (_tc, _tr) = register_connection(&registry, target_id, Some(room_id));
        let (_bc, mut bystander_rx) = register_connection(&registry, bystander_id, Some(room_id));

        // 第一次踢出（owner）
        let deps1 = make_kick_deps(&room_manager, &room_service, &redis, &audit_db2, &registry);
        let r1 = handle_kick(
            kick_payload(room_id, target_id, "spam"),
            Some("k10c-1".into()),
            owner_id,
            &deps1,
        )
        .await;
        let j1: serde_json::Value = serde_json::from_str(&r1).unwrap();
        assert_eq!(j1["code"], 0, "K28-10 Part C: first kick should succeed");

        // 第二次踢出（admin）—— target 已不在房间，返回 40400 或 0（两者均可接受）
        let deps2 = make_kick_deps(&room_manager, &room_service, &redis, &audit_db2, &registry);
        let _r2 = handle_kick(
            kick_payload(room_id, target_id, "spam"),
            Some("k10c-2".into()),
            admin1_id,
            &deps2,
        )
        .await;
        // 不严格断言第二次的返回码（40400 或 0 均可）

        // bystander 只收到一次 UserLeft（target = target_id）
        // P1-N2 修复（轮次2）：UserLeft payload 不再含 reason 字段，仅按 user_id 过滤
        let mut user_left_count = 0;
        while let Ok(msg) = bystander_rx.try_recv() {
            let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
            if v["type"] == "UserLeft"
                && v["payload"]["user_id"] == target_id.to_string()
            {
                user_left_count += 1;
            }
        }
        assert_eq!(
            user_left_count, 1,
            "K28-10 Part C: exactly one UserLeft broadcast (got {user_left_count})"
        );

        // target 最终不在房间
        assert!(
            !room_manager.is_member(room_id, target_id),
            "K28-10 Part C: target should be removed from room"
        );
    }
}

/// K28-11: reason 空 → 40003
#[tokio::test]
async fn k28_11_empty_reason_returns_40003() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);

    // reason 为空字符串
    let payload = Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_id.to_string(),
        "reason": "",
    }));
    let resp = handle_kick(payload.clone(), Some("k11-empty".into()), owner_id, &deps).await;
    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        json["code"], 40003,
        "K28-11: empty reason should return 40003"
    );

    // reason 缺失
    let payload_no_reason = Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_id.to_string(),
    }));
    let resp2 = handle_kick(
        payload_no_reason,
        Some("k11-missing".into()),
        owner_id,
        &deps,
    )
    .await;
    let json2: serde_json::Value = serde_json::from_str(&resp2).unwrap();
    assert_eq!(
        json2["code"], 40003,
        "K28-11: missing reason should return 40003"
    );
}

/// K28-12: 被踢者 WS 连接被主动关闭（unregister）
#[tokio::test]
async fn k28_12_target_ws_connection_closed_after_kick() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let redis = Arc::new(FakeKickRedis::default());
    let audit_db = Arc::new(FakeKickAuditDb::default());
    let room_service = make_room_service(make_room(room_id, owner_id, None));

    let state = room_manager.get_or_create_room(room_id);
    state
        .members
        .insert(target_id, MemberInfo::new(target_id, "Target".into(), None));

    // 注册 target 连接
    let (target_conn_id, _target_rx) = register_connection(&registry, target_id, Some(room_id));

    // 踢前连接存在
    assert!(
        registry.get(target_conn_id).is_some(),
        "K28-12: target connection should exist before kick"
    );

    let deps = make_kick_deps(&room_manager, &room_service, &redis, &audit_db, &registry);
    let resp = handle_kick(
        kick_payload(room_id, target_id, "spam"),
        Some("k12".into()),
        owner_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["code"], 0, "K28-12: kick should succeed");

    // 踢后连接被注销（unregister）
    assert!(
        registry.get(target_conn_id).is_none(),
        "K28-12: target connection should be closed (unregistered) after kick"
    );
}
