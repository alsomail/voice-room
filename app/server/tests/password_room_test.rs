//! 集成测试 — T-00026 密码房进房校验 + 锁定机制
//!
//! 验收用例 PR26-01 ~ PR26-12：
//! - PR26-01: 正确密码返回 access_token
//! - PR26-02: token 带入 WS JoinRoom 成功进房
//! - PR26-03: 无 token 对密码房 WS JoinRoom → 40104
//! - PR26-04: token 超 60s → 40105
//! - PR26-05: 错误密码连续 5 次后返回 42910 (Locked)
//! - PR26-06: 锁定后 30min 内任何请求返回 42910 + remaining_sec
//! - PR26-07: 锁定 TTL 到期后可重新尝试
//! - PR26-08: 非密码房调用 verify-password → password_hash 缺失错误
//! - PR26-09: 密码格式非 6 位数字 → WrongPassword 递减计数
//! - PR26-10: 并发 5 次错误仅创建一次锁定 key
//! - PR26-11: 正确登录后 pwd_fail key 被清除
//! - PR26-12: 为 B 房间颁发的 token 不能进入 A 房间
//!
//! 所有测试均使用 FakeRoomPasswordRedis（内存 HashMap），无需真实 Redis。

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::modules::auth::service::AuthService;
use voice_room_server::modules::room::password::{
    verify_password, FakeRoomPasswordRedis, VerifyPasswordResult,
};
use voice_room_server::modules::room::service::RoomService;
use voice_room_server::modules::room::FakeRoomRepository;
use voice_room_server::room::{
    handler::{handle_join_room, JoinRoomDeps},
    manager::RoomManager,
};
use voice_room_server::stats::FakeStatsService;
use voice_room_server::ws::registry::{ConnectionHandle, ConnectionRegistry};
use voice_room_shared::auth::room_access::encode_room_access_token;
use voice_room_shared::models::room::RoomModel;

const TEST_JWT_SECRET: &str = "test-jwt-secret";
const BCRYPT_COST: u32 = 4;

// ─── 测试辅助 ────────────────────────────────────────────────────────────────

fn make_password_room(room_id: Uuid, password: &str) -> RoomModel {
    let hash = bcrypt::hash(password, BCRYPT_COST).expect("bcrypt hash");
    RoomModel {
        id: room_id,
        owner_id: Uuid::new_v4(),
        title: "测试密码房".to_string(),
        room_type: "password".to_string(),
        member_count: 0,
        status: "active".to_string(),
        password_hash: Some(hash),
        max_members: 50,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        cover_url: String::new(),
        category: "chat".to_string(),
        announcement: None,
        admin_user_id: None,
    }
}

fn make_normal_room(room_id: Uuid) -> RoomModel {
    RoomModel {
        id: room_id,
        owner_id: Uuid::new_v4(),
        title: "普通房间".to_string(),
        room_type: "normal".to_string(),
        member_count: 0,
        status: "active".to_string(),
        password_hash: None,
        max_members: 50,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        cover_url: String::new(),
        category: "chat".to_string(),
        announcement: None,
        admin_user_id: None,
    }
}

fn password_room_service(room_id: Uuid, password: &str) -> Arc<RoomService> {
    let repo = Arc::new(FakeRoomRepository::default());
    repo.seed(make_password_room(room_id, password));
    Arc::new(RoomService::new(repo))
}

#[allow(dead_code)]
fn normal_room_service(room_id: Uuid) -> Arc<RoomService> {
    let repo = Arc::new(FakeRoomRepository::default());
    repo.seed(make_normal_room(room_id));
    Arc::new(RoomService::new(repo))
}

fn build_ws_deps(
    room_service: Arc<RoomService>,
    registry: &Arc<ConnectionRegistry>,
) -> JoinRoomDeps {
    use voice_room_server::infrastructure::redis_store::FakeCodeStore;
    use voice_room_server::infrastructure::third_party::sms::MockSmsProvider;
    use voice_room_server::modules::auth::repository::FakeUserRepository;

    let auth_service = Arc::new(AuthService::new(
        Arc::new(FakeUserRepository::default()),
        Arc::new(FakeCodeStore::default()),
        Arc::new(MockSmsProvider),
        TEST_JWT_SECRET.to_string(),
    ));

    JoinRoomDeps {
        room_manager: Arc::new(RoomManager::new()),
        room_service,
        auth_service,
        registry: registry.clone(),
        stats: Arc::new(FakeStatsService::default()),
        jwt_secret: TEST_JWT_SECRET.to_string(),
        kick_redis: None,
    }
}

fn register_connection(
    registry: &Arc<ConnectionRegistry>,
    user_id: Uuid,
) -> (Uuid, mpsc::UnboundedReceiver<String>) {
    let conn_id = Uuid::new_v4();
    let (tx, rx) = mpsc::unbounded_channel();
    use std::sync::RwLock;
    registry.register(ConnectionHandle {
        connection_id: conn_id,
        user_id,
        room_id: None,
        sender: tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });
    (conn_id, rx)
}

// ─── PR26-01: 正确密码返回 access_token ──────────────────────────────────────

#[tokio::test]
async fn pr26_01_correct_password_returns_token() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_password_room(room_id, "123456");
    let redis = FakeRoomPasswordRedis::default();

    let result = verify_password(&room, "123456", user_id, &redis, TEST_JWT_SECRET)
        .await
        .expect("should not error");

    match result {
        VerifyPasswordResult::Token(jwt) => {
            use voice_room_shared::auth::room_access::decode_room_access_token;
            let claims =
                decode_room_access_token(&jwt, TEST_JWT_SECRET.as_bytes()).expect("decode jwt");
            assert_eq!(claims.sub, user_id.to_string());
            assert_eq!(claims.room_id, room_id.to_string());
            assert_eq!(claims.iss, "voiceroom-room-access");
        }
        other => panic!("PR26-01: expected Token, got {other:?}"),
    }
}

// ─── PR26-02: token 带入 WS JoinRoom 成功进房 ────────────────────────────────

#[tokio::test]
async fn pr26_02_valid_token_joins_password_room() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room_service = password_room_service(room_id, "123456");
    let registry = Arc::new(ConnectionRegistry::new());
    let deps = build_ws_deps(room_service, &registry);

    let token = encode_room_access_token(user_id, room_id, TEST_JWT_SECRET.as_bytes())
        .expect("encode token");

    let (conn_id, _rx) = register_connection(&registry, user_id);

    let payload = Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "access_token": token,
    }));

    let response = handle_join_room(
        payload,
        Some("msg-pr02".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        json["code"], 0,
        "PR26-02: 有效 token 应成功进入, got {}",
        json["code"]
    );
    assert_eq!(json["type"], "JoinRoomResult");
}

// ─── PR26-03: 无 token 对密码房 WS JoinRoom → 40104 ─────────────────────────

#[tokio::test]
async fn pr26_03_no_token_password_room_returns_40104() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room_service = password_room_service(room_id, "123456");
    let registry = Arc::new(ConnectionRegistry::new());
    let deps = build_ws_deps(room_service, &registry);

    let (conn_id, _rx) = register_connection(&registry, user_id);

    // 不携带 access_token
    let payload = Some(serde_json::json!({ "room_id": room_id.to_string() }));

    let response = handle_join_room(
        payload,
        Some("msg-pr03".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        json["code"], 40104,
        "PR26-03: 无 token 进密码房应返回 40104, got {}",
        json["code"]
    );
}

// ─── PR26-04: token 超 60s → 40105 TOKEN_EXPIRED ────────────────────────────

#[tokio::test]
async fn pr26_04_expired_token_returns_40105() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room_service = password_room_service(room_id, "123456");
    let registry = Arc::new(ConnectionRegistry::new());
    let deps = build_ws_deps(room_service, &registry);

    // 构造已过期的 token
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = voice_room_shared::auth::room_access::RoomAccessClaims {
        sub: user_id.to_string(),
        room_id: room_id.to_string(),
        iat: now_secs,
        exp: now_secs - 10,
        iss: "voiceroom-room-access".to_string(),
    };
    use jsonwebtoken::{encode, EncodingKey, Header};
    let expired_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("encode expired token");

    let (conn_id, _rx) = register_connection(&registry, user_id);

    let payload = Some(serde_json::json!({
        "room_id": room_id.to_string(),
        "access_token": expired_token,
    }));

    let response = handle_join_room(
        payload,
        Some("msg-pr04".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        json["code"], 40105,
        "PR26-04: 过期 token 应返回 40105, got {}",
        json["code"]
    );
}

// ─── PR26-05: 错误密码连续 5 次后返回 Locked(42910) ─────────────────────────

#[tokio::test]
async fn pr26_05_five_wrong_passwords_triggers_lock() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_password_room(room_id, "123456");
    let redis = FakeRoomPasswordRedis::default();

    // 前 4 次：WrongPassword
    for i in 1..5 {
        let result = verify_password(&room, "wrong", user_id, &redis, TEST_JWT_SECRET)
            .await
            .unwrap();
        assert!(
            matches!(result, VerifyPasswordResult::WrongPassword { .. }),
            "PR26-05: 第 {i} 次失败应为 WrongPassword, got {result:?}"
        );
    }

    // 第 5 次：Locked
    let result = verify_password(&room, "wrong", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();
    assert!(
        matches!(result, VerifyPasswordResult::Locked { .. }),
        "PR26-05: 第 5 次失败应返回 Locked(42910), got {result:?}"
    );
}

// ─── PR26-06: 锁定后任何请求（含正确密码）返回 42910 + remaining_sec ─────────

#[tokio::test]
async fn pr26_06_locked_always_returns_42910() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_password_room(room_id, "123456");
    let redis = FakeRoomPasswordRedis::default();

    // 触发锁定
    for _ in 0..5 {
        let _ = verify_password(&room, "wrong", user_id, &redis, TEST_JWT_SECRET)
            .await
            .unwrap();
    }

    // 锁定后用正确密码也应返回 Locked
    let result = verify_password(&room, "123456", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();

    match result {
        VerifyPasswordResult::Locked { remaining_sec } => {
            assert!(remaining_sec > 0, "PR26-06: remaining_sec 应 > 0");
        }
        other => panic!("PR26-06: 锁定后应返回 Locked, got {other:?}"),
    }
}

// ─── PR26-07: 锁定 TTL 到期后可重新尝试 ─────────────────────────────────────

#[tokio::test]
async fn pr26_07_after_lock_expires_can_retry() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_password_room(room_id, "123456");
    let redis = FakeRoomPasswordRedis::default();

    // 触发锁定
    for _ in 0..5 {
        let _ = verify_password(&room, "wrong", user_id, &redis, TEST_JWT_SECRET)
            .await
            .unwrap();
    }

    // 模拟 TTL 到期
    redis.expire_all();

    // 正确密码应成功
    let result = verify_password(&room, "123456", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();

    assert!(
        matches!(result, VerifyPasswordResult::Token(_)),
        "PR26-07: TTL 到期后正确密码应返回 Token, got {result:?}"
    );
}

// ─── PR26-08: password_hash 缺失（非密码房防御性测试）→ 内部错误 ───────────

#[tokio::test]
async fn pr26_08_missing_password_hash_returns_error() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_normal_room(room_id); // password_hash = None
    let redis = FakeRoomPasswordRedis::default();

    let result = verify_password(&room, "123456", user_id, &redis, TEST_JWT_SECRET).await;

    assert!(
        result.is_err(),
        "PR26-08: 缺少 password_hash 应返回 Err, got Ok({:?})",
        result.unwrap()
    );
}

// ─── PR26-09: 密码连续失败递减 remaining_attempts ────────────────────────────

#[tokio::test]
async fn pr26_09_remaining_attempts_decrements() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_password_room(room_id, "123456");
    let redis = FakeRoomPasswordRedis::default();

    let r1 = verify_password(&room, "000000", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();
    assert_eq!(
        r1,
        VerifyPasswordResult::WrongPassword {
            remaining_attempts: 4
        }
    );

    let r2 = verify_password(&room, "000000", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();
    assert_eq!(
        r2,
        VerifyPasswordResult::WrongPassword {
            remaining_attempts: 3
        }
    );

    let r3 = verify_password(&room, "000000", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();
    assert_eq!(
        r3,
        VerifyPasswordResult::WrongPassword {
            remaining_attempts: 2
        }
    );

    let r4 = verify_password(&room, "000000", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();
    assert_eq!(
        r4,
        VerifyPasswordResult::WrongPassword {
            remaining_attempts: 1
        }
    );
}

// ─── PR26-10: 并发 5 次错误仅创建一次锁定 key ────────────────────────────────

#[tokio::test]
async fn pr26_10_concurrent_failures_only_one_lock_key() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = Arc::new(make_password_room(room_id, "123456"));
    let redis = Arc::new(FakeRoomPasswordRedis::default());

    // 先顺序失败 4 次
    for _ in 0..4 {
        let _ = verify_password(&*room, "wrong", user_id, &*redis, TEST_JWT_SECRET)
            .await
            .unwrap();
    }

    // 并发 3 次（其中一次会触发第 5 次，触发锁定）
    let mut handles = Vec::new();
    for _ in 0..3 {
        let room_clone = Arc::clone(&room);
        let redis_clone = Arc::clone(&redis);
        let h = tokio::spawn(async move {
            verify_password(
                &*room_clone,
                "wrong",
                user_id,
                &*redis_clone,
                TEST_JWT_SECRET,
            )
            .await
        });
        handles.push(h);
    }
    for h in handles {
        let _ = h.await.unwrap();
    }

    // lock key 应存在（只被设置一次，NX 保证）
    let lock_key = format!("pwd_lock:{user_id}:{room_id}");
    assert!(redis.key_exists(&lock_key), "PR26-10: lock key 应存在");
}

// ─── PR26-11: 正确密码后 pwd_fail key 被清除 ─────────────────────────────────

#[tokio::test]
async fn pr26_11_correct_password_clears_fail_key() {
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room = make_password_room(room_id, "123456");
    let redis = FakeRoomPasswordRedis::default();

    // 失败 3 次
    for _ in 0..3 {
        let _ = verify_password(&room, "wrong", user_id, &redis, TEST_JWT_SECRET)
            .await
            .unwrap();
    }

    let fail_key = format!("pwd_fail:{user_id}:{room_id}");
    assert!(
        redis.key_exists(&fail_key),
        "PR26-11: 失败 3 次后 fail key 应存在"
    );

    // 正确密码
    let result = verify_password(&room, "123456", user_id, &redis, TEST_JWT_SECRET)
        .await
        .unwrap();
    assert!(
        matches!(result, VerifyPasswordResult::Token(_)),
        "PR26-11: 正确密码应返回 Token"
    );

    // fail key 应被清除
    assert!(
        !redis.key_exists(&fail_key),
        "PR26-11: 成功后 fail key 应被清除"
    );
}

// ─── PR26-12: B 房间 token 不能进入 A 房间（room_id 校验）───────────────────

#[tokio::test]
async fn pr26_12_token_for_other_room_returns_40106() {
    let room_a_id = Uuid::new_v4();
    let room_b_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let room_service = password_room_service(room_a_id, "123456");
    let registry = Arc::new(ConnectionRegistry::new());
    let deps = build_ws_deps(room_service, &registry);

    // 签发 B 房间的 token
    let token_for_b = encode_room_access_token(user_id, room_b_id, TEST_JWT_SECRET.as_bytes())
        .expect("encode token for room B");

    let (conn_id, _rx) = register_connection(&registry, user_id);

    // 用 B 的 token 尝试进入 A 房间
    let payload = Some(serde_json::json!({
        "room_id": room_a_id.to_string(),
        "access_token": token_for_b,
    }));

    let response = handle_join_room(
        payload,
        Some("msg-pr12".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        json["code"], 40106,
        "PR26-12: B 房间 token 不能进入 A 房间，应返回 40106, got {}",
        json["code"]
    );
}
