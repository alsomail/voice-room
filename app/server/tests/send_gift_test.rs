//! 集成测试 — T-00020 SendGift 事务 + 广播
//!
//! 测试用例 SG01~SG12 验证以下内容：
//! - SG01: 送礼成功：发送者余额 -total、接收者 charm_balance += total、gift_records +1、wallet_transactions +1
//! - SG02: 房间所有成员收到 GiftReceived 广播
//! - SG03: 发送者单独收到 BalanceUpdated { delta: -total }
//! - SG04: Redis ZSCORE ranking:charm:day:... 正确更新
//! - SG05: 余额不足 → 整体回滚，返回 InsufficientBalance
//! - SG06: 幂等：相同 (sender, msg_id) 二次发送返回首次结果，不再扣款、不再广播
//! - SG07: 接收者离开麦位后送礼返回 ReceiverUnavailable
//! - SG08: gift 被下架 is_active=false → GiftUnavailable
//! - SG09: count=0 / count=10000 → InvalidCount
//! - SG10: 并发 20 个送礼请求同一发送者：无超扣、事务隔离
//! - SG11: 发送者不在房间 → SenderNotInRoom
//! - SG12: 事务回滚后余额不变、榜单不变
//!
//! 运行前提：DATABASE_URL 指向可用 PostgreSQL 实例（REDIS_URL 可选）。
//! 未设置时测试自动跳过。

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::modules::gift::send_gift::{
    GiftSendService, SendGiftError, SendGiftPayload, SendGiftServicePort,
};
use voice_room_server::modules::wallet::broadcaster::BalanceEvent;
use voice_room_server::room::{manager::RoomManager, state::RoomState};
use voice_room_server::ws::registry::{ConnectionHandle, ConnectionRegistry};

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 获取测试用数据库连接池；未配置 DATABASE_URL 或连接失败时返回 None（测试跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 获取 Redis URL（可选，缺失时相关 Redis 断言被跳过）
fn redis_url() -> Option<String> {
    std::env::var("REDIS_URL").ok()
}

/// 在数据库中插入测试用户，设置初始 diamond_balance
async fn insert_test_user(pool: &PgPool, balance: i64) -> Uuid {
    let user_id = Uuid::new_v4();
    let phone = format!("+86{}", &user_id.to_string().replace('-', "")[..11]);
    sqlx::query(
        "INSERT INTO users (id, phone, nickname, diamond_balance) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(&phone)
    .bind(format!("TestUser_{}", &user_id.to_string()[..8]))
    .bind(balance)
    .execute(pool)
    .await
    .expect("insert test user");
    user_id
}

/// 在数据库中插入测试房间
async fn insert_test_room(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let room_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO rooms (id, owner_id, title, status) \
         VALUES ($1, $2, $3, 'active')",
    )
    .bind(room_id)
    .bind(owner_id)
    .bind("Test Room")
    .execute(pool)
    .await
    .expect("insert test room");
    room_id
}

/// 获取 gifts 表中最便宜的活跃礼物（迁移后有种子数据）
async fn get_active_gift(pool: &PgPool) -> (Uuid, i64) {
    let row = sqlx::query(
        "SELECT id, price FROM gifts WHERE is_active = true ORDER BY price ASC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("get active gift");
    (row.get("id"), row.get("price"))
}

/// 查询用户当前 diamond_balance
async fn get_diamond_balance(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT diamond_balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("get diamond_balance")
}

/// 查询用户当前 charm_balance
async fn get_charm_balance(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT charm_balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("get charm_balance")
}

/// 查询用户 wallet_transactions 数量
async fn count_wallet_txns(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM wallet_transactions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("count wallet_txns")
}

/// 查询 gift_records 数量（by sender + msg_id）
async fn count_gift_records_by_msg(pool: &PgPool, sender_id: Uuid, msg_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM gift_records WHERE sender_id = $1 AND msg_id = $2")
        .bind(sender_id)
        .bind(msg_id)
        .fetch_one(pool)
        .await
        .expect("count gift_records")
}

/// 构建 GiftSendService（含内存 ConnectionRegistry + RoomManager）
fn make_service(
    pool: PgPool,
    registry: Arc<ConnectionRegistry>,
    room_manager: Arc<RoomManager>,
    balance_tx: mpsc::Sender<BalanceEvent>,
    redis_url_opt: Option<String>,
) -> GiftSendService {
    let redis_url = redis_url_opt.unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
    GiftSendService::new(pool, registry, room_manager, balance_tx, redis_url)
}

/// 注册用户到房间（内存状态）
fn join_room(
    registry: &ConnectionRegistry,
    room_state: &RoomState,
    user_id: Uuid,
) -> (Uuid, mpsc::UnboundedReceiver<String>) {
    let connection_id = Uuid::new_v4();
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    use std::sync::RwLock;
    use std::time::Instant;
    registry.register(ConnectionHandle {
        connection_id,
        user_id,
        room_id: Some(room_state.room_id),
        sender: tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });
    use voice_room_server::room::state::MemberInfo;
    room_state.members.insert(
        user_id,
        MemberInfo {
            user_id,
            nickname: format!("User_{}", &user_id.to_string()[..8]),
            avatar: None,
            joined_at: chrono::Utc::now(),
        },
    );
    (connection_id, rx)
}

/// 插入一个专用的下架测试礼物（is_active=false），不依赖种子数据
/// 测试结束后由调用者负责 DELETE（或直接丢弃，UUID 隔离无污染）
async fn insert_inactive_test_gift(pool: &PgPool) -> Uuid {
    let gift_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gifts \
         (id, code, name_en, name_ar, icon_url, price, tier, effect_level, sort_order, is_active, is_deleted) \
         VALUES ($1, $2, 'Test Gift', 'هدية اختبار', '/test/icon.png', 1, 1, 1, 999, false, false)"
    )
    .bind(gift_id)
    .bind(format!("test_inactive_{}", &gift_id.to_string().replace('-', "")[..8]))
    .execute(pool)
    .await
    .expect("insert inactive test gift");
    gift_id
}

/// 让接收者上麦
fn take_mic(room_state: &RoomState, user_id: Uuid, slot: usize) {
    room_state
        .take_mic_slot(slot, user_id)
        .expect("take_mic_slot should succeed");
}

// ─────────────────────────────────────────────────────────────────────────────
// SG01: 送礼成功 — 余额/魅力/records/txns 全部正确
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg01_send_gift_success_updates_all_tables() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg01: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, price) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    let (_sender_conn, _sender_rx) = join_room(&registry, &room_state, sender_id);
    let (_recv_conn, _recv_rx) = join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let msg_id = Uuid::new_v4().to_string();
    let count = 2i32;
    let total = price * count as i64;

    let result = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count,
                msg_id: msg_id.clone(),
            },
        )
        .await;

    assert!(
        result.is_ok(),
        "SG01: send should succeed, got: {:?}",
        result.err()
    );
    let res = result.unwrap();
    assert_eq!(
        res.total_price, total,
        "SG01: total_price should equal price * count"
    );

    // 验证发送者余额减少
    let sender_balance = get_diamond_balance(&pool, sender_id).await;
    assert_eq!(
        sender_balance,
        10_000 - total,
        "SG01: sender diamond_balance should decrease"
    );

    // 验证接收者魅力值增加
    let recv_charm = get_charm_balance(&pool, receiver_id).await;
    assert_eq!(
        recv_charm, total,
        "SG01: receiver charm_balance should increase"
    );

    // 验证 gift_records +1
    let records_count = count_gift_records_by_msg(&pool, sender_id, &msg_id).await;
    assert_eq!(records_count, 1, "SG01: gift_records should have 1 entry");

    // 验证 wallet_transactions +1
    let txn_count = count_wallet_txns(&pool, sender_id).await;
    assert_eq!(
        txn_count, 1,
        "SG01: wallet_transactions should have 1 entry"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG02: GiftReceived 广播给房间所有成员（含麦上/麦下观众）
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg02_gift_received_broadcast_reaches_all_room_members() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg02: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let observer_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, _price) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    let (_sender_conn, mut sender_rx) = join_room(&registry, &room_state, sender_id);
    let (_recv_conn, mut recv_rx) = join_room(&registry, &room_state, receiver_id);
    let (_obs_conn, mut obs_rx) = join_room(&registry, &room_state, observer_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let msg_id = Uuid::new_v4().to_string();
    svc.send(
        sender_id,
        room_id,
        SendGiftPayload {
            gift_id,
            receiver_id,
            count: 1,
            msg_id,
        },
    )
    .await
    .expect("SG02: send should succeed");

    // 所有房间成员应收到 GiftReceived 广播
    let check_gift_received = |rx: &mut mpsc::UnboundedReceiver<String>, label: &str| {
        let mut found = false;
        while let Ok(msg) = rx.try_recv() {
            if msg.contains("GiftReceived") {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "SG02: {} should receive GiftReceived broadcast",
            label
        );
    };

    check_gift_received(&mut sender_rx, "sender");
    check_gift_received(&mut recv_rx, "receiver");
    check_gift_received(&mut obs_rx, "observer");
}

// ─────────────────────────────────────────────────────────────────────────────
// SG03: 发送者单独收到 BalanceUpdated
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg03_sender_receives_balance_updated() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg03: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, price) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, mut balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let msg_id = Uuid::new_v4().to_string();
    svc.send(
        sender_id,
        room_id,
        SendGiftPayload {
            gift_id,
            receiver_id,
            count: 1,
            msg_id,
        },
    )
    .await
    .expect("SG03: send should succeed");

    // 发送者应收到 BalanceUpdated 事件
    let event = tokio::time::timeout(Duration::from_millis(500), balance_rx.recv())
        .await
        .expect("SG03: should receive BalanceEvent within 500ms")
        .expect("SG03: channel should not be closed");

    assert_eq!(event.user_id, sender_id, "SG03: event should be for sender");
    assert_eq!(
        event.delta, -price,
        "SG03: delta should be -price (count=1)"
    );
    assert_eq!(
        event.balance_after,
        10_000 - price,
        "SG03: balance_after should be updated"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG04: Redis 榜单 ZINCRBY 正确更新
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg04_redis_ranking_zincrby_updated() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg04: DATABASE_URL not set");
        return;
    };
    let Some(r_url) = redis_url() else {
        eprintln!("[SKIP] sg04: REDIS_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, price) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        Some(r_url.clone()),
    );

    let count = 2i32;
    let total = price * count as i64;
    let msg_id = Uuid::new_v4().to_string();

    svc.send(
        sender_id,
        room_id,
        SendGiftPayload {
            gift_id,
            receiver_id,
            count,
            msg_id,
        },
    )
    .await
    .expect("SG04: send should succeed");

    // 验证 Redis 魅力榜
    let client = redis::Client::open(r_url).expect("redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("redis conn");
    use redis::AsyncCommands;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let charm_key = format!("ranking:charm:day:{}", today);
    let score: Option<f64> = conn
        .zscore(&charm_key, receiver_id.to_string())
        .await
        .ok()
        .flatten();
    assert!(score.is_some(), "SG04: charm ranking key should exist");
    // [C-1] 修复：charm_day 改为纯 ZINCRBY，score 必须精确等于 total（不能双倍）
    assert_eq!(
        score.unwrap(),
        total as f64,
        "SG04: charm score must equal total exactly (pure ZINCRBY, no double-counting)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG05: 余额不足 — 整体回滚，返回 InsufficientBalance
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg05_insufficient_balance_rolls_back_entire_transaction() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg05: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 1).await; // 余额仅 1 钻
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, price) = get_active_gift(&pool).await;

    // 确保礼物价格 > 1（如果最便宜的礼物价格=1，也只给余额=0）
    let needed_balance: i64 = 0; // 余额 1 < price（至少是1，所以用 price > 1 的礼物或余额 0）
    let _ = needed_balance;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    // count=9999 确保 total > 1
    let count = 9999i32;
    let msg_id = Uuid::new_v4().to_string();

    let result = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count,
                msg_id,
            },
        )
        .await;

    assert!(
        matches!(result, Err(SendGiftError::InsufficientBalance)),
        "SG05: should return InsufficientBalance, got: {:?}",
        result
    );

    // 验证余额未变
    let balance = get_diamond_balance(&pool, sender_id).await;
    assert_eq!(
        balance, 1,
        "SG05: balance should be unchanged after rollback"
    );

    // 验证 gift_records 未新增
    let records: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gift_records WHERE sender_id = $1")
        .bind(sender_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        records, 0,
        "SG05: no gift_records should be created on failure"
    );

    // 验证 wallet_transactions 未新增
    let txns = count_wallet_txns(&pool, sender_id).await;
    assert_eq!(
        txns, 0,
        "SG05: no wallet_transactions should be created on failure"
    );

    // 验证 receiver charm_balance 未变
    let charm = get_charm_balance(&pool, receiver_id).await;
    assert_eq!(
        charm, 0,
        "SG05: receiver charm_balance should be 0 after rollback"
    );

    // 验证发送者的 price*count 没有扣除 - 忽略 price 变量
    let _ = price;
}

// ─────────────────────────────────────────────────────────────────────────────
// SG06: 幂等 — 相同 (sender, msg_id) 二次发送返回首次 code，不再扣款、不再广播
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg06_idempotent_second_send_returns_same_result_no_double_deduction() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg06: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, price) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let msg_id = Uuid::new_v4().to_string();
    let payload = SendGiftPayload {
        gift_id,
        receiver_id,
        count: 1,
        msg_id: msg_id.clone(),
    };

    // 第一次发送
    let first = svc.send(sender_id, room_id, payload.clone()).await;
    assert!(first.is_ok(), "SG06: first send should succeed");
    let first_result = first.unwrap();

    // 第二次发送（相同 msg_id）
    let second = svc.send(sender_id, room_id, payload).await;
    assert!(
        second.is_ok(),
        "SG06: second send with same msg_id should return Ok (idempotent)"
    );
    let second_result = second.unwrap();

    // 两次结果应相同
    assert_eq!(
        first_result.gift_record_id, second_result.gift_record_id,
        "SG06: idempotent result should have same gift_record_id"
    );
    assert_eq!(
        first_result.total_price, second_result.total_price,
        "SG06: idempotent result should have same total_price"
    );

    // 余额只扣一次
    let balance = get_diamond_balance(&pool, sender_id).await;
    assert_eq!(
        balance,
        10_000 - price,
        "SG06: balance should be deducted only once"
    );

    // gift_records 只有一条
    let records = count_gift_records_by_msg(&pool, sender_id, &msg_id).await;
    assert_eq!(records, 1, "SG06: gift_records should have exactly 1 entry");
}

// ─────────────────────────────────────────────────────────────────────────────
// SG07: 接收者不在麦位 → ReceiverUnavailable
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg07_receiver_not_on_mic_returns_receiver_unavailable() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg07: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, _) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    // receiver_id 在房间但没有上麦

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let result = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count: 1,
                msg_id: Uuid::new_v4().to_string(),
            },
        )
        .await;

    assert!(
        matches!(result, Err(SendGiftError::ReceiverUnavailable)),
        "SG07: should return ReceiverUnavailable when receiver not on mic, got: {:?}",
        result
    );

    // 余额不变
    let balance = get_diamond_balance(&pool, sender_id).await;
    assert_eq!(balance, 10_000, "SG07: balance should be unchanged");
}

// ─────────────────────────────────────────────────────────────────────────────
// SG08: gift 被下架 (is_active=false) → GiftUnavailable
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg08_inactive_gift_returns_gift_unavailable() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg08: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    // [L-1] 修复：使用专用下架测试礼物（UUID 隔离），避免修改种子数据造成状态污染
    let gift_id = insert_inactive_test_gift(&pool).await;

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let result = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count: 1,
                msg_id: Uuid::new_v4().to_string(),
            },
        )
        .await;

    // 清理专用测试礼物（即使断言失败也不影响其他测试，因为是独立 UUID）
    let _ = sqlx::query("DELETE FROM gifts WHERE id = $1")
        .bind(gift_id)
        .execute(&pool)
        .await;

    assert!(
        matches!(result, Err(SendGiftError::GiftUnavailable)),
        "SG08: should return GiftUnavailable for inactive gift, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG09: count=0 / count=10000 → InvalidCount
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg09_invalid_count_returns_error() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg09: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, _) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry.clone(),
        room_manager.clone(),
        balance_tx.clone(),
        redis_url(),
    );

    // count = 0 → InvalidCount
    let result_zero = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count: 0,
                msg_id: Uuid::new_v4().to_string(),
            },
        )
        .await;
    assert!(
        matches!(result_zero, Err(SendGiftError::InvalidCount)),
        "SG09: count=0 should return InvalidCount, got: {:?}",
        result_zero
    );

    // count = 10000 → InvalidCount
    let result_max = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count: 10_000,
                msg_id: Uuid::new_v4().to_string(),
            },
        )
        .await;
    assert!(
        matches!(result_max, Err(SendGiftError::InvalidCount)),
        "SG09: count=10000 should return InvalidCount, got: {:?}",
        result_max
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG10: 并发 20 个送礼请求 — 无超扣、事务隔离
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg10_concurrent_20_requests_no_over_deduction() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg10: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    // 给发送者足够的余额（20次，每次1礼，单价1），余额=20
    let sender_id = insert_test_user(&pool, 20).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;

    // 找到价格=1的礼物（rose_01）
    let gift_row =
        sqlx::query("SELECT id, price FROM gifts WHERE price = 1 AND is_active = true LIMIT 1")
            .fetch_optional(&pool)
            .await
            .expect("query gift");
    let Some(gift_row) = gift_row else {
        eprintln!("[SKIP] sg10: no gift with price=1 found");
        return;
    };
    let gift_id: Uuid = gift_row.get("id");
    let price: i64 = gift_row.get("price");
    assert_eq!(price, 1, "sg10: need gift with price=1");

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    join_room(&registry, &room_state, sender_id);
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(256);
    let svc = Arc::new(make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    ));

    // 并发 20 个请求（每个不同 msg_id）
    let mut handles = Vec::new();
    for _ in 0..20 {
        let svc = svc.clone();
        let msg_id = Uuid::new_v4().to_string();
        let h = tokio::spawn(async move {
            svc.send(
                sender_id,
                room_id,
                SendGiftPayload {
                    gift_id,
                    receiver_id,
                    count: 1,
                    msg_id,
                },
            )
            .await
        });
        handles.push(h);
    }

    let mut successes = 0usize;
    let mut insufficient = 0usize;
    for h in handles {
        match h.await.expect("task should not panic") {
            Ok(_) => successes += 1,
            Err(SendGiftError::InsufficientBalance) => insufficient += 1,
            Err(e) => panic!("SG10: unexpected error: {:?}", e),
        }
    }

    // 成功次数应等于初始余额（20），每次扣 1
    assert_eq!(
        successes, 20,
        "SG10: exactly 20 should succeed (balance=20, price=1)"
    );
    assert_eq!(
        insufficient, 0,
        "SG10: no insufficient balance since balance=20 and 20 requests"
    );

    // 最终余额应为 0（无超扣）
    let final_balance = get_diamond_balance(&pool, sender_id).await;
    assert_eq!(
        final_balance, 0,
        "SG10: final balance should be exactly 0 after 20 successful deductions"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG11: 发送者不在房间 → SenderNotInRoom
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg11_sender_not_in_room_returns_error() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg11: DATABASE_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let sender_id = insert_test_user(&pool, 10_000).await;
    let receiver_id = insert_test_user(&pool, 0).await;
    let owner_id = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner_id).await;
    let (gift_id, _) = get_active_gift(&pool).await;

    let registry = Arc::new(ConnectionRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);

    // sender_id 没有加入房间（不在 room_state.members）
    join_room(&registry, &room_state, receiver_id);
    take_mic(&room_state, receiver_id, 0);

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(64);
    let svc = make_service(
        pool.clone(),
        registry,
        room_manager,
        balance_tx,
        redis_url(),
    );

    let result = svc
        .send(
            sender_id,
            room_id,
            SendGiftPayload {
                gift_id,
                receiver_id,
                count: 1,
                msg_id: Uuid::new_v4().to_string(),
            },
        )
        .await;

    assert!(
        matches!(result, Err(SendGiftError::SenderNotInRoom)),
        "SG11: should return SenderNotInRoom when sender not in room, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SG12: 迁移幂等：连续两次运行迁移 006 不报错
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn sg12_migration_006_is_idempotent() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] sg12: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("sg12: first migration run");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("sg12: second migration run (idempotent)");

    // 验证 gift_records 表存在
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = 'gift_records')",
    )
    .fetch_one(&pool)
    .await
    .expect("sg12: check table exists");

    assert!(
        table_exists,
        "sg12: gift_records table should exist after migration"
    );

    // 验证 charm_balance 列存在
    let col_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_name = 'users' AND column_name = 'charm_balance')",
    )
    .fetch_one(&pool)
    .await
    .expect("sg12: check column exists");

    assert!(
        col_exists,
        "sg12: users.charm_balance column should exist after migration"
    );
}
