//! Room signaling handlers (P2-9 split)
//!
//! 本目录将原 2660 行的 `room/handler.rs` 按业务边界拆分为 3 个子模块，
//! 每个文件控制在 350 行以内：
//!
//! - `lifecycle.rs` — JoinRoom / LeaveRoom（含密码房 token / 踢出冷却 / 重连续传）
//! - `mic.rs`       — TakeMic / LeaveMic + 共享 `broadcast_mic_left`
//! - `chat.rs`      — SendMessage（含禁言前置 / 敏感词过滤 / 幂等去重）
//!
//! 通过 `pub use` 重新导出全部公共 API，外部调用方（`ws::connection`、
//! `modules::governance::*`、集成测试等）的导入路径保持不变。

mod chat;
mod lifecycle;
mod mic;

// 重新导出全部公共 API，保持 `crate::room::handler::*` 路径稳定
pub use chat::{handle_send_message, SendMessageDeps};
pub use lifecycle::{
    do_leave_room, handle_join_room, handle_leave_room, JoinRoomDeps, LeaveRoomDeps,
};
pub(crate) use mic::broadcast_mic_left;
pub use mic::{handle_leave_mic, handle_take_mic, LeaveMicDeps, TakeMicDeps};

// ─── 测试辅助：让 tests 模块能通过 `use super::*;` 看到关键类型 ──────────
// （这些类型本身不构成 handler 的对外 API，仅为内部测试代码兼容老路径）
#[cfg(test)]
use crate::modules::auth::service::AuthService;
#[cfg(test)]
use crate::modules::room::service::RoomService;
#[cfg(test)]
use crate::room::manager::RoomManager;
#[cfg(test)]
use crate::room::state::MemberInfo;
#[cfg(test)]
use crate::stats::StatsPort;
#[cfg(test)]
use crate::ws::registry::ConnectionRegistry;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use uuid::Uuid;

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    use chrono::Utc;
    use tokio::sync::mpsc;
    use uuid::Uuid;
    use voice_room_shared::models::room::RoomModel;
    use voice_room_shared::models::user::UserModel;

    use crate::infrastructure::redis_store::FakeCodeStore;
    use crate::infrastructure::third_party::sms::MockSmsProvider;
    use crate::modules::auth::repository::{FailingUserRepository, FakeUserRepository};
    use crate::modules::room::repository::FakeRoomRepository;
    use crate::stats::FakeStatsService;
    use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    /// 创建空 RoomService（无任何房间）— 模拟"房间不存在"场景
    fn empty_room_service() -> Arc<RoomService> {
        Arc::new(RoomService::new(Arc::new(FakeRoomRepository::default())))
    }

    /// 创建含指定房间的 RoomService
    fn room_service_with(room_id: Uuid) -> Arc<RoomService> {
        let repo = Arc::new(FakeRoomRepository::default());
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id: Uuid::new_v4(),
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
            admin_user_id: None,
        });
        Arc::new(RoomService::new(repo))
    }

    /// 创建含指定用户的 AuthService
    fn auth_service_with(user_id: Uuid, nickname: &str) -> Arc<AuthService> {
        let user_repo = Arc::new(FakeUserRepository::default());
        let now = Utc::now();
        user_repo.seed(UserModel {
            id: user_id,
            phone: "+8613800000000".to_string(),
            nickname: nickname.to_string(),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            coin_balance: 0,
            diamond_balance: 0,
            charm_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        });
        Arc::new(AuthService::new(
            user_repo,
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        ))
    }

    /// 创建空 AuthService（无任何用户）
    fn empty_auth_service() -> Arc<AuthService> {
        Arc::new(AuthService::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        ))
    }

    /// 创建总是返回 DB 错误的 AuthService（用于 I-02 测试）
    fn failing_auth_service() -> Arc<AuthService> {
        Arc::new(AuthService::new(
            Arc::new(FailingUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        ))
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

    /// 构建 JoinRoom payload
    fn join_payload(room_id: Uuid) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "room_id": room_id.to_string() }))
    }

    /// 构建 JoinRoomDeps（测试辅助，减少重复代码）
    fn build_deps(
        room_manager: &Arc<RoomManager>,
        room_service: &Arc<RoomService>,
        auth_service: &Arc<AuthService>,
        registry: &Arc<ConnectionRegistry>,
        stats: &Arc<dyn StatsPort>,
    ) -> JoinRoomDeps {
        JoinRoomDeps {
            room_manager: room_manager.clone(),
            room_service: room_service.clone(),
            auth_service: auth_service.clone(),
            registry: registry.clone(),
            stats: stats.clone(),
            jwt_secret: "test-secret".to_string(),
            kick_redis: None,
        }
    }

    // J03: FakeRoomService 返回 None → 响应 code=40400
    #[tokio::test]
    async fn j03_room_not_found_returns_40400() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = empty_room_service(); // 无房间，get_active_room_detail 返回 None
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-j03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40400,
            "should return code 40400 when room not found"
        );
        assert_eq!(json["type"], "JoinRoomResult");
        assert_eq!(json["msg_id"], "msg-j03");
    }

    // J04: 成功加入 → members 包含 user_id
    #[tokio::test]
    async fn j04_success_members_contains_user_id() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Alice");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        handle_join_room(join_payload(room_id), None, conn_id, user_id, &deps).await;

        let room_state = room_manager
            .get_room(room_id)
            .expect("room should exist in manager");
        assert!(
            room_state.members.contains_key(&user_id),
            "members should contain user_id after successful join"
        );
    }

    // J05: 成功加入 → registry.get_connections_in_room 包含该连接
    #[tokio::test]
    async fn j05_success_registry_contains_connection() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Bob");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        handle_join_room(join_payload(room_id), None, conn_id, user_id, &deps).await;

        let conns = registry.get_connections_in_room(room_id);
        assert!(
            conns.iter().any(|(cid, _)| *cid == conn_id),
            "get_connections_in_room should include the newly joined connection"
        );
    }

    // J06: 成功加入 → FakeStatsService.active_rooms 包含 room_id
    #[tokio::test]
    async fn j06_success_stats_active_rooms_contains_room_id() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Carol");
        let registry = Arc::new(ConnectionRegistry::new());
        let fake_stats = Arc::new(FakeStatsService::default());
        let stats: Arc<dyn StatsPort> = fake_stats.clone();

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        handle_join_room(join_payload(room_id), None, conn_id, user_id, &deps).await;

        assert!(
            fake_stats.active_rooms.lock().unwrap().contains(&room_id),
            "FakeStatsService.active_rooms should contain room_id after join"
        );
    }

    // J07: 成功加入 → 已有连接的 rx 收到 UserJoined
    #[tokio::test]
    async fn j07_success_existing_connection_receives_user_joined() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4();
        let user_b_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_b_id, "UserB");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 用户 A 已在房间中（直接设置 room_id）
        let (conn_a_id, mut rx_a) = register_connection(&registry, user_a_id, Some(room_id));
        let _ = conn_a_id;

        // 用户 B 加入房间
        let (conn_b_id, _rx_b) = register_connection(&registry, user_b_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        handle_join_room(join_payload(room_id), None, conn_b_id, user_b_id, &deps).await;

        // 用户 A 的 rx 应该收到 UserJoined 消息
        let msg = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("rx_a should not timeout")
            .expect("channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["type"], "UserJoined",
            "existing connection should receive UserJoined"
        );
    }

    // J08: 成功加入 → 响应 code=0，payload.room.member_count >= 1
    #[tokio::test]
    async fn j08_success_response_code_0_and_member_count() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Dave");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-j08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "success response should have code=0");
        assert_eq!(json["type"], "JoinRoomResult");

        let member_count = json["payload"]["room"]["member_count"]
            .as_i64()
            .expect("member_count should be i64");
        assert!(
            member_count >= 1,
            "member_count should be >= 1 after joining; got {member_count}"
        );

        // mic_slots 应为 9 个元素
        let mic_slots = json["payload"]["room"]["mic_slots"]
            .as_array()
            .expect("mic_slots should be array");
        assert_eq!(mic_slots.len(), 9, "mic_slots should have 9 elements");
    }

    // J09: 用户 B 加入 → 用户 A 收到 UserJoined(user_id=B)
    #[tokio::test]
    async fn j09_user_b_joins_user_a_receives_user_joined_with_b_user_id() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4();
        let user_b_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_b_id, "UserB");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 用户 A 已在房间
        let (_conn_a_id, mut rx_a) = register_connection(&registry, user_a_id, Some(room_id));

        // 用户 B 加入
        let (conn_b_id, _rx_b) = register_connection(&registry, user_b_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        handle_join_room(join_payload(room_id), None, conn_b_id, user_b_id, &deps).await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("rx_a should not timeout")
            .expect("channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "UserJoined");
        assert_eq!(
            json["payload"]["user_id"],
            user_b_id.to_string(),
            "UserJoined payload.user_id must be user B's ID"
        );
        assert_eq!(
            json["payload"]["nickname"], "UserB",
            "UserJoined payload.nickname must match user B's nickname"
        );
    }

    // ── LeaveRoom 测试辅助 ────────────────────────────────────────────────────

    /// 构建 LeaveRoomDeps（测试辅助）
    fn build_leave_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
        stats: &Arc<dyn StatsPort>,
    ) -> LeaveRoomDeps {
        LeaveRoomDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            stats: stats.clone(),
        }
    }

    // L01: do_leave_room 成员被移除（room_state.members.get(&user_id) 为 None）
    #[tokio::test]
    async fn l01_do_leave_room_removes_member() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 先加入：手动建立 room_state 并插入成员
        let room_state = room_manager.get_or_create_room(room_id);
        room_state
            .members
            .insert(user_id, MemberInfo::new(user_id, "Alice".to_string(), None));

        // 注册连接并设置 room_id
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        assert!(
            room_state.members.get(&user_id).is_none(),
            "L01: member should be removed from room_state.members after do_leave_room"
        );
    }

    // L02: 用户未加入房间（registry.get_room_id = None），do_leave_room 静默返回无 panic
    #[tokio::test]
    async fn l02_do_leave_room_no_room_id_silent_return() {
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let user_id = Uuid::new_v4();
        // 注册连接但不设置 room_id（room_id = None）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        // 不应 panic
        do_leave_room(conn_id, user_id, &deps).await;

        assert_eq!(
            room_manager.room_count(),
            0,
            "L02: no rooms should be created"
        );
    }

    // L03: 在麦用户离开后 mic_slots_snapshot()[slot_idx] 为 None
    #[tokio::test]
    async fn l03_do_leave_room_removes_user_from_mic_slot() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state
            .members
            .insert(user_id, MemberInfo::new(user_id, "Bob".to_string(), None));
        // 将用户放到麦位 2
        {
            let mut slots = room_state.mic_slots.write().unwrap();
            slots[2] = Some(user_id);
        }

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[2].is_none(),
            "L03: mic slot 2 should be None after on-mic user leaves"
        );
    }

    // L04: 广播 UserLeft，已有成员的 rx 收到含 user_id 的消息
    #[tokio::test]
    async fn l04_do_leave_room_broadcasts_user_left_to_remaining_members() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let leaver_id = Uuid::new_v4();
        let stayer_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            leaver_id,
            MemberInfo::new(leaver_id, "Leaver".to_string(), None),
        );
        room_state.members.insert(
            stayer_id,
            MemberInfo::new(stayer_id, "Stayer".to_string(), None),
        );

        // leaver 的连接
        let (leaver_conn, _rx_leaver) = register_connection(&registry, leaver_id, None);
        registry.set_room_id(leaver_conn, room_id);

        // stayer 的连接（已在房间）
        let (_stayer_conn, mut rx_stayer) =
            register_connection(&registry, stayer_id, Some(room_id));

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(leaver_conn, leaver_id, &deps).await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_stayer.recv())
            .await
            .expect("L04: rx_stayer should not timeout")
            .expect("L04: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["type"], "UserLeft",
            "L04: broadcast type should be UserLeft"
        );
        assert_eq!(
            json["payload"]["user_id"],
            leaver_id.to_string(),
            "L04: UserLeft payload.user_id should match leaver"
        );
    }

    // L05: 广播不含离开者本身（离开者的 rx 不收到 UserLeft）
    #[tokio::test]
    async fn l05_do_leave_room_does_not_broadcast_to_leaver() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let leaver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            leaver_id,
            MemberInfo::new(leaver_id, "Solo".to_string(), None),
        );

        let (leaver_conn, mut rx_leaver) = register_connection(&registry, leaver_id, None);
        registry.set_room_id(leaver_conn, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(leaver_conn, leaver_id, &deps).await;

        // leaver 自己的 rx 不应收到 UserLeft（100ms 超时）
        let result = tokio::time::timeout(Duration::from_millis(100), rx_leaver.recv()).await;
        assert!(
            result.is_err(),
            "L05: leaver should NOT receive UserLeft broadcast (channel should timeout)"
        );
    }

    // L06: 最后一个成员离开 room_manager.room_count() == 0
    #[tokio::test]
    async fn l06_do_leave_room_last_member_removes_room() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state
            .members
            .insert(user_id, MemberInfo::new(user_id, "Last".to_string(), None));

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        assert_eq!(room_manager.room_count(), 1);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        assert_eq!(
            room_manager.room_count(),
            0,
            "L06: room should be removed when last member leaves"
        );
    }

    // L07: FakeStatsService.active_rooms 不含 room_id（通过 user_leave_room 触发）
    #[tokio::test]
    async fn l07_do_leave_room_stats_active_rooms_does_not_contain_room_id() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let fake_stats = Arc::new(FakeStatsService::default());
        let stats: Arc<dyn StatsPort> = fake_stats.clone();

        // 预先将 room_id 加入 active_rooms（模拟 join 时的状态）
        fake_stats.active_rooms.lock().unwrap().insert(room_id);

        let room_state = room_manager.get_or_create_room(room_id);
        room_state
            .members
            .insert(user_id, MemberInfo::new(user_id, "Stat".to_string(), None));

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        assert!(
            !fake_stats.active_rooms.lock().unwrap().contains(&room_id),
            "L07: active_rooms should NOT contain room_id after user leaves"
        );
    }

    // L08: handle_leave_room 返回 JSON {"type":"LeaveRoomResult","code":0,...}
    #[tokio::test]
    async fn l08_handle_leave_room_returns_code_0_and_correct_type() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "H8User".to_string(), None),
        );

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        let response =
            handle_leave_room(Some("msg-l08".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["type"], "LeaveRoomResult",
            "L08: type should be LeaveRoomResult"
        );
        assert_eq!(
            json["code"], 0,
            "L08: code should be 0 for successful leave"
        );
        assert_eq!(
            json["msg_id"], "msg-l08",
            "L08: msg_id should be echoed back"
        );
    }

    // L10: 不在麦上的用户离开后 mic_slots_snapshot() 全部为 None
    #[tokio::test]
    async fn l10_do_leave_room_non_mic_user_slots_unchanged() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state
            .members
            .insert(user_id, MemberInfo::new(user_id, "NoMic".to_string(), None));
        // 不向任何麦位插入用户（初始全为 None）

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot.iter().all(|s| s.is_none()),
            "L10: all mic slots should remain None when a non-mic user leaves"
        );
    }

    // L11: 在麦上用户 do_leave_room 后，离开者 rx 在 100ms 内收不到 MicLeft；
    //      但旁听者能收到 MicLeft（包含正确的 mic_index 和 user_id）
    #[tokio::test]
    async fn l11_do_leave_room_mic_left_not_sent_to_leaver() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4(); // 在麦位 0
        let user_b_id = Uuid::new_v4(); // 旁听

        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_a_id,
            MemberInfo::new(user_a_id, "OnMic".to_string(), None),
        );
        room_state.members.insert(
            user_b_id,
            MemberInfo::new(user_b_id, "Listener".to_string(), None),
        );
        // user_a 在麦位 0
        {
            let mut slots = room_state.mic_slots.write().unwrap();
            slots[0] = Some(user_a_id);
        }

        // user_a 的连接（即将离开）
        let (conn_a, mut rx_a) = register_connection(&registry, user_a_id, None);
        registry.set_room_id(conn_a, room_id);

        // user_b 的连接（旁听，留在房间）
        let (_conn_b, mut rx_b) = register_connection(&registry, user_b_id, Some(room_id));

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_a, user_a_id, &deps).await;

        // 验证1：user_a（离开者）在 100ms 内收不到任何消息（含 MicLeft）
        let result_a = tokio::time::timeout(Duration::from_millis(100), rx_a.recv()).await;
        assert!(
            result_a.is_err(),
            "L11: leaver (user_a) should NOT receive MicLeft after do_leave_room"
        );

        // 验证2：user_b（旁听者）能收到 MicLeft，且内容正确
        let msg_b = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
            .await
            .expect("L11: rx_b should not timeout — listener must receive MicLeft")
            .expect("L11: rx_b channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg_b).unwrap();
        // 可能先收到 MicLeft 或 UserLeft，找到 MicLeft
        if json["type"] == "MicLeft" {
            assert_eq!(
                json["payload"]["mic_index"], 0,
                "L11: MicLeft payload.mic_index should be 0"
            );
            assert_eq!(
                json["payload"]["user_id"],
                user_a_id.to_string(),
                "L11: MicLeft payload.user_id should be user_a"
            );
        } else {
            // 第一条是 UserLeft，第二条应为 MicLeft
            let msg_b2 = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
                .await
                .expect("L11: rx_b second recv should not timeout")
                .expect("L11: rx_b second channel should not be closed");
            let json2: serde_json::Value = serde_json::from_str(&msg_b2).unwrap();
            assert_eq!(
                json2["type"], "MicLeft",
                "L11: second message to listener should be MicLeft"
            );
            assert_eq!(
                json2["payload"]["mic_index"], 0,
                "L11: MicLeft payload.mic_index should be 0"
            );
            assert_eq!(
                json2["payload"]["user_id"],
                user_a_id.to_string(),
                "L11: MicLeft payload.user_id should be user_a"
            );
        }
    }

    // ── TakeMic 测试辅助 ──────────────────────────────────────────────────────

    /// 构建 TakeMicDeps（测试辅助）
    fn build_take_mic_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
    ) -> TakeMicDeps {
        TakeMicDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            mute_redis: None,
            mic_lock: None,
        }
    }

    /// 构建 TakeMic payload
    fn take_mic_payload(mic_index: u64) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "mic_index": mic_index }))
    }

    // M01: 成功上麦，mic_slots_snapshot()[0] == Some(user_id)
    #[tokio::test]
    async fn m01_take_mic_success_slot_occupied() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        assert_eq!(
            room_state.mic_slots_snapshot()[0],
            Some(user_id),
            "M01: mic slot 0 should be occupied by user_id after successful take_mic"
        );
    }

    // M02: 麦位已被他人占用，code=40303
    #[tokio::test]
    async fn m02_take_mic_slot_occupied_returns_40303() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 预先占用 slot 0（另一个用户）
        room_state
            .take_mic_slot(0, other_user)
            .expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40303,
            "M02: occupied slot should return code 40303"
        );
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M03: 用户已在某麦位，再次上麦，code=40301
    #[tokio::test]
    async fn m03_take_mic_user_already_on_mic_returns_40301() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 用户已在 slot 1
        room_state
            .take_mic_slot(1, user_id)
            .expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        // 尝试占用 slot 0（用户已在 slot 1）
        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40301,
            "M03: user already on mic should return code 40301"
        );
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M04: 用户在 banned_mics 中，code=40302
    #[tokio::test]
    async fn m04_take_mic_user_banned_returns_40302() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 将用户加入禁麦列表
        room_state.banned_mics.insert(user_id);

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40302,
            "M04: banned user should return code 40302"
        );
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M05: 用户不在房间（get_room_id=None），code=40400
    #[tokio::test]
    async fn m05_take_mic_user_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        // 注册连接但 room_id=None（用户未进入任何房间）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40400,
            "M05: user not in room should return code 40400"
        );
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M06: mic_index=9（超出0-8），code=40002
    #[tokio::test]
    async fn m06_take_mic_index_out_of_range_returns_40002() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        // mic_index=9 超出有效范围 0-8
        let response = handle_take_mic(take_mic_payload(9), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40002,
            "M06: mic_index=9 should return code 40002"
        );
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M07: 成功上麦后，其他连接的 rx 收到含正确 user_id 和 mic_index 的 MicTaken 广播
    #[tokio::test]
    async fn m07_take_mic_broadcasts_mic_taken_to_other_connections() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let observer_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        // 观察者已在房间
        let (_obs_conn, mut rx_observer) =
            register_connection(&registry, observer_id, Some(room_id));

        // 上麦者的连接
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        handle_take_mic(take_mic_payload(3), None, conn_id, user_id, &deps).await;

        // 观察者应收到 MicTaken 广播
        let msg = tokio::time::timeout(Duration::from_millis(200), rx_observer.recv())
            .await
            .expect("M07: rx_observer should not timeout")
            .expect("M07: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["type"], "MicTaken",
            "M07: broadcast type should be MicTaken"
        );
        assert_eq!(
            json["payload"]["user_id"],
            user_id.to_string(),
            "M07: MicTaken payload.user_id should match the user who took the mic"
        );
        assert_eq!(
            json["payload"]["mic_index"], 3,
            "M07: MicTaken payload.mic_index should match the requested slot"
        );
    }

    // M08: 响应 code=0，type="TakeMicResult"，payload.mic_index 正确
    #[tokio::test]
    async fn m08_take_mic_success_response_code_0_and_correct_payload() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(
            take_mic_payload(5),
            Some("msg-m08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "M08: success response should have code=0");
        assert_eq!(
            json["type"], "TakeMicResult",
            "M08: type should be TakeMicResult"
        );
        assert_eq!(
            json["msg_id"], "msg-m08",
            "M08: msg_id should be echoed back"
        );
        assert_eq!(
            json["payload"]["mic_index"], 5,
            "M08: payload.mic_index should match the requested slot"
        );
    }

    // M09: 并发抢麦 — 两个 tokio::spawn 并发调用 take_mic_slot(0, ...)，只有一个 Ok
    #[tokio::test]
    async fn m09_concurrent_take_mic_slot_only_one_succeeds() {        use crate::room::state::TakeMicError;

        let room_id = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let room_state = Arc::new(crate::room::state::RoomState::new(room_id));

        let state_a = room_state.clone();
        let state_b = room_state.clone();

        let task_a = tokio::spawn(async move { state_a.take_mic_slot(0, user_a) });
        let task_b = tokio::spawn(async move { state_b.take_mic_slot(0, user_b) });

        let result_a = task_a.await.expect("M09: task_a should not panic");
        let result_b = task_b.await.expect("M09: task_b should not panic");

        // 恰好一个成功，另一个返回 SlotOccupied
        let successes = [result_a.is_ok(), result_b.is_ok()]
            .iter()
            .filter(|&&x| x)
            .count();
        assert_eq!(
            successes, 1,
            "M09: exactly one concurrent take_mic_slot should succeed"
        );

        // 失败者返回 SlotOccupied（而不是 AlreadyOnMic）
        let failure = if result_a.is_err() {
            result_a
        } else {
            result_b
        };
        assert_eq!(
            failure.unwrap_err(),
            TakeMicError::SlotOccupied,
            "M09: losing task should get SlotOccupied error"
        );

        // slot 0 恰好被一个用户占用
        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[0].is_some(),
            "M09: slot 0 should be occupied by exactly one user after concurrent take"
        );
    }

    // M10 (P2-12): handle_take_mic 在注入 MicLock 后的并发 contract — 两个并发 handler 调用，恰好一个返回 code=0，另一个返回 40303。
    #[tokio::test]
    async fn m10_concurrent_handle_take_mic_with_mic_lock_only_one_succeeds() {
        use crate::room::mic_lock::FakeMicLock;

        let room_id = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        room_manager.get_or_create_room(room_id);
        let (conn_a, _rx_a) = register_connection(&registry, user_a, Some(room_id));
        let (conn_b, _rx_b) = register_connection(&registry, user_b, Some(room_id));

        let mic_lock: Arc<dyn crate::room::mic_lock::MicLock> = Arc::new(FakeMicLock::default());
        let deps_a = Arc::new(TakeMicDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            mute_redis: None,
            mic_lock: Some(mic_lock.clone()),
        });
        let deps_b = deps_a.clone();

        let (resp_a, resp_b) = tokio::join!(
            async {
                handle_take_mic(take_mic_payload(0), Some("a".into()), conn_a, user_a, &deps_a)
                    .await
            },
            async {
                handle_take_mic(take_mic_payload(0), Some("b".into()), conn_b, user_b, &deps_b)
                    .await
            },
        );

        let json_a: serde_json::Value = serde_json::from_str(&resp_a).unwrap();
        let json_b: serde_json::Value = serde_json::from_str(&resp_b).unwrap();
        let code_a = json_a["code"].as_i64().unwrap();
        let code_b = json_b["code"].as_i64().unwrap();

        let success_count = [code_a, code_b].iter().filter(|&&c| c == 0).count();
        let occupied_count = [code_a, code_b].iter().filter(|&&c| c == 40303).count();
        assert_eq!(
            success_count, 1,
            "M10: exactly one concurrent handle_take_mic should succeed (code=0); got a={code_a} b={code_b}"
        );
        assert_eq!(
            occupied_count, 1,
            "M10: the loser must return SLOT_OCCUPIED (40303); got a={code_a} b={code_b}"
        );

        // 麦位最终只被一个用户占用
        let snapshot = room_manager
            .get_room(room_id)
            .unwrap()
            .mic_slots_snapshot();
        assert!(snapshot[0].is_some(), "M10: slot 0 must be taken");
    }

    // M11 (P2-12): MicLock 注入为 None 时不影响既有路径（fail-safe 兼容旧调用方）
    #[tokio::test]
    async fn m11_handle_take_mic_without_mic_lock_still_works() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));

        let deps = build_take_mic_deps(&room_manager, &registry); // mic_lock = None

        let resp = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;
        let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(json["code"], 0, "M11: take_mic without mic_lock should still succeed");
    }

    // ── LeaveMic 测试辅助 ─────────────────────────────────────────────────────

    /// 构建 LeaveMicDeps（测试辅助）
    fn build_leave_mic_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
    ) -> LeaveMicDeps {
        LeaveMicDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
        }
    }

    // N01: 成功下麦，mic_slots_snapshot()[idx] == None（麦位已被清空）
    #[tokio::test]
    async fn n01_leave_mic_success_slot_cleared() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 将用户预先放到麦位 2
        room_state
            .take_mic_slot(2, user_id)
            .expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        handle_leave_mic(None, conn_id, user_id, &deps).await;

        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[2].is_none(),
            "N01: mic slot 2 should be None after successful leave_mic"
        );
    }

    // N02: 用户不在麦上，返回 code=40304
    #[tokio::test]
    async fn n02_leave_mic_user_not_on_mic_returns_40304() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        // 用户在房间内，但未占用任何麦位
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        let response = handle_leave_mic(Some("msg-n02".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40304,
            "N02: user not on mic should return code 40304"
        );
        assert_eq!(
            json["type"], "LeaveMicResult",
            "N02: type should be LeaveMicResult"
        );
        assert_eq!(json["msg_id"], "msg-n02");
    }

    // N03: 用户不在房间（get_room_id=None），返回 code=40400
    #[tokio::test]
    async fn n03_leave_mic_user_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        // 注册连接但 room_id=None（用户未进入任何房间）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_leave_mic_deps(&room_manager, &registry);

        let response = handle_leave_mic(Some("msg-n03".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40400,
            "N03: user not in room should return code 40400"
        );
        assert_eq!(json["type"], "LeaveMicResult");
        assert_eq!(json["msg_id"], "msg-n03");
    }

    // N04: 成功下麦，响应 code=0，payload.mic_index 与实际麦位一致
    #[tokio::test]
    async fn n04_leave_mic_success_response_code_0_and_correct_payload() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 用户在麦位 5
        room_state
            .take_mic_slot(5, user_id)
            .expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        let response = handle_leave_mic(Some("msg-n04".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "N04: success response should have code=0");
        assert_eq!(
            json["type"], "LeaveMicResult",
            "N04: type should be LeaveMicResult"
        );
        assert_eq!(json["msg_id"], "msg-n04");
        assert_eq!(
            json["payload"]["mic_index"], 5,
            "N04: payload.mic_index should match the slot the user was on"
        );
    }

    // N05: 成功下麦，房间内其他成员收到含正确 mic_index 和 user_id 的 MicLeft 广播
    #[tokio::test]
    async fn n05_leave_mic_broadcasts_mic_left_to_other_members() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let observer_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 用户在麦位 3
        room_state
            .take_mic_slot(3, user_id)
            .expect("pre-fill should succeed");

        // 观察者已在房间
        let (_obs_conn, mut rx_observer) =
            register_connection(&registry, observer_id, Some(room_id));

        // 下麦者的连接
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        handle_leave_mic(None, conn_id, user_id, &deps).await;

        // 观察者应收到 MicLeft 广播
        let msg = tokio::time::timeout(Duration::from_millis(200), rx_observer.recv())
            .await
            .expect("N05: rx_observer should not timeout")
            .expect("N05: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["type"], "MicLeft",
            "N05: broadcast type should be MicLeft"
        );
        assert_eq!(
            json["payload"]["mic_index"], 3,
            "N05: MicLeft payload.mic_index should match the slot vacated"
        );
        assert_eq!(
            json["payload"]["user_id"],
            user_id.to_string(),
            "N05: MicLeft payload.user_id should match the user who left the mic"
        );
    }

    // N06: 下麦不影响其他用户的麦位（其余槽位保持不变）
    #[tokio::test]
    async fn n06_leave_mic_does_not_affect_other_slots() {
        let room_id = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // user_a 在麦位 1，user_b 在麦位 4
        room_state
            .take_mic_slot(1, user_a)
            .expect("pre-fill a should succeed");
        room_state
            .take_mic_slot(4, user_b)
            .expect("pre-fill b should succeed");

        // user_a 下麦
        let (conn_a, _rx_a) = register_connection(&registry, user_a, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);
        handle_leave_mic(None, conn_a, user_a, &deps).await;

        // 验证：user_a 的麦位 1 已清空，user_b 的麦位 4 不受影响
        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[1].is_none(),
            "N06: slot 1 should be None after user_a leaves mic"
        );
        assert_eq!(
            snapshot[4],
            Some(user_b),
            "N06: slot 4 should remain occupied by user_b"
        );
    }

    // N07: leave_mic_slot 用户在麦上时原子性地返回 Some(idx)
    #[tokio::test]
    async fn n07_leave_mic_slot_returns_some_idx_when_on_mic() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_state = crate::room::state::RoomState::new(room_id);
        // 将用户放到麦位 6
        room_state
            .take_mic_slot(6, user_id)
            .expect("pre-fill should succeed");

        let result = room_state.leave_mic_slot(user_id);

        assert_eq!(
            result,
            Some(6),
            "N07: leave_mic_slot should return Some(6) when user is on slot 6"
        );
        // 且槽位已被置为 None
        assert!(
            room_state.mic_slots_snapshot()[6].is_none(),
            "N07: slot 6 should be None after leave_mic_slot"
        );
    }

    // N08: leave_mic_slot 对不在任何麦位的用户返回 None
    #[tokio::test]
    async fn n08_leave_mic_slot_returns_none_when_not_on_mic() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_state = crate::room::state::RoomState::new(room_id);
        // 用户未在任何麦位

        let result = room_state.leave_mic_slot(user_id);

        assert!(
            result.is_none(),
            "N08: leave_mic_slot should return None when user is not on any mic slot"
        );
    }

    // ── SendMessage 测试辅助 ──────────────────────────────────────────────────

    /// 构建 SendMessageDeps（测试辅助）
    fn build_send_message_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
    ) -> SendMessageDeps {
        SendMessageDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            mute_redis: None,
        }
    }

    /// 构建 SendMessage payload
    fn send_message_payload(content: &str) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "content": content }))
    }

    // S01: 成功发送，其他成员 rx 收到 RoomMessage，payload.content 正确
    #[tokio::test]
    async fn s01_send_message_success_other_member_receives_room_message() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        // receiver 已在房间
        let (_recv_conn, mut rx_receiver) =
            register_connection(&registry, receiver_id, Some(room_id));

        // sender 的连接，已在房间
        let (sender_conn, _rx_sender) = register_connection(&registry, sender_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        handle_send_message(
            send_message_payload("Hello everyone!"),
            Some("msg-s01".to_string()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_receiver.recv())
            .await
            .expect("S01: rx_receiver should not timeout")
            .expect("S01: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["type"], "RoomMessage",
            "S01: broadcast type should be RoomMessage"
        );
        assert_eq!(
            json["payload"]["content"], "Hello everyone!",
            "S01: broadcast content should match sent content"
        );
    }

    // S02: 超过 500 字符，code=40001
    #[tokio::test]
    async fn s02_send_message_too_long_returns_40001() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        // 501 字符
        let long_content = "a".repeat(501);
        let response = handle_send_message(
            send_message_payload(&long_content),
            Some("msg-s02".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40001,
            "S02: content > 500 chars should return code 40001"
        );
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S03: 用户在 muted_users，code=40303
    #[tokio::test]
    async fn s03_send_message_muted_user_returns_40303() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 将用户加入禁言列表
        room_state.muted_users.insert(user_id);

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload("Hello"),
            Some("msg-s03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40303,
            "S03: muted user should return code 40303"
        );
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S04: 用户不在房间（get_room_id=None），code=40400
    #[tokio::test]
    async fn s04_send_message_user_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        // 注册连接但 room_id=None（用户未进入任何房间）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload("Hello"),
            Some("msg-s04".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40400,
            "S04: user not in room should return code 40400"
        );
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S05: content 为空字符串，code=40002
    #[tokio::test]
    async fn s05_send_message_empty_content_returns_40002() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload(""), // 空字符串
            Some("msg-s05".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40002,
            "S05: empty content should return code 40002"
        );
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S06: 幂等：相同 msg_id 第二次调用 code=0，不触发第二次广播
    #[tokio::test]
    async fn s06_send_message_duplicate_msg_id_no_second_broadcast() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        // receiver 已在房间
        let (_recv_conn, mut rx_receiver) =
            register_connection(&registry, receiver_id, Some(room_id));

        // sender 的连接
        let (sender_conn, _rx_sender) = register_connection(&registry, sender_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let msg_id = "msg-s06-unique-idempotent".to_string();

        // 第一次发送
        let response1 = handle_send_message(
            send_message_payload("Hello"),
            Some(msg_id.clone()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        // 第二次发送（相同 msg_id）
        let response2 = handle_send_message(
            send_message_payload("Hello"),
            Some(msg_id.clone()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        // 两次响应都是 code=0
        let json1: serde_json::Value = serde_json::from_str(&response1).unwrap();
        let json2: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(json1["code"], 0, "S06: first send should return code=0");
        assert_eq!(
            json2["code"], 0,
            "S06: duplicate send should also return code=0"
        );

        // rx_receiver 只收到 1 条 RoomMessage（第一次广播）
        let first_msg = tokio::time::timeout(Duration::from_millis(200), rx_receiver.recv())
            .await
            .expect("S06: should receive first broadcast")
            .expect("S06: channel should not be closed");

        let json_bc: serde_json::Value = serde_json::from_str(&first_msg).unwrap();
        assert_eq!(
            json_bc["type"], "RoomMessage",
            "S06: first broadcast should be RoomMessage"
        );

        // 第二条不应到达（超时）
        let no_second = tokio::time::timeout(Duration::from_millis(100), rx_receiver.recv()).await;
        assert!(
            no_second.is_err(),
            "S06: duplicate msg_id should NOT trigger a second broadcast"
        );
    }

    // S07: 含敏感词，广播的 content 中敏感词被替换为 ***
    #[tokio::test]
    async fn s07_send_message_sensitive_word_replaced_in_broadcast() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        let (_recv_conn, mut rx_receiver) =
            register_connection(&registry, receiver_id, Some(room_id));
        let (sender_conn, _rx_sender) = register_connection(&registry, sender_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        // 发送含敏感词的消息
        handle_send_message(
            send_message_payload("Hello badword world"),
            Some("msg-s07".to_string()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_receiver.recv())
            .await
            .expect("S07: rx_receiver should not timeout")
            .expect("S07: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "RoomMessage");
        let content = json["payload"]["content"]
            .as_str()
            .expect("content should be string");
        assert!(
            !content.contains("badword"),
            "S07: sensitive word 'badword' should be replaced; got: {content}"
        );
        assert!(
            content.contains("***"),
            "S07: replaced content should contain ***; got: {content}"
        );
    }

    // S08: 响应 code=0，type="SendMessageResult"，msg_id 回写
    #[tokio::test]
    async fn s08_send_message_success_response_type_and_code() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload("Test message"),
            Some("msg-s08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "S08: success response should have code=0");
        assert_eq!(
            json["type"], "SendMessageResult",
            "S08: type should be SendMessageResult"
        );
        assert_eq!(
            json["msg_id"], "msg-s08",
            "S08: msg_id should be echoed back"
        );
    }

    // S09: content 恰好 500 字符（边界值），成功发送
    #[tokio::test]
    async fn s09_send_message_exactly_500_chars_succeeds() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        // 恰好 500 字符
        let content_500 = "a".repeat(500);
        let response = handle_send_message(
            send_message_payload(&content_500),
            Some("msg-s09".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 0,
            "S09: exactly 500 chars should succeed with code=0"
        );
        assert_eq!(json["type"], "SendMessageResult");
    }

    // J10: get_user_by_id 返回 Err(_) → 响应 code=50000
    //
    // 验证 I-02 修复：DB 故障时必须记录日志并返回 50000，不能静默吞掉错误。
    #[tokio::test]
    async fn j10_get_user_by_id_db_error_returns_50000() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id); // 房间存在，跳过 step-2
        let auth_service = failing_auth_service(); // find_by_id 总返回 Err(_)
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-j10".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 50000,
            "DB error in get_user_by_id should return code 50000; got: {}",
            json["code"]
        );
        assert_eq!(json["type"], "JoinRoomResult");
        assert_eq!(json["msg_id"], "msg-j10");
    }

    // ── PR26-02 ~ PR26-04, PR26-12: 密码房 access_token 校验（T-00026）────────

    /// 创建含密码房间的 RoomService（room_type="password"）
    fn password_room_service_with(room_id: Uuid) -> Arc<RoomService> {
        let repo = Arc::new(FakeRoomRepository::default());
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id: Uuid::new_v4(),
            title: "密码房".to_string(),
            room_type: "password".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: Some("$2b$04$hash".to_string()),
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        Arc::new(RoomService::new(repo))
    }

    /// 构建含有 access_token 的 JoinRoom payload
    fn join_payload_with_token(room_id: Uuid, access_token: &str) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "room_id": room_id.to_string(),
            "access_token": access_token,
        }))
    }

    // PR26-02: 带有效 token 进入密码房 → 成功（code=0）
    #[tokio::test]
    async fn pr26_02_valid_token_joins_password_room() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "TestUser");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let secret = b"test-secret";
        let token = voice_room_shared::auth::room_access::encode_room_access_token(
            user_id, room_id, secret,
        )
        .expect("encode token");

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = JoinRoomDeps {
            room_manager: room_manager.clone(),
            room_service: room_service.clone(),
            auth_service: auth_service.clone(),
            registry: registry.clone(),
            stats: stats.clone(),
            jwt_secret: "test-secret".to_string(),
            kick_redis: None,
        };

        let response = handle_join_room(
            join_payload_with_token(room_id, &token),
            Some("msg-pr02".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 0,
            "PR26-02: 有效 token 应成功进入，got code={}",
            json["code"]
        );
        assert_eq!(json["type"], "JoinRoomResult");
    }

    // PR26-03: 无 token 对密码房 WS JoinRoom → 40104 PASSWORD_REQUIRED
    #[tokio::test]
    async fn pr26_03_no_token_for_password_room_returns_40104() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_id);
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        // 不带 access_token
        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-pr03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40104,
            "PR26-03: 无 token 进入密码房应返回 40104, got {}",
            json["code"]
        );
    }

    // PR26-04: token 超 60s → 40105 TOKEN_EXPIRED
    #[tokio::test]
    async fn pr26_04_expired_token_returns_40105() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_id);
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 手动构造一个过期的 token（exp = iat - 10）
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = voice_room_shared::auth::room_access::RoomAccessClaims {
            sub: user_id.to_string(),
            room_id: room_id.to_string(),
            iat: now_secs,
            exp: now_secs - 10, // 已过期
            iss: "voiceroom-room-access".to_string(),
        };
        use jsonwebtoken::{encode, EncodingKey, Header};
        let expired_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test-secret"),
        )
        .expect("encode expired token");

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        let response = handle_join_room(
            join_payload_with_token(room_id, &expired_token),
            Some("msg-pr04".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40105,
            "PR26-04: 过期 token 应返回 40105 TOKEN_EXPIRED, got {}",
            json["code"]
        );
    }

    // PR26-12: 为 B 房间颁发的 token 不能进入 A 房间（room_id 校验）
    #[tokio::test]
    async fn pr26_12_token_for_other_room_returns_40106() {
        let room_a_id = Uuid::new_v4();
        let room_b_id = Uuid::new_v4(); // 不同的房间
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_a_id); // 进入 A 房间
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 签发 B 房间的 token
        let token_for_b = voice_room_shared::auth::room_access::encode_room_access_token(
            user_id,
            room_b_id, // B 房间的 token
            b"test-secret",
        )
        .expect("encode token for room B");

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        // 尝试用 B 房间的 token 进入 A 房间
        let response = handle_join_room(
            join_payload_with_token(room_a_id, &token_for_b),
            Some("msg-pr12".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40106,
            "PR26-12: B 房间 token 不能进入 A 房间，应返回 40106 INVALID_TOKEN, got {}",
            json["code"]
        );
    }

    // ─── P1-6: last_msg_id 重连续传集成测试 ─────────────────────────────────────
    //
    // 协议：客户端在 JoinRoom payload 携带 `last_msg_id`，服务端比对
    // `recent_broadcasts` 环缓冲：
    //   - 命中 → 把 [last_msg_id 之后, 现在] 区间的消息逐条 send_to(connection_id)
    //   - 出窗 → 不重放，记录 info 日志（客户端应主动拉取兜底接口）
    //   - 缓冲为空 → 行为同出窗（None / 空 vec 都视作"无续传"）

    /// 构造一份带 last_msg_id 的 JoinRoom payload
    fn join_payload_with_last_msg(room_id: Uuid, last_msg_id: &str) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "room_id": room_id.to_string(),
            "last_msg_id": last_msg_id,
        }))
    }

    // J-replay-01: last_msg_id 命中 → 仅 joining 连接收到错过的消息（不重广播给其他人）
    #[tokio::test]
    async fn j_replay_01_last_msg_id_hit_replays_only_to_joining_connection() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4();
        let user_b_id = Uuid::new_v4();
        let user_c_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let _auth_service = auth_service_with(user_c_id, "UserC");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 预创建 room_state（user_a 已在房间里）
        let room_state = room_manager.get_or_create_room(room_id);
        let (_conn_a, mut rx_a) = register_connection(&registry, user_a_id, Some(room_id));

        // 模拟服务端历史广播过 3 条消息：m1, m2, m3
        let m1 = crate::ws::broadcaster::broadcast_to_room(
            &registry,
            &room_state,
            serde_json::json!({"type":"RoomMessage","payload":{"text":"hello1"},"timestamp":1}),
        );
        let _m2 = crate::ws::broadcaster::broadcast_to_room(
            &registry,
            &room_state,
            serde_json::json!({"type":"RoomMessage","payload":{"text":"hello2"},"timestamp":2}),
        );
        let _m3 = crate::ws::broadcaster::broadcast_to_room(
            &registry,
            &room_state,
            serde_json::json!({"type":"RoomMessage","payload":{"text":"hello3"},"timestamp":3}),
        );
        assert!(!m1.is_empty(), "broadcast should return non-empty msg_id");

        // user_a 的 rx 已经收到 3 条历史广播；清空，避免与 replay 混淆
        for _ in 0..3 {
            let _ = tokio::time::timeout(Duration::from_millis(50), rx_a.recv()).await;
        }

        // user_b（断线重连场景）：last_msg_id = m1，期望续传 m2, m3 + 然后收到 UserJoined(b)
        let (conn_b_id, mut rx_b) = register_connection(&registry, user_b_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service_with(user_b_id, "UserB"),
            &registry,
            &stats,
        );
        let _ = handle_join_room(
            join_payload_with_last_msg(room_id, &m1),
            None,
            conn_b_id,
            user_b_id,
            &deps,
        )
        .await;

        // 收到 m2
        let r1 = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
            .await
            .expect("J-replay-01: 应收到 replay m2")
            .expect("channel open");
        let r1_json: serde_json::Value = serde_json::from_str(&r1).unwrap();
        assert_eq!(
            r1_json["payload"]["text"], "hello2",
            "J-replay-01: 第一条续传应为 m2"
        );

        // 收到 m3
        let r2 = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
            .await
            .expect("J-replay-01: 应收到 replay m3")
            .expect("channel open");
        let r2_json: serde_json::Value = serde_json::from_str(&r2).unwrap();
        assert_eq!(
            r2_json["payload"]["text"], "hello3",
            "J-replay-01: 第二条续传应为 m3"
        );

        // user_a 不应再收到任何 replay 消息（仅会收到接下来的 UserJoined 广播）
        let next_a = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("rx_a 应收到 UserJoined")
            .expect("channel open");
        let next_a_json: serde_json::Value = serde_json::from_str(&next_a).unwrap();
        assert_eq!(
            next_a_json["type"], "UserJoined",
            "J-replay-01: rx_a 不应收到 replay，只收到 UserJoined"
        );
    }

    // J-replay-02: last_msg_id 不在缓冲窗口（如服务端重启 / 缓冲已驱逐）→ 不重放，不报错
    #[tokio::test]
    async fn j_replay_02_last_msg_id_out_of_window_no_replay_no_error() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "User");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 房间历史只有一条广播
        let room_state = room_manager.get_or_create_room(room_id);
        let _real = crate::ws::broadcaster::broadcast_to_room(
            &registry,
            &room_state,
            serde_json::json!({"type":"RoomMessage","payload":{"text":"x"},"timestamp":1}),
        );

        let (conn_id, mut rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        // 客户端传一个不存在的 last_msg_id
        let unknown = Uuid::new_v4().to_string();
        let response = handle_join_room(
            join_payload_with_last_msg(room_id, &unknown),
            Some("rep-02".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 0,
            "J-replay-02: 出窗 last_msg_id 不应导致 JoinRoom 失败"
        );

        // rx 收到的第一条应是 UserJoined（自己），而不是任何 replay
        let first = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("应收到 UserJoined")
            .expect("channel open");
        let first_json: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert_eq!(
            first_json["type"], "UserJoined",
            "J-replay-02: 出窗时不应有任何 replay 消息先于 UserJoined"
        );
    }

    // J-replay-03: 不传 last_msg_id → 行为不变（不重放，正常 JoinRoom）
    #[tokio::test]
    async fn j_replay_03_no_last_msg_id_behaves_as_before() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "User");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 房间已有一条历史广播
        let room_state = room_manager.get_or_create_room(room_id);
        let _ = crate::ws::broadcaster::broadcast_to_room(
            &registry,
            &room_state,
            serde_json::json!({"type":"RoomMessage","payload":{"text":"history"},"timestamp":1}),
        );

        let (conn_id, mut rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(
            &room_manager,
            &room_service,
            &auth_service,
            &registry,
            &stats,
        );

        let response = handle_join_room(
            join_payload(room_id),
            Some("rep-03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "J-replay-03: 普通 JoinRoom 应成功");

        // 第一条应是 UserJoined（自己），不应是历史 RoomMessage
        let first = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("应收到 UserJoined")
            .expect("channel open");
        let first_json: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert_eq!(
            first_json["type"], "UserJoined",
            "J-replay-03: 不传 last_msg_id 不应触发任何回放"
        );
    }

    // J-replay-04: 服务端 broadcast envelope 必含 msg_id（UUID v4 字符串），供客户端记录
    #[tokio::test]
    async fn j_replay_04_broadcast_envelope_contains_server_minted_msg_id() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let (_conn_a, mut rx_a) = register_connection(&registry, user_a_id, Some(room_id));
        let room_state = room_manager.get_or_create_room(room_id);

        let returned_id = crate::ws::broadcaster::broadcast_to_room(
            &registry,
            &room_state,
            serde_json::json!({"type":"RoomMessage","payload":{"text":"x"},"timestamp":1}),
        );

        let received = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("应收到广播")
            .expect("channel open");
        let json: serde_json::Value = serde_json::from_str(&received).unwrap();

        let msg_id = json["msg_id"]
            .as_str()
            .expect("J-replay-04: envelope 必须含 msg_id 字符串字段");
        assert_eq!(
            msg_id, returned_id,
            "J-replay-04: 接收方看到的 msg_id 必须等于 broadcast_to_room 返回值"
        );
        // UUID v4 长度 36 字符（带连字符）
        assert_eq!(
            msg_id.len(),
            36,
            "J-replay-04: msg_id 应为 UUID v4 标准长度 36"
        );
    }
}
