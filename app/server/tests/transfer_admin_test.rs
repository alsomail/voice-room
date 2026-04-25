//! 集成测试 — T-00030 WS TransferAdmin 信令
//!
//! 验收用例：
//! - TA30-01: 房主 assign 新管理员 → rooms.admin_user_id 更新 + AdminChanged 广播
//! - TA30-02: 已有管理员再 assign 另一人 → previous_admin_id 非 null
//! - TA30-03: 管理员尝试 TransferAdmin → 40301
//! - TA30-04: assign 房主自己 → 40302
//! - TA30-05: revoke 非当前管理员 → 40404
//! - TA30-06: AdminChanged 广播给房间所有成员
//! - TA30-14: TransferAdmin 原子：DB 失败时不广播

use std::sync::{Arc, RwLock};
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::modules::governance::transfer::{
    handle_transfer_admin, FailingTransferAdminRepo, FakeTransferAdminRepo, TransferAdminDeps,
    TransferAdminRepo,
};
use voice_room_server::modules::room::service::RoomService;
use voice_room_server::modules::room::FakeRoomRepository;
use voice_room_server::room::manager::RoomManager;
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

/// 构建 TransferAdminDeps（使用 Fake 实现）
fn make_deps(
    room_service: &Arc<RoomService>,
    room_repo: Arc<dyn TransferAdminRepo>,
    registry: &Arc<ConnectionRegistry>,
) -> TransferAdminDeps {
    TransferAdminDeps {
        room_manager: Arc::new(RoomManager::new()),
        room_service: room_service.clone(),
        room_repo,
        registry: registry.clone(),
    }
}

/// 构建 TransferAdmin payload
fn transfer_payload(
    room_id: Uuid,
    target_user_id: Uuid,
    action: &str,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "target_user_id": target_user_id.to_string(),
        "action": action,
    }))
}

// ─── 测试用例 ─────────────────────────────────────────────────────────────────

/// TA30-01: 房主 assign 新管理员 → DB 更新 + 广播 AdminChanged
#[tokio::test]
async fn ta30_01_owner_assign_new_admin_success() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let repo = Arc::new(FakeTransferAdminRepo::default());
    let registry = Arc::new(ConnectionRegistry::new());

    // 注册一个房间成员连接（用来接收广播）
    let (_conn, mut rx) = register_connection(&registry, owner_id, Some(room_id));

    let deps = make_deps(&room_service, repo.clone(), &registry);

    let resp = handle_transfer_admin(
        transfer_payload(room_id, target_id, "assign"),
        Some("msg-1".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["type"], "TransferAdminResult",
        "type should be TransferAdminResult"
    );
    assert_eq!(v["code"], 0, "code should be 0 for success");

    // 验证 DB 更新
    let stored = repo.get_admin(room_id);
    assert_eq!(
        stored,
        Some(Some(target_id)),
        "admin_user_id should be updated to target"
    );

    // 验证广播
    let broadcast = rx
        .try_recv()
        .expect("TA30-01: should have received AdminChanged broadcast");
    let bv: serde_json::Value = serde_json::from_str(&broadcast).unwrap();
    assert_eq!(
        bv["type"], "AdminChanged",
        "broadcast type should be AdminChanged"
    );
    assert_eq!(
        bv["payload"]["admin_user_id"],
        target_id.to_string(),
        "admin_user_id in broadcast should be target"
    );
    assert_eq!(
        bv["payload"]["operator_id"],
        owner_id.to_string(),
        "operator_id should be owner"
    );
}

/// TA30-02: 已有管理员再 assign 另一人 → previous_admin_id 非 null
#[tokio::test]
async fn ta30_02_assign_when_existing_admin_shows_previous() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let old_admin_id = Uuid::new_v4();
    let new_admin_id = Uuid::new_v4();

    // 房间已有管理员 old_admin_id
    let room = make_room(room_id, owner_id, Some(old_admin_id));
    let room_service = make_room_service(room);
    let repo = Arc::new(FakeTransferAdminRepo::default());
    let registry = Arc::new(ConnectionRegistry::new());

    let (_conn, mut rx) = register_connection(&registry, owner_id, Some(room_id));

    let deps = make_deps(&room_service, repo.clone(), &registry);

    let resp = handle_transfer_admin(
        transfer_payload(room_id, new_admin_id, "assign"),
        Some("msg-2".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["code"], 0, "TA30-02: should succeed");

    // previous_admin_id 应为旧管理员
    let broadcast = rx.try_recv().expect("TA30-02: should receive AdminChanged");
    let bv: serde_json::Value = serde_json::from_str(&broadcast).unwrap();
    assert_eq!(bv["type"], "AdminChanged");
    assert_eq!(
        bv["payload"]["previous_admin_id"],
        old_admin_id.to_string(),
        "TA30-02: previous_admin_id should be old admin"
    );
    assert_eq!(
        bv["payload"]["admin_user_id"],
        new_admin_id.to_string(),
        "TA30-02: admin_user_id should be new admin"
    );
}

/// TA30-03: 管理员（非房主）尝试 TransferAdmin → 40301
#[tokio::test]
async fn ta30_03_admin_cannot_transfer() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, Some(admin_id));
    let room_service = make_room_service(room);
    let repo = Arc::new(FakeTransferAdminRepo::default());
    let registry = Arc::new(ConnectionRegistry::new());

    let deps = make_deps(&room_service, repo.clone(), &registry);

    // 管理员（不是房主）操作
    let resp = handle_transfer_admin(
        transfer_payload(room_id, target_id, "assign"),
        Some("msg-3".to_string()),
        admin_id, // operator = admin, not owner
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["code"], 40301, "TA30-03: admin should get 40301");

    // DB 不应该被更新
    assert!(
        repo.get_admin(room_id).is_none(),
        "TA30-03: DB should not be updated"
    );
}

/// TA30-04: 房主 assign 自己 → 40302
#[tokio::test]
async fn ta30_04_cannot_assign_owner_as_admin() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let repo = Arc::new(FakeTransferAdminRepo::default());
    let registry = Arc::new(ConnectionRegistry::new());

    let deps = make_deps(&room_service, repo.clone(), &registry);

    // owner 尝试把自己设为 admin
    let resp = handle_transfer_admin(
        transfer_payload(room_id, owner_id, "assign"),
        Some("msg-4".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["code"], 40302,
        "TA30-04: assigning owner as admin should get 40302"
    );
}

/// TA30-05: revoke 非当前管理员 → 40404
#[tokio::test]
async fn ta30_05_revoke_non_current_admin_returns_40404() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let current_admin_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4(); // 不是管理员

    let room = make_room(room_id, owner_id, Some(current_admin_id));
    let room_service = make_room_service(room);
    let repo = Arc::new(FakeTransferAdminRepo::default());
    let registry = Arc::new(ConnectionRegistry::new());

    let deps = make_deps(&room_service, repo.clone(), &registry);

    // 尝试 revoke 一个不是当前管理员的用户
    let resp = handle_transfer_admin(
        transfer_payload(room_id, other_user_id, "revoke"),
        Some("msg-5".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(
        v["code"], 40404,
        "TA30-05: revoke non-admin should get 40404"
    );
}

/// TA30-06: AdminChanged 广播给房间所有成员
#[tokio::test]
async fn ta30_06_admin_changed_broadcast_to_all_room_members() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let member1_id = Uuid::new_v4();
    let member2_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    let repo = Arc::new(FakeTransferAdminRepo::default());
    let registry = Arc::new(ConnectionRegistry::new());

    // 注册房间内三个连接（owner + 2 members）
    let (_, mut rx_owner) = register_connection(&registry, owner_id, Some(room_id));
    let (_, mut rx_m1) = register_connection(&registry, member1_id, Some(room_id));
    let (_, mut rx_m2) = register_connection(&registry, member2_id, Some(room_id));

    let deps = make_deps(&room_service, repo.clone(), &registry);

    let resp = handle_transfer_admin(
        transfer_payload(room_id, target_id, "assign"),
        Some("msg-6".to_string()),
        owner_id,
        &deps,
    )
    .await;

    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["code"], 0, "TA30-06: should succeed");

    // 所有三个连接都应收到广播
    for (name, rx) in [
        ("owner", &mut rx_owner),
        ("member1", &mut rx_m1),
        ("member2", &mut rx_m2),
    ] {
        let msg = rx
            .try_recv()
            .unwrap_or_else(|_| panic!("TA30-06: {name} should receive AdminChanged"));
        let bv: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            bv["type"], "AdminChanged",
            "TA30-06: {name} should receive AdminChanged"
        );
    }
}

/// TA30-14: TransferAdmin 原子性 — DB 失败时不广播
#[tokio::test]
async fn ta30_14_db_failure_does_not_broadcast() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let room = make_room(room_id, owner_id, None);
    let room_service = make_room_service(room);
    // 使用总是失败的 repo
    let failing_repo = Arc::new(FailingTransferAdminRepo);
    let registry = Arc::new(ConnectionRegistry::new());

    let (_, mut rx_owner) = register_connection(&registry, owner_id, Some(room_id));

    let deps = make_deps(&room_service, failing_repo, &registry);

    let resp = handle_transfer_admin(
        transfer_payload(room_id, target_id, "assign"),
        Some("msg-14".to_string()),
        owner_id,
        &deps,
    )
    .await;

    // DB 失败应返回 50000
    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["code"], 50000, "TA30-14: DB failure should return 50000");

    // 不应广播 AdminChanged
    assert!(
        rx_owner.try_recv().is_err(),
        "TA30-14: should NOT broadcast when DB fails"
    );
}
